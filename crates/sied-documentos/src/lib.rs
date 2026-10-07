//! Leitura documental local: texto UTF-8 fiel e texto nativo de páginas PDF.
//!
//! A origem só é acessada por [`FonteSomenteLeitura`]. O confinamento da origem é
//! responsabilidade dessa implementação; esta biblioteca não resolve caminhos,
//! percorre pastas nem acessa arquivos por outros meios. Não há escrita, cache,
//! banco, processo externo, rede, OCR ou modelo de IA neste incremento.
//!
//! Os bytes e o tamanho são conferidos contra o inventário antes de interpretar
//! o conteúdo. Um resultado UTF-8 pronto preserva integralmente os bytes,
//! incluindo BOM, finais de linha e caracteres não ASCII. Não há normalização,
//! truncamento, conversão com perdas ou resultado parcial.
//!
//! O limite de leitura é [`TAMANHO_MAXIMO`] mais um byte sentinela necessário para
//! detectar excesso quando o tamanho do inventário não reflete a origem. Esse
//! byte nunca é retido nem publicado. O prazo e o cancelamento são cooperativos:
//! são verificados antes e depois de abrir, de cada leitura, da interpretação e
//! da publicação. Uma chamada bloqueada à fonte não pode ser interrompida por
//! este contrato; não se promete um limite rígido de tempo de E/S.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sied_dominio::{CasoId, ExtracaoId};
use sied_ingestao::fonte::{CaminhoFonte, FonteSomenteLeitura, SEPARADOR_CONTEINER};
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use ts_rs::TS;

pub mod entidades;
mod pdf;
pub mod visao;

/// Máximo de bytes da origem e do texto publicado: 2 MiB.
pub const TAMANHO_MAXIMO: u64 = 2 * 1024 * 1024;
/// Prazo cooperativo máximo por chamada, contado desde o início de [`analisar`].
pub const PRAZO_PADRAO: Duration = Duration::from_secs(5);
/// Identificador estável do motor de texto UTF-8.
pub const MOTOR_ID: &str = "texto_utf8_nativo";
/// Versão semântica da interpretação, independente da origem do documento.
pub const MOTOR_VERSAO: &str = "0.1.0";
pub const MOTOR_PDF_ID: &str = pdf::MOTOR_ID;
pub const MOTOR_PDF_VERSAO: &str = pdf::MOTOR_VERSAO;

const TAMANHO_CHUNK: usize = 16 * 1024;

/// Seleção do motor. OCR é uma solicitação explícita sem motor instalado aqui.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModoDocumento {
    #[default]
    Automatico,
    Ocr,
}

/// Identidade e integridade conhecidas pelo inventário, sem acesso à origem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PedidoDocumento {
    pub caso_id: CasoId,
    pub extracao_id: ExtracaoId,
    /// Identificador positivo do arquivo no inventário do caso.
    pub arquivo_id: i64,
    pub caminho: CaminhoFonte,
    /// SHA-256 completo do inventário: exatamente 64 dígitos hexadecimais minúsculos.
    pub sha256: Option<String>,
    pub tamanho: u64,
    pub modo: ModoDocumento,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EstadoDocumento {
    Pronto,
    /// Motor, extensão ou codificação indisponível; não implica origem verificada.
    SemSuporte,
    /// Tamanho ou SHA-256 dos bytes lidos difere do inventário.
    OrigemDivergente,
    OrigemIndisponivel,
    HashAusente,
    /// Limite de bytes ou prazo cooperativo excedido.
    LimiteExcedido,
    /// Cancelamento explicitamente sinalizado pelo chamador.
    Cancelado,
    /// Pedido inválido ou falha no contrato de leitura.
    Falhou,
}

/// Texto derivado com a identidade completa da origem e do motor.
///
/// `sha256_texto` considera somente os bytes de `texto`: horário, identidade e
/// demais metadados não participam desse hash. No motor UTF-8 os bytes são
/// idênticos à origem. No PDF, este é o hash do texto DERIVADO da página, não
/// o hash do PDF nem uma garantia de ordem visual ou reprodução do layout.
/// Somente PDF informa página e contagem observadas; texto plano usa `None`.
/// Não há região nem confiança de OCR neste contrato.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TextoDocumento {
    pub caso_id: CasoId,
    pub extracao_id: ExtracaoId,
    #[ts(type = "number")]
    pub arquivo_id: i64,
    pub caminho: CaminhoFonte,
    pub sha256: String,
    pub motor_id: String,
    pub motor_versao: String,
    pub executado_em: DateTime<Utc>,
    pub sha256_texto: String,
    pub texto: String,
    pub pagina: Option<u32>,
    pub total_paginas: Option<u32>,
}

/// Só `Pronto` traz texto. Toda outra saída informa motivo fixo, sem caminhos,
/// conteúdo da evidência ou detalhes de erro fornecidos pela origem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResultadoDocumento {
    pub estado: EstadoDocumento,
    pub texto: Option<TextoDocumento>,
    pub motivo: Option<String>,
}

impl ResultadoDocumento {
    pub(crate) fn sem(estado: EstadoDocumento, motivo: &str) -> Self {
        Self {
            estado,
            texto: None,
            motivo: Some(motivo.to_owned()),
        }
    }
}

/// Sinal de cancelamento compartilhável entre threads e prazo por análise.
#[derive(Debug)]
pub struct ControleDocumento {
    cancelado: AtomicBool,
    prazo: Duration,
}

impl ControleDocumento {
    pub fn novo() -> Self {
        Self::com_prazo(PRAZO_PADRAO)
    }

    /// Escolhe um prazo menor ou igual a cinco segundos. O relógio começa em
    /// [`analisar`]; um prazo zero impede que a fonte seja aberta.
    pub fn com_prazo(prazo: Duration) -> Self {
        Self {
            cancelado: AtomicBool::new(false),
            prazo: prazo.min(PRAZO_PADRAO),
        }
    }

    /// Sinal monotônico: um controle cancelado não pode ser reutilizado como ativo.
    pub fn cancelar(&self) {
        self.cancelado.store(true, Ordering::Release);
    }

    /// Consulta cooperativa do sinal, inclusive entre leitura e publicação de
    /// derivados. Não interrompe E/S nem processos nativos já bloqueados.
    pub fn esta_cancelado(&self) -> bool {
        self.cancelado.load(Ordering::Acquire)
    }

    pub(crate) fn interrupcao(&self, inicio: Instant) -> Option<ResultadoDocumento> {
        if self.esta_cancelado() {
            Some(ResultadoDocumento::sem(
                EstadoDocumento::Cancelado,
                "A análise do documento foi cancelada.",
            ))
        } else if inicio.elapsed() >= self.prazo {
            Some(ResultadoDocumento::sem(
                EstadoDocumento::LimiteExcedido,
                "O prazo cooperativo da análise do documento foi excedido.",
            ))
        } else {
            None
        }
    }
}

impl Default for ControleDocumento {
    fn default() -> Self {
        Self::novo()
    }
}

/// Analisa integralmente um arquivo inventariado sem modificar a origem.
///
/// São aceitas as extensões `txt`, `csv`, `tsv`, `json`, `xml`, `md`, `log` e
/// um subconjunto conservador de PDF nativo. PDF usa a primeira página por padrão.
/// Contêineres, Office, imagens e OCR são explicitamente
/// sem suporte, sem abertura da fonte e sem alegação de integridade verificada.
/// Para formatos admitidos, hash e tamanho do inventário são obrigatórios e são
/// conferidos antes da interpretação. No motor UTF-8, assinaturas PDF/ZIP e
/// controles binários impedem publicar um binário renomeado como texto.
///
/// O chamador deve resolver o pedido no inventário do caso aberto, vinculando
/// caso, extração, arquivo e fonte. Esta biblioteca não consulta o banco nem
/// certifica a associação dos identificadores informados; não deve receber
/// metadados arbitrários de um frontend como se fossem dados do inventário.
pub fn analisar(
    fonte: &dyn FonteSomenteLeitura,
    p: &PedidoDocumento,
    controle: &ControleDocumento,
) -> ResultadoDocumento {
    analisar_pagina(fonte, p, controle, None)
}

/// Seleciona uma página real do PDF, numerada a partir de 1. Texto plano não
/// aceita paginação. Os mesmos limites e conferência de origem de [`analisar`]
/// se aplicam a cada leitura; não há cache persistente nem publicação parcial.
pub fn analisar_pagina(
    fonte: &dyn FonteSomenteLeitura,
    p: &PedidoDocumento,
    controle: &ControleDocumento,
    pagina: Option<u32>,
) -> ResultadoDocumento {
    use EstadoDocumento::*;

    let inicio = Instant::now();
    if let Some(resultado) = controle.interrupcao(inicio) {
        return resultado;
    }
    if p.arquivo_id <= 0 {
        return ResultadoDocumento::sem(
            Falhou,
            "O identificador do arquivo no inventário é inválido.",
        );
    }
    if p.modo == ModoDocumento::Ocr {
        return ResultadoDocumento::sem(
            SemSuporte,
            "Não há motor de OCR disponível neste incremento.",
        );
    }
    if p.caminho.como_str().contains(SEPARADOR_CONTEINER) {
        return ResultadoDocumento::sem(
            SemSuporte,
            "Documentos dentro de contêineres não são analisados neste incremento.",
        );
    }
    let eh_pdf = p.caminho.extensao().as_deref() == Some("pdf");
    if !matches!(
        p.caminho.extensao().as_deref(),
        Some("txt" | "csv" | "tsv" | "json" | "xml" | "md" | "log" | "pdf")
    ) {
        return ResultadoDocumento::sem(
            SemSuporte,
            "A extensão do documento não tem motor disponível neste incremento.",
        );
    }
    if (eh_pdf && !(1..=pdf::MAXIMO_PAGINAS).contains(&pagina.unwrap_or(1)))
        || (!eh_pdf && pagina.is_some())
    {
        return ResultadoDocumento::sem(
            Falhou,
            "A página solicitada não é válida para este documento.",
        );
    }
    let Some(sha256_esperado) = p.sha256.as_deref() else {
        return ResultadoDocumento::sem(
            HashAusente,
            "O inventário não contém SHA-256 para este arquivo.",
        );
    };
    if !hash_valido(sha256_esperado) {
        return ResultadoDocumento::sem(Falhou, "O SHA-256 registrado no inventário é inválido.");
    }
    if p.tamanho > TAMANHO_MAXIMO {
        return limite_de_bytes();
    }
    if let Some(resultado) = controle.interrupcao(inicio) {
        return resultado;
    }

    let abertura = fonte.abrir(&p.caminho);
    if let Some(resultado) = controle.interrupcao(inicio) {
        return resultado;
    }
    let mut leitor = match abertura {
        Ok(leitor) => leitor,
        Err(_) => {
            return ResultadoDocumento::sem(
                OrigemIndisponivel,
                "A origem do documento não está disponível para leitura.",
            );
        }
    };

    let mut bytes = Vec::with_capacity(p.tamanho as usize);
    let mut hasher = Sha256::new();
    let mut chunk = [0_u8; TAMANHO_CHUNK];
    loop {
        if let Some(resultado) = controle.interrupcao(inicio) {
            return resultado;
        }
        // Quando o buffer atingir o limite, só um byte é solicitado para distinguir
        // EOF de excesso. Um leitor nunca pode nos fazer consumir o restante da origem.
        let permitido = (TAMANHO_MAXIMO as usize - bytes.len() + 1).min(chunk.len());
        let leitura = leitor.read(&mut chunk[..permitido]);
        if let Some(resultado) = controle.interrupcao(inicio) {
            return resultado;
        }
        let lido = match leitura {
            Ok(lido) if lido <= permitido => lido,
            Ok(_) => {
                return ResultadoDocumento::sem(
                    Falhou,
                    "O leitor da origem retornou um tamanho inválido.",
                );
            }
            Err(erro) if erro.kind() == ErrorKind::Interrupted => continue,
            Err(_) => {
                return ResultadoDocumento::sem(
                    OrigemIndisponivel,
                    "Não foi possível concluir a leitura da origem do documento.",
                );
            }
        };
        if lido == 0 {
            break;
        }
        if bytes.len() + lido > TAMANHO_MAXIMO as usize {
            return limite_de_bytes();
        }
        hasher.update(&chunk[..lido]);
        bytes.extend_from_slice(&chunk[..lido]);
    }

    if let Some(resultado) = controle.interrupcao(inicio) {
        return resultado;
    }
    let sha256_lido = hex::encode(hasher.finalize());
    if let Some(resultado) = controle.interrupcao(inicio) {
        return resultado;
    }
    if bytes.len() as u64 != p.tamanho || sha256_lido != sha256_esperado {
        return ResultadoDocumento::sem(
            OrigemDivergente,
            "O SHA-256 ou o tamanho da origem difere do inventário. Nenhum texto foi gerado.",
        );
    }
    if eh_pdf {
        // Pânico do parser vira falha sem texto. Isto não isola o processo nem
        // contém OOM: os limites do motor são um envelope conservador, não RSS.
        let extracao = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pdf::extrair(&bytes, pagina.unwrap_or(1), controle, inicio)
        }));
        if let Some(resultado) = controle.interrupcao(inicio) {
            return resultado;
        }
        let (texto, total_paginas) = match extracao {
            Ok(Ok(texto)) => texto,
            Ok(Err(resultado)) => return *resultado,
            Err(_) => {
                return ResultadoDocumento::sem(
                    Falhou,
                    "O motor PDF não conseguiu concluir a leitura. Nenhum texto foi publicado.",
                )
            }
        };
        let sha256_texto = hex::encode(Sha256::digest(texto.as_bytes()));
        if let Some(resultado) = controle.interrupcao(inicio) {
            return resultado;
        }
        return ResultadoDocumento {
            estado: Pronto,
            texto: Some(TextoDocumento {
                caso_id: p.caso_id,
                extracao_id: p.extracao_id,
                arquivo_id: p.arquivo_id,
                caminho: p.caminho.clone(),
                sha256: sha256_esperado.to_owned(),
                motor_id: MOTOR_PDF_ID.to_owned(),
                motor_versao: MOTOR_PDF_VERSAO.to_owned(),
                executado_em: Utc::now(),
                sha256_texto,
                texto,
                pagina: Some(pagina.unwrap_or(1)),
                total_paginas: Some(total_paginas),
            }),
            motivo: None,
        };
    }
    let conteudo_sem_suporte = assinatura_sem_suporte(&bytes) || possui_controles_binarios(&bytes);
    if let Some(resultado) = controle.interrupcao(inicio) {
        return resultado;
    }
    if conteudo_sem_suporte {
        return ResultadoDocumento::sem(
            SemSuporte,
            "O conteúdo não é um documento de texto nativo admitido neste incremento.",
        );
    }
    let decodificacao = String::from_utf8(bytes);
    if let Some(resultado) = controle.interrupcao(inicio) {
        return resultado;
    }
    let texto = match decodificacao {
        Ok(texto) => texto,
        Err(_) => {
            return ResultadoDocumento::sem(
                SemSuporte,
                "O documento não contém texto UTF-8 válido. Não há conversão com perdas.",
            );
        }
    };

    if let Some(resultado) = controle.interrupcao(inicio) {
        return resultado;
    }
    let resultado = ResultadoDocumento {
        estado: Pronto,
        texto: Some(TextoDocumento {
            caso_id: p.caso_id,
            extracao_id: p.extracao_id,
            arquivo_id: p.arquivo_id,
            caminho: p.caminho.clone(),
            sha256: sha256_esperado.to_owned(),
            motor_id: MOTOR_ID.to_owned(),
            motor_versao: MOTOR_VERSAO.to_owned(),
            executado_em: Utc::now(),
            // UTF-8 estrito e sem transformações: o hash verificado é também o
            // hash do conteúdo publicado, independentemente dos metadados.
            sha256_texto: sha256_lido,
            texto,
            pagina: None,
            total_paginas: None,
        }),
        motivo: None,
    };
    if let Some(interrupcao) = controle.interrupcao(inicio) {
        return interrupcao;
    }
    resultado
}

fn hash_valido(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn limite_de_bytes() -> ResultadoDocumento {
    ResultadoDocumento::sem(
        EstadoDocumento::LimiteExcedido,
        "O documento excede o limite de 2 MiB para leitura e texto neste incremento.",
    )
}

fn assinatura_sem_suporte(bytes: &[u8]) -> bool {
    // O BOM continua nos bytes originais. Só a inspeção da assinatura o ignora.
    let inicio = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    inicio.starts_with(b"%PDF-")
        || inicio.starts_with(b"PK\x03\x04")
        || inicio.starts_with(b"PK\x05\x06")
        || inicio.starts_with(b"PK\x07\x08")
}

fn possui_controles_binarios(bytes: &[u8]) -> bool {
    // TAB, LF e CR são preservados. NUL, outros controles C0 e DEL não são
    // admitidos como texto nativo; bytes não ASCII seguem para o UTF-8 estrito.
    bytes
        .iter()
        .any(|&byte| (byte < b' ' && !matches!(byte, b'\t' | b'\n' | b'\r')) || byte == 0x7f)
}

#[cfg(test)]
mod contratos {
    use super::*;

    /// A CI publica o texto realmente gerado pelo Rust no log para recuperação
    /// dos contratos quando o toolchain nativo não estiver disponível localmente.
    #[test]
    fn exportar_contratos_documentais() {
        for (nome, conteudo) in [
            (
                "EstadoDocumento.ts",
                EstadoDocumento::export_to_string().unwrap(),
            ),
            (
                "TextoDocumento.ts",
                TextoDocumento::export_to_string().unwrap(),
            ),
            (
                "ResultadoDocumento.ts",
                ResultadoDocumento::export_to_string().unwrap(),
            ),
            (
                "AssinaturaVisual.ts",
                visao::AssinaturaVisual::export_to_string().unwrap(),
            ),
            (
                "ResultadoVisual.ts",
                visao::ResultadoVisual::export_to_string().unwrap(),
            ),
        ] {
            println!("SIED_CONTRATO_INICIO:{nome}");
            print!("{conteudo}");
            println!("\nSIED_CONTRATO_FIM:{nome}");
        }
    }
}
