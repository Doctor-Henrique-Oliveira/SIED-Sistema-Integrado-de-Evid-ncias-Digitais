//! Parsers embutidos (um por aplicativo/tipo de artefato).

pub mod android_chamadas;
pub mod android_contatos;
pub mod android_mms;
pub mod android_sms;
pub(crate) mod comum;
pub mod iped_lista;
mod sqlite;
pub mod whatsapp_android;
