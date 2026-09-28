//! Consultas sobre extrações importadas: inventário, execuções de parser e artefatos
//! com a referência de origem completa (visão técnica).

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use serde_json::Value;
use sied_dominio::{DispositivoId, ExtracaoId};
use ts_rs::TS;

use crate::{ErroArmazenamento, Resultado};

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Extracao {
    pub id: ExtracaoId,
    pub dispositivo_id: DispositivoId,
    pub importador: String,
    pub importador_nome: String,
    pub importador_versao: String,
    pub ferramenta_versao: Option<String>,
    pub tipo_extracao: Option<String>,
    pub origem_caminho: String,
    pub sied_versao: String,
    pub usuario: String,
    pub maquina: String,
    pub iniciada_em: String,
    pub concluida_em: Option<String>,
    /// em_andamento, concluida, cancelada, falhou, interrompida
    pub status: String,
    #[ts(type = "number | null")]
    pub total_arquivos: Option<i64>,
    #[ts(type = "number | null")]
    pub total_bytes: Option<i64>,
    #[ts(type = "number | null")]
    pub arquivos_ilegiveis: Option<i64>,
    #[ts(type = "number | null")]
    pub itens_ignorados: Option<i64>,
    pub hash_manifesto: Option<String>,
    #[ts(type = "number | null")]
    pub total_artefatos: Option<i64>,
    #[ts(type = "number | null")]
    pub total_avisos: Option<i64>,
    pub erro: Option<String>,
    pub observacoes: Vec<String>,
}

const COLUNAS_EXTRACAO: &str = "id, dispositivo_id, importador, importador_nome, importador_versao,
    ferramenta_versao, tipo_extracao, origem_caminho, sied_versao, usuario, maquina, iniciada_em,
    concluida_em, status, total_arquivos, total_bytes, arquivos_ilegiveis, itens_ignorados,
    hash_manifesto, total_artefatos, total_avisos, erro, observacoes";

pub(crate) fn uuid<T: std::str::FromStr>(texto: String) -> rusqlite::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    texto.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn json_ou_nulo(texto: Option<String>) -> Value {
    texto
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null)
}

fn ler_extracao(r: &Row) -> rusqlite::Result<Extracao> {
    let obs: Option<String> = r.get(22)?;
    Ok(Extracao {
        id: uuid(r.get(0)?)?,
        dispositivo_id: uuid(r.get(1)?)?,
        importador: r.get(2)?,
        importador_nome: r.get(3)?,
        importador_versao: r.get(4)?,
        ferramenta_versao: r.get(5)?,
        tipo_extracao: r.get(6)?,
        origem_caminho: r.get(7)?,
        sied_versao: r.get(8)?,
        usuario: r.get(9)?,
        maquina: r.get(10)?,
        iniciada_em: r.get(11)?,
        concluida_em: r.get(12)?,
        status: r.get(13)?,
        total_arquivos: r.get(14)?,
        total_bytes: r.get(15)?,
        arquivos_ilegiveis: r.get(16)?,
        itens_ignorados: r.get(17)?,
        hash_manifesto: r.get(18)?,
        total_artefatos: r.get(19)?,
        total_avisos: r.get(20)?,
        erro: r.get(21)?,
        observacoes: obs
            .and_then(|o| serde_json::from_str(&o).ok())
            .unwrap_or_default(),
    })
}

pub fn obter(conn: &Connection, id: ExtracaoId) -> Resultado<Extracao> {
    conn.query_row(
        &format!("SELECT {COLUNAS_EXTRACAO} FROM extracao WHERE id = ?1"),
        [id.to_string()],
        ler_extracao,
    )
    .optional()?
    .ok_or_else(|| ErroArmazenamento::NaoEncontrado("extração".into()))
}

pub fn listar(conn: &Connection) -> Resultado<Vec<Extracao>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUNAS_EXTRACAO} FROM extracao ORDER BY iniciada_em, id"
    ))?;
    let v = stmt.query_map([], ler_extracao)?;
    Ok(v.collect::<Result<_, _>>()?)
}

// ---------- Inventário ----------

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ArquivoExtracao {
    #[ts(type = "number")]
    pub id: i64,
    pub caminho: String,
    #[ts(type = "number")]
    pub tamanho: i64,
    pub modificado_em: Option<String>,
    pub criado_em: Option<String>,
    pub sha256: Option<String>,
    pub assinatura: Option<String>,
    pub mime: Option<String>,
    pub categoria: String,
    pub erro: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct TotalCategoria {
    pub categoria: String,
    #[ts(type = "number")]
    pub arquivos: i64,
    #[ts(type = "number")]
    pub bytes: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Pagina<T: TS> {
    #[ts(type = "number")]
    pub total: i64,
    pub itens: Vec<T>,
}

pub fn totais_por_categoria(conn: &Connection, id: ExtracaoId) -> Resultado<Vec<TotalCategoria>> {
    let mut stmt = conn.prepare(
        "SELECT categoria, COUNT(*), COALESCE(SUM(tamanho), 0) FROM arquivo
         WHERE extracao_id = ?1 GROUP BY categoria ORDER BY COUNT(*) DESC",
    )?;
    let v = stmt.query_map([id.to_string()], |r| {
        Ok(TotalCategoria {
            categoria: r.get(0)?,
            arquivos: r.get(1)?,
            bytes: r.get(2)?,
        })
    })?;
    Ok(v.collect::<Result<_, _>>()?)
}

/// `texto` filtra por trecho do caminho (sem diferenciar maiúsculas) ou por hash exato.
pub fn listar_arquivos(
    conn: &Connection,
    id: ExtracaoId,
    categoria: Option<&str>,
    texto: Option<&str>,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<ArquivoExtracao>> {
    // "midia" agrupa imagens, vídeos, áudios e documentos (galeria de mídias).
    let filtro = "extracao_id = ?1
         AND (?2 IS NULL OR categoria = ?2
              OR (?2 = 'midia' AND categoria IN ('imagem', 'video', 'audio', 'documento')))
         AND (?3 IS NULL OR instr(lower(caminho), lower(?3)) > 0 OR sha256 = lower(?3))";
    let texto = texto.map(str::trim).filter(|t| !t.is_empty());
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM arquivo WHERE {filtro}"),
        params![id.to_string(), categoria, texto],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT id, caminho, tamanho, modificado_em, criado_em, sha256, assinatura, mime, categoria, erro
         FROM arquivo WHERE {filtro} ORDER BY caminho LIMIT ?4 OFFSET ?5"
    ))?;
    let itens = stmt
        .query_map(
            params![id.to_string(), categoria, texto, limite, deslocamento],
            |r| {
                Ok(ArquivoExtracao {
                    id: r.get(0)?,
                    caminho: r.get(1)?,
                    tamanho: r.get(2)?,
                    modificado_em: r.get(3)?,
                    criado_em: r.get(4)?,
                    sha256: r.get(5)?,
                    assinatura: r.get(6)?,
                    mime: r.get(7)?,
                    categoria: r.get(8)?,
                    erro: r.get(9)?,
                })
            },
        )?
        .collect::<Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

// ---------- Execuções de parser ----------

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ExecucaoParser {
    #[ts(type = "number")]
    pub id: i64,
    pub parser: String,
    pub parser_nome: String,
    pub parser_versao: String,
    pub arquivo_caminho: Option<String>,
    pub copia_sha256: Option<String>,
    pub iniciada_em: String,
    pub concluida_em: Option<String>,
    pub status: String,
    #[ts(type = "number | null")]
    pub artefatos: Option<i64>,
    #[ts(type = "number | null")]
    pub avisos: Option<i64>,
    pub erro: Option<String>,
}

pub fn listar_execucoes(conn: &Connection, id: ExtracaoId) -> Resultado<Vec<ExecucaoParser>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.parser, e.parser_nome, e.parser_versao, a.caminho, e.copia_sha256, e.iniciada_em,
                e.concluida_em, e.status, e.artefatos, e.avisos, e.erro
         FROM execucao_parser e LEFT JOIN arquivo a ON a.id = e.arquivo_id
         WHERE e.extracao_id = ?1 ORDER BY e.id",
    )?;
    let v = stmt.query_map([id.to_string()], |r| {
        Ok(ExecucaoParser {
            id: r.get(0)?,
            parser: r.get(1)?,
            parser_nome: r.get(2)?,
            parser_versao: r.get(3)?,
            arquivo_caminho: r.get(4)?,
            copia_sha256: r.get(5)?,
            iniciada_em: r.get(6)?,
            concluida_em: r.get(7)?,
            status: r.get(8)?,
            artefatos: r.get(9)?,
            avisos: r.get(10)?,
            erro: r.get(11)?,
        })
    })?;
    Ok(v.collect::<Result<_, _>>()?)
}

// ---------- Artefatos ----------

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ArtefatoResumo {
    #[ts(type = "number")]
    pub id: i64,
    pub tipo: String,
    pub subtipo: Option<String>,
    pub app: Option<String>,
    pub remetente: Option<String>,
    pub destinatarios: Vec<String>,
    pub de_mim: Option<bool>,
    #[ts(type = "number | null")]
    pub data_hora_ms: Option<i64>,
    pub conteudo: Option<String>,
    pub conversa_id_externo: Option<String>,
    pub estado_registro: String,
}

pub(crate) fn ler_resumo(r: &Row) -> rusqlite::Result<ArtefatoResumo> {
    let dest: String = r.get(5)?;
    Ok(ArtefatoResumo {
        id: r.get(0)?,
        tipo: r.get(1)?,
        subtipo: r.get(2)?,
        app: r.get(3)?,
        remetente: r.get(4)?,
        destinatarios: serde_json::from_str(&dest).unwrap_or_default(),
        de_mim: r.get(6)?,
        data_hora_ms: r.get(7)?,
        conteudo: r.get(8)?,
        conversa_id_externo: r.get(9)?,
        estado_registro: r.get(10)?,
    })
}

pub fn listar_artefatos(
    conn: &Connection,
    id: ExtracaoId,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<ArtefatoResumo>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM artefato WHERE extracao_id = ?1",
        [id.to_string()],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT id, tipo, subtipo, app, remetente, destinatarios, de_mim, data_hora_ms, conteudo,
                conversa_id_externo, estado_registro
         FROM artefato WHERE extracao_id = ?1
         ORDER BY data_hora_ms IS NULL, data_hora_ms, id LIMIT ?2 OFFSET ?3",
    )?;
    let itens = stmt
        .query_map(params![id.to_string(), limite, deslocamento], ler_resumo)?
        .collect::<Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ReferenciaOrigemDetalhe {
    pub principal: bool,
    pub papel_arquivo: String,
    #[ts(type = "number")]
    pub arquivo_id: i64,
    pub arquivo_caminho: String,
    pub arquivo_sha256: String,
    pub tabela: Option<String>,
    pub id_registro: Option<String>,
    pub consulta: Option<String>,
    #[ts(type = "number | null")]
    pub deslocamento: Option<i64>,
    #[ts(type = "Record<string, unknown>")]
    pub valores_brutos: Value,
}

/// Tudo sobre um artefato: dados normalizados, como foi lido e de onde veio.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ArtefatoDetalhe {
    pub resumo: ArtefatoResumo,
    pub data_hora_original: Option<String>,
    pub formato_data: Option<String>,
    pub id_externo: Option<String>,
    pub anexo_caminho: Option<String>,
    pub confianca: String,
    #[ts(type = "Record<string, unknown>")]
    pub metadados: Value,
    pub hash_registro: String,
    pub extracao_id: ExtracaoId,
    pub importador: String,
    pub importador_versao: String,
    pub origem_caminho: String,
    pub hash_manifesto: Option<String>,
    pub parser: String,
    pub parser_versao: String,
    pub referencias: Vec<ReferenciaOrigemDetalhe>,
}

pub fn detalhe_artefato(conn: &Connection, artefato_id: i64) -> Resultado<ArtefatoDetalhe> {
    let mut d = conn
        .query_row(
            "SELECT a.id, a.tipo, a.subtipo, a.app, a.remetente, a.destinatarios, a.de_mim, a.data_hora_ms,
                    a.conteudo, a.conversa_id_externo, a.estado_registro,
                    a.data_hora_original, a.formato_data, a.id_externo, a.anexo_caminho, a.confianca,
                    a.metadados, a.hash_registro, x.id, x.importador, x.importador_versao, x.origem_caminho,
                    x.hash_manifesto, e.parser, e.parser_versao
             FROM artefato a
             JOIN extracao x ON x.id = a.extracao_id
             JOIN execucao_parser e ON e.id = a.execucao_id
             WHERE a.id = ?1",
            [artefato_id],
            |r| {
                Ok(ArtefatoDetalhe {
                    resumo: ler_resumo(r)?,
                    data_hora_original: r.get(11)?,
                    formato_data: r.get(12)?,
                    id_externo: r.get(13)?,
                    anexo_caminho: r.get(14)?,
                    confianca: r.get(15)?,
                    metadados: json_ou_nulo(r.get(16)?),
                    hash_registro: r.get(17)?,
                    extracao_id: uuid(r.get(18)?)?,
                    importador: r.get(19)?,
                    importador_versao: r.get(20)?,
                    origem_caminho: r.get(21)?,
                    hash_manifesto: r.get(22)?,
                    parser: r.get(23)?,
                    parser_versao: r.get(24)?,
                    referencias: Vec::new(),
                })
            },
        )
        .optional()?
        .ok_or_else(|| ErroArmazenamento::NaoEncontrado("artefato".into()))?;
    let mut stmt = conn.prepare(
        "SELECT o.principal, o.papel_arquivo, o.arquivo_id, f.caminho, o.arquivo_sha256, o.tabela,
                o.id_registro, o.consulta, o.deslocamento, o.valores_brutos
         FROM referencia_origem o JOIN arquivo f ON f.id = o.arquivo_id
         WHERE o.artefato_id = ?1 ORDER BY o.principal DESC, o.id",
    )?;
    d.referencias = stmt
        .query_map([artefato_id], |r| {
            Ok(ReferenciaOrigemDetalhe {
                principal: r.get(0)?,
                papel_arquivo: r.get(1)?,
                arquivo_id: r.get(2)?,
                arquivo_caminho: r.get(3)?,
                arquivo_sha256: r.get(4)?,
                tabela: r.get(5)?,
                id_registro: r.get(6)?,
                consulta: r.get(7)?,
                deslocamento: r.get(8)?,
                valores_brutos: json_ou_nulo(r.get(9)?),
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(d)
}
