use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::time::Duration;

use crate::migracoes::TipoBanco;
use crate::{ErroArmazenamento, Resultado};

/// Se o arquivo pode ser criado (banco novo) ou precisa existir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Abertura {
    Criar,
    Existente,
}

/// Arquivo que não é banco SQLite (truncado, corrompido, outro formato).
pub(crate) fn corrompido(e: rusqlite::Error, caminho: &Path) -> ErroArmazenamento {
    use rusqlite::ErrorCode as C;
    match e.sqlite_error_code() {
        Some(C::NotADatabase | C::DatabaseCorrupt) => ErroArmazenamento::Inconsistente(format!(
            "o arquivo {} está corrompido ou não é um banco SQLite",
            caminho.display()
        )),
        _ => e.into(),
    }
}

/// [`corrompido`] para erros já convertidos (ex.: vindos das migrações).
pub(crate) fn corrompido_em(e: ErroArmazenamento, caminho: &Path) -> ErroArmazenamento {
    match e {
        ErroArmazenamento::Sqlite(s) => corrompido(s, caminho),
        outro => outro,
    }
}

/// Abre um banco de trabalho do SIED (nunca um banco da evidência) com as
/// configurações de durabilidade usadas em todo o projeto.
///
/// O tipo do banco (`application_id`) é conferido **antes** de qualquer escrita: um
/// arquivo que não é do SIED não ganha WAL nem cabeçalho novo. Um arquivo vazio só é
/// aceito quando o banco está sendo criado.
pub(crate) fn abrir(caminho: &Path, tipo: &TipoBanco, abertura: Abertura) -> Resultado<Connection> {
    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if abertura == Abertura::Criar {
        flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }
    let conn = Connection::open_with_flags(caminho, flags)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    let app_id: i32 = conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .map_err(|e| corrompido(e, caminho))?;
    if app_id == 0 {
        let objetos: i64 = conn
            .query_row("SELECT COUNT(*) FROM sqlite_schema", [], |r| r.get(0))
            .map_err(|e| corrompido(e, caminho))?;
        if objetos > 0 {
            return Err(ErroArmazenamento::TipoBancoIncorreto {
                caminho: caminho.into(),
                esperado: tipo.nome,
            });
        }
        if abertura == Abertura::Existente {
            return Err(ErroArmazenamento::Inconsistente(format!(
                "o arquivo {} está vazio",
                caminho.display()
            )));
        }
    } else if app_id != tipo.application_id {
        return Err(ErroArmazenamento::TipoBancoIncorreto {
            caminho: caminho.into(),
            esperado: tipo.nome,
        });
    }
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    // FULL: cada transação confirmada sobrevive a queda de energia.
    conn.pragma_update(None, "synchronous", "FULL")?;
    Ok(conn)
}
