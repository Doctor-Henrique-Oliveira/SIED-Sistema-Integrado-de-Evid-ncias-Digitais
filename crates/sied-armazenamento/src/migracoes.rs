//! Migrações versionadas dos bancos do SIED.
//!
//! - Cada tipo de banco tem um `application_id` próprio (cabeçalho do SQLite), para
//!   que um banco global nunca seja aberto como banco de caso e vice-versa.
//! - Migrações são arquivos SQL embutidos no executável, aplicados em ordem, cada um
//!   numa transação, e registrados em `schema_migracao` com o SHA-256 do SQL.
//! - Um banco mais novo que o SIED instalado é recusado (evita que uma versão antiga
//!   grave num esquema que não conhece).
//! - Antes de migrar um banco que já tem dados, é feita uma cópia de segurança
//!   (`<arquivo>.antes-v<N>.bak`).

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::{ErroArmazenamento, Resultado};

pub struct Migracao {
    pub versao: i64,
    pub nome: &'static str,
    pub sql: &'static str,
}

pub struct TipoBanco {
    pub nome: &'static str,
    pub application_id: i32,
    pub migracoes: &'static [Migracao],
}

impl TipoBanco {
    pub fn versao_atual(&self) -> i64 {
        self.migracoes.last().map(|m| m.versao).unwrap_or(0)
    }
}

pub const BANCO_GLOBAL: TipoBanco = TipoBanco {
    nome: "global",
    application_id: 0x5345_4447, // "SEDG"
    migracoes: &[Migracao {
        versao: 1,
        nome: "inicial",
        sql: include_str!("../migracoes/global/0001_inicial.sql"),
    }],
};

pub const BANCO_CASO: TipoBanco = TipoBanco {
    nome: "caso",
    application_id: 0x5345_4443, // "SEDC"
    migracoes: &[Migracao {
        versao: 1,
        nome: "inicial",
        sql: include_str!("../migracoes/caso/0001_inicial.sql"),
    }],
};

const TABELA_CONTROLE: &str = "CREATE TABLE IF NOT EXISTS schema_migracao (
    versao       INTEGER PRIMARY KEY,
    nome         TEXT NOT NULL,
    sha256       TEXT NOT NULL,
    aplicada_em  TEXT NOT NULL,
    versao_sied  TEXT NOT NULL
)";

fn sha256(sql: &str) -> String {
    hex::encode(Sha256::digest(sql.as_bytes()))
}

fn banco_vazio(conn: &Connection) -> Resultado<bool> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM sqlite_schema", [], |r| r.get(0))?;
    Ok(n == 0)
}

/// Confere o tipo do banco e aplica as migrações pendentes. Devolve a versão final.
pub fn preparar(conn: &mut Connection, caminho: &Path, tipo: &TipoBanco) -> Resultado<i64> {
    let app_id: i32 = conn.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    if app_id == 0 {
        if !banco_vazio(conn)? {
            return Err(ErroArmazenamento::TipoBancoIncorreto {
                caminho: caminho.into(),
                esperado: tipo.nome,
            });
        }
        conn.pragma_update(None, "application_id", tipo.application_id)?;
    } else if app_id != tipo.application_id {
        return Err(ErroArmazenamento::TipoBancoIncorreto {
            caminho: caminho.into(),
            esperado: tipo.nome,
        });
    }

    conn.execute(TABELA_CONTROLE, [])?;
    let atual: i64 = conn.query_row(
        "SELECT COALESCE(MAX(versao), 0) FROM schema_migracao",
        [],
        |r| r.get(0),
    )?;
    if atual > tipo.versao_atual() {
        return Err(ErroArmazenamento::VersaoMaisNova {
            tipo: tipo.nome,
            encontrada: atual,
            suportada: tipo.versao_atual(),
        });
    }

    // Migrações já aplicadas precisam ser idênticas às embutidas.
    for m in tipo.migracoes.iter().filter(|m| m.versao <= atual) {
        let gravado: Option<String> = conn
            .query_row(
                "SELECT sha256 FROM schema_migracao WHERE versao = ?1",
                [m.versao],
                |r| r.get(0),
            )
            .optional()?;
        if gravado.as_deref() != Some(sha256(m.sql).as_str()) {
            return Err(ErroArmazenamento::MigracaoDivergente {
                tipo: tipo.nome,
                versao: m.versao,
            });
        }
    }

    let pendentes: Vec<&Migracao> = tipo.migracoes.iter().filter(|m| m.versao > atual).collect();
    if pendentes.is_empty() {
        return Ok(atual);
    }

    if atual > 0 {
        let copia = caminho.with_file_name(format!(
            "{}.antes-v{}.bak",
            caminho
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("banco"),
            pendentes[0].versao
        ));
        tracing::info!(banco = tipo.nome, copia = %copia.display(), "cópia de segurança antes da migração");
        conn.execute("VACUUM INTO ?1", [copia.to_string_lossy()])?;
    }

    for m in pendentes {
        let tx = conn.transaction()?;
        tx.execute_batch(m.sql)?;
        tx.execute(
            "INSERT INTO schema_migracao(versao, nome, sha256, aplicada_em, versao_sied) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![m.versao, m.nome, sha256(m.sql), Utc::now().to_rfc3339(), env!("CARGO_PKG_VERSION")],
        )?;
        tx.pragma_update(None, "user_version", m.versao)?;
        tx.commit()?;
        tracing::info!(
            banco = tipo.nome,
            versao = m.versao,
            nome = m.nome,
            "migração aplicada"
        );
    }
    Ok(tipo.versao_atual())
}
