//! Datasets de avaliação explicitamente autorizados, exclusivamente no caso.
//!
//! Não copia evidência para catálogo global, não envia dados e não treina motores.
//! Previsão significa ocorrência sintática com os mesmos offsets no snapshot;
//! rótulos não revisados/inconclusivos/descartados nunca entram nas métricas.

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sied_dominio::{CasoId, ExtracaoId};
use std::sync::Arc;
use ts_rs::TS;

use crate::controle_descoberta::GuardaDescoberta;
use crate::documentos::{self, EstadoRevisaoEntidade};
use crate::extracoes::{uuid, Pagina};
use crate::{auditoria, ControleDescoberta, ErroArmazenamento, Resultado, Sessao};

const MAX_AMOSTRAS: i64 = 10_000;

/// Filtros da fila operacional de candidatos que ainda precisam de decisão
/// humana. O filtro nunca altera o snapshot: só restringe a consulta.
#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct FiltroFilaRevisao {
    pub estado: Option<EstadoRevisaoEntidade>,
    pub motor_versao: Option<String>,
}

/// Candidato derivado com sua proveniência mínima para revisão. A inferência
/// original (literal, offsets, hashes e versão do motor) é somente leitura;
/// decisões posteriores são gravadas em `revisao_entidade_documental`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct CandidatoRevisao {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub documento_id: i64,
    pub caso_id: CasoId,
    pub extracao_id: ExtracaoId,
    #[ts(type = "number")]
    pub arquivo_id: i64,
    pub caminho: String,
    pub tipo: String,
    pub literal: String,
    pub inicio_utf8: u32,
    pub fim_utf8: u32,
    pub estado: EstadoRevisaoEntidade,
    pub nota: Option<String>,
    pub motor_versao: String,
    pub sha256_origem: String,
    pub sha256_texto: String,
    pub revista_em: Option<String>,
    pub revisor: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DatasetSupervisionado {
    #[ts(type = "number")]
    pub id: i64,
    pub caso_id: CasoId,
    pub nome: String,
    #[ts(type = "number")]
    pub versao: i64,
    pub autorizacao: String,
    pub retencao: String,
    pub criado_em: String,
    pub autor: String,
    #[ts(type = "number")]
    pub total_amostras: i64,
    #[ts(type = "number")]
    pub total_versoes: i64,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct PedidoAmostraSupervisionada {
    #[ts(type = "number")]
    pub documento_id: i64,
    pub inicio_utf8: u32,
    pub fim_utf8: u32,
    pub esperado: bool,
    /// O valor declarado é comparado com a ocorrência REAL do motor armazenado;
    /// o frontend não consegue fabricar uma previsão.
    pub previsto: bool,
    pub estado_rotulo: EstadoRevisaoEntidade,
    pub nota: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AmostraSupervisionada {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub dataset_id: i64,
    #[ts(type = "number")]
    pub documento_id: i64,
    #[ts(type = "number")]
    pub versao: i64,
    pub inicio_utf8: u32,
    pub fim_utf8: u32,
    pub trecho: String,
    pub sha256_texto_documento: String,
    pub sha256_origem: String,
    pub sha256_amostra: String,
    pub entidades_motor_versao: String,
    pub tipos_previstos: Vec<String>,
    pub esperado: bool,
    pub previsto: bool,
    pub estado_rotulo: EstadoRevisaoEntidade,
    pub nota: Option<String>,
    pub criado_em: String,
    pub revisor: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AvaliacaoSupervisionada {
    #[ts(type = "number")]
    pub dataset_id: i64,
    #[ts(type = "number")]
    pub total_amostras: i64,
    #[ts(type = "number")]
    pub total_versoes: i64,
    #[ts(type = "number")]
    pub elegiveis: i64,
    #[ts(type = "number")]
    pub nao_revisadas: i64,
    #[ts(type = "number")]
    pub inconclusivas: i64,
    #[ts(type = "number")]
    pub descartadas: i64,
    #[ts(type = "number")]
    pub tp: i64,
    #[ts(type = "number")]
    pub fp: i64,
    #[serde(rename = "fn")]
    #[ts(type = "number")]
    pub fn_: i64,
    #[ts(type = "number")]
    pub tn: i64,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    pub acuracia: Option<f64>,
    pub cobertura: Option<f64>,
    pub motores_avaliados: Vec<String>,
    pub limites: Vec<String>,
}

fn recusar(motivo: &str) -> ErroArmazenamento {
    ErroArmazenamento::Recusado(motivo.into())
}

fn paginacao(deslocamento: i64, limite: i64) -> Resultado<()> {
    if deslocamento < 0 || !(1..=100).contains(&limite) {
        return Err(recusar("paginação supervisionada inválida"));
    }
    Ok(())
}

fn campo(valor: &str, limite: usize) -> bool {
    !valor.trim().is_empty() && valor.len() <= limite && !valor.contains('\0')
}

fn ler_candidato(r: &Row<'_>) -> rusqlite::Result<CandidatoRevisao> {
    Ok(CandidatoRevisao {
        id: r.get(0)?,
        documento_id: r.get(1)?,
        caso_id: uuid(r.get(2)?)?,
        extracao_id: uuid(r.get(3)?)?,
        arquivo_id: r.get(4)?,
        caminho: r.get(5)?,
        tipo: r.get(6)?,
        literal: r.get(7)?,
        inicio_utf8: r.get(8)?,
        fim_utf8: r.get(9)?,
        estado: serde_json::from_value(json!(r.get::<_, String>(10)?))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        nota: r.get(11)?,
        motor_versao: r.get(12)?,
        sha256_origem: r.get(13)?,
        sha256_texto: r.get(14)?,
        revista_em: r.get(15)?,
        revisor: r.get(16)?,
    })
}

const COLUNAS_FILA: &str = "e.id,e.documento_id,d.caso_id,d.extracao_id,d.arquivo_id,d.caminho,
    e.tipo,e.literal,e.inicio_utf8,e.fim_utf8,coalesce(r.estado,'nao_revisado'),r.nota,
    d.entidades_motor_versao,d.sha256_origem,d.sha256_texto,r.revista_em,r.revisor";

const ULTIMA_REVISAO_FILA: &str = "LEFT JOIN revisao_entidade_documental r ON r.id =
    (SELECT MAX(r2.id) FROM revisao_entidade_documental r2 WHERE r2.entidade_id=e.id)";

/// Retorna candidatos de entidades documentais de forma paginada. Cada linha
/// é conferida novamente contra o texto/hash do snapshot antes de sair do
/// backend, evitando que uma fila de revisão mascare corrupção derivada.
pub fn fila_revisao(
    conn: &Connection,
    caso_id: CasoId,
    filtro: &FiltroFilaRevisao,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<CandidatoRevisao>> {
    paginacao(deslocamento, limite)?;
    if filtro
        .motor_versao
        .as_deref()
        .is_some_and(|v| v.len() > 400 || v.contains('\0'))
    {
        return Err(recusar("filtro de versão do motor inválido"));
    }
    let caso = caso_id.to_string();
    let estado = filtro.estado.map(estado_str);
    let motor = filtro.motor_versao.as_deref();
    let total: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM entidade_documental e
             JOIN documento_derivado d ON e.documento_id=d.id {ULTIMA_REVISAO_FILA}
             WHERE d.caso_id=?1 AND (?2 IS NULL OR coalesce(r.estado,'nao_revisado')=?2)
               AND (?3 IS NULL OR d.entidades_motor_versao=?3)"
        ),
        params![&caso, estado, motor],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUNAS_FILA} FROM entidade_documental e
         JOIN documento_derivado d ON e.documento_id=d.id {ULTIMA_REVISAO_FILA}
         WHERE d.caso_id=?1 AND (?2 IS NULL OR coalesce(r.estado,'nao_revisado')=?2)
           AND (?3 IS NULL OR d.entidades_motor_versao=?3)
         ORDER BY e.id DESC LIMIT ?4 OFFSET ?5"
    ))?;
    let itens: Vec<CandidatoRevisao> = stmt
        .query_map(
            params![&caso, estado, motor, limite, deslocamento],
            ler_candidato,
        )?
        .collect::<std::result::Result<_, _>>()?;
    for item in &itens {
        let documento = documentos::obter(conn, item.documento_id)?;
        let trecho = documento
            .texto
            .get(item.inicio_utf8 as usize..item.fim_utf8 as usize);
        if documento.caso_id != caso_id
            || documento.sha256_origem != item.sha256_origem
            || documento.sha256_texto != item.sha256_texto
            || trecho != Some(item.literal.as_str())
        {
            return Err(ErroArmazenamento::Inconsistente(
                "fila de revisão diverge do snapshot documental".into(),
            ));
        }
    }
    Ok(Pagina { total, itens })
}

const COLUNAS_DATASET: &str =
    "s.id,s.caso_id,s.nome,s.versao,s.autorizacao,s.retencao,s.criado_em,s.autor,
    (SELECT COUNT(*) FROM (SELECT documento_id,inicio_utf8,fim_utf8 FROM amostra_supervisionada
        WHERE dataset_id=s.id GROUP BY documento_id,inicio_utf8,fim_utf8)),
    (SELECT COUNT(*) FROM amostra_supervisionada WHERE dataset_id=s.id)";

fn ler_dataset(r: &Row<'_>) -> rusqlite::Result<DatasetSupervisionado> {
    Ok(DatasetSupervisionado {
        id: r.get(0)?,
        caso_id: uuid(r.get(1)?)?,
        nome: r.get(2)?,
        versao: r.get(3)?,
        autorizacao: r.get(4)?,
        retencao: r.get(5)?,
        criado_em: r.get(6)?,
        autor: r.get(7)?,
        total_amostras: r.get(8)?,
        total_versoes: r.get(9)?,
    })
}

pub fn obter_dataset(
    conn: &Connection,
    caso_id: CasoId,
    id: i64,
) -> Resultado<DatasetSupervisionado> {
    conn.query_row(
        &format!(
            "SELECT {COLUNAS_DATASET} FROM dataset_supervisionado s WHERE s.id=?1 AND s.caso_id=?2"
        ),
        params![id, caso_id.to_string()],
        ler_dataset,
    )
    .optional()?
    .ok_or_else(|| ErroArmazenamento::NaoEncontrado("dataset deste caso".into()))
}

pub fn criar_dataset(
    tx: &Transaction<'_>,
    sessao: &Sessao,
    caso_id: CasoId,
    nome: &str,
    autorizacao: &str,
    retencao: &str,
) -> Resultado<DatasetSupervisionado> {
    if !campo(nome, 200) || !campo(autorizacao, 4000) || !campo(retencao, 2000) {
        return Err(recusar(
            "informe nome, autorização explícita e política declarada de retenção do dataset",
        ));
    }
    let versao: i64 = tx.query_row(
        "SELECT COALESCE(MAX(versao),0)+1 FROM dataset_supervisionado WHERE caso_id=?1 AND nome=?2",
        params![caso_id.to_string(), nome],
        |r| r.get(0),
    )?;
    tx.execute("INSERT INTO dataset_supervisionado(caso_id,nome,versao,autorizacao,retencao,criado_em,autor)
        VALUES(?1,?2,?3,?4,?5,?6,?7)", params![caso_id.to_string(),nome,versao,autorizacao,retencao,chrono::Utc::now().to_rfc3339(),sessao.usuario])?;
    let id = tx.last_insert_rowid();
    auditoria::registrar(
        tx,
        sessao,
        "dataset_supervisionado_criado",
        json!({"dataset_id":id,"versao":versao,
        "autorizacao_sha256":hex::encode(Sha256::digest(autorizacao.as_bytes())),
        "retencao_sha256":hex::encode(Sha256::digest(retencao.as_bytes())),"escopo":"caso_local_sem_treinamento"}),
    )?;
    obter_dataset(tx, caso_id, id)
}

pub fn listar_datasets(
    conn: &Connection,
    caso_id: CasoId,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<DatasetSupervisionado>> {
    paginacao(deslocamento, limite)?;
    let total = conn.query_row(
        "SELECT COUNT(*) FROM dataset_supervisionado WHERE caso_id=?1",
        [caso_id.to_string()],
        |r| r.get(0),
    )?;
    let mut stmt=conn.prepare(&format!("SELECT {COLUNAS_DATASET} FROM dataset_supervisionado s WHERE s.caso_id=?1 ORDER BY s.id DESC LIMIT ?2 OFFSET ?3"))?;
    let itens = stmt
        .query_map(
            params![caso_id.to_string(), limite, deslocamento],
            ler_dataset,
        )?
        .collect::<std::result::Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

fn hash_amostra(a: &AmostraSupervisionada) -> String {
    // Formato estável e próprio: nunca a regra histórica hash_registro.
    let campos = json!({"formato":"sied-amostra-supervisionada-v1","dataset_id":a.dataset_id,
        "documento_id":a.documento_id,"versao":a.versao,"inicio_utf8":a.inicio_utf8,"fim_utf8":a.fim_utf8,
        "trecho":a.trecho,"sha256_texto_documento":a.sha256_texto_documento,"sha256_origem":a.sha256_origem,
        "entidades_motor_versao":a.entidades_motor_versao,"tipos_previstos":a.tipos_previstos,"esperado":a.esperado,
        "previsto":a.previsto,"estado_rotulo":a.estado_rotulo,"nota":a.nota,"criado_em":a.criado_em,"revisor":a.revisor});
    hex::encode(Sha256::digest(campos.to_string().as_bytes()))
}

fn estado_str(estado: EstadoRevisaoEntidade) -> &'static str {
    match estado {
        EstadoRevisaoEntidade::NaoRevisado => "nao_revisado",
        EstadoRevisaoEntidade::Confirmado => "confirmado",
        EstadoRevisaoEntidade::Descartado => "descartado",
        EstadoRevisaoEntidade::Inconclusivo => "inconclusivo",
    }
}

pub fn adicionar_amostra(
    tx: &Transaction<'_>,
    sessao: &Sessao,
    caso_id: CasoId,
    dataset_id: i64,
    pedido: &PedidoAmostraSupervisionada,
) -> Resultado<AmostraSupervisionada> {
    let dataset = obter_dataset(tx, caso_id, dataset_id)?;
    if !campo(&dataset.autorizacao, 4000) || !campo(&dataset.retencao, 2000) {
        return Err(recusar(
            "dataset sem autorização/retenção explícitas não pode receber amostras",
        ));
    }
    let doc = documentos::obter(tx, pedido.documento_id)?;
    if doc.caso_id != caso_id
        || pedido
            .nota
            .as_deref()
            .is_some_and(|n| n.len() > 4000 || n.contains('\0'))
    {
        return Err(recusar("amostra de outro caso ou nota acima dos limites"));
    }
    let inicio = pedido.inicio_utf8 as usize;
    let fim = pedido.fim_utf8 as usize;
    let trecho = doc
        .texto
        .get(inicio..fim)
        .filter(|t| !t.is_empty() && t.len() <= 4096)
        .ok_or_else(|| {
            recusar("selecione intervalo UTF-8 válido de até 4096 bytes no texto conferido")
        })?
        .to_string();
    let mut tipos_previstos: Vec<String> = documentos::entidades(tx, doc.id)?
        .into_iter()
        .filter(|e| e.inicio_utf8 == pedido.inicio_utf8 && e.fim_utf8 == pedido.fim_utf8)
        .map(|e| e.tipo)
        .collect();
    tipos_previstos.sort();
    tipos_previstos.dedup();
    let previsto = !tipos_previstos.is_empty();
    if previsto != pedido.previsto {
        return Err(recusar(
            "previsão declarada não corresponde à ocorrência exata do motor neste snapshot",
        ));
    }
    let versao: i64 = tx.query_row(
        "SELECT COALESCE(MAX(versao),0)+1 FROM amostra_supervisionada
        WHERE dataset_id=?1 AND documento_id=?2 AND inicio_utf8=?3 AND fim_utf8=?4",
        params![dataset_id, doc.id, pedido.inicio_utf8, pedido.fim_utf8],
        |r| r.get(0),
    )?;
    if versao == 1 && dataset.total_amostras >= MAX_AMOSTRAS {
        return Err(recusar(
            "dataset atingiu o limite de 10000 intervalos distintos",
        ));
    }
    if dataset.total_versoes >= MAX_AMOSTRAS * 10 {
        return Err(recusar(
            "dataset atingiu o limite de 100000 versões de anotação",
        ));
    }
    let mut a = AmostraSupervisionada {
        id: 0,
        dataset_id,
        documento_id: doc.id,
        versao,
        inicio_utf8: pedido.inicio_utf8,
        fim_utf8: pedido.fim_utf8,
        trecho,
        sha256_texto_documento: doc.sha256_texto,
        sha256_origem: doc.sha256_origem,
        sha256_amostra: String::new(),
        entidades_motor_versao: doc.entidades_motor_versao,
        tipos_previstos,
        esperado: pedido.esperado,
        previsto,
        estado_rotulo: pedido.estado_rotulo,
        nota: pedido.nota.clone(),
        criado_em: chrono::Utc::now().to_rfc3339(),
        revisor: sessao.usuario.clone(),
    };
    a.sha256_amostra = hash_amostra(&a);
    tx.execute("INSERT INTO amostra_supervisionada(dataset_id,documento_id,versao,inicio_utf8,fim_utf8,
        trecho,sha256_texto_documento,sha256_origem,sha256_amostra,entidades_motor_versao,tipos_previstos,
        esperado,previsto,estado_rotulo,nota,criado_em,revisor) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
        params![a.dataset_id,a.documento_id,a.versao,a.inicio_utf8,a.fim_utf8,a.trecho,a.sha256_texto_documento,
            a.sha256_origem,a.sha256_amostra,a.entidades_motor_versao,serde_json::to_string(&a.tipos_previstos).map_err(|_|recusar("tipos de previsão inválidos"))?,
            a.esperado,a.previsto,estado_str(a.estado_rotulo),a.nota,a.criado_em,a.revisor])?;
    a.id = tx.last_insert_rowid();
    auditoria::registrar(
        tx,
        sessao,
        "amostra_supervisionada_adicionada",
        json!({"dataset_id":dataset_id,
        "amostra_id":a.id,"documento_id":a.documento_id,"versao":a.versao,"sha256_amostra":a.sha256_amostra,
        "estado_rotulo":a.estado_rotulo,"escopo":"intervalo_literal_sem_conclusao_pericial"}),
    )?;
    Ok(a)
}

const COLUNAS_AMOSTRA:&str="a.id,a.dataset_id,a.documento_id,a.versao,a.inicio_utf8,a.fim_utf8,a.trecho,
    a.sha256_texto_documento,a.sha256_origem,a.sha256_amostra,a.entidades_motor_versao,a.tipos_previstos,
    a.esperado,a.previsto,a.estado_rotulo,a.nota,a.criado_em,a.revisor";

fn ler_amostra(r: &Row<'_>) -> rusqlite::Result<AmostraSupervisionada> {
    fn json_col<T: serde::de::DeserializeOwned>(r: &Row<'_>, indice: usize) -> rusqlite::Result<T> {
        let valor: String = r.get(indice)?;
        serde_json::from_str(&valor).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                indice,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })
    }
    let estado: String = r.get(14)?;
    Ok(AmostraSupervisionada {
        id: r.get(0)?,
        dataset_id: r.get(1)?,
        documento_id: r.get(2)?,
        versao: r.get(3)?,
        inicio_utf8: r.get(4)?,
        fim_utf8: r.get(5)?,
        trecho: r.get(6)?,
        sha256_texto_documento: r.get(7)?,
        sha256_origem: r.get(8)?,
        sha256_amostra: r.get(9)?,
        entidades_motor_versao: r.get(10)?,
        tipos_previstos: json_col(r, 11)?,
        esperado: r.get(12)?,
        previsto: r.get(13)?,
        estado_rotulo: serde_json::from_value(json!(estado))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        nota: r.get(15)?,
        criado_em: r.get(16)?,
        revisor: r.get(17)?,
    })
}

fn conferir_amostra(
    conn: &Connection,
    caso_id: CasoId,
    a: &AmostraSupervisionada,
) -> Resultado<()> {
    let d = documentos::obter(conn, a.documento_id)?;
    conferir_no_documento(caso_id, a, &d)
}

fn conferir_no_documento(
    caso_id: CasoId,
    a: &AmostraSupervisionada,
    d: &documentos::DocumentoIndexado,
) -> Resultado<()> {
    if a.trecho.len() > 4096
        || a.sha256_amostra != hash_amostra(a)
        || d.caso_id != caso_id
        || d.sha256_texto != a.sha256_texto_documento
        || d.sha256_origem != a.sha256_origem
        || d.entidades_motor_versao != a.entidades_motor_versao
        || d.texto.get(a.inicio_utf8 as usize..a.fim_utf8 as usize) != Some(a.trecho.as_str())
    {
        return Err(ErroArmazenamento::Inconsistente(
            "amostra supervisionada não confere com seu hash/origem documental".into(),
        ));
    }
    Ok(())
}

/// Listagem do histórico completo, incluindo anotações substituídas; a avaliação
/// usa somente a versão mais recente de cada intervalo/documento.
pub fn amostras_dataset(
    conn: &Connection,
    caso_id: CasoId,
    dataset_id: i64,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<AmostraSupervisionada>> {
    let dataset = obter_dataset(conn, caso_id, dataset_id)?;
    paginacao(deslocamento, limite)?;
    let mut stmt=conn.prepare(&format!("SELECT {COLUNAS_AMOSTRA} FROM amostra_supervisionada a WHERE a.dataset_id=?1 ORDER BY a.id DESC LIMIT ?2 OFFSET ?3"))?;
    let itens: Vec<_> = stmt
        .query_map(params![dataset_id, limite, deslocamento], ler_amostra)?
        .collect::<std::result::Result<_, _>>()?;
    for a in &itens {
        conferir_amostra(conn, caso_id, a)?;
    }
    Ok(Pagina {
        total: dataset.total_versoes,
        itens,
    })
}

fn fracao(numerador: i64, denominador: i64) -> Option<f64> {
    (denominador > 0).then(|| numerador as f64 / denominador as f64)
}

pub fn avaliar_dataset(
    conn: &Connection,
    caso_id: CasoId,
    dataset_id: i64,
    controle: &Arc<ControleDescoberta>,
) -> Resultado<AvaliacaoSupervisionada> {
    let _guarda = GuardaDescoberta::instalar(conn, controle);
    let resultado = (|| {
        controle.verificar()?;
        let dataset = obter_dataset(conn, caso_id, dataset_id)?;
        let mut a=AvaliacaoSupervisionada{dataset_id,total_amostras:dataset.total_amostras,total_versoes:dataset.total_versoes,
            elegiveis:0,nao_revisadas:0,inconclusivas:0,descartadas:0,tp:0,fp:0,fn_:0,tn:0,
            precision:None,recall:None,f1:None,acuracia:None,cobertura:None,motores_avaliados:Vec::new(),
            limites:vec!["Avaliação dos intervalos explicitamente anotados neste dataset; não mede precisão ou cobertura do caso inteiro.".into(),
                "Previsão é coincidência exata de offsets no snapshot sintático, não identidade, significado ou fato pericial.".into(),
                "Somente rótulos humanos CONFIRMADOS da última versão de cada intervalo são elegíveis. Não revisados, descartados e inconclusivos são excluídos.".into(),
                "Autorização e retenção são declarações registradas; não há treinamento, transferência entre casos ou exclusão automática.".into()]};
        let mut motores = std::collections::BTreeSet::new();
        let mut stmt=conn.prepare(&format!("SELECT {COLUNAS_AMOSTRA} FROM amostra_supervisionada a
            WHERE a.dataset_id=?1 AND a.id=(SELECT MAX(a2.id) FROM amostra_supervisionada a2
                WHERE a2.dataset_id=a.dataset_id AND a2.documento_id=a.documento_id AND a2.inicio_utf8=a.inicio_utf8 AND a2.fim_utf8=a.fim_utf8)
            ORDER BY a.documento_id,a.inicio_utf8,a.fim_utf8 LIMIT 10001"))?;
        let linhas = stmt.query_map([dataset_id], ler_amostra)?;
        // No máximo um texto de até 2 MiB retido; cada documento é conferido
        // uma vez por grupo, sem acumular o corpus do dataset em memória.
        let mut documento_atual: Option<documentos::DocumentoIndexado> = None;
        for linha in linhas {
            controle.verificar()?;
            let amostra = linha?;
            if documento_atual
                .as_ref()
                .is_none_or(|d| d.id != amostra.documento_id)
            {
                documento_atual = Some(documentos::obter(conn, amostra.documento_id)?);
            }
            if let Some(d) = documento_atual.as_ref() {
                conferir_no_documento(caso_id, &amostra, d)?;
            }
            match amostra.estado_rotulo {
                EstadoRevisaoEntidade::NaoRevisado => a.nao_revisadas += 1,
                EstadoRevisaoEntidade::Inconclusivo => a.inconclusivas += 1,
                EstadoRevisaoEntidade::Descartado => a.descartadas += 1,
                EstadoRevisaoEntidade::Confirmado => {
                    a.elegiveis += 1;
                    motores.insert(amostra.entidades_motor_versao);
                    match (amostra.esperado, amostra.previsto) {
                        (true, true) => a.tp += 1,
                        (false, true) => a.fp += 1,
                        (true, false) => a.fn_ += 1,
                        (false, false) => a.tn += 1,
                    }
                }
            }
        }
        if a.elegiveis + a.nao_revisadas + a.inconclusivas + a.descartadas != a.total_amostras
            || a.total_amostras > MAX_AMOSTRAS
        {
            return Err(ErroArmazenamento::Inconsistente(
                "contagem de amostras supervisionadas diverge ou ultrapassa limites".into(),
            ));
        }
        a.precision = fracao(a.tp, a.tp + a.fp);
        a.recall = fracao(a.tp, a.tp + a.fn_);
        a.f1 = fracao(2 * a.tp, 2 * a.tp + a.fp + a.fn_);
        a.acuracia = fracao(a.tp + a.tn, a.elegiveis);
        a.cobertura = fracao(a.elegiveis, a.total_amostras);
        a.motores_avaliados = motores.into_iter().collect();
        if a.motores_avaliados.len() > 1 {
            a.limites.push("Há versões distintas do motor neste dataset; as métricas agregam apenas as amostras selecionadas, não uma comparação controlada entre versões.".into());
        }
        controle.verificar()?;
        Ok(a)
    })();
    resultado.map_err(|e| controle.traduzir_sqlite(e))
}

#[cfg(test)]
mod contratos {
    use super::*;
    #[test]
    fn imprime_contratos_supervisao_rust() {
        macro_rules! exportar {
            ($t:ty) => {
                println!("SIED_CONTRATO_INICIO:{}.ts", <$t>::name());
                println!("{}", <$t>::export_to_string().unwrap());
                println!("SIED_CONTRATO_FIM:{}.ts", <$t>::name());
            };
        }
        exportar!(DatasetSupervisionado);
        exportar!(FiltroFilaRevisao);
        exportar!(CandidatoRevisao);
        exportar!(PedidoAmostraSupervisionada);
        exportar!(AmostraSupervisionada);
        exportar!(AvaliacaoSupervisionada);
    }
}
