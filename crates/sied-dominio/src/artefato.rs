//! Contrato do modelo universal de artefatos.
//!
//! Todo dado que o SIED mostra (mensagem, contato, chamada, mídia, localização...)
//! é um artefato normalizado, qualquer que seja a ferramenta de origem.
//! Um artefato **sempre** carrega a referência ao registro de origem: não há como
//! construir [`NovoArtefato`] sem uma [`ReferenciaRegistro`].
//!
//! Nesta etapa só existem os tipos; tabelas e gravação chegam com o primeiro importador.
//! Os tipos são serializáveis para que parsers externos (outro processo, outra linguagem)
//! possam emitir artefatos em JSON no futuro.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TipoArtefato {
    Mensagem,
    Contato,
    Chamada,
    Midia,
    Documento,
    Localizacao,
    Conta,
    EventoNavegador,
    Arquivo,
    Outro,
}

/// Situação do registro na fonte.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EstadoRegistro {
    #[default]
    Ativo,
    Apagado,
    /// Encontrado apenas no WAL/journal, não no banco principal.
    RecuperadoWal,
    Desconhecido,
}

/// Grau de confiança do parser na interpretação do registro.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Confianca {
    #[default]
    Alta,
    Media,
    Baixa,
}

/// Como o valor de data estava codificado na fonte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FormatoData {
    UnixSegundos,
    UnixMilissegundos,
    /// Microssegundos desde 1601-01-01 (Chrome/WebKit).
    Webkit,
    /// Segundos desde 2001-01-01 (Apple).
    Cocoa,
    Texto,
}

/// Data preservando o valor original e a conversão feita.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DataHora {
    /// UTC em milissegundos, usado para ordenar e filtrar.
    #[ts(type = "number | null")]
    pub utc_ms: Option<i64>,
    /// Valor exatamente como estava na fonte.
    pub original: String,
    pub formato: FormatoData,
}

/// Onde, dentro do arquivo de origem, o dado foi lido.
///
/// O parser preenche isto. O pipeline completa com o que o parser não sabe
/// (caminho original, ferramenta, hash do arquivo, parser e versão) ao gravar
/// a `referencia_origem`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReferenciaRegistro {
    /// Papel do arquivo na entrada do parser (ex.: "msgstore", "wa", "anexo").
    pub papel_arquivo: String,
    /// Tabela, quando a origem é um banco.
    pub tabela: Option<String>,
    /// Identificador do registro na origem (ex.: "_id=48213" ou "rowid=10").
    pub id_registro: Option<String>,
    /// Consulta usada para ler o registro, para reprodução por terceiros.
    pub consulta: Option<String>,
    /// Deslocamento em bytes, para dados recuperados fora de tabelas.
    #[ts(type = "number | null")]
    pub deslocamento: Option<u64>,
    /// Valores brutos relevantes, coluna a coluna, como estavam na fonte.
    #[ts(type = "Record<string, unknown>")]
    pub valores_brutos: Map<String, Value>,
}

/// Artefato normalizado emitido por um parser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NovoArtefato {
    pub tipo: TipoArtefato,
    pub subtipo: Option<String>,
    pub app: Option<String>,
    pub conta: Option<String>,
    pub remetente: Option<String>,
    pub destinatarios: Vec<String>,
    pub de_mim: Option<bool>,
    pub data_hora: Option<DataHora>,
    pub conteudo: Option<String>,
    /// Identificador externo (ex.: key_id da mensagem), para vincular respostas.
    pub id_externo: Option<String>,
    pub resposta_a_id_externo: Option<String>,
    pub conversa_id_externo: Option<String>,
    /// Caminho, dentro da extração, da mídia/documento anexado.
    pub anexo_caminho: Option<String>,
    pub mime: Option<String>,
    pub nome_arquivo: Option<String>,
    #[ts(type = "number | null")]
    pub tamanho: Option<u64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub estado: EstadoRegistro,
    pub confianca: Confianca,
    #[ts(type = "Record<string, unknown>")]
    pub metadados: Map<String, Value>,
    /// Referência principal (obrigatória) ao registro de origem.
    pub origem: ReferenciaRegistro,
    /// Referências adicionais (anexo, contato resolvido em outro banco etc.).
    pub origens_adicionais: Vec<ReferenciaRegistro>,
}

impl NovoArtefato {
    /// Único construtor: exige tipo e referência de origem.
    pub fn novo(tipo: TipoArtefato, origem: ReferenciaRegistro) -> Self {
        NovoArtefato {
            tipo,
            subtipo: None,
            app: None,
            conta: None,
            remetente: None,
            destinatarios: Vec::new(),
            de_mim: None,
            data_hora: None,
            conteudo: None,
            id_externo: None,
            resposta_a_id_externo: None,
            conversa_id_externo: None,
            anexo_caminho: None,
            mime: None,
            nome_arquivo: None,
            tamanho: None,
            latitude: None,
            longitude: None,
            estado: EstadoRegistro::Ativo,
            confianca: Confianca::Alta,
            metadados: Map::new(),
            origem,
            origens_adicionais: Vec::new(),
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn artefato_ida_e_volta_json() {
        let mut brutos = Map::new();
        brutos.insert("_id".into(), json!(48213));
        let mut a = NovoArtefato::novo(
            TipoArtefato::Mensagem,
            ReferenciaRegistro {
                papel_arquivo: "msgstore".into(),
                tabela: Some("message".into()),
                id_registro: Some("_id=48213".into()),
                consulta: None,
                deslocamento: None,
                valores_brutos: brutos,
            },
        );
        a.conteudo = Some("texto sintético".into());
        let txt = serde_json::to_string(&a).unwrap();
        let volta: NovoArtefato = serde_json::from_str(&txt).unwrap();
        assert_eq!(a, volta);
    }
}
