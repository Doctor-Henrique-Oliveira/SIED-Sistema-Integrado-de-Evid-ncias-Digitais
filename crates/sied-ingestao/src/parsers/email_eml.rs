//! Mensagens RFC 5322 em arquivos `.eml`.
//!
//! O parser lê cabeçalhos e o corpo textual sem tentar adivinhar MIME complexo ou
//! decodificar anexos. O arquivo original continua no inventário; a interpretação é
//! derivada e registra a codificação declarada quando ela existe.

use serde_json::json;
use sied_dominio::artefato::{DataHora, FormatoData, NovoArtefato, TipoArtefato};
use std::fs;

use super::comum::Emissor;
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "email_eml";
pub const PAPEL: &str = "email_eml";
const LIMITE_EML: u64 = 64 * 1024 * 1024;
const LIMITE_CORPO: usize = 256 * 1024;

pub struct ParserEmailEml;

fn desdobrar_cabecalhos(texto: &str) -> (Vec<(String, String)>, &str) {
    let (cabecalho, corpo) = texto
        .split_once("\r\n\r\n")
        .or_else(|| texto.split_once("\n\n"))
        .unwrap_or((texto, ""));
    let mut campos = Vec::new();
    for linha in cabecalho.lines() {
        if linha.starts_with(' ') || linha.starts_with('\t') {
            if let Some((_, valor)) = campos.last_mut() {
                valor.push(' ');
                valor.push_str(linha.trim());
            }
        } else if let Some((nome, valor)) = linha.split_once(':') {
            campos.push((nome.trim().to_ascii_lowercase(), valor.trim().to_string()));
        }
    }
    (campos, corpo)
}

fn primeiro(campos: &[(String, String)], nome: &str) -> Option<String> {
    campos
        .iter()
        .find(|(n, _)| n == nome)
        .map(|(_, v)| v.clone())
}

fn lista(campos: &[(String, String)], nome: &str) -> Vec<String> {
    campos
        .iter()
        .filter(|(n, _)| n == nome)
        .flat_map(|(_, v)| v.split(',').map(str::trim))
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .collect()
}

fn data(v: &str) -> Option<DataHora> {
    let parsed = chrono::DateTime::parse_from_rfc2822(v).ok()?;
    Some(DataHora {
        utc_ms: sied_dominio::artefato::DataHora::ms_plausivel(parsed.timestamp_millis()),
        original: v.into(),
        formato: FormatoData::Texto,
    })
}

impl Parser for ParserEmailEml {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "E-mail RFC 5322 (.eml)".into(),
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
        if tamanho > LIMITE_EML {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{}: .eml excede o limite de {LIMITE_EML} bytes",
                arq.origem
            )));
        }
        let bytes = fs::read(&arq.copia_verificada)?;
        let texto = String::from_utf8_lossy(&bytes);
        let (campos, corpo) = desdobrar_cabecalhos(&texto);
        if campos.is_empty() {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{}: não há cabeçalhos RFC 5322 reconhecíveis",
                arq.origem
            )));
        }
        let mut origem = crate::parsers::comum::referencia(
            PAPEL,
            "rfc5322",
            "message-id",
            &serde_json::Map::from_iter([(
                "message-id".into(),
                primeiro(&campos, "message-id")
                    .map_or(serde_json::Value::Null, serde_json::Value::String),
            )]),
        );
        origem.consulta = Some("cabeçalhos RFC 5322".into());
        let mut a = NovoArtefato::novo(TipoArtefato::Mensagem, origem);
        a.app = Some("email".into());
        a.subtipo = Some("eml".into());
        a.id_externo = primeiro(&campos, "message-id");
        a.remetente = primeiro(&campos, "from");
        a.destinatarios = lista(&campos, "to");
        a.destinatarios.extend(lista(&campos, "cc"));
        a.data_hora = primeiro(&campos, "date").as_deref().and_then(data);
        let corpo = corpo.trim_matches(['\r', '\n']);
        a.conteudo = Some(corpo.chars().take(LIMITE_CORPO).collect());
        a.metadados
            .insert("assunto".into(), json!(primeiro(&campos, "subject")));
        a.metadados
            .insert("message_id".into(), json!(primeiro(&campos, "message-id")));
        if let Some(v) = primeiro(&campos, "content-transfer-encoding") {
            a.metadados
                .insert("content_transfer_encoding".into(), json!(v));
        }
        a.metadados.insert(
            "corpo_truncado".into(),
            json!(corpo.chars().count() > LIMITE_CORPO),
        );
        let mut e = Emissor::novo(saida);
        e.emitir(a)?;
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
    fn fixture_eml_sintetica() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("mensagem.eml");
        fs::write(
            &p,
            "From: Ana <ana@example.test>\r\nTo: Bruno <bruno@example.test>\r\nSubject: Teste\r\nDate: Thu, 1 Jan 2026 12:00:00 +0000\r\nMessage-ID: <sintetico-1@example.test>\r\n\r\nCorpo da mensagem.",
        )
        .unwrap();
        let entrada = EntradaParser {
            arquivos: vec![crate::parser::ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: crate::fonte::CaminhoFonte::novo("mail/mensagem.eml").unwrap(),
                sha256: "0".repeat(64),
                copia_verificada: PathBuf::from(p),
            }],
            fuso_horario: "UTC".into(),
        };
        let mut s = Saida::default();
        ParserEmailEml.processar(&entrada, &mut s).unwrap();
        assert_eq!(s.0.len(), 1);
        assert_eq!(s.0[0].remetente.as_deref(), Some("Ana <ana@example.test>"));
        assert_eq!(
            s.0[0].data_hora.as_ref().unwrap().utc_ms,
            Some(1_767_268_800_000)
        );
    }
}
