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

/// Se alguma das fontes embutidas desenha o caractere.
fn desenhavel(c: char) -> bool {
    let livro = fontes().book();
    (0..)
        .map_while(|i| livro.info(i))
        .any(|info| info.coverage.contains(c as u32))
}

/// Caracteres invisíveis que mudam a leitura (largura zero, direção, junção, seletores
/// de variação, marcas de ordem): no relatório eles aparecem pelo código.
fn invisivel(c: char) -> bool {
    matches!(c,
        '\u{200B}'..='\u{200F}'
        | '\u{202A}'..='\u{202E}'
        | '\u{2060}'..='\u{2064}'
        | '\u{2066}'..='\u{2069}'
        | '\u{FE00}'..='\u{FE0F}'
        | '\u{FEFF}'
        | '\u{180E}'
        | '\u{FFF9}'..='\u{FFFB}'
        | '\u{E0000}'..='\u{E007F}'
        | '\u{E0100}'..='\u{E01EF}')
}

/// Troca o que as fontes embutidas não desenham (emojis, alfabetos sem fonte) e os
/// caracteres de controle ou invisíveis por `[U+XXXX ...]`, em sequência e na ordem
/// original. Sem isso, o PDF mostraria quadrados vazios (ou nada) no lugar deles, e
/// um "❤️" ficaria igual a um "🔫". O valor exato continua nos valores brutos.
pub fn texto_imprimivel(t: &str) -> std::borrow::Cow<'_, str> {
    let fora = |c: char| {
        !matches!(c, '\n' | '\t' | '\r' | ' ') && (c.is_control() || invisivel(c) || !desenhavel(c))
    };
    if !t.chars().any(fora) {
        return std::borrow::Cow::Borrowed(t);
    }
    let mut saida = String::with_capacity(t.len() + 16);
    let mut grupo: Vec<String> = Vec::new();
    for c in t.chars() {
        // Seletor de estilo (texto/emoji) depois de um caractere desenhado não muda o
        // sentido: fica só nos valores brutos.
        if matches!(c, '\u{FE0E}' | '\u{FE0F}') && grupo.is_empty() && !saida.is_empty() {
            continue;
        }
        if fora(c) {
            grupo.push(format!("U+{:04X}", c as u32));
        } else {
            if !grupo.is_empty() {
                saida.push_str(&format!("[{}]", grupo.join(" ")));
                grupo.clear();
            }
            saida.push(c);
        }
    }
    if !grupo.is_empty() {
        saida.push_str(&format!("[{}]", grupo.join(" ")));
    }
    std::borrow::Cow::Owned(saida)
}

fn imprimivel_json(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::String(s) => {
            if let std::borrow::Cow::Owned(novo) = texto_imprimivel(s) {
                *s = novo;
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(imprimivel_json),
        serde_json::Value::Object(o) => o.values_mut().for_each(imprimivel_json),
        _ => {}
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
    let mut dados: serde_json::Value = serde_json::from_str(dados_json)
        .map_err(|e| ErroTypst(format!("dados do relatório inválidos: {e}")))?;
    imprimivel_json(&mut dados);
    let mut arquivos = HashMap::new();
    arquivos.insert(
        id("/dados.json"),
        Bytes::new(serde_json::to_vec(&dados).expect("JSON serializável")),
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
    // Um defeito do Typst diante de algum texto não pode derrubar o SIED: vira erro.
    let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        typst::compile::<PagedDocument>(&mundo)
    }))
    .map_err(|_| ErroTypst("o motor de diagramação falhou de forma inesperada".into()))?;
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
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        typst_pdf::pdf(&doc, &opcoes)
    }))
    .map_err(|_| ErroTypst("o gerador de PDF falhou de forma inesperada".into()))?
    .map_err(|erros| ErroTypst(texto_erros(&erros)))
}

#[cfg(test)]
mod testes {
    use super::texto_imprimivel;

    #[test]
    fn caracteres_sem_desenho_viram_codigo() {
        assert_eq!(
            texto_imprimivel("Olá, ação ü ñ ç — “aspas”"),
            "Olá, ação ü ñ ç — “aspas”"
        );
        assert_eq!(
            texto_imprimivel("oi 👩‍👧 tchau"),
            "oi [U+1F469 U+200D U+1F467] tchau"
        );
        // O coração tem desenho (DejaVu); o seletor de emoji depois dele é omitido.
        assert_eq!(texto_imprimivel("❤️ ok"), "❤ ok");
        assert_eq!(texto_imprimivel("🔫"), "[U+1F52B]");
        assert_eq!(texto_imprimivel("a\u{0}b\u{7}c"), "a[U+0000]b[U+0007]c");
        assert_eq!(texto_imprimivel("gpj.\u{202E}exe"), "gpj.[U+202E]exe");
        assert_eq!(
            texto_imprimivel("linha 1\nlinha 2\tfim"),
            "linha 1\nlinha 2\tfim"
        );
    }
}
