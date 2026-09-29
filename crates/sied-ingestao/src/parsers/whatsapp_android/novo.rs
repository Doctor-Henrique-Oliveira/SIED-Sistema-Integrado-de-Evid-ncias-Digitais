//! Esquema "novo" do `msgstore.db`: tabela `message`, com conversas em `chat` e
//! identificadores (jid) normalizados em `jid`.
//!
//! Cadeia de uma mensagem até a conversa: `message.chat_row_id` → `chat._id` →
//! `chat.jid_row_id` → `jid._id` → `jid.raw_string`. A linha da mensagem traz os
//! números dessa cadeia nos valores brutos; o registro da conversa (tipo `conversa`)
//! traz as linhas de `chat` e `jid`. As tabelas auxiliares com dado próprio da
//! mensagem (mídia, citação, localização, cartão de contato, encaminhamento, evento
//! de sistema) entram como referências adicionais de cada mensagem.

use rusqlite::Connection;
use serde_json::json;
use sied_dominio::artefato::{Confianca, NovoArtefato, TipoArtefato};
use std::collections::HashMap;

use super::super::sqlite::{colunas, linha_bruta};
use super::comum::{
    coordenadas, data_ms, faltando, inteiro, nome_do_caminho, referencia, situacao_envio,
    subtipo_mensagem, tem_arquivo, texto, tipo_conversa, Auxiliar, Emissor, Linha,
};
use super::PAPEL_MSGSTORE;
use crate::Resultado;

pub const ESQUEMA: &str = "message";
const CONSULTA: &str = "SELECT * FROM message ORDER BY _id";
const CONSULTA_CHAT: &str = "SELECT * FROM chat ORDER BY _id";
pub const OBRIGATORIAS: &[&str] = &["_id", "chat_row_id", "from_me", "key_id", "timestamp"];

/// jid._id → (texto do jid, linha bruta).
type Jids = HashMap<i64, (String, Linha)>;

fn carregar_jids(conn: &Connection) -> Resultado<Jids> {
    let cols = colunas(conn, "jid")?;
    let mut jids = HashMap::new();
    if cols.is_empty() {
        return Ok(jids);
    }
    let mut stmt = conn.prepare("SELECT * FROM jid")?;
    let mut rs = stmt.query([])?;
    while let Some(r) = rs.next()? {
        let l = linha_bruta(r, &cols)?;
        let Some(id) = inteiro(&l, "_id") else {
            continue;
        };
        let jid =
            texto(&l, "raw_string").or_else(|| match (texto(&l, "user"), texto(&l, "server")) {
                (Some(u), Some(s)) => Some(format!("{u}@{s}")),
                _ => None,
            });
        if let Some(j) = jid {
            jids.insert(id, (j, l));
        }
    }
    Ok(jids)
}

fn jid_de(jids: &Jids, l: &Linha, coluna: &str) -> Option<String> {
    inteiro(l, coluna)
        .filter(|&id| id > 0)
        .and_then(|id| jids.get(&id))
        .map(|(j, _)| j.clone())
}

/// chat._id → jid da conversa.
type Chats = HashMap<i64, String>;

pub fn processar(conn: &Connection, app: &str, e: &mut Emissor) -> Resultado<()> {
    let jids = carregar_jids(conn)?;
    let chats = conversas(conn, app, &jids, e)?;
    mensagens(conn, app, &jids, &chats, e)
}

fn conversas(conn: &Connection, app: &str, jids: &Jids, e: &mut Emissor) -> Resultado<Chats> {
    let mut chats = HashMap::new();
    let cols = colunas(conn, "chat")?;
    if !faltando(&cols, &["_id", "jid_row_id"]).is_empty() {
        e.avisar(
            "tabela 'chat' ausente ou sem 'jid_row_id': mensagens sem conversa".into(),
            PAPEL_MSGSTORE,
            None,
        );
        return Ok(chats);
    }
    let membros = Auxiliar::carregar(conn, "group_participant_user", "group_jid_row_id")?;

    let mut stmt = conn.prepare(CONSULTA_CHAT)?;
    let mut rs = stmt.query([])?;
    while let Some(r) = rs.next()? {
        let l = linha_bruta(r, &cols)?;
        let mut origem = referencia(PAPEL_MSGSTORE, "chat", "_id", &l);
        origem.consulta = Some(CONSULTA_CHAT.into());
        let (Some(id), Some(jid_id)) = (inteiro(&l, "_id"), inteiro(&l, "jid_row_id")) else {
            continue;
        };
        let Some((jid, linha_jid)) = jids.get(&jid_id) else {
            e.avisar(
                format!("conversa com jid_row_id={jid_id} sem jid correspondente"),
                PAPEL_MSGSTORE,
                origem.id_registro,
            );
            continue;
        };
        chats.insert(id, jid.clone());

        let tipo = tipo_conversa(jid);
        let mut a = NovoArtefato::novo(TipoArtefato::Conversa, origem);
        a.app = Some(app.into());
        a.subtipo = Some(tipo.into());
        a.id_externo = Some(jid.clone());
        a.conteudo = texto(&l, "subject");
        a.data_hora = data_ms(&l, "created_timestamp");
        a.origens_adicionais
            .push(referencia(PAPEL_MSGSTORE, "jid", "_id", linha_jid));

        let mut participantes = Vec::new();
        for p in membros.todas(jid_id) {
            a.origens_adicionais
                .push(membros.referencia(PAPEL_MSGSTORE, p));
            if let Some((pj, pl)) = inteiro(p, "user_jid_row_id").and_then(|u| jids.get(&u)) {
                participantes.push(json!({
                    "jid": pj,
                    "rank": p.get("rank"),
                    "pendente": p.get("pending"),
                }));
                a.origens_adicionais
                    .push(referencia(PAPEL_MSGSTORE, "jid", "_id", pl));
            }
        }
        let m = &mut a.metadados;
        m.insert("esquema".into(), json!(ESQUEMA));
        m.insert("tipo_conversa".into(), json!(tipo));
        m.insert("jid".into(), json!(jid));
        for c in ["archived", "hidden", "group_type"] {
            if let Some(v) = l.get(c).filter(|v| !v.is_null()) {
                m.insert(c.into(), v.clone());
            }
        }
        if !participantes.is_empty() {
            m.insert("participantes".into(), json!(participantes));
        }
        e.emitir(a)?;
    }
    Ok(chats)
}

struct Auxiliares {
    midia: Auxiliar,
    citacao: Auxiliar,
    local: Auxiliar,
    vcard: Auxiliar,
    encaminhada: Auxiliar,
    sistema: Auxiliar,
    revogada: Auxiliar,
}

fn mensagens(
    conn: &Connection,
    app: &str,
    jids: &Jids,
    chats: &Chats,
    e: &mut Emissor,
) -> Resultado<()> {
    let x = Auxiliares {
        midia: Auxiliar::carregar(conn, "message_media", "message_row_id")?,
        citacao: Auxiliar::carregar(conn, "message_quoted", "message_row_id")?,
        local: Auxiliar::carregar(conn, "message_location", "message_row_id")?,
        vcard: Auxiliar::carregar(conn, "message_vcard", "message_row_id")?,
        encaminhada: Auxiliar::carregar(conn, "message_forwarded", "message_row_id")?,
        sistema: Auxiliar::carregar(conn, "message_system", "message_row_id")?,
        revogada: Auxiliar::carregar(conn, "message_revoked", "message_row_id")?,
    };
    let cols = colunas(conn, "message")?;
    let mut stmt = conn.prepare(CONSULTA)?;
    let mut rs = stmt.query([])?;
    while let Some(r) = rs.next()? {
        let l = linha_bruta(r, &cols)?;
        let mut origem = referencia(PAPEL_MSGSTORE, "message", "_id", &l);
        origem.consulta = Some(CONSULTA.into());
        let Some(id) = inteiro(&l, "_id") else {
            continue;
        };
        let id_registro = origem.id_registro.clone();
        let mut a = NovoArtefato::novo(TipoArtefato::Mensagem, origem);
        a.app = Some(app.into());
        a.id_externo = texto(&l, "key_id");
        a.data_hora = data_ms(&l, "timestamp");
        a.conteudo = texto(&l, "text_data");

        let chat_id = inteiro(&l, "chat_row_id");
        let conversa = chat_id.and_then(|c| chats.get(&c)).cloned();
        if conversa.is_none() {
            e.avisar(
                format!(
                    "conversa não encontrada para chat_row_id={}",
                    chat_id.map_or("vazio".into(), |c| c.to_string())
                ),
                PAPEL_MSGSTORE,
                id_registro.clone(),
            );
            a.confianca = Confianca::Media;
        }
        let grupo = conversa.as_deref().map(tipo_conversa) == Some("grupo");
        let de_mim = inteiro(&l, "from_me") == Some(1);
        a.de_mim = Some(de_mim);
        a.conversa_id_externo = conversa.clone();

        let codigo = inteiro(&l, "message_type");
        let subtipo = codigo.and_then(subtipo_mensagem);
        a.subtipo = Some(subtipo.unwrap_or("desconhecido").into());
        if subtipo.is_none() {
            a.confianca = Confianca::Media;
        }

        if de_mim {
            if let (false, Some(c)) = (grupo, &conversa) {
                a.destinatarios.push(c.clone());
            }
        } else if grupo {
            a.remetente = jid_de(jids, &l, "sender_jid_row_id");
            if a.remetente.is_none() && subtipo != Some("sistema") {
                e.avisar(
                    "mensagem de grupo sem remetente identificado".into(),
                    PAPEL_MSGSTORE,
                    id_registro.clone(),
                );
                a.confianca = Confianca::Media;
            }
        } else {
            a.remetente = conversa.clone();
        }

        let mut m = serde_json::Map::new();
        m.insert("esquema".into(), json!(ESQUEMA));
        m.insert("tipo_mensagem".into(), json!(codigo));
        if let Some(s) = inteiro(&l, "status") {
            m.insert("status".into(), json!(s));
            if let Some(sit) = situacao_envio(de_mim, s) {
                m.insert("situacao".into(), json!(sit));
            }
        }
        for c in [
            "received_timestamp",
            "receipt_server_timestamp",
            "starred",
            "broadcast",
        ] {
            if let Some(v) = inteiro(&l, c).filter(|&v| v > 0) {
                m.insert(c.into(), json!(v));
            }
        }

        if let Some(md) = x.midia.primeira(id) {
            let caminho = texto(md, "file_path");
            if subtipo.is_some_and(tem_arquivo) || caminho.is_some() {
                a.anexo_caminho = caminho.clone();
                a.mime = texto(md, "mime_type");
                a.nome_arquivo = texto(md, "media_name")
                    .or_else(|| caminho.as_deref().and_then(nome_do_caminho));
                a.tamanho = inteiro(md, "file_size")
                    .filter(|&t| t > 0)
                    .map(|t| t as u64);
            }
            if let Some(d) = inteiro(md, "media_duration").filter(|&d| d > 0) {
                m.insert("duracao_s".into(), json!(d));
            }
            a.origens_adicionais
                .push(x.midia.referencia(PAPEL_MSGSTORE, md));
        }
        if let Some(q) = x.citacao.primeira(id) {
            a.resposta_a_id_externo = texto(q, "key_id");
            m.insert(
                "citacao".into(),
                json!({ "key_id": texto(q, "key_id"), "texto": texto(q, "text_data") }),
            );
            a.origens_adicionais
                .push(x.citacao.referencia(PAPEL_MSGSTORE, q));
        }
        if let Some(loc) = x.local.primeira(id) {
            let (coord, descartadas) = coordenadas(loc);
            if let Some((la, lo)) = coord {
                a.latitude = Some(la);
                a.longitude = Some(lo);
            }
            if descartadas {
                m.insert("coordenadas_invalidas".into(), json!(true));
            }
            for (c, k) in [
                ("place_name", "local_nome"),
                ("place_address", "local_endereco"),
            ] {
                if let Some(v) = texto(loc, c) {
                    m.insert(k.into(), json!(v));
                }
            }
            if a.conteudo.is_none() {
                a.conteudo = texto(loc, "place_name");
            }
            a.origens_adicionais
                .push(x.local.referencia(PAPEL_MSGSTORE, loc));
        }
        let cartoes = x.vcard.todas(id);
        if !cartoes.is_empty() {
            m.insert(
                "vcards".into(),
                json!(cartoes
                    .iter()
                    .map(|c| texto(c, "vcard"))
                    .collect::<Vec<_>>()),
            );
            for c in cartoes {
                a.origens_adicionais
                    .push(x.vcard.referencia(PAPEL_MSGSTORE, c));
            }
        }
        if let Some(f) = x.encaminhada.primeira(id) {
            m.insert("encaminhada".into(), json!(true));
            if let Some(s) = inteiro(f, "forward_score") {
                m.insert("forward_score".into(), json!(s));
            }
            a.origens_adicionais
                .push(x.encaminhada.referencia(PAPEL_MSGSTORE, f));
        }
        if let Some(s) = x.sistema.primeira(id) {
            m.insert("acao_sistema".into(), json!(inteiro(s, "action_type")));
            a.origens_adicionais
                .push(x.sistema.referencia(PAPEL_MSGSTORE, s));
        }
        if subtipo == Some("apagada") {
            m.insert("apagada_para_todos".into(), json!(true));
        }
        if let Some(rv) = x.revogada.primeira(id) {
            a.origens_adicionais
                .push(x.revogada.referencia(PAPEL_MSGSTORE, rv));
        }
        a.metadados = m;
        e.emitir(a)?;
    }
    Ok(())
}
