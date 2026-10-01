//! Registro dos relatórios gerados (o PDF fica em `<pasta do caso>/relatorios/`).

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection, Transaction};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
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

/// Situação do PDF de um relatório registrado, conferido na pasta do caso.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PdfConferido {
    #[ts(type = "number")]
    pub relatorio_id: i64,
    /// "confere", "divergente" (o SHA-256 do arquivo não é o registrado) ou "ausente".
    pub situacao: String,
}

/// Arquivo na pasta de relatórios que não pertence a nenhum relatório registrado.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ArquivoAvulso {
    pub nome: String,
    #[ts(type = "number")]
    pub tamanho: i64,
    /// "pdf" (PDF sem registro, ex.: queda entre gravar e registrar), "parcial"
    /// (gravação interrompida), "sha256" (arquivo de hash sem relatório) ou "outro".
    pub tipo: String,
}

/// Conferência da pasta `relatorios/` com o que o caso registrou. Só lê: nada é
/// apagado, movido nem registrado.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ConferenciaPastaRelatorios {
    pub pdfs: Vec<PdfConferido>,
    pub avulsos: Vec<ArquivoAvulso>,
}

fn sha256_arquivo(caminho: &Path) -> std::io::Result<String> {
    let mut f = fs::File::open(caminho)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            return Ok(hex::encode(h.finalize()));
        }
        h.update(&buf[..n]);
    }
}

/// Confere cada PDF registrado (existe e tem o SHA-256 registrado) e lista o que mais
/// houver na pasta de relatórios. O arquivo `.sha256` de um relatório registrado é
/// esperado e não entra como avulso.
pub fn conferir_pasta(
    pasta_caso: &Path,
    registrados: &[RelatorioRegistrado],
) -> Resultado<ConferenciaPastaRelatorios> {
    let mut esperados = HashSet::new();
    let mut pdfs = Vec::with_capacity(registrados.len());
    for r in registrados {
        let caminho = pasta_caso.join(&r.caminho);
        if let Some(nome) = Path::new(&r.caminho).file_name().and_then(|n| n.to_str()) {
            esperados.insert(nome.to_string());
            esperados.insert(format!("{nome}.sha256"));
        }
        let situacao = match sha256_arquivo(&caminho) {
            Ok(h) if h == r.sha256 => "confere",
            Ok(_) => "divergente",
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => "ausente",
            Err(e) => return Err(e.into()),
        };
        pdfs.push(PdfConferido {
            relatorio_id: r.id,
            situacao: situacao.into(),
        });
    }
    let mut avulsos = Vec::new();
    let pasta = pasta_caso.join(PASTA_RELATORIOS);
    let entradas = match fs::read_dir(&pasta) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ConferenciaPastaRelatorios { pdfs, avulsos })
        }
        Err(e) => return Err(e.into()),
    };
    for e in entradas {
        let e = e?;
        let nome = e.file_name().to_string_lossy().into_owned();
        if esperados.contains(&nome) {
            continue;
        }
        let meta = e.metadata()?;
        let minusculo = nome.to_lowercase();
        let tipo = if meta.is_dir() {
            "outro"
        } else if minusculo.ends_with(".pdf") {
            "pdf"
        } else if minusculo.ends_with(".parcial") {
            "parcial"
        } else if minusculo.ends_with(".sha256") {
            "sha256"
        } else {
            "outro"
        };
        avulsos.push(ArquivoAvulso {
            nome,
            tamanho: if meta.is_dir() { 0 } else { meta.len() as i64 },
            tipo: tipo.into(),
        });
    }
    avulsos.sort_by(|a, b| a.nome.cmp(&b.nome));
    Ok(ConferenciaPastaRelatorios { pdfs, avulsos })
}
