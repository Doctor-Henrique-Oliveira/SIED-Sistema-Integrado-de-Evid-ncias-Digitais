//! Formato de troca para parsers fora do processo do SIED.
//!
//! Ainda não existe nenhum parser externo. Este módulo só fixa o formato, para que a
//! arquitetura não dependa de parsers em Rust:
//!
//! 1. o SIED inicia o processo do parser (Python, Java...) e escreve em stdin uma linha
//!    JSON com [`EntradaParser`];
//! 2. o parser escreve em stdout uma [`MensagemParser`] JSON por linha;
//! 3. a última mensagem é `fim` (sucesso) ou `falha`.
//!
//! Um adaptador em Rust implementará [`Parser`](crate::parser::Parser) sobre esse processo,
//! então o restante do pipeline (rastreabilidade, auditoria, gravação) é o mesmo.

use serde::{Deserialize, Serialize};
use sied_dominio::artefato::NovoArtefato;
use ts_rs::TS;

use crate::parser::{AvisoParser, ResumoParser};

pub const VERSAO_PROTOCOLO: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", content = "dados", rename_all = "snake_case")]
#[ts(export)]
pub enum MensagemParser {
    Artefato(Box<NovoArtefato>),
    Aviso(AvisoParser),
    Progresso { fracao: f32 },
    Fim(ResumoParser),
    Falha { mensagem: String },
}

#[cfg(test)]
mod testes {
    use super::*;
    use sied_dominio::artefato::{ReferenciaRegistro, TipoArtefato};

    #[test]
    fn linha_json_estavel() {
        let a = NovoArtefato::novo(
            TipoArtefato::Contato,
            ReferenciaRegistro {
                papel_arquivo: "contatos".into(),
                tabela: Some("raw_contacts".into()),
                id_registro: Some("_id=7".into()),
                consulta: None,
                deslocamento: None,
                valores_brutos: Default::default(),
            },
        );
        let linhas = [
            MensagemParser::Artefato(Box::new(a)),
            MensagemParser::Progresso { fracao: 0.5 },
            MensagemParser::Fim(ResumoParser {
                artefatos: 1,
                avisos: 0,
            }),
        ];
        for m in linhas {
            let txt = serde_json::to_string(&m).unwrap();
            assert!(!txt.contains('\n'), "uma mensagem por linha");
            assert_eq!(serde_json::from_str::<MensagemParser>(&txt).unwrap(), m);
        }
        let fim = serde_json::to_string(&MensagemParser::Fim(ResumoParser {
            artefatos: 2,
            avisos: 1,
        }))
        .unwrap();
        assert_eq!(fim, r#"{"tipo":"fim","dados":{"artefatos":2,"avisos":1}}"#);
    }
}
