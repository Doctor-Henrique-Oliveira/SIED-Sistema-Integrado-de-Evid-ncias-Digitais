//! Trilha encadeada das ações da estação (providers, permissões e cópias).
//!
//! Ela fica separada da trilha do caso: configurações da estação não são evidência
//! nem devem aparecer como se tivessem sido realizadas dentro de um caso.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::auditoria::{RegistroAuditoria, VerificacaoAuditoria};
use crate::{Resultado, Sessao};

const HASH_GENESE: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn hash(
    seq: i64,
    ts: &str,
    usuario: &str,
    maquina: &str,
    acao: &str,
    detalhes: &str,
    anterior: &str,
) -> String {
    let texto =
        format!("{seq}\x1f{ts}\x1f{usuario}\x1f{maquina}\x1f{acao}\x1f{detalhes}\x1f{anterior}");
    hex::encode(Sha256::digest(texto.as_bytes()))
}

pub fn registrar(
    conn: &Connection,
    sessao: &Sessao,
    acao: &str,
    detalhes: Value,
) -> Resultado<RegistroAuditoria> {
    let ultimo: Option<(i64, String)> = conn
        .query_row(
            "SELECT seq, hash FROM auditoria_estacao ORDER BY seq DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (seq, anterior) = ultimo
        .map(|(seq, hash)| (seq + 1, hash))
        .unwrap_or_else(|| (1, HASH_GENESE.into()));
    let ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let detalhes_txt = serde_json::to_string(&detalhes).expect("JSON serializável");
    let atual = hash(
        seq,
        &ts,
        &sessao.usuario,
        &sessao.maquina,
        acao,
        &detalhes_txt,
        &anterior,
    );
    conn.execute(
        "INSERT INTO auditoria_estacao
         (seq, ts, usuario, maquina, acao, detalhes, hash_anterior, hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            seq,
            ts,
            sessao.usuario,
            sessao.maquina,
            acao,
            detalhes_txt,
            anterior,
            atual
        ],
    )?;
    Ok(RegistroAuditoria {
        seq,
        ts,
        usuario: sessao.usuario.clone(),
        maquina: sessao.maquina.clone(),
        acao: acao.into(),
        detalhes,
        hash_anterior: anterior,
        hash: atual,
    })
}

pub fn listar(conn: &Connection, limite: i64) -> Resultado<Vec<RegistroAuditoria>> {
    let mut stmt = conn.prepare(
        "SELECT seq, ts, usuario, maquina, acao, detalhes, hash_anterior, hash
         FROM auditoria_estacao ORDER BY seq DESC LIMIT ?1",
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

pub fn verificar(conn: &Connection) -> Resultado<VerificacaoAuditoria> {
    let mut stmt = conn.prepare(
        "SELECT seq, ts, usuario, maquina, acao, detalhes, hash_anterior, hash
         FROM auditoria_estacao ORDER BY seq",
    )?;
    let mut linhas = stmt.query([])?;
    let mut anterior_esperado = HASH_GENESE.to_string();
    let mut seq_esperado = 1;
    while let Some(r) = linhas.next()? {
        let seq: i64 = r.get(0)?;
        let ts: String = r.get(1)?;
        let usuario: String = r.get(2)?;
        let maquina: String = r.get(3)?;
        let acao: String = r.get(4)?;
        let detalhes: String = r.get(5)?;
        let anterior: String = r.get(6)?;
        let atual: String = r.get(7)?;
        if seq != seq_esperado {
            return Err(crate::ErroArmazenamento::AuditoriaCorrompida {
                seq,
                motivo: format!("sequência esperada {seq_esperado}"),
            });
        }
        if anterior != anterior_esperado {
            return Err(crate::ErroArmazenamento::AuditoriaCorrompida {
                seq,
                motivo: "hash anterior não confere na trilha da estação".into(),
            });
        }
        if hash(seq, &ts, &usuario, &maquina, &acao, &detalhes, &anterior) != atual {
            return Err(crate::ErroArmazenamento::AuditoriaCorrompida {
                seq,
                motivo: "conteúdo alterado na trilha da estação".into(),
            });
        }
        anterior_esperado = atual;
        seq_esperado += 1;
    }
    Ok(VerificacaoAuditoria {
        registros: seq_esperado - 1,
        hash_final: anterior_esperado,
    })
}
