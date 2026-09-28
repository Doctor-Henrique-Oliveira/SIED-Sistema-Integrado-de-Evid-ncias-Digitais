//! Modelo de domínio do SIED.
//!
//! Este crate não faz E/S: só define os tipos e as regras de validação.
//! Armazenamento (`sied-armazenamento`), ingestão (`sied-ingestao`) e UI dependem dele;
//! ele não depende de nenhum deles.
//!
//! Tipos marcados com `#[derive(TS)]` são exportados para TypeScript
//! (`app/src/api/tipos/`) ao rodar `cargo test`, mantendo frontend e backend alinhados.

pub mod artefato;
pub mod caso;
pub mod dispositivo;
pub mod ids;
pub mod validacao;

pub use caso::{Caso, NovoCaso};
pub use dispositivo::{Dispositivo, NovoDispositivo, SistemaOperacional};
pub use ids::{CasoId, DispositivoId};
pub use validacao::{ErroCampo, ErrosValidacao};
