//! Números do caso para o painel e lista de identificadores (remetentes e
//! destinatários) vistos nas conversas.
//!
//! Tudo é contado a partir do que está gravado no banco do caso; nada é estimado.
//! Os identificadores são exatamente como o parser os leu: o SIED não junta números
//! diferentes nem deduz a quem pertencem.

use rusqlite::{params, Connection};
use serde::Serialize;
use ts_rs::TS;

use crate::extracoes::{Pagina, TotalCategoria};
use crate::Resultado;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct TotalTipo {
    pub tipo: String,
    #[ts(type = "number")]
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResumoCaso {
    #[ts(type = "number")]
    pub dispositivos: i64,
    #[ts(type = "number")]
    pub extracoes: i64,
    #[ts(type = "number")]
    pub extracoes_concluidas: i64,
    /// Arquivos inventariados em todas as extrações.
    #[ts(type = "number")]
    pub arquivos: i64,
    #[ts(type = "number")]
    pub bytes: i64,
    pub arquivos_por_categoria: Vec<TotalCategoria>,
    /// Registros interpretados pelos parsers (artefatos).
    #[ts(type = "number")]
    pub registros: i64,
    pub registros_por_tipo: Vec<TotalTipo>,
    #[ts(type = "number")]
    pub conversas: i64,
    /// Identificadores distintos (por app) que aparecem nas conversas.
    #[ts(type = "number")]
    pub identificadores: i64,
    #[ts(type = "number")]
    pub execucoes_parser: i64,
    /// Execuções que falharam, foram canceladas ou interrompidas.
    #[ts(type = "number")]
    pub execucoes_com_problema: i64,
    #[ts(type = "number")]
    pub avisos_parser: i64,
    /// Registros com latitude e longitude.
    #[ts(type = "number")]
    pub localizacoes: i64,
}

pub fn resumo(conn: &Connection) -> Resultado<ResumoCaso> {
    let contar = |sql: &str| -> Resultado<i64> { Ok(conn.query_row(sql, [], |r| r.get(0))?) };

    let mut stmt = conn.prepare(
        "SELECT categoria, COUNT(*), COALESCE(SUM(tamanho), 0) FROM arquivo
         GROUP BY categoria ORDER BY COUNT(*) DESC, categoria",
    )?;
    let arquivos_por_categoria: Vec<TotalCategoria> = stmt
        .query_map([], |r| {
            Ok(TotalCategoria {
                categoria: r.get(0)?,
                arquivos: r.get(1)?,
                bytes: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT tipo, COUNT(*) FROM artefato GROUP BY tipo ORDER BY COUNT(*) DESC, tipo",
    )?;
    let registros_por_tipo: Vec<TotalTipo> = stmt
        .query_map([], |r| {
            Ok(TotalTipo {
                tipo: r.get(0)?,
                total: r.get(1)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    Ok(ResumoCaso {
        dispositivos: contar("SELECT COUNT(*) FROM dispositivo")?,
        extracoes: contar("SELECT COUNT(*) FROM extracao")?,
        extracoes_concluidas: contar("SELECT COUNT(*) FROM extracao WHERE status = 'concluida'")?,
        arquivos: arquivos_por_categoria.iter().map(|c| c.arquivos).sum(),
        bytes: arquivos_por_categoria.iter().map(|c| c.bytes).sum(),
        arquivos_por_categoria,
        registros: registros_por_tipo.iter().map(|t| t.total).sum(),
        registros_por_tipo,
        conversas: contar("SELECT COUNT(*) FROM conversa")?,
        identificadores: contar(
            "SELECT COUNT(*) FROM (SELECT DISTINCT c.app, p.identificador
               FROM conversa_participante p JOIN conversa c ON c.id = p.conversa_id)",
        )?,
        execucoes_parser: contar("SELECT COUNT(*) FROM execucao_parser")?,
        execucoes_com_problema: contar(
            "SELECT COUNT(*) FROM execucao_parser
             WHERE status IN ('falhou', 'cancelada', 'interrompida')",
        )?,
        avisos_parser: contar("SELECT COUNT(*) FROM aviso_parser")?,
        localizacoes: contar(
            "SELECT COUNT(*) FROM artefato WHERE latitude IS NOT NULL AND longitude IS NOT NULL",
        )?,
    })
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Identificador {
    /// Como o parser leu (telefone, jid...), sem normalização.
    pub identificador: String,
    pub app: String,
    #[ts(type = "number")]
    pub conversas: i64,
    #[ts(type = "number")]
    pub mensagens: i64,
    #[ts(type = "number | null")]
    pub primeira_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub ultima_ms: Option<i64>,
}

/// Identificadores das conversas, dos com mais mensagens para os com menos.
/// `texto` filtra por trecho do identificador.
pub fn identificadores(
    conn: &Connection,
    texto: Option<&str>,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<Identificador>> {
    let texto = texto.map(str::trim).filter(|t| !t.is_empty());
    let base = "FROM conversa_participante p JOIN conversa c ON c.id = p.conversa_id
                WHERE ?1 IS NULL OR instr(lower(p.identificador), lower(?1)) > 0";
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM (SELECT DISTINCT c.app, p.identificador {base})"),
        params![texto],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT p.identificador, c.app, COUNT(DISTINCT c.id), SUM(p.mensagens),
                MIN(c.primeira_ms), MAX(c.ultima_ms)
         {base}
         GROUP BY c.app, p.identificador
         ORDER BY SUM(p.mensagens) DESC, p.identificador
         LIMIT ?2 OFFSET ?3"
    ))?;
    let itens = stmt
        .query_map(params![texto, limite, deslocamento], |r| {
            Ok(Identificador {
                identificador: r.get(0)?,
                app: r.get(1)?,
                conversas: r.get(2)?,
                mensagens: r.get(3)?,
                primeira_ms: r.get(4)?,
                ultima_ms: r.get(5)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(Pagina { total, itens })
}
