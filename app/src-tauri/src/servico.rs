//! Regras do aplicativo, independentes do Tauri (testáveis sem janela).
//! Os comandos Tauri apenas travam o estado e chamam estes métodos.

use serde::Serialize;
use sied_armazenamento::auditoria::{RegistroAuditoria, VerificacaoAuditoria};
use sied_armazenamento::caso::{nome_pasta_sugerido, NOME_ARQUIVO_CASO};
use sied_armazenamento::{BancoCaso, CasoCatalogado, CatalogoGlobal, Sessao};
use sied_dominio::{Caso, CasoId, Dispositivo, NovoCaso, NovoDispositivo};
use std::path::{Path, PathBuf};
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

pub struct Servico {
    catalogo: CatalogoGlobal,
    caso: Option<BancoCaso>,
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
