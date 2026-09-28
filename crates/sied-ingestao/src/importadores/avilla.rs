//! Importador de extrações do Avilla Forensics.
//!
//! **Somente contrato.** A estrutura de pastas que o Avilla gera ainda não foi
//! documentada com uma extração real, e não será presumida. Até lá o importador se
//! registra no SIED, mas responde `NaoImplementado` a qualquer fonte.
//!
//! Quando a árvore de uma extração real for fornecida, `detectar` e `planejar` serão
//! implementados a partir dela e documentados em `docs/formatos/avilla.md`.

use crate::fonte::FonteSomenteLeitura;
use crate::importador::{Deteccao, Importador, PlanoImportacao, Suporte};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "avilla";

const MOTIVO_PENDENTE: &str =
    "estrutura das extrações do Avilla Forensics ainda não documentada; aguardando extração de teste real";

pub struct AvillaImporter;

impl Importador for AvillaImporter {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "Avilla Forensics".into(),
            versao: "0.0.0".into(),
        }
    }

    fn detectar(&self, _fonte: &dyn FonteSomenteLeitura) -> Resultado<Deteccao> {
        Ok(Deteccao {
            importador: self.info(),
            confianca: 0.0,
            suporte: Suporte::NaoImplementado,
            motivos: vec![MOTIVO_PENDENTE.into()],
        })
    }

    fn planejar(&self, _fonte: &dyn FonteSomenteLeitura) -> Resultado<PlanoImportacao> {
        Err(ErroIngestao::NaoImplementado {
            componente: "AvillaImporter".into(),
            motivo: MOTIVO_PENDENTE.into(),
        })
    }
}
