//! Conversas reconstruídas a partir dos artefatos.
//!
//! Uma conversa é o agrupamento dos artefatos de um mesmo app com o mesmo
//! `conversa_id_externo` numa extração. A tabela `conversa` é derivada (mantida por
//! gatilho, migração 3): os dados continuam nos artefatos e na referência de origem.
//!
//! Título, tipo e nomes vêm, na hora da consulta, dos registros que o próprio app
//! gravou na mesma extração: o registro `conversa` (ex.: assunto do grupo no
//! WhatsApp) e os registros `contato` com o mesmo identificador. Nada é gravado na
//! conversa derivada, então um parser novo ou uma reimportação não deixa nome velho.

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use sied_dominio::ExtracaoId;
use ts_rs::TS;

use crate::extracoes::{ler_resumo, uuid, ArtefatoResumo, Pagina, COLUNAS_RESUMO};
use crate::{ErroArmazenamento, Resultado};

/// Participantes exibidos na lista de conversas (os mais frequentes).
const MAX_PARTICIPANTES_LISTA: i64 = 5;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ParticipanteConversa {
    /// Como o parser emitiu (telefone, jid...).
    pub identificador: String,
    #[ts(type = "number")]
    pub mensagens: i64,
    /// Nome do registro de contato com o mesmo identificador, na mesma extração.
    pub nome: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Conversa {
    #[ts(type = "number")]
    pub id: i64,
    pub extracao_id: ExtracaoId,
    pub app: String,
    pub id_externo: String,
    #[ts(type = "number")]
    pub total_mensagens: i64,
    #[ts(type = "number")]
    pub enviadas: i64,
    #[ts(type = "number")]
    pub com_anexo: i64,
    #[ts(type = "number | null")]
    pub primeira_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub ultima_ms: Option<i64>,
    /// Mais frequentes primeiro; no máximo 5 na lista, todos no detalhe.
    pub participantes: Vec<ParticipanteConversa>,
    #[ts(type = "number")]
    pub total_participantes: i64,
    /// Texto do último registro, ou o nome do anexo (para a prévia na lista).
    pub ultima_mensagem: Option<String>,
    /// Assunto do registro da conversa (grupos) ou, sem ele, nome do contato.
    pub titulo: Option<String>,
    /// "individual", "grupo"... quando o app grava o registro da conversa.
    pub tipo: Option<String>,
    /// Registro `conversa` do app (para a visão técnica).
    #[ts(type = "number | null")]
    pub registro_id: Option<i64>,
}

const COLUNAS: &str = "c.id, c.extracao_id, c.app, c.id_externo, c.total_mensagens, c.enviadas,
    c.com_anexo, c.primeira_ms, c.ultima_ms,
    (SELECT COUNT(*) FROM conversa_participante p WHERE p.conversa_id = c.id),
    (SELECT coalesce(a.conteudo, a.nome_arquivo) FROM artefato a
      WHERE a.extracao_id = c.extracao_id AND coalesce(a.app, '') = c.app
        AND a.conversa_id_externo = c.id_externo
      ORDER BY a.data_hora_ms DESC, a.id DESC LIMIT 1),
    coalesce(
      (SELECT r.conteudo FROM artefato r
        WHERE r.extracao_id = c.extracao_id AND r.app = c.app AND r.tipo = 'conversa'
          AND r.id_externo = c.id_externo AND r.conteudo IS NOT NULL ORDER BY r.id LIMIT 1),
      (SELECT r.conteudo FROM artefato r
        WHERE r.extracao_id = c.extracao_id AND r.app = c.app AND r.tipo = 'contato'
          AND r.id_externo = c.id_externo AND r.conteudo IS NOT NULL ORDER BY r.id LIMIT 1)),
    (SELECT r.subtipo FROM artefato r
      WHERE r.extracao_id = c.extracao_id AND r.app = c.app AND r.tipo = 'conversa'
        AND r.id_externo = c.id_externo ORDER BY r.id LIMIT 1),
    (SELECT r.id FROM artefato r
      WHERE r.extracao_id = c.extracao_id AND r.app = c.app AND r.tipo = 'conversa'
        AND r.id_externo = c.id_externo ORDER BY r.id LIMIT 1)";

fn ler(r: &Row) -> rusqlite::Result<Conversa> {
    Ok(Conversa {
        id: r.get(0)?,
        extracao_id: uuid(r.get(1)?)?,
        app: r.get(2)?,
        id_externo: r.get(3)?,
        total_mensagens: r.get(4)?,
        enviadas: r.get(5)?,
        com_anexo: r.get(6)?,
        primeira_ms: r.get(7)?,
        ultima_ms: r.get(8)?,
        participantes: Vec::new(),
        total_participantes: r.get(9)?,
        ultima_mensagem: r.get(10)?,
        titulo: r.get(11)?,
        tipo: r.get(12)?,
        registro_id: r.get(13)?,
    })
}

fn participantes(
    conn: &Connection,
    conversa_id: i64,
    limite: i64,
) -> Resultado<Vec<ParticipanteConversa>> {
    let mut stmt = conn.prepare_cached(
        "SELECT p.identificador, p.mensagens,
                (SELECT r.conteudo FROM artefato r
                  WHERE r.extracao_id = c.extracao_id AND r.app = c.app AND r.tipo = 'contato'
                    AND r.id_externo = p.identificador AND r.conteudo IS NOT NULL
                  ORDER BY r.id LIMIT 1)
         FROM conversa_participante p JOIN conversa c ON c.id = p.conversa_id
         WHERE p.conversa_id = ?1
         ORDER BY p.mensagens DESC, p.identificador LIMIT ?2",
    )?;
    let v = stmt.query_map(params![conversa_id, limite], |r| {
        Ok(ParticipanteConversa {
            identificador: r.get(0)?,
            mensagens: r.get(1)?,
            nome: r.get(2)?,
        })
    })?;
    Ok(v.collect::<Result<_, _>>()?)
}

/// Conversas de uma extração (ou de todas), da mais recente para a mais antiga.
pub fn listar(
    conn: &Connection,
    extracao: Option<ExtracaoId>,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<Conversa>> {
    let extracao = extracao.map(|e| e.to_string());
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM conversa WHERE ?1 IS NULL OR extracao_id = ?1",
        [&extracao],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUNAS} FROM conversa c WHERE ?1 IS NULL OR c.extracao_id = ?1
         ORDER BY c.ultima_ms IS NULL, c.ultima_ms DESC, c.id LIMIT ?2 OFFSET ?3"
    ))?;
    let mut itens: Vec<Conversa> = stmt
        .query_map(params![extracao, limite, deslocamento], ler)?
        .collect::<Result<_, _>>()?;
    for c in &mut itens {
        c.participantes = participantes(conn, c.id, MAX_PARTICIPANTES_LISTA)?;
    }
    Ok(Pagina { total, itens })
}

pub fn obter(conn: &Connection, conversa_id: i64) -> Resultado<Conversa> {
    let mut c = conn
        .query_row(
            &format!("SELECT {COLUNAS} FROM conversa c WHERE c.id = ?1"),
            [conversa_id],
            ler,
        )
        .optional()?
        .ok_or_else(|| ErroArmazenamento::NaoEncontrado("conversa".into()))?;
    c.participantes = participantes(conn, c.id, i64::MAX)?;
    Ok(c)
}

/// Registros da conversa em ordem cronológica (sem data vão para o fim, pelo id).
pub fn mensagens(
    conn: &Connection,
    conversa_id: i64,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<ArtefatoResumo>> {
    let c = obter(conn, conversa_id)?;
    let filtro = "a.extracao_id = ?1 AND coalesce(a.app, '') = ?2 AND a.conversa_id_externo = ?3";
    let chave = params![c.extracao_id.to_string(), c.app, c.id_externo];
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM artefato a WHERE {filtro}"),
        chave,
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUNAS_RESUMO} FROM artefato a WHERE {filtro}
         ORDER BY a.data_hora_ms IS NULL, a.data_hora_ms, a.id LIMIT ?4 OFFSET ?5"
    ))?;
    let itens = stmt
        .query_map(
            params![
                c.extracao_id.to_string(),
                c.app,
                c.id_externo,
                limite,
                deslocamento
            ],
            ler_resumo,
        )?
        .collect::<Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

/// Id da conversa a que um artefato pertence (para "abrir na conversa" a partir da
/// busca ou da visão técnica).
pub fn conversa_do_artefato(conn: &Connection, artefato_id: i64) -> Resultado<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT c.id FROM artefato a JOIN conversa c
               ON c.extracao_id = a.extracao_id AND c.app = coalesce(a.app, '')
              AND c.id_externo = a.conversa_id_externo
             WHERE a.id = ?1",
            [artefato_id],
            |r| r.get(0),
        )
        .optional()?)
}
