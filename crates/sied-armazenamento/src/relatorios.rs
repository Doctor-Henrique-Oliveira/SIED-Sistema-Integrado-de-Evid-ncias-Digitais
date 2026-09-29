//! Registro dos relatórios gerados (o PDF fica em `<pasta do caso>/relatorios/`).

use chrono::Utc;
use rusqlite::{params, Connection, Transaction};
use serde::Serialize;
use serde_json::{json, Value};
use ts_rs::TS;

use crate::auditoria;
use crate::{Resultado, Sessao};

pub const PASTA_RELATORIOS: &str = "relatorios";

/// Um relatório já gerado.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RelatorioRegistrado {
    #[ts(type = "number")]
    pub id: i64,
    pub modelo: String,
    #[ts(type = "unknown")]
    pub secoes: Value,
    #[ts(type = "unknown")]
    pub evidencias: Value,
    pub gerado_por: String,
    pub maquina: String,
    pub gerado_em: String,
    /// Relativo à pasta do caso.
    pub caminho: String,
    pub sha256: String,
    #[ts(type = "number")]
    pub tamanho: i64,
    pub sied_versao: String,
    #[ts(type = "number")]
    pub auditoria_ate_seq: i64,
    pub auditoria_hash: String,
}

/// O que registrar de um PDF recém-gravado.
pub struct NovoRelatorio {
    pub modelo: String,
    pub secoes: Value,
    pub evidencias: Value,
    pub gerado_em: String,
    pub caminho: String,
    pub sha256: String,
    pub tamanho: i64,
    pub auditoria_ate_seq: i64,
    pub auditoria_hash: String,
}

pub fn registrar(
    tx: &Transaction,
    sessao: &Sessao,
    novo: NovoRelatorio,
) -> Resultado<RelatorioRegistrado> {
    let versao = env!("CARGO_PKG_VERSION");
    tx.execute(
        "INSERT INTO relatorio(modelo, secoes, evidencias, gerado_por, maquina, gerado_em, caminho,
                               sha256, tamanho, sied_versao, auditoria_ate_seq, auditoria_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            novo.modelo,
            novo.secoes.to_string(),
            novo.evidencias.to_string(),
            sessao.usuario,
            sessao.maquina,
            novo.gerado_em,
            novo.caminho,
            novo.sha256,
            novo.tamanho,
            versao,
            novo.auditoria_ate_seq,
            novo.auditoria_hash,
        ],
    )?;
    let id = tx.last_insert_rowid();
    auditoria::registrar(
        tx,
        sessao,
        "relatorio_gerado",
        json!({
            "relatorio_id": id, "modelo": novo.modelo, "caminho": novo.caminho,
            "sha256": novo.sha256, "tamanho": novo.tamanho, "secoes": novo.secoes,
            "evidencias": novo.evidencias, "auditoria_ate_seq": novo.auditoria_ate_seq,
            "auditoria_hash": novo.auditoria_hash, "registrado_em": Utc::now().to_rfc3339(),
        }),
    )?;
    Ok(listar(tx)?
        .into_iter()
        .find(|r| r.id == id)
        .expect("relatório recém-gravado"))
}

/// Mais recentes primeiro.
pub fn listar(conn: &Connection) -> Resultado<Vec<RelatorioRegistrado>> {
    let mut stmt = conn.prepare(
        "SELECT id, modelo, secoes, evidencias, gerado_por, maquina, gerado_em, caminho, sha256,
                tamanho, sied_versao, auditoria_ate_seq, auditoria_hash
         FROM relatorio ORDER BY id DESC",
    )?;
    let linhas = stmt.query_map([], |r| {
        let secoes: String = r.get(2)?;
        let evidencias: String = r.get(3)?;
        Ok(RelatorioRegistrado {
            id: r.get(0)?,
            modelo: r.get(1)?,
            secoes: serde_json::from_str(&secoes).unwrap_or(Value::Null),
            evidencias: serde_json::from_str(&evidencias).unwrap_or(Value::Null),
            gerado_por: r.get(4)?,
            maquina: r.get(5)?,
            gerado_em: r.get(6)?,
            caminho: r.get(7)?,
            sha256: r.get(8)?,
            tamanho: r.get(9)?,
            sied_versao: r.get(10)?,
            auditoria_ate_seq: r.get(11)?,
            auditoria_hash: r.get(12)?,
        })
    })?;
    Ok(linhas.collect::<Result<_, _>>()?)
}
