//! Recuperação integrada sobre índices REAIS. RRF combina posições, não mistura
//! BM25/confianças como probabilidades. Modelos/OCR/transcrição ausentes não geram
//! sinais. A janela de candidatos é limitada e declarada, sem contagem global falsa.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sied_dominio::{CasoId, ExtracaoId};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use ts_rs::TS;

use crate::busca::{self, FiltroBusca};
use crate::controle_descoberta::GuardaDescoberta;
use crate::{documentos, ControleDescoberta, ErroArmazenamento, Resultado};

const CANDIDATOS_POR_MECANISMO: i64 = 100;
const K_RRF: f64 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum MecanismoUniversal {
    Registros,
    Arquivos,
    Documentos,
    Entidades,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct PedidoUniversal {
    pub filtro: FiltroBusca,
    pub mecanismos: Vec<MecanismoUniversal>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SinalUniversal {
    pub mecanismo: MecanismoUniversal,
    pub motivo: String,
    pub posicao: u32,
    /// Parcela 1/(60+posição). Não é confiança nem probabilidade de verdade.
    pub contribuicao_rrf: f64,
    pub motor: String,
    #[ts(type = "number | null")]
    pub documento_id: Option<i64>,
    pub pagina: Option<u32>,
    pub hash_derivado: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ItemUniversal {
    pub chave: String,
    pub titulo: String,
    pub trecho: String,
    pub tipo: String,
    pub extracao_id: ExtracaoId,
    #[ts(type = "number | null")]
    pub artefato_id: Option<i64>,
    #[ts(type = "number | null")]
    pub arquivo_id: Option<i64>,
    #[ts(type = "number | null")]
    pub conversa_id: Option<i64>,
    #[ts(type = "number | null")]
    pub data_hora_ms: Option<i64>,
    pub hash_origem: Option<String>,
    pub sinais: Vec<SinalUniversal>,
    pub score_ranking: f64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResultadoUniversal {
    pub caso_id: CasoId,
    pub pedido: PedidoUniversal,
    pub algoritmo: String,
    pub executada_em: String,
    pub itens: Vec<ItemUniversal>,
    /// União da janela recuperada, não o número total de artefatos do caso.
    pub recuperados_na_janela: u32,
    pub janela_limitada: bool,
    pub avisos: Vec<String>,
}

fn recusar(motivo: &str) -> ErroArmazenamento {
    ErroArmazenamento::Recusado(motivo.into())
}

fn sinal(
    mecanismo: MecanismoUniversal,
    posicao: usize,
    motivo: &str,
    motor: &str,
) -> SinalUniversal {
    SinalUniversal {
        mecanismo,
        motivo: motivo.into(),
        posicao: (posicao + 1) as u32,
        contribuicao_rrf: 1.0 / (K_RRF + posicao as f64 + 1.0),
        motor: motor.into(),
        documento_id: None,
        pagina: None,
        hash_derivado: None,
    }
}

fn adicionar(
    itens: &mut BTreeMap<String, ItemUniversal>,
    mut item: ItemUniversal,
    sinal: SinalUniversal,
) {
    let existente = itens.entry(item.chave.clone()).or_insert_with(|| {
        item.sinais.clear();
        item
    });
    // Uma fonte só vota uma vez por item. Várias páginas/snapshots permanecem
    // explicáveis, mas não multiplicam o voto desse mecanismo.
    if let Some(anterior) = existente
        .sinais
        .iter_mut()
        .find(|s| s.mecanismo == sinal.mecanismo)
    {
        if sinal.posicao < anterior.posicao {
            *anterior = sinal;
        }
    } else {
        existente.sinais.push(sinal);
    }
}

fn contexto_arquivo(
    conn: &Connection,
    id: i64,
    pedido: &PedidoUniversal,
) -> Resultado<Option<(Option<String>, Option<i64>)>> {
    let origem: Option<(Option<String>, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT sha256, criado_em, modificado_em FROM arquivo WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((hash, criado, modificado)) = origem else {
        return Ok(None);
    };
    let data = criado
        .or(modificado)
        .and_then(|d| chrono::DateTime::parse_from_rfc3339(&d).ok())
        .map(|d| d.timestamp_millis());
    let f = &pedido.filtro;
    if f.de_ms.is_some_and(|de| data.is_none_or(|d| d < de))
        || f.ate_ms.is_some_and(|ate| data.is_none_or(|d| d > ate))
    {
        return Ok(None);
    }
    Ok(Some((hash, data)))
}

pub fn pesquisar(
    conn: &Connection,
    caso_id: CasoId,
    pedido: &PedidoUniversal,
    deslocamento: i64,
    limite: i64,
    controle: &Arc<ControleDescoberta>,
) -> Resultado<ResultadoUniversal> {
    if pedido.filtro.texto.trim().is_empty()
        || pedido.filtro.texto.len() > 1024
        || pedido.filtro.texto.contains('\0')
        || pedido.filtro.texto.split_whitespace().count() > 64
        || pedido.mecanismos.is_empty()
        || pedido.mecanismos.len() > 4
        || deslocamento < 0
        || deslocamento > 400
        || !(1..=50).contains(&limite)
        || pedido
            .filtro
            .de_ms
            .zip(pedido.filtro.ate_ms)
            .is_some_and(|(de, ate)| de > ate)
    {
        return Err(recusar("consulta universal inválida ou acima dos limites"));
    }
    let mecanismos: BTreeSet<_> = pedido.mecanismos.iter().copied().collect();
    if mecanismos.len() != pedido.mecanismos.len() {
        return Err(recusar("mecanismos repetidos na consulta"));
    }
    let _guarda = GuardaDescoberta::instalar(conn, controle);
    let resultado = (|| {
        controle.verificar()?;
        let mut itens = BTreeMap::new();
        let mut limitada = false;
        let mut avisos = vec!["Scores expressam posição no ranking, não confiança pericial. OCR, embeddings, transcrição e modelos de objetos não contribuem nesta versão.".into()];
        let admite_documentos = pedido.filtro.app.as_deref().is_none_or(str::is_empty)
            && pedido.filtro.tipo.as_deref().is_none_or(str::is_empty);
        if mecanismos.contains(&MecanismoUniversal::Registros)
            || mecanismos.contains(&MecanismoUniversal::Arquivos)
        {
            let encontrados = busca::buscar(conn, &pedido.filtro, 0, CANDIDATOS_POR_MECANISMO)?;
            if mecanismos.contains(&MecanismoUniversal::Registros) {
                limitada |= encontrados.artefatos.total > CANDIDATOS_POR_MECANISMO;
                for (pos, achado) in encontrados.artefatos.itens.into_iter().enumerate() {
                    controle.verificar()?;
                    let a = achado.artefato;
                    let hash: String = conn.query_row(
                        "SELECT hash_registro FROM artefato WHERE id=?1",
                        [a.id],
                        |r| r.get(0),
                    )?;
                    let trecho = achado
                        .trecho
                        .map(|p| p.into_iter().map(|p| p.texto).collect::<String>())
                        .unwrap_or_else(|| a.conteudo.clone().unwrap_or_default())
                        .chars()
                        .take(400)
                        .collect();
                    adicionar(
                        &mut itens,
                        ItemUniversal {
                            chave: format!("artefato:{}", a.id),
                            titulo: a
                                .remetente
                                .clone()
                                .or(a.nome_arquivo)
                                .unwrap_or_else(|| format!("Registro {}", a.id)),
                            trecho,
                            tipo: a.tipo,
                            extracao_id: achado.extracao_id,
                            artefato_id: Some(a.id),
                            arquivo_id: a.anexo_arquivo_id,
                            conversa_id: achado.conversa_id,
                            data_hora_ms: a.data_hora_ms,
                            hash_origem: Some(hash),
                            sinais: vec![],
                            score_ranking: 0.0,
                        },
                        sinal(
                            MecanismoUniversal::Registros,
                            pos,
                            "Correspondência na busca textual original (texto/identificadores).",
                            "fts5/telefone-sied",
                        ),
                    );
                }
            }
            if mecanismos.contains(&MecanismoUniversal::Arquivos) && admite_documentos {
                limitada |= encontrados.arquivos.total > CANDIDATOS_POR_MECANISMO;
                for (pos, a) in encontrados.arquivos.itens.into_iter().enumerate() {
                    controle.verificar()?;
                    let Some((hash, data)) = contexto_arquivo(conn, a.id, pedido)? else {
                        continue;
                    };
                    adicionar(&mut itens, ItemUniversal { chave:format!("arquivo:{}",a.id),titulo:a.caminho.clone(),trecho:a.caminho,
                        tipo:a.categoria,extracao_id:a.extracao_id,artefato_id:None,arquivo_id:Some(a.id),conversa_id:None,
                        data_hora_ms:data,hash_origem:hash,sinais:vec![],score_ranking:0.0 },
                        sinal(MecanismoUniversal::Arquivos,pos,"Correspondência no caminho do inventário; não significa leitura do conteúdo.","fts5-caminho"));
                }
            }
        }
        // App/tipo de registro não são atributos documentais inventados.
        if !admite_documentos {
            avisos.push("Arquivos/documentos/entidades sem app ou tipo de registro correspondente foram excluídos pelos filtros.".into());
        }
        if admite_documentos
            && (mecanismos.contains(&MecanismoUniversal::Documentos)
                || mecanismos.contains(&MecanismoUniversal::Entidades))
        {
            if mecanismos.contains(&MecanismoUniversal::Documentos) {
                let docs =
                    documentos::buscar_filtrado(conn, &pedido.filtro, 0, CANDIDATOS_POR_MECANISMO)?;
                limitada |= docs.total > CANDIDATOS_POR_MECANISMO;
                for (pos, d) in docs.itens.into_iter().enumerate() {
                    controle.verificar()?;
                    if pedido
                        .filtro
                        .extracao_id
                        .is_some_and(|e| e != d.extracao_id)
                    {
                        continue;
                    }
                    let Some((hash, data)) = contexto_arquivo(conn, d.arquivo_id, pedido)? else {
                        continue;
                    };
                    let mut s = sinal(MecanismoUniversal::Documentos,pos,"Correspondência no texto derivado armazenado; a origem atual não foi relida.",&format!("{}@{}",d.motor_id,d.motor_versao));
                    s.documento_id = Some(d.id);
                    s.pagina = d.pagina;
                    s.hash_derivado = Some(d.sha256_texto);
                    adicionar(
                        &mut itens,
                        ItemUniversal {
                            chave: format!("arquivo:{}", d.arquivo_id),
                            titulo: d.caminho,
                            trecho: d.trecho,
                            tipo: "documento".into(),
                            extracao_id: d.extracao_id,
                            arquivo_id: Some(d.arquivo_id),
                            artefato_id: None,
                            conversa_id: None,
                            data_hora_ms: data,
                            hash_origem: hash,
                            sinais: vec![],
                            score_ranking: 0.0,
                        },
                        s,
                    );
                }
            }
            if mecanismos.contains(&MecanismoUniversal::Entidades) {
                let termo = busca::consulta_fts(&pedido.filtro.texto)
                    .ok_or_else(|| recusar("consulta sem termos indexáveis"))?;
                let mut stmt=conn.prepare("SELECT e.id,e.documento_id,e.tipo,e.literal,d.arquivo_id FROM busca_entidade_documental
                    JOIN entidade_documental e ON e.id=busca_entidade_documental.rowid
                    JOIN documento_derivado d ON d.id=e.documento_id JOIN arquivo a ON a.id=d.arquivo_id
                    WHERE busca_entidade_documental MATCH ?1 AND (?2 IS NULL OR d.extracao_id=?2)
                    AND (?3 IS NULL OR sied_data_arquivo_ms(COALESCE(a.criado_em,a.modificado_em))>=?3)
                    AND (?4 IS NULL OR sied_data_arquivo_ms(COALESCE(a.criado_em,a.modificado_em))<=?4)
                    ORDER BY e.id DESC LIMIT 101")?;
                let candidatos = stmt
                    .query_map(
                        params![
                            termo,
                            pedido.filtro.extracao_id.map(|e| e.to_string()),
                            pedido.filtro.de_ms,
                            pedido.filtro.ate_ms
                        ],
                        |r| {
                            Ok((
                                r.get::<_, i64>(0)?,
                                r.get::<_, i64>(1)?,
                                r.get::<_, String>(2)?,
                                r.get::<_, String>(3)?,
                                r.get::<_, i64>(4)?,
                            ))
                        },
                    )?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                limitada |= candidatos.len() > 100;
                for (pos, (_id, docid, tipo, literal, arquivo_id)) in
                    candidatos.into_iter().take(100).enumerate()
                {
                    controle.verificar()?;
                    let d = documentos::obter(conn, docid)?;
                    if pedido
                        .filtro
                        .extracao_id
                        .is_some_and(|e| e != d.extracao_id)
                    {
                        continue;
                    }
                    let Some((hash, data)) = contexto_arquivo(conn, arquivo_id, pedido)? else {
                        continue;
                    };
                    let mut s=sinal(MecanismoUniversal::Entidades,pos,&format!("Ocorrência literal candidata de {tipo}: {literal}. Não comprova identidade/titularidade."),&d.entidades_motor_versao);
                    s.documento_id = Some(docid);
                    s.pagina = d.pagina;
                    s.hash_derivado = Some(d.sha256_texto);
                    adicionar(
                        &mut itens,
                        ItemUniversal {
                            chave: format!("arquivo:{arquivo_id}"),
                            titulo: d.caminho,
                            trecho: literal,
                            tipo: "documento".into(),
                            extracao_id: d.extracao_id,
                            arquivo_id: Some(arquivo_id),
                            artefato_id: None,
                            conversa_id: None,
                            data_hora_ms: data,
                            hash_origem: hash,
                            sinais: vec![],
                            score_ranking: 0.0,
                        },
                        s,
                    );
                }
            }
        }
        controle.verificar()?;
        let denominador = mecanismos.len() as f64 / (K_RRF + 1.0);
        let mut itens: Vec<_> = itens.into_values().collect();
        for i in &mut itens {
            i.score_ranking =
                i.sinais.iter().map(|s| s.contribuicao_rrf).sum::<f64>() / denominador;
        }
        itens.sort_by(|a, b| {
            b.score_ranking
                .total_cmp(&a.score_ranking)
                .then_with(|| a.chave.cmp(&b.chave))
        });
        let recuperados = itens.len() as u32;
        if limitada {
            avisos.push("Janela limitada aos 100 primeiros candidatos por mecanismo; a paginação percorre essa união, não todos os resultados do caso.".into());
        }
        avisos.push("Datas de arquivos são timestamps registrados no inventário, não data do conteúdo ou da indexação. Itens sem data são excluídos quando há filtro temporal.".into());
        Ok(ResultadoUniversal {
            caso_id,
            pedido: pedido.clone(),
            algoritmo: "rrf-k60-v1-equal-mechanisms".into(),
            executada_em: chrono::Utc::now().to_rfc3339(),
            itens: itens
                .into_iter()
                .skip(deslocamento as usize)
                .take(limite as usize)
                .collect(),
            recuperados_na_janela: recuperados,
            janela_limitada: limitada,
            avisos,
        })
    })();
    resultado.map_err(|e| controle.traduzir_sqlite(e))
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn imprime_contratos_universais_rust() {
        macro_rules! exportar {
            ($t:ty) => {
                println!("SIED_CONTRATO_INICIO:{}.ts", <$t>::name());
                println!("{}", <$t>::export_to_string().unwrap());
                println!("SIED_CONTRATO_FIM:{}.ts", <$t>::name());
            };
        }
        exportar!(MecanismoUniversal);
        exportar!(PedidoUniversal);
        exportar!(SinalUniversal);
        exportar!(ItemUniversal);
        exportar!(ResultadoUniversal);
    }
}
