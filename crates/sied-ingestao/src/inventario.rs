//! Inventário: hash SHA-256 e classificação de cada arquivo da extração.
//!
//! A classificação usa a assinatura (primeiros bytes do conteúdo), não só a extensão:
//! um `msgstore.db` renomeado para `.jpg` continua sendo reconhecido como banco SQLite.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use ts_rs::TS;

use crate::fonte::{CaminhoFonte, EntradaFonte};
use crate::{ErroIngestao, Resultado};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Categoria {
    BancoSqlite,
    Imagem,
    Video,
    Audio,
    Documento,
    Compactado,
    Texto,
    Vazio,
    Outro,
}

impl Categoria {
    pub fn como_str(self) -> &'static str {
        match self {
            Categoria::BancoSqlite => "banco_sqlite",
            Categoria::Imagem => "imagem",
            Categoria::Video => "video",
            Categoria::Audio => "audio",
            Categoria::Documento => "documento",
            Categoria::Compactado => "compactado",
            Categoria::Texto => "texto",
            Categoria::Vazio => "vazio",
            Categoria::Outro => "outro",
        }
    }

    pub fn de_str(s: &str) -> Option<Self> {
        Some(match s {
            "banco_sqlite" => Categoria::BancoSqlite,
            "imagem" => Categoria::Imagem,
            "video" => Categoria::Video,
            "audio" => Categoria::Audio,
            "documento" => Categoria::Documento,
            "compactado" => Categoria::Compactado,
            "texto" => Categoria::Texto,
            "vazio" => Categoria::Vazio,
            "outro" => Categoria::Outro,
            _ => return None,
        })
    }
}

/// Tipo identificado: pela assinatura quando possível, senão pela extensão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classificacao {
    pub categoria: Categoria,
    /// Nome da assinatura reconhecida (ex.: "sqlite3", "jpeg"); `None` se só pela extensão.
    pub assinatura: Option<&'static str>,
    pub mime: Option<&'static str>,
}

fn por_assinatura(c: &[u8]) -> Option<(Categoria, &'static str, &'static str)> {
    use Categoria::*;
    let comeca = |m: &[u8]| c.starts_with(m);
    Some(if comeca(b"SQLite format 3\0") {
        (BancoSqlite, "sqlite3", "application/vnd.sqlite3")
    } else if comeca(&[0xFF, 0xD8, 0xFF]) {
        (Imagem, "jpeg", "image/jpeg")
    } else if comeca(b"\x89PNG\r\n\x1a\n") {
        (Imagem, "png", "image/png")
    } else if comeca(b"GIF87a") || comeca(b"GIF89a") {
        (Imagem, "gif", "image/gif")
    } else if comeca(b"RIFF") && c.get(8..12) == Some(b"WEBP") {
        (Imagem, "webp", "image/webp")
    } else if comeca(b"RIFF") && c.get(8..12) == Some(b"WAVE") {
        (Audio, "wav", "audio/wav")
    } else if comeca(b"BM") && c.len() > 14 && c.get(6..10) == Some(&[0, 0, 0, 0]) {
        (Imagem, "bmp", "image/bmp")
    } else if c.get(4..8) == Some(b"ftyp") {
        match c.get(8..12) {
            Some(b"heic") | Some(b"heix") | Some(b"mif1") | Some(b"msf1") => {
                (Imagem, "heic", "image/heic")
            }
            Some(b"M4A ") => (Audio, "m4a", "audio/mp4"),
            Some(m) if m.starts_with(b"3g") => (Video, "3gp", "video/3gpp"),
            Some(b"qt  ") => (Video, "quicktime", "video/quicktime"),
            _ => (Video, "mp4", "video/mp4"),
        }
    } else if comeca(b"OggS") {
        (Audio, "ogg", "audio/ogg")
    } else if comeca(b"ID3") || comeca(&[0xFF, 0xFB]) {
        (Audio, "mp3", "audio/mpeg")
    } else if comeca(b"#!AMR") {
        (Audio, "amr", "audio/amr")
    } else if comeca(b"%PDF-") {
        (Documento, "pdf", "application/pdf")
    } else if comeca(b"PK\x03\x04") || comeca(b"PK\x05\x06") {
        (Compactado, "zip", "application/zip")
    } else if comeca(b"ANDROID BACKUP") {
        (Compactado, "android_backup", "application/octet-stream")
    } else if comeca(&[0x1F, 0x8B]) {
        (Compactado, "gzip", "application/gzip")
    } else if c.get(257..262) == Some(b"ustar") {
        (Compactado, "tar", "application/x-tar")
    } else if comeca(b"bplist00") {
        (Documento, "bplist", "application/x-bplist")
    } else if comeca(b"<?xml") {
        (Texto, "xml", "application/xml")
    } else {
        return None;
    })
}

fn por_extensao(ext: &str) -> Categoria {
    use Categoria::*;
    match ext {
        "db" | "sqlite" | "sqlite3" => BancoSqlite,
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "bmp" => Imagem,
        "mp4" | "3gp" | "mov" | "mkv" | "avi" => Video,
        "opus" | "ogg" | "mp3" | "m4a" | "aac" | "amr" | "wav" => Audio,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "plist" => Documento,
        "zip" | "tar" | "gz" | "ab" | "7z" | "rar" => Compactado,
        "txt" | "csv" | "json" | "xml" | "log" | "html" | "htm" | "prop" => Texto,
        _ => Outro,
    }
}

/// Classifica pelo cabeçalho; sem assinatura conhecida, usa a extensão.
pub fn classificar(caminho: &CaminhoFonte, tamanho: u64, cabecalho: &[u8]) -> Classificacao {
    if tamanho == 0 {
        return Classificacao {
            categoria: Categoria::Vazio,
            assinatura: None,
            mime: None,
        };
    }
    if let Some((categoria, assinatura, mime)) = por_assinatura(cabecalho) {
        return Classificacao {
            categoria,
            assinatura: Some(assinatura),
            mime: Some(mime),
        };
    }
    Classificacao {
        categoria: caminho
            .extensao()
            .map(|e| por_extensao(&e))
            .unwrap_or(Categoria::Outro),
        assinatura: None,
        mime: None,
    }
}

/// Resultado da leitura integral de um arquivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leitura {
    pub sha256: String,
    pub bytes: u64,
    pub cabecalho: Vec<u8>,
}

/// Lê o arquivo inteiro calculando o SHA-256. Se `copia` for informado, grava os
/// mesmos bytes nele (cópia de trabalho). `a_cada_bloco` recebe os bytes lidos e
/// devolve `false` para cancelar.
pub fn ler_com_hash(
    leitor: &mut dyn Read,
    mut copia: Option<&mut dyn Write>,
    mut a_cada_bloco: impl FnMut(u64) -> bool,
) -> Resultado<Leitura> {
    // Bytes iniciais guardados para a assinatura; 262 alcança a marca "ustar" do tar.
    const CABECALHO_TAR: usize = 262;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut cabecalho = Vec::with_capacity(CABECALHO_TAR);
    let mut bytes = 0u64;
    loop {
        let n = match leitor.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        };
        let bloco = &buf[..n];
        hasher.update(bloco);
        if let Some(c) = copia.as_mut() {
            c.write_all(bloco)?;
        }
        if cabecalho.len() < CABECALHO_TAR {
            let falta = CABECALHO_TAR - cabecalho.len();
            cabecalho.extend_from_slice(&bloco[..falta.min(n)]);
        }
        bytes += n as u64;
        if !a_cada_bloco(n as u64) {
            return Err(ErroIngestao::Cancelado);
        }
    }
    if let Some(c) = copia.as_mut() {
        c.flush()?;
    }
    Ok(Leitura {
        sha256: hex::encode(hasher.finalize()),
        bytes,
        cabecalho,
    })
}

/// Calcula hashes em paralelo de um lote limitado de arquivos da fonte.
///
/// O lote é deliberadamente fornecido pelo pipeline em janelas pequenas: não há
/// carregamento do caso inteiro na memória. Os workers só recebem caminhos da fonte
/// e devolvem hashes/cabeçalhos; nenhum conteúdo é compartilhado entre threads e a
/// fonte continua somente leitura. `workers` é limitado pelo chamador e o cancelamento
/// é cooperativo entre blocos.
pub fn ler_varios_com_hash(
    fonte: &dyn crate::fonte::FonteSomenteLeitura,
    entradas: Vec<EntradaFonte>,
    workers: usize,
    cancelar: &AtomicBool,
) -> Vec<(EntradaFonte, Resultado<Leitura>)> {
    if entradas.is_empty() {
        return Vec::new();
    }
    let workers = workers.clamp(1, 8).min(entradas.len());
    let (tx_jobs, rx_jobs) = mpsc::sync_channel::<EntradaFonte>(workers * 2);
    let (tx_resultados, rx_resultados) = mpsc::channel::<(EntradaFonte, Resultado<Leitura>)>();
    let mut resultados = Vec::with_capacity(entradas.len());
    let rx_jobs = Arc::new(Mutex::new(rx_jobs));

    thread::scope(|escopo| {
        for _ in 0..workers {
            let rx_jobs = Arc::clone(&rx_jobs);
            let tx_resultados = tx_resultados.clone();
            escopo.spawn(move || loop {
                let entrada = match rx_jobs.lock().ok().and_then(|rx| rx.recv().ok()) {
                    Some(entrada) => entrada,
                    None => break,
                };
                let resultado = if cancelar.load(Ordering::SeqCst) {
                    Err(ErroIngestao::Cancelado)
                } else {
                    fonte.abrir(&entrada.caminho).and_then(|mut leitor| {
                        ler_com_hash(&mut leitor, None, |_| !cancelar.load(Ordering::SeqCst))
                    })
                };
                if tx_resultados.send((entrada, resultado)).is_err() {
                    break;
                }
            });
        }
        drop(tx_resultados);
        for entrada in entradas {
            if cancelar.load(Ordering::SeqCst) {
                break;
            }
            if tx_jobs.send(entrada).is_err() {
                break;
            }
        }
        drop(tx_jobs);
        // Todos os workers pertencem ao escopo; a coleta bloqueia somente até que
        // cada item enviado tenha recebido um resultado.
        while let Ok(item) = rx_resultados.recv() {
            resultados.push(item);
        }
    });
    resultados
}

/// Um arquivo inventariado, pronto para gravar.
#[derive(Debug, Clone, PartialEq)]
pub struct ArquivoInventariado {
    pub entrada: EntradaFonte,
    /// `None` quando o arquivo não pôde ser lido (ver `erro`).
    pub sha256: Option<String>,
    pub categoria: Categoria,
    pub assinatura: Option<&'static str>,
    pub mime: Option<&'static str>,
    pub erro: Option<String>,
}

/// Linha do manifesto no formato do `sha256sum` (modo binário), verificável com
/// ferramentas externas: `sha256sum -c manifesto.sha256` a partir da raiz da extração.
///
/// Nomes com `\\` ou quebra de linha usam o escape do próprio `sha256sum` (linha
/// iniciada por `\\`).
pub fn linha_manifesto(sha256: &str, caminho: &CaminhoFonte) -> String {
    let c = caminho.como_str();
    if c.contains(['\\', '\n', '\r']) {
        let esc = c
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\r', "\\r");
        format!("\\{sha256} *{esc}\n")
    } else {
        format!("{sha256} *{c}\n")
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fonte::{CaminhoFonte, EntradaFonte, FonteSomenteLeitura};
    use std::io::Cursor;

    fn c(s: &str) -> CaminhoFonte {
        CaminhoFonte::novo(s).unwrap()
    }

    #[test]
    fn assinatura_prevalece_sobre_extensao() {
        let k = classificar(&c("foto.jpg"), 100, b"SQLite format 3\0resto");
        assert_eq!(k.categoria, Categoria::BancoSqlite);
        assert_eq!(k.assinatura, Some("sqlite3"));
        let k = classificar(&c("sem_ext"), 10, &[0xFF, 0xD8, 0xFF, 0xE0]);
        assert_eq!(k.categoria, Categoria::Imagem);
        let k = classificar(&c("audio.opus"), 10, b"xxxx");
        assert_eq!((k.categoria, k.assinatura), (Categoria::Audio, None));
        let k = classificar(&c("v.mp4"), 10, b"\0\0\0\x18ftypmp42");
        assert_eq!(k.categoria, Categoria::Video);
        assert_eq!(classificar(&c("x.db"), 0, b"").categoria, Categoria::Vazio);
    }

    #[test]
    fn hash_conhecido_e_copia_identica() {
        let dados = b"abc".repeat(1_000_000); // atravessa vários blocos
        let mut copia = Vec::new();
        let mut blocos = 0;
        let l = ler_com_hash(&mut &dados[..], Some(&mut copia), |_| {
            blocos += 1;
            true
        })
        .unwrap();
        assert_eq!(l.bytes, 3_000_000);
        assert_eq!(copia, dados);
        assert!(blocos >= 3);
        assert_eq!(l.cabecalho.len(), 262);
        let l = ler_com_hash(&mut &b"abc"[..], None, |_| true).unwrap();
        assert_eq!(
            l.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn cancela_no_meio() {
        let dados = vec![0u8; 3 << 20];
        let r = ler_com_hash(&mut &dados[..], None, |_| false);
        assert!(matches!(r, Err(ErroIngestao::Cancelado)));
    }

    #[test]
    fn manifesto_no_formato_sha256sum() {
        assert_eq!(linha_manifesto("ab", &c("a b/c.db")), "ab *a b/c.db\n");
        assert_eq!(linha_manifesto("ab", &c("a/x\ny")), "\\ab *a/x\\ny\n");
    }

    struct FonteLote;

    impl FonteSomenteLeitura for FonteLote {
        fn descricao(&self) -> String {
            "FIXTURE E12".into()
        }
        fn listar(&self) -> Box<dyn Iterator<Item = Resultado<EntradaFonte>> + '_> {
            Box::new((0..4).map(|n| {
                Ok(EntradaFonte {
                    caminho: CaminhoFonte::novo(&format!("lote/{n}.bin")).unwrap(),
                    diretorio: false,
                    tamanho: 4,
                    modificado_em: None,
                    criado_em: None,
                })
            }))
        }
        fn entrada(&self, caminho: &CaminhoFonte) -> Resultado<Option<EntradaFonte>> {
            Ok(Some(EntradaFonte {
                caminho: caminho.clone(),
                diretorio: false,
                tamanho: 4,
                modificado_em: None,
                criado_em: None,
            }))
        }
        fn abrir(&self, caminho: &CaminhoFonte) -> Resultado<Box<dyn crate::fonte::LeitorFonte>> {
            Ok(Box::new(Cursor::new(caminho.nome().as_bytes().to_vec())))
        }
    }

    #[test]
    fn lote_hash_paralelo_preserva_todos_os_itens() {
        let fonte = FonteLote;
        let entradas: Vec<_> = fonte.listar().map(Result::unwrap).collect();
        let cancelar = AtomicBool::new(false);
        let resultados = ler_varios_com_hash(&fonte, entradas, 4, &cancelar);
        assert_eq!(resultados.len(), 4);
        assert!(resultados.iter().all(|(_, r)| r.is_ok()));
    }

    #[test]
    fn lote_hash_paralelo_respeita_cancelamento() {
        let fonte = FonteLote;
        let entradas: Vec<_> = fonte.listar().map(Result::unwrap).collect();
        let cancelar = AtomicBool::new(true);
        let resultados = ler_varios_com_hash(&fonte, entradas, 4, &cancelar);
        assert!(resultados.is_empty());
    }
}
