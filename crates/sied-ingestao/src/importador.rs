//! Contrato dos importadores (um por ferramenta de extração).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::fonte::{CaminhoFonte, FonteSomenteLeitura};
use crate::{InfoComponente, Resultado};

/// Nível de suporte do importador para a fonte analisada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Suporte {
    /// A fonte foi reconhecida e pode ser importada.
    Completo,
    /// Reconhecida, mas parte do conteúdo não será interpretada.
    Parcial,
    /// A ferramenta é conhecida, mas o importador ainda não foi implementado.
    NaoImplementado,
    /// A fonte não parece ser desta ferramenta.
    NaoReconhecido,
}

/// Resultado da detecção de uma fonte por um importador.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Deteccao {
    pub importador: InfoComponente,
    /// 0.0 a 1.0.
    pub confianca: f32,
    pub suporte: Suporte,
    /// Evidências da detecção, legíveis pelo analista (ex.: "encontrado arquivo X").
    pub motivos: Vec<String>,
}

/// Um arquivo da extração que um parser deve ler, com o papel que ele cumpre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArquivoParaParser {
    /// Papel esperado pelo parser (ex.: "msgstore", "wa", "midia").
    pub papel: String,
    pub caminho: CaminhoFonte,
}

/// Trabalho que o importador pede a um parser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TarefaParser {
    pub parser_id: String,
    pub arquivos: Vec<ArquivoParaParser>,
}

/// Informação do aparelho encontrada na extração, sempre com o arquivo de onde veio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AtributoDetectado {
    /// fabricante, modelo, versao_sistema, imei1, serial, conta...
    pub chave: String,
    pub valor: String,
    pub origem: CaminhoFonte,
}

/// O que o importador descobriu e o que deve ser processado.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlanoImportacao {
    pub ferramenta_versao: Option<String>,
    pub tipo_extracao: Option<String>,
    pub atributos_dispositivo: Vec<AtributoDetectado>,
    pub tarefas: Vec<TarefaParser>,
    /// Observações para o log da importação (ex.: pastas ignoradas e por quê).
    pub observacoes: Vec<String>,
}

/// Importador de uma ferramenta.
///
/// O pipeline (próxima etapa) chama, nesta ordem: `detectar` → inventário e hashes de
/// todos os arquivos (feito pelo pipeline, igual para todas as ferramentas) → `planejar`
/// → execução dos parsers pedidos no plano.
pub trait Importador: Send + Sync {
    fn info(&self) -> InfoComponente;

    /// Examina a fonte e diz se é desta ferramenta. Deve ser rápido: olha só nomes e
    /// alguns arquivos-chave, nunca a extração inteira.
    fn detectar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Deteccao>;

    /// Monta o plano de processamento a partir da fonte já inventariada.
    fn planejar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<PlanoImportacao>;
}
