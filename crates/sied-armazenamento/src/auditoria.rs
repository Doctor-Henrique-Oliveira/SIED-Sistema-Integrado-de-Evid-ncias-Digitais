//! Trilha de auditoria encadeada por hash.
//!
//! Cada registro guarda o SHA-256 do anterior. Alterar ou apagar um registro
//! (mesmo fora do SIED) quebra a cadeia, e [`verificar`] aponta onde.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::{ErroArmazenamento, Resultado, Sessao};

pub const HASH_GENESE: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RegistroAuditoria {
    #[ts(type = "number")]
    pub seq: i64,
    pub ts: String,
    pub usuario: String,
    pub maquina: String,
    pub acao: String,
    #[ts(type = "unknown")]
    pub detalhes: Value,
    pub hash_anterior: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct VerificacaoAuditoria {
    #[ts(type = "number")]
    pub registros: i64,
    pub hash_final: String,
}

fn calcular_hash(
    seq: i64,
    ts: &str,
    usuario: &str,
    maquina: &str,
    acao: &str,
    detalhes: &str,
    anterior: &str,
) -> String {
    // \x1f (separador de unidade) evita ambiguidade entre campos.
    let conteudo =
        format!("{seq}\x1f{ts}\x1f{usuario}\x1f{maquina}\x1f{acao}\x1f{detalhes}\x1f{anterior}");
    hex::encode(Sha256::digest(conteudo.as_bytes()))
}

/// Grava um registro. Chame dentro da mesma transação da ação auditada.
pub fn registrar(
    conn: &Connection,
    sessao: &Sessao,
    acao: &str,
    detalhes: Value,
) -> Resultado<RegistroAuditoria> {
    let ultimo: Option<(i64, String)> = conn
        .query_row(
            "SELECT seq, hash FROM auditoria ORDER BY seq DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (seq, anterior) = match ultimo {
        Some((s, h)) => (s + 1, h),
        None => (1, HASH_GENESE.to_string()),
    };
    let ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let detalhes_txt = serde_json::to_string(&detalhes).expect("JSON serializável");
    let hash = calcular_hash(
        seq,
        &ts,
        &sessao.usuario,
        &sessao.maquina,
        acao,
        &detalhes_txt,
        &anterior,
    );
    conn.execute(
        "INSERT INTO auditoria(seq, ts, usuario, maquina, acao, detalhes, hash_anterior, hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            seq,
            ts,
            sessao.usuario,
            sessao.maquina,
            acao,
            detalhes_txt,
            anterior,
            hash
        ],
    )?;
    tracing::debug!(seq, acao, "auditoria registrada");
    Ok(RegistroAuditoria {
        seq,
        ts,
        usuario: sessao.usuario.clone(),
        maquina: sessao.maquina.clone(),
        acao: acao.into(),
        detalhes,
        hash_anterior: anterior,
        hash,
    })
}

/// Registros mais recentes primeiro.
pub fn listar(conn: &Connection, limite: i64) -> Resultado<Vec<RegistroAuditoria>> {
    let mut stmt = conn.prepare(
        "SELECT seq, ts, usuario, maquina, acao, detalhes, hash_anterior, hash
         FROM auditoria ORDER BY seq DESC LIMIT ?1",
    )?;
    let linhas = stmt.query_map([limite], |r| {
        let detalhes: String = r.get(5)?;
        Ok(RegistroAuditoria {
            seq: r.get(0)?,
            ts: r.get(1)?,
            usuario: r.get(2)?,
            maquina: r.get(3)?,
            acao: r.get(4)?,
            detalhes: serde_json::from_str(&detalhes).unwrap_or(Value::String(detalhes)),
            hash_anterior: r.get(6)?,
            hash: r.get(7)?,
        })
    })?;
    Ok(linhas.collect::<Result<_, _>>()?)
}

/// Recalcula toda a cadeia e falha no primeiro registro inconsistente.
pub fn verificar(conn: &Connection) -> Resultado<VerificacaoAuditoria> {
    let mut stmt = conn
        .prepare("SELECT seq, ts, usuario, maquina, acao, detalhes, hash_anterior, hash FROM auditoria ORDER BY seq")?;
    let mut linhas = stmt.query([])?;
    let mut anterior_esperado = HASH_GENESE.to_string();
    let mut seq_esperado = 1i64;
    while let Some(r) = linhas.next()? {
        let seq: i64 = r.get(0)?;
        let ts: String = r.get(1)?;
        let usuario: String = r.get(2)?;
        let maquina: String = r.get(3)?;
        let acao: String = r.get(4)?;
        let detalhes: String = r.get(5)?;
        let anterior: String = r.get(6)?;
        let hash: String = r.get(7)?;
        let falha = |motivo: &str| ErroArmazenamento::AuditoriaCorrompida {
            seq,
            motivo: motivo.into(),
        };
        if seq != seq_esperado {
            return Err(falha(&format!(
                "sequência esperada {seq_esperado} (registro ausente)"
            )));
        }
        if anterior != anterior_esperado {
            return Err(falha("hash anterior não confere"));
        }
        if calcular_hash(seq, &ts, &usuario, &maquina, &acao, &detalhes, &anterior) != hash {
            return Err(falha("conteúdo alterado"));
        }
        anterior_esperado = hash;
        seq_esperado += 1;
    }
    Ok(VerificacaoAuditoria {
        registros: seq_esperado - 1,
        hash_final: anterior_esperado,
    })
}
