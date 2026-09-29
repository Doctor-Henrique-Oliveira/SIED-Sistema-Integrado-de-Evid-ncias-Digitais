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
         AND (?3 IS NULL OR instr(sied_normal(caminho), sied_normal(?3)) > 0 OR sha256 = lower(?3))";
    let texto = texto.map(str::trim).filter(|t| !t.is_empty());
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM arquivo WHERE {filtro}"),
        params![id.to_string(), categoria, texto],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUNAS_ARQUIVO} FROM arquivo WHERE {filtro} ORDER BY caminho LIMIT ?4 OFFSET ?5"
    ))?;
    let itens = stmt
        .query_map(
            params![id.to_string(), categoria, texto, limite, deslocamento],
            ler_arquivo,
        )?
        .collect::<Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

const COLUNAS_ARQUIVO: &str =
    "id, caminho, tamanho, modificado_em, criado_em, sha256, assinatura, mime, categoria, erro";

fn ler_arquivo(r: &Row) -> rusqlite::Result<ArquivoExtracao> {
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
}

/// Um arquivo do inventário e a pasta da extração a que pertence (para ler a
/// evidência, sempre somente leitura).
#[derive(Debug, Clone)]
pub struct ArquivoComOrigem {
    pub arquivo: ArquivoExtracao,
    pub origem_caminho: String,
}

pub fn arquivo_com_origem(conn: &Connection, arquivo_id: i64) -> Resultado<ArquivoComOrigem> {
    conn.query_row(
        &format!(
            "SELECT {}, x.origem_caminho FROM arquivo a JOIN extracao x ON x.id = a.extracao_id
             WHERE a.id = ?1",
            COLUNAS_ARQUIVO
                .split(", ")
                .map(|c| format!("a.{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        [arquivo_id],
        |r| {
            Ok(ArquivoComOrigem {
                arquivo: ler_arquivo(r)?,
                origem_caminho: r.get(10)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| ErroArmazenamento::NaoEncontrado("arquivo".into()))
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
    /// Identificador do registro no app (ex.: key_id da mensagem).
    pub id_externo: Option<String>,
    /// `id_externo` da mensagem respondida.
    pub resposta_a_id_externo: Option<String>,
    pub nome_arquivo: Option<String>,
    pub mime: Option<String>,
    #[ts(type = "number | null")]
    pub tamanho: Option<i64>,
    /// Arquivo do inventário ligado ao anexo, quando encontrado.
    #[ts(type = "number | null")]
    pub anexo_arquivo_id: Option<i64>,
    /// Como o anexo foi procurado no inventário (ver `VinculoAnexo`).
    pub anexo_vinculo: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Colunas lidas por [`ler_resumo`], com a tabela `artefato` como `a`.
pub(crate) const COLUNAS_RESUMO: &str = "a.id, a.tipo, a.subtipo, a.app, a.remetente,
    a.destinatarios, a.de_mim, a.data_hora_ms, a.conteudo, a.conversa_id_externo,
    a.estado_registro, a.id_externo, a.resposta_a_id_externo, a.nome_arquivo, a.mime, a.tamanho,
    a.anexo_arquivo_id, a.anexo_vinculo, a.latitude, a.longitude";
/// Quantas colunas [`COLUNAS_RESUMO`] ocupa (as seguintes começam neste índice).
pub(crate) const N_RESUMO: usize = 20;

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
        id_externo: r.get(11)?,
        resposta_a_id_externo: r.get(12)?,
        nome_arquivo: r.get(13)?,
        mime: r.get(14)?,
        tamanho: r.get(15)?,
        anexo_arquivo_id: r.get(16)?,
        anexo_vinculo: r.get(17)?,
        latitude: r.get(18)?,
        longitude: r.get(19)?,
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
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUNAS_RESUMO} FROM artefato a WHERE a.extracao_id = ?1
         ORDER BY a.data_hora_ms IS NULL, a.data_hora_ms, a.id LIMIT ?2 OFFSET ?3"
    ))?;
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

/// Arquivo do inventário ligado ao anexo de um artefato.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AnexoInventario {
    #[ts(type = "number")]
    pub id: i64,
    pub caminho: String,
    pub sha256: Option<String>,
    #[ts(type = "number")]
    pub tamanho: i64,
    pub categoria: String,
}

/// Tudo sobre um artefato: dados normalizados, como foi lido e de onde veio.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ArtefatoDetalhe {
    pub resumo: ArtefatoResumo,
    pub data_hora_original: Option<String>,
    pub formato_data: Option<String>,
    pub id_externo: Option<String>,
    /// Caminho do anexo como o app gravou.
    pub anexo_caminho: Option<String>,
    pub anexo_arquivo: Option<AnexoInventario>,
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
            &format!(
                "SELECT {COLUNAS_RESUMO},
                    a.data_hora_original, a.formato_data, a.id_externo, a.anexo_caminho, a.confianca,
                    a.metadados, a.hash_registro, x.id, x.importador, x.importador_versao, x.origem_caminho,
                    x.hash_manifesto, e.parser, e.parser_versao,
                    f.id, f.caminho, f.sha256, f.tamanho, f.categoria
                 FROM artefato a
                 JOIN extracao x ON x.id = a.extracao_id
                 JOIN execucao_parser e ON e.id = a.execucao_id
                 LEFT JOIN arquivo f ON f.id = a.anexo_arquivo_id
                 WHERE a.id = ?1"
            ),
            [artefato_id],
            |r| {
                let n = N_RESUMO;
                let anexo_id: Option<i64> = r.get(n + 14)?;
                Ok(ArtefatoDetalhe {
                    resumo: ler_resumo(r)?,
                    data_hora_original: r.get(n)?,
                    formato_data: r.get(n + 1)?,
                    id_externo: r.get(n + 2)?,
                    anexo_caminho: r.get(n + 3)?,
                    anexo_arquivo: match anexo_id {
                        Some(id) => Some(AnexoInventario {
                            id,
                            caminho: r.get(n + 15)?,
                            sha256: r.get(n + 16)?,
                            tamanho: r.get(n + 17)?,
                            categoria: r.get(n + 18)?,
                        }),
                        None => None,
                    },
                    confianca: r.get(n + 4)?,
                    metadados: json_ou_nulo(r.get(n + 5)?),
                    hash_registro: r.get(n + 6)?,
                    extracao_id: uuid(r.get(n + 7)?)?,
                    importador: r.get(n + 8)?,
                    importador_versao: r.get(n + 9)?,
                    origem_caminho: r.get(n + 10)?,
                    hash_manifesto: r.get(n + 11)?,
                    parser: r.get(n + 12)?,
                    parser_versao: r.get(n + 13)?,
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

// ---------- Conferência do hash_registro ----------

/// Resultado de recalcular o `hash_registro` de um artefato a partir do banco do caso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConferenciaRegistro {
    pub artefato_id: i64,
    /// Gravado na importação (coluna `hash_registro`).
    pub gravado: String,
    /// Recalculado agora do conteúdo gravado; `Err` traz o motivo quando o conteúdo nem
    /// pôde ser remontado (tipo de coluna inesperado, JSON inválido, valor desconhecido),
    /// o que por si só indica alteração fora do SIED.
    pub recalculado: Result<String, String>,
}

impl ConferenciaRegistro {
    pub fn confere(&self) -> bool {
        matches!(&self.recalculado, Ok(h) if *h == self.gravado)
    }
}

type Remontado<T> = std::result::Result<T, String>;

fn texto(v: &rusqlite::types::Value, coluna: &str) -> Remontado<Option<String>> {
    use rusqlite::types::Value as V;
    match v {
        V::Null => Ok(None),
        V::Text(t) => Ok(Some(t.clone())),
        _ => Err(format!("coluna {coluna} com tipo inesperado")),
    }
}

fn inteiro(v: &rusqlite::types::Value, coluna: &str) -> Remontado<Option<i64>> {
    use rusqlite::types::Value as V;
    match v {
        V::Null => Ok(None),
        V::Integer(i) => Ok(Some(*i)),
        _ => Err(format!("coluna {coluna} com tipo inesperado")),
    }
}

fn real(v: &rusqlite::types::Value, coluna: &str) -> Remontado<Option<f64>> {
    use rusqlite::types::Value as V;
    match v {
        V::Null => Ok(None),
        V::Real(f) => Ok(Some(*f)),
        // Coluna REAL: o SQLite guarda valores inteiros (ex.: 10.0) como inteiros.
        V::Integer(i) => Ok(Some(*i as f64)),
        _ => Err(format!("coluna {coluna} com tipo inesperado")),
    }
}

fn exigido<T>(v: Remontado<Option<T>>, coluna: &str) -> Remontado<T> {
    v?.ok_or_else(|| format!("coluna {coluna} vazia"))
}

fn enumerado<T: serde::de::DeserializeOwned>(texto: String, coluna: &str) -> Remontado<T> {
    serde_json::from_value(Value::String(texto))
        .map_err(|_| format!("valor desconhecido na coluna {coluna}"))
}

fn json<T: serde::de::DeserializeOwned>(texto: String, coluna: &str) -> Remontado<T> {
    serde_json::from_str(&texto).map_err(|e| format!("JSON inválido na coluna {coluna}: {e}"))
}

/// Remonta o [`NovoArtefato`](sied_dominio::artefato::NovoArtefato) exatamente como o
/// parser o emitiu: colunas de `artefato` mais as referências de origem (a principal
/// primeiro, depois as adicionais na ordem em que foram gravadas).
fn remontar(
    linha: &[rusqlite::types::Value],
    refs: &[Vec<rusqlite::types::Value>],
) -> Remontado<sied_dominio::artefato::NovoArtefato> {
    use sied_dominio::artefato::{DataHora, NovoArtefato, ReferenciaRegistro};
    let c = |i: usize| &linha[i];
    let mut principal = None;
    let mut adicionais = Vec::new();
    for r in refs {
        let e_principal = match exigido(inteiro(&r[0], "principal"), "principal")? {
            0 => false,
            1 => true,
            _ => return Err("coluna principal com valor inesperado".into()),
        };
        let referencia = ReferenciaRegistro {
            papel_arquivo: exigido(texto(&r[1], "papel_arquivo"), "papel_arquivo")?,
            tabela: texto(&r[2], "tabela")?,
            id_registro: texto(&r[3], "id_registro")?,
            consulta: texto(&r[4], "consulta")?,
            deslocamento: inteiro(&r[5], "deslocamento")?.map(|d| d as u64),
            valores_brutos: json(
                exigido(texto(&r[6], "valores_brutos"), "valores_brutos")?,
                "valores_brutos",
            )?,
        };
        if e_principal {
            if principal.replace(referencia).is_some() {
                return Err("mais de uma referência principal".into());
            }
        } else {
            adicionais.push(referencia);
        }
    }
    let origem = principal.ok_or("sem referência de origem principal")?;
    let tipo = enumerado(exigido(texto(c(0), "tipo"), "tipo")?, "tipo")?;
    let mut a = NovoArtefato::novo(tipo, origem);
    a.origens_adicionais = adicionais;
    a.subtipo = texto(c(1), "subtipo")?;
    a.app = texto(c(2), "app")?;
    a.conta = texto(c(3), "conta")?;
    a.remetente = texto(c(4), "remetente")?;
    a.destinatarios = json(
        exigido(texto(c(5), "destinatarios"), "destinatarios")?,
        "destinatarios",
    )?;
    a.de_mim = match inteiro(c(6), "de_mim")? {
        None => None,
        Some(0) => Some(false),
        Some(1) => Some(true),
        Some(_) => return Err("coluna de_mim com valor inesperado".into()),
    };
    // Sem data: as três colunas vazias. Com data: formato e valor original presentes.
    a.data_hora = match texto(c(9), "formato_data")? {
        None => {
            if inteiro(c(7), "data_hora_ms")?.is_some()
                || texto(c(8), "data_hora_original")?.is_some()
            {
                return Err("data sem formato".into());
            }
            None
        }
        Some(formato) => Some(DataHora {
            utc_ms: inteiro(c(7), "data_hora_ms")?,
            original: exigido(texto(c(8), "data_hora_original"), "data_hora_original")?,
            formato: enumerado(formato, "formato_data")?,
        }),
    };
    a.conteudo = texto(c(10), "conteudo")?;
    a.id_externo = texto(c(11), "id_externo")?;
    a.resposta_a_id_externo = texto(c(12), "resposta_a_id_externo")?;
    a.conversa_id_externo = texto(c(13), "conversa_id_externo")?;
    a.anexo_caminho = texto(c(14), "anexo_caminho")?;
    a.mime = texto(c(15), "mime")?;
    a.nome_arquivo = texto(c(16), "nome_arquivo")?;
    a.tamanho = inteiro(c(17), "tamanho")?.map(|t| t as u64);
    a.latitude = real(c(18), "latitude")?;
    a.longitude = real(c(19), "longitude")?;
    a.estado = enumerado(
        exigido(texto(c(20), "estado_registro"), "estado_registro")?,
        "estado_registro",
    )?;
    a.confianca = enumerado(
        exigido(texto(c(21), "confianca"), "confianca")?,
        "confianca",
    )?;
    a.metadados = json(
        exigido(texto(c(22), "metadados"), "metadados")?,
        "metadados",
    )?;
    Ok(a)
}

/// Hash do artefato remontado. Registros gravados antes do pipeline normalizar -0.0
/// (coordenada exatamente zero) podem ter entrado no hash com o sinal negativo, que a
/// coluna REAL não guarda: as variantes com sinal também são aceitas.
fn hash_remontado(mut a: sied_dominio::artefato::NovoArtefato, gravado: &str) -> String {
    let h = a.hash_registro();
    if h == gravado {
        return h;
    }
    let (lat, lon) = (a.latitude, a.longitude);
    // Some(-0.0) == Some(0.0): os dois sinais do zero.
    let sinais = |v: Option<f64>| -> Vec<Option<f64>> {
        if v == Some(0.0) {
            vec![Some(0.0), Some(-0.0)]
        } else {
            vec![v]
        }
    };
    for la in sinais(lat) {
        for lo in sinais(lon) {
            a.latitude = la;
            a.longitude = lo;
            if a.hash_registro() == gravado {
                return gravado.to_string();
            }
        }
    }
    h
}

/// Recalcula o `hash_registro` de um artefato a partir do que está gravado no banco do
/// caso e o compara com o hash gravado na importação.
pub fn conferir_registro(conn: &Connection, artefato_id: i64) -> Resultado<ConferenciaRegistro> {
    let (linha, gravado): (Vec<rusqlite::types::Value>, String) = conn
        .query_row(
            "SELECT tipo, subtipo, app, conta, remetente, destinatarios, de_mim, data_hora_ms,
                    data_hora_original, formato_data, conteudo, id_externo, resposta_a_id_externo,
                    conversa_id_externo, anexo_caminho, mime, nome_arquivo, tamanho, latitude,
                    longitude, estado_registro, confianca, metadados, hash_registro
             FROM artefato WHERE id = ?1",
            [artefato_id],
            |r| {
                let v = (0..23).map(|i| r.get(i)).collect::<Result<Vec<_>, _>>()?;
                // O hash gravado é lido como valor bruto: adulterado para outro tipo, não confere.
                let h: rusqlite::types::Value = r.get(23)?;
                let h = match h {
                    rusqlite::types::Value::Text(t) => t,
                    _ => String::new(),
                };
                Ok((v, h))
            },
        )
        .optional()?
        .ok_or_else(|| ErroArmazenamento::NaoEncontrado("artefato".into()))?;
    let mut stmt = conn.prepare_cached(
        "SELECT principal, papel_arquivo, tabela, id_registro, consulta, deslocamento, valores_brutos
         FROM referencia_origem WHERE artefato_id = ?1 ORDER BY principal DESC, id",
    )?;
    let refs = stmt
        .query_map([artefato_id], |r| {
            (0..7).map(|i| r.get(i)).collect::<Result<Vec<_>, _>>()
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let recalculado = remontar(&linha, &refs).map(|a| hash_remontado(a, &gravado));
    Ok(ConferenciaRegistro {
        artefato_id,
        gravado,
        recalculado,
    })
}

/// Confere todos os registros do caso (ou de uma extração) e devolve os que não
/// conferem. Percorre registro a registro: use em verificações completas, não na tela.
pub fn registros_que_nao_conferem(
    conn: &Connection,
    extracao: Option<ExtracaoId>,
) -> Resultado<Vec<ConferenciaRegistro>> {
    let ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT id FROM artefato WHERE ?1 IS NULL OR extracao_id = ?1 ORDER BY id")?;
        let v = stmt
            .query_map([extracao.map(|x| x.to_string())], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        v
    };
    let mut ruins = Vec::new();
    for id in ids {
        let c = conferir_registro(conn, id)?;
        if !c.confere() {
            ruins.push(c);
        }
    }
    Ok(ruins)
}

#[cfg(test)]
mod testes {
    use super::*;
    use sied_dominio::artefato::{NovoArtefato, ReferenciaRegistro, TipoArtefato};

    #[test]
    fn aceita_zero_negativo_gravado_antes_da_normalizacao() {
        let mut a = NovoArtefato::novo(
            TipoArtefato::Localizacao,
            ReferenciaRegistro {
                papel_arquivo: "msgstore".into(),
                tabela: Some("message_location".into()),
                id_registro: Some("_id=1".into()),
                consulta: None,
                deslocamento: None,
                valores_brutos: Default::default(),
            },
        );
        a.latitude = Some(-0.0);
        a.longitude = Some(-46.6);
        let antigo = a.hash_registro();
        // A coluna REAL devolve 0.0: o hash antigo (com -0.0) ainda é reconhecido.
        a.latitude = Some(0.0);
        assert_eq!(hash_remontado(a.clone(), &antigo), antigo);
        // Qualquer outra diferença continua divergindo.
        a.longitude = Some(-46.5);
        assert_ne!(hash_remontado(a, &antigo), antigo);
    }
}
