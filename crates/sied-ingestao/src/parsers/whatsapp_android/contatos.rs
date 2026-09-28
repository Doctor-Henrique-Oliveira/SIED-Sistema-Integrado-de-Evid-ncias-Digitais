//! Contatos do `wa.db` (tabela `wa_contacts`).
//!
//! `display_name` é o nome gravado na agenda do aparelho; `wa_name` é o nome que a
//! própria pessoa definiu no perfil do WhatsApp. O nome exibido prefere a agenda e
//! os metadados dizem de onde ele veio.

use rusqlite::Connection;
use serde_json::json;
use sied_dominio::artefato::{NovoArtefato, TipoArtefato};

use super::super::sqlite::{colunas, linha_bruta};
use super::comum::{faltando, inteiro, referencia, texto, tipo_conversa, Emissor};
use super::PAPEL_WA;
use crate::{ErroIngestao, Resultado};

const TABELA: &str = "wa_contacts";
const CONSULTA: &str = "SELECT * FROM wa_contacts ORDER BY _id";

pub fn processar(conn: &Connection, app: &str, e: &mut Emissor) -> Resultado<()> {
    let cols = colunas(conn, TABELA)?;
    if cols.is_empty() {
        return Err(ErroIngestao::FormatoNaoReconhecido(
            "wa.db: tabela 'wa_contacts' não encontrada".into(),
        ));
    }
    let falta = faltando(&cols, &["_id", "jid"]);
    if !falta.is_empty() {
        return Err(ErroIngestao::FormatoNaoReconhecido(format!(
            "wa.db: tabela 'wa_contacts' sem as colunas {falta:?}"
        )));
    }

    let mut stmt = conn.prepare(CONSULTA)?;
    let mut rs = stmt.query([])?;
    while let Some(r) = rs.next()? {
        let l = linha_bruta(r, &cols)?;
        let mut origem = referencia(PAPEL_WA, TABELA, "_id", &l);
        origem.consulta = Some(CONSULTA.into());
        let Some(jid) = texto(&l, "jid") else {
            e.avisar(
                "contato sem 'jid'; registro ignorado".into(),
                PAPEL_WA,
                origem.id_registro,
            );
            continue;
        };

        let agenda = texto(&l, "display_name");
        let perfil = texto(&l, "wa_name");
        let mut a = NovoArtefato::novo(TipoArtefato::Contato, origem);
        a.app = Some(app.into());
        a.subtipo = Some(if tipo_conversa(&jid) == "grupo" {
            "grupo".into()
        } else {
            "contato".into()
        });
        a.id_externo = Some(jid.clone());
        a.conteudo = agenda.clone().or_else(|| perfil.clone());

        let m = &mut a.metadados;
        m.insert("jid".into(), json!(jid));
        m.insert(
            "fonte_do_nome".into(),
            json!(match (&agenda, &perfil) {
                (Some(_), _) => Some("agenda"),
                (None, Some(_)) => Some("perfil_whatsapp"),
                _ => None,
            }),
        );
        if let Some(n) = agenda {
            m.insert("nome_agenda".into(), json!(n));
        }
        if let Some(n) = perfil {
            m.insert("nome_perfil".into(), json!(n));
        }
        for (coluna, chave) in [
            ("number", "numero"),
            ("status", "recado"),
            ("given_name", "nome"),
            ("family_name", "sobrenome"),
            ("nickname", "apelido"),
            ("company", "empresa"),
        ] {
            if let Some(v) = texto(&l, coluna) {
                m.insert(chave.into(), json!(v));
            }
        }
        if let Some(v) = inteiro(&l, "status_timestamp").filter(|&v| v > 0) {
            m.insert("recado_em_ms".into(), json!(v));
        }
        if let Some(v) = inteiro(&l, "is_whatsapp_user") {
            m.insert("usuario_whatsapp".into(), json!(v == 1));
        }
        e.emitir(a)?;
    }
    Ok(())
}
