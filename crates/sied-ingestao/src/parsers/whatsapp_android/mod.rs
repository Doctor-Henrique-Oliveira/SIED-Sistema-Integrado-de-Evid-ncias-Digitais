//! WhatsApp do Android: `msgstore.db` (mensagens e conversas) e `wa.db` (contatos).
//!
//! O parser lê os bancos do próprio aplicativo, então vale para qualquer importador
//! que encontre esses arquivos. Ele não sabe de qual ferramenta de extração vieram.
//!
//! **Esquema detectado pelo conteúdo, nunca pela versão do app.** O WhatsApp mudou o
//! `msgstore.db` ao longo das versões:
//! - esquema `message` ([`novo`]): `message`, `chat`, `jid` e tabelas auxiliares;
//! - esquema `messages` ([`antigo`]): uma linha por mensagem com o jid da conversa.
//!
//! Se as duas tabelas existirem (versões de transição), vale `message`, a não ser
//! que esteja vazia e `messages` tenha linhas. A variante lida vai em
//! `metadados.esquema` de cada registro.
//!
//! Registros emitidos: `conversa` (registro da conversa no app: título do grupo,
//! tipo, membros), `mensagem` e `contato` (do `wa.db`). Nada é inventado: código
//! desconhecido vira subtipo "desconhecido" com o valor bruto preservado; coluna
//! ausente vira aviso. Os nomes de tabelas e colunas seguem a referência pública do
//! app e foram testados com bancos sintéticos (`testes/fixtures/whatsapp`); falta
//! conferir com um banco real de aparelho de teste.

mod antigo;
mod comum;
mod contatos;
mod novo;

use rusqlite::Connection;

use super::sqlite::{abrir_copia, colunas};
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};
use comum::{app_do_caminho, faltando, Emissor};

pub const ID: &str = "whatsapp_android";
pub const PAPEL_MSGSTORE: &str = "msgstore";
pub const PAPEL_WA: &str = "wa";

pub struct ParserWhatsAppAndroid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Esquema {
    Novo,
    Antigo,
}

fn linhas(conn: &Connection, tabela: &str) -> Resultado<i64> {
    Ok(
        conn.query_row(&format!("SELECT COUNT(*) FROM \"{tabela}\""), [], |r| {
            r.get(0)
        })?,
    )
}

fn detectar_esquema(conn: &Connection, origem: &str) -> Resultado<Esquema> {
    let novo = colunas(conn, "message")?;
    let antigo = colunas(conn, "messages")?;
    let falta_novo = faltando(&novo, novo::OBRIGATORIAS);
    let falta_antigo = faltando(&antigo, antigo::OBRIGATORIAS);
    match (falta_novo.is_empty(), falta_antigo.is_empty()) {
        (true, true) => {
            if linhas(conn, "message")? == 0 && linhas(conn, "messages")? > 0 {
                Ok(Esquema::Antigo)
            } else {
                Ok(Esquema::Novo)
            }
        }
        (true, false) => Ok(Esquema::Novo),
        (false, true) => Ok(Esquema::Antigo),
        (false, false) => Err(ErroIngestao::FormatoNaoReconhecido(format!(
            "{origem}: nenhum esquema conhecido do msgstore.db (tabela 'message' {}, tabela 'messages' {})",
            if novo.is_empty() {
                "ausente".to_string()
            } else {
                format!("sem {falta_novo:?}")
            },
            if antigo.is_empty() {
                "ausente".to_string()
            } else {
                format!("sem {falta_antigo:?}")
            },
        ))),
    }
}

impl Parser for ParserWhatsAppAndroid {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "WhatsApp (Android)".into(),
            versao: "1.1.0".into(),
        }
    }

    fn papeis(&self) -> &'static [&'static str] {
        &[PAPEL_MSGSTORE, PAPEL_WA]
    }

    fn processar(
        &self,
        entrada: &EntradaParser,
        saida: &mut dyn SaidaParser,
    ) -> Resultado<ResumoParser> {
        let msgstore = entrada.arquivo(PAPEL_MSGSTORE);
        let wa = entrada.arquivo(PAPEL_WA);
        let Some(principal) = msgstore.or(wa) else {
            return Err(ErroIngestao::Parser {
                parser: ID.into(),
                mensagem: format!("nenhum arquivo com papel '{PAPEL_MSGSTORE}' ou '{PAPEL_WA}'"),
            });
        };
        let app = app_do_caminho(&principal.origem);
        let mut e = Emissor::novo(saida);

        // Contatos primeiro: sem eles as conversas continuam legíveis, só sem nomes.
        if let Some(w) = wa {
            let r = abrir_copia(&w.copia_verificada)
                .and_then(|conn| contatos::processar(&conn, app, &mut e));
            match r {
                Ok(()) => {}
                Err(err @ (ErroIngestao::FormatoNaoReconhecido(_) | ErroIngestao::Sqlite(_)))
                    if msgstore.is_some() =>
                {
                    e.avisar(
                        format!("{}: não lido, conversas ficam sem nomes ({err})", w.origem),
                        PAPEL_WA,
                        None,
                    );
                }
                Err(err) => return Err(err),
            }
        }

        if let Some(m) = msgstore {
            let conn = abrir_copia(&m.copia_verificada)?;
            match detectar_esquema(&conn, m.origem.como_str())? {
                Esquema::Novo => novo::processar(&conn, app, &mut e)?,
                Esquema::Antigo => antigo::processar(&conn, app, &mut e)?,
            }
        }
        Ok(e.resumo)
    }
}

#[cfg(test)]
mod testes;
