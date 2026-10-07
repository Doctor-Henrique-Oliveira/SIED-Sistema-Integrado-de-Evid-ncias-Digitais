//! Contexto relacional EXPERIMENTAL e somente leitura, não um motor de causalidade.
//!
//! Relações diretas são referências de origem gravadas e pertencimento à conversa
//! pela chave real extração/app/id. Proximidade UTC é sempre uma sugestão, nunca
//! identidade, comunicação, relevância criminal ou sequência causal. Não cria
//! pessoa, evidência, revisão, modelo, grafo persistido ou evento de auditoria.
//!
//! Reutiliza a linha do tempo e a regra de plausibilidade do domínio, sem tentar
//! reinterpretar datas originais. Um banco autocontido deve ter somente o caso
//! esperado; origem/eventos/arquivos/conversas têm associação conferida por IDs.
//! Somente os primeiros 30 eventos cronológicos e até 60 referências entram na
//! página/grafo. Há consultas em lote, sem cruzamento N² ou carregar todo o caso.
//! Limite de itens não é um orçamento de bytes dos campos dos registros.

use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use serde::Serialize;
use sied_dominio::{CasoId, DataHora, ExtracaoId};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use ts_rs::TS;

use crate::busca::{colunas_achado, ler_achado, AchadoArtefato};
use crate::controle_descoberta::{ControleDescoberta, GuardaDescoberta};
use crate::extracoes::{uuid, Pagina};
use crate::linha_tempo::{linha_do_tempo, FiltroLinhaTempo};
use crate::{ErroArmazenamento, Resultado};

pub const MAX_EVENTOS: i64 = 30;
const MAX_REFERENCIAS: usize = 60;
pub const MECANISMO_TEMPORAL: &str = "proximidade_utc_linear_janela_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ClasseRelacao {
    Direta,
    Sugerida,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ArestaRelacao {
    pub de: String,
    pub para: String,
    pub classe: ClasseRelacao,
    pub motivo: String,
    pub mecanismo: String,
    /// Somente proximidade normalizada `1 - |delta_ms|/janela_ms`, em 0–1.
    /// Não é probabilidade, confiança de modelo ou relevância pericial.
    pub score_contexto: Option<f64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct NoRelacao {
    /// Inclui o caso, evitando confundir IDs locais de bancos diferentes.
    pub chave: String,
    pub tipo: String,
    pub rotulo: String,
    #[ts(type = "number | null")]
    pub artefato_id: Option<i64>,
    #[ts(type = "number | null")]
    pub arquivo_id: Option<i64>,
    #[ts(type = "number | null")]
    pub conversa_id: Option<i64>,
    pub extracao_id: ExtracaoId,
    /// SHA-256 histórico do arquivo no inventário, NÃO uma releitura atual nem
    /// o hash_registro. Só o nó arquivo possui esse valor nesta versão.
    pub hash_origem: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ContextoRelacoes {
    pub caso_id: CasoId,
    pub origem: AchadoArtefato,
    pub nos: Vec<NoRelacao>,
    pub arestas: Vec<ArestaRelacao>,
    pub eventos: Pagina<AchadoArtefato>,
    pub janela_minutos: u32,
    pub limitado: bool,
    pub avisos: Vec<String>,
}

/// Leitura cancelável; o chamador deve manter a conexão sob sua exclusão normal.
/// O callback SQLite é removido em sucesso/erro/unwind antes de publicar.
pub fn correlacionar(
    conn: &Connection,
    caso_id: &CasoId,
    artefato_id: i64,
    janela_minutos: u32,
    limite: i64,
    controle: &Arc<ControleDescoberta>,
) -> Resultado<ContextoRelacoes> {
    controle.verificar()?;
    if artefato_id <= 0
        || !matches!(janela_minutos, 5 | 30 | 60 | 1440)
        || !(1..=MAX_EVENTOS).contains(&limite)
    {
        return Err(ErroArmazenamento::Recusado(
            "contexto exige registro positivo, janela de 5/30/60/1440 minutos e limite de 1 a 30"
                .into(),
        ));
    }
    let mut guarda = GuardaDescoberta::instalar(conn, controle);
    let resultado =
        correlacionar_interno(conn, caso_id, artefato_id, janela_minutos, limite, controle);
    let limpeza = guarda.limpar();
    drop(guarda);
    match resultado {
        Ok(contexto) => {
            limpeza?;
            controle.concluir()?;
            Ok(contexto)
        }
        Err(erro) => Err(controle.traduzir_sqlite(erro)),
    }
}

const ASSOCIACAO: &str = "JOIN extracao x ON x.id = a.extracao_id
    JOIN dispositivo v ON v.id = x.dispositivo_id AND v.id = a.dispositivo_id
    JOIN caso c ON c.id = v.caso_id";

fn chave(caso_id: &CasoId, tipo: &str, id: i64) -> String {
    format!("caso:{caso_id}/{tipo}:{id}")
}

fn no_artefato(caso_id: &CasoId, achado: &AchadoArtefato) -> NoRelacao {
    NoRelacao {
        chave: chave(caso_id, "artefato", achado.artefato.id),
        tipo: "artefato".into(),
        rotulo: format!("Registro #{}", achado.artefato.id),
        artefato_id: Some(achado.artefato.id),
        arquivo_id: None,
        conversa_id: None,
        extracao_id: achado.extracao_id,
        hash_origem: None,
    }
}

fn correlacionar_interno(
    conn: &Connection,
    caso_id: &CasoId,
    artefato_id: i64,
    janela_minutos: u32,
    limite: i64,
    controle: &Arc<ControleDescoberta>,
) -> Resultado<ContextoRelacoes> {
    // A helper da timeline trabalha no banco autocontido, não no catálogo global.
    let (total_casos, caso_esperado): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), coalesce(SUM(id = ?1),0) FROM caso",
        [caso_id.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if total_casos != 1 || caso_esperado != 1 {
        return Err(ErroArmazenamento::Recusado(
            "o banco não corresponde exclusivamente ao caso solicitado".into(),
        ));
    }
    let origem = conn
        .query_row(
            &format!(
                "SELECT {} FROM artefato a {ASSOCIACAO} WHERE a.id = ?1 AND c.id = ?2",
                colunas_achado()
            ),
            params![artefato_id, caso_id.to_string()],
            |r| ler_achado(r, false),
        )
        .optional()?
        .ok_or_else(|| ErroArmazenamento::NaoEncontrado("registro no caso solicitado".into()))?;
    controle.verificar()?;
    let data_origem = origem
        .artefato
        .data_hora_ms
        .and_then(DataHora::ms_plausivel);
    let janela_ms = i64::from(janela_minutos) * 60_000;
    let mut avisos = vec![
        "EXPERIMENTAL: proximidade temporal não prova causalidade, identidade ou relevância; nenhum modelo neural é utilizado.".into(),
        "Hashes dos nós arquivo são do inventário/referência histórica, sem releitura atual da extração.".into(),
    ];
    let eventos = if let Some(ms) = data_origem {
        let filtro = FiltroLinhaTempo {
            de_ms: Some(ms.saturating_sub(janela_ms).max(1)),
            ate_ms: Some(ms.saturating_add(janela_ms).min(DataHora::UTC_MS_MAXIMO)),
            ..Default::default()
        };
        linha_do_tempo(conn, &filtro, 0, limite)?
    } else {
        avisos.push("Proximidade temporal indisponível: o registro não possui data UTC plausível. A data original não foi reinterpretada.".into());
        Pagina {
            total: 0,
            itens: Vec::new(),
        }
    };
    let mut limitado = eventos.total > eventos.itens.len() as i64;
    if limitado {
        avisos.push("A lista mostra apenas os primeiros eventos cronológicos da janela, não necessariamente os mais próximos do registro. Há eventos adicionais fora desta página.".into());
    }
    let mut achados = vec![&origem];
    achados.extend(
        eventos
            .itens
            .iter()
            .filter(|a| a.artefato.id != artefato_id),
    );
    let ids: Vec<i64> = achados.iter().map(|a| a.artefato.id).collect();
    let marcas = vec!["?"; ids.len()].join(",");
    let mut valores: Vec<Value> = ids.iter().map(|id| Value::Integer(*id)).collect();
    valores.push(Value::Text(caso_id.to_string()));
    let associados: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM artefato a {ASSOCIACAO} WHERE a.id IN ({marcas}) AND c.id = ?"
        ),
        params_from_iter(valores.iter()),
        |r| r.get(0),
    )?;
    if associados != ids.len() as i64 {
        return Err(ErroArmazenamento::Inconsistente(
            "um evento da janela não possui associação íntegra com o caso solicitado".into(),
        ));
    }
    let mut nos: BTreeMap<String, NoRelacao> = BTreeMap::new();
    let mut arestas = Vec::new();
    for achado in &achados {
        controle.verificar()?;
        let no = no_artefato(caso_id, achado);
        if achado.artefato.id != artefato_id {
            if let (Some(ms), Some(evento_ms)) = (
                data_origem,
                achado
                    .artefato
                    .data_hora_ms
                    .and_then(DataHora::ms_plausivel),
            ) {
                let delta = ms.abs_diff(evento_ms);
                arestas.push(ArestaRelacao {
                    de: chave(caso_id, "artefato", artefato_id), para: no.chave.clone(),
                    classe: ClasseRelacao::Sugerida,
                    motivo: format!("Diferença UTC de {delta} ms dentro da janela. Apenas proximidade temporal, não causalidade."),
                    mecanismo: MECANISMO_TEMPORAL.into(),
                    score_contexto: Some((1.0 - delta as f64 / janela_ms as f64).clamp(0.0, 1.0)),
                });
            }
        }
        nos.insert(no.chave.clone(), no);
    }
    let referencias = referencias(conn, &ids, caso_id)?;
    if referencias.len() > MAX_REFERENCIAS {
        limitado = true;
        avisos.push("Relações documentais limitadas a 60 referências conferíveis nesta página; o grafo não representa todo o caso.".into());
    }
    let mut pares = BTreeSet::new();
    let mut referencias_conferidas = 0;
    for referencia in referencias.into_iter().take(MAX_REFERENCIAS) {
        controle.verificar()?;
        let Some(hash) = referencia
            .sha256
            .filter(|sha| *sha == referencia.sha256_referencia)
        else {
            avisos.push(format!(
                "Referência do arquivo #{} omitida: seu SHA-256 não confere com o inventário.",
                referencia.arquivo_id
            ));
            continue;
        };
        let key = chave(caso_id, "arquivo", referencia.arquivo_id);
        nos.entry(key.clone()).or_insert_with(|| NoRelacao {
            chave: key.clone(),
            tipo: "arquivo".into(),
            rotulo: format!(
                "Arquivo #{} · {}",
                referencia.arquivo_id,
                referencia.caminho.chars().take(160).collect::<String>()
            ),
            artefato_id: None,
            arquivo_id: Some(referencia.arquivo_id),
            conversa_id: None,
            extracao_id: referencia.extracao_id,
            hash_origem: Some(hash),
        });
        let origem_no = chave(caso_id, "artefato", referencia.artefato_id);
        if pares.insert((origem_no.clone(), key.clone())) {
            arestas.push(ArestaRelacao { de: origem_no, para: key, classe: ClasseRelacao::Direta,
                motivo: "Referência de origem registrada e hash correspondente ao inventário. Não significa relação entre pessoas.".into(),
                mecanismo: "referencia_origem_inventario_v1".into(), score_contexto: None });
            referencias_conferidas += 1;
        }
    }
    if referencias_conferidas == 0 {
        avisos.push("Nenhuma referência documental conferível foi localizada para os registros desta página.".into());
    }
    for achado in &achados {
        controle.verificar()?;
        let Some(id) = achado.conversa_id else {
            continue;
        };
        let confere: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM conversa co JOIN extracao x ON x.id = co.extracao_id
             JOIN dispositivo v ON v.id = x.dispositivo_id WHERE co.id = ?1 AND co.extracao_id = ?2
             AND co.app = ?3 AND co.id_externo = ?4 AND v.caso_id = ?5)",
            params![
                id,
                achado.extracao_id.to_string(),
                achado.artefato.app.as_deref().unwrap_or(""),
                achado.artefato.conversa_id_externo,
                caso_id.to_string()
            ],
            |r| r.get(0),
        )?;
        if !confere {
            return Err(ErroArmazenamento::Inconsistente(
                "a conversa derivada não corresponde à chave original do registro".into(),
            ));
        }
        let key = chave(caso_id, "conversa", id);
        nos.entry(key.clone()).or_insert_with(|| NoRelacao {
            chave: key.clone(),
            tipo: "conversa".into(),
            rotulo: format!("Conversa #{id}"),
            artefato_id: None,
            arquivo_id: None,
            conversa_id: Some(id),
            extracao_id: achado.extracao_id,
            hash_origem: None,
        });
        arestas.push(ArestaRelacao {
            de: chave(caso_id, "artefato", achado.artefato.id), para: key,
            classe: ClasseRelacao::Direta,
            motivo: "Pertencimento pela chave registrada de extração/app/id da conversa. Não infere quem é o titular.".into(),
            mecanismo: "conversa_chave_registrada_v1".into(), score_contexto: None,
        });
    }
    controle.verificar()?;
    Ok(ContextoRelacoes {
        caso_id: *caso_id,
        origem,
        nos: nos.into_values().collect(),
        arestas,
        eventos,
        janela_minutos,
        limitado,
        avisos,
    })
}

struct Referencia {
    artefato_id: i64,
    arquivo_id: i64,
    caminho: String,
    extracao_id: ExtracaoId,
    sha256: Option<String>,
    sha256_referencia: String,
}

fn referencias(conn: &Connection, ids: &[i64], caso_id: &CasoId) -> Resultado<Vec<Referencia>> {
    let marcas = vec!["?"; ids.len()].join(",");
    let mut valores: Vec<Value> = ids.iter().map(|id| Value::Integer(*id)).collect();
    valores.push(Value::Text(caso_id.to_string()));
    valores.push(Value::Integer(MAX_REFERENCIAS as i64 + 1));
    let mut stmt = conn.prepare(&format!(
        "SELECT DISTINCT r.artefato_id, f.id, f.caminho, f.extracao_id, f.sha256, r.arquivo_sha256
         FROM referencia_origem r JOIN artefato a ON a.id = r.artefato_id
         JOIN arquivo f ON f.id = r.arquivo_id AND f.extracao_id = a.extracao_id
         JOIN extracao x ON x.id = f.extracao_id AND x.dispositivo_id = a.dispositivo_id
         JOIN dispositivo v ON v.id = x.dispositivo_id
         WHERE r.artefato_id IN ({marcas}) AND v.caso_id = ?
         ORDER BY r.artefato_id, f.id LIMIT ?"
    ))?;
    let rows = stmt
        .query_map(params_from_iter(valores.iter()), |r| {
            Ok(Referencia {
                artefato_id: r.get(0)?,
                arquivo_id: r.get(1)?,
                caminho: r.get(2)?,
                extracao_id: uuid(r.get(3)?)?,
                sha256: r.get(4)?,
                sha256_referencia: r.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod contratos {
    use super::*;

    #[test]
    fn imprimir_contratos_relacionais_rust() {
        macro_rules! contrato {
            ($tipo:ty) => {
                println!("SIED_CONTRATO_INICIO:{}.ts", <$tipo>::name());
                println!("{}", <$tipo>::export_to_string().unwrap());
                println!("SIED_CONTRATO_FIM:{}.ts", <$tipo>::name());
            };
        }
        contrato!(ClasseRelacao);
        contrato!(ArestaRelacao);
        contrato!(NoRelacao);
        contrato!(ContextoRelacoes);
    }
}
