//! Descritor visual determinístico local, sem classificador neural.
//!
//! PNG/JPEG inventariados são lidos uma única vez pela fonte somente leitura;
//! tamanho/SHA-256 são conferidos ANTES de usar o decodificador publicado `image`.
//! A biblioteca não resolve caminhos, escreve, mantém cache nem faz rede. O
//! chamador é responsável por conferir a associação caso/extração/inventário.
//!
//! dHash64 compara luminâncias de uma matriz reduzida a 9×8 por filtro Triangle.
//! Pixel RGBA8 é composto sobre preto antes da comparação e da média técnica.
//! Dimensões/média referem-se à matriz decodificada, sem rotação EXIF, perfil ICC
//! ou interpretação fotométrica. Não há score/confiança/região de modelo.
//!
//! O descritor detecta semelhança estrutural grosseira, não identidade. Imagens
//! uniformes diferentes podem ter o mesmo dHash; recorte, rotação, transparência,
//! recompressão e alterações pequenas podem mudar o resultado. Sem classificador
//! instalado, objetos, pessoas, conteúdo e significado permanecem indisponíveis.
//!
//! Limites: 2 MiB de origem, 4096 por lado, 16 milhões de pixels e buffer de saída
//! até 64 MiB. `image::Limits::max_alloc` é best-effort, NÃO quota rígida de RSS.
//! O decoder pode ter alocações internas; não há isolamento de processo/OOM ou
//! interrupção preemptiva. Prazo/cancelamento são cooperativos, inclusive durante
//! a média, e conferidos antes da publicação. PNG animado e 16-bit não são aceitos.
//! JPEG multipart/MPO não é reduzido ao primeiro quadro. O envelope é conferido,
//! mas a interpretação dos pixels continua sujeita às tolerâncias do codec.

use chrono::{DateTime, Utc};
use image::codecs::{jpeg::JpegDecoder, png::PngDecoder};
use image::imageops::FilterType;
use image::{ColorType, DynamicImage, GenericImageView, ImageDecoder, ImageError, Limits, Rgba};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sied_dominio::{CasoId, ExtracaoId};
use sied_ingestao::fonte::{CaminhoFonte, FonteSomenteLeitura, SEPARADOR_CONTEINER};
use std::io::{Cursor, ErrorKind};
use std::time::Instant;
use ts_rs::TS;

use crate::{
    hash_valido, ControleDocumento, EstadoDocumento, ModoDocumento, PedidoDocumento, TAMANHO_MAXIMO,
};

pub const MOTOR_ID: &str = "visao_dhash_local";
/// Mudar esta versão quando decodificador, orientação, fundo ou redução mudarem.
pub const MOTOR_VERSAO: &str = "image-0.25.10/dhash64-triangle-rgba8-preto-sem-exif-v1";
pub const MAX_LADO: u32 = 4_096;
pub const MAX_PIXELS: u64 = 16_000_000;
pub const MAX_BUFFER_BYTES: u64 = 64 * 1024 * 1024;
pub const LIMITACOES: &str = "Descritor estrutural, não identidade. Objetos, pessoas e classificação neural indisponíveis. Matriz sem orientação EXIF/perfil ICC; imagens uniformes distintas podem coincidir.";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AssinaturaVisual {
    pub caso_id: CasoId,
    pub extracao_id: ExtracaoId,
    #[ts(type = "number")]
    pub arquivo_id: i64,
    pub caminho: CaminhoFonte,
    pub sha256: String,
    pub motor_id: String,
    pub motor_versao: String,
    pub executado_em: DateTime<Utc>,
    pub largura: u32,
    pub altura: u32,
    /// Exatamente 16 dígitos hexadecimais minúsculos; não é hash criptográfico.
    pub dhash: String,
    /// Média RGBA8/BT.601 composta sobre preto, em escala 0–255, sem calibração.
    pub luminancia_media: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResultadoVisual {
    pub estado: EstadoDocumento,
    pub assinatura: Option<AssinaturaVisual>,
    /// Nas recusas descreve o erro; em Pronto contém limitações do descritor.
    pub motivo: Option<String>,
}

impl ResultadoVisual {
    fn sem(estado: EstadoDocumento, motivo: &str) -> Self {
        Self {
            estado,
            assinatura: None,
            motivo: Some(motivo.to_owned()),
        }
    }
}

fn interrupcao(controle: &ControleDocumento, inicio: Instant) -> Option<ResultadoVisual> {
    controle
        .interrupcao(inicio)
        .map(|resultado| ResultadoVisual {
            estado: resultado.estado,
            assinatura: None,
            motivo: resultado.motivo,
        })
}

/// Produz métricas reais de uma imagem admitida, sem identificar seu conteúdo.
pub fn analisar(
    fonte: &dyn FonteSomenteLeitura,
    pedido: &PedidoDocumento,
    controle: &ControleDocumento,
) -> ResultadoVisual {
    use EstadoDocumento::*;
    let inicio = Instant::now();
    if let Some(resultado) = interrupcao(controle, inicio) {
        return resultado;
    }
    if pedido.arquivo_id <= 0 {
        return ResultadoVisual::sem(
            Falhou,
            "O identificador da imagem no inventário é inválido.",
        );
    }
    if pedido.modo != ModoDocumento::Automatico {
        return ResultadoVisual::sem(
            SemSuporte,
            "Este descritor visual não executa OCR nem classificação neural.",
        );
    }
    if pedido.caminho.como_str().contains(SEPARADOR_CONTEINER) {
        return ResultadoVisual::sem(
            SemSuporte,
            "Imagens dentro de contêineres não são analisadas neste motor.",
        );
    }
    let png = match pedido.caminho.extensao().as_deref() {
        Some("png") => true,
        Some("jpg" | "jpeg") => false,
        _ => {
            return ResultadoVisual::sem(
                SemSuporte,
                "O descritor visual admite somente PNG estático e JPEG.",
            )
        }
    };
    let Some(sha256) = pedido.sha256.as_deref() else {
        return ResultadoVisual::sem(
            HashAusente,
            "O inventário não contém SHA-256 para esta imagem.",
        );
    };
    if !hash_valido(sha256) {
        return ResultadoVisual::sem(Falhou, "O SHA-256 registrado no inventário é inválido.");
    }
    if pedido.tamanho > TAMANHO_MAXIMO {
        return ResultadoVisual::sem(
            LimiteExcedido,
            "A imagem excede o limite de origem de 2 MiB deste motor.",
        );
    }
    let bytes = match ler_conferido(fonte, pedido, controle, inicio) {
        Ok(bytes) => bytes,
        Err(resultado) => return *resultado,
    };
    if (png && !bytes.starts_with(b"\x89PNG\r\n\x1a\n"))
        || (!png && !bytes.starts_with(b"\xff\xd8\xff"))
    {
        return ResultadoVisual::sem(
            SemSuporte,
            "A assinatura do conteúdo não corresponde à extensão PNG/JPEG admitida.",
        );
    }
    if !png && !bytes.ends_with(b"\xff\xd9") {
        return ResultadoVisual::sem(
            Falhou,
            "O JPEG não contém um encerramento íntegro. Nenhum descritor foi publicado.",
        );
    }
    // Um PNG sem IEND, ou com bytes residuais após seu encerramento, não é
    // publicado como uma imagem completa por este perfil conservador.
    if png && !bytes.ends_with(b"\x00\x00\x00\x00IEND\xae\x42\x60\x82") {
        return ResultadoVisual::sem(
            Falhou,
            "O PNG não contém um encerramento íntegro. Nenhum descritor foi publicado.",
        );
    }
    let analise = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        descrever(&bytes, png, controle, inicio)
    }));
    if let Some(resultado) = interrupcao(controle, inicio) {
        return resultado;
    }
    let medidas =
        match analise {
            Ok(Ok(medidas)) => medidas,
            Ok(Err(resultado)) => return *resultado,
            Err(_) => return ResultadoVisual::sem(
                Falhou,
                "O decodificador não conseguiu concluir a imagem. Nenhum descritor foi publicado.",
            ),
        };
    let resultado = ResultadoVisual {
        estado: Pronto,
        assinatura: Some(AssinaturaVisual {
            caso_id: pedido.caso_id,
            extracao_id: pedido.extracao_id,
            arquivo_id: pedido.arquivo_id,
            caminho: pedido.caminho.clone(),
            sha256: sha256.to_owned(),
            motor_id: MOTOR_ID.into(),
            motor_versao: MOTOR_VERSAO.into(),
            executado_em: Utc::now(),
            largura: medidas.largura,
            altura: medidas.altura,
            dhash: format!("{:016x}", medidas.dhash),
            luminancia_media: medidas.luminancia_media,
        }),
        motivo: Some(LIMITACOES.into()),
    };
    if let Some(interrompido) = interrupcao(controle, inicio) {
        interrompido
    } else {
        resultado
    }
}

fn ler_conferido(
    fonte: &dyn FonteSomenteLeitura,
    pedido: &PedidoDocumento,
    controle: &ControleDocumento,
    inicio: Instant,
) -> Result<Vec<u8>, Box<ResultadoVisual>> {
    let abertura = fonte.abrir(&pedido.caminho);
    if let Some(resultado) = interrupcao(controle, inicio) {
        return Err(Box::new(resultado));
    }
    let mut leitor = abertura.map_err(|_| {
        Box::new(ResultadoVisual::sem(
            EstadoDocumento::OrigemIndisponivel,
            "A origem da imagem não está disponível para leitura.",
        ))
    })?;
    let mut bytes = Vec::with_capacity(pedido.tamanho as usize);
    let mut hasher = Sha256::new();
    let mut chunk = [0_u8; 16 * 1024];
    loop {
        if let Some(resultado) = interrupcao(controle, inicio) {
            return Err(Box::new(resultado));
        }
        let permitido = (TAMANHO_MAXIMO as usize - bytes.len() + 1).min(chunk.len());
        let leitura = leitor.read(&mut chunk[..permitido]);
        if let Some(resultado) = interrupcao(controle, inicio) {
            return Err(Box::new(resultado));
        }
        let lido = match leitura {
            Ok(lido) if lido <= permitido => lido,
            Ok(_) => {
                return Err(Box::new(ResultadoVisual::sem(
                    EstadoDocumento::Falhou,
                    "O leitor da imagem retornou um tamanho inválido.",
                )))
            }
            Err(erro) if erro.kind() == ErrorKind::Interrupted => continue,
            Err(_) => {
                return Err(Box::new(ResultadoVisual::sem(
                    EstadoDocumento::OrigemIndisponivel,
                    "Não foi possível concluir a leitura da imagem.",
                )))
            }
        };
        if lido == 0 {
            break;
        }
        if bytes.len() + lido > TAMANHO_MAXIMO as usize {
            return Err(Box::new(ResultadoVisual::sem(
                EstadoDocumento::LimiteExcedido,
                "A origem excede o limite de 2 MiB. Nenhum descritor foi publicado.",
            )));
        }
        hasher.update(&chunk[..lido]);
        bytes.extend_from_slice(&chunk[..lido]);
    }
    if bytes.len() as u64 != pedido.tamanho
        || Some(hex::encode(hasher.finalize()).as_str()) != pedido.sha256.as_deref()
    {
        return Err(Box::new(ResultadoVisual::sem(EstadoDocumento::OrigemDivergente, "O SHA-256 ou o tamanho da imagem difere do inventário. Nenhum descritor foi publicado.")));
    }
    Ok(bytes)
}

struct Medidas {
    largura: u32,
    altura: u32,
    dhash: u64,
    luminancia_media: f64,
}

fn limites() -> Limits {
    let mut limites = Limits::default();
    limites.max_image_width = Some(MAX_LADO);
    limites.max_image_height = Some(MAX_LADO);
    limites.max_alloc = Some(MAX_BUFFER_BYTES);
    limites
}

fn erro_imagem(erro: ImageError) -> Box<ResultadoVisual> {
    let (estado, motivo) = match erro {
        ImageError::Limits(_) => (
            EstadoDocumento::LimiteExcedido,
            "A imagem excede os limites de dimensões ou buffer de decodificação.",
        ),
        ImageError::Unsupported(_) => (
            EstadoDocumento::SemSuporte,
            "A variante de PNG/JPEG não tem suporte neste descritor.",
        ),
        _ => (
            EstadoDocumento::Falhou,
            "A imagem não pôde ser decodificada. Nenhum descritor foi publicado.",
        ),
    };
    Box::new(ResultadoVisual::sem(estado, motivo))
}

fn conferir_decoder(decoder: &impl ImageDecoder) -> Result<(), Box<ResultadoVisual>> {
    let (largura, altura) = decoder.dimensions();
    if largura == 0 || altura == 0 {
        return Err(Box::new(ResultadoVisual::sem(
            EstadoDocumento::Falhou,
            "A imagem não possui dimensões válidas.",
        )));
    }
    if largura > MAX_LADO
        || altura > MAX_LADO
        || u64::from(largura) * u64::from(altura) > MAX_PIXELS
        || decoder.total_bytes() > MAX_BUFFER_BYTES
    {
        return Err(Box::new(ResultadoVisual::sem(
            EstadoDocumento::LimiteExcedido,
            "A imagem excede 4096 pixels por lado, 16 milhões de pixels ou 64 MiB de buffer.",
        )));
    }
    if !matches!(
        decoder.color_type(),
        ColorType::L8 | ColorType::La8 | ColorType::Rgb8 | ColorType::Rgba8
    ) {
        return Err(Box::new(ResultadoVisual::sem(
            EstadoDocumento::SemSuporte,
            "Imagens acima de 8 bits por canal não são interpretadas neste perfil visual.",
        )));
    }
    Ok(())
}

fn descrever(
    bytes: &[u8],
    png: bool,
    controle: &ControleDocumento,
    inicio: Instant,
) -> Result<Medidas, Box<ResultadoVisual>> {
    let imagem = if png {
        let decoder =
            PngDecoder::with_limits(Cursor::new(bytes), limites()).map_err(erro_imagem)?;
        if decoder.is_apng().map_err(erro_imagem)? {
            return Err(Box::new(ResultadoVisual::sem(EstadoDocumento::SemSuporte, "PNG animado não é analisado; selecionar um frame não representaria o arquivo inteiro.")));
        }
        conferir_decoder(&decoder)?;
        if let Some(resultado) = interrupcao(controle, inicio) {
            return Err(Box::new(resultado));
        }
        DynamicImage::from_decoder(decoder).map_err(erro_imagem)?
    } else {
        conferir_jpeg_unico(bytes, controle, inicio)?;
        let mut decoder = JpegDecoder::new(Cursor::new(bytes)).map_err(erro_imagem)?;
        decoder.set_limits(limites()).map_err(erro_imagem)?;
        conferir_decoder(&decoder)?;
        if let Some(resultado) = interrupcao(controle, inicio) {
            return Err(Box::new(resultado));
        }
        DynamicImage::from_decoder(decoder).map_err(erro_imagem)?
    };
    if let Some(resultado) = interrupcao(controle, inicio) {
        return Err(Box::new(resultado));
    }
    let (largura, altura) = imagem.dimensions();
    let reduzida = imagem.resize_exact(9, 8, FilterType::Triangle);
    if let Some(resultado) = interrupcao(controle, inicio) {
        return Err(Box::new(resultado));
    }
    let mut dhash = 0_u64;
    for y in 0..8 {
        for x in 0..8 {
            dhash <<= 1;
            if luminancia(reduzida.get_pixel(x, y)) > luminancia(reduzida.get_pixel(x + 1, y)) {
                dhash |= 1;
            }
        }
    }
    let mut soma = 0_u64;
    for (indice, (_, _, pixel)) in imagem.pixels().enumerate() {
        if indice % 16_384 == 0 {
            if let Some(resultado) = interrupcao(controle, inicio) {
                return Err(Box::new(resultado));
            }
        }
        soma += u64::from(luminancia(pixel));
    }
    Ok(Medidas {
        largura,
        altura,
        dhash,
        luminancia_media: soma as f64 / (u64::from(largura) * u64::from(altura)) as f64 / 1_000.0,
    })
}

/// Inspeção limitada do envelope de markers, não um decodificador de JPEG.
/// Segmentos de metadados são pulados por comprimento; thumbnails EXIF internos
/// não são confundidos com um segundo JPEG concatenado. Entropia é decodificada
/// exclusivamente pela biblioteca publicada depois desta conferência.
fn conferir_jpeg_unico(
    bytes: &[u8],
    controle: &ControleDocumento,
    inicio: Instant,
) -> Result<(), Box<ResultadoVisual>> {
    let erro = || {
        Box::new(ResultadoVisual::sem(
        EstadoDocumento::Falhou,
        "O envelope JPEG não representa uma única imagem íntegra. Nenhum descritor foi publicado.",
    ))
    };
    let mut posicao = 2;
    let mut entropia = false;
    let mut teve_scan = false;
    let mut markers = 0;
    while posicao < bytes.len() {
        if posicao % 16_384 == 0 {
            if let Some(resultado) = interrupcao(controle, inicio) {
                return Err(Box::new(resultado));
            }
        }
        if entropia && bytes[posicao] != 0xff {
            posicao += 1;
            continue;
        }
        if bytes[posicao] != 0xff {
            return Err(erro());
        }
        while bytes.get(posicao) == Some(&0xff) {
            posicao += 1;
        }
        let marker = *bytes.get(posicao).ok_or_else(erro)?;
        posicao += 1;
        if entropia && (marker == 0x00 || (0xd0..=0xd7).contains(&marker)) {
            continue;
        }
        if marker == 0xd9 {
            return if teve_scan && posicao == bytes.len() {
                Ok(())
            } else {
                Err(erro())
            };
        }
        if marker == 0xd8 || marker == 0x00 || (0xd0..=0xd7).contains(&marker) {
            return Err(erro());
        }
        if marker == 0x01 {
            continue;
        }
        markers += 1;
        if markers > 16_384 {
            return Err(Box::new(ResultadoVisual::sem(
                EstadoDocumento::LimiteExcedido,
                "O envelope JPEG excede o limite de segmentos deste perfil.",
            )));
        }
        let tamanho_bytes = bytes.get(posicao..posicao + 2).ok_or_else(erro)?;
        let tamanho = usize::from(u16::from_be_bytes([tamanho_bytes[0], tamanho_bytes[1]]));
        if tamanho < 2 {
            return Err(erro());
        }
        let fim = posicao.checked_add(tamanho).ok_or_else(erro)?;
        let segmento = bytes.get(posicao + 2..fim).ok_or_else(erro)?;
        if marker == 0xe2 && segmento.starts_with(b"MPF\0") {
            return Err(Box::new(ResultadoVisual::sem(
                EstadoDocumento::SemSuporte,
                "JPEG MPO/multipart não é analisado como uma única imagem neste motor.",
            )));
        }
        posicao = fim;
        entropia = marker == 0xda;
        teve_scan |= entropia;
    }
    Err(erro())
}

/// Luminância técnica em milésimos de 0–255, alpha composto sobre preto.
fn luminancia(pixel: Rgba<u8>) -> u32 {
    let [r, g, b, a] = pixel.0;
    (299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b)) * u32::from(a) / 255
}

/// Distância estrutural entre dois descritores deste mesmo motor/versão.
/// Zero NÃO certifica que imagens são iguais ou representam a mesma coisa.
pub fn distancia_dhash(esquerda: &str, direita: &str) -> Option<u32> {
    let valida = |valor: &str| {
        valor.len() == 16
            && valor
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    };
    if !valida(esquerda) || !valida(direita) {
        return None;
    }
    Some(
        (u64::from_str_radix(esquerda, 16).ok()? ^ u64::from_str_radix(direita, 16).ok()?)
            .count_ones(),
    )
}
