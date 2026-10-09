//! Transcrições locais derivadas de áudio/vídeo.
//!
//! O módulo só persiste texto produzido por um provedor local explícito. A origem
//! continua no inventário e cada execução é identificada por hash, motor e versão.
//! Segmentos são append-only para que uma nova execução nunca apague a anterior.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sied_dominio::{CasoId, ExtracaoId};
use ts_rs::TS;

use crate::extracoes::Pagina;
use crate::{auditoria, ErroArmazenamento, Resultado, Sessao};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EstadoTranscricao {
    Processando,
    Pronta,
    SemSuporte,
    Cancelada,
    Erro,
}

impl EstadoTranscricao {
    fn as_str(self) -> &'static str {
        match self {
            Self::Processando => "processando",
            Self::Pronta => "pronta",
            Self::SemSuporte => "sem_suporte",
            Self::Cancelada => "cancelada",
            Self::Erro => "erro",
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct TranscricaoLocal {
    #[ts(type = "number")]
    pub id: i64,
    pub caso_id: CasoId,
    pub extracao_id: ExtracaoId,
    #[ts(type = "number")]
    pub arquivo_id: i64,
    pub caminho: String,
    pub sha256_origem: String,
    pub estado: EstadoTranscricao,
    pub idioma: Option<String>,
    pub motor_id: String,
    pub motor_versao: String,
    pub iniciado_em: String,
    pub concluido_em: Option<String>,
    pub erro: Option<String>,
    pub hash_derivado: String,
    #[ts(type = "number")]
    pub total_segmentos: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SegmentoTranscricao {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub transcricao_id: i64,
    #[ts(type = "number")]
    pub ordem: u32,
    #[ts(type = "number")]
    pub inicio_ms: i64,
    #[ts(type = "number")]
    pub fim_ms: i64,
    pub texto: String,
    pub confianca: Option<f64>,
    pub falante: Option<String>,
    pub hash_segmento: String,
}

#[derive(Debug, Clone)]
pub struct NovoSegmento {
    pub ordem: u32,
    pub inicio_ms: i64,
    pub fim_ms: i64,
    pub texto: String,
    pub confianca: Option<f64>,
    pub falante: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NovaTranscricao {
    pub caso_id: CasoId,
    pub extracao_id: ExtracaoId,
    pub arquivo_id: i64,
    pub caminho: String,
    pub sha256_origem: String,
    pub estado: EstadoTranscricao,
    pub idioma: Option<String>,
    pub motor_id: String,
    pub motor_versao: String,
    pub erro: Option<String>,
    pub segmentos: Vec<NovoSegmento>,
}

fn hash_bytes(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(bytes.as_ref()))
}

fn hash_segmento(s: &NovoSegmento) -> String {
    hash_bytes(format!(
        "{}|{}|{}|{}|{:?}|{:?}",
        s.ordem, s.inicio_ms, s.fim_ms, s.texto, s.confianca, s.falante
    ))
}

fn recusar(m: &str) -> ErroArmazenamento {
    ErroArmazenamento::Recusado(m.into())
}

fn ler_estado(v: String) -> rusqlite::Result<EstadoTranscricao> {
    match v.as_str() {
        "processando" => Ok(EstadoTranscricao::Processando),
        "pronta" => Ok(EstadoTranscricao::Pronta),
        "sem_suporte" => Ok(EstadoTranscricao::SemSuporte),
        "cancelada" => Ok(EstadoTranscricao::Cancelada),
        "erro" => Ok(EstadoTranscricao::Erro),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "estado de transcrição desconhecido",
            )),
        )),
    }
}

fn ler_transcricao(r: &Row<'_>) -> rusqlite::Result<TranscricaoLocal> {
    Ok(TranscricaoLocal {
        id: r.get(0)?,
        caso_id: crate::extracoes::uuid(r.get(1)?)?,
        extracao_id: crate::extracoes::uuid(r.get(2)?)?,
        arquivo_id: r.get(3)?,
        caminho: r.get(4)?,
        sha256_origem: r.get(5)?,
        estado: ler_estado(r.get(6)?)?,
        idioma: r.get(7)?,
        motor_id: r.get(8)?,
        motor_versao: r.get(9)?,
        iniciado_em: r.get(10)?,
        concluido_em: r.get(11)?,
        erro: r.get(12)?,
        hash_derivado: r.get(13)?,
        total_segmentos: r.get(14)?,
    })
}

fn ler_segmento(r: &Row<'_>) -> rusqlite::Result<SegmentoTranscricao> {
    Ok(SegmentoTranscricao {
        id: r.get(0)?,
        transcricao_id: r.get(1)?,
        ordem: r.get(2)?,
        inicio_ms: r.get(3)?,
        fim_ms: r.get(4)?,
        texto: r.get(5)?,
        confianca: r.get(6)?,
        falante: r.get(7)?,
        hash_segmento: r.get(8)?,
    })
}

fn origem_confere(conn: &Connection, novo: &NovaTranscricao) -> Resultado<()> {
    let ok: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM arquivo a JOIN extracao x ON x.id=a.extracao_id JOIN dispositivo d ON d.id=x.dispositivo_id JOIN caso c ON c.id=d.caso_id WHERE c.id=?1 AND x.id=?2 AND a.id=?3 AND a.caminho=?4 AND a.sha256=?5 AND a.categoria IN ('audio','video'))",
        params![novo.caso_id.to_string(), novo.extracao_id.to_string(), novo.arquivo_id, novo.caminho, novo.sha256_origem], |r| r.get(0))?;
    if ok {
        Ok(())
    } else {
        Err(recusar(
            "origem de áudio/vídeo não corresponde ao inventário deste caso",
        ))
    }
}

fn validar(novo: &NovaTranscricao) -> Resultado<()> {
    if novo.arquivo_id <= 0
        || novo.caminho.is_empty()
        || novo.sha256_origem.len() != 64
        || novo.motor_id.is_empty()
        || novo.motor_versao.is_empty()
        || novo.segmentos.len() > 100_000
    {
        return Err(recusar("transcrição local inválida ou acima dos limites"));
    }
    let mut anterior = None;
    for (i, s) in novo.segmentos.iter().enumerate() {
        if s.ordem as usize != i
            || s.inicio_ms < 0
            || s.fim_ms <= s.inicio_ms
            || s.texto.trim().is_empty()
            || s.texto.len() > 4000
            || s.confianca.is_some_and(|x| !(0.0..=1.0).contains(&x))
            || anterior.is_some_and(|fim| s.inicio_ms < fim)
        {
            return Err(recusar(
                "segmentos de transcrição não são monotônicos ou têm campos inválidos",
            ));
        }
        anterior = Some(s.fim_ms);
    }
    Ok(())
}

pub fn guardar(
    tx: &Transaction<'_>,
    sessao: &Sessao,
    novo: &NovaTranscricao,
) -> Resultado<TranscricaoLocal> {
    validar(novo)?;
    origem_confere(tx, novo)?;
    let iniciado = Utc::now().to_rfc3339();
    let material = serde_json::to_vec(
        &novo
            .segmentos
            .iter()
            .map(|s| {
                (
                    &s.ordem,
                    &s.inicio_ms,
                    &s.fim_ms,
                    &s.texto,
                    &s.confianca,
                    &s.falante,
                )
            })
            .collect::<Vec<_>>(),
    )
    .map_err(|_| recusar("não foi possível calcular o hash da transcrição"))?;
    let hash = hash_bytes(material);
    let existente: Option<(i64, String, String)> = tx.query_row("SELECT id, hash_derivado, estado FROM transcricao_local WHERE arquivo_id=?1 AND sha256_origem=?2 AND motor_id=?3 AND motor_versao=?4", params![novo.arquivo_id, novo.sha256_origem, novo.motor_id, novo.motor_versao], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional()?;
    let id = if let Some(id) = existente {
        if id.1 != hash || id.2 != novo.estado.as_str() {
            return Err(recusar(
                "reprocessamento divergente requer nova versão do motor",
            ));
        }
        id.0
    } else {
        tx.execute("INSERT INTO transcricao_local(caso_id,extracao_id,arquivo_id,caminho,sha256_origem,estado,idioma,motor_id,motor_versao,iniciado_em,concluido_em,erro,hash_derivado) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10,?11,?12)", params![novo.caso_id.to_string(), novo.extracao_id.to_string(), novo.arquivo_id, novo.caminho, novo.sha256_origem, novo.estado.as_str(), novo.idioma, novo.motor_id, novo.motor_versao, iniciado, novo.erro, hash])?;
        let id = tx.last_insert_rowid();
        for s in &novo.segmentos {
            tx.execute("INSERT INTO segmento_transcricao(transcricao_id,ordem,inicio_ms,fim_ms,texto,confianca,falante,hash_segmento) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)", params![id,s.ordem,s.inicio_ms,s.fim_ms,s.texto,s.confianca,s.falante,hash_segmento(s)])?;
        }
        id
    };
    auditoria::registrar(
        tx,
        sessao,
        "transcricao_local_guardada",
        serde_json::json!({"transcricao_id":id,"arquivo_id":novo.arquivo_id,"sha256_origem":novo.sha256_origem,"estado":novo.estado.as_str(),"motor_id":novo.motor_id,"motor_versao":novo.motor_versao,"segmentos":novo.segmentos.len()}),
    )?;
    obter(tx, id)
}

pub fn obter(conn: &Connection, id: i64) -> Resultado<TranscricaoLocal> {
    conn.query_row("SELECT t.id,t.caso_id,t.extracao_id,t.arquivo_id,t.caminho,t.sha256_origem,t.estado,t.idioma,t.motor_id,t.motor_versao,t.iniciado_em,t.concluido_em,t.erro,t.hash_derivado,(SELECT COUNT(*) FROM segmento_transcricao s WHERE s.transcricao_id=t.id) FROM transcricao_local t WHERE t.id=?1", [id], ler_transcricao).optional()?.ok_or_else(|| ErroArmazenamento::NaoEncontrado("transcrição local".into()))
}

pub fn listar(conn: &Connection, arquivo_id: i64) -> Resultado<Vec<TranscricaoLocal>> {
    let mut st = conn.prepare("SELECT t.id,t.caso_id,t.extracao_id,t.arquivo_id,t.caminho,t.sha256_origem,t.estado,t.idioma,t.motor_id,t.motor_versao,t.iniciado_em,t.concluido_em,t.erro,t.hash_derivado,(SELECT COUNT(*) FROM segmento_transcricao s WHERE s.transcricao_id=t.id) FROM transcricao_local t WHERE t.arquivo_id=?1 ORDER BY t.iniciado_em DESC")?;
    Ok(st
        .query_map([arquivo_id], ler_transcricao)?
        .collect::<Result<_, _>>()?)
}

/// Lista todas as transcrições derivadas do caso para consumo de relatórios.
/// A ordenação é estável e não altera o conteúdo original do inventário.
pub fn listar_caso(conn: &Connection, caso_id: CasoId) -> Resultado<Vec<TranscricaoLocal>> {
    let mut st = conn.prepare("SELECT t.id,t.caso_id,t.extracao_id,t.arquivo_id,t.caminho,t.sha256_origem,t.estado,t.idioma,t.motor_id,t.motor_versao,t.iniciado_em,t.concluido_em,t.erro,t.hash_derivado,(SELECT COUNT(*) FROM segmento_transcricao s WHERE s.transcricao_id=t.id) FROM transcricao_local t WHERE t.caso_id=?1 ORDER BY t.iniciado_em ASC, t.id ASC")?;
    Ok(st
        .query_map([caso_id.to_string()], ler_transcricao)?
        .collect::<Result<_, _>>()?)
}

/// Segmentos limitados para anexar contexto temporal a um relatório.
pub fn segmentos_relatorio(
    conn: &Connection,
    transcricao_id: i64,
    limite: i64,
) -> Resultado<Vec<SegmentoTranscricao>> {
    let limite = limite.clamp(1, 500);
    let mut st = conn.prepare("SELECT s.id,s.transcricao_id,s.ordem,s.inicio_ms,s.fim_ms,s.texto,s.confianca,s.falante,s.hash_segmento FROM segmento_transcricao s WHERE s.transcricao_id=?1 ORDER BY s.ordem LIMIT ?2")?;
    Ok(st
        .query_map(params![transcricao_id, limite], ler_segmento)?
        .collect::<Result<_, _>>()?)
}

pub fn segmentos(
    conn: &Connection,
    transcricao_id: i64,
    deslocamento: i64,
    limite: i64,
    termo: Option<&str>,
) -> Resultado<Pagina<SegmentoTranscricao>> {
    if transcricao_id <= 0 || deslocamento < 0 || !(1..=200).contains(&limite) {
        return Err(recusar("paginação de transcrição inválida"));
    }
    let termo = termo
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .and_then(crate::busca::consulta_fts);
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM segmento_transcricao s WHERE s.transcricao_id=?1 AND (?2 IS NULL OR s.id IN (SELECT rowid FROM busca_transcricao WHERE busca_transcricao MATCH ?2))", params![transcricao_id, termo], |r| r.get(0))?;
    let mut st = conn.prepare("SELECT s.id,s.transcricao_id,s.ordem,s.inicio_ms,s.fim_ms,s.texto,s.confianca,s.falante,s.hash_segmento FROM segmento_transcricao s WHERE s.transcricao_id=?1 AND (?2 IS NULL OR s.id IN (SELECT rowid FROM busca_transcricao WHERE busca_transcricao MATCH ?2)) ORDER BY s.ordem LIMIT ?3 OFFSET ?4")?;
    let itens = st
        .query_map(
            params![transcricao_id, termo, limite, deslocamento],
            ler_segmento,
        )?
        .collect::<Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn hash_de_segmento_e_deterministico() {
        let s = NovoSegmento {
            ordem: 0,
            inicio_ms: 0,
            fim_ms: 1000,
            texto: "fala sintética".into(),
            confianca: None,
            falante: None,
        };
        assert_eq!(hash_segmento(&s), hash_segmento(&s));
    }
    #[test]
    fn rejeita_segmento_descendente() {
        let n = NovaTranscricao {
            caso_id: CasoId::novo(),
            extracao_id: ExtracaoId::novo(),
            arquivo_id: 1,
            caminho: "a.wav".into(),
            sha256_origem: "0".repeat(64),
            estado: EstadoTranscricao::Pronta,
            idioma: None,
            motor_id: "x".into(),
            motor_versao: "1".into(),
            erro: None,
            segmentos: vec![
                NovoSegmento {
                    ordem: 0,
                    inicio_ms: 10,
                    fim_ms: 20,
                    texto: "a".into(),
                    confianca: None,
                    falante: None,
                },
                NovoSegmento {
                    ordem: 1,
                    inicio_ms: 19,
                    fim_ms: 30,
                    texto: "b".into(),
                    confianca: None,
                    falante: None,
                },
            ],
        };
        assert!(validar(&n).is_err());
    }

    #[test]
    fn imprime_contratos_transcricao_rust() {
        for (nome, contrato) in [
            (
                TranscricaoLocal::name(),
                TranscricaoLocal::export_to_string().unwrap(),
            ),
            (
                SegmentoTranscricao::name(),
                SegmentoTranscricao::export_to_string().unwrap(),
            ),
            (
                EstadoTranscricao::name(),
                EstadoTranscricao::export_to_string().unwrap(),
            ),
        ] {
            println!("SIED_CONTRATO_INICIO:{nome}.ts");
            println!("{contrato}");
            println!("SIED_CONTRATO_FIM:{nome}.ts");
        }
    }
}
