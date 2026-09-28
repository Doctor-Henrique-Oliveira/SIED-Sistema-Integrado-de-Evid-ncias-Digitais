//! Contratos de ingestão do SIED.
//!
//! Princípio fundamental: **importador ≠ parser**.
//!
//! - [`Importador`](importador::Importador): entende como uma *ferramenta* (Avilla, IPED, ADB...)
//!   organiza a saída. Detecta a ferramenta, e diz quais arquivos cada parser deve ler.
//! - [`Parser`](parser::Parser): entende um *aplicativo/artefato* (WhatsApp, SMS, contatos...).
//!   Recebe arquivos já identificados e emite artefatos normalizados. Não sabe de qual
//!   ferramenta os arquivos vieram.
//!
//! Toda leitura da extração passa por [`FonteSomenteLeitura`](fonte::FonteSomenteLeitura),
//! que não tem métodos de escrita. Parsers nunca recebem o arquivo original: recebem o
//! caminho de uma cópia de trabalho já conferida por hash.
//!
//! Os tipos de entrada e saída de parser são serializáveis ([`protocolo`]), para que um
//! parser em outro processo/linguagem (Python, Java...) possa ser plugado no futuro
//! sem mudar o restante do pipeline.

pub mod erro;
pub mod fonte;
pub mod importador;
pub mod importadores;
pub mod parser;
pub mod protocolo;
pub mod registro;

pub use erro::{ErroIngestao, Resultado};
pub use registro::Registro;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Identificação de um importador ou parser, gravada na rastreabilidade de cada artefato.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InfoComponente {
    /// Identificador estável (ex.: "avilla", "whatsapp_android").
    pub id: String,
    /// Nome para exibição.
    pub nome: String,
    /// Versão semântica; muda sempre que a interpretação dos dados mudar.
    pub versao: String,
}
