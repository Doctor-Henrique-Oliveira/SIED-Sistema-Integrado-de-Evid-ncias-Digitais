//! Contrato dos parsers (um por aplicativo/tipo de artefato).

use serde::{Deserialize, Serialize};
use sied_dominio::artefato::NovoArtefato;
use std::path::PathBuf;
use ts_rs::TS;

use crate::fonte::CaminhoFonte;
use crate::{InfoComponente, Resultado};

/// Arquivo entregue ao parser: sempre uma cópia de trabalho já conferida por hash,
/// nunca o original da extração.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArquivoDeTrabalho {
    /// Papel pedido pelo importador (ex.: "msgstore").
    pub papel: String,
    /// Onde o arquivo está na extração (endereço estável da evidência).
    pub origem: CaminhoFonte,
    /// SHA-256 do original, igual ao da cópia.
    pub sha256: String,
    /// Cópia na área de trabalho do caso. Arquivos auxiliares do SQLite (-wal, -shm)
    /// são copiados ao lado dela com o mesmo nome base.
    #[ts(type = "string")]
    pub copia_verificada: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EntradaParser {
    pub arquivos: Vec<ArquivoDeTrabalho>,
    /// Fuso do caso, para parsers que leem datas em hora local.
    pub fuso_horario: String,
}

impl EntradaParser {
    pub fn arquivo(&self, papel: &str) -> Option<&ArquivoDeTrabalho> {
        self.arquivos.iter().find(|a| a.papel == papel)
    }
}

/// Problema não fatal encontrado durante o parsing (registro ilegível, coluna ausente...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AvisoParser {
    pub mensagem: String,
    pub papel_arquivo: Option<String>,
    pub id_registro: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResumoParser {
    #[ts(type = "number")]
    pub artefatos: u64,
    #[ts(type = "number")]
    pub avisos: u64,
}

/// Destino dos artefatos. O pipeline grava em lote no banco do caso e completa a
/// referência de origem com caminho, ferramenta, hash, parser e versão.
pub trait SaidaParser {
    fn emitir(&mut self, artefato: NovoArtefato) -> Resultado<()>;
    fn avisar(&mut self, aviso: AvisoParser);
    /// Verificado periodicamente; o parser deve parar cedo se a importação for cancelada.
    fn cancelado(&self) -> bool {
        false
    }
}

pub trait Parser: Send + Sync {
    fn info(&self) -> InfoComponente;

    /// Papéis de arquivo que este parser aceita (ex.: ["msgstore", "wa"]).
    fn papeis(&self) -> &'static [&'static str];

    /// Processa os arquivos e emite artefatos. Um registro ilegível vira aviso,
    /// não interrompe o processamento; erro só quando nada pode ser lido.
    fn processar(
        &self,
        entrada: &EntradaParser,
        saida: &mut dyn SaidaParser,
    ) -> Resultado<ResumoParser>;
}
