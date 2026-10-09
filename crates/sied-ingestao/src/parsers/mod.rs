//! Parsers embutidos (um por aplicativo/tipo de artefato).

pub mod android_chamadas;
pub mod android_contatos;
pub mod android_mms;
pub mod android_sms;
pub mod browser_history;
pub(crate) mod comum;
pub mod email_eml;
pub mod ios_contatos;
pub mod ios_sms;
pub mod iped_lista;
pub mod leapp_tsv;
mod sqlite;
pub mod telegram_json;
pub mod whatsapp_android;
