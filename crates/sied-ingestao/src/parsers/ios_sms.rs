//! Mensagens do iOS (`sms.db`), usando somente o esquema público mínimo da tabela
//! `message`. O parser não depende do nome da ferramenta que produziu a extração.
//!
//! A estrutura do banco varia entre versões. Por isso a tabela e as colunas são
//! conferidas antes da leitura; colunas opcionais ficam nos valores brutos quando
//! existem, e uma coluna ausente não é preenchida por suposição.

use rusqlite::Connection;
use serde_json::json;
use sied_dominio::artefato::{DataHora, FormatoData, NovoArtefato, TipoArtefato};

use super::comum::{inteiro, referencia, texto, Emissor};
use super::sqlite::{abrir_copia, colunas, linha_bruta};
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "ios_sms";
pub const PAPEL: &str = "ios_sms";

const OPCIONAIS: &[&str] = &[
    "handle_id",
    "service",
    "subject",
    "is_read",
    "is_delivered",
    "date_read",
    "date_delivered",
];

pub struct ParserIosSms;

fn data_cocoa(v: i64) -> DataHora {
    let ms = v
        .checked_mul(1_000)
        .and_then(|x| x.checked_add(978_307_200_000));
    DataHora {
        utc_ms: ms.and_then(DataHora::ms_plausivel),
        original: v.to_string(),
        formato: FormatoData::Cocoa,
    }
}

fn colunas_leitura(conn: &Connection) -> Resultado<Vec<String>> {
    let disponiveis = colunas(conn, "message")?;
    if !disponiveis.iter().any(|c| c == "text")
        || !disponiveis.iter().any(|c| c == "date")
        || !disponiveis.iter().any(|c| c == "is_from_me")
    {
        return Err(ErroIngestao::FormatoNaoReconhecido(
            "ios_sms: tabela message sem text, date e is_from_me".into(),
        ));
    }
    let mut lidas = vec![
        "rowid AS __rowid".to_string(),
        "text".into(),
        "date".into(),
        "is_from_me".into(),
    ];
    lidas.extend(
        OPCIONAIS
            .iter()
            .filter(|c| disponiveis.iter().any(|x| x == *c))
            .map(|c| (*c).into()),
    );
    Ok(lidas)
}

impl Parser for ParserIosSms {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "Mensagens iOS".into(),
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
        let conn = abrir_copia(&arq.copia_verificada)?;
        let selecao = colunas_leitura(&conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM message ORDER BY rowid",
            selecao.join(", ")
        ))?;
        let mut rows = stmt.query([])?;
        let mut e = Emissor::novo(saida);
        while let Some(row) = rows.next()? {
            let l = linha_bruta(row, &selecao)?;
            let Some(id) = inteiro(&l, "__rowid") else {
                e.avisar("message sem rowid legível".into(), PAPEL, None);
                continue;
            };
            let mut a = NovoArtefato::novo(
                TipoArtefato::Mensagem,
                referencia(PAPEL, "message", "__rowid", &l),
            );
            a.app = Some("messages_ios".into());
            a.subtipo = Some("sms".into());
            a.conteudo = texto(&l, "text");
            a.data_hora = inteiro(&l, "date").map(data_cocoa);
            let de_mim = inteiro(&l, "is_from_me") == Some(1);
            a.de_mim = Some(de_mim);
            if let Some(handle) =
                texto(&l, "handle_id").or_else(|| inteiro(&l, "handle_id").map(|v| v.to_string()))
            {
                a.conversa_id_externo = Some(handle.clone());
                if de_mim {
                    a.destinatarios.push(handle);
                } else {
                    a.remetente = Some(handle);
                }
            }
            a.metadados
                .insert("esquema".into(), json!("ios_sms_message"));
            for c in OPCIONAIS {
                if let Some(v) = l.get(*c).filter(|v| !v.is_null()) {
                    a.metadados.insert((*c).into(), v.clone());
                }
            }
            a.origem.id_registro = Some(format!("rowid={id}"));
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
    use std::sync::Mutex;

    #[derive(Default)]
    struct Saida {
        itens: Mutex<Vec<NovoArtefato>>,
    }
    impl SaidaParser for Saida {
        fn emitir(&mut self, a: NovoArtefato) -> Resultado<()> {
            self.itens.get_mut().unwrap().push(a);
            Ok(())
        }
        fn avisar(&mut self, _: AvisoParser) {}
    }

    #[test]
    fn data_cocoa_preserva_original_e_converte() {
        let d = data_cocoa(757_382_400);
        assert_eq!(d.formato, FormatoData::Cocoa);
        assert_eq!(d.utc_ms, Some(1_735_689_600_000));
        assert_eq!(d.original, "757382400");
    }

    #[test]
    fn fixture_ios_sms_sintetica() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("sms.db");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE message(text TEXT, date INTEGER, is_from_me INTEGER, handle_id TEXT);\
             INSERT INTO message VALUES('mensagem iOS', 757382400, 0, '+5511900000000');",
        )
        .unwrap();
        let mut saida = Saida::default();
        let entrada = EntradaParser {
            arquivos: vec![crate::parser::ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: crate::fonte::CaminhoFonte::novo("Library/SMS/sms.db").unwrap(),
                sha256: "0".repeat(64),
                copia_verificada: PathBuf::from(db),
            }],
            fuso_horario: "UTC".into(),
        };
        ParserIosSms.processar(&entrada, &mut saida).unwrap();
        let itens = saida.itens.into_inner().unwrap();
        assert_eq!(itens.len(), 1);
        assert_eq!(itens[0].app.as_deref(), Some("messages_ios"));
        assert_eq!(itens[0].remetente.as_deref(), Some("+5511900000000"));
        assert_eq!(itens[0].origem.tabela.as_deref(), Some("message"));
    }
}
