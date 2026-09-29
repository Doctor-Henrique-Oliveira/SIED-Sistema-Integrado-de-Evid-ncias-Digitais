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
    /// Registro da conversa no próprio app (título, tipo, membros do grupo). As
    /// mensagens continuam agrupadas pelo `conversa_id_externo`; este registro só
    /// acrescenta o que o app guarda sobre a conversa.
    Conversa,
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

impl DataHora {
    /// Último milissegundo de 31/12/9999 UTC. Acima disso a data não é exibível (o
    /// JavaScript não representa depois de 275760 e as bibliotecas de data recusam).
    pub const UTC_MS_MAXIMO: i64 = 253_402_300_799_999;

    /// `utc_ms` só para valores plausíveis (depois de 1970 e até 9999). Fora disso o
    /// parser deixa `utc_ms` vazio e guarda o valor em `original`: prefira "não sei" a
    /// inventar uma data.
    pub fn ms_plausivel(ms: i64) -> Option<i64> {
        (ms > 0 && ms <= Self::UTC_MS_MAXIMO).then_some(ms)
    }
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

    /// `hash_registro`: SHA-256 (hex) do JSON do artefato como o parser o emitiu
    /// (serde_json compacto, campos na ordem da declaração, chaves dos mapas em ordem
    /// alfabética). É gravado na importação e **recalculado a partir do banco do caso**
    /// na geração do relatório, para provar que o conteúdo não mudou. Mudar esta regra
    /// (ou os campos de [`NovoArtefato`]) faz todo registro já importado divergir.
    pub fn hash_registro(&self) -> String {
        use sha2::{Digest, Sha256};
        let json = serde_json::to_vec(self).expect("artefato sempre serializável em JSON");
        hex::encode(Sha256::digest(json))
    }

    /// Ajusta o que o banco do caso não preservaria, para que o artefato gravado seja
    /// idêntico ao artefato do hash: uma coluna REAL do SQLite guarda -0.0 como 0; e
    /// app vazio vira ausente (a conversa derivada não distingue os dois, e as
    /// consultas de conversa contam com isso). Chamado pelo pipeline antes do hash.
    pub fn normalizar(&mut self) {
        if self.app.as_deref() == Some("") {
            self.app = None;
        }
        for v in [&mut self.latitude, &mut self.longitude]
            .into_iter()
            .flatten()
        {
            if *v == 0.0 {
                *v = 0.0;
            }
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

    #[test]
    fn hash_registro_estavel_e_sensivel_ao_conteudo() {
        let origem = ReferenciaRegistro {
            papel_arquivo: "msgstore".into(),
            tabela: Some("message".into()),
            id_registro: Some("_id=1".into()),
            consulta: None,
            deslocamento: None,
            valores_brutos: Map::new(),
        };
        let mut a = NovoArtefato::novo(TipoArtefato::Mensagem, origem);
        a.conteudo = Some("texto sintético".into());
        // Regra fixa: JSON e hash conhecidos (conferível com `sha256sum`). Se este teste
        // falhar, todos os registros já importados passariam a divergir no relatório.
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"tipo":"mensagem","subtipo":null,"app":null,"conta":null,"remetente":null,"destinatarios":[],"de_mim":null,"data_hora":null,"conteudo":"texto sintético","id_externo":null,"resposta_a_id_externo":null,"conversa_id_externo":null,"anexo_caminho":null,"mime":null,"nome_arquivo":null,"tamanho":null,"latitude":null,"longitude":null,"estado":"ativo","confianca":"alta","metadados":{},"origem":{"papel_arquivo":"msgstore","tabela":"message","id_registro":"_id=1","consulta":null,"deslocamento":null,"valores_brutos":{}},"origens_adicionais":[]}"#
        );
        assert_eq!(
            a.hash_registro(),
            "ff919c01ed7f39482603b371537b7455fc106031e3495732fa4aadef7a83a5f7"
        );
        let mut b = a.clone();
        b.conteudo = Some("texto sintético!".into());
        assert_ne!(a.hash_registro(), b.hash_registro());
    }

    /// Metadados e valores brutos ficam no banco do caso como JSON e são relidos para
    /// recalcular o hash: todo número real precisa voltar idêntico, bit a bit
    /// (recurso `float_roundtrip` do serde_json, ativado no Cargo.toml da raiz).
    #[test]
    fn numeros_reais_voltam_identicos_do_json() {
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut valores = vec![0.1 + 0.2, 5e-324, f64::MAX, f64::MIN_POSITIVE, -1.5e-300];
        for _ in 0..200_000 {
            // xorshift64*: sequência fixa, sem dependência de aleatoriedade externa.
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            let f = f64::from_bits(x.wrapping_mul(0x2545_F491_4F6C_DD1D));
            if f.is_finite() {
                valores.push(f);
            }
        }
        for f in valores {
            let txt = serde_json::to_string(&f).unwrap();
            let volta: f64 = serde_json::from_str(&txt).unwrap();
            assert_eq!(f.to_bits(), volta.to_bits(), "{txt}");
        }
    }

    #[test]
    fn data_plausivel_so_entre_1970_e_9999() {
        assert_eq!(
            DataHora::ms_plausivel(1_767_225_600_000),
            Some(1_767_225_600_000)
        );
        assert_eq!(DataHora::ms_plausivel(0), None);
        assert_eq!(DataHora::ms_plausivel(-1), None);
        assert_eq!(
            DataHora::ms_plausivel(DataHora::UTC_MS_MAXIMO),
            Some(DataHora::UTC_MS_MAXIMO)
        );
        assert_eq!(DataHora::ms_plausivel(DataHora::UTC_MS_MAXIMO + 1), None);
        assert_eq!(DataHora::ms_plausivel(i64::MAX), None);
    }

    #[test]
    fn normalizar_troca_zero_negativo_e_app_vazio() {
        let origem = ReferenciaRegistro {
            papel_arquivo: "x".into(),
            tabela: None,
            id_registro: None,
            consulta: None,
            deslocamento: None,
            valores_brutos: Map::new(),
        };
        let mut a = NovoArtefato::novo(TipoArtefato::Localizacao, origem);
        a.latitude = Some(-0.0);
        a.longitude = Some(-23.5);
        a.app = Some(String::new());
        a.normalizar();
        assert!(a.latitude.unwrap().is_sign_positive());
        assert_eq!(a.longitude, Some(-23.5));
        assert_eq!(a.app, None);
        a.app = Some("sms".into());
        a.normalizar();
        assert_eq!(a.app.as_deref(), Some("sms"));
    }
}
