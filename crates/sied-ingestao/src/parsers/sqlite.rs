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
/// `{"blob_hex": "..."}` para não se confundirem com texto; um REAL infinito (o SQLite
/// aceita `9e999`), que o JSON não representa e viraria `null`, vira
/// `{"real_nao_finito": "inf"}` ou `"-inf"`.
pub fn valor_bruto(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) if !f.is_finite() => json!({ "real_nao_finito": if f.is_nan() {
            "nan"
        } else if f > 0.0 {
            "inf"
        } else {
            "-inf"
        } }),
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

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn valores_que_o_json_nao_representa_ficam_explicitos() {
        assert_eq!(
            valor_bruto(ValueRef::Real(f64::INFINITY)),
            json!({ "real_nao_finito": "inf" })
        );
        assert_eq!(
            valor_bruto(ValueRef::Real(f64::NEG_INFINITY)),
            json!({ "real_nao_finito": "-inf" })
        );
        assert_eq!(valor_bruto(ValueRef::Real(-0.0)), json!(-0.0));
        assert_eq!(
            valor_bruto(ValueRef::Blob(&[0, 255])),
            json!({ "blob_hex": "00ff" })
        );
        assert_eq!(
            valor_bruto(ValueRef::Text(b"\xC3\x28")),
            json!({ "texto_invalido_hex": "c328" })
        );
    }
}
