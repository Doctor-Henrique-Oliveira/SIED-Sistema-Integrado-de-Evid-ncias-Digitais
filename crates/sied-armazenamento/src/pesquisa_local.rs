//! Vocabulários e snapshots locais AI-0. Nenhum significado criminal predefinido.
//! Persistência exclusivamente no caso; histórico e auditoria na mesma transação.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::auditoria;
use crate::descoberta::{self, ModoDescoberta, PedidoDescoberta, ResultadoDescoberta};
use crate::{ErroArmazenamento, Resultado, Sessao};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct RamoVocabulario {
    pub nome: String,
    pub habilitado: bool,
    pub termos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct VocabularioPesquisa {
    pub nome: String,
    /// Proveniência declarada pelo analista, não um significado inferido.
    pub origem: String,
    pub ramos: Vec<RamoVocabulario>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VersaoVocabularioPesquisa {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub versao: i64,
    pub sha256: String,
    pub criado_em: String,
    pub autor: String,
    pub dados: VocabularioPesquisa,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ConsultaDescobertaSalva {
    #[ts(type = "number")]
    pub id: i64,
    pub sha256: String,
    pub criada_em: String,
    pub pedido: PedidoDescoberta,
    pub termos_efetivos: Vec<String>,
    pub algoritmo: String,
    #[ts(type = "number")]
    pub versao_indice: i64,
    #[ts(type = "number | null")]
    pub vocabulario_id: Option<i64>,
    #[ts(type = "number")]
    pub total_artefatos: i64,
    #[ts(type = "number")]
    pub total_arquivos: i64,
    #[ts(type = "number")]
    pub ultimo_artefato_id: i64,
    #[ts(type = "number")]
    pub ultimo_arquivo_id: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ExecucaoDescoberta {
    pub consulta: ConsultaDescobertaSalva,
    pub resultado: ResultadoDescoberta,
}

fn recusado(mensagem: &str) -> ErroArmazenamento {
    ErroArmazenamento::Recusado(mensagem.into())
}

fn json_texto<T: Serialize>(dados: &T) -> Resultado<String> {
    serde_json::to_string(dados)
        .map_err(|_| recusado("Não foi possível serializar a configuração de pesquisa."))
}

fn sha256(texto: &str) -> String {
    hex::encode(Sha256::digest(texto.as_bytes()))
}

fn hash_vocabulario(
    dados: &VocabularioPesquisa,
    versao: i64,
    criado_em: &str,
    autor: &str,
) -> Resultado<String> {
    // Hash próprio do derivado, inclui proveniência; não é hash_registro forense.
    Ok(sha256(&json_texto(&json!({
        "formato": "sied-vocabulario-pesquisa-v1", "dados": dados,
        "versao": versao, "criado_em": criado_em, "autor": autor
    }))?))
}

fn conferir_json<T: DeserializeOwned>(texto: &str, hash: &str) -> Resultado<T> {
    if sha256(texto) != hash {
        return Err(ErroArmazenamento::Inconsistente(
            "Hash da configuração/snapshot de pesquisa divergente.".into(),
        ));
    }
    serde_json::from_str(texto).map_err(|_| {
        ErroArmazenamento::Inconsistente("Configuração/snapshot de pesquisa ilegível.".into())
    })
}

fn campo_valido(valor: &str, max: usize) -> bool {
    !valor.trim().is_empty()
        && valor.chars().count() <= max
        && !valor.chars().any(|c| c.is_control())
}

fn validar_vocabulario(dados: &VocabularioPesquisa) -> Resultado<()> {
    if !campo_valido(&dados.nome, 100) || !campo_valido(&dados.origem, 300) {
        return Err(recusado(
            "Informe nome (até 100 caracteres) e origem (até 300) do vocabulário.",
        ));
    }
    if dados.ramos.is_empty() || dados.ramos.len() > 8 {
        return Err(recusado("O vocabulário deve ter entre 1 e 8 ramos."));
    }
    if dados.ramos.iter().map(|r| r.termos.len()).sum::<usize>() > 31 {
        return Err(recusado(
            "O vocabulário admite até 31 termos, além da consulta original.",
        ));
    }
    for ramo in &dados.ramos {
        if !campo_valido(&ramo.nome, 100) || ramo.termos.iter().any(|t| !campo_valido(t, 200)) {
            return Err(recusado(
                "Informe o nome de cada ramo e termos não vazios de até 200 caracteres.",
            ));
        }
    }
    Ok(())
}

/// O chamador abre a transação IMMEDIATE, para versão e auditoria serem atômicas.
pub(crate) fn salvar_vocabulario(
    conn: &Connection,
    sessao: &Sessao,
    mut dados: VocabularioPesquisa,
) -> Resultado<VersaoVocabularioPesquisa> {
    dados.nome = dados.nome.trim().into();
    dados.origem = dados.origem.trim().into();
    for ramo in &mut dados.ramos {
        ramo.nome = ramo.nome.trim().into();
        for termo in &mut ramo.termos {
            *termo = termo.trim().into();
        }
    }
    validar_vocabulario(&dados)?;
    let versao = conn.query_row(
        "SELECT coalesce(max(versao), 0) + 1 FROM vocabulario_pesquisa WHERE nome = ?1",
        [&dados.nome],
        |r| r.get(0),
    )?;
    let criado_em = Utc::now().to_rfc3339();
    let texto = json_texto(&dados)?;
    let hash = hash_vocabulario(&dados, versao, &criado_em, &sessao.usuario)?;
    conn.execute(
        "INSERT INTO vocabulario_pesquisa(nome, versao, criado_em, autor, dados, sha256)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![dados.nome, versao, criado_em, sessao.usuario, texto, hash],
    )?;
    let id = conn.last_insert_rowid();
    auditoria::registrar(
        conn,
        sessao,
        "vocabulario_pesquisa_versionado",
        json!({
            "vocabulario_id": id, "versao": versao, "sha256": hash, "escopo": "caso"
        }),
    )?;
    Ok(VersaoVocabularioPesquisa {
        id,
        versao,
        sha256: hash,
        criado_em,
        autor: sessao.usuario.clone(),
        dados,
    })
}

pub(crate) fn obter_vocabulario(
    conn: &Connection,
    id: i64,
) -> Resultado<VersaoVocabularioPesquisa> {
    let linha = conn.query_row(
        "SELECT nome, versao, criado_em, autor, dados, sha256 FROM vocabulario_pesquisa WHERE id = ?1",
        [id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?,
            r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?)),
    ).optional()?.ok_or_else(|| ErroArmazenamento::NaoEncontrado("Vocabulário".into()))?;
    let dados: VocabularioPesquisa = serde_json::from_str(&linha.4)
        .map_err(|_| ErroArmazenamento::Inconsistente("Vocabulário ilegível.".into()))?;
    if hash_vocabulario(&dados, linha.1, &linha.2, &linha.3)? != linha.5 {
        return Err(ErroArmazenamento::Inconsistente(
            "Hash do vocabulário/proveniência divergente.".into(),
        ));
    }
    validar_vocabulario(&dados)?;
    if dados.nome != linha.0 {
        return Err(ErroArmazenamento::Inconsistente(
            "Nome do vocabulário divergente.".into(),
        ));
    }
    Ok(VersaoVocabularioPesquisa {
        id,
        versao: linha.1,
        criado_em: linha.2,
        autor: linha.3,
        dados,
        sha256: linha.5,
    })
}

pub(crate) fn listar_vocabularios(conn: &Connection) -> Resultado<Vec<VersaoVocabularioPesquisa>> {
    let mut stmt =
        conn.prepare("SELECT id FROM vocabulario_pesquisa ORDER BY id DESC LIMIT 100")?;
    let ids = stmt
        .query_map([], |r| r.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    ids.into_iter()
        .map(|id| obter_vocabulario(conn, id))
        .collect()
}

pub(crate) fn exportar_vocabulario(
    conn: &Connection,
    sessao: &Sessao,
    id: i64,
) -> Resultado<String> {
    let versao = obter_vocabulario(conn, id)?;
    let exportado = json_texto(&json!({
        "formato": "sied-vocabulario-pesquisa-v1", "id": versao.id,
        "dados": versao.dados, "versao": versao.versao, "sha256": versao.sha256,
        "autor": versao.autor, "criado_em": versao.criado_em
    }))?;
    auditoria::registrar(
        conn,
        sessao,
        "vocabulario_pesquisa_exportado",
        json!({
            "vocabulario_id": id, "sha256": versao.sha256, "formato": "json", "escopo": "caso"
        }),
    )?;
    Ok(exportado)
}

fn universo(conn: &Connection) -> Resultado<(i64, i64)> {
    Ok((
        conn.query_row("SELECT coalesce(max(id), 0) FROM artefato", [], |r| {
            r.get(0)
        })?,
        conn.query_row("SELECT coalesce(max(id), 0) FROM arquivo", [], |r| r.get(0))?,
    ))
}

/// Snapshot de CONFIGURAÇÃO e contagens, não uma cópia congelada dos resultados.
pub(crate) fn executar(
    conn: &Connection,
    sessao: &Sessao,
    pedido: PedidoDescoberta,
    vocabulario_id: Option<i64>,
) -> Resultado<ExecucaoDescoberta> {
    if let Some(id) = vocabulario_id {
        let vocab = obter_vocabulario(conn, id)?;
        let termos_da_versao: Vec<String> = vocab
            .dados
            .ramos
            .iter()
            .filter(|r| r.habilitado)
            .flat_map(|r| r.termos.clone())
            .collect();
        if !matches!(pedido.modo, ModoDescoberta::Expansao) || pedido.termos != termos_da_versao {
            return Err(recusado(
                "Os termos não pertencem à versão de vocabulário escolhida. Salve uma nova versão.",
            ));
        }
    }
    let resultado = descoberta::buscar(conn, &pedido, 0, 50)?;
    let (ultimo_artefato_id, ultimo_arquivo_id) = universo(conn)?;
    let mut consulta = ConsultaDescobertaSalva {
        id: 0,
        sha256: String::new(),
        criada_em: resultado.executada_em.clone(),
        pedido,
        termos_efetivos: resultado.termos_efetivos.clone(),
        algoritmo: resultado.algoritmo.clone(),
        // Representação FTS criada na migração 3, inalterada pela AI-0.
        versao_indice: 3,
        vocabulario_id,
        total_artefatos: resultado.artefatos.total,
        total_arquivos: resultado.arquivos.total,
        ultimo_artefato_id,
        ultimo_arquivo_id,
    };
    let texto = json_texto(&consulta)?;
    let hash = sha256(&texto);
    conn.execute(
        "INSERT INTO consulta_descoberta(criada_em, dados, sha256, vocabulario_id) VALUES (?1, ?2, ?3, ?4)",
        params![consulta.criada_em, texto, hash, vocabulario_id],
    )?;
    consulta.id = conn.last_insert_rowid();
    consulta.sha256 = hash;
    auditoria::registrar(
        conn,
        sessao,
        "consulta_descoberta_executada",
        json!({
            "consulta_id": consulta.id, "sha256": consulta.sha256, "algoritmo": consulta.algoritmo,
            "registros": consulta.total_artefatos, "arquivos": consulta.total_arquivos,
            "vocabulario_id": vocabulario_id
        }),
    )?;
    Ok(ExecucaoDescoberta {
        consulta,
        resultado,
    })
}

fn obter_consulta(conn: &Connection, id: i64) -> Resultado<ConsultaDescobertaSalva> {
    let linha = conn.query_row(
        "SELECT criada_em, dados, sha256, vocabulario_id FROM consulta_descoberta WHERE id = ?1",
        [id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?,
            r.get::<_, String>(2)?, r.get::<_, Option<i64>>(3)?)),
    ).optional()?.ok_or_else(|| ErroArmazenamento::NaoEncontrado("Consulta".into()))?;
    let mut consulta: ConsultaDescobertaSalva = conferir_json(&linha.1, &linha.2)?;
    if consulta.id != 0
        || !consulta.sha256.is_empty()
        || consulta.criada_em != linha.0
        || consulta.vocabulario_id != linha.3
    {
        return Err(ErroArmazenamento::Inconsistente(
            "Metadados do snapshot divergentes.".into(),
        ));
    }
    consulta.id = id;
    consulta.sha256 = linha.2;
    Ok(consulta)
}

pub(crate) fn listar_consultas(conn: &Connection) -> Resultado<Vec<ConsultaDescobertaSalva>> {
    let mut stmt = conn.prepare("SELECT id FROM consulta_descoberta ORDER BY id DESC LIMIT 20")?;
    let ids = stmt
        .query_map([], |r| r.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    ids.into_iter().map(|id| obter_consulta(conn, id)).collect()
}

pub(crate) fn paginar(
    conn: &Connection,
    id: i64,
    deslocamento: i64,
    limite: i64,
) -> Resultado<ResultadoDescoberta> {
    let consulta = obter_consulta(conn, id)?;
    if consulta.versao_indice != 3 {
        return Err(recusado(
            "Versão de índice do snapshot não suportada. Execute uma nova consulta.",
        ));
    }
    if universo(conn)? != (consulta.ultimo_artefato_id, consulta.ultimo_arquivo_id) {
        return Err(recusado("O caso recebeu novos registros/arquivos. Execute novamente para obter um novo snapshot."));
    }
    let mut resultado = descoberta::buscar(
        conn,
        &consulta.pedido,
        deslocamento.max(0),
        limite.clamp(1, 200),
    )?;
    if resultado.algoritmo != consulta.algoritmo
        || resultado.termos_efetivos != consulta.termos_efetivos
        || resultado.artefatos.total != consulta.total_artefatos
        || resultado.arquivos.total != consulta.total_arquivos
    {
        return Err(recusado("A configuração ou os índices mudaram. Execute novamente para registrar uma nova consulta."));
    }
    resultado.executada_em = consulta.criada_em;
    Ok(resultado)
}
