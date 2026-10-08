//! Cache versionado e pesquisa por exemplo. Similaridade não é identidade.
use crate::extracoes::uuid;
use crate::{auditoria, ErroArmazenamento, Resultado, Sessao};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sied_dominio::{CasoId, ExtracaoId};
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct VisualIndexado {
    #[ts(type = "number")]
    pub id: i64,
    pub caso_id: CasoId,
    pub extracao_id: ExtracaoId,
    #[ts(type = "number")]
    pub arquivo_id: i64,
    pub caminho: String,
    pub sha256_origem: String,
    pub dhash: String,
    pub largura: u32,
    pub altura: u32,
    pub luminancia_media: f64,
    pub motor_id: String,
    pub motor_versao: String,
    pub executado_em: String,
    pub hash_derivado: String,
}
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SimilarVisual {
    #[ts(type = "number")]
    pub arquivo_id: i64,
    pub extracao_id: ExtracaoId,
    pub caminho: String,
    pub sha256: String,
    pub mecanismo: String,
    pub distancia_bits: Option<u32>,
    pub score_tecnico: Option<f64>,
}
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResultadoSimilarVisual {
    pub referencia: VisualIndexado,
    pub itens: Vec<SimilarVisual>,
    pub descritores_examinados: u32,
    pub varredura_limitada: bool,
    pub limites: Vec<String>,
}
const COLUNAS:&str="id,caso_id,extracao_id,arquivo_id,caminho,sha256_origem,dhash,largura,altura,luminancia_media,motor_id,motor_versao,executado_em,hash_derivado";
fn recusar(m: &str) -> ErroArmazenamento {
    ErroArmazenamento::Recusado(m.into())
}
fn hex_valido(h: &str, n: usize) -> bool {
    h.len() == n
        && h.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn hash(d: &VisualIndexado) -> String {
    // Perfil técnico, conteúdo e identidade; tempo de leitura fica separado.
    let v = json!([
        d.caso_id,
        d.extracao_id,
        d.arquivo_id,
        d.caminho,
        d.sha256_origem,
        d.dhash,
        d.largura,
        d.altura,
        d.luminancia_media,
        d.motor_id,
        d.motor_versao
    ]);
    hex::encode(Sha256::digest(v.to_string().as_bytes()))
}
fn ler(r: &rusqlite::Row<'_>) -> rusqlite::Result<VisualIndexado> {
    Ok(VisualIndexado {
        id: r.get(0)?,
        caso_id: uuid(r.get(1)?)?,
        extracao_id: uuid(r.get(2)?)?,
        arquivo_id: r.get(3)?,
        caminho: r.get(4)?,
        sha256_origem: r.get(5)?,
        dhash: r.get(6)?,
        largura: r.get(7)?,
        altura: r.get(8)?,
        luminancia_media: r.get(9)?,
        motor_id: r.get(10)?,
        motor_versao: r.get(11)?,
        executado_em: r.get(12)?,
        hash_derivado: r.get(13)?,
    })
}
fn validar(d: &VisualIndexado) -> Resultado<()> {
    if d.arquivo_id <= 0
        || !hex_valido(&d.sha256_origem, 64)
        || !hex_valido(&d.dhash, 16)
        || d.largura == 0
        || d.altura == 0
        || d.largura > 4096
        || d.altura > 4096
        || u64::from(d.largura) * u64::from(d.altura) > 16_000_000
        || !d.luminancia_media.is_finite()
        || !(0.0..=255.0).contains(&d.luminancia_media)
        || d.motor_id.is_empty()
        || d.motor_id.len() > 128
        || d.motor_versao.is_empty()
        || d.motor_versao.len() > 128
        || DateTime::parse_from_rfc3339(&d.executado_em).is_err()
    {
        return Err(recusar("descritor visual inválido"));
    }
    Ok(())
}
pub fn obter(conn: &Connection, id: i64) -> Resultado<VisualIndexado> {
    let d = conn
        .query_row(
            &format!("SELECT {COLUNAS} FROM visual_derivado WHERE id=?1"),
            [id],
            ler,
        )
        .optional()?
        .ok_or_else(|| ErroArmazenamento::NaoEncontrado("descritor visual".into()))?;
    validar(&d)?;
    if hash(&d) != d.hash_derivado {
        return Err(ErroArmazenamento::Inconsistente(
            "descritor visual não confere com o hash derivado".into(),
        ));
    }
    Ok(d)
}
pub fn existente(
    conn: &Connection,
    arquivo_id: i64,
    motor_id: &str,
    motor_versao: &str,
) -> Resultado<Option<VisualIndexado>> {
    let id=conn.query_row("SELECT v.id FROM visual_derivado v JOIN arquivo a ON a.id=v.arquivo_id AND a.sha256=v.sha256_origem WHERE v.arquivo_id=?1 AND motor_id=?2 AND motor_versao=?3 ORDER BY v.id DESC LIMIT 1",params![arquivo_id,motor_id,motor_versao],|r|r.get(0)).optional()?;
    id.map(|i| obter(conn, i)).transpose()
}
/// Entrada exclusivamente do motor Rust, não exposta como escrita IPC.
pub fn guardar(
    tx: &Transaction<'_>,
    sessao: &Sessao,
    mut d: VisualIndexado,
) -> Resultado<VisualIndexado> {
    validar(&d)?;
    let confere:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM arquivo a JOIN extracao x ON x.id=a.extracao_id JOIN dispositivo v ON v.id=x.dispositivo_id WHERE a.id=?1 AND x.id=?2 AND v.caso_id=?3 AND a.caminho=?4 AND a.sha256=?5 AND a.categoria='imagem')",params![d.arquivo_id,d.extracao_id.to_string(),d.caso_id.to_string(),d.caminho,d.sha256_origem],|r|r.get(0))?;
    if !confere {
        return Err(recusar("descritor não corresponde ao inventário do caso"));
    }
    d.hash_derivado = hash(&d);
    if let Some(atual) = existente(tx, d.arquivo_id, &d.motor_id, &d.motor_versao)? {
        if atual.hash_derivado != d.hash_derivado {
            return Err(ErroArmazenamento::Inconsistente(
                "mesmo motor/origem produziu descritor diferente; nova versão é necessária".into(),
            ));
        }
        return Ok(atual);
    }
    tx.execute("INSERT INTO visual_derivado(caso_id,extracao_id,arquivo_id,caminho,sha256_origem,dhash,largura,altura,luminancia_media,motor_id,motor_versao,executado_em,hash_derivado) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![d.caso_id.to_string(),d.extracao_id.to_string(),d.arquivo_id,d.caminho,d.sha256_origem,d.dhash,d.largura,d.altura,d.luminancia_media,d.motor_id,d.motor_versao,d.executado_em,d.hash_derivado])?;
    d.id = tx.last_insert_rowid();
    auditoria::registrar(
        tx,
        sessao,
        "descritor_visual_indexado",
        json!({"id":d.id,"arquivo_id":d.arquivo_id,"sha256_origem":d.sha256_origem,"hash_derivado":d.hash_derivado,"motor_id":d.motor_id,"motor_versao":d.motor_versao}),
    )?;
    Ok(d)
}
pub fn semelhantes(
    conn: &Connection,
    id: i64,
    max_bits: u32,
    limite: usize,
    controle: &std::sync::Arc<crate::ControleDescoberta>,
) -> Resultado<ResultadoSimilarVisual> {
    controle.verificar()?;
    if max_bits > 16 || !(1..=50).contains(&limite) {
        return Err(recusar("distância visual deve ser 0–16 e limite 1–50"));
    }
    let referencia = obter(conn, id)?;
    let assinatura =
        u64::from_str_radix(&referencia.dhash, 16).map_err(|_| recusar("dHash inválido"))?;
    let mut itens = Vec::new();
    // Duplicatas por SHA-256 do inventário independem de haver descritor visual.
    let mut stmt=conn.prepare("SELECT id,extracao_id,caminho,sha256 FROM arquivo WHERE sha256=?1 AND categoria='imagem' AND id<>?2 ORDER BY id LIMIT ?3")?;
    let exatos = stmt
        .query_map(
            params![
                referencia.sha256_origem,
                referencia.arquivo_id,
                limite as i64 + 1
            ],
            |r| {
                Ok(SimilarVisual {
                    arquivo_id: r.get(0)?,
                    extracao_id: uuid(r.get(1)?)?,
                    caminho: r.get(2)?,
                    sha256: r.get(3)?,
                    mecanismo: "sha256_exato".into(),
                    distancia_bits: None,
                    score_tecnico: None,
                })
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let exatos_limitados = exatos.len() > limite;
    itens.extend(exatos.into_iter().take(limite));
    let mut stmt=conn.prepare(&format!("SELECT {COLUNAS} FROM visual_derivado WHERE motor_id=?1 AND motor_versao=?2 AND arquivo_id<>?3 ORDER BY id LIMIT 5001"))?;
    let mut rows = stmt.query(params![
        referencia.motor_id,
        referencia.motor_versao,
        referencia.arquivo_id
    ])?;
    let mut examinados = 0;
    let mut limitada = exatos_limitados;
    let mut vistos = std::collections::BTreeSet::new();
    vistos.extend(itens.iter().map(|i| i.arquivo_id));
    while let Some(r) = rows.next()? {
        controle.verificar()?;
        if examinados == 5000 {
            limitada = true;
            break;
        }
        examinados += 1;
        let d = ler(r)?;
        validar(&d)?;
        if hash(&d) != d.hash_derivado {
            return Err(ErroArmazenamento::Inconsistente(
                "descritor consultado foi alterado".into(),
            ));
        }
        let bits = (assinatura
            ^ u64::from_str_radix(&d.dhash, 16).map_err(|_| recusar("dHash inválido"))?)
        .count_ones();
        if bits <= max_bits && vistos.insert(d.arquivo_id) {
            itens.push(SimilarVisual {
                arquivo_id: d.arquivo_id,
                extracao_id: d.extracao_id,
                caminho: d.caminho,
                sha256: d.sha256_origem,
                mecanismo: "dhash64_candidato".into(),
                distancia_bits: Some(bits),
                score_tecnico: Some(1.0 - f64::from(bits) / 64.0),
            });
        }
    }
    itens.sort_by(|a, b| {
        (a.mecanismo != "sha256_exato")
            .cmp(&(b.mecanismo != "sha256_exato"))
            .then_with(|| {
                a.distancia_bits
                    .unwrap_or(0)
                    .cmp(&b.distancia_bits.unwrap_or(0))
            })
            .then_with(|| a.arquivo_id.cmp(&b.arquivo_id))
    });
    itens.truncate(limite);
    controle.verificar()?;
    Ok(ResultadoSimilarVisual{referencia,itens,descritores_examinados:examinados,varredura_limitada:limitada,limites:vec!["Duplicatas exatas são priorizadas e limitadas à página solicitada; somente imagens indexadas possuem dHash. Até 5.000 descritores compatíveis são examinados, sem garantia de exaustividade em casos maiores.".into(),"dHash é semelhança estrutural grosseira, não identidade nem detecção de objetos. Imagens uniformes distintas podem coincidir; sem rotação EXIF ou perfil ICC.".into()]})
}
#[cfg(test)]
mod contratos {
    use super::*;
    #[test]
    fn imprime_contratos_visuais_rust() {
        macro_rules! c {
            ($t:ty) => {
                println!("SIED_CONTRATO_INICIO:{}.ts", <$t>::name());
                println!("{}", <$t>::export_to_string().unwrap());
                println!("SIED_CONTRATO_FIM:{}.ts", <$t>::name());
            };
        }
        c!(VisualIndexado);
        c!(SimilarVisual);
        c!(ResultadoSimilarVisual);
    }
}
