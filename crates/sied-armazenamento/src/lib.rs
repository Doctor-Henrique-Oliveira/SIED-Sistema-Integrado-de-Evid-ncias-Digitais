//! Armazenamento local do SIED.
//!
//! Dois tipos de banco SQLite, cada um com seu próprio conjunto de migrações:
//!
//! - **Global** (`sied-global.db`, na pasta de dados do aplicativo): catálogo de casos
//!   conhecidos por esta estação e configurações. Nunca guarda dados de casos.
//! - **Caso** (`caso.sied.db`, dentro da pasta do caso): tudo do caso — procedimento,
//!   dispositivos, extrações importadas (inventário, artefatos e origem) e auditoria.
//!   É autocontido: copiar a pasta do caso leva todos os dados junto.

pub mod agenda;
pub mod auditoria;
pub mod busca;
pub mod caso;
mod conexao;
pub use conexao::{normalizar_busca, so_digitos};
pub mod conversas;
pub mod derivados;
pub mod descoberta;
pub mod erro;
pub mod evidencias;
pub mod extracoes;
pub mod global;
pub mod importacao;
pub mod linha_tempo;
pub mod migracoes;
pub mod pesquisa_local;
pub mod relatorios;
pub mod resumo;
pub mod sessao;

pub use caso::BancoCaso;
pub use erro::{ErroArmazenamento, Resultado};
pub use global::{CasoCatalogado, CatalogoGlobal};
pub use sessao::Sessao;
