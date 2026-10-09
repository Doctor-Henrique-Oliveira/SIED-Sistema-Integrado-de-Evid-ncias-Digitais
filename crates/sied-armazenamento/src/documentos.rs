//! Snapshots documentais derivados, pesquisa local e revisão humana de ocorrências.
//!
//! Não altera o inventário, artefatos, evidências ou hashes históricos. Texto e
//! candidatos vêm exclusivamente do motor Rust; o frontend não pode gravá-los.
//! O hash do texto é recalculado antes de persistir e ao abrir um snapshot.

use chrono::DateTime;
use rusqlite::{
    params, params_from_iter, types::Value as ValorSql, Connection, OptionalExtension, Row,
    Transaction,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sied_dominio::{CasoId, ExtracaoId};
use ts_rs::TS;

use crate::extracoes::{uuid, Pagina};
use crate::{auditoria, ErroArmazenamento, Resultado, Sessao};

pub const TAMANHO_MAXIMO_TEXTO: usize = 2 * 1024 * 1024;
pub const MAX_ENTIDADES_DOCUMENTO: usize = 1000;

/// Entrada interna, nunca um DTO de escrita aceito por IPC.
#[derive(Debug, Clone)]
pub struct NovoDocumento {
    pub caso_id: CasoId,
    pub extracao_id: ExtracaoId,
    pub arquivo_id: i64,
    pub caminho: String,
    pub sha256_origem: String,
    pub sha256_texto: String,
    pub texto: String,
    pub pagina: Option<u32>,
    pub total_paginas: Option<u32>,
    pub largura_px: Option<u32>,
    pub altura_px: Option<u32>,
    pub regioes: Vec<NovaRegiaoDocumento>,
    pub motor_id: String,
    pub motor_versao: String,
    pub executado_em: String,
    pub entidades_limitadas: bool,
    pub entidades_motor_versao: String,
    pub entidades: Vec<NovaEntidadeDocumento>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NovaRegiaoDocumento {
    pub ordem: u32,
    pub inicio_utf8: u32,
    pub fim_utf8: u32,
    pub texto: String,
    pub x: u32,
    pub y: u32,
    pub largura: u32,
    pub altura: u32,
    pub confianca: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RegiaoDocumento {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub documento_id: i64,
    pub ordem: u32,
    pub inicio_utf8: u32,
    pub fim_utf8: u32,
    pub texto: String,
    pub x: u32,
    pub y: u32,
    pub largura: u32,
    pub altura: u32,
    pub confianca: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NovaEntidadeDocumento {
    pub tipo: String,
    pub inicio_utf8: u32,
    pub fim_utf8: u32,
    pub literal: String,
    pub regra_versao: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DocumentoIndexado {
    #[ts(type = "number")]
    pub id: i64,
    pub caso_id: CasoId,
    pub extracao_id: ExtracaoId,
    #[ts(type = "number")]
    pub arquivo_id: i64,
    pub caminho: String,
    pub sha256_origem: String,
    pub sha256_texto: String,
    pub texto: String,
    pub pagina: Option<u32>,
    pub total_paginas: Option<u32>,
    pub largura_px: Option<u32>,
    pub altura_px: Option<u32>,
    #[ts(type = "number")]
    pub total_regioes: i64,
    pub motor_id: String,
    pub motor_versao: String,
    pub executado_em: String,
    pub entidades_limitadas: bool,
    pub entidades_motor_versao: String,
}

/// Resultado paginado: não transporta documentos inteiros. O trecho contém no
/// máximo 400 caracteres; abrir o resultado recupera o snapshot íntegro.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AchadoDocumento {
    #[ts(type = "number")]
    pub id: i64,
    pub caso_id: CasoId,
    pub extracao_id: ExtracaoId,
    #[ts(type = "number")]
    pub arquivo_id: i64,
    pub caminho: String,
    pub sha256_origem: String,
    pub sha256_texto: String,
    pub pagina: Option<u32>,
    pub total_paginas: Option<u32>,
    pub motor_id: String,
    pub motor_versao: String,
    pub executado_em: String,
    pub entidades_limitadas: bool,
    pub entidades_motor_versao: String,
    pub trecho: String,
}

/// Confirmação significa exclusivamente que a ocorrência literal foi conferida
/// pelo analista. Não confirma identidade, autoria, validade material ou crime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EstadoRevisaoEntidade {
    NaoRevisado,
    Confirmado,
    Descartado,
    Inconclusivo,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct EntidadeDocumento {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub documento_id: i64,
    pub tipo: String,
    pub inicio_utf8: u32,
    pub fim_utf8: u32,
    pub literal: String,
    pub regra_versao: String,
    pub estado: EstadoRevisaoEntidade,
    pub nota: Option<String>,
    pub revista_em: Option<String>,
    pub revisor: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RevisaoEntidade {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub entidade_id: i64,
    pub estado: EstadoRevisaoEntidade,
    pub nota: Option<String>,
    pub revista_em: String,
    pub revisor: String,
    pub maquina: String,
}

const COLUNAS: &str = "d.id, d.caso_id, d.extracao_id, d.arquivo_id, d.caminho,
    d.sha256_origem, d.sha256_texto, d.texto, d.pagina, d.total_paginas,
    d.motor_id, d.motor_versao, d.executado_em, d.entidades_limitadas,
    d.entidades_motor_versao, d.largura_px, d.altura_px,
    (SELECT COUNT(*) FROM regiao_documental r WHERE r.documento_id = d.id)";

fn recusar(motivo: &str) -> ErroArmazenamento {
    ErroArmazenamento::Recusado(motivo.into())
}

fn hash(texto: &str) -> String {
    hex::encode(Sha256::digest(texto.as_bytes()))
}

fn hash_valido(valor: &str) -> bool {
    valor.len() == 64
        && valor
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn rotulo_valido(valor: &str) -> bool {
    !valor.is_empty() && valor.len() <= 128 && !valor.chars().any(char::is_control)
}

fn tipo_valido(tipo: &str) -> bool {
    matches!(
        tipo,
        "telefone" | "email" | "url" | "dominio" | "ip" | "imei"
    )
}

fn validar_ocorrencia(texto: &str, entidade: &NovaEntidadeDocumento) -> Resultado<()> {
    let inicio = entidade.inicio_utf8 as usize;
    let fim = entidade.fim_utf8 as usize;
    if !tipo_valido(&entidade.tipo)
        || !rotulo_valido(&entidade.regra_versao)
        || inicio >= fim
        || entidade.literal.len() > 4096
        || texto.get(inicio..fim) != Some(entidade.literal.as_str())
    {
        return Err(recusar(
            "ocorrência candidata não confere com o texto documental",
        ));
    }
    Ok(())
}

fn validar_novo(novo: &NovoDocumento) -> Resultado<()> {
    if novo.arquivo_id <= 0
        || novo.texto.len() > TAMANHO_MAXIMO_TEXTO
        || !hash_valido(&novo.sha256_origem)
        || !hash_valido(&novo.sha256_texto)
        || hash(&novo.texto) != novo.sha256_texto
        || !rotulo_valido(&novo.motor_id)
        || !rotulo_valido(&novo.motor_versao)
        || !rotulo_valido(&novo.entidades_motor_versao)
        || DateTime::parse_from_rfc3339(&novo.executado_em).is_err()
        || novo.entidades.len() > MAX_ENTIDADES_DOCUMENTO
    {
        return Err(recusar("snapshot documental inválido ou acima dos limites"));
    }
    match (novo.pagina, novo.total_paginas) {
        (None, None) => {}
        (Some(p), Some(total)) if p > 0 && p <= total && total <= 512 => {}
        _ => return Err(recusar("página e total de páginas não conferem")),
    }
    match (novo.largura_px, novo.altura_px, novo.regioes.is_empty()) {
        (None, None, true) => {}
        (Some(l), Some(a), _) if l > 0 && a > 0 => {}
        _ => return Err(recusar("dimensões e regiões OCR não conferem")),
    }
    if novo.regioes.len() > 50_000 {
        return Err(recusar("snapshot OCR excede o limite de regiões"));
    }
    for (ordem, regiao) in novo.regioes.iter().enumerate() {
        let inicio = regiao.inicio_utf8 as usize;
        let fim = regiao.fim_utf8 as usize;
        if regiao.ordem as usize != ordem
            || inicio >= fim
            || novo.texto.get(inicio..fim) != Some(regiao.texto.as_str())
            || regiao.confianca.is_some_and(|c| c > 100)
            || regiao.largura == 0
            || regiao.altura == 0
            || regiao.x.saturating_add(regiao.largura) > novo.largura_px.unwrap_or(0)
            || regiao.y.saturating_add(regiao.altura) > novo.altura_px.unwrap_or(0)
        {
            return Err(recusar("região OCR não corresponde ao texto ou à imagem"));
        }
    }
    let mut vistos = std::collections::BTreeSet::new();
    for entidade in &novo.entidades {
        validar_ocorrencia(&novo.texto, entidade)?;
        if !vistos.insert((
            &entidade.tipo,
            entidade.inicio_utf8,
            entidade.fim_utf8,
            &entidade.regra_versao,
        )) {
            return Err(recusar("ocorrência candidata duplicada no snapshot"));
        }
    }
    Ok(())
}

fn origem_confere(
    conn: &Connection,
    caso_id: CasoId,
    extracao_id: ExtracaoId,
    arquivo_id: i64,
    caminho: &str,
    sha256: &str,
) -> Resultado<()> {
    let confere: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM arquivo a
         JOIN extracao x ON x.id = a.extracao_id
         JOIN dispositivo v ON v.id = x.dispositivo_id
         JOIN caso c ON c.id = v.caso_id
         WHERE c.id = ?1 AND x.id = ?2 AND a.id = ?3
           AND a.caminho = ?4 AND a.sha256 = ?5)",
        params![
            caso_id.to_string(),
            extracao_id.to_string(),
            arquivo_id,
            caminho,
            sha256
        ],
        |r| r.get(0),
    )?;
    if !confere {
        return Err(recusar(
            "origem documental não corresponde ao inventário deste caso",
        ));
    }
    Ok(())
}

fn ler_documento(r: &Row<'_>) -> rusqlite::Result<DocumentoIndexado> {
    let pagina: u32 = r.get(8)?;
    Ok(DocumentoIndexado {
        id: r.get(0)?,
        caso_id: uuid(r.get(1)?)?,
        extracao_id: uuid(r.get(2)?)?,
        arquivo_id: r.get(3)?,
        caminho: r.get(4)?,
        sha256_origem: r.get(5)?,
        sha256_texto: r.get(6)?,
        texto: r.get(7)?,
        pagina: (pagina != 0).then_some(pagina),
        total_paginas: r.get(9)?,
        motor_id: r.get(10)?,
        motor_versao: r.get(11)?,
        executado_em: r.get(12)?,
        entidades_limitadas: r.get(13)?,
        entidades_motor_versao: r.get(14)?,
        largura_px: r.get(15)?,
        altura_px: r.get(16)?,
        total_regioes: r.get(17)?,
    })
}

fn ler_regiao(r: &Row<'_>) -> rusqlite::Result<RegiaoDocumento> {
    Ok(RegiaoDocumento {
        id: r.get(0)?,
        documento_id: r.get(1)?,
        ordem: r.get(2)?,
        inicio_utf8: r.get(3)?,
        fim_utf8: r.get(4)?,
        texto: r.get(5)?,
        x: r.get(6)?,
        y: r.get(7)?,
        largura: r.get(8)?,
        altura: r.get(9)?,
        confianca: r.get(10)?,
    })
}

fn regioes_originais(conn: &Connection, documento_id: i64) -> Resultado<Vec<NovaRegiaoDocumento>> {
    let mut stmt = conn.prepare(
        "SELECT ordem, inicio_utf8, fim_utf8, texto,
                x, y, largura, altura, confianca
         FROM regiao_documental WHERE documento_id = ?1 ORDER BY ordem LIMIT 50001",
    )?;
    let itens = stmt
        .query_map([documento_id], |r| {
            Ok(NovaRegiaoDocumento {
                ordem: r.get(0)?,
                inicio_utf8: r.get(1)?,
                fim_utf8: r.get(2)?,
                texto: r.get(3)?,
                x: r.get(4)?,
                y: r.get(5)?,
                largura: r.get(6)?,
                altura: r.get(7)?,
                confianca: r.get(8)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if itens.len() > 50_000 {
        return Err(ErroArmazenamento::Inconsistente(
            "snapshot OCR excede o limite de regiões".into(),
        ));
    }
    Ok(itens)
}

pub fn obter(conn: &Connection, id: i64) -> Resultado<DocumentoIndexado> {
    if id <= 0 {
        return Err(recusar("identificador documental inválido"));
    }
    let documento = conn
        .query_row(
            &format!("SELECT {COLUNAS} FROM documento_derivado d WHERE d.id = ?1"),
            [id],
            ler_documento,
        )
        .optional()?
        .ok_or_else(|| ErroArmazenamento::NaoEncontrado("documento derivado".into()))?;
    if documento.texto.len() > TAMANHO_MAXIMO_TEXTO
        || !hash_valido(&documento.sha256_texto)
        || hash(&documento.texto) != documento.sha256_texto
    {
        return Err(ErroArmazenamento::Inconsistente(
            "conteúdo do snapshot documental não confere com seu hash".into(),
        ));
    }
    origem_confere(
        conn,
        documento.caso_id,
        documento.extracao_id,
        documento.arquivo_id,
        &documento.caminho,
        &documento.sha256_origem,
    )?;
    Ok(documento)
}

fn candidatos_originais(
    conn: &Connection,
    documento_id: i64,
) -> Resultado<Vec<NovaEntidadeDocumento>> {
    let mut stmt = conn.prepare(
        "SELECT tipo, inicio_utf8, fim_utf8, literal, regra_versao
         FROM entidade_documental WHERE documento_id = ?1
         ORDER BY inicio_utf8, fim_utf8, tipo, regra_versao LIMIT 1001",
    )?;
    let candidatos = stmt
        .query_map([documento_id], |r| {
            Ok(NovaEntidadeDocumento {
                tipo: r.get(0)?,
                inicio_utf8: r.get(1)?,
                fim_utf8: r.get(2)?,
                literal: r.get(3)?,
                regra_versao: r.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if candidatos.len() > MAX_ENTIDADES_DOCUMENTO {
        return Err(ErroArmazenamento::Inconsistente(
            "snapshot possui mais ocorrências candidatas que o limite".into(),
        ));
    }
    Ok(candidatos)
}

/// Grava snapshot+candidatos+auditoria atomicamente. Entradas idênticas reutilizam
/// o snapshot sem apagar revisões; nova versão do motor cria outro snapshot.
pub fn guardar(
    tx: &Transaction<'_>,
    sessao: &Sessao,
    novo: &NovoDocumento,
) -> Resultado<DocumentoIndexado> {
    validar_novo(novo)?;
    origem_confere(
        tx,
        novo.caso_id,
        novo.extracao_id,
        novo.arquivo_id,
        &novo.caminho,
        &novo.sha256_origem,
    )?;
    let pagina = novo.pagina.unwrap_or(0);
    let existente: Option<i64> = tx
        .query_row(
            "SELECT id FROM documento_derivado WHERE arquivo_id = ?1 AND sha256_origem = ?2
         AND motor_id = ?3 AND motor_versao = ?4 AND pagina = ?5 AND sha256_texto = ?6
         AND entidades_motor_versao = ?7",
            params![
                novo.arquivo_id,
                novo.sha256_origem,
                novo.motor_id,
                novo.motor_versao,
                pagina,
                novo.sha256_texto,
                novo.entidades_motor_versao
            ],
            |r| r.get(0),
        )
        .optional()?;
    let (id, reutilizado) = if let Some(id) = existente {
        let guardado = obter(tx, id)?;
        let mut esperados = novo.entidades.clone();
        esperados.sort_by(|a, b| {
            (a.inicio_utf8, a.fim_utf8, &a.tipo, &a.regra_versao).cmp(&(
                b.inicio_utf8,
                b.fim_utf8,
                &b.tipo,
                &b.regra_versao,
            ))
        });
        if guardado.texto != novo.texto
            || guardado.total_paginas != novo.total_paginas
            || guardado.largura_px != novo.largura_px
            || guardado.altura_px != novo.altura_px
            || regioes_originais(tx, id)?.as_slice() != novo.regioes.as_slice()
            || guardado.entidades_limitadas != novo.entidades_limitadas
            || candidatos_originais(tx, id)? != esperados
        {
            return Err(recusar(
                "reprocessamento divergente requer uma nova versão do motor",
            ));
        }
        (id, true)
    } else {
        tx.execute(
            "INSERT INTO documento_derivado(caso_id, extracao_id, arquivo_id, caminho,
             sha256_origem, sha256_texto, texto, pagina, total_paginas, motor_id,
             motor_versao, executado_em, entidades_limitadas, entidades_motor_versao,
             largura_px, altura_px)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                novo.caso_id.to_string(),
                novo.extracao_id.to_string(),
                novo.arquivo_id,
                novo.caminho,
                novo.sha256_origem,
                novo.sha256_texto,
                novo.texto,
                pagina,
                novo.total_paginas,
                novo.motor_id,
                novo.motor_versao,
                novo.executado_em,
                novo.entidades_limitadas,
                novo.entidades_motor_versao,
                novo.largura_px,
                novo.altura_px
            ],
        )?;
        let id = tx.last_insert_rowid();
        for r in &novo.regioes {
            tx.execute(
                "INSERT INTO regiao_documental(documento_id, ordem, inicio_utf8, fim_utf8,
                 texto, x, y, largura, altura, confianca)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    id,
                    r.ordem,
                    r.inicio_utf8,
                    r.fim_utf8,
                    r.texto,
                    r.x,
                    r.y,
                    r.largura,
                    r.altura,
                    r.confianca
                ],
            )?;
        }
        for e in &novo.entidades {
            tx.execute(
                "INSERT INTO entidade_documental(documento_id, tipo, inicio_utf8, fim_utf8, literal, regra_versao)
                 VALUES(?1,?2,?3,?4,?5,?6)",
                params![id, e.tipo, e.inicio_utf8, e.fim_utf8, e.literal, e.regra_versao],
            )?;
        }
        (id, false)
    };
    auditoria::registrar(
        tx,
        sessao,
        if reutilizado {
            "documento_derivado_reutilizado"
        } else {
            "documento_derivado_indexado"
        },
        json!({"documento_id": id, "arquivo_id": novo.arquivo_id,
               "sha256_origem": novo.sha256_origem, "sha256_texto": novo.sha256_texto,
               "pagina": novo.pagina, "motor_id": novo.motor_id, "motor_versao": novo.motor_versao,
               "entidades_motor_versao": novo.entidades_motor_versao,
               "regioes_ocr": novo.regioes.len(), "largura_px": novo.largura_px,
               "altura_px": novo.altura_px,
               "ocorrencias_candidatas": novo.entidades.len(), "entidades_limitadas": novo.entidades_limitadas}),
    )?;
    obter(tx, id)
}

fn paginacao(deslocamento: i64, limite: i64) -> Resultado<(i64, i64)> {
    if deslocamento < 0 || !(1..=100).contains(&limite) {
        return Err(recusar(
            "paginação documental deve ter deslocamento não negativo e limite de 1 a 100",
        ));
    }
    Ok((deslocamento, limite))
}

/// Regiões OCR são carregadas sob demanda. Abrir o documento valida primeiro o
/// hash do derivado e sua origem registrada; nenhuma página transporta mais de
/// 100 caixas, mesmo quando o snapshot atingiu o limite técnico de 50.000.
pub fn regioes(
    conn: &Connection,
    documento_id: i64,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<RegiaoDocumento>> {
    let (deslocamento, limite) = paginacao(deslocamento, limite)?;
    let documento = obter(conn, documento_id)?;
    if documento.total_regioes > 50_000 {
        return Err(ErroArmazenamento::Inconsistente(
            "snapshot OCR excede o limite de regiões".into(),
        ));
    }
    let mut stmt = conn.prepare(
        "SELECT id, documento_id, ordem, inicio_utf8, fim_utf8, texto,
                x, y, largura, altura, confianca
         FROM regiao_documental WHERE documento_id = ?1
         ORDER BY ordem LIMIT ?2 OFFSET ?3",
    )?;
    let itens = stmt
        .query_map(params![documento_id, limite, deslocamento], ler_regiao)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Pagina {
        total: documento.total_regioes,
        itens,
    })
}

/// Resolve uma faixa do texto persistido para a primeira região OCR que a
/// intersecta. A faixa é conferida contra fronteiras UTF-8 do snapshot antes da
/// consulta; o frontend não escolhe diretamente uma caixa nem sua ordem.
pub fn localizar_regiao(
    conn: &Connection,
    documento_id: i64,
    inicio_utf8: u32,
    fim_utf8: u32,
) -> Resultado<Option<u32>> {
    let documento = obter(conn, documento_id)?;
    let inicio = inicio_utf8 as usize;
    let fim = fim_utf8 as usize;
    if inicio >= fim
        || fim > documento.texto.len()
        || !documento.texto.is_char_boundary(inicio)
        || !documento.texto.is_char_boundary(fim)
    {
        return Err(recusar("faixa textual para localização OCR é inválida"));
    }
    Ok(conn
        .query_row(
            "SELECT ordem FROM regiao_documental
             WHERE documento_id = ?1 AND inicio_utf8 < ?3 AND fim_utf8 > ?2
             ORDER BY ordem LIMIT 1",
            params![documento_id, inicio_utf8, fim_utf8],
            |r| r.get(0),
        )
        .optional()?)
}

/// Pesquisa somente os derivados já analisados. Não processa arquivos nem faz
/// rede; os operadores digitados são escapados pela consulta FTS existente.
pub fn buscar(
    conn: &Connection,
    termo: &str,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<AchadoDocumento>> {
    buscar_filtrado(
        conn,
        &crate::busca::FiltroBusca {
            texto: termo.into(),
            ..Default::default()
        },
        deslocamento,
        limite,
    )
}

/// Filtros documentais aplicados antes da janela FTS, não depois de recuperar
/// os primeiros candidatos de outras extrações. Datas vêm do inventário, nunca
/// da indexação nem de uma data inferida do conteúdo.
pub fn buscar_filtrado(
    conn: &Connection,
    filtro: &crate::busca::FiltroBusca,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<AchadoDocumento>> {
    let (deslocamento, limite) = paginacao(deslocamento, limite)?;
    let termo = &filtro.texto;
    if termo.len() > 1024
        || termo.contains('\0')
        || termo.split_whitespace().count() > 64
        || filtro
            .de_ms
            .zip(filtro.ate_ms)
            .is_some_and(|(de, ate)| de > ate)
    {
        return Err(recusar("consulta documental acima dos limites"));
    }
    if filtro.app.as_deref().is_some_and(|a| !a.is_empty())
        || filtro.tipo.as_deref().is_some_and(|t| !t.is_empty())
    {
        return Ok(Pagina {
            total: 0,
            itens: Vec::new(),
        });
    }
    let Some(consulta) = crate::busca::consulta_fts(termo) else {
        return Ok(Pagina {
            total: 0,
            itens: Vec::new(),
        });
    };
    let mut condicoes = "busca_documento MATCH ?".to_string();
    let mut valores = vec![ValorSql::Text(consulta)];
    if let Some(extracao) = filtro.extracao_id {
        condicoes.push_str(" AND d.extracao_id = ?");
        valores.push(ValorSql::Text(extracao.to_string()));
    }
    let data_ms = "sied_data_arquivo_ms(COALESCE(a.criado_em,a.modificado_em))";
    if let Some(de) = filtro.de_ms {
        condicoes.push_str(&format!(" AND {data_ms} >= ?"));
        valores.push(ValorSql::Integer(de));
    }
    if let Some(ate) = filtro.ate_ms {
        condicoes.push_str(&format!(" AND {data_ms} <= ?"));
        valores.push(ValorSql::Integer(ate));
    }
    let tabelas = "busca_documento JOIN documento_derivado d ON d.id = busca_documento.rowid
        JOIN arquivo a ON a.id = d.arquivo_id";
    let total = conn.query_row(
        &format!("SELECT COUNT(*) FROM {tabelas} WHERE {condicoes}"),
        params_from_iter(valores.iter()),
        |r| r.get(0),
    )?;
    // Mesmo conjunto de colunas do documento, substituindo somente o texto por
    // trecho limitado. Nunca confundir o trecho com o conteúdo integral/hash.
    let colunas = COLUNAS.replace(
        "d.texto",
        "substr(snippet(busca_documento,0,'','',' … ',24),1,400)",
    );
    let mut stmt = conn.prepare(&format!(
        "SELECT {colunas} FROM {tabelas} WHERE {condicoes}
         ORDER BY bm25(busca_documento), d.id DESC LIMIT ? OFFSET ?"
    ))?;
    valores.push(ValorSql::Integer(limite));
    valores.push(ValorSql::Integer(deslocamento));
    let itens = stmt
        .query_map(params_from_iter(valores.iter()), |r| {
            let d = ler_documento(r)?;
            Ok(AchadoDocumento {
                id: d.id,
                caso_id: d.caso_id,
                extracao_id: d.extracao_id,
                arquivo_id: d.arquivo_id,
                caminho: d.caminho,
                sha256_origem: d.sha256_origem,
                sha256_texto: d.sha256_texto,
                pagina: d.pagina,
                total_paginas: d.total_paginas,
                motor_id: d.motor_id,
                motor_versao: d.motor_versao,
                executado_em: d.executado_em,
                entidades_limitadas: d.entidades_limitadas,
                entidades_motor_versao: d.entidades_motor_versao,
                trecho: d.texto,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

fn estado_str(estado: EstadoRevisaoEntidade) -> &'static str {
    match estado {
        EstadoRevisaoEntidade::NaoRevisado => "nao_revisado",
        EstadoRevisaoEntidade::Confirmado => "confirmado",
        EstadoRevisaoEntidade::Descartado => "descartado",
        EstadoRevisaoEntidade::Inconclusivo => "inconclusivo",
    }
}

fn ler_estado(valor: String) -> rusqlite::Result<EstadoRevisaoEntidade> {
    match valor.as_str() {
        "nao_revisado" => Ok(EstadoRevisaoEntidade::NaoRevisado),
        "confirmado" => Ok(EstadoRevisaoEntidade::Confirmado),
        "descartado" => Ok(EstadoRevisaoEntidade::Descartado),
        "inconclusivo" => Ok(EstadoRevisaoEntidade::Inconclusivo),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

const COLUNAS_ENTIDADE: &str = "e.id, e.documento_id, e.tipo, e.inicio_utf8, e.fim_utf8,
    e.literal, e.regra_versao, coalesce(r.estado,'nao_revisado'), r.nota, r.revista_em, r.revisor";
const ULTIMA_REVISAO: &str = "LEFT JOIN revisao_entidade_documental r ON r.id =
    (SELECT MAX(r2.id) FROM revisao_entidade_documental r2 WHERE r2.entidade_id = e.id)";

fn ler_entidade(r: &Row<'_>) -> rusqlite::Result<EntidadeDocumento> {
    Ok(EntidadeDocumento {
        id: r.get(0)?,
        documento_id: r.get(1)?,
        tipo: r.get(2)?,
        inicio_utf8: r.get(3)?,
        fim_utf8: r.get(4)?,
        literal: r.get(5)?,
        regra_versao: r.get(6)?,
        estado: ler_estado(r.get(7)?)?,
        nota: r.get(8)?,
        revista_em: r.get(9)?,
        revisor: r.get(10)?,
    })
}

fn conferir_entidade(documento: &DocumentoIndexado, e: &EntidadeDocumento) -> Resultado<()> {
    validar_ocorrencia(
        &documento.texto,
        &NovaEntidadeDocumento {
            tipo: e.tipo.clone(),
            inicio_utf8: e.inicio_utf8,
            fim_utf8: e.fim_utf8,
            literal: e.literal.clone(),
            regra_versao: e.regra_versao.clone(),
        },
    )
}

pub fn entidades(conn: &Connection, documento_id: i64) -> Resultado<Vec<EntidadeDocumento>> {
    let documento = obter(conn, documento_id)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUNAS_ENTIDADE} FROM entidade_documental e {ULTIMA_REVISAO}
         WHERE e.documento_id = ?1 ORDER BY e.inicio_utf8, e.id LIMIT 1001"
    ))?;
    let entidades = stmt
        .query_map([documento_id], ler_entidade)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if entidades.len() > MAX_ENTIDADES_DOCUMENTO {
        return Err(ErroArmazenamento::Inconsistente(
            "snapshot excede o limite de candidatos".into(),
        ));
    }
    for e in &entidades {
        conferir_entidade(&documento, e)?;
    }
    Ok(entidades)
}

fn obter_entidade(conn: &Connection, id: i64) -> Resultado<EntidadeDocumento> {
    let entidade = conn
        .query_row(
            &format!(
        "SELECT {COLUNAS_ENTIDADE} FROM entidade_documental e {ULTIMA_REVISAO} WHERE e.id = ?1"
    ),
            [id],
            ler_entidade,
        )
        .optional()?
        .ok_or_else(|| ErroArmazenamento::NaoEncontrado("ocorrência candidata".into()))?;
    let documento = obter(conn, entidade.documento_id)?;
    conferir_entidade(&documento, &entidade)?;
    Ok(entidade)
}

pub fn revisar(
    tx: &Transaction<'_>,
    sessao: &Sessao,
    id: i64,
    estado: EstadoRevisaoEntidade,
    nota: Option<&str>,
) -> Resultado<EntidadeDocumento> {
    let entidade = obter_entidade(tx, id)?;
    if nota.is_some_and(|n| n.len() > 4000 || n.contains('\0')) {
        return Err(recusar(
            "nota de revisão deve ter no máximo 4000 bytes e não pode conter NUL",
        ));
    }
    let nota = nota.filter(|n| !n.trim().is_empty());
    let agora = chrono::Utc::now().to_rfc3339();
    tx.execute(
        "INSERT INTO revisao_entidade_documental(entidade_id, estado, nota, revista_em, revisor, maquina)
         VALUES(?1,?2,?3,?4,?5,?6)",
        params![id, estado_str(estado), nota, agora, sessao.usuario, sessao.maquina],
    )?;
    let revisao_id = tx.last_insert_rowid();
    auditoria::registrar(
        tx,
        sessao,
        "entidade_documental_revisada",
        json!({
            "documento_id": entidade.documento_id, "entidade_id": id, "revisao_id": revisao_id,
            "estado_anterior": estado_str(entidade.estado), "estado": estado_str(estado),
            "nota_presente": nota.is_some(), "escopo_confirmacao": "ocorrencia_literal"
        }),
    )?;
    obter_entidade(tx, id)
}

pub fn revisoes(
    conn: &Connection,
    entidade_id: i64,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<RevisaoEntidade>> {
    obter_entidade(conn, entidade_id)?;
    let (deslocamento, limite) = paginacao(deslocamento, limite)?;
    let total = conn.query_row(
        "SELECT COUNT(*) FROM revisao_entidade_documental WHERE entidade_id = ?1",
        [entidade_id],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT id, entidade_id, estado, nota, revista_em, revisor, maquina
         FROM revisao_entidade_documental WHERE entidade_id = ?1 ORDER BY id DESC LIMIT ?2 OFFSET ?3"
    )?;
    let itens = stmt
        .query_map(params![entidade_id, limite, deslocamento], |r| {
            Ok(RevisaoEntidade {
                id: r.get(0)?,
                entidade_id: r.get(1)?,
                estado: ler_estado(r.get(2)?)?,
                nota: r.get(3)?,
                revista_em: r.get(4)?,
                revisor: r.get(5)?,
                maquina: r.get(6)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

#[cfg(test)]
mod contratos {
    use super::*;

    #[test]
    fn imprime_contratos_documentais_rust() {
        macro_rules! contrato {
            ($tipo:ty) => {
                println!("SIED_CONTRATO_INICIO:{}.ts", <$tipo>::name());
                println!("{}", <$tipo>::export_to_string().unwrap());
                println!("SIED_CONTRATO_FIM:{}.ts", <$tipo>::name());
            };
        }
        contrato!(DocumentoIndexado);
        contrato!(RegiaoDocumento);
        contrato!(AchadoDocumento);
        contrato!(EstadoRevisaoEntidade);
        contrato!(EntidadeDocumento);
        contrato!(RevisaoEntidade);
        contrato!(crate::descoberta::ModoDescoberta);
        contrato!(crate::descoberta::TipoIndicador);
        contrato!(crate::descoberta::PedidoDescoberta);
        contrato!(crate::descoberta::IndicadorDescoberta);
        contrato!(crate::descoberta::AchadoDescoberta);
        contrato!(crate::descoberta::ResultadoDescoberta);
        contrato!(crate::pesquisa_local::RamoVocabulario);
        contrato!(crate::pesquisa_local::VocabularioPesquisa);
        contrato!(crate::pesquisa_local::VersaoVocabularioPesquisa);
        contrato!(crate::pesquisa_local::ConsultaDescobertaSalva);
        contrato!(crate::pesquisa_local::ExecucaoDescoberta);
    }
}
