//! Esquema "antigo" do `msgstore.db`: tabela `messages`, com o jid da conversa em
//! cada linha (`key_remote_jid`), conversas em `chat_list` e membros de grupo em
//! `group_participants`.
//!
//! Mídia: o caminho do arquivo vem de `message_media` quando a tabela existe; sem
//! ela, só nome, tipo e tamanho gravados na própria linha. Citação: `quoted_row_id`
//! aponta para `messages_quotes._id`. Mensagem de sistema: `status = 6`.

use rusqlite::Connection;
use serde_json::json;
use sied_dominio::artefato::{Confianca, NovoArtefato, TipoArtefato};
use std::collections::HashMap;

use super::super::sqlite::{colunas, linha_bruta};
use super::comum::{
    data_ms, faltando, inteiro, nome_do_caminho, real, referencia, situacao_envio,
    subtipo_mensagem, tem_arquivo, texto, tipo_conversa, Auxiliar, Emissor, Linha,
};
use super::PAPEL_MSGSTORE;
use crate::Resultado;

pub const ESQUEMA: &str = "messages";
const CONSULTA: &str = "SELECT * FROM messages ORDER BY _id";
const CONSULTA_CHAT: &str = "SELECT * FROM chat_list ORDER BY _id";
pub const OBRIGATORIAS: &[&str] = &[
    "_id",
    "key_remote_jid",
    "key_from_me",
    "key_id",
    "timestamp",
];

pub fn processar(conn: &Connection, app: &str, e: &mut Emissor) -> Resultado<()> {
    conversas(conn, app, e)?;
    mensagens(conn, app, e)
}

fn conversas(conn: &Connection, app: &str, e: &mut Emissor) -> Resultado<()> {
    let cols = colunas(conn, "chat_list")?;
    if !faltando(&cols, &["_id", "key_remote_jid"]).is_empty() {
        return Ok(()); // sem a lista de conversas, as mensagens agrupam sozinhas
    }
    // gjid → linhas de group_participants
    let mut membros: HashMap<String, Vec<Linha>> = HashMap::new();
    let cols_membros = colunas(conn, "group_participants")?;
    if faltando(&cols_membros, &["gjid", "jid"]).is_empty() {
        let mut stmt = conn.prepare("SELECT * FROM group_participants ORDER BY _id")?;
        let mut rs = stmt.query([])?;
        while let Some(r) = rs.next()? {
            let l = linha_bruta(r, &cols_membros)?;
            if let Some(g) = texto(&l, "gjid") {
                membros.entry(g).or_default().push(l);
            }
        }
    }

    let mut stmt = conn.prepare(CONSULTA_CHAT)?;
    let mut rs = stmt.query([])?;
    while let Some(r) = rs.next()? {
        let l = linha_bruta(r, &cols)?;
        let mut origem = referencia(PAPEL_MSGSTORE, "chat_list", "_id", &l);
        origem.consulta = Some(CONSULTA_CHAT.into());
        let Some(jid) = texto(&l, "key_remote_jid") else {
            continue;
        };
        let tipo = tipo_conversa(&jid);
        let mut a = NovoArtefato::novo(TipoArtefato::Conversa, origem);
        a.app = Some(app.into());
        a.subtipo = Some(tipo.into());
        a.id_externo = Some(jid.clone());
        a.conteudo = texto(&l, "subject");
        a.data_hora = data_ms(&l, "creation");

        let mut participantes = Vec::new();
        for p in membros.get(&jid).map(Vec::as_slice).unwrap_or(&[]) {
            if let Some(pj) = texto(p, "jid").filter(|j| !j.is_empty()) {
                participantes.push(json!({ "jid": pj, "admin": p.get("admin") }));
            }
            let mut r = referencia(PAPEL_MSGSTORE, "group_participants", "_id", p);
            r.consulta = Some("SELECT * FROM group_participants ORDER BY _id".into());
            a.origens_adicionais.push(r);
        }
        let m = &mut a.metadados;
        m.insert("esquema".into(), json!(ESQUEMA));
        m.insert("tipo_conversa".into(), json!(tipo));
        m.insert("jid".into(), json!(jid));
        if let Some(v) = l.get("archived").filter(|v| !v.is_null()) {
            m.insert("archived".into(), v.clone());
        }
        if !participantes.is_empty() {
            m.insert("participantes".into(), json!(participantes));
        }
        e.emitir(a)?;
    }
    Ok(())
}

fn mensagens(conn: &Connection, app: &str, e: &mut Emissor) -> Resultado<()> {
    let midia = Auxiliar::carregar(conn, "message_media", "message_row_id")?;
    let citacoes = Auxiliar::carregar(conn, "messages_quotes", "_id")?;
    let cols = colunas(conn, "messages")?;
    let mut stmt = conn.prepare(CONSULTA)?;
    let mut rs = stmt.query([])?;
    while let Some(r) = rs.next()? {
        let l = linha_bruta(r, &cols)?;
        let mut origem = referencia(PAPEL_MSGSTORE, "messages", "_id", &l);
        origem.consulta = Some(CONSULTA.into());
        let Some(id) = inteiro(&l, "_id") else {
            continue;
        };
        let id_registro = origem.id_registro.clone();
        let mut a = NovoArtefato::novo(TipoArtefato::Mensagem, origem);
        a.app = Some(app.into());
        a.id_externo = texto(&l, "key_id");
        a.data_hora = data_ms(&l, "timestamp");
        a.conteudo = texto(&l, "data").or_else(|| texto(&l, "media_caption"));

        let conversa = texto(&l, "key_remote_jid");
        let grupo = conversa.as_deref().map(tipo_conversa) == Some("grupo");
        let de_mim = inteiro(&l, "key_from_me") == Some(1);
        a.de_mim = Some(de_mim);
        a.conversa_id_externo = conversa.clone();

        let status = inteiro(&l, "status");
        let codigo = inteiro(&l, "media_wa_type");
        let subtipo = if status == Some(6) {
            Some("sistema")
        } else {
            codigo.and_then(subtipo_mensagem)
        };
        a.subtipo = Some(subtipo.unwrap_or("desconhecido").into());
        if subtipo.is_none() {
            a.confianca = Confianca::Media;
        }

        if de_mim {
            if let (false, Some(c)) = (grupo, &conversa) {
                a.destinatarios.push(c.clone());
            }
        } else if grupo {
            a.remetente = texto(&l, "remote_resource");
            if a.remetente.is_none() && subtipo != Some("sistema") {
                e.avisar(
                    "mensagem de grupo sem remetente (remote_resource vazio)".into(),
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
        if let Some(s) = status {
            m.insert("status".into(), json!(s));
            if let Some(sit) = situacao_envio(de_mim, s) {
                m.insert("situacao".into(), json!(sit));
            }
        }
        for c in ["received_timestamp", "receipt_server_timestamp"] {
            if let Some(v) = inteiro(&l, c).filter(|&v| v > 0) {
                m.insert(c.into(), json!(v));
            }
        }

        if subtipo.is_some_and(tem_arquivo) {
            a.mime = texto(&l, "media_mime_type");
            a.nome_arquivo = texto(&l, "media_name");
            a.tamanho = inteiro(&l, "media_size")
                .filter(|&t| t > 0)
                .map(|t| t as u64);
            if let Some(d) = inteiro(&l, "media_duration").filter(|&d| d > 0) {
                m.insert("duracao_s".into(), json!(d));
            }
        }
        if let Some(md) = midia.primeira(id) {
            let caminho = texto(md, "file_path");
            a.anexo_caminho = caminho.clone();
            a.mime = texto(md, "mime_type").or(a.mime.take());
            a.nome_arquivo = texto(md, "media_name")
                .or(a.nome_arquivo.take())
                .or_else(|| caminho.as_deref().and_then(nome_do_caminho));
            a.tamanho = inteiro(md, "file_size")
                .filter(|&t| t > 0)
                .map(|t| t as u64)
                .or(a.tamanho);
            a.origens_adicionais
                .push(midia.referencia(PAPEL_MSGSTORE, md));
        }
        if matches!(subtipo, Some("localizacao" | "localizacao_tempo_real")) {
            a.latitude = real(&l, "latitude");
            a.longitude = real(&l, "longitude");
            if let Some(nome) = texto(&l, "media_name") {
                m.insert("local_nome".into(), json!(nome));
                if a.conteudo.is_none() {
                    a.conteudo = Some(nome);
                }
            }
        }
        if let Some(q) = inteiro(&l, "quoted_row_id")
            .filter(|&q| q > 0)
            .and_then(|q| citacoes.primeira(q))
        {
            a.resposta_a_id_externo = texto(q, "key_id");
            m.insert(
                "citacao".into(),
                json!({ "key_id": texto(q, "key_id"), "texto": texto(q, "data") }),
            );
            a.origens_adicionais
                .push(citacoes.referencia(PAPEL_MSGSTORE, q));
        }
        if inteiro(&l, "forwarded").is_some_and(|f| f > 0) {
            m.insert("encaminhada".into(), json!(true));
        }
        if subtipo == Some("apagada") {
            m.insert("apagada_para_todos".into(), json!(true));
        }
        a.metadados = m;
        e.emitir(a)?;
    }
    Ok(())
}
