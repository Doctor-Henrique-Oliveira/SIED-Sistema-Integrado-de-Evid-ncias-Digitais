//! Utilidades comuns aos parsers de bancos SQLite.

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, Row};
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::Resultado;

/// Abre a cópia de trabalho somente para leitura. Um `-wal` copiado ao lado é lido
/// pelo SQLite normalmente (registros ainda não consolidados no banco principal).
pub fn abrir_copia(caminho: &Path) -> Resultado<Connection> {
    let conn = Connection::open_with_flags(
        caminho,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    Ok(conn)
}

pub fn colunas(conn: &Connection, tabela: &str) -> Resultado<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM pragma_table_info(?1)")?;
    let nomes = stmt.query_map([tabela], |r| r.get::<_, String>(0))?;
    Ok(nomes.collect::<Result<_, _>>()?)
}

/// Valor de uma coluna exatamente como está no banco. BLOBs viram
/// `{"blob_hex": "..."}` para não se confundirem com texto.
pub fn valor_bruto(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(t) => match std::str::from_utf8(t) {
            Ok(s) => json!(s),
            Err(_) => json!({ "texto_invalido_hex": hex::encode(t) }),
        },
        ValueRef::Blob(b) => json!({ "blob_hex": hex::encode(b) }),
    }
}

/// A linha inteira, coluna a coluna.
pub fn linha_bruta(r: &Row<'_>, colunas: &[String]) -> rusqlite::Result<Map<String, Value>> {
    let mut m = Map::new();
    for (i, c) in colunas.iter().enumerate() {
        m.insert(c.clone(), valor_bruto(r.get_ref(i)?));
    }
    Ok(m)
}
