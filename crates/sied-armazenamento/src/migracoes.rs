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
use std::path::{Path, PathBuf};

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
    migracoes: &[
        Migracao {
            versao: 1,
            nome: "inicial",
            sql: include_str!("../migracoes/caso/0001_inicial.sql"),
        },
        Migracao {
            versao: 2,
            nome: "importacao",
            sql: include_str!("../migracoes/caso/0002_importacao.sql"),
        },
        Migracao {
            versao: 3,
            nome: "conversas_busca",
            sql: include_str!("../migracoes/caso/0003_conversas_busca.sql"),
        },
        Migracao {
            versao: 4,
            nome: "whatsapp",
            sql: include_str!("../migracoes/caso/0004_whatsapp.sql"),
        },
        Migracao {
            versao: 5,
            nome: "evidencias_relatorio",
            sql: include_str!("../migracoes/caso/0005_evidencias_relatorio.sql"),
        },
        Migracao {
            versao: 6,
            nome: "ordem_conversa",
            sql: include_str!("../migracoes/caso/0006_ordem_conversa.sql"),
        },
        Migracao {
            versao: 7,
            nome: "pesquisa_local",
            sql: include_str!("../migracoes/caso/0007_pesquisa_local.sql"),
        },
        Migracao {
            versao: 8,
            nome: "documentos_derivados",
            sql: include_str!("../migracoes/caso/0008_documentos_derivados.sql"),
        },
        Migracao {
            versao: 9,
            nome: "supervisao_local",
            sql: include_str!("../migracoes/caso/0009_supervisao_local.sql"),
        },
        Migracao {
            versao: 10,
            nome: "similaridade_visual",
            sql: include_str!("../migracoes/caso/0010_similaridade_visual.sql"),
        },
        Migracao {
            versao: 11,
            nome: "ocr_documental",
            sql: include_str!("../migracoes/caso/0011_ocr_documental.sql"),
        },
    ],
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

/// `<arquivo>.antes-v<N>.bak`, ou `-2`, `-3`... se já existir: uma cópia anterior
/// (ex.: de uma migração que falhou no meio) nunca é sobrescrita, e o `VACUUM INTO`
/// recusaria gravar sobre ela, impedindo o caso de abrir.
fn caminho_copia(caminho: &Path, versao: i64) -> PathBuf {
    let base = caminho
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("banco");
    (1..)
        .map(|n| {
            caminho.with_file_name(if n == 1 {
                format!("{base}.antes-v{versao}.bak")
            } else {
                format!("{base}.antes-v{versao}-{n}.bak")
            })
        })
        .find(|c| !c.exists())
        .expect("sequência infinita de nomes")
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
        let copia = caminho_copia(caminho, pendentes[0].versao);
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
