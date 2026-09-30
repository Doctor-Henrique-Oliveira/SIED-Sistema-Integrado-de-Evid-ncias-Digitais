//! Busca global (FTS5) nos artefatos e no inventário.
//!
//! O texto digitado nunca vai cru para o `MATCH`: cada palavra vira uma frase entre
//! aspas com busca por prefixo, para que aspas, hífens ou operadores digitados pelo
//! analista não mudem o sentido da consulta nem causem erro de sintaxe.
//!
//! Telefones: um texto que só tem dígitos e separadores (`+55 (11) 98765-4321`) é
//! buscado pelos dígitos em qualquer posição do remetente e dos destinatários, já
//! que o mesmo número aparece com e sem DDI/DDD conforme o app. Contatos também são
//! achados pelo próprio identificador.

use rusqlite::{params_from_iter, types::Value as ValorSql, Connection};
use serde::{Deserialize, Serialize};
use sied_dominio::ExtracaoId;
use ts_rs::TS;

use crate::extracoes::{ler_resumo, uuid, ArtefatoResumo, Pagina, COLUNAS_RESUMO, N_RESUMO};
use crate::Resultado;

/// Marcadores do trecho destacado. Caracteres de controle não aparecem em texto
/// de mensagem, então a interface pode separar o trecho com segurança.
pub const MARCA_INICIO: &str = "\u{1}";
pub const MARCA_FIM: &str = "\u{2}";

/// Mínimo de dígitos para tratar o texto como telefone.
const MIN_DIGITOS_TELEFONE: usize = 6;

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct FiltroBusca {
    pub texto: String,
    #[ts(optional)]
    pub extracao_id: Option<ExtracaoId>,
    #[ts(optional)]
    pub app: Option<String>,
    #[ts(optional)]
    pub tipo: Option<String>,
    /// Intervalo em ms UTC, inclusivo.
    #[ts(optional, type = "number")]
    pub de_ms: Option<i64>,
    #[ts(optional, type = "number")]
    pub ate_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AchadoArtefato {
    pub artefato: ArtefatoResumo,
    pub extracao_id: ExtracaoId,
    /// Trecho com o termo entre `\u0001` e `\u0002`.
    pub trecho: Option<String>,
    #[ts(type = "number | null")]
    pub conversa_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AchadoArquivo {
    #[ts(type = "number")]
    pub id: i64,
    pub extracao_id: ExtracaoId,
    pub caminho: String,
    pub categoria: String,
    #[ts(type = "number")]
    pub tamanho: i64,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResultadoBusca {
    /// Como o texto foi interpretado: "texto", "telefone" ou "vazio".
    pub modo: String,
    pub artefatos: Pagina<AchadoArtefato>,
    pub arquivos: Pagina<AchadoArquivo>,
}

/// Consulta FTS5 segura a partir do texto digitado. `None` se não sobrar palavra.
pub fn consulta_fts(texto: &str) -> Option<String> {
    let termos: Vec<String> = texto
        .split_whitespace()
        .map(|p| p.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|p| !p.is_empty())
        .map(|p| format!("\"{}\"*", p.replace('"', "\"\"")))
        .collect();
    (!termos.is_empty()).then(|| termos.join(" "))
}

/// Dígitos do texto, se ele parecer um telefone.
pub fn digitos_telefone(texto: &str) -> Option<String> {
    let t = texto.trim();
    let so_telefone = !t.is_empty()
        && t.chars()
            .all(|c| c.is_ascii_digit() || " +-().".contains(c));
    let digitos: String = t.chars().filter(char::is_ascii_digit).collect();
    (so_telefone && digitos.len() >= MIN_DIGITOS_TELEFONE).then_some(digitos)
}

/// Mesma normalização da migração 3 (apenas dígitos de remetente + destinatários).
/// Em contatos, o identificador do próprio contato (ex.: jid do WhatsApp) e os
/// telefones da agenda (número gravado e normalizado) também contam, para que o número
/// leve ao nome gravado no aparelho.
const PESSOAS_SO_DIGITOS: &str = "replace(replace(replace(replace(replace(replace(
    coalesce(a.remetente, '') || ' ' || a.destinatarios || ' ' ||
    CASE WHEN a.tipo = 'contato' THEN coalesce(a.id_externo, '') || ' ' || coalesce(
        (SELECT group_concat(coalesce(t.value ->> '$.numero', '') || ' ' ||
                             coalesce(t.value ->> '$.normalizado', ''), ' ')
           FROM json_each(a.metadados, '$.telefones') t), '') ELSE '' END,
    '+', ''), '-', ''), '(', ''), ')', ''), '.', ''), ' ', '')";

struct Filtros {
    sql: String,
    valores: Vec<ValorSql>,
}

fn filtros_artefato(f: &FiltroBusca) -> Filtros {
    let mut sql = String::new();
    let mut valores = Vec::new();
    let mut add = |cond: &str, v: ValorSql| {
        sql.push_str(" AND ");
        sql.push_str(cond);
        valores.push(v);
    };
    if let Some(e) = f.extracao_id {
        add("a.extracao_id = ?", ValorSql::Text(e.to_string()));
    }
    if let Some(app) = f.app.as_deref().filter(|s| !s.is_empty()) {
        add("a.app = ?", ValorSql::Text(app.into()));
    }
    if let Some(tipo) = f.tipo.as_deref().filter(|s| !s.is_empty()) {
        add("a.tipo = ?", ValorSql::Text(tipo.into()));
    }
    if let Some(de) = f.de_ms {
        add("a.data_hora_ms >= ?", ValorSql::Integer(de));
    }
    if let Some(ate) = f.ate_ms {
        add("a.data_hora_ms <= ?", ValorSql::Integer(ate));
    }
    Filtros { sql, valores }
}

fn colunas_achado() -> String {
    format!(
        "{COLUNAS_RESUMO}, a.extracao_id,
         (SELECT c.id FROM conversa c WHERE c.extracao_id = a.extracao_id
            AND c.app = coalesce(a.app, '') AND c.id_externo = a.conversa_id_externo)"
    )
}

fn ler_achado(r: &rusqlite::Row, com_trecho: bool) -> rusqlite::Result<AchadoArtefato> {
    Ok(AchadoArtefato {
        artefato: ler_resumo(r)?,
        extracao_id: uuid(r.get(N_RESUMO)?)?,
        conversa_id: r.get(N_RESUMO + 1)?,
        trecho: if com_trecho {
            r.get(N_RESUMO + 2)?
        } else {
            None
        },
    })
}

fn vazio<T: TS>() -> Pagina<T> {
    Pagina {
        total: 0,
        itens: Vec::new(),
    }
}

pub fn buscar(
    conn: &Connection,
    filtro: &FiltroBusca,
    deslocamento: i64,
    limite: i64,
) -> Resultado<ResultadoBusca> {
    let f = filtros_artefato(filtro);

    if let Some(digitos) = digitos_telefone(&filtro.texto) {
        let cond = format!("instr({PESSOAS_SO_DIGITOS}, ?) > 0{}", f.sql);
        let mut valores = vec![ValorSql::Text(digitos)];
        valores.extend(f.valores);
        let total: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM artefato a WHERE {cond}"),
            params_from_iter(valores.iter()),
            |r| r.get(0),
        )?;
        valores.push(ValorSql::Integer(limite));
        valores.push(ValorSql::Integer(deslocamento));
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM artefato a WHERE {cond}
             ORDER BY a.data_hora_ms DESC, a.id DESC LIMIT ? OFFSET ?",
            colunas_achado()
        ))?;
        let itens = stmt
            .query_map(params_from_iter(valores.iter()), |r| ler_achado(r, false))?
            .collect::<Result<_, _>>()?;
        return Ok(ResultadoBusca {
            modo: "telefone".into(),
            artefatos: Pagina { total, itens },
            arquivos: vazio(),
        });
    }

    let Some(consulta) = consulta_fts(&filtro.texto) else {
        return Ok(ResultadoBusca {
            modo: "vazio".into(),
            artefatos: vazio(),
            arquivos: vazio(),
        });
    };

    // Artefatos
    let cond = format!("busca_artefato MATCH ?{}", f.sql);
    let mut valores = vec![ValorSql::Text(consulta.clone())];
    valores.extend(f.valores);
    let total: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM busca_artefato JOIN artefato a ON a.id = busca_artefato.rowid
             WHERE {cond}"
        ),
        params_from_iter(valores.iter()),
        |r| r.get(0),
    )?;
    valores.push(ValorSql::Integer(limite));
    valores.push(ValorSql::Integer(deslocamento));
    let mut stmt = conn.prepare(&format!(
        "SELECT {},
                snippet(busca_artefato, -1, '{MARCA_INICIO}', '{MARCA_FIM}', '…', 16)
         FROM busca_artefato JOIN artefato a ON a.id = busca_artefato.rowid
         WHERE {cond} ORDER BY rank, a.data_hora_ms DESC LIMIT ? OFFSET ?",
        colunas_achado()
    ))?;
    let itens = stmt
        .query_map(params_from_iter(valores.iter()), |r| ler_achado(r, true))?
        .collect::<Result<_, _>>()?;
    let artefatos = Pagina { total, itens };

    // Arquivos (o inventário não tem app/tipo/data: só o filtro de extração vale)
    let mut cond = String::from("busca_arquivo MATCH ?");
    let mut valores = vec![ValorSql::Text(consulta)];
    if let Some(e) = filtro.extracao_id {
        cond.push_str(" AND f.extracao_id = ?");
        valores.push(ValorSql::Text(e.to_string()));
    }
    let total: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM busca_arquivo JOIN arquivo f ON f.id = busca_arquivo.rowid
             WHERE {cond}"
        ),
        params_from_iter(valores.iter()),
        |r| r.get(0),
    )?;
    valores.push(ValorSql::Integer(limite));
    valores.push(ValorSql::Integer(deslocamento));
    let mut stmt = conn.prepare(&format!(
        "SELECT f.id, f.extracao_id, f.caminho, f.categoria, f.tamanho, f.sha256
         FROM busca_arquivo JOIN arquivo f ON f.id = busca_arquivo.rowid
         WHERE {cond} ORDER BY rank, f.caminho LIMIT ? OFFSET ?"
    ))?;
    let itens = stmt
        .query_map(params_from_iter(valores.iter()), |r| {
            Ok(AchadoArquivo {
                id: r.get(0)?,
                extracao_id: uuid(r.get(1)?)?,
                caminho: r.get(2)?,
                categoria: r.get(3)?,
                tamanho: r.get(4)?,
                sha256: r.get(5)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    Ok(ResultadoBusca {
        modo: "texto".into(),
        artefatos,
        arquivos: Pagina { total, itens },
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn consulta_neutraliza_operadores() {
        assert_eq!(consulta_fts("  "), None);
        assert_eq!(consulta_fts("\"--\""), None);
        assert_eq!(
            consulta_fts("encontro OR NEAR(x) \"a\"b"),
            Some("\"encontro\"* \"OR\"* \"NEAR(x\"* \"a\"\"b\"*".into())
        );
    }

    #[test]
    fn reconhece_telefone() {
        assert_eq!(
            digitos_telefone("+55 (11) 98765-4321").as_deref(),
            Some("5511987654321")
        );
        assert_eq!(digitos_telefone("98765-4321").as_deref(), Some("987654321"));
        assert_eq!(digitos_telefone("12345"), None);
        assert_eq!(digitos_telefone("rua 123456"), None);
    }
}
