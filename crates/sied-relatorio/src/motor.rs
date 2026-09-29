//! Typst embutido: compila o modelo do relatório com os dados em JSON e as imagens
//! em memória. Não lê nada do disco nem da rede: os únicos "arquivos" que o modelo
//! enxerga são `/modelo.typ`, `/dados.json` e as imagens `/img/<n>.jpg`.

use std::collections::HashMap;
use std::sync::OnceLock;

use chrono::{DateTime, Datelike, Timelike, Utc};
use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration, Smart};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;

pub const MODELO: &str = include_str!("modelo.typ");

fn fontes() -> &'static FontStore {
    static F: OnceLock<FontStore> = OnceLock::new();
    F.get_or_init(|| {
        let mut f = FontStore::new();
        f.extend(typst_kit::fonts::embedded());
        f
    })
}

fn biblioteca() -> &'static LazyHash<Library> {
    static B: OnceLock<LazyHash<Library>> = OnceLock::new();
    B.get_or_init(|| LazyHash::new(Library::default()))
}

fn id(caminho: &str) -> FileId {
    FileId::new(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(caminho).expect("caminho virtual válido"),
    ))
}

struct Mundo {
    principal: FileId,
    fonte: Source,
    arquivos: HashMap<FileId, Bytes>,
    agora: DateTime<Utc>,
}

impl World for Mundo {
    fn library(&self) -> &LazyHash<Library> {
        biblioteca()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        fontes().book()
    }

    fn main(&self) -> FileId {
        self.principal
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.principal {
            Ok(self.fonte.clone())
        } else {
            Err(FileError::NotSource)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.principal {
            return Ok(Bytes::from_string(self.fonte.clone()));
        }
        self.arquivos
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.get().vpath().get_with_slash().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        fontes().font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let d = match offset {
            Some(o) => self.agora + chrono::Duration::seconds(o.seconds() as i64),
            None => self.agora,
        };
        Datetime::from_ymd(d.year(), d.month() as u8, d.day() as u8)
    }
}

/// Falha ao diagramar o relatório (erro no modelo ou nos dados).
#[derive(Debug, thiserror::Error)]
#[error("falha ao diagramar o relatório: {0}")]
pub struct ErroTypst(pub String);

fn texto_erros(erros: &[SourceDiagnostic]) -> String {
    erros
        .iter()
        .map(|e| e.message.to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Compila o modelo com os dados e as imagens (`nome` → JPEG) e devolve as páginas.
pub fn paginar(
    dados_json: &str,
    imagens: &HashMap<String, Vec<u8>>,
    agora: DateTime<Utc>,
) -> Result<PagedDocument, ErroTypst> {
    let principal = id("/modelo.typ");
    let mut arquivos = HashMap::new();
    arquivos.insert(
        id("/dados.json"),
        Bytes::new(dados_json.as_bytes().to_vec()),
    );
    for (nome, bytes) in imagens {
        arquivos.insert(id(&format!("/img/{nome}")), Bytes::new(bytes.clone()));
    }
    let mundo = Mundo {
        principal,
        fonte: Source::new(principal, MODELO.to_string()),
        arquivos,
        agora,
    };
    let resultado = typst::compile::<PagedDocument>(&mundo);
    for aviso in &resultado.warnings {
        tracing::debug!(aviso = %aviso.message, "aviso do Typst no relatório");
    }
    resultado
        .output
        .map_err(|erros| ErroTypst(texto_erros(&erros)))
}

/// Gera o PDF. `ident` identifica o relatório de forma estável (vai, em hash, para o
/// identificador do PDF).
pub fn pdf(
    dados_json: &str,
    imagens: &HashMap<String, Vec<u8>>,
    agora: DateTime<Utc>,
    ident: &str,
) -> Result<Vec<u8>, ErroTypst> {
    let doc = paginar(dados_json, imagens, agora)?;
    let data = Datetime::from_ymd_hms(
        agora.year(),
        agora.month() as u8,
        agora.day() as u8,
        agora.hour() as u8,
        agora.minute() as u8,
        agora.second() as u8,
    )
    .ok_or_else(|| ErroTypst("data de geração inválida".into()))?;
    let opcoes = typst_pdf::PdfOptions {
        ident: Smart::Custom(ident.to_string()),
        creator: Smart::Custom(Some(format!("SIED {}", env!("CARGO_PKG_VERSION")))),
        timestamp: Some(typst_pdf::Timestamp::new_utc(data)),
        ..Default::default()
    };
    typst_pdf::pdf(&doc, &opcoes).map_err(|erros| ErroTypst(texto_erros(&erros)))
}
