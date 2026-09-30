//! Utilidades comuns aos parsers de bancos SQLite: linhas brutas, colunas
//! obrigatórias, tabelas auxiliares agrupadas por chave, referência de origem e o
//! emissor que conta artefatos e avisos.

use rusqlite::Connection;
use serde_json::{Map, Value};
use sied_dominio::artefato::{NovoArtefato, ReferenciaRegistro};
use std::collections::HashMap;

use super::sqlite::{colunas, linha_bruta};
use crate::parser::{AvisoParser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, Resultado};

/// Uma linha do banco, coluna a coluna, exatamente como estava.
pub type Linha = Map<String, Value>;

/// Inteiro de uma coluna. Aceita texto numérico (o esquema antigo do WhatsApp, por
/// exemplo, guarda alguns códigos como TEXT).
pub fn inteiro(l: &Linha, c: &str) -> Option<i64> {
    match l.get(c)? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

pub fn texto(l: &Linha, c: &str) -> Option<String> {
    match l.get(c)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Linhas de uma tabela agrupadas por uma coluna (ex.: `message_row_id`). Tabela ou
/// coluna ausente = vazio: cada variante de um app tem só parte das tabelas.
pub struct Auxiliar {
    pub tabela: &'static str,
    pub chave: &'static str,
    linhas: HashMap<i64, Vec<Linha>>,
}

impl Auxiliar {
    pub fn carregar(
        conn: &Connection,
        tabela: &'static str,
        chave: &'static str,
    ) -> Resultado<Self> {
        let mut aux = Auxiliar {
            tabela,
            chave,
            linhas: HashMap::new(),
        };
        let cols = colunas(conn, tabela)?;
        if !cols.iter().any(|c| c == chave) {
            return Ok(aux);
        }
        let mut stmt = conn.prepare(&format!("SELECT * FROM \"{tabela}\""))?;
        let mut rs = stmt.query([])?;
        while let Some(r) = rs.next()? {
            let l = linha_bruta(r, &cols)?;
            if let Some(k) = inteiro(&l, chave) {
                aux.linhas.entry(k).or_default().push(l);
            }
        }
        Ok(aux)
    }

    pub fn primeira(&self, chave: i64) -> Option<&Linha> {
        self.linhas.get(&chave).and_then(|v| v.first())
    }

    /// Chaves presentes, em ordem crescente.
    pub fn chaves(&self) -> Vec<i64> {
        let mut v: Vec<i64> = self.linhas.keys().copied().collect();
        v.sort_unstable();
        v
    }

    pub fn todas(&self, chave: i64) -> &[Linha] {
        self.linhas.get(&chave).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Referência de origem para uma linha desta tabela.
    pub fn referencia(&self, papel: &str, l: &Linha) -> ReferenciaRegistro {
        referencia(papel, self.tabela, self.chave, l)
    }
}

/// Referência de origem de uma linha. O identificador preferido é `_id`; sem ele,
/// a coluna-chave usada na leitura.
pub fn referencia(papel: &str, tabela: &str, chave: &str, l: &Linha) -> ReferenciaRegistro {
    let id_registro = ["_id", chave].iter().find_map(|c| {
        l.get(*c)
            .filter(|v| !v.is_null())
            .map(|v| format!("{c}={v}"))
    });
    ReferenciaRegistro {
        papel_arquivo: papel.into(),
        tabela: Some(tabela.into()),
        id_registro,
        consulta: Some(format!("SELECT * FROM {tabela}")),
        deslocamento: None,
        valores_brutos: l.clone(),
    }
}

/// Encaminha artefatos e avisos à saída, contando e respeitando o cancelamento.
pub struct Emissor<'a> {
    pub saida: &'a mut dyn SaidaParser,
    pub resumo: ResumoParser,
}

impl<'a> Emissor<'a> {
    pub fn novo(saida: &'a mut dyn SaidaParser) -> Self {
        Emissor {
            saida,
            resumo: ResumoParser::default(),
        }
    }

    pub fn emitir(&mut self, a: NovoArtefato) -> Resultado<()> {
        if self.resumo.artefatos.is_multiple_of(500) && self.saida.cancelado() {
            return Err(ErroIngestao::Cancelado);
        }
        self.saida.emitir(a)?;
        self.resumo.artefatos += 1;
        Ok(())
    }

    pub fn avisar(&mut self, mensagem: String, papel: &str, id_registro: Option<String>) {
        self.saida.avisar(AvisoParser {
            mensagem,
            papel_arquivo: Some(papel.into()),
            id_registro,
        });
        self.resumo.avisos += 1;
    }
}

/// Confere se a tabela tem as colunas pedidas. Devolve as que faltam.
pub fn faltando(cols: &[String], obrigatorias: &[&str]) -> Vec<String> {
    obrigatorias
        .iter()
        .filter(|c| !cols.iter().any(|x| x == *c))
        .map(|c| c.to_string())
        .collect()
}
