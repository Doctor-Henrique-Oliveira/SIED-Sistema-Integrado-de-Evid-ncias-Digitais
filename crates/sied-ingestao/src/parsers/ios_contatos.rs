//! Contatos de backups iOS (`AddressBook.sqlitedb`).
//!
//! O parser é deliberadamente conservador: só reconhece um banco que contenha a
//! tabela `ABPerson`. As colunas completas são preservadas como metadados brutos;
//! nenhum campo ausente é inferido e tabelas de versões desconhecidas não são
//! interpretadas como se fossem o esquema esperado.

use rusqlite::Connection;
use serde_json::{Map, Value};
use sied_dominio::artefato::{NovoArtefato, TipoArtefato};

use super::comum::{referencia, Emissor};
use super::sqlite::{abrir_copia, colunas, linha_bruta};
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "ios_contatos";
pub const PAPEL: &str = "ios_contatos";
const TABELA: &str = "ABPerson";

pub struct ParserIosContatos;

fn nome_coluna<'a>(linha: &'a Map<String, Value>, nomes: &[&str]) -> Option<&'a str> {
    nomes.iter().find_map(|nome| {
        linha.iter().find_map(|(chave, valor)| {
            (chave.eq_ignore_ascii_case(nome))
                .then(|| valor.as_str())
                .flatten()
        })
    })
}

fn identificador_sql(coluna: &str) -> String {
    format!("\"{}\"", coluna.replace('"', "\"\""))
}

fn colunas_abperson(conn: &Connection) -> Resultado<Vec<String>> {
    let disponiveis = colunas(conn, TABELA)?;
    if disponiveis.is_empty() {
        return Err(ErroIngestao::FormatoNaoReconhecido(
            "ios_contatos: tabela ABPerson sem colunas".into(),
        ));
    }
    // A consulta é limitada para evitar que um banco hostil materialize uma seleção
    // arbitrariamente grande de colunas no derivado.
    let mut selecao = vec!["rowid AS __rowid".to_string()];
    selecao.extend(
        disponiveis
            .into_iter()
            .take(100)
            .map(|coluna| identificador_sql(&coluna)),
    );
    Ok(selecao)
}

impl Parser for ParserIosContatos {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "Contatos iOS".into(),
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
        let selecao = colunas_abperson(&conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM {TABELA} ORDER BY rowid",
            selecao.join(", ")
        ))?;
        let mut rows = stmt.query([])?;
        let mut emissor = Emissor::novo(saida);
        while let Some(row) = rows.next()? {
            let linha = linha_bruta(row, &selecao)?;
            let Some(id) = linha.get("__rowid").and_then(Value::as_i64) else {
                emissor.avisar("ABPerson sem rowid legível".into(), PAPEL, None);
                continue;
            };
            let mut artefato = NovoArtefato::novo(
                TipoArtefato::Contato,
                referencia(PAPEL, TABELA, "__rowid", &linha),
            );
            artefato.app = Some("contacts_ios".into());
            artefato.subtipo = Some("contato".into());
            artefato.id_externo = Some(id.to_string());
            artefato.conteudo = nome_coluna(
                &linha,
                &[
                    "First",
                    "FirstName",
                    "Last",
                    "LastName",
                    "Organization",
                    "Name",
                ],
            )
            .map(str::to_owned);
            artefato
                .metadados
                .insert("esquema".into(), Value::from("ios_addressbook_abperson"));
            artefato
                .metadados
                .insert("colunas_brutas".into(), Value::Object(linha));
            artefato.origem.id_registro = Some(format!("rowid={id}"));
            emissor.emitir(artefato)?;
        }
        Ok(emissor.resumo)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fonte::CaminhoFonte;
    use crate::parser::{ArquivoDeTrabalho, AvisoParser};
    use std::path::PathBuf;

    #[derive(Default)]
    struct Saida(Vec<NovoArtefato>);
    impl SaidaParser for Saida {
        fn emitir(&mut self, artefato: NovoArtefato) -> Resultado<()> {
            self.0.push(artefato);
            Ok(())
        }
        fn avisar(&mut self, _: AvisoParser) {}
    }

    #[test]
    fn fixture_abperson_sintetica_preserva_colunas_brutas() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("AddressBook.sqlitedb");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE ABPerson(First TEXT, Last TEXT, Phone TEXT);
             INSERT INTO ABPerson VALUES('Ana','Teste','+5511900000000');",
        )
        .unwrap();
        let entrada = EntradaParser {
            arquivos: vec![ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: CaminhoFonte::novo("Library/AddressBook/AddressBook.sqlitedb").unwrap(),
                sha256: "0".repeat(64),
                copia_verificada: PathBuf::from(db),
            }],
            fuso_horario: "UTC".into(),
        };
        let mut saida = Saida::default();
        ParserIosContatos.processar(&entrada, &mut saida).unwrap();
        assert_eq!(saida.0.len(), 1);
        assert_eq!(saida.0[0].conteudo.as_deref(), Some("Ana"));
        assert_eq!(saida.0[0].metadados["esquema"], "ios_addressbook_abperson");
        assert!(saida.0[0].metadados["colunas_brutas"]["Phone"] == "+5511900000000");
    }

    #[test]
    fn banco_sem_abperson_e_recusado() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("sem_addressbook.db");
        Connection::open(&db)
            .unwrap()
            .execute_batch("CREATE TABLE outra(id INTEGER);")
            .unwrap();
        let entrada = EntradaParser {
            arquivos: vec![ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: CaminhoFonte::novo("x.db").unwrap(),
                sha256: "0".repeat(64),
                copia_verificada: PathBuf::from(db),
            }],
            fuso_horario: "UTC".into(),
        };
        assert!(matches!(
            ParserIosContatos.processar(&entrada, &mut Saida::default()),
            Err(ErroIngestao::FormatoNaoReconhecido(_))
        ));
    }
}
