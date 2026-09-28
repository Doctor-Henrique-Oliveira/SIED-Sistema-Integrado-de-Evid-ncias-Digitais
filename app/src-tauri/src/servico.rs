//! Regras do aplicativo, independentes do Tauri (testáveis sem janela).
//! Os comandos Tauri apenas travam o estado e chamam estes métodos.

use serde::Serialize;
use sied_armazenamento::auditoria::{RegistroAuditoria, VerificacaoAuditoria};
use sied_armazenamento::caso::{nome_pasta_sugerido, NOME_ARQUIVO_CASO};
use sied_armazenamento::extracoes::{
    ArquivoExtracao, ArtefatoDetalhe, ArtefatoResumo, ExecucaoParser, Extracao, Pagina,
    TotalCategoria,
};
use sied_armazenamento::{BancoCaso, CasoCatalogado, CatalogoGlobal, Sessao};
use sied_dominio::{
    Caso, CasoId, Dispositivo, DispositivoId, ExtracaoId, NovoCaso, NovoDispositivo,
};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::importador::{Deteccao, Suporte};
use sied_ingestao::pipeline::{self, Controle, Progresso};
use sied_ingestao::Registro;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use ts_rs::TS;

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};

const CONFIG_PASTA_CASOS: &str = "pasta_casos_padrao";

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct InfoApp {
    pub versao: String,
    pub usuario: String,
    pub maquina: String,
    pub pasta_dados: String,
    pub pasta_logs: String,
    pub pasta_casos_padrao: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CasoAberto {
    pub caso: Caso,
    pub pasta: String,
}

/// Situação da importação em curso (ou da última, até iniciar outra).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct EstadoImportacao {
    pub extracao_id: ExtracaoId,
    pub dispositivo_id: DispositivoId,
    pub progresso: Progresso,
    pub terminada: bool,
    /// Registro final da extração, quando terminada.
    pub extracao: Option<Extracao>,
    /// Falha ao gravar o resultado (a própria importação registra seus erros na extração).
    pub erro: Option<String>,
}

struct TarefaImportacao {
    extracao_id: ExtracaoId,
    dispositivo_id: DispositivoId,
    controle: Arc<Controle>,
    fim: Arc<Mutex<Option<Result<Extracao, String>>>>,
    thread: Option<JoinHandle<()>>,
}

impl TarefaImportacao {
    fn em_curso(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }

    /// Pede o cancelamento e espera a thread terminar de gravar.
    fn cancelar_e_esperar(&mut self) {
        self.controle.pedir_cancelamento();
        if let Some(t) = self.thread.take() {
            if t.join().is_err() {
                tracing::error!(extracao = %self.extracao_id, "thread de importação terminou com pânico");
            }
        }
    }

    fn estado(&self) -> EstadoImportacao {
        let fim = self.fim.lock().ok().and_then(|f| f.clone());
        EstadoImportacao {
            extracao_id: self.extracao_id,
            dispositivo_id: self.dispositivo_id,
            progresso: self.controle.progresso(),
            terminada: fim.is_some(),
            erro: fim.as_ref().and_then(|r| r.as_ref().err().cloned()),
            extracao: fim.and_then(Result::ok),
        }
    }
}

pub struct Servico {
    catalogo: CatalogoGlobal,
    caso: Option<BancoCaso>,
    importacao: Option<TarefaImportacao>,
    sessao: Sessao,
    pasta_dados: PathBuf,
    pasta_logs: PathBuf,
    pasta_casos_inicial: PathBuf,
}

impl Servico {
    pub fn iniciar(
        pasta_dados: &Path,
        pasta_logs: &Path,
        pasta_casos_inicial: &Path,
        sessao: Sessao,
    ) -> ResultadoApp<Self> {
        let catalogo = CatalogoGlobal::abrir(pasta_dados)?;
        Ok(Servico {
            catalogo,
            caso: None,
            importacao: None,
            sessao,
            pasta_dados: pasta_dados.into(),
            pasta_logs: pasta_logs.into(),
            pasta_casos_inicial: pasta_casos_inicial.into(),
        })
    }

    fn pasta_casos_padrao(&self) -> ResultadoApp<PathBuf> {
        Ok(self
            .catalogo
            .configuracao(CONFIG_PASTA_CASOS)?
            .map(PathBuf::from)
            .unwrap_or_else(|| self.pasta_casos_inicial.clone()))
    }

    pub fn info(&self) -> ResultadoApp<InfoApp> {
        Ok(InfoApp {
            versao: env!("CARGO_PKG_VERSION").into(),
            usuario: self.sessao.usuario.clone(),
            maquina: self.sessao.maquina.clone(),
            pasta_dados: self.pasta_dados.display().to_string(),
            pasta_logs: self.pasta_logs.display().to_string(),
            pasta_casos_padrao: self.pasta_casos_padrao()?.display().to_string(),
        })
    }

    pub fn listar_casos(&self) -> ResultadoApp<Vec<CasoCatalogado>> {
        Ok(self.catalogo.listar_casos()?)
    }

    fn fechar_atual(&mut self) -> ResultadoApp<()> {
        // Uma importação em curso é cancelada (e registrada como tal) antes de fechar.
        if let Some(mut t) = self.importacao.take() {
            t.cancelar_e_esperar();
        }
        if let Some(banco) = self.caso.take() {
            banco.fechar()?;
        }
        Ok(())
    }

    fn ativar(&mut self, banco: BancoCaso) -> ResultadoApp<CasoAberto> {
        self.catalogo.registrar_caso(banco.caso(), banco.pasta())?;
        let aberto = CasoAberto {
            caso: banco.caso().clone(),
            pasta: banco.pasta().display().to_string(),
        };
        self.caso = Some(banco);
        Ok(aberto)
    }

    /// Cria o caso em `<pasta_base>/<nome sugerido>`; se o nome já existir, acrescenta -2, -3...
    pub fn criar_caso(
        &mut self,
        dados: NovoCaso,
        pasta_base: Option<String>,
    ) -> ResultadoApp<CasoAberto> {
        let dados = dados.normalizado();
        // Valida antes de mexer em pastas ou fechar o caso atual.
        dados
            .validar()
            .map_err(sied_armazenamento::ErroArmazenamento::from)?;
        let base = match pasta_base.filter(|p| !p.trim().is_empty()) {
            Some(p) => PathBuf::from(p),
            None => self.pasta_casos_padrao()?,
        };
        let nome = nome_pasta_sugerido(&dados);
        let mut pasta = base.join(&nome);
        let mut n = 2;
        while pasta.exists() {
            pasta = base.join(format!("{nome}-{n}"));
            n += 1;
        }
        self.fechar_atual()?;
        let banco = BancoCaso::criar(&pasta, dados, self.sessao.clone())?;
        self.catalogo
            .definir_configuracao(CONFIG_PASTA_CASOS, &base.display().to_string())?;
        self.ativar(banco)
    }

    pub fn abrir_caso(&mut self, caso_id: CasoId) -> ResultadoApp<CasoAberto> {
        if let Some(b) = &self.caso {
            if b.caso().id == caso_id {
                return Ok(CasoAberto {
                    caso: b.caso().clone(),
                    pasta: b.pasta().display().to_string(),
                });
            }
        }
        let pasta = self
            .catalogo
            .pasta_do_caso(caso_id)?
            .ok_or_else(|| ErroApp::novo(CodigoErro::NaoEncontrado, "caso não está no catálogo"))?;
        self.abrir_pasta(&pasta)
    }

    /// Abre uma pasta de caso (ex.: copiada de outra estação) e a inclui no catálogo.
    pub fn abrir_pasta(&mut self, pasta: &Path) -> ResultadoApp<CasoAberto> {
        let pasta = if pasta
            .file_name()
            .map(|n| n == NOME_ARQUIVO_CASO)
            .unwrap_or(false)
        {
            pasta.parent().unwrap_or(pasta)
        } else {
            pasta
        };
        self.fechar_atual()?;
        let banco = BancoCaso::abrir(pasta, self.sessao.clone())?;
        self.ativar(banco)
    }

    pub fn fechar_caso(&mut self) -> ResultadoApp<()> {
        self.fechar_atual()
    }

    pub fn caso_atual(&self) -> Option<CasoAberto> {
        self.caso.as_ref().map(|b| CasoAberto {
            caso: b.caso().clone(),
            pasta: b.pasta().display().to_string(),
        })
    }

    fn banco(&mut self) -> ResultadoApp<&mut BancoCaso> {
        self.caso.as_mut().ok_or_else(ErroApp::nenhum_caso)
    }

    pub fn cadastrar_dispositivo(&mut self, dados: NovoDispositivo) -> ResultadoApp<Dispositivo> {
        Ok(self.banco()?.cadastrar_dispositivo(dados)?)
    }

    pub fn listar_dispositivos(&mut self) -> ResultadoApp<Vec<Dispositivo>> {
        Ok(self.banco()?.listar_dispositivos()?)
    }

    pub fn listar_auditoria(&mut self, limite: i64) -> ResultadoApp<Vec<RegistroAuditoria>> {
        Ok(self.banco()?.auditoria(limite.clamp(1, 1000))?)
    }

    pub fn verificar_auditoria(&mut self) -> ResultadoApp<VerificacaoAuditoria> {
        Ok(self.banco()?.verificar_auditoria()?)
    }

    // ---------- Importação ----------

    /// Pergunta a cada importador se reconhece a pasta. Não grava nada.
    pub fn detectar_fonte(&self, pasta: &Path) -> ResultadoApp<Vec<Deteccao>> {
        let fonte = PastaSomenteLeitura::abrir(pasta)?;
        Ok(Registro::padrao().detectar(&fonte)?)
    }

    pub fn iniciar_importacao(
        &mut self,
        dispositivo_id: DispositivoId,
        pasta: &Path,
        importador_id: &str,
    ) -> ResultadoApp<EstadoImportacao> {
        if self.importacao.as_ref().is_some_and(|t| t.em_curso()) {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "já existe uma importação em andamento; aguarde ou cancele",
            ));
        }
        let registro = Registro::padrao();
        let importador = registro.importador(importador_id).ok_or_else(|| {
            ErroApp::novo(
                CodigoErro::NaoEncontrado,
                format!("importador '{importador_id}' não existe"),
            )
        })?;
        let fonte = PastaSomenteLeitura::abrir(pasta)?;
        let deteccao = importador.detectar(&fonte)?;
        if matches!(
            deteccao.suporte,
            Suporte::NaoImplementado | Suporte::NaoReconhecido
        ) {
            return Err(ErroApp::novo(
                CodigoErro::NaoSuportado,
                format!(
                    "{}: {}",
                    deteccao.importador.nome,
                    deteccao.motivos.join("; ")
                ),
            ));
        }
        let info = importador.info();
        let mut gravador = self
            .banco()?
            .iniciar_importacao(dispositivo_id, &info, pasta)?;
        let extracao_id = gravador.extracao_id();
        let controle = Arc::new(Controle::default());
        let fim = Arc::new(Mutex::new(None));
        let (c, f) = (controle.clone(), fim.clone());
        let importador_id = importador_id.to_string();
        let thread = std::thread::Builder::new()
            .name(format!("importacao-{extracao_id}"))
            .spawn(move || {
                let registro = Registro::padrao();
                let importador = registro
                    .importador(&importador_id)
                    .expect("importador validado antes");
                let ctx = gravador.contexto().clone();
                let r = pipeline::executar(&fonte, importador, &registro, &mut gravador, &c, &ctx);
                match &r {
                    Ok(_) => {}
                    Err(sied_ingestao::ErroIngestao::Cancelado) => {
                        c.atualizar(|p| p.etapa = pipeline::Etapa::Cancelada)
                    }
                    Err(e) => c.atualizar(|p| {
                        p.etapa = pipeline::Etapa::Falhou;
                        p.mensagem = Some(e.to_string());
                    }),
                }
                let final_ = gravador.finalizar(&r).map_err(|e| {
                    tracing::error!(erro = %e, "falha ao gravar o fim da importação");
                    e.to_string()
                });
                if let Ok(mut f) = f.lock() {
                    *f = Some(final_);
                }
            })
            .map_err(|e| {
                ErroApp::novo(
                    CodigoErro::Interno,
                    format!("não foi possível iniciar a importação: {e}"),
                )
            })?;
        let tarefa = TarefaImportacao {
            extracao_id,
            dispositivo_id,
            controle,
            fim,
            thread: Some(thread),
        };
        let estado = tarefa.estado();
        self.importacao = Some(tarefa);
        Ok(estado)
    }

    pub fn estado_importacao(&self) -> Option<EstadoImportacao> {
        self.importacao.as_ref().map(TarefaImportacao::estado)
    }

    pub fn cancelar_importacao(&self) {
        if let Some(t) = &self.importacao {
            t.controle.pedir_cancelamento();
        }
    }

    pub fn listar_extracoes(&mut self) -> ResultadoApp<Vec<Extracao>> {
        Ok(self.banco()?.extracoes()?)
    }

    pub fn obter_extracao(&mut self, id: ExtracaoId) -> ResultadoApp<Extracao> {
        Ok(self.banco()?.extracao(id)?)
    }

    pub fn totais_por_categoria(&mut self, id: ExtracaoId) -> ResultadoApp<Vec<TotalCategoria>> {
        Ok(self.banco()?.totais_por_categoria(id)?)
    }

    pub fn listar_arquivos(
        &mut self,
        id: ExtracaoId,
        categoria: Option<String>,
        texto: Option<String>,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<ArquivoExtracao>> {
        Ok(self.banco()?.arquivos(
            id,
            categoria.as_deref(),
            texto.as_deref(),
            deslocamento.max(0),
            limite.clamp(1, 500),
        )?)
    }

    pub fn listar_execucoes(&mut self, id: ExtracaoId) -> ResultadoApp<Vec<ExecucaoParser>> {
        Ok(self.banco()?.execucoes(id)?)
    }

    pub fn listar_artefatos(
        &mut self,
        id: ExtracaoId,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<ArtefatoResumo>> {
        Ok(self
            .banco()?
            .artefatos(id, deslocamento.max(0), limite.clamp(1, 500))?)
    }

    pub fn detalhe_artefato(&mut self, artefato_id: i64) -> ResultadoApp<ArtefatoDetalhe> {
        Ok(self.banco()?.artefato(artefato_id)?)
    }

    /// Chamado ao encerrar o aplicativo.
    pub fn encerrar(&mut self) {
        if let Err(e) = self.fechar_atual() {
            tracing::error!(erro = %e.mensagem, "falha ao fechar o caso no encerramento");
        }
        if let Err(e) = self.catalogo.consolidar() {
            tracing::error!(erro = %e, "falha ao consolidar o banco global");
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use sied_dominio::SistemaOperacional;

    fn servico(raiz: &Path) -> Servico {
        Servico::iniciar(
            &raiz.join("dados"),
            &raiz.join("dados/logs"),
            &raiz.join("Casos"),
            Sessao {
                usuario: "teste".into(),
                maquina: "PC".into(),
            },
        )
        .unwrap()
    }

    fn caso() -> NovoCaso {
        NovoCaso {
            numero_procedimento: Some("IP 1/2026".into()),
            responsavel_analise: "Analista".into(),
            unidade: "DP".into(),
            ..Default::default()
        }
    }

    #[test]
    fn fluxo_completo_com_reinicio() {
        let raiz = tempfile::tempdir().unwrap();
        let id = {
            let mut s = servico(raiz.path());
            let aberto = s.criar_caso(caso(), None).unwrap();
            assert!(aberto.pasta.ends_with("IP-1-2026"));
            s.cadastrar_dispositivo(NovoDispositivo {
                apelido: "Celular A".into(),
                sistema: SistemaOperacional::Android,
                ..Default::default()
            })
            .unwrap();
            s.encerrar();
            aberto.caso.id
        };
        let mut s = servico(raiz.path());
        assert!(s.caso_atual().is_none());
        let casos = s.listar_casos().unwrap();
        assert_eq!(casos.len(), 1);
        s.abrir_caso(id).unwrap();
        let d = s.listar_dispositivos().unwrap();
        assert_eq!(d[0].dados.apelido, "Celular A");
        assert_eq!(s.verificar_auditoria().unwrap().registros, 4);
    }

    #[test]
    fn nomes_de_pasta_nao_colidem() {
        let raiz = tempfile::tempdir().unwrap();
        let mut s = servico(raiz.path());
        let a = s.criar_caso(caso(), None).unwrap();
        let b = s.criar_caso(caso(), None).unwrap();
        assert_ne!(a.pasta, b.pasta);
        assert!(b.pasta.ends_with("IP-1-2026-2"));
        assert_eq!(s.listar_casos().unwrap().len(), 2);
    }

    #[test]
    fn caso_invalido_nao_fecha_o_atual() {
        let raiz = tempfile::tempdir().unwrap();
        let mut s = servico(raiz.path());
        s.criar_caso(caso(), None).unwrap();
        let erro = s.criar_caso(NovoCaso::default(), None).unwrap_err();
        assert!(matches!(erro.codigo, CodigoErro::Validacao));
        assert!(!erro.campos.is_empty());
        assert!(s.caso_atual().is_some());
    }

    #[test]
    fn dispositivo_sem_caso_aberto() {
        let raiz = tempfile::tempdir().unwrap();
        let mut s = servico(raiz.path());
        let erro = s
            .cadastrar_dispositivo(NovoDispositivo {
                apelido: "x".into(),
                ..Default::default()
            })
            .unwrap_err();
        assert!(matches!(erro.codigo, CodigoErro::NenhumCasoAberto));
    }

    /// Extração SINTÉTICA mínima: um mmssms.db com uma mensagem inventada.
    fn extracao_sintetica(dir: &Path) {
        std::fs::create_dir_all(dir.join("telefonia")).unwrap();
        std::fs::write(dir.join("SINTETICO.txt"), "dados sintéticos de teste").unwrap();
        rusqlite::Connection::open(dir.join("telefonia/mmssms.db"))
            .unwrap()
            .execute_batch(
                "CREATE TABLE sms(_id INTEGER PRIMARY KEY, thread_id, address, date, type, body);
                 INSERT INTO sms VALUES (1, 1, '+5500000000000', 1767225600000, 1, 'sintético');",
            )
            .unwrap();
    }

    fn esperar(s: &Servico) -> EstadoImportacao {
        for _ in 0..500 {
            let e = s.estado_importacao().unwrap();
            if e.terminada {
                return e;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("importação não terminou");
    }

    #[test]
    fn importa_em_segundo_plano() {
        let raiz = tempfile::tempdir().unwrap();
        let origem = raiz.path().join("Extracao");
        extracao_sintetica(&origem);
        let mut s = servico(raiz.path());
        s.criar_caso(caso(), None).unwrap();
        let d = s
            .cadastrar_dispositivo(NovoDispositivo {
                apelido: "A".into(),
                ..Default::default()
            })
            .unwrap();

        let det = s.detectar_fonte(&origem).unwrap();
        assert_eq!(det[0].importador.id, "generico");
        let erro = s.iniciar_importacao(d.id, &origem, "avilla").unwrap_err();
        assert!(matches!(erro.codigo, CodigoErro::NaoSuportado));

        s.iniciar_importacao(d.id, &origem, "generico").unwrap();
        let fim = esperar(&s);
        let x = fim.extracao.expect("extração final");
        assert_eq!(x.status, "concluida");
        assert_eq!(x.total_arquivos, Some(2));
        assert_eq!(x.total_artefatos, Some(1));
        assert_eq!(fim.progresso.etapa, pipeline::Etapa::Concluida);
        assert_eq!(s.listar_extracoes().unwrap().len(), 1);
        let a = s.listar_artefatos(x.id, 0, 10).unwrap();
        let det = s.detalhe_artefato(a.itens[0].id).unwrap();
        assert_eq!(det.referencias[0].arquivo_caminho, "telefonia/mmssms.db");
        s.verificar_auditoria().unwrap();
    }

    #[test]
    fn fechar_o_caso_durante_a_importacao_cancela_e_registra() {
        let raiz = tempfile::tempdir().unwrap();
        let origem = raiz.path().join("Extracao");
        extracao_sintetica(&origem);
        std::fs::write(origem.join("grande.bin"), vec![7u8; 64 << 20]).unwrap();
        let mut s = servico(raiz.path());
        let aberto = s.criar_caso(caso(), None).unwrap();
        let d = s
            .cadastrar_dispositivo(NovoDispositivo {
                apelido: "A".into(),
                ..Default::default()
            })
            .unwrap();
        s.iniciar_importacao(d.id, &origem, "generico").unwrap();
        s.fechar_caso().unwrap();
        s.abrir_caso(aberto.caso.id).unwrap();
        let x = &s.listar_extracoes().unwrap()[0];
        assert!(
            ["cancelada", "concluida"].contains(&x.status.as_str()),
            "{}",
            x.status
        );
        s.verificar_auditoria().unwrap();
    }

    #[test]
    fn abre_pasta_copiada_e_cataloga() {
        let raiz = tempfile::tempdir().unwrap();
        let pasta = {
            let mut s = servico(raiz.path());
            let a = s.criar_caso(caso(), None).unwrap();
            s.encerrar();
            PathBuf::from(a.pasta)
        };
        // Outra estação (outro banco global) abre a mesma pasta.
        let outra = raiz.path().join("outra");
        let mut s2 = servico(&outra);
        assert!(s2.listar_casos().unwrap().is_empty());
        s2.abrir_pasta(&pasta.join(NOME_ARQUIVO_CASO)).unwrap();
        assert_eq!(s2.listar_casos().unwrap().len(), 1);
    }
}
