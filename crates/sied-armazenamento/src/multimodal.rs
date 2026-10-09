//! Recuperação multimodal local sobre os índices já existentes.
//!
//! Cada mecanismo contribui separadamente para o ranking. A união deduplica
//! pelo artefato/arquivo de origem, mas nunca transforma sinais derivados em
//! evidência nova ou em conclusão pericial.

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sied_dominio::{CasoId, ExtracaoId};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use ts_rs::TS;

use crate::busca::{self, FiltroBusca};
use crate::controle_descoberta::ControleDescoberta;
use crate::documentos;
use crate::extracoes::Pagina;
use crate::{embeddings, ErroArmazenamento, Resultado};

const CANDIDATOS: i64 = 100;
const K_RRF: f64 = 60.0;
const MECANISMOS: [&str; 8] = [
    "mensagens",
    "ocr",
    "documentos",
    "transcricoes",
    "embeddings",
    "visao",
    "metadados",
    "entidades",
];

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct PedidoMultimodal {
    pub filtro: FiltroBusca,
    pub mecanismos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SinalMultimodal {
    pub mecanismo: String,
    pub motivo: String,
    pub posicao: u32,
    pub contribuicao_rrf: f64,
    pub motor: String,
    pub motor_versao: String,
    pub hash_derivado: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ItemMultimodal {
    pub chave: String,
    pub origem_tipo: String,
    #[ts(type = "number")]
    pub origem_id: i64,
    pub extracao_id: ExtracaoId,
    #[ts(type = "number | null")]
    pub arquivo_id: Option<i64>,
    pub titulo: String,
    pub trecho: String,
    #[ts(type = "number | null")]
    pub inicio_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub fim_ms: Option<i64>,
    pub hash_origem: Option<String>,
    pub sinais: Vec<SinalMultimodal>,
    pub score_ranking: f64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResultadoMultimodal {
    pub caso_id: CasoId,
    pub pedido: PedidoMultimodal,
    pub algoritmo: String,
    pub executada_em: String,
    pub itens: Pagina<ItemMultimodal>,
    pub recuperados_na_janela: u32,
    pub janela_limitada: bool,
    pub avisos: Vec<String>,
}

fn recusar(motivo: &str) -> ErroArmazenamento {
    ErroArmazenamento::Recusado(motivo.into())
}

fn escapar_like(texto: &str) -> String {
    texto
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn inserir(
    itens: &mut BTreeMap<String, ItemMultimodal>,
    mut item: ItemMultimodal,
    mecanismo: &str,
    motivo: String,
    motor: &str,
    versao: &str,
    posicao: usize,
    hash_derivado: Option<String>,
) {
    let chave = item.chave.clone();
    let sinal = SinalMultimodal {
        mecanismo: mecanismo.into(),
        motivo,
        posicao: (posicao + 1) as u32,
        contribuicao_rrf: 1.0 / (K_RRF + posicao as f64 + 1.0),
        motor: motor.into(),
        motor_versao: versao.into(),
        hash_derivado,
    };
    let existente = itens.entry(chave).or_insert_with(|| {
        item.sinais.clear();
        item
    });
    if let Some(anterior) = existente
        .sinais
        .iter_mut()
        .find(|x| x.mecanismo == mecanismo)
    {
        if sinal.posicao < anterior.posicao {
            *anterior = sinal;
        }
    } else {
        existente.sinais.push(sinal);
    }
}

fn origem_arquivo(conn: &Connection, id: i64) -> Resultado<(Option<String>, Option<i64>)> {
    let r: Option<(Option<String>, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT sha256, criado_em, modificado_em FROM arquivo WHERE id=?1",
            [id],
            |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?)),
        )
        .optional()?;
    let Some((hash, criado, modificado)) = r else {
        return Ok((None, None));
    };
    let data = criado
        .or(modificado)
        .and_then(|x| chrono::DateTime::parse_from_rfc3339(&x).ok())
        .map(|x| x.timestamp_millis());
    Ok((hash, data))
}

fn dentro_filtro(filtro: &FiltroBusca, extracao_id: &str, data: Option<i64>) -> bool {
    if filtro
        .extracao_id
        .is_some_and(|x| x.to_string() != extracao_id)
    {
        return false;
    }
    if filtro.de_ms.is_some_and(|x| data.is_none_or(|d| d < x))
        || filtro.ate_ms.is_some_and(|x| data.is_none_or(|d| d > x))
    {
        return false;
    }
    true
}

fn item(
    chave: String,
    origem_tipo: &str,
    origem_id: i64,
    extracao_id: &str,
    arquivo_id: Option<i64>,
    titulo: String,
    trecho: String,
    inicio_ms: Option<i64>,
    fim_ms: Option<i64>,
    hash_origem: Option<String>,
) -> Resultado<ItemMultimodal> {
    Ok(ItemMultimodal {
        chave,
        origem_tipo: origem_tipo.into(),
        origem_id,
        extracao_id: extracao_id
            .parse()
            .map_err(|_| ErroArmazenamento::Inconsistente("extração inválida".into()))?,
        arquivo_id,
        titulo,
        trecho: trecho.chars().take(500).collect(),
        inicio_ms,
        fim_ms,
        hash_origem,
        sinais: Vec::new(),
        score_ranking: 0.0,
    })
}

pub fn pesquisar(
    conn: &Connection,
    caso_id: CasoId,
    pedido: &PedidoMultimodal,
    deslocamento: i64,
    limite: i64,
    controle: &Arc<ControleDescoberta>,
) -> Resultado<ResultadoMultimodal> {
    if pedido.filtro.texto.trim().is_empty()
        || pedido.filtro.texto.len() > 1024
        || pedido.mecanismos.is_empty()
        || pedido.mecanismos.len() > MECANISMOS.len()
        || !(0..=400).contains(&deslocamento)
        || !(1..=50).contains(&limite)
    {
        return Err(recusar("consulta multimodal inválida ou acima dos limites"));
    }
    let mecanismos: BTreeSet<_> = pedido.mecanismos.iter().map(String::as_str).collect();
    if mecanismos.len() != pedido.mecanismos.len()
        || mecanismos.iter().any(|x| !MECANISMOS.contains(x))
    {
        return Err(recusar("mecanismo multimodal desconhecido ou repetido"));
    }
    let mut itens = BTreeMap::new();
    let mut avisos = vec![
        "Cada sinal é mostrado separadamente; ranking não é confiança nem conclusão pericial."
            .into(),
    ];
    let mut limitada = false;
    let f = &pedido.filtro;

    if mecanismos.contains("mensagens") {
        let r = busca::buscar(conn, f, 0, CANDIDATOS)?;
        limitada |= r.artefatos.total > CANDIDATOS;
        for (pos, a) in r.artefatos.itens.into_iter().enumerate() {
            controle.verificar()?;
            let hash: Option<String> = conn
                .query_row(
                    "SELECT hash_registro FROM artefato WHERE id=?1",
                    [a.artefato.id],
                    |x| x.get(0),
                )
                .ok();
            let trecho = a
                .trecho
                .map(|x| x.into_iter().map(|p| p.texto).collect::<String>())
                .unwrap_or_else(|| a.artefato.conteudo.clone().unwrap_or_default());
            inserir(
                &mut itens,
                item(
                    format!("artefato:{}", a.artefato.id),
                    "mensagem",
                    a.artefato.id,
                    &a.extracao_id.to_string(),
                    a.artefato.anexo_arquivo_id,
                    a.artefato
                        .remetente
                        .clone()
                        .unwrap_or_else(|| format!("Registro {}", a.artefato.id)),
                    trecho,
                    a.artefato.data_hora_ms,
                    a.artefato.data_hora_ms,
                    hash,
                )?,
                "mensagens",
                "Correspondência textual na mensagem/registro original.".into(),
                "sqlite-fts5",
                "unicode61",
                pos,
                None,
            );
        }
    }

    if mecanismos.contains("documentos") || mecanismos.contains("ocr") {
        if f.app.is_some() || f.tipo.is_some() {
            avisos.push("Filtros de app/tipo não se aplicam ao texto documental; documentos foram mantidos somente quando o filtro de extração/data permitiu.".into());
        }
        let docs = documentos::buscar_filtrado(conn, f, 0, CANDIDATOS)?;
        limitada |= docs.total > CANDIDATOS;
        for (pos, d) in docs.itens.into_iter().enumerate() {
            controle.verificar()?;
            let ocr = d.motor_id.to_ascii_lowercase().contains("ocr");
            let mecanismo = if ocr { "ocr" } else { "documentos" };
            if !mecanismos.contains(mecanismo) {
                continue;
            }
            let (hash, _data) = origem_arquivo(conn, d.arquivo_id)?;
            inserir(
                &mut itens,
                item(
                    format!("arquivo:{}", d.arquivo_id),
                    if ocr { "ocr" } else { "documento" },
                    d.id,
                    &d.extracao_id.to_string(),
                    Some(d.arquivo_id),
                    d.caminho,
                    d.trecho,
                    None,
                    None,
                    hash,
                )?,
                mecanismo,
                "Correspondência no texto derivado armazenado; a origem não foi alterada.".into(),
                &d.motor_id,
                &d.motor_versao,
                pos,
                Some(d.sha256_texto),
            );
        }
    }

    if mecanismos.contains("transcricoes") {
        let Some(fts) = busca::consulta_fts(&f.texto) else {
            return Err(recusar("consulta multimodal sem termos indexáveis"));
        };
        let mut st = conn.prepare("SELECT s.id,t.extracao_id,t.arquivo_id,t.caminho,s.texto,s.inicio_ms,s.fim_ms,t.sha256_origem,t.motor_id,t.motor_versao FROM busca_transcricao JOIN segmento_transcricao s ON s.id=busca_transcricao.rowid JOIN transcricao_local t ON t.id=s.transcricao_id WHERE busca_transcricao MATCH ?1 AND t.estado='pronta' ORDER BY bm25(busca_transcricao),s.id LIMIT 101")?;
        let rows = st
            .query_map([fts.as_str()], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, String>(9)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        limitada |= rows.len() as i64 > CANDIDATOS;
        for (pos, (id, ext, arquivo, caminho, texto, inicio, fim, hash, motor, versao)) in
            rows.into_iter().take(CANDIDATOS as usize).enumerate()
        {
            controle.verificar()?;
            let (_, data) = origem_arquivo(conn, arquivo)?;
            if !dentro_filtro(f, &ext, data) {
                continue;
            }
            inserir(
                &mut itens,
                item(
                    format!("arquivo:{}", arquivo),
                    "transcricao",
                    id,
                    &ext,
                    Some(arquivo),
                    caminho,
                    texto,
                    Some(inicio),
                    Some(fim),
                    Some(hash),
                )?,
                "transcricoes",
                "Correspondência no texto transcrito com timestamps preservados.".into(),
                &motor,
                &versao,
                pos,
                None,
            );
        }
    }

    if mecanismos.contains("embeddings") {
        let r = embeddings::buscar(conn, &caso_id, &f.texto, 0, CANDIDATOS, 1.0, controle)?;
        limitada |= r.itens.total > CANDIDATOS;
        for (pos, e) in r.itens.itens.into_iter().enumerate() {
            controle.verificar()?;
            let chave = e
                .arquivo_id
                .map(|x| format!("arquivo:{x}"))
                .unwrap_or_else(|| format!("{}:{}", e.origem_tipo, e.origem_id));
            let sinais = e.sinais;
            let mut item = item(
                chave,
                &e.origem_tipo,
                e.origem_id,
                &e.extracao_id.to_string(),
                e.arquivo_id,
                e.titulo,
                e.trecho,
                e.inicio_ms,
                e.fim_ms,
                None,
            )?;
            let key = item.chave.clone();
            let entry = itens.entry(key).or_insert(item);
            entry
                .sinais
                .extend(sinais.into_iter().map(|s| SinalMultimodal {
                    mecanismo: "embeddings".into(),
                    motivo: s.motivo,
                    posicao: s.posicao.unwrap_or((pos + 1) as u32),
                    contribuicao_rrf: s.contribuicao,
                    motor: s.motor_id,
                    motor_versao: s.motor_versao,
                    hash_derivado: None,
                }));
        }
    }

    if mecanismos.contains("metadados")
        || mecanismos.contains("visao")
        || mecanismos.contains("entidades")
    {
        let termo = format!("%{}%", escapar_like(&f.texto.to_ascii_lowercase()));
        if mecanismos.contains("entidades") {
            let mut st = conn.prepare("SELECT e.id,d.arquivo_id,d.extracao_id,d.caminho,e.literal,e.tipo,d.sha256_origem,d.entidades_motor_versao FROM entidade_documental e JOIN documento_derivado d ON d.id=e.documento_id WHERE lower(e.literal) LIKE ?1 ESCAPE '\\' OR lower(e.tipo) LIKE ?1 ESCAPE '\\' LIMIT 101")?;
            let rows = st
                .query_map([termo.as_str()], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, String>(7)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            limitada |= rows.len() as i64 > CANDIDATOS;
            for (pos, (id, arquivo, ext, caminho, literal, tipo, hash, versao)) in
                rows.into_iter().take(CANDIDATOS as usize).enumerate()
            {
                controle.verificar()?;
                let (_, data) = origem_arquivo(conn, arquivo)?;
                if !dentro_filtro(f, &ext, data) {
                    continue;
                }
                inserir(
                    &mut itens,
                    item(
                        format!("entidade:{id}"),
                        "entidade",
                        id,
                        &ext,
                        Some(arquivo),
                        caminho,
                        format!("{tipo}: {literal}"),
                        None,
                        None,
                        Some(hash),
                    )?,
                    "entidades",
                    "Correspondência literal em candidato de entidade documental; não comprova identidade ou titularidade.".into(),
                    "entidades-documentais",
                    &versao,
                    pos,
                    None,
                );
            }
        }
        if mecanismos.contains("metadados") {
            let mut st = conn.prepare("SELECT id,extracao_id,caminho,categoria,COALESCE(mime,''),sha256 FROM arquivo WHERE (lower(caminho) LIKE ?1 ESCAPE '\\' OR lower(categoria) LIKE ?1 ESCAPE '\\' OR lower(COALESCE(mime,'')) LIKE ?1 ESCAPE '\\') LIMIT 101")?;
            let rows = st
                .query_map([termo.as_str()], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, Option<String>>(5)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            limitada |= rows.len() as i64 > CANDIDATOS;
            for (pos, (id, ext, caminho, categoria, mime, hash)) in
                rows.into_iter().take(CANDIDATOS as usize).enumerate()
            {
                controle.verificar()?;
                let (hash, data) = origem_arquivo(conn, id)?;
                if !dentro_filtro(f, &ext, data) {
                    continue;
                }
                inserir(
                    &mut itens,
                    item(
                        format!("arquivo:{id}"),
                        "metadado",
                        id,
                        &ext,
                        Some(id),
                        caminho.clone(),
                        format!("{categoria} · {mime}"),
                        None,
                        None,
                        hash,
                    )?,
                    "metadados",
                    "Correspondência em caminho, categoria ou MIME do inventário.".into(),
                    "sqlite-metadata",
                    "1",
                    pos,
                    None,
                );
            }
        }
        if mecanismos.contains("visao") {
            let mut st = conn.prepare("SELECT v.id,v.arquivo_id,v.extracao_id,v.caminho,v.dhash,v.motor_id,v.motor_versao,v.sha256_origem FROM visual_derivado v WHERE lower(v.caminho) LIKE ?1 ESCAPE '\\' OR lower(v.dhash) LIKE ?1 ESCAPE '\\' OR lower(v.motor_id) LIKE ?1 ESCAPE '\\' LIMIT 101")?;
            let rows = st
                .query_map([termo.as_str()], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, String>(7)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            limitada |= rows.len() as i64 > CANDIDATOS;
            for (pos, (id, arquivo, ext, caminho, dhash, motor, versao, hash)) in
                rows.into_iter().take(CANDIDATOS as usize).enumerate()
            {
                controle.verificar()?;
                let (_, data) = origem_arquivo(conn, arquivo)?;
                if !dentro_filtro(f, &ext, data) {
                    continue;
                }
                inserir(
                    &mut itens,
                    item(
                        format!("arquivo:{arquivo}"),
                        "visual",
                        id,
                        &ext,
                        Some(arquivo),
                        caminho,
                        format!("dHash {dhash}"),
                        None,
                        None,
                        Some(hash),
                    )?,
                    "visao",
                    "Correspondência no descritor visual local; não é identificação de objeto."
                        .into(),
                    &motor,
                    &versao,
                    pos,
                    None,
                );
            }
        }
    }

    let total_mecanismos = mecanismos.len() as f64;
    let mut itens: Vec<_> = itens.into_values().collect();
    for i in &mut itens {
        i.score_ranking = (i.sinais.iter().map(|x| x.contribuicao_rrf).sum::<f64>()
            / (total_mecanismos * (1.0 / (K_RRF + 1.0))))
            .clamp(0.0, 1.0);
        i.sinais.sort_by_key(|x| x.posicao);
    }
    itens.sort_by(|a, b| {
        b.score_ranking
            .total_cmp(&a.score_ranking)
            .then_with(|| a.chave.cmp(&b.chave))
    });
    let total = itens.len() as u32;
    if limitada {
        avisos.push("A janela foi limitada a 100 candidatos por mecanismo; a paginação percorre somente essa união.".into());
    }
    Ok(ResultadoMultimodal {
        caso_id,
        pedido: pedido.clone(),
        algoritmo: "rrf-multimodal-local-v1-equal-mechanisms".into(),
        executada_em: Utc::now().to_rfc3339(),
        itens: Pagina {
            total: total as i64,
            itens: itens
                .into_iter()
                .skip(deslocamento as usize)
                .take(limite as usize)
                .collect(),
        },
        recuperados_na_janela: total,
        janela_limitada: limitada,
        avisos,
    })
}
