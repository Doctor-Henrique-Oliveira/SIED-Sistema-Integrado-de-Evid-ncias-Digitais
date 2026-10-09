//! Embeddings locais determinísticos e ranking híbrido.
//!
//! O motor `hash-embedding-local/tfidf-hash-v1` não é um modelo neural nem envia
//! conteúdo para fora do caso. Ele transforma tokens normalizados em um vetor
//! esparso de dimensão fixa, permitindo uma busca semântica lexical reprodutível
//! quando nenhum modelo local autorizado está instalado. Scores são ranking, nunca
//! confiança ou conclusão pericial.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sied_dominio::{CasoId, ExtracaoId};
use std::collections::HashMap;
use std::sync::Arc;
use ts_rs::TS;

use crate::busca::consulta_fts;
use crate::controle_descoberta::{ControleDescoberta, GuardaDescoberta};
use crate::extracoes::Pagina;
use crate::{ErroArmazenamento, Resultado};

pub const DIMENSAO: usize = 256;
pub const MOTOR_ID: &str = "hash-embedding-local";
pub const MOTOR_VERSAO: &str = "tfidf-hash-v1";
const MAX_TEXTO: usize = 1_000_000;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResumoIndexacaoEmbeddings {
    #[ts(type = "number")]
    pub examinados: i64,
    #[ts(type = "number")]
    pub criados: i64,
    #[ts(type = "number")]
    pub reutilizados: i64,
    #[ts(type = "number")]
    pub ignorados: i64,
    pub motor_id: String,
    pub motor_versao: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SinalBuscaHibrida {
    pub mecanismo: String,
    pub motivo: String,
    pub contribuicao: f64,
    #[ts(type = "number | null")]
    pub posicao: Option<u32>,
    pub motor_id: String,
    pub motor_versao: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ItemBuscaHibrida {
    pub origem_tipo: String,
    #[ts(type = "number")]
    pub origem_id: i64,
    pub extracao_id: ExtracaoId,
    #[ts(type = "number | null")]
    pub arquivo_id: Option<i64>,
    pub titulo: String,
    pub trecho: String,
    pub score_hibrido: f64,
    pub score_semantico: Option<f64>,
    pub score_textual: Option<f64>,
    #[ts(type = "number | null")]
    pub inicio_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub fim_ms: Option<i64>,
    pub sinais: Vec<SinalBuscaHibrida>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResultadoBuscaHibrida {
    pub consulta: String,
    pub itens: Pagina<ItemBuscaHibrida>,
    pub motor_id: String,
    pub motor_versao: String,
    pub peso_semantico: f64,
    pub avisos: Vec<String>,
}

#[derive(Debug, Clone)]
struct Origem {
    tipo: String,
    id: i64,
    extracao_id: String,
    arquivo_id: Option<i64>,
    titulo: String,
    texto: String,
    inicio_ms: Option<i64>,
    fim_ms: Option<i64>,
}

#[derive(Debug, Clone)]
struct Vetor {
    bytes: Vec<u8>,
    valores: Vec<f32>,
    norma: f32,
}

fn hash_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn vetorizar(texto: &str) -> Vetor {
    let mut valores = vec![0f32; DIMENSAO];
    for token in crate::normalizar_busca(texto).split_whitespace() {
        if token.len() < 2 {
            continue;
        }
        let digest = Sha256::digest(token.as_bytes());
        let indice = u16::from_le_bytes([digest[0], digest[1]]) as usize % DIMENSAO;
        let sinal = if digest[2] & 1 == 0 { 1.0 } else { -1.0 };
        valores[indice] += sinal;
    }
    let norma = valores.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norma > 0.0 {
        for v in &mut valores {
            *v /= norma;
        }
    }
    let bytes = valores.iter().flat_map(|v| v.to_le_bytes()).collect();
    Vetor {
        bytes,
        valores,
        norma,
    }
}

fn decodificar(bytes: &[u8], norma: f64) -> Resultado<Vec<f32>> {
    if bytes.len() != DIMENSAO * 4 || !norma.is_finite() || norma < 0.0 {
        return Err(ErroArmazenamento::Inconsistente(
            "vetor derivado inválido".into(),
        ));
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|p| f32::from_le_bytes([p[0], p[1], p[2], p[3]]))
        .collect())
}

fn similaridade(a: &[f32], b: &[f32]) -> f64 {
    let produto = a
        .iter()
        .zip(b)
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum::<f64>();
    ((produto + 1.0) / 2.0).clamp(0.0, 1.0)
}

fn texto_hash(texto: &str) -> String {
    hash_bytes(texto.as_bytes())
}

fn inserir_embedding(
    tx: &rusqlite::Transaction<'_>,
    caso_id: &CasoId,
    origem: &Origem,
    vetor: &Vetor,
    agora: &str,
) -> Result<bool, rusqlite::Error> {
    let hash_derivado = hash_bytes(
        &[
            MOTOR_ID.as_bytes(),
            MOTOR_VERSAO.as_bytes(),
            vetor.bytes.as_slice(),
        ]
        .concat(),
    );
    let alteradas = tx.execute(
        "INSERT OR IGNORE INTO embedding_derivado(caso_id,origem_tipo,origem_id,extracao_id,arquivo_id,texto_hash,dimensao,vetor,norma,motor_id,motor_versao,executado_em,hash_derivado) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![caso_id.to_string(), &origem.tipo, origem.id, &origem.extracao_id, origem.arquivo_id, texto_hash(&origem.texto), DIMENSAO as i64, &vetor.bytes, vetor.norma, MOTOR_ID, MOTOR_VERSAO, agora, hash_derivado],
    )?;
    Ok(alteradas > 0)
}

fn origens(conn: &Connection, controle: &Arc<ControleDescoberta>) -> Resultado<Vec<Origem>> {
    let _guarda = GuardaDescoberta::instalar(conn, controle);
    let mut saida = Vec::new();
    let mut st = conn.prepare("SELECT id,extracao_id,anexo_arquivo_id,COALESCE(conteudo,'') FROM artefato WHERE trim(COALESCE(conteudo,'')) <> ''")?;
    for row in st.query_map([], |r| {
        Ok(Origem {
            tipo: "artefato".into(),
            id: r.get(0)?,
            extracao_id: r.get(1)?,
            arquivo_id: r.get(2)?,
            titulo: "Registro de evidência".into(),
            texto: r.get(3)?,
            inicio_ms: None,
            fim_ms: None,
        })
    })? {
        controle.verificar()?;
        let o = row?;
        if o.texto.len() <= MAX_TEXTO {
            saida.push(o);
        }
    }
    let mut st = conn.prepare("SELECT d.id,d.extracao_id,d.arquivo_id,d.caminho,d.texto,d.pagina FROM documento_derivado d WHERE trim(d.texto) <> ''")?;
    for row in st.query_map([], |r| {
        Ok(Origem {
            tipo: "documento".into(),
            id: r.get(0)?,
            extracao_id: r.get(1)?,
            arquivo_id: Some(r.get(2)?),
            titulo: r.get::<_, String>(3)?,
            texto: r.get(4)?,
            inicio_ms: None,
            fim_ms: None,
        })
    })? {
        controle.verificar()?;
        let o = row?;
        if o.texto.len() <= MAX_TEXTO {
            saida.push(o);
        }
    }
    let mut st = conn.prepare("SELECT s.id,t.extracao_id,t.arquivo_id,t.caminho,s.texto,s.inicio_ms,s.fim_ms FROM segmento_transcricao s JOIN transcricao_local t ON t.id=s.transcricao_id WHERE t.estado='pronta' AND trim(s.texto) <> ''")?;
    for row in st.query_map([], |r| {
        Ok(Origem {
            tipo: "transcricao".into(),
            id: r.get(0)?,
            extracao_id: r.get(1)?,
            arquivo_id: Some(r.get(2)?),
            titulo: r.get(3)?,
            texto: r.get(4)?,
            inicio_ms: Some(r.get(5)?),
            fim_ms: Some(r.get(6)?),
        })
    })? {
        controle.verificar()?;
        saida.push(row?);
    }
    Ok(saida)
}

pub fn indexar(
    conn: &mut Connection,
    caso_id: &CasoId,
    controle: &Arc<ControleDescoberta>,
    sessao: &crate::Sessao,
) -> Resultado<ResumoIndexacaoEmbeddings> {
    let lista = origens(conn, controle)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let agora = Utc::now().to_rfc3339();
    let mut resumo = ResumoIndexacaoEmbeddings {
        examinados: 0,
        criados: 0,
        reutilizados: 0,
        ignorados: 0,
        motor_id: MOTOR_ID.into(),
        motor_versao: MOTOR_VERSAO.into(),
    };
    for origem in &lista {
        controle.verificar()?;
        resumo.examinados += 1;
        let vetor = vetorizar(&origem.texto);
        if vetor.norma == 0.0 {
            resumo.ignorados += 1;
            continue;
        }
        if inserir_embedding(&tx, caso_id, origem, &vetor, &agora)? {
            resumo.criados += 1;
        } else {
            resumo.reutilizados += 1;
        }
    }
    controle.concluir()?;
    crate::auditoria::registrar(
        &tx,
        sessao,
        "embedding_indexado",
        serde_json::json!({"motor_id":MOTOR_ID,"motor_versao":MOTOR_VERSAO,"examinados":resumo.examinados,"criados":resumo.criados,"reutilizados":resumo.reutilizados,"ignorados":resumo.ignorados}),
    )?;
    tx.commit()?;
    Ok(resumo)
}

fn adicionar_textual(
    map: &mut HashMap<(String, i64), (u32, String)>,
    tipo: &str,
    id: i64,
    pos: u32,
) {
    map.entry((tipo.into(), id)).or_insert((
        pos,
        format!("Correspondência textual local · posição {pos}"),
    ));
}

pub fn buscar(
    conn: &Connection,
    caso_id: &CasoId,
    texto: &str,
    deslocamento: i64,
    limite: i64,
    peso_semantico: f64,
    controle: &Arc<ControleDescoberta>,
) -> Resultado<ResultadoBuscaHibrida> {
    if texto.trim().is_empty()
        || texto.len() > 1024
        || deslocamento < 0
        || !(1..=200).contains(&limite)
        || !peso_semantico.is_finite()
        || !(0.0..=1.0).contains(&peso_semantico)
    {
        return Err(ErroArmazenamento::Recusado(
            "consulta híbrida inválida".into(),
        ));
    }
    let consulta = texto.trim().to_string();
    let vetor_consulta = vetorizar(&consulta);
    let mut textual: HashMap<(String, i64), (u32, String)> = HashMap::new();
    let fts = consulta_fts(&consulta);
    if let Some(fts) = fts {
        let mut st = conn.prepare("SELECT a.id FROM busca_artefato JOIN artefato a ON a.id=busca_artefato.rowid WHERE busca_artefato MATCH ?1 ORDER BY bm25(busca_artefato),a.id LIMIT 100")?;
        for (pos, row) in st
            .query_map([fts.as_str()], |r| r.get::<_, i64>(0))?
            .enumerate()
        {
            controle.verificar()?;
            adicionar_textual(&mut textual, "artefato", row?, pos as u32 + 1);
        }
        let mut st = conn.prepare("SELECT d.id FROM busca_documento JOIN documento_derivado d ON d.id=busca_documento.rowid WHERE busca_documento MATCH ?1 ORDER BY bm25(busca_documento),d.id LIMIT 100")?;
        for (pos, row) in st
            .query_map([fts.as_str()], |r| r.get::<_, i64>(0))?
            .enumerate()
        {
            controle.verificar()?;
            adicionar_textual(&mut textual, "documento", row?, pos as u32 + 1);
        }
        let mut st = conn.prepare("SELECT s.id FROM busca_transcricao JOIN segmento_transcricao s ON s.id=busca_transcricao.rowid WHERE busca_transcricao MATCH ?1 ORDER BY bm25(busca_transcricao),s.id LIMIT 100")?;
        for (pos, row) in st
            .query_map([fts.as_str()], |r| r.get::<_, i64>(0))?
            .enumerate()
        {
            controle.verificar()?;
            adicionar_textual(&mut textual, "transcricao", row?, pos as u32 + 1);
        }
    }
    let mut sem: HashMap<(String, i64), (f64, String)> = HashMap::new();
    let mut st = conn.prepare("SELECT origem_tipo,origem_id,vetor,norma FROM embedding_derivado WHERE caso_id=?1 AND motor_id=?2 AND motor_versao=?3 ORDER BY id")?;
    for row in st.query_map(params![caso_id.to_string(), MOTOR_ID, MOTOR_VERSAO], |r| {
        let bytes: Vec<u8> = r.get(2)?;
        let norma: f64 = r.get(3)?;
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, bytes, norma))
    })? {
        controle.verificar()?;
        let (tipo, id, bytes, norma) = row?;
        let vetor = decodificar(&bytes, norma)?;
        let score = similaridade(&vetor_consulta.valores, &vetor);
        if score > 0.5 {
            sem.insert(
                (tipo, id),
                (score, format!("Similaridade vetorial local · {:.3}", score)),
            );
        }
    }
    let mut sem_rank: HashMap<(String, i64), f64> = HashMap::new();
    let mut sem_ordenados: Vec<((String, i64), f64)> = sem
        .iter()
        .map(|(chave, (score, _))| (chave.clone(), *score))
        .collect();
    sem_ordenados.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (pos, (chave, _)) in sem_ordenados.into_iter().enumerate() {
        sem_rank.insert(chave, 1.0 / (60.0 + pos as f64 + 1.0));
    }
    // Releitura por chave mantém a explicabilidade e evita materializar a origem no índice.
    let mut chaves: Vec<(String, i64)> = textual.keys().chain(sem.keys()).cloned().collect();
    chaves.sort();
    chaves.dedup();
    let mut completos = Vec::new();
    for (idx, (tipo, id)) in chaves.into_iter().enumerate() {
        controle.verificar()?;
        let origem = match tipo.as_str() {
            "artefato" => conn.query_row("SELECT extracao_id,anexo_arquivo_id,COALESCE(nome_arquivo,COALESCE(app,'Registro de evidência')),substr(COALESCE(conteudo,''),1,500),NULL,NULL FROM artefato WHERE id=?1", [id], |r| Ok(Origem{tipo:tipo.clone(),id,extracao_id:r.get(0)?,arquivo_id:r.get(1)?,titulo:r.get(2)?,texto:r.get(3)?,inicio_ms:None,fim_ms:None})).optional()?,
            "documento" => conn.query_row("SELECT extracao_id,arquivo_id,caminho,substr(texto,1,500),NULL,NULL FROM documento_derivado WHERE id=?1", [id], |r| Ok(Origem{tipo:tipo.clone(),id,extracao_id:r.get(0)?,arquivo_id:Some(r.get(1)?),titulo:r.get(2)?,texto:r.get(3)?,inicio_ms:None,fim_ms:None})).optional()?,
            _ => conn.query_row("SELECT t.extracao_id,t.arquivo_id,t.caminho,substr(s.texto,1,500),s.inicio_ms,s.fim_ms FROM segmento_transcricao s JOIN transcricao_local t ON t.id=s.transcricao_id WHERE s.id=?1", [id], |r| Ok(Origem{tipo:tipo.clone(),id,extracao_id:r.get(0)?,arquivo_id:Some(r.get(1)?),titulo:r.get(2)?,texto:r.get(3)?,inicio_ms:r.get(4)?,fim_ms:r.get(5)?})).optional()?,
        };
        let Some(origem) = origem else {
            continue;
        };
        let txt = textual.get(&(tipo.clone(), id));
        let s = sem.get(&(tipo.clone(), id));
        let text_score = txt.map(|(p, _)| 1.0 / (60.0 + *p as f64));
        let sem_score = s.map(|(v, _)| *v);
        let sem_rank_score = sem_rank.get(&(tipo.clone(), id)).copied();
        let h = peso_semantico * sem_rank_score.unwrap_or(0.0)
            + (1.0 - peso_semantico) * text_score.unwrap_or(0.0);
        let mut sinais = Vec::new();
        if let Some((p, m)) = txt {
            sinais.push(SinalBuscaHibrida {
                mecanismo: "textual".into(),
                motivo: m.clone(),
                contribuicao: (1.0 - peso_semantico) * text_score.unwrap_or(0.0),
                posicao: Some(*p),
                motor_id: "sqlite-fts5".into(),
                motor_versao: "unicode61".into(),
            });
        }
        if let Some((_v, m)) = s {
            sinais.push(SinalBuscaHibrida {
                mecanismo: "semantico_local".into(),
                motivo: m.clone(),
                contribuicao: peso_semantico * sem_rank_score.unwrap_or(0.0),
                posicao: None,
                motor_id: MOTOR_ID.into(),
                motor_versao: MOTOR_VERSAO.into(),
            });
        }
        completos.push((
            h,
            ItemBuscaHibrida {
                origem_tipo: tipo,
                origem_id: id,
                extracao_id: origem
                    .extracao_id
                    .parse()
                    .map_err(|_| ErroArmazenamento::Inconsistente("extracao inválida".into()))?,
                arquivo_id: origem.arquivo_id,
                titulo: origem.titulo,
                trecho: origem.texto,
                score_hibrido: h,
                score_semantico: sem_score,
                score_textual: text_score,
                inicio_ms: origem.inicio_ms,
                fim_ms: origem.fim_ms,
                sinais,
            },
        ));
        let _ = idx;
    }
    completos.sort_by(|a, b| b.0.total_cmp(&a.0));
    let total = completos.len() as i64;
    let itens = completos
        .into_iter()
        .skip(deslocamento as usize)
        .take(limite as usize)
        .map(|(_, i)| i)
        .collect();
    Ok(ResultadoBuscaHibrida{consulta,peso_semantico,motor_id:MOTOR_ID.into(),motor_versao:MOTOR_VERSAO.into(),itens:Pagina{total,itens},avisos:vec!["O mecanismo semântico é um embedding local hash lexical, não um modelo neural.".into(),"Scores servem somente para ordenar candidatos; não são confiança nem conclusão pericial.".into()]})
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn vetor_e_deterministico_e_normalizado() {
        let a = vetorizar("Áudio com telefone");
        let b = vetorizar("Áudio com telefone");
        assert_eq!(a.bytes, b.bytes);
        assert!((a.norma - 1.0).abs() < 0.01);
        assert!(similaridade(&a.valores, &b.valores) > 0.99);
    }
    #[test]
    fn vetor_vazio_nao_finge_similaridade() {
        assert_eq!(vetorizar("a").norma, 0.0);
    }
}
