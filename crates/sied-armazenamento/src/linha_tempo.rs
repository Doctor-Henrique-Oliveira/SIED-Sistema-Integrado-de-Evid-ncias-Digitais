//! Linha do tempo do caso: todos os registros com data, de todas as extrações e apps,
//! em ordem cronológica (mensagens, chamadas, registros com coordenadas...).
//!
//! Só consulta: nada é gravado nem deduzido. A ordem é a da data gravada pelo parser
//! (`data_hora_ms`, UTC); registros sem data (ou com data implausível) ficam fora da
//! lista e são contados à parte, para o analista saber que existem.
//!
//! O registro "conversa" não entra: é o cabeçalho da conversa no app (a data dele é a
//! da última mensagem, que já está na linha do tempo).

use rusqlite::{params_from_iter, types::Value as ValorSql, Connection};
use serde::{Deserialize, Serialize};
use sied_dominio::ExtracaoId;
use ts_rs::TS;

use crate::busca::{colunas_achado, ler_achado, AchadoArtefato};
use crate::extracoes::Pagina;
use crate::resumo::TotalTipo;
use crate::Resultado;

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct FiltroLinhaTempo {
    #[ts(optional)]
    pub extracao_id: Option<ExtracaoId>,
    /// Tipos de registro ("mensagem", "chamada"...); vazio = todos.
    #[ts(optional)]
    pub tipos: Option<Vec<String>>,
    #[ts(optional)]
    pub app: Option<String>,
    /// Intervalo em ms UTC, inclusivo.
    #[ts(optional, type = "number")]
    pub de_ms: Option<i64>,
    #[ts(optional, type = "number")]
    pub ate_ms: Option<i64>,
    /// Só registros com latitude e longitude.
    #[ts(optional)]
    pub com_coordenadas: Option<bool>,
    /// Mais recentes primeiro (o padrão é do mais antigo para o mais novo).
    #[ts(optional)]
    pub recentes_primeiro: Option<bool>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResumoLinhaTempo {
    /// Registros com data que atendem ao filtro, por tipo (sem o filtro de tipo).
    pub por_tipo: Vec<TotalTipo>,
    /// Apps presentes (com o filtro de tipo), para os filtros da tela.
    pub apps: Vec<TotalTipo>,
    #[ts(type = "number | null")]
    pub primeira_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub ultima_ms: Option<i64>,
    /// Registros que atenderiam ao filtro mas não têm data utilizável.
    #[ts(type = "number")]
    pub sem_data: i64,
}

struct Condicoes {
    sql: String,
    valores: Vec<ValorSql>,
}

/// Condições do filtro. `com_tipo`/`com_app` = aplicar esses filtros (o resumo conta
/// os tipos sem o filtro de tipo, para mostrar quantos há de cada um).
fn condicoes(f: &FiltroLinhaTempo, com_tipo: bool, com_app: bool) -> Condicoes {
    let mut c = Condicoes {
        sql: String::from("a.tipo <> 'conversa'"),
        valores: Vec::new(),
    };
    let mut add = |cond: &str, v: Vec<ValorSql>| {
        c.sql.push_str(" AND ");
        c.sql.push_str(cond);
        c.valores.extend(v);
    };
    if let Some(e) = f.extracao_id {
        add("a.extracao_id = ?", vec![ValorSql::Text(e.to_string())]);
    }
    let tipos: Vec<&String> = f.tipos.iter().flatten().filter(|t| !t.is_empty()).collect();
    if com_tipo && !tipos.is_empty() {
        let marcas = vec!["?"; tipos.len()].join(", ");
        add(
            &format!("a.tipo IN ({marcas})"),
            tipos.iter().map(|t| ValorSql::Text((*t).clone())).collect(),
        );
    }
    if let Some(app) = f
        .app
        .as_deref()
        .filter(|s| !s.is_empty())
        .filter(|_| com_app)
    {
        add("a.app = ?", vec![ValorSql::Text(app.into())]);
    }
    if let Some(de) = f.de_ms {
        add("a.data_hora_ms >= ?", vec![ValorSql::Integer(de)]);
    }
    if let Some(ate) = f.ate_ms {
        add("a.data_hora_ms <= ?", vec![ValorSql::Integer(ate)]);
    }
    if f.com_coordenadas == Some(true) {
        add("a.latitude IS NOT NULL AND a.longitude IS NOT NULL", vec![]);
    }
    c
}

pub fn linha_do_tempo(
    conn: &Connection,
    f: &FiltroLinhaTempo,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<AchadoArtefato>> {
    let c = condicoes(f, true, true);
    let onde = format!("a.data_hora_ms IS NOT NULL AND {}", c.sql);
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM artefato a WHERE {onde}"),
        params_from_iter(c.valores.iter()),
        |r| r.get(0),
    )?;
    let ordem = if f.recentes_primeiro == Some(true) {
        "DESC"
    } else {
        "ASC"
    };
    let mut v = c.valores.clone();
    v.push(ValorSql::Integer(limite));
    v.push(ValorSql::Integer(deslocamento));
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM artefato a INDEXED BY idx_artefato_tempo WHERE {onde}
         ORDER BY a.data_hora_ms {ordem}, a.id {ordem} LIMIT ? OFFSET ?",
        colunas_achado()
    ))?;
    let itens = stmt
        .query_map(params_from_iter(v.iter()), |r| ler_achado(r, false))?
        .collect::<Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

pub fn resumo_linha_do_tempo(
    conn: &Connection,
    f: &FiltroLinhaTempo,
) -> Resultado<ResumoLinhaTempo> {
    let totais = |sql: &str, c: &Condicoes| -> Resultado<Vec<TotalTipo>> {
        let mut stmt = conn.prepare(sql)?;
        let v = stmt
            .query_map(params_from_iter(c.valores.iter()), |r| {
                Ok(TotalTipo {
                    tipo: r.get(0)?,
                    total: r.get(1)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(v)
    };
    let sem_tipo = condicoes(f, false, true);
    let por_tipo = totais(
        &format!(
            "SELECT a.tipo, COUNT(*) FROM artefato a
             WHERE a.data_hora_ms IS NOT NULL AND {}
             GROUP BY a.tipo ORDER BY COUNT(*) DESC, a.tipo",
            sem_tipo.sql
        ),
        &sem_tipo,
    )?;
    let sem_app = condicoes(f, true, false);
    let apps = totais(
        &format!(
            "SELECT a.app, COUNT(*) FROM artefato a
             WHERE a.data_hora_ms IS NOT NULL AND a.app IS NOT NULL AND {}
             GROUP BY a.app ORDER BY COUNT(*) DESC, a.app",
            sem_app.sql
        ),
        &sem_app,
    )?;
    let c = condicoes(f, true, true);
    let (primeira_ms, ultima_ms): (Option<i64>, Option<i64>) = conn.query_row(
        &format!(
            "SELECT MIN(a.data_hora_ms), MAX(a.data_hora_ms) FROM artefato a WHERE {}",
            c.sql
        ),
        params_from_iter(c.valores.iter()),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    // Sem data: o filtro de período não se aplica (não há data para comparar).
    let sem_periodo = FiltroLinhaTempo {
        de_ms: None,
        ate_ms: None,
        ..f.clone()
    };
    let s = condicoes(&sem_periodo, true, true);
    let sem_data: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM artefato a WHERE a.data_hora_ms IS NULL AND a.tipo <> 'contato' AND {}",
            s.sql
        ),
        params_from_iter(s.valores.iter()),
        |r| r.get(0),
    )?;
    Ok(ResumoLinhaTempo {
        por_tipo,
        apps,
        primeira_ms,
        ultima_ms,
        sem_data,
    })
}
