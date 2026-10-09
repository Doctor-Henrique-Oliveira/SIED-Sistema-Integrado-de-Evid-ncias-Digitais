//! Exportação JSON do Telegram Desktop.
//!
//! O exportador oficial pode colocar as mensagens na raiz (`messages`) ou dentro de
//! `chats.list[*].messages`. O parser aceita as duas formas, preserva os campos do
//! registro em `valores_brutos` e não trata o export como banco interno do aplicativo.

use serde_json::{Map, Value};
use sied_dominio::artefato::{DataHora, FormatoData, NovoArtefato, TipoArtefato};
use std::fs;

use super::comum::Emissor;
use crate::fonte::CaminhoFonte;
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "telegram_json";
pub const PAPEL: &str = "telegram_json";
const LIMITE_JSON: u64 = 256 * 1024 * 1024;

pub struct ParserTelegramJson;

fn texto(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Array(a) => {
            let s = a
                .iter()
                .filter_map(|x| match x {
                    Value::String(s) => Some(s.as_str()),
                    Value::Object(o) => o.get("text").and_then(Value::as_str),
                    _ => None,
                })
                .collect::<String>();
            (!s.is_empty()).then_some(s)
        }
        _ => None,
    }
}

fn data(v: &Value) -> Option<DataHora> {
    let s = v.as_str()?;
    if let Ok(d) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(DataHora {
            utc_ms: sied_dominio::artefato::DataHora::ms_plausivel(d.timestamp_millis()),
            original: s.into(),
            formato: FormatoData::Texto,
        });
    }
    let _d = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()?;
    Some(DataHora {
        utc_ms: None,
        original: s.into(),
        formato: FormatoData::Texto,
    })
}

fn string(v: &Map<String, Value>, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(str::to_string)
}

fn id(v: &Map<String, Value>) -> Option<String> {
    v.get("id").and_then(|x| {
        x.as_i64()
            .map(|n| n.to_string())
            .or_else(|| x.as_str().map(str::to_string))
    })
}

fn mensagens<'a>(raiz: &'a Value) -> Vec<(Option<&'a Map<String, Value>>, &'a Map<String, Value>)> {
    let mut r = Vec::new();
    if let Some(ms) = raiz.get("messages").and_then(Value::as_array) {
        r.extend(ms.iter().filter_map(|m| m.as_object().map(|o| (None, o))));
    }
    if let Some(chats) = raiz
        .get("chats")
        .and_then(|x| x.get("list"))
        .and_then(Value::as_array)
    {
        for chat in chats {
            let Some(chat) = chat.as_object() else {
                continue;
            };
            let Some(ms) = chat.get("messages").and_then(Value::as_array) else {
                continue;
            };
            r.extend(
                ms.iter()
                    .filter_map(|m| m.as_object().map(|o| (Some(chat), o))),
            );
        }
    }
    r
}

impl Parser for ParserTelegramJson {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "Telegram — exportação JSON".into(),
            versao: "1.0.0".into(),
        }
    }

    fn papeis(&self) -> &'static [&'static str] {
        &[PAPEL]
    }

    fn processar(
        &self,
        entrada: &EntradaParser,
        saida: &mut dyn SaidaParser,
    ) -> Resultado<ResumoParser> {
        let arq = entrada.arquivo(PAPEL).ok_or_else(|| ErroIngestao::Parser {
            parser: ID.into(),
            mensagem: format!("arquivo com papel '{PAPEL}' não informado"),
        })?;
        let tamanho = fs::metadata(&arq.copia_verificada)?.len();
        if tamanho > LIMITE_JSON {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{}: exportação JSON excede o limite de {LIMITE_JSON} bytes",
                arq.origem
            )));
        }
        let raiz: Value =
            serde_json::from_slice(&fs::read(&arq.copia_verificada)?).map_err(|e| {
                ErroIngestao::FormatoNaoReconhecido(format!("{}: JSON inválido: {e}", arq.origem))
            })?;
        let itens = mensagens(&raiz);
        if itens.is_empty() {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{}: exportação sem mensagens em messages ou chats.list[*].messages",
                arq.origem
            )));
        }
        let mut e = Emissor::novo(saida);
        for (chat, m) in itens {
            let Some(id_externo) = id(m) else {
                e.avisar("mensagem Telegram sem id".into(), PAPEL, None);
                continue;
            };
            let mut origem = crate::parsers::comum::referencia(
                PAPEL,
                "messages",
                "id",
                &Map::from_iter([("id".into(), Value::String(id_externo.clone()))]),
            );
            origem.consulta = Some("exportação JSON Telegram".into());
            let mut a = NovoArtefato::novo(TipoArtefato::Mensagem, origem);
            a.app = Some("telegram".into());
            a.subtipo = string(m, "type");
            a.id_externo = Some(id_externo);
            a.conteudo = m.get("text").and_then(texto);
            a.remetente = string(m, "from").or_else(|| string(m, "from_id"));
            a.data_hora = m.get("date").and_then(data);
            if let Some(chat) = chat {
                a.conversa_id_externo = id(chat).or_else(|| string(chat, "name"));
                a.metadados.insert(
                    "chat_tipo".into(),
                    chat.get("type").cloned().unwrap_or(Value::Null),
                );
                a.metadados.insert(
                    "chat_nome".into(),
                    chat.get("name").cloned().unwrap_or(Value::Null),
                );
            }
            if let Some(reply) = m.get("reply_to_message_id") {
                a.resposta_a_id_externo = reply.as_i64().map(|n| n.to_string());
            }
            if let Some(file) = string(m, "file") {
                a.anexo_caminho = Some(file);
                a.mime = string(m, "mime_type");
                a.nome_arquivo = a
                    .anexo_caminho
                    .as_deref()
                    .and_then(|p| CaminhoFonte::novo(p).ok())
                    .map(|c| c.nome().into());
            }
            for key in ["media_type", "edited", "forwarded_from", "duration_seconds"] {
                if let Some(v) = m.get(key) {
                    a.metadados.insert(key.into(), v.clone());
                }
            }
            e.emitir(a)?;
        }
        Ok(e.resumo)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::parser::AvisoParser;
    use std::path::PathBuf;

    #[derive(Default)]
    struct Saida(Vec<NovoArtefato>);
    impl SaidaParser for Saida {
        fn emitir(&mut self, a: NovoArtefato) -> Resultado<()> {
            self.0.push(a);
            Ok(())
        }
        fn avisar(&mut self, _: AvisoParser) {}
    }

    #[test]
    fn fixture_exportacao_telegram_sintetica() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("result.json");
        fs::write(
            &p,
            r#"{"chats":{"list":[{"id":7,"name":"Conversa sintética","type":"personal_chat","messages":[{"id":1,"type":"message","date":"2026-01-01T12:00:00","from":"Ana","text":"Olá","file":"photos/a.jpg","media_type":"image"}]}]}}"#,
        )
        .unwrap();
        let entrada = EntradaParser {
            arquivos: vec![crate::parser::ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: crate::fonte::CaminhoFonte::novo("Telegram Desktop/result.json").unwrap(),
                sha256: "0".repeat(64),
                copia_verificada: PathBuf::from(p),
            }],
            fuso_horario: "UTC".into(),
        };
        let mut s = Saida::default();
        ParserTelegramJson.processar(&entrada, &mut s).unwrap();
        assert_eq!(s.0.len(), 1);
        assert_eq!(s.0[0].app.as_deref(), Some("telegram"));
        assert_eq!(s.0[0].anexo_caminho.as_deref(), Some("photos/a.jpg"));
    }
}
