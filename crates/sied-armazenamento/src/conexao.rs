use rusqlite::functions::FunctionFlags;
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

/// Forma de comparação dos filtros de texto (conversas, arquivos, identificadores):
/// minúsculas e sem acentos, como a busca geral (FTS5 com `remove_diacritics`).
/// O `lower()` do SQLite só troca letras ASCII: "JOSÉ" virava "josÉ" e não casava.
pub fn normalizar_busca(t: &str) -> String {
    use unicode_normalization::char::is_combining_mark;
    use unicode_normalization::UnicodeNormalization;
    if t.is_ascii() {
        return t.to_ascii_lowercase();
    }
    t.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Só os dígitos ASCII de um texto (telefone com ou sem máscara). Vazio vira `None`.
pub fn so_digitos(t: &str) -> Option<String> {
    let d: String = t.chars().filter(char::is_ascii_digit).collect();
    (!d.is_empty()).then_some(d)
}

/// Funções usadas só em consultas, nunca no esquema (gatilhos, índices, visões): o
/// banco do caso continua legível por qualquer ferramenta SQLite.
///
/// - `sied_normal(x)`: [`normalizar_busca`];
/// - `sied_digitos(x)`: [`so_digitos`] (NULL quando não há dígitos).
fn registrar_funcoes(conn: &Connection) -> rusqlite::Result<()> {
    conn.create_scalar_function(
        "sied_digitos",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            Ok(match ctx.get_raw(0) {
                rusqlite::types::ValueRef::Text(t) => so_digitos(&String::from_utf8_lossy(t)),
                rusqlite::types::ValueRef::Integer(i) => so_digitos(&i.to_string()),
                _ => None,
            })
        },
    )?;
    conn.create_scalar_function(
        "sied_normal",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            Ok(match ctx.get_raw(0) {
                rusqlite::types::ValueRef::Text(t) => {
                    Some(normalizar_busca(&String::from_utf8_lossy(t)))
                }
                rusqlite::types::ValueRef::Null => None,
                outro => Some(normalizar_busca(&match outro {
                    rusqlite::types::ValueRef::Integer(i) => i.to_string(),
                    rusqlite::types::ValueRef::Real(f) => f.to_string(),
                    _ => String::new(),
                })),
            })
        },
    )
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
    registrar_funcoes(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod testes {
    use super::{normalizar_busca, so_digitos};

    #[test]
    fn so_os_digitos() {
        assert_eq!(
            so_digitos("+55 (11) 90000-0011").as_deref(),
            Some("5511900000011")
        );
        assert_eq!(so_digitos("sem número"), None);
    }

    #[test]
    fn minusculas_sem_acento() {
        assert_eq!(normalizar_busca("JOSÉ Ção Ü ñ"), "jose cao u n");
        assert_eq!(normalizar_busca("ÁRVORE"), normalizar_busca("arvore"));
        assert_eq!(normalizar_busca("+55 (11) 9"), "+55 (11) 9");
        assert_eq!(normalizar_busca("Mensagem ABC"), "mensagem abc");
    }
}
