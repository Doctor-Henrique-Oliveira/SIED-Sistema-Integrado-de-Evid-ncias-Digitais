//! Regras do aplicativo, independentes do Tauri (testáveis sem janela).
//! Os comandos Tauri apenas travam o estado e chamam estes métodos.

use serde::Serialize;
use sied_armazenamento::agenda::{
    Chamada, ContatoAgenda, FiltroChamadas, FiltroContatos, ResumoChamadas, TotalApp,
};
use sied_armazenamento::auditoria::{RegistroAuditoria, VerificacaoAuditoria};
use sied_armazenamento::busca::{AchadoArtefato, FiltroBusca, ResultadoBusca};
use sied_armazenamento::caso::{nome_pasta_sugerido, NOME_ARQUIVO_CASO};
use sied_armazenamento::controle_descoberta::ControleDescoberta;
use sied_armazenamento::conversas::{AppConversas, Conversa, FiltroConversas, PosicaoMensagem};
use sied_armazenamento::derivados::{ConferenciaDerivados, ReconstrucaoDerivados};
use sied_armazenamento::descoberta::{PedidoDescoberta, ResultadoDescoberta};
use sied_armazenamento::embeddings::{ResultadoBuscaHibrida, ResumoIndexacaoEmbeddings};
use sied_armazenamento::evidencias::{Evidencia, Marcacoes, NovaEvidencia, TextosEvidencia};
use sied_armazenamento::extracoes::{
    ArquivoExtracao, ArtefatoDetalhe, ArtefatoResumo, ExecucaoParser, Extracao, Pagina,
    TotalCategoria,
};
use sied_armazenamento::linha_tempo::{FiltroLinhaTempo, ResumoLinhaTempo};
use sied_armazenamento::pesquisa_local::{
    ConsultaDescobertaSalva, ExecucaoDescoberta, VersaoVocabularioPesquisa, VocabularioPesquisa,
};
use sied_armazenamento::relatorios::{ConferenciaPastaRelatorios, RelatorioRegistrado};
use sied_armazenamento::resumo::{Identificador, ResumoCaso};
use sied_armazenamento::transcricoes::{NovaTranscricao, SegmentoTranscricao, TranscricaoLocal};
use sied_armazenamento::{
    BancoCaso, CasoCatalogado, CatalogoGlobal, NovoProvedorEstacao, ProvedorEstacao, Sessao,
};
use sied_documentos::{ControleDocumento, ModoDocumento, PedidoDocumento, ResultadoDocumento};
use sied_dominio::{
    Caso, CasoId, Dispositivo, DispositivoId, ExtracaoId, NovoCaso, NovoDispositivo,
};
use sied_ingestao::eventos::{EventoImportacao, NivelEvento};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::fonte::{CaminhoFonte, EntradaFonte, FonteSomenteLeitura, LeitorFonte};
use sied_ingestao::importador::{Deteccao, Suporte};
use sied_ingestao::pipeline::{self, Controle, Progresso};
use sied_ingestao::{ErroIngestao, Registro};
use sied_midia::Pedido as PedidoMiniatura;
use sied_relatorio::PedidoRelatorio;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use ts_rs::TS;

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};

const CONFIG_PASTA_CASOS: &str = "pasta_casos_padrao";

/// Dados resolvidos no inventário do caso ativo, nunca fornecidos pelo frontend.
/// A leitura da origem acontece somente depois de soltar a trava do serviço.
pub struct LeituraDocumento {
    pasta_extracao: PathBuf,
    pedido: PedidoDocumento,
}

/// Endereço resolvido do inventário para a transcrição local. O frontend só
/// fornece o ID; caminho, hash e extração vêm do banco do caso.
pub(crate) struct LeituraTranscricao {
    pub(crate) pasta_extracao: PathBuf,
    pub(crate) caminho: CaminhoFonte,
    pub(crate) caso_id: CasoId,
    pub(crate) extracao_id: ExtracaoId,
    pub(crate) arquivo_id: i64,
    pub(crate) sha256: String,
}

impl LeituraTranscricao {
    pub(crate) fn abrir(&self) -> ResultadoApp<Box<dyn LeitorFonte>> {
        self.abrir_caminho(&self.caminho)
    }

    pub(crate) fn abrir_caminho(
        &self,
        caminho: &CaminhoFonte,
    ) -> ResultadoApp<Box<dyn LeitorFonte>> {
        let fonte = FonteDocumento {
            pasta: self.pasta_extracao.clone(),
            caminho: caminho.clone(),
        };
        Ok(fonte.abrir(caminho)?)
    }

    /// Caminho físico somente para um arquivo regular da pasta da extração.
    /// Contêineres (`!/`) continuam restritos ao leitor somente leitura e não
    /// são entregues a um processo externo.
    pub(crate) fn caminho_absoluto(&self) -> Option<PathBuf> {
        if self
            .caminho
            .como_str()
            .contains(sied_ingestao::fonte::SEPARADOR_CONTEINER)
        {
            return None;
        }
        let caminho = self.pasta_extracao.join(self.caminho.como_str());
        caminho.is_file().then_some(caminho)
    }
}

impl LeituraDocumento {
    /// Caminho físico conferido pelo inventário para um provider local de visão.
    /// Contêineres e caminhos ausentes nunca são entregues a processos externos.
    pub(crate) fn caminho_absoluto(&self) -> Option<PathBuf> {
        if self
            .pedido
            .caminho
            .como_str()
            .contains(sied_ingestao::fonte::SEPARADOR_CONTEINER)
        {
            return None;
        }
        let caminho = self.pasta_extracao.join(self.pedido.caminho.como_str());
        caminho.is_file().then_some(caminho)
    }

    pub(crate) fn tamanho_origem(&self) -> u64 {
        self.pedido.tamanho
    }

    pub(crate) fn tem_hash_origem(&self) -> bool {
        self.pedido.sha256.is_some()
    }

    pub fn com_modo(mut self, modo: ModoDocumento) -> Self {
        self.pedido.modo = modo;
        self
    }

    #[cfg(test)]
    pub fn executar(self) -> ResultadoDocumento {
        self.executar_pagina(None)
    }

    #[cfg(test)]
    pub fn executar_pagina(self, pagina: Option<u32>) -> ResultadoDocumento {
        self.executar_controlado(pagina, &ControleDocumento::novo())
    }

    pub fn executar_controlado(
        self,
        pagina: Option<u32>,
        controle: &ControleDocumento,
    ) -> ResultadoDocumento {
        let fonte = FonteDocumento {
            pasta: self.pasta_extracao,
            caminho: self.pedido.caminho.clone(),
        };
        sied_documentos::analisar_pagina(&fonte, &self.pedido, controle, pagina)
    }

    pub fn executar_visual(
        self,
        controle: &ControleDocumento,
    ) -> sied_documentos::visao::ResultadoVisual {
        let fonte = FonteDocumento {
            pasta: self.pasta_extracao,
            caminho: self.pedido.caminho.clone(),
        };
        sied_documentos::visao::analisar(&fonte, &self.pedido, controle)
    }
}

/// Fonte restrita ao único endereço resolvido no inventário. A abertura é tardia:
/// formatos sem suporte e pedidos acima dos limites não tocam o disco da evidência.
struct FonteDocumento {
    pasta: PathBuf,
    caminho: CaminhoFonte,
}

impl FonteDocumento {
    fn recusar(&self) -> ErroIngestao {
        ErroIngestao::ItemIgnorado {
            caminho: self.caminho.to_string(),
            motivo: "origem documental não é um arquivo regular confinado, sem links".into(),
        }
    }

    /// Confere os componentes intermediários também, não apenas o arquivo final.
    /// Reparse points do Windows (incluindo junções) são recusados. Esta barreira
    /// não promete proteção rígida contra substituição concorrente do filesystem;
    /// o hash e o tamanho do inventário continuam obrigatórios antes de publicar.
    fn conferir_caminho(&self, raiz: &Path) -> sied_ingestao::Resultado<()> {
        let mut destino = raiz.to_path_buf();
        let partes: Vec<_> = self.caminho.como_str().split('/').collect();
        for (indice, parte) in partes.iter().enumerate() {
            let mut componentes = Path::new(parte).components();
            if !matches!(
                (componentes.next(), componentes.next()),
                (Some(Component::Normal(_)), None)
            ) {
                return Err(self.recusar());
            }
            #[cfg(windows)]
            if parte.contains(':') {
                // Não permitir alternate data streams como um documento do inventário.
                return Err(self.recusar());
            }
            destino.push(parte);
            let metadados = std::fs::symlink_metadata(&destino)?;
            if metadados.file_type().is_symlink() || ponto_de_reparse(&metadados) {
                return Err(self.recusar());
            }
            if indice + 1 == partes.len() {
                if !metadados.is_file() {
                    return Err(self.recusar());
                }
            } else if !metadados.is_dir() {
                return Err(self.recusar());
            }
            if !destino.canonicalize()?.starts_with(raiz) {
                return Err(self.recusar());
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
fn ponto_de_reparse(metadados: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadados.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
}

#[cfg(not(windows))]
fn ponto_de_reparse(_: &std::fs::Metadata) -> bool {
    false
}

impl FonteSomenteLeitura for FonteDocumento {
    fn descricao(&self) -> String {
        "Documento restrito ao inventário do caso".into()
    }

    fn listar(&self) -> Box<dyn Iterator<Item = sied_ingestao::Resultado<EntradaFonte>> + '_> {
        Box::new(std::iter::empty())
    }

    fn entrada(&self, _: &CaminhoFonte) -> sied_ingestao::Resultado<Option<EntradaFonte>> {
        Err(self.recusar())
    }

    fn abrir(&self, caminho: &CaminhoFonte) -> sied_ingestao::Resultado<Box<dyn LeitorFonte>> {
        if caminho != &self.caminho {
            return Err(self.recusar());
        }
        let fonte = PastaSomenteLeitura::abrir(&self.pasta)?;
        self.conferir_caminho(fonte.raiz())?;
        let leitor = fonte.abrir(caminho)?;
        self.conferir_caminho(fonte.raiz())?;
        Ok(leitor)
    }
}

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

    pub fn listar_provedores_estacao(&self) -> ResultadoApp<Vec<ProvedorEstacao>> {
        Ok(self.catalogo.listar_provedores()?)
    }

    pub fn salvar_provedor_estacao(
        &self,
        dados: NovoProvedorEstacao,
    ) -> ResultadoApp<ProvedorEstacao> {
        let provedor = self.catalogo.salvar_provedor(dados)?;
        tracing::info!(
            provedor = %provedor.id,
            tipo = %provedor.tipo,
            modo = %provedor.modo,
            habilitado = provedor.habilitado,
            "configuração de provedor da estação atualizada"
        );
        Ok(provedor)
    }

    /// Retira o caso apenas do catálogo global. Não abre, altera ou remove a
    /// pasta do caso nem qualquer dado forense nela contido.
    pub fn remover_caso_catalogo(&self, caso_id: CasoId) -> ResultadoApp<bool> {
        Ok(self.catalogo.remover_caso(caso_id)?)
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

    pub(crate) fn banco_documental(&mut self, caso_id: CasoId) -> ResultadoApp<&mut BancoCaso> {
        let banco = self.banco()?;
        if banco.caso().id != caso_id {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "o caso ativo mudou; selecione novamente o documento no caso atual",
            ));
        }
        Ok(banco)
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

    /// Confere conversas e índices de busca contra os registros de origem. Só lê.
    pub fn conferir_derivados(&mut self) -> ResultadoApp<ConferenciaDerivados> {
        Ok(self.banco()?.conferir_derivados()?)
    }

    /// Recalcula conversas e índices de busca (com auditoria). Recusado durante uma
    /// importação, que grava nas mesmas tabelas.
    pub fn reconstruir_derivados(&mut self) -> ResultadoApp<ReconstrucaoDerivados> {
        if self.importacao.as_ref().is_some_and(|t| t.em_curso()) {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "há uma importação em andamento; reconstrua depois que ela terminar",
            ));
        }
        Ok(self.banco()?.reconstruir_derivados()?)
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
                // Mesmo um pânico fora dos parsers termina com a extração finalizada
                // (status "falhou") e o estado da tarefa preenchido.
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    pipeline::executar(&fonte, importador, &registro, &mut gravador, &c, &ctx)
                }))
                .unwrap_or_else(|p| {
                    Err(sied_ingestao::ErroIngestao::Interno(format!(
                        "a importação parou por um defeito do SIED ({})",
                        pipeline::texto_do_panico(p.as_ref())
                    )))
                });
                match &r {
                    Ok(_) => {}
                    Err(sied_ingestao::ErroIngestao::Cancelado) => {
                        c.atualizar(|p| p.entrar(pipeline::Etapa::Cancelada))
                    }
                    Err(e) => c.atualizar(|p| {
                        p.entrar(pipeline::Etapa::Falhou);
                        p.mensagem = Some(e.to_string());
                    }),
                }
                c.anunciar(
                    NivelEvento::Info,
                    "Gravando o resultado da extração no caso e na trilha de auditoria...",
                );
                let final_ = gravador.finalizar(&r).map_err(|e| {
                    tracing::error!(erro = %e, "falha ao gravar o fim da importação");
                    e.to_string()
                });
                match &final_ {
                    Ok(x) => c.anunciar(
                        NivelEvento::Ok,
                        format!("Extração registrada com status \"{}\".", x.status),
                    ),
                    Err(e) => c.anunciar(
                        NivelEvento::Erro,
                        format!("Falha ao gravar o resultado da extração: {e}"),
                    ),
                };
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

    /// Eventos do console da importação em curso (ou da última) depois de `desde`.
    pub fn eventos_importacao(&self, desde: u64) -> Vec<EventoImportacao> {
        self.importacao
            .as_ref()
            .map(|t| t.controle.eventos_desde(desde))
            .unwrap_or_default()
    }

    /// Console de uma importação já terminada, relido do log gravado no caso.
    pub fn eventos_da_extracao(&mut self, id: ExtracaoId) -> ResultadoApp<Vec<EventoImportacao>> {
        Ok(self.banco()?.eventos_da_importacao(id)?)
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

    /// Sem filtros além da extração (atalho dos testes; a tela usa a versão filtrada).
    #[cfg(test)]
    pub fn listar_conversas(
        &mut self,
        extracao_id: Option<ExtracaoId>,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<Conversa>> {
        Ok(self
            .banco()?
            .conversas(extracao_id, deslocamento.max(0), limite.clamp(1, 500))?)
    }

    pub fn listar_conversas_filtradas(
        &mut self,
        filtro: &FiltroConversas,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<Conversa>> {
        Ok(self
            .banco()?
            .conversas_filtradas(filtro, deslocamento.max(0), limite.clamp(1, 500))?)
    }

    pub fn apps_das_conversas(
        &mut self,
        extracao_id: Option<ExtracaoId>,
    ) -> ResultadoApp<Vec<AppConversas>> {
        Ok(self.banco()?.apps_das_conversas(extracao_id)?)
    }

    pub fn posicao_mensagem(
        &mut self,
        conversa_id: i64,
        artefato_id: i64,
    ) -> ResultadoApp<Option<i64>> {
        Ok(self.banco()?.posicao_mensagem(conversa_id, artefato_id)?)
    }

    pub fn localizar_na_conversa(
        &mut self,
        conversa_id: i64,
        termo: &str,
        limite: i64,
    ) -> ResultadoApp<Pagina<PosicaoMensagem>> {
        Ok(self
            .banco()?
            .localizar_na_conversa(conversa_id, termo, limite.clamp(1, 5000))?)
    }

    /// O que é preciso para gerar a miniatura fora da trava do estado (a leitura da
    /// evidência e a decodificação não seguram os outros comandos).
    pub fn pedido_miniatura(&mut self, arquivo_id: i64) -> ResultadoApp<PedidoMiniatura> {
        let banco = self.banco()?;
        let a = banco.arquivo_com_origem(arquivo_id)?;
        Ok(PedidoMiniatura {
            pasta_extracao: PathBuf::from(a.origem_caminho),
            caminho: a.arquivo.caminho,
            sha256: a.arquivo.sha256,
            tamanho: a.arquivo.tamanho.max(0) as u64,
            categoria: a.arquivo.categoria,
            pasta_caso: banco.pasta().to_path_buf(),
        })
    }

    /// O frontend escolhe apenas o ID do inventário e o caso que espera estar ativo.
    /// Caminho, extração, tamanho e SHA-256 vêm exclusivamente do banco desse caso.
    pub fn pedido_documento(
        &mut self,
        arquivo_id: i64,
        caso_id: CasoId,
    ) -> ResultadoApp<LeituraDocumento> {
        let banco = self.banco()?;
        if banco.caso().id != caso_id {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "o caso ativo mudou; selecione novamente o arquivo no caso atual",
            ));
        }
        if arquivo_id <= 0 {
            return Err(ErroApp::novo(
                CodigoErro::Validacao,
                "o identificador do arquivo no inventário é inválido",
            ));
        }
        let a = banco.arquivo_com_origem(arquivo_id)?;
        let tamanho = u64::try_from(a.arquivo.tamanho).map_err(|_| {
            ErroApp::novo(
                CodigoErro::Integridade,
                "o tamanho registrado no inventário é inválido",
            )
        })?;
        let caminho = CaminhoFonte::novo(&a.arquivo.caminho).map_err(|_| {
            ErroApp::novo(
                CodigoErro::Integridade,
                "o caminho registrado no inventário é inválido",
            )
        })?;
        Ok(LeituraDocumento {
            pasta_extracao: PathBuf::from(a.origem_caminho),
            pedido: PedidoDocumento {
                caso_id: banco.caso().id,
                extracao_id: a.extracao_id,
                arquivo_id: a.arquivo.id,
                caminho,
                sha256: a.arquivo.sha256,
                tamanho,
                modo: ModoDocumento::Automatico,
            },
        })
    }

    pub fn pedido_transcricao(
        &mut self,
        arquivo_id: i64,
        caso_id: CasoId,
    ) -> ResultadoApp<LeituraTranscricao> {
        let banco = self.banco()?;
        if banco.caso().id != caso_id {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "o caso ativo mudou; selecione novamente o arquivo",
            ));
        }
        let a = banco.arquivo_com_origem(arquivo_id)?;
        if !matches!(a.arquivo.categoria.as_str(), "audio" | "video") {
            return Err(ErroApp::novo(
                CodigoErro::NaoSuportado,
                "somente áudio e vídeo podem receber transcrição",
            ));
        }
        let sha256 = a.arquivo.sha256.clone().ok_or_else(|| {
            ErroApp::novo(
                CodigoErro::Integridade,
                "o arquivo de mídia não possui SHA-256 no inventário",
            )
        })?;
        let caminho = CaminhoFonte::novo(&a.arquivo.caminho).map_err(|_| {
            ErroApp::novo(
                CodigoErro::Integridade,
                "o caminho da mídia no inventário é inválido",
            )
        })?;
        Ok(LeituraTranscricao {
            pasta_extracao: PathBuf::from(a.origem_caminho),
            caminho,
            caso_id: banco.caso().id,
            extracao_id: a.extracao_id,
            arquivo_id: a.arquivo.id,
            sha256,
        })
    }

    pub fn guardar_transcricao(
        &mut self,
        novo: &NovaTranscricao,
        controle: &ControleDocumento,
    ) -> ResultadoApp<TranscricaoLocal> {
        crate::documentos::conferir_cancelamento(controle)?;
        Ok(self
            .banco_documental(novo.caso_id)?
            .guardar_transcricao(novo)?)
    }

    pub fn listar_transcricoes(
        &mut self,
        caso_id: CasoId,
        arquivo_id: i64,
    ) -> ResultadoApp<Vec<TranscricaoLocal>> {
        Ok(self
            .banco_documental(caso_id)?
            .listar_transcricoes(arquivo_id)?)
    }

    pub fn segmentos_transcricao(
        &mut self,
        caso_id: CasoId,
        transcricao_id: i64,
        deslocamento: i64,
        limite: i64,
        termo: Option<String>,
    ) -> ResultadoApp<sied_armazenamento::extracoes::Pagina<SegmentoTranscricao>> {
        Ok(self.banco_documental(caso_id)?.segmentos_transcricao(
            transcricao_id,
            deslocamento.max(0),
            limite.clamp(1, 200),
            termo.as_deref(),
        )?)
    }

    /// Resolve o caminho do arquivo de mídia somente a partir do inventário
    /// do caso ativo. O frontend converte o caminho para `asset:` do Tauri;
    /// nenhum caminho informado pelo usuário é aceito diretamente.
    pub fn caminho_midia_transcricao(
        &mut self,
        caso_id: CasoId,
        arquivo_id: i64,
    ) -> ResultadoApp<String> {
        let leitura = self.pedido_transcricao(arquivo_id, caso_id)?;
        let caminho = leitura.caminho_absoluto().ok_or_else(|| {
            ErroApp::novo(
                CodigoErro::NaoSuportado,
                "a reprodução direta está disponível apenas para arquivos regulares da extração",
            )
        })?;
        Ok(caminho.to_string_lossy().into_owned())
    }

    pub fn obter_conversa(&mut self, conversa_id: i64) -> ResultadoApp<Conversa> {
        Ok(self.banco()?.conversa(conversa_id)?)
    }

    pub fn mensagens_conversa(
        &mut self,
        conversa_id: i64,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<ArtefatoResumo>> {
        Ok(self.banco()?.mensagens_conversa(
            conversa_id,
            deslocamento.max(0),
            limite.clamp(1, 1000),
        )?)
    }

    pub fn conversa_do_artefato(&mut self, artefato_id: i64) -> ResultadoApp<Option<i64>> {
        Ok(self.banco()?.conversa_do_artefato(artefato_id)?)
    }

    pub fn buscar(
        &mut self,
        filtro: &FiltroBusca,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<ResultadoBusca> {
        Ok(self
            .banco()?
            .buscar(filtro, deslocamento.max(0), limite.clamp(1, 200))?)
    }

    pub fn indexar_embeddings(
        &mut self,
        caso_id: CasoId,
        controle: &Arc<ControleDescoberta>,
    ) -> ResultadoApp<ResumoIndexacaoEmbeddings> {
        controle.verificar()?;
        self.pesquisa_sem_importacao()?;
        Ok(self
            .banco_documental(caso_id)?
            .indexar_embeddings(controle)?)
    }

    pub fn buscar_hibrida(
        &mut self,
        caso_id: CasoId,
        texto: String,
        deslocamento: i64,
        limite: i64,
        peso_semantico: f64,
        controle: &Arc<ControleDescoberta>,
    ) -> ResultadoApp<ResultadoBuscaHibrida> {
        controle.verificar()?;
        self.pesquisa_sem_importacao()?;
        Ok(self.banco_documental(caso_id)?.buscar_hibrida(
            &texto,
            deslocamento.max(0),
            limite.clamp(1, 200),
            peso_semantico.clamp(0.0, 1.0),
            controle,
        )?)
    }

    pub fn resumo_caso(&mut self) -> ResultadoApp<ResumoCaso> {
        Ok(self.banco()?.resumo()?)
    }

    fn pesquisa_sem_importacao(&self) -> ResultadoApp<()> {
        if self.importacao.as_ref().is_some_and(|t| t.em_curso()) {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "Há uma importação em andamento. Execute a descoberta quando ela terminar.",
            ));
        }
        Ok(())
    }

    pub fn listar_vocabularios_pesquisa(
        &mut self,
        caso_id: CasoId,
    ) -> ResultadoApp<Vec<VersaoVocabularioPesquisa>> {
        Ok(self
            .banco_documental(caso_id)?
            .listar_vocabularios_pesquisa()?)
    }

    pub fn salvar_vocabulario_pesquisa(
        &mut self,
        caso_id: CasoId,
        dados: VocabularioPesquisa,
    ) -> ResultadoApp<VersaoVocabularioPesquisa> {
        Ok(self
            .banco_documental(caso_id)?
            .salvar_vocabulario_pesquisa(dados)?)
    }

    pub fn exportar_vocabulario_pesquisa(
        &mut self,
        caso_id: CasoId,
        id: i64,
    ) -> ResultadoApp<String> {
        Ok(self
            .banco_documental(caso_id)?
            .exportar_vocabulario_pesquisa(id)?)
    }

    pub fn executar_descoberta(
        &mut self,
        caso_id: CasoId,
        pedido: PedidoDescoberta,
        vocabulario_id: Option<i64>,
        controle: &Arc<ControleDescoberta>,
    ) -> ResultadoApp<ExecucaoDescoberta> {
        controle.verificar()?;
        self.pesquisa_sem_importacao()?;
        let banco = self.banco()?;
        if banco.caso().id != caso_id {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "o caso ativo mudou; abra a pesquisa no caso atual",
            ));
        }
        Ok(banco.executar_descoberta_com_controle(pedido, vocabulario_id, controle)?)
    }

    pub fn paginar_descoberta(
        &mut self,
        caso_id: CasoId,
        consulta_id: i64,
        deslocamento: i64,
        limite: i64,
        controle: &Arc<ControleDescoberta>,
    ) -> ResultadoApp<ResultadoDescoberta> {
        controle.verificar()?;
        self.pesquisa_sem_importacao()?;
        let banco = self.banco()?;
        if banco.caso().id != caso_id {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "o caso ativo mudou; abra a pesquisa no caso atual",
            ));
        }
        Ok(banco.paginar_descoberta_com_controle(
            consulta_id,
            deslocamento.max(0),
            limite.clamp(1, 200),
            controle,
        )?)
    }

    pub fn listar_consultas_descoberta(
        &mut self,
        caso_id: CasoId,
    ) -> ResultadoApp<Vec<ConsultaDescobertaSalva>> {
        Ok(self
            .banco_documental(caso_id)?
            .listar_consultas_descoberta()?)
    }

    pub fn listar_identificadores(
        &mut self,
        texto: Option<String>,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<Identificador>> {
        Ok(self.banco()?.identificadores(
            texto.as_deref(),
            deslocamento.max(0),
            limite.clamp(1, 500),
        )?)
    }

    // ------------------------------------------------------------ agenda e chamadas

    pub fn listar_contatos(
        &mut self,
        filtro: &FiltroContatos,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<ContatoAgenda>> {
        Ok(self
            .banco()?
            .contatos(filtro, deslocamento.max(0), limite.clamp(1, 500))?)
    }

    pub fn apps_dos_contatos(&mut self) -> ResultadoApp<Vec<TotalApp>> {
        Ok(self.banco()?.apps_contatos()?)
    }

    pub fn listar_chamadas(
        &mut self,
        filtro: &FiltroChamadas,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<Chamada>> {
        Ok(self
            .banco()?
            .chamadas(filtro, deslocamento.max(0), limite.clamp(1, 500))?)
    }

    pub fn resumo_chamadas(&mut self) -> ResultadoApp<ResumoChamadas> {
        Ok(self.banco()?.resumo_chamadas()?)
    }

    // ------------------------------------------------------------ linha do tempo

    pub fn linha_do_tempo(
        &mut self,
        filtro: &FiltroLinhaTempo,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<AchadoArtefato>> {
        Ok(self
            .banco()?
            .linha_do_tempo(filtro, deslocamento.max(0), limite.clamp(1, 500))?)
    }

    pub fn resumo_linha_do_tempo(
        &mut self,
        filtro: &FiltroLinhaTempo,
    ) -> ResultadoApp<ResumoLinhaTempo> {
        Ok(self.banco()?.resumo_linha_do_tempo(filtro)?)
    }

    // ------------------------------------------------------------ evidências

    pub fn listar_evidencias(&mut self) -> ResultadoApp<Vec<Evidencia>> {
        Ok(self.banco()?.evidencias()?)
    }

    pub fn marcacoes(&mut self) -> ResultadoApp<Marcacoes> {
        Ok(self.banco()?.marcacoes()?)
    }

    pub fn adicionar_evidencia(&mut self, nova: NovaEvidencia) -> ResultadoApp<Evidencia> {
        Ok(self.banco()?.adicionar_evidencia(nova)?)
    }

    pub fn alterar_evidencia(
        &mut self,
        id: i64,
        textos: TextosEvidencia,
    ) -> ResultadoApp<Evidencia> {
        Ok(self.banco()?.alterar_evidencia(id, textos)?)
    }

    pub fn mover_evidencia(&mut self, id: i64, numero: i64) -> ResultadoApp<Vec<Evidencia>> {
        Ok(self.banco()?.mover_evidencia(id, numero)?)
    }

    pub fn remover_evidencia(&mut self, id: i64, motivo: Option<String>) -> ResultadoApp<()> {
        Ok(self.banco()?.remover_evidencia(id, motivo)?)
    }

    // ------------------------------------------------------------ relatórios

    pub fn listar_relatorios(&mut self) -> ResultadoApp<Vec<RelatorioRegistrado>> {
        Ok(self.banco()?.relatorios()?)
    }

    /// Relê os PDFs registrados e lista o que não tem registro em `relatorios/`.
    pub fn conferir_pasta_relatorios(&mut self) -> ResultadoApp<ConferenciaPastaRelatorios> {
        Ok(self.banco()?.conferir_pasta_relatorios()?)
    }

    /// Gera o PDF em `<pasta do caso>/relatorios/` e o registra (com auditoria).
    pub fn gerar_relatorio(
        &mut self,
        pedido: PedidoRelatorio,
    ) -> ResultadoApp<RelatorioRegistrado> {
        let banco = self.banco()?;
        let inicio = std::time::Instant::now();
        let r = sied_relatorio::gerar(banco, pedido)?;
        tracing::info!(
            relatorio = r.id,
            caminho = %r.caminho,
            ms = inicio.elapsed().as_millis() as u64,
            "relatório gerado"
        );
        Ok(r)
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
    fn descoberta_enfileirada_nao_pesquisa_outro_caso_ativo() {
        let raiz = tempfile::tempdir().unwrap();
        let mut s = servico(raiz.path());
        let anterior = s.criar_caso(caso(), None).unwrap().caso.id;
        s.fechar_caso().unwrap();
        let mut novo = caso();
        novo.numero_procedimento = Some("SINTETICO 2/2026".into());
        let atual = s.criar_caso(novo, None).unwrap().caso.id;
        let pedido = PedidoDescoberta {
            filtro: FiltroBusca {
                texto: "SINTÉTICO".into(),
                ..Default::default()
            },
            modo: sied_armazenamento::descoberta::ModoDescoberta::Literal,
            termos: vec![],
            indicador: None,
        };
        let antes = s.verificar_auditoria().unwrap().registros;
        let erro = s
            .executar_descoberta(anterior, pedido.clone(), None, &ControleDescoberta::novo())
            .unwrap_err();
        assert!(matches!(erro.codigo, CodigoErro::Conflito));
        let erro = s
            .paginar_descoberta(anterior, 1, 0, 50, &ControleDescoberta::novo())
            .unwrap_err();
        assert!(matches!(erro.codigo, CodigoErro::Conflito));
        assert!(s.listar_consultas_descoberta().unwrap().is_empty());
        assert_eq!(s.verificar_auditoria().unwrap().registros, antes);
        let controle = ControleDescoberta::novo();
        assert!(controle.cancelar());
        let erro = s
            .executar_descoberta(atual, pedido, None, &controle)
            .unwrap_err();
        assert!(matches!(erro.codigo, CodigoErro::DescobertaCancelada));
        assert!(s.listar_consultas_descoberta().unwrap().is_empty());
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
        let conversas = s.listar_conversas(Some(x.id), 0, 10).unwrap();
        assert_eq!(conversas.total, 1);
        let msgs = s.mensagens_conversa(conversas.itens[0].id, 0, 10).unwrap();
        assert_eq!(msgs.itens[0].id, a.itens[0].id);
        let achado = s
            .buscar(
                &FiltroBusca {
                    texto: "mmssms".into(),
                    ..Default::default()
                },
                0,
                10,
            )
            .unwrap();
        assert_eq!(achado.arquivos.total, 1);
        s.verificar_auditoria().unwrap();
    }

    fn com_documento_sintetico(raiz: &Path) -> (Servico, CasoId, ExtracaoId, i64, PathBuf) {
        com_documento_sintetico_texto(raiz, "dados sintéticos de teste")
    }

    fn com_documento_sintetico_texto(
        raiz: &Path,
        texto: &str,
    ) -> (Servico, CasoId, ExtracaoId, i64, PathBuf) {
        let origem = raiz.join("Extracao-DOCUMENTAL-SINTETICA");
        extracao_sintetica(&origem);
        // Constrói a fixture antes da importação normal; nunca altera a evidência
        // já inventariada para fazer uma conferência passar.
        std::fs::write(origem.join("SINTETICO.txt"), texto).unwrap();
        let mut s = servico(raiz);
        let caso_id = s.criar_caso(caso(), None).unwrap().caso.id;
        let dispositivo = s
            .cadastrar_dispositivo(NovoDispositivo {
                apelido: "Dispositivo SINTÉTICO documental".into(),
                ..Default::default()
            })
            .unwrap();
        s.iniciar_importacao(dispositivo.id, &origem, "generico")
            .unwrap();
        let extracao = esperar(&s).extracao.unwrap();
        assert_eq!(extracao.status, "concluida");
        let arquivo_id = s
            .listar_arquivos(extracao.id, None, None, 0, 10)
            .unwrap()
            .itens
            .into_iter()
            .find(|a| a.caminho == "SINTETICO.txt")
            .unwrap()
            .id;
        (s, caso_id, extracao.id, arquivo_id, origem)
    }

    #[test]
    fn documento_resolve_origem_real_e_nao_grava_dados_ou_auditoria() {
        let raiz = tempfile::tempdir().unwrap();
        let (mut s, caso_id, extracao_id, arquivo_id, origem) =
            com_documento_sintetico(raiz.path());
        let arquivo = origem.join("SINTETICO.txt");
        let bytes = std::fs::read(&arquivo).unwrap();
        let modificado = std::fs::metadata(&arquivo).unwrap().modified().unwrap();
        let auditoria = serde_json::to_value(s.listar_auditoria(100).unwrap()).unwrap();
        let inventario =
            serde_json::to_value(s.listar_arquivos(extracao_id, None, None, 0, 10).unwrap())
                .unwrap();

        let pedido = s.pedido_documento(arquivo_id, caso_id).unwrap();
        assert_eq!(pedido.pedido.caso_id, caso_id);
        assert_eq!(pedido.pedido.extracao_id, extracao_id);
        assert_eq!(pedido.pedido.arquivo_id, arquivo_id);
        assert_eq!(pedido.pedido.caminho.como_str(), "SINTETICO.txt");
        let resultado = pedido.executar();
        assert_eq!(resultado.estado, sied_documentos::EstadoDocumento::Pronto);
        let texto = resultado.texto.unwrap();
        assert_eq!(texto.texto.as_bytes(), bytes);
        assert_eq!(texto.caso_id, caso_id);
        assert_eq!(texto.extracao_id, extracao_id);
        assert_eq!(texto.arquivo_id, arquivo_id);
        assert_eq!(texto.sha256, texto.sha256_texto);
        assert!(texto.pagina.is_none());
        assert_eq!(std::fs::read(&arquivo).unwrap(), bytes);
        assert_eq!(
            std::fs::metadata(&arquivo).unwrap().modified().unwrap(),
            modificado
        );
        assert_eq!(
            serde_json::to_value(s.listar_auditoria(100).unwrap()).unwrap(),
            auditoria
        );
        assert_eq!(
            serde_json::to_value(s.listar_arquivos(extracao_id, None, None, 0, 10).unwrap())
                .unwrap(),
            inventario
        );
        s.verificar_auditoria().unwrap();
    }

    #[test]
    fn documento_recusa_caso_trocado_e_id_fora_do_inventario() {
        let raiz = tempfile::tempdir().unwrap();
        let (mut s, caso_id, _, arquivo_id, _) = com_documento_sintetico(raiz.path());
        let erro = s.pedido_documento(i64::MAX, caso_id).err().unwrap();
        assert!(matches!(erro.codigo, CodigoErro::NaoEncontrado));
        let erro = s.pedido_documento(0, caso_id).err().unwrap();
        assert!(matches!(erro.codigo, CodigoErro::Validacao));

        let novo_caso_id = s.criar_caso(caso(), None).unwrap().caso.id;
        assert_ne!(novo_caso_id, caso_id);
        let erro = s.pedido_documento(arquivo_id, caso_id).err().unwrap();
        assert!(matches!(erro.codigo, CodigoErro::Conflito));
        let erro = s.pedido_documento(arquivo_id, novo_caso_id).err().unwrap();
        assert!(matches!(erro.codigo, CodigoErro::NaoEncontrado));
        s.fechar_caso().unwrap();
        let erro = s.pedido_documento(arquivo_id, caso_id).err().unwrap();
        assert!(matches!(erro.codigo, CodigoErro::NenhumCasoAberto));
    }

    #[test]
    fn documento_divergente_nao_publica_texto_sem_adulterar_a_fixture() {
        let raiz = tempfile::tempdir().unwrap();
        let (mut s, caso_id, _, arquivo_id, origem) = com_documento_sintetico(raiz.path());
        let bytes = std::fs::read(origem.join("SINTETICO.txt")).unwrap();
        let mut pedido = s.pedido_documento(arquivo_id, caso_id).unwrap();
        // Somente o pedido em memória é divergente; banco e origem continuam intactos.
        pedido.pedido.sha256 = Some("0".repeat(64));
        let resultado = pedido.executar();
        assert_eq!(
            resultado.estado,
            sied_documentos::EstadoDocumento::OrigemDivergente
        );
        assert!(resultado.texto.is_none());
        assert_eq!(std::fs::read(origem.join("SINTETICO.txt")).unwrap(), bytes);
        assert_eq!(
            s.pedido_documento(arquivo_id, caso_id)
                .unwrap()
                .executar()
                .estado,
            sied_documentos::EstadoDocumento::Pronto
        );
    }

    #[test]
    fn fonte_documental_recusa_outro_endereco_e_diretorio() {
        let raiz = tempfile::tempdir().unwrap();
        std::fs::create_dir(raiz.path().join("diretorio.txt")).unwrap();
        let caminho = CaminhoFonte::novo("diretorio.txt").unwrap();
        let fonte = FonteDocumento {
            pasta: raiz.path().to_path_buf(),
            caminho: caminho.clone(),
        };
        assert!(fonte.abrir(&caminho).is_err());
        assert!(fonte
            .abrir(&CaminhoFonte::novo("outro.txt").unwrap())
            .is_err());
    }

    #[test]
    fn fluxo_documental_indexa_pesquisa_reabre_sem_modificar_origem_ou_inventario() {
        let raiz = tempfile::tempdir().unwrap();
        let (mut s, caso_id, extracao_id, arquivo_id, origem) = com_documento_sintetico_texto(
            raiz.path(),
            "SINTÉTICO 🧪 pessoa@example.test 192.0.2.3 +5511987654321 pessoa@example.test",
        );
        let caminho = origem.join("SINTETICO.txt");
        let bytes = std::fs::read(&caminho).unwrap();
        let mtime = std::fs::metadata(&caminho).unwrap().modified().unwrap();
        let inventario =
            serde_json::to_value(s.listar_arquivos(extracao_id, None, None, 0, 10).unwrap())
                .unwrap();
        let controle = ControleDocumento::novo();
        let leitura = s
            .pedido_documento(arquivo_id, caso_id)
            .unwrap()
            .executar_controlado(None, &controle);
        let novo = crate::documentos::preparar_derivado(leitura, &controle).unwrap();
        let guardado = s.guardar_documento(novo.clone(), &controle).unwrap();
        let repetido = s.guardar_documento(novo, &controle).unwrap();
        assert_eq!(guardado.id, repetido.id);
        assert_eq!(guardado.texto.as_bytes(), bytes);
        assert_eq!(
            s.buscar_documentos(caso_id, "sintetico", 0, 20)
                .unwrap()
                .total,
            1
        );
        let candidatos = s.entidades_documento(caso_id, guardado.id).unwrap();
        assert_eq!(candidatos.len(), 4);
        assert_eq!(candidatos[0].literal, "pessoa@example.test");
        let confirmada = s
            .revisar_entidade(
                caso_id,
                candidatos[0].id,
                sied_armazenamento::documentos::EstadoRevisaoEntidade::Confirmado,
                Some("Ocorrência SINTÉTICA conferida; sem atribuição de identidade".into()),
            )
            .unwrap();
        assert_eq!(confirmada.literal, candidatos[0].literal);
        assert_eq!(
            s.revisoes_entidade(caso_id, candidatos[0].id, 0, 20)
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            serde_json::to_value(s.listar_arquivos(extracao_id, None, None, 0, 10).unwrap())
                .unwrap(),
            inventario
        );
        assert_eq!(std::fs::read(&caminho).unwrap(), bytes);
        assert_eq!(
            std::fs::metadata(&caminho).unwrap().modified().unwrap(),
            mtime
        );
        s.verificar_auditoria().unwrap();
        s.fechar_caso().unwrap();
        s.abrir_caso(caso_id).unwrap();
        assert_eq!(
            s.obter_documento(caso_id, guardado.id)
                .unwrap()
                .sha256_texto,
            guardado.sha256_texto
        );
    }

    #[test]
    fn indexacao_cancelada_ou_caso_trocado_nao_grava_snapshot() {
        let raiz = tempfile::tempdir().unwrap();
        let (mut s, caso_id, _, arquivo_id, _) = com_documento_sintetico(raiz.path());
        let controle = ControleDocumento::novo();
        let leitura = s
            .pedido_documento(arquivo_id, caso_id)
            .unwrap()
            .executar_controlado(None, &controle);
        let novo = crate::documentos::preparar_derivado(leitura, &controle).unwrap();
        controle.cancelar();
        assert!(s.guardar_documento(novo.clone(), &controle).is_err());
        assert_eq!(s.buscar_documentos(caso_id, "", 0, 20).unwrap().total, 0);
        let outro = s.criar_caso(caso(), None).unwrap().caso.id;
        assert!(s
            .guardar_documento(novo, &ControleDocumento::novo())
            .is_err());
        assert!(s.buscar_documentos(caso_id, "", 0, 20).is_err());
        assert_eq!(s.buscar_documentos(outro, "", 0, 20).unwrap().total, 0);
        s.verificar_auditoria().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn fonte_documental_recusa_links_intermediarios_e_no_arquivo_final() {
        use std::os::unix::fs::symlink;

        let raiz = tempfile::tempdir().unwrap();
        let origem = raiz.path().join("origem");
        let externa = raiz.path().join("externa");
        std::fs::create_dir(&origem).unwrap();
        std::fs::create_dir(&externa).unwrap();
        std::fs::write(externa.join("SINTETICO.txt"), "conteúdo SINTÉTICO externo").unwrap();
        symlink(&externa, origem.join("subpasta")).unwrap();
        symlink(externa.join("SINTETICO.txt"), origem.join("final.txt")).unwrap();
        for relativo in ["subpasta/SINTETICO.txt", "final.txt"] {
            let caminho = CaminhoFonte::novo(relativo).unwrap();
            let fonte = FonteDocumento {
                pasta: origem.clone(),
                caminho: caminho.clone(),
            };
            assert!(fonte.abrir(&caminho).is_err(), "{relativo}");
        }
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
