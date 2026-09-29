//! Erro devolvido ao frontend: sempre com código estável e mensagem em português.

use serde::Serialize;
use sied_armazenamento::ErroArmazenamento;
use sied_dominio::ErroCampo;
use sied_ingestao::ErroIngestao;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CodigoErro {
    Validacao,
    NaoEncontrado,
    NenhumCasoAberto,
    Conflito,
    VersaoIncompativel,
    Integridade,
    /// Recurso ainda não implementado (ex.: importador do Avilla).
    NaoSuportado,
    Interno,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ErroApp {
    pub codigo: CodigoErro,
    pub mensagem: String,
    /// Erros por campo, quando `codigo` = validacao.
    pub campos: Vec<ErroCampo>,
}

impl ErroApp {
    pub fn novo(codigo: CodigoErro, mensagem: impl Into<String>) -> Self {
        ErroApp {
            codigo,
            mensagem: mensagem.into(),
            campos: Vec::new(),
        }
    }

    pub fn nenhum_caso() -> Self {
        Self::novo(CodigoErro::NenhumCasoAberto, "nenhum caso aberto")
    }
}

impl From<ErroArmazenamento> for ErroApp {
    fn from(e: ErroArmazenamento) -> Self {
        use ErroArmazenamento as E;
        let codigo = match &e {
            E::Validacao(v) => {
                return ErroApp {
                    codigo: CodigoErro::Validacao,
                    mensagem: "corrija os campos destacados".into(),
                    campos: v.erros.clone(),
                }
            }
            E::CasoNaoEncontrado(_) | E::NaoEncontrado(_) => CodigoErro::NaoEncontrado,
            E::OrigemInvalida(_) => CodigoErro::Validacao,
            E::CasoJaExiste(_) | E::PastaNaoVazia(_) | E::Recusado(_) => CodigoErro::Conflito,
            E::VersaoMaisNova { .. }
            | E::MigracaoDivergente { .. }
            | E::TipoBancoIncorreto { .. } => CodigoErro::VersaoIncompativel,
            E::AuditoriaCorrompida { .. } | E::Inconsistente(_) => CodigoErro::Integridade,
            E::Sqlite(_) | E::Io(_) => CodigoErro::Interno,
        };
        match codigo {
            CodigoErro::Interno => tracing::error!(erro = %e, "falha de armazenamento"),
            _ => tracing::warn!(erro = %e, "operação recusada"),
        }
        ErroApp::novo(codigo, e.to_string())
    }
}

impl From<ErroIngestao> for ErroApp {
    fn from(e: ErroIngestao) -> Self {
        use ErroIngestao as E;
        let codigo = match &e {
            E::NaoImplementado { .. } => CodigoErro::NaoSuportado,
            E::CaminhoInvalido(_) | E::FormatoNaoReconhecido(_) | E::ItemIgnorado { .. } => {
                CodigoErro::Validacao
            }
            E::Cancelado => CodigoErro::Conflito,
            E::OrigemAlterada { .. } => CodigoErro::Integridade,
            E::Io(_) | E::Sqlite(_) | E::Parser { .. } | E::Gravacao(_) => CodigoErro::Interno,
        };
        tracing::warn!(erro = %e, "falha de ingestão");
        ErroApp::novo(codigo, e.to_string())
    }
}

impl From<sied_relatorio::ErroRelatorio> for ErroApp {
    fn from(e: sied_relatorio::ErroRelatorio) -> Self {
        use sied_relatorio::ErroRelatorio as E;
        match e {
            E::Armazenamento(e) => e.into(),
            E::Auditoria(_) => {
                tracing::error!(erro = %e, "relatório recusado");
                ErroApp::novo(CodigoErro::Integridade, e.to_string())
            }
            E::Typst(_) | E::Io(_) => {
                tracing::error!(erro = %e, "falha ao gerar o relatório");
                ErroApp::novo(CodigoErro::Interno, e.to_string())
            }
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for ErroApp {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        tracing::error!("estado interno bloqueado após falha anterior");
        ErroApp::novo(
            CodigoErro::Interno,
            "estado interno inconsistente; reinicie o SIED",
        )
    }
}

pub type ResultadoApp<T> = Result<T, ErroApp>;
