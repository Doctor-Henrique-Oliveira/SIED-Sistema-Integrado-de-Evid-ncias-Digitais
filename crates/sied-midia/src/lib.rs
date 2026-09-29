//! Miniaturas das imagens inventariadas numa extração.
//!
//! - A imagem é lida da pasta da extração só por [`PastaSomenteLeitura`]: nada é
//!   gravado na evidência.
//! - Os bytes lidos são conferidos com o SHA-256 do inventário **antes** de
//!   decodificar. Se diferirem, nenhuma miniatura é gerada: o arquivo mudou desde a
//!   importação e a tela diz isso.
//! - A miniatura (JPEG, lado maior de [`LADO_MAXIMO`] px) fica em
//!   `<pasta do caso>/miniaturas/<2 primeiros do hash>/<sha256>-v<versão>.jpg`. É dado
//!   derivado: pode ser apagada e é refeita a partir da evidência. Como o nome é o
//!   hash do original, arquivos iguais em extrações diferentes dividem a miniatura.
//! - Vídeos, áudios e formatos sem decodificador (ex.: HEIC) ficam sem miniatura,
//!   com o motivo informado.

use base64::Engine;
use image::{DynamicImage, ImageDecoder, ImageReader, Limits};
use serde::Serialize;
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::fonte::{CaminhoFonte, FonteSomenteLeitura};
use sied_ingestao::inventario::ler_com_hash;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, OnceLock};
use ts_rs::TS;

/// Lado maior da miniatura, em pixels.
pub const LADO_MAXIMO: u32 = 320;
/// Imagens maiores que isto não são lidas para miniatura.
pub const TAMANHO_MAXIMO: u64 = 256 << 20;
/// Muda quando o algoritmo muda; miniaturas de versões anteriores são ignoradas.
pub const VERSAO: u32 = 1;
/// Subpasta da pasta do caso onde as miniaturas ficam.
pub const PASTA_MINIATURAS: &str = "miniaturas";
const QUALIDADE_JPEG: u8 = 82;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EstadoMiniatura {
    /// `dados` traz a imagem.
    Pronta,
    /// Tipo ou formato sem miniatura nesta versão (vídeo, áudio, HEIC...).
    SemSuporte,
    /// A pasta ou o arquivo da extração não estão acessíveis agora.
    OrigemIndisponivel,
    /// O arquivo na pasta da extração não tem o SHA-256 do inventário.
    OrigemDivergente,
    /// Arquivo lido e conferido, mas a imagem não pôde ser decodificada.
    Falhou,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Miniatura {
    pub estado: EstadoMiniatura,
    /// `data:image/jpeg;base64,...` quando pronta.
    pub dados: Option<String>,
    /// Por que não há miniatura, em português, para a tela.
    pub motivo: Option<String>,
}

impl Miniatura {
    fn sem(estado: EstadoMiniatura, motivo: impl Into<String>) -> Self {
        Miniatura {
            estado,
            dados: None,
            motivo: Some(motivo.into()),
        }
    }

    fn pronta(jpeg: &[u8]) -> Self {
        let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg);
        Miniatura {
            estado: EstadoMiniatura::Pronta,
            dados: Some(format!("data:image/jpeg;base64,{b64}")),
            motivo: None,
        }
    }
}

/// O que se sabe do arquivo pelo inventário do caso.
#[derive(Debug, Clone)]
pub struct Pedido {
    /// Pasta da extração, como gravada na importação.
    pub pasta_extracao: PathBuf,
    /// Caminho relativo no inventário.
    pub caminho: String,
    /// SHA-256 do inventário (`None` se o arquivo estava ilegível).
    pub sha256: Option<String>,
    pub tamanho: u64,
    pub categoria: String,
    /// Pasta do caso (a miniatura fica em `<pasta_caso>/miniaturas`).
    pub pasta_caso: PathBuf,
}

/// Caminho da miniatura no cache do caso.
pub fn caminho_cache(pasta_caso: &Path, sha256: &str) -> PathBuf {
    let prefixo = sha256.get(..2).unwrap_or("00");
    pasta_caso
        .join(PASTA_MINIATURAS)
        .join(prefixo)
        .join(format!("{sha256}-v{VERSAO}.jpg"))
}

/// Devolve a miniatura do cache ou a gera a partir da evidência.
pub fn obter(p: &Pedido) -> Miniatura {
    use EstadoMiniatura::*;
    if p.categoria != "imagem" {
        return Miniatura::sem(
            SemSuporte,
            "Miniatura só para imagens; vídeos e áudios não são decodificados nesta versão.",
        );
    }
    let Some(sha) = p.sha256.as_deref() else {
        return Miniatura::sem(
            Falhou,
            "O arquivo não pôde ser lido na importação (sem SHA-256).",
        );
    };
    let cache = caminho_cache(&p.pasta_caso, sha);
    if let Ok(jpeg) = fs::read(&cache) {
        return Miniatura::pronta(&jpeg);
    }
    if p.tamanho > TAMANHO_MAXIMO {
        return Miniatura::sem(
            SemSuporte,
            "Imagem grande demais para miniatura (mais de 256 MB).",
        );
    }

    let _vaga = vaga();
    // Outro pedido pode ter gerado a mesma miniatura enquanto este esperava.
    if let Ok(jpeg) = fs::read(&cache) {
        return Miniatura::pronta(&jpeg);
    }
    let bytes = match ler_conferido(p, sha) {
        Ok(b) => b,
        Err(m) => return m,
    };
    let jpeg = match reduzir(&bytes) {
        Ok(j) => j,
        Err(m) => return m,
    };
    if let Err(e) = gravar_cache(&cache, &jpeg) {
        tracing::warn!(erro = %e, cache = %cache.display(), "miniatura não gravada no cache");
    }
    Miniatura::pronta(&jpeg)
}

/// Lê o arquivo inteiro da extração e confere o SHA-256 com o inventário.
fn ler_conferido(p: &Pedido, sha: &str) -> Result<Vec<u8>, Miniatura> {
    let mut bytes = Vec::with_capacity(p.tamanho.min(TAMANHO_MAXIMO) as usize);
    conferir(p, sha, Some(&mut bytes)).map_err(|mut m| {
        if m.estado == EstadoMiniatura::OrigemDivergente {
            m.motivo = m
                .motivo
                .map(|t| format!("{t} Nenhuma miniatura foi gerada."));
        }
        m
    })?;
    Ok(bytes)
}

/// Relê o arquivo na pasta da extração (somente leitura) e devolve o SHA-256 lido
/// agora, sem guardar os bytes nem limite de tamanho. O relatório compara com o hash
/// registrado na seleção.
pub fn sha256_original(p: &Pedido) -> Result<String, Miniatura> {
    ler(p, None)
}

fn conferir(p: &Pedido, sha: &str, copia: Option<&mut Vec<u8>>) -> Result<(), Miniatura> {
    let lido = ler(p, copia)?;
    if lido != sha {
        tracing::warn!(
            caminho = %p.caminho,
            esperado = sha,
            lido = %lido,
            "arquivo da extração difere do inventário"
        );
        return Err(Miniatura::sem(
            EstadoMiniatura::OrigemDivergente,
            format!(
                "O arquivo na pasta da extração difere do inventário (SHA-256 lido {}…).",
                &lido[..16]
            ),
        ));
    }
    Ok(())
}

fn ler(p: &Pedido, copia: Option<&mut Vec<u8>>) -> Result<String, Miniatura> {
    use EstadoMiniatura::*;
    let fonte = PastaSomenteLeitura::abrir(&p.pasta_extracao).map_err(|_| {
        Miniatura::sem(
            OrigemIndisponivel,
            format!(
                "A pasta da extração não está acessível: {}",
                p.pasta_extracao.display()
            ),
        )
    })?;
    let caminho = CaminhoFonte::novo(&p.caminho)
        .map_err(|e| Miniatura::sem(Falhou, format!("Caminho inválido no inventário: {e}")))?;
    let mut leitor = fonte.abrir(&caminho).map_err(|_| {
        Miniatura::sem(
            OrigemIndisponivel,
            "O arquivo não foi encontrado na pasta da extração.",
        )
    })?;
    let lido = ler_com_hash(
        &mut leitor,
        copia.map(|c| c as &mut dyn std::io::Write),
        |_| true,
    )
    .map_err(|e| Miniatura::sem(OrigemIndisponivel, format!("Falha ao ler o arquivo: {e}")))?;
    Ok(lido.sha256)
}

/// Lado maior da imagem que vai para o relatório (boa leitura impressa sem inflar o PDF).
pub const LADO_RELATORIO: u32 = 1600;

/// Imagem para o relatório: lida da extração agora, com o SHA-256 conferido com o
/// inventário, e reduzida para JPEG (lado maior [`LADO_RELATORIO`]). Não usa nem grava
/// cache: o relatório sempre confere o original no momento em que é gerado.
/// Em caso de falha, devolve o estado e o motivo (como na miniatura).
pub fn imagem_para_relatorio(p: &Pedido) -> Result<Vec<u8>, Miniatura> {
    use EstadoMiniatura::*;
    if p.categoria != "imagem" {
        return Err(Miniatura::sem(
            SemSuporte,
            "Só imagens entram como figura no relatório.",
        ));
    }
    let Some(sha) = p.sha256.as_deref() else {
        return Err(Miniatura::sem(
            Falhou,
            "O arquivo não pôde ser lido na importação (sem SHA-256).",
        ));
    };
    if p.tamanho > TAMANHO_MAXIMO {
        return Err(Miniatura::sem(
            SemSuporte,
            "Imagem grande demais (mais de 256 MB).",
        ));
    }
    let _vaga = vaga();
    let bytes = ler_conferido(p, sha)?;
    reduzir_para(&bytes, LADO_RELATORIO)
}

/// Decodifica, aplica a orientação gravada (EXIF) e reduz para JPEG.
fn reduzir(bytes: &[u8]) -> Result<Vec<u8>, Miniatura> {
    reduzir_para(bytes, LADO_MAXIMO)
}

fn reduzir_para(bytes: &[u8], lado: u32) -> Result<Vec<u8>, Miniatura> {
    use EstadoMiniatura::*;
    let mut leitor = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| Miniatura::sem(Falhou, format!("Falha ao identificar a imagem: {e}")))?;
    if leitor.format().is_none() {
        return Err(Miniatura::sem(
            SemSuporte,
            "Formato de imagem sem decodificador nesta versão (ex.: HEIC).",
        ));
    }
    let mut limites = Limits::default();
    limites.max_image_width = Some(20_000);
    limites.max_image_height = Some(20_000);
    limites.max_alloc = Some(1 << 30);
    leitor.limits(limites);
    // O texto do decodificador vai só como detalhe técnico, numa linha só.
    let ilegivel = |e: image::ImageError| {
        let detalhe = e.to_string().replace("\\n", " ").replace('\n', " ");
        Miniatura::sem(
            Falhou,
            format!("Imagem não pôde ser decodificada ({}).", detalhe.trim()),
        )
    };
    let mut decodificador = leitor.into_decoder().map_err(ilegivel)?;
    let orientacao = decodificador.orientation().ok();
    let mut img = DynamicImage::from_decoder(decodificador).map_err(ilegivel)?;
    if let Some(o) = orientacao {
        img.apply_orientation(o);
    }
    // Só reduz: imagem menor que a miniatura fica no tamanho original.
    if img.width() > lado || img.height() > lado {
        img = img.thumbnail(lado, lado);
    }
    let pequena = img.into_rgb8();
    let mut saida = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut saida, QUALIDADE_JPEG)
        .encode_image(&pequena)
        .map_err(|e| Miniatura::sem(Falhou, format!("Falha ao gerar a miniatura: {e}")))?;
    Ok(saida)
}

/// Grava num temporário e renomeia, para nunca deixar miniatura pela metade.
fn gravar_cache(destino: &Path, jpeg: &[u8]) -> std::io::Result<()> {
    let pasta = destino.parent().expect("caminho do cache tem pasta");
    fs::create_dir_all(pasta)?;
    let tmp = pasta.join(format!(
        ".{}.{}.tmp",
        destino.file_name().and_then(|n| n.to_str()).unwrap_or("m"),
        std::process::id()
    ));
    fs::write(&tmp, jpeg)?;
    match fs::rename(&tmp, destino) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            if destino.exists() {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

// ---------- Limite de decodificações simultâneas ----------

struct Vagas {
    livres: Mutex<usize>,
    aviso: Condvar,
}

fn vagas() -> &'static Vagas {
    static V: OnceLock<Vagas> = OnceLock::new();
    V.get_or_init(|| Vagas {
        livres: Mutex::new(
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(2)
                .clamp(1, 4),
        ),
        aviso: Condvar::new(),
    })
}

/// Ocupa uma vaga até sair de escopo: a grade pede dezenas de miniaturas de uma vez
/// e decodificar todas juntas travaria a estação.
struct Vaga;

fn vaga() -> Vaga {
    let v = vagas();
    let mut livres = v.livres.lock().unwrap_or_else(|e| e.into_inner());
    while *livres == 0 {
        livres = v.aviso.wait(livres).unwrap_or_else(|e| e.into_inner());
    }
    *livres -= 1;
    Vaga
}

impl Drop for Vaga {
    fn drop(&mut self) {
        let v = vagas();
        let mut livres = v.livres.lock().unwrap_or_else(|e| e.into_inner());
        *livres += 1;
        v.aviso.notify_one();
    }
}

#[cfg(test)]
mod testes;
