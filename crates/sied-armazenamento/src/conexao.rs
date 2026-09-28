use rusqlite::Connection;
use std::path::Path;
use std::time::Duration;

use crate::Resultado;

/// Abre um banco de trabalho do SIED (nunca um banco da evidência) com as
/// configurações de durabilidade usadas em todo o projeto.
pub(crate) fn abrir(caminho: &Path) -> Resultado<Connection> {
    let conn = Connection::open(caminho)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    // FULL: cada transação confirmada sobrevive a queda de energia.
    conn.pragma_update(None, "synchronous", "FULL")?;
    Ok(conn)
}
