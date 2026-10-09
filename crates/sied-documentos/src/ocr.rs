//! OCR local sobre bytes já conferidos contra o inventário.
//!
//! No Windows o provedor é a API nativa `Windows.Media.Ocr`: nenhum byte é
//! enviado pela rede, nenhum executável externo é iniciado e nenhum caminho da
//! evidência é entregue ao sistema operacional. PDF é renderizado em memória
//! por `Windows.Data.Pdf`. Outros sistemas devolvem indisponibilidade explícita.

use crate::{ControleDocumento, EstadoDocumento, RegiaoTexto, ResultadoDocumento};
use std::time::Instant;

pub const MOTOR_ID: &str = "windows_media_ocr";
pub const MOTOR_VERSAO: &str = "1.0.0";
pub const TAMANHO_MAXIMO_ORIGEM: u64 = 64 * 1024 * 1024;

pub(crate) const fn disponivel() -> bool {
    cfg!(windows)
}

pub(crate) struct SaidaOcr {
    pub texto: String,
    pub regioes: Vec<RegiaoTexto>,
    pub pagina: u32,
    pub total_paginas: u32,
    pub largura_px: u32,
    pub altura_px: u32,
    pub idioma: String,
}

pub(crate) fn reconhecer(
    bytes: &[u8],
    eh_pdf: bool,
    pagina: u32,
    controle: &ControleDocumento,
    inicio: Instant,
) -> Result<SaidaOcr, Box<ResultadoDocumento>> {
    if let Some(resultado) = controle.interrupcao(inicio) {
        return Err(Box::new(resultado));
    }
    #[cfg(windows)]
    {
        windows_impl::reconhecer(bytes, eh_pdf, pagina, controle, inicio)
    }
    #[cfg(not(windows))]
    {
        let _ = (bytes, eh_pdf, pagina);
        Err(Box::new(ResultadoDocumento::sem(
            EstadoDocumento::SemSuporte,
            "O OCR local desta versão usa o motor nativo do Windows e não está disponível neste sistema.",
        )))
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use windows::Data::Pdf::PdfDocument;
    use windows::Graphics::Imaging::{BitmapDecoder, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

    fn falha(motivo: &str) -> Box<ResultadoDocumento> {
        Box::new(ResultadoDocumento::sem(EstadoDocumento::Falhou, motivo))
    }

    fn sem_suporte(motivo: &str) -> Box<ResultadoDocumento> {
        Box::new(ResultadoDocumento::sem(EstadoDocumento::SemSuporte, motivo))
    }

    fn stream_com_bytes(bytes: &[u8]) -> windows::core::Result<InMemoryRandomAccessStream> {
        let stream = InMemoryRandomAccessStream::new()?;
        let writer = DataWriter::CreateDataWriter(&stream)?;
        writer.WriteBytes(bytes)?;
        writer.StoreAsync()?.join()?;
        writer.FlushAsync()?.join()?;
        writer.DetachStream()?;
        stream.Seek(0)?;
        Ok(stream)
    }

    fn bitmap_imagem(bytes: &[u8]) -> windows::core::Result<SoftwareBitmap> {
        let stream = stream_com_bytes(bytes)?;
        let decoder = BitmapDecoder::CreateAsync(&stream)?.join()?;
        decoder.GetSoftwareBitmapAsync()?.join()
    }

    fn bitmap_pdf(bytes: &[u8], pagina: u32) -> windows::core::Result<(SoftwareBitmap, u32)> {
        let entrada = stream_com_bytes(bytes)?;
        let pdf = PdfDocument::LoadFromStreamAsync(&entrada)?.join()?;
        let total = pdf.PageCount()?;
        if pagina == 0 || pagina > total {
            return Err(windows::core::Error::new(
                windows::core::HRESULT(0x80070057_u32 as i32),
                "Página OCR fora do intervalo do PDF",
            ));
        }
        let pagina_pdf = pdf.GetPage(pagina - 1)?;
        let render = InMemoryRandomAccessStream::new()?;
        pagina_pdf.RenderToStreamAsync(&render)?.join()?;
        render.Seek(0)?;
        let decoder = BitmapDecoder::CreateAsync(&render)?.join()?;
        Ok((decoder.GetSoftwareBitmapAsync()?.join()?, total))
    }

    pub(super) fn reconhecer(
        bytes: &[u8],
        eh_pdf: bool,
        pagina: u32,
        controle: &ControleDocumento,
        inicio: Instant,
    ) -> Result<SaidaOcr, Box<ResultadoDocumento>> {
        let (bitmap, total_paginas) = if eh_pdf {
            bitmap_pdf(bytes, pagina).map_err(|_| {
                falha("O PDF não pôde ser renderizado localmente para OCR. Nenhum texto foi publicado.")
            })?
        } else {
            (
                bitmap_imagem(bytes).map_err(|_| {
                    sem_suporte("A imagem não pôde ser decodificada pelo motor local de OCR.")
                })?,
                1,
            )
        };
        if let Some(resultado) = controle.interrupcao(inicio) {
            return Err(Box::new(resultado));
        }
        let largura = u32::try_from(
            bitmap
                .PixelWidth()
                .map_err(|_| falha("O motor OCR não informou a largura da imagem."))?,
        )
        .map_err(|_| falha("A largura da imagem excede os limites do OCR."))?;
        let altura = u32::try_from(
            bitmap
                .PixelHeight()
                .map_err(|_| falha("O motor OCR não informou a altura da imagem."))?,
        )
        .map_err(|_| falha("A altura da imagem excede os limites do OCR."))?;
        let maximo = OcrEngine::MaxImageDimension()
            .map_err(|_| falha("O limite do motor OCR local não pôde ser consultado."))?;
        if largura > maximo || altura > maximo {
            return Err(Box::new(ResultadoDocumento::sem(
                EstadoDocumento::LimiteExcedido,
                "A imagem excede a dimensão máxima aceita pelo motor OCR local.",
            )));
        }
        let motor = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| {
            sem_suporte("Nenhum idioma OCR compatível está instalado no perfil do Windows.")
        })?;
        let idioma = motor
            .RecognizerLanguage()
            .and_then(|lingua| lingua.LanguageTag())
            .map_err(|_| falha("O motor OCR local não informou o idioma utilizado."))?
            .to_string_lossy();
        let resultado = motor
            .RecognizeAsync(&bitmap)
            .and_then(|op| op.join())
            .map_err(|_| falha("O motor OCR local não conseguiu reconhecer esta página."))?;
        if let Some(resultado) = controle.interrupcao(inicio) {
            return Err(Box::new(resultado));
        }

        let linhas = resultado
            .Lines()
            .map_err(|_| falha("O motor OCR retornou linhas inválidas."))?;
        let mut texto = String::new();
        let mut regioes = Vec::new();
        for indice_linha in 0..linhas
            .Size()
            .map_err(|_| falha("O motor OCR retornou uma coleção inválida."))?
        {
            if indice_linha > 0 {
                texto.push('\n');
            }
            let linha = linhas
                .GetAt(indice_linha)
                .map_err(|_| falha("Uma linha OCR não pôde ser lida."))?;
            let palavras = linha
                .Words()
                .map_err(|_| falha("Uma linha OCR contém palavras inválidas."))?;
            for indice_palavra in 0..palavras
                .Size()
                .map_err(|_| falha("O motor OCR retornou palavras inválidas."))?
            {
                if indice_palavra > 0 {
                    texto.push(' ');
                }
                let palavra = palavras
                    .GetAt(indice_palavra)
                    .map_err(|_| falha("Uma palavra OCR não pôde ser lida."))?;
                let literal = palavra
                    .Text()
                    .map_err(|_| falha("Uma palavra OCR não contém texto válido."))?
                    .to_string_lossy();
                let inicio_utf8 = u32::try_from(texto.len())
                    .map_err(|_| falha("O texto OCR excede o limite publicável."))?;
                texto.push_str(&literal);
                let fim_utf8 = u32::try_from(texto.len())
                    .map_err(|_| falha("O texto OCR excede o limite publicável."))?;
                let caixa = palavra
                    .BoundingRect()
                    .map_err(|_| falha("A região de uma palavra OCR é inválida."))?;
                let x = caixa.X.max(0.0).floor().min(largura as f32) as u32;
                let y = caixa.Y.max(0.0).floor().min(altura as f32) as u32;
                let direita = (caixa.X + caixa.Width).max(0.0).ceil().min(largura as f32) as u32;
                let base = (caixa.Y + caixa.Height).max(0.0).ceil().min(altura as f32) as u32;
                regioes.push(RegiaoTexto {
                    ordem: regioes.len() as u32,
                    inicio_utf8,
                    fim_utf8,
                    texto: literal,
                    x,
                    y,
                    largura: direita.saturating_sub(x),
                    altura: base.saturating_sub(y),
                    confianca: None,
                });
            }
        }
        Ok(SaidaOcr {
            texto,
            regioes,
            pagina,
            total_paginas,
            largura_px: largura,
            altura_px: altura,
            idioma,
        })
    }
}
