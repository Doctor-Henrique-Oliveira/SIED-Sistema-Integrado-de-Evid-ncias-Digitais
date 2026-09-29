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

/// Falhas do SQLite que o analista consegue entender e resolver, em português.
/// O texto técnico original continua no log.
fn traduzir_sqlite(e: &rusqlite::Error) -> Option<(CodigoErro, &'static str)> {
    use rusqlite::ErrorCode as C;
    Some(match e.sqlite_error_code()? {
        C::DatabaseCorrupt | C::NotADatabase => (
            CodigoErro::Integridade,
            "o banco de dados está corrompido ou não é um banco do SIED; restaure uma cópia do caso",
        ),
        C::DiskFull => (
            CodigoErro::Interno,
            "não há espaço livre no disco da pasta do caso; libere espaço e tente de novo",
        ),
        C::ReadOnly | C::PermissionDenied | C::CannotOpen => (
            CodigoErro::Interno,
            "sem permissão para gravar na pasta do caso (somente leitura, sincronização ou antivírus bloqueando); confira a pasta e tente de novo",
        ),
        C::DatabaseBusy | C::DatabaseLocked => (
            CodigoErro::Conflito,
            "o banco do caso está ocupado por outro programa; feche-o e tente de novo",
        ),
        C::SystemIoFailure => (
            CodigoErro::Interno,
            "falha de leitura ou gravação no disco da pasta do caso (disco removido ou com defeito?)",
        ),
        _ => return None,
    })
}

impl From<ErroArmazenamento> for ErroApp {
    fn from(e: ErroArmazenamento) -> Self {
        use ErroArmazenamento as E;
        if let E::Sqlite(s) = &e {
            if let Some((codigo, mensagem)) = traduzir_sqlite(s) {
                tracing::error!(erro = %e, "falha de armazenamento");
                return ErroApp::novo(codigo, mensagem);
            }
        }
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
            E::CasoJaExiste(_) | E::PastaNaoVazia(_) | E::CasoEmUso(_) | E::Recusado(_) => {
                CodigoErro::Conflito
            }
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

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn falhas_do_sqlite_viram_mensagens_em_portugues() {
        let falha = |codigo: i32| {
            ErroApp::from(ErroArmazenamento::Sqlite(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(codigo),
                Some("mensagem técnica em inglês".into()),
            )))
        };
        let e = falha(rusqlite::ffi::SQLITE_CORRUPT);
        assert!(matches!(e.codigo, CodigoErro::Integridade));
        assert!(e.mensagem.contains("corrompido"));
        let e = falha(rusqlite::ffi::SQLITE_FULL);
        assert!(e.mensagem.contains("espaço"));
        let e = falha(rusqlite::ffi::SQLITE_READONLY);
        assert!(e.mensagem.contains("permissão"));
        let e = falha(rusqlite::ffi::SQLITE_BUSY);
        assert!(matches!(e.codigo, CodigoErro::Conflito));
        // Outros erros seguem com o texto original.
        let e = falha(rusqlite::ffi::SQLITE_CONSTRAINT);
        assert!(matches!(e.codigo, CodigoErro::Interno));
        assert!(e.mensagem.contains("inglês"), "{}", e.mensagem);
    }
}
