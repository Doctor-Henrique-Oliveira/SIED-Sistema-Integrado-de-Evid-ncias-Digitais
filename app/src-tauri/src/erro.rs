//! Erro devolvido ao frontend: sempre com código estável e mensagem em português.

use serde::Serialize;
use sied_armazenamento::ErroArmazenamento;
use sied_dominio::ErroCampo;
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
            E::CasoNaoEncontrado(_) => CodigoErro::NaoEncontrado,
            E::CasoJaExiste(_) | E::PastaNaoVazia(_) => CodigoErro::Conflito,
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
