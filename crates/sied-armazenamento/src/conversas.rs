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

use rusqlite::{
    params, params_from_iter, types::Value as ValorSql, Connection, OptionalExtension, Row,
};
use serde::{Deserialize, Serialize};
use sied_dominio::ExtracaoId;
use ts_rs::TS;

use crate::busca::digitos_telefone;
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

/// Filtros da lista de conversas, aplicados no banco (sobre o caso inteiro, não só
/// sobre a página exibida).
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct FiltroConversas {
    #[ts(optional)]
    pub extracao_id: Option<ExtracaoId>,
    /// Trecho do título, do identificador da conversa, de um participante ou do nome
    /// dele nos contatos do app. Telefone também casa só pelos dígitos.
    #[ts(optional)]
    pub texto: Option<String>,
    #[ts(optional)]
    pub app: Option<String>,
    /// "grupo" ou "individual" (pelo tipo gravado pelo app; sem ele, pela quantidade
    /// de participantes, como a tela exibe).
    #[ts(optional)]
    pub tipo: Option<String>,
    /// Só conversas com ao menos um anexo.
    #[ts(optional)]
    pub com_anexo: Option<bool>,
}

/// Quantas conversas cada app tem (para os filtros da lista).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AppConversas {
    pub app: String,
    #[ts(type = "number")]
    pub conversas: i64,
}

/// Tipo gravado pelo app para a conversa (registro `conversa`).
const TIPO_REGISTRO: &str = "(SELECT r.subtipo FROM artefato r
      WHERE r.extracao_id = c.extracao_id AND r.app = c.app AND r.tipo = 'conversa'
        AND r.id_externo = c.id_externo ORDER BY r.id LIMIT 1)";

/// Participantes sem os separadores de telefone (mesma ideia da busca).
const PARTICIPANTE_SO_DIGITOS: &str = "replace(replace(replace(replace(replace(replace(
    p.identificador, '+', ''), '-', ''), '(', ''), ')', ''), '.', ''), ' ', '')";

fn condicoes(f: &FiltroConversas) -> (String, Vec<ValorSql>) {
    let mut sql = String::from("1 = 1");
    let mut v: Vec<ValorSql> = Vec::new();
    if let Some(e) = f.extracao_id {
        v.push(ValorSql::Text(e.to_string()));
        sql.push_str(&format!(" AND c.extracao_id = ?{}", v.len()));
    }
    if let Some(app) = f.app.as_deref().filter(|a| !a.is_empty()) {
        v.push(ValorSql::Text(app.into()));
        sql.push_str(&format!(" AND c.app = ?{}", v.len()));
    }
    match f.tipo.as_deref() {
        Some("grupo") => sql.push_str(&format!(
            " AND coalesce({TIPO_REGISTRO} = 'grupo',
                 (SELECT COUNT(*) FROM conversa_participante p WHERE p.conversa_id = c.id) > 1)"
        )),
        Some("individual") => sql.push_str(&format!(
            " AND NOT coalesce({TIPO_REGISTRO} = 'grupo',
                 (SELECT COUNT(*) FROM conversa_participante p WHERE p.conversa_id = c.id) > 1)"
        )),
        _ => {}
    }
    if f.com_anexo == Some(true) {
        sql.push_str(" AND c.com_anexo > 0");
    }
    if let Some(t) = f.texto.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        v.push(ValorSql::Text(t.to_lowercase()));
        let n = v.len();
        let mut ou = format!(
            "instr(lower(c.id_externo), ?{n}) > 0
             OR EXISTS (SELECT 1 FROM artefato r
                WHERE r.extracao_id = c.extracao_id AND r.app = c.app
                  AND r.tipo IN ('conversa', 'contato') AND r.id_externo = c.id_externo
                  AND instr(lower(r.conteudo), ?{n}) > 0)
             OR EXISTS (SELECT 1 FROM conversa_participante p WHERE p.conversa_id = c.id
                AND (instr(lower(p.identificador), ?{n}) > 0
                  OR EXISTS (SELECT 1 FROM artefato r
                    WHERE r.extracao_id = c.extracao_id AND r.app = c.app AND r.tipo = 'contato'
                      AND r.id_externo = p.identificador AND instr(lower(r.conteudo), ?{n}) > 0)))"
        );
        if let Some(d) = digitos_telefone(t) {
            v.push(ValorSql::Text(d));
            let n = v.len();
            ou.push_str(&format!(
                " OR EXISTS (SELECT 1 FROM conversa_participante p WHERE p.conversa_id = c.id
                    AND instr({PARTICIPANTE_SO_DIGITOS}, ?{n}) > 0)"
            ));
        }
        sql.push_str(&format!(" AND ({ou})"));
    }
    (sql, v)
}

/// Conversas do caso (com filtros), da mais recente para a mais antiga.
pub fn listar(
    conn: &Connection,
    filtro: &FiltroConversas,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<Conversa>> {
    let (onde, mut valores) = condicoes(filtro);
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM conversa c WHERE {onde}"),
        params_from_iter(valores.iter()),
        |r| r.get(0),
    )?;
    valores.push(ValorSql::Integer(limite));
    valores.push(ValorSql::Integer(deslocamento));
    let n = valores.len();
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUNAS} FROM conversa c WHERE {onde}
         ORDER BY c.ultima_ms IS NULL, c.ultima_ms DESC, c.id LIMIT ?{} OFFSET ?{n}",
        n - 1
    ))?;
    let mut itens: Vec<Conversa> = stmt
        .query_map(params_from_iter(valores.iter()), ler)?
        .collect::<Result<_, _>>()?;
    for c in &mut itens {
        c.participantes = participantes(conn, c.id, MAX_PARTICIPANTES_LISTA)?;
    }
    Ok(Pagina { total, itens })
}

/// Apps com conversas (numa extração ou no caso), com a quantidade de cada um.
pub fn apps(conn: &Connection, extracao: Option<ExtracaoId>) -> Resultado<Vec<AppConversas>> {
    let mut stmt = conn.prepare(
        "SELECT app, COUNT(*) FROM conversa WHERE ?1 IS NULL OR extracao_id = ?1
         GROUP BY app ORDER BY COUNT(*) DESC, app",
    )?;
    let v = stmt.query_map([extracao.map(|e| e.to_string())], |r| {
        Ok(AppConversas {
            app: r.get(0)?,
            conversas: r.get(1)?,
        })
    })?;
    Ok(v.collect::<Result<_, _>>()?)
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

/// Os registros de uma conversa: mesma extração, mesmo app, mesmo id externo.
const DA_CONVERSA: &str =
    "a.extracao_id = ?1 AND coalesce(a.app, '') = ?2 AND a.conversa_id_externo = ?3";

/// Ordem cronológica; sem data vão para o fim, pelo id. A posição de uma mensagem
/// (usada para rolar a lista até ela) segue esta mesma ordem.
const ORDEM_MENSAGENS: &str = "a.data_hora_ms IS NULL, a.data_hora_ms, a.id";

fn chave(conn: &Connection, conversa_id: i64) -> Resultado<(String, String, String)> {
    conn.query_row(
        "SELECT extracao_id, app, id_externo FROM conversa WHERE id = ?1",
        [conversa_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()?
    .ok_or_else(|| ErroArmazenamento::NaoEncontrado("conversa".into()))
}

/// Registros da conversa em ordem cronológica (sem data vão para o fim, pelo id).
pub fn mensagens(
    conn: &Connection,
    conversa_id: i64,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<ArtefatoResumo>> {
    let (x, app, id) = chave(conn, conversa_id)?;
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM artefato a WHERE {DA_CONVERSA}"),
        params![x, app, id],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUNAS_RESUMO} FROM artefato a WHERE {DA_CONVERSA}
         ORDER BY {ORDEM_MENSAGENS} LIMIT ?4 OFFSET ?5"
    ))?;
    let itens = stmt
        .query_map(params![x, app, id, limite, deslocamento], ler_resumo)?
        .collect::<Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

/// Posição (a partir de 0) de um registro na ordem da conversa, ou `None` se ele
/// não pertence a ela.
pub fn posicao_mensagem(
    conn: &Connection,
    conversa_id: i64,
    artefato_id: i64,
) -> Resultado<Option<i64>> {
    let (x, app, id) = chave(conn, conversa_id)?;
    let alvo: Option<Option<i64>> = conn
        .query_row(
            &format!("SELECT a.data_hora_ms FROM artefato a WHERE {DA_CONVERSA} AND a.id = ?4"),
            params![x, app, id, artefato_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(ms) = alvo else { return Ok(None) };
    let antes: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM artefato a WHERE {DA_CONVERSA} AND CASE
               WHEN ?4 IS NULL THEN a.data_hora_ms IS NOT NULL OR a.id < ?5
               ELSE a.data_hora_ms IS NOT NULL
                    AND (a.data_hora_ms < ?4 OR (a.data_hora_ms = ?4 AND a.id < ?5)) END"
        ),
        params![x, app, id, ms, artefato_id],
        |r| r.get(0),
    )?;
    Ok(Some(antes))
}

/// Uma mensagem achada ao localizar texto dentro da conversa.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PosicaoMensagem {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub posicao: i64,
}

/// Mensagens da conversa cujo texto contém `termo` (sem diferenciar maiúsculas
/// ASCII), em ordem; `total` conta todas, `itens` traz até `limite`.
pub fn localizar(
    conn: &Connection,
    conversa_id: i64,
    termo: &str,
    limite: i64,
) -> Resultado<Pagina<PosicaoMensagem>> {
    let termo = termo.trim().to_lowercase();
    if termo.is_empty() {
        return Ok(Pagina {
            total: 0,
            itens: Vec::new(),
        });
    }
    let (x, app, id) = chave(conn, conversa_id)?;
    let base = format!(
        "SELECT a.id, a.conteudo, ROW_NUMBER() OVER (ORDER BY {ORDEM_MENSAGENS}) - 1 AS posicao
         FROM artefato a WHERE {DA_CONVERSA}"
    );
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM ({base}) WHERE instr(lower(conteudo), ?4) > 0"),
        params![x, app, id, termo],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT id, posicao FROM ({base}) WHERE instr(lower(conteudo), ?4) > 0
         ORDER BY posicao LIMIT ?5"
    ))?;
    let itens = stmt
        .query_map(params![x, app, id, termo, limite], |r| {
            Ok(PosicaoMensagem {
                id: r.get(0)?,
                posicao: r.get(1)?,
            })
        })?
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
