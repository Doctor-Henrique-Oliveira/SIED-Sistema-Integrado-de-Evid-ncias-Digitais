//! Registro de chamadas do Android: tabela `calls` do provedor de contatos (AOSP).
//!
//! Do Android 7 em diante a tabela fica em `calllog.db`; antes, em `contacts2.db`.
//! O parser lê a tabela onde ela estiver: o importador diz qual banco entregar.
//!
//! Colunas usadas (documentação pública `CallLog.Calls`): `_id`, `number`, `date`
//! (ms UTC), `duration` (segundos), `type` (1 recebida, 2 efetuada, 3 perdida,
//! 4 correio de voz, 5 rejeitada, 6 bloqueada, 7 atendida em outro aparelho),
//! `presentation` (1 permitido, 2 restrito, 3 desconhecido, 4 telefone público),
//! `name` (nome que o aparelho guardou junto da chamada, copiado da agenda na hora),
//! `normalized_number`, `formatted_number`, `numbertype`, `numberlabel`, `countryiso`,
//! `geocoded_location`, `features`, `subscription_id`, `subscription_component_name`,
//! `new`, `is_read`, `via_number`, `post_dial_digits`, `last_modified`, `deleted`
//! (linha marcada como apagada pelo provedor → estado "apagado"). A linha
//! inteira vai para `valores_brutos`; colunas ausentes (variam por versão) ficam de
//! fora sem aviso.
//!
//! Nada é deduzido: código de tipo desconhecido vira subtipo "desconhecido" (o valor
//! bruto fica nos metadados), número especial das versões antigas (`-1`, `-2`, `-3`)
//! não vira telefone e data fora do intervalo plausível fica só no valor original.

use serde_json::{json, Map, Value};
use sied_dominio::artefato::{DataHora, EstadoRegistro, FormatoData, NovoArtefato, TipoArtefato};

use super::comum::{faltando, inteiro, referencia, texto, Emissor};
use super::sqlite::{abrir_copia, colunas, linha_bruta};
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "android_chamadas";
/// Banco que contém a tabela `calls` (`calllog.db`, ou `contacts2.db` até o Android 6).
pub const PAPEL: &str = "chamadas";
/// Valor de `app` dos registros emitidos (discador do próprio Android).
pub const APP: &str = "telefone";
const TABELA: &str = "calls";
const CONSULTA: &str = "SELECT * FROM calls ORDER BY _id";
const OBRIGATORIAS: &[&str] = &["_id", "number", "date", "type"];

pub struct ParserChamadasAndroid;

/// `type` → subtipo (constantes `CallLog.Calls.*_TYPE`).
pub fn subtipo_chamada(codigo: i64) -> Option<&'static str> {
    Some(match codigo {
        1 => "recebida",
        2 => "efetuada",
        3 => "perdida",
        4 => "correio_de_voz",
        5 => "rejeitada",
        6 => "bloqueada",
        7 => "atendida_em_outro_aparelho",
        _ => return None,
    })
}

/// `presentation` → como a operadora apresentou o número.
fn apresentacao(codigo: i64) -> Option<&'static str> {
    Some(match codigo {
        1 => "permitido",
        2 => "restrito",
        3 => "desconhecido",
        4 => "telefone_publico",
        _ => return None,
    })
}

/// Bits de `features` (constantes `CallLog.Calls.FEATURES_*`).
const RECURSOS: &[(i64, &str)] = &[
    (1, "video"),
    (2, "transferida_de_outro_aparelho"),
    (4, "hd"),
    (8, "wifi"),
    (16, "discagem_assistida"),
    (32, "rtt"),
    (64, "volte"),
];

/// Versões antigas do Android gravavam códigos no lugar do número quando ele não era
/// apresentado (-1 desconhecido, -2 privado, -3 telefone público). Não são telefones.
fn numero_especial(n: &str) -> bool {
    matches!(n.trim(), "-1" | "-2" | "-3" | "-4")
}

impl Parser for ParserChamadasAndroid {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "Registro de chamadas (Android)".into(),
            versao: "1.0.0".into(),
        }
    }

    fn papeis(&self) -> &'static [&'static str] {
        &[PAPEL]
    }

    fn processar(
        &self,
        entrada: &EntradaParser,
        saida: &mut dyn SaidaParser,
    ) -> Resultado<ResumoParser> {
        let arq = entrada.arquivo(PAPEL).ok_or_else(|| ErroIngestao::Parser {
            parser: ID.into(),
            mensagem: format!("arquivo com papel '{PAPEL}' não informado"),
        })?;
        let conn = abrir_copia(&arq.copia_verificada)?;
        let cols = colunas(&conn, TABELA)?;
        let mut e = Emissor::novo(saida);
        if cols.is_empty() {
            // Caso esperado em contacts2.db do Android 7 ou mais novo, quando o
            // calllog.db não está na extração: é dito, não é falha do banco.
            e.avisar(
                format!(
                    "{}: tabela 'calls' não encontrada; do Android 7 em diante o registro \
                     de chamadas fica em calllog.db, que não foi encontrado junto deste banco",
                    arq.origem
                ),
                PAPEL,
                None,
            );
            return Ok(e.resumo);
        }
        let falta = faltando(&cols, OBRIGATORIAS);
        if !falta.is_empty() {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{}: tabela 'calls' sem as colunas {falta:?}",
                arq.origem
            )));
        }

        let mut stmt = conn.prepare(CONSULTA)?;
        let mut linhas = stmt.query([])?;
        while let Some(r) = linhas.next()? {
            let l = linha_bruta(r, &cols)?;
            let mut origem = referencia(PAPEL, TABELA, "_id", &l);
            origem.consulta = Some(CONSULTA.into());
            let id_registro = origem.id_registro.clone();
            let codigo = inteiro(&l, "type");
            let subtipo = codigo.and_then(subtipo_chamada);
            if codigo.is_none() {
                e.avisar(
                    "coluna 'type' vazia ou não numérica: direção da chamada desconhecida".into(),
                    PAPEL,
                    id_registro.clone(),
                );
            }

            let mut a = NovoArtefato::novo(TipoArtefato::Chamada, origem);
            a.app = Some(APP.into());
            a.subtipo = Some(subtipo.unwrap_or("desconhecido").into());
            a.id_externo = l.get("_id").filter(|v| !v.is_null()).map(|v| match v {
                Value::String(s) => s.clone(),
                outro => outro.to_string(),
            });
            a.de_mim = match codigo {
                Some(2) => Some(true),
                Some(1 | 3..=7) => Some(false),
                _ => None,
            };
            let bruto = texto(&l, "number");
            let numero = bruto.clone().filter(|n| !numero_especial(n));
            if let Some(n) = numero {
                if a.de_mim == Some(true) {
                    a.destinatarios.push(n);
                } else {
                    a.remetente = Some(n);
                }
            }
            if inteiro(&l, "deleted") == Some(1) {
                // O próprio provedor marcou a linha como apagada (aguardando sincronização).
                a.estado = EstadoRegistro::Apagado;
            }
            a.data_hora = match l.get("date") {
                Some(Value::Number(n)) => Some(DataHora {
                    utc_ms: n.as_i64().and_then(DataHora::ms_plausivel),
                    original: n.to_string(),
                    formato: FormatoData::UnixMilissegundos,
                }),
                Some(v) if !v.is_null() => {
                    e.avisar(
                        format!("coluna 'date' não numérica: {v}"),
                        PAPEL,
                        id_registro.clone(),
                    );
                    None
                }
                _ => None,
            };

            let mut m = Map::new();
            if let Some(c) = codigo {
                m.insert("tipo_android".into(), json!(c));
            }
            if let Some(b) = bruto.filter(|n| numero_especial(n)) {
                m.insert("numero_especial".into(), json!(b));
            }
            if let Some(d) = inteiro(&l, "duration") {
                if d >= 0 {
                    m.insert("duracao_s".into(), json!(d));
                } else {
                    e.avisar(
                        format!("coluna 'duration' negativa ({d}): duração não interpretada"),
                        PAPEL,
                        id_registro.clone(),
                    );
                }
            }
            if let Some(p) = inteiro(&l, "presentation") {
                m.insert("apresentacao_android".into(), json!(p));
                if let Some(nome) = apresentacao(p) {
                    m.insert("apresentacao".into(), json!(nome));
                }
            }
            if let Some(f) = inteiro(&l, "features").filter(|&f| f > 0) {
                let recursos: Vec<&str> = RECURSOS
                    .iter()
                    .filter(|(bit, _)| f & bit != 0)
                    .map(|(_, n)| *n)
                    .collect();
                m.insert("recursos".into(), json!(recursos));
            }
            for (coluna, chave) in [
                ("name", "nome_no_registro"),
                ("numberlabel", "rotulo_do_numero"),
                ("normalized_number", "numero_normalizado"),
                ("formatted_number", "numero_formatado"),
                ("matched_number", "numero_correspondente"),
                ("countryiso", "pais"),
                ("geocoded_location", "local_geocodificado"),
                ("subscription_id", "conta_telefonica"),
                ("subscription_component_name", "componente_da_conta"),
                ("phone_account_address", "endereco_da_conta"),
                ("via_number", "via_numero"),
                ("post_dial_digits", "digitos_pos_discagem"),
                ("transcription", "transcricao"),
            ] {
                if let Some(v) = texto(&l, coluna) {
                    m.insert(chave.into(), json!(v));
                }
            }
            for (coluna, chave) in [("numbertype", "tipo_do_numero"), ("new", "nova")] {
                if let Some(v) = inteiro(&l, coluna) {
                    m.insert(chave.into(), json!(v));
                }
            }
            if let Some(v) = inteiro(&l, "is_read") {
                m.insert("lida".into(), json!(v == 1));
            }
            if let Some(v) = inteiro(&l, "last_modified").and_then(DataHora::ms_plausivel) {
                m.insert("modificado_em_ms".into(), json!(v));
            }
            a.metadados = m;
            e.emitir(a)?;
        }
        Ok(e.resumo)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fonte::CaminhoFonte;
    use crate::parser::{ArquivoDeTrabalho, AvisoParser};

    #[derive(Default)]
    struct Guarda(Vec<NovoArtefato>, Vec<AvisoParser>);
    impl SaidaParser for Guarda {
        fn emitir(&mut self, a: NovoArtefato) -> Resultado<()> {
            self.0.push(a);
            Ok(())
        }
        fn avisar(&mut self, a: AvisoParser) {
            self.1.push(a);
        }
    }

    const FIXTURE: &str = include_str!("../../../../testes/fixtures/android/calllog.sql");

    fn entrada(sql: &str) -> (tempfile::TempDir, EntradaParser) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("calllog.db");
        rusqlite::Connection::open(&db)
            .unwrap()
            .execute_batch(sql)
            .unwrap();
        let e = EntradaParser {
            arquivos: vec![ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: CaminhoFonte::novo("calllog.db").unwrap(),
                sha256: "sintetico".into(),
                copia_verificada: db,
            }],
            fuso_horario: "America/Sao_Paulo".into(),
        };
        (dir, e)
    }

    fn ler(sql: &str) -> Guarda {
        let (_d, e) = entrada(sql);
        let mut g = Guarda::default();
        ParserChamadasAndroid.processar(&e, &mut g).unwrap();
        g
    }

    #[test]
    fn le_a_fixture_sintetica() {
        let g = ler(FIXTURE);
        assert_eq!(g.0.len(), 9);
        assert!(g.1.is_empty(), "{:?}", g.1);
        let por_id = |id: &str| {
            g.0.iter()
                .find(|a| a.id_externo.as_deref() == Some(id))
                .unwrap()
        };

        // Recebida de contato da agenda, com nome guardado pelo aparelho.
        let a = por_id("1");
        assert_eq!(a.tipo, TipoArtefato::Chamada);
        assert_eq!(a.app.as_deref(), Some(APP));
        assert_eq!(a.subtipo.as_deref(), Some("recebida"));
        assert_eq!(a.de_mim, Some(false));
        assert_eq!(a.remetente.as_deref(), Some("+5511900000011"));
        assert!(a.destinatarios.is_empty());
        assert_eq!(
            a.data_hora.as_ref().unwrap().utc_ms,
            Some(1_767_258_000_000)
        );
        assert_eq!(a.metadados["duracao_s"], json!(125));
        assert_eq!(a.metadados["nome_no_registro"], json!("Ana Sintética"));
        assert_eq!(a.metadados["apresentacao"], json!("permitido"));
        assert_eq!(a.origem.tabela.as_deref(), Some("calls"));
        assert_eq!(a.origem.id_registro.as_deref(), Some("_id=1"));
        assert_eq!(a.origem.valores_brutos["number"], json!("+5511900000011"));

        // Efetuada: o número vai para destinatários.
        let b = por_id("2");
        assert_eq!(b.subtipo.as_deref(), Some("efetuada"));
        assert_eq!(b.de_mim, Some(true));
        assert_eq!(b.remetente, None);
        assert_eq!(b.destinatarios, vec!["(11) 90000-0012".to_string()]);
        assert_eq!(b.metadados["recursos"], json!(["video", "wifi"]));

        assert_eq!(por_id("3").subtipo.as_deref(), Some("perdida"));
        assert_eq!(por_id("3").metadados["duracao_s"], json!(0));
        assert_eq!(por_id("4").subtipo.as_deref(), Some("rejeitada"));
        assert_eq!(por_id("5").subtipo.as_deref(), Some("bloqueada"));

        // Número restrito: não há telefone para mostrar.
        let r = por_id("6");
        assert_eq!(r.remetente, None);
        assert_eq!(r.metadados["apresentacao"], json!("restrito"));

        // Código desconhecido: subtipo "desconhecido", código bruto preservado.
        let d = por_id("7");
        assert_eq!(d.subtipo.as_deref(), Some("desconhecido"));
        assert_eq!(d.de_mim, None);
        assert_eq!(d.metadados["tipo_android"], json!(99));

        // Número especial das versões antigas não vira telefone.
        let x = por_id("8");
        assert_eq!(x.remetente, None);
        assert_eq!(x.metadados["numero_especial"], json!("-2"));

        let n = por_id("9");
        assert_eq!(n.remetente.as_deref(), Some("071900000017"));
        assert_eq!(n.metadados["numero_normalizado"], json!("+5571900000017"));
        assert_eq!(n.metadados["nova"], json!(1));
        assert_eq!(n.metadados["lida"], json!(false));
    }

    #[test]
    fn contacts2_sem_tabela_calls_gera_aviso_e_nenhum_registro() {
        let g = ler("CREATE TABLE raw_contacts(_id INTEGER PRIMARY KEY);");
        assert!(g.0.is_empty());
        assert_eq!(g.1.len(), 1);
        assert!(g.1[0].mensagem.contains("calllog.db"));
    }

    #[test]
    fn recusa_tabela_sem_colunas_obrigatorias() {
        let (_d, e) = entrada("CREATE TABLE calls(_id INTEGER PRIMARY KEY, number);");
        let r = ParserChamadasAndroid.processar(&e, &mut Guarda::default());
        assert!(matches!(r, Err(ErroIngestao::FormatoNaoReconhecido(m)) if m.contains("date")));
    }

    #[test]
    fn valores_estranhos_viram_aviso_nunca_dado_inventado() {
        let g = ler(
            "CREATE TABLE calls(_id INTEGER PRIMARY KEY, number, date, duration, type);
             INSERT INTO calls VALUES (1, '1', 9223372036854775807, -5, 1);
             INSERT INTO calls VALUES (2, '1', 'ontem', 3, NULL);",
        );
        assert_eq!(g.0.len(), 2);
        let a = &g.0[0];
        assert_eq!(a.data_hora.as_ref().unwrap().utc_ms, None);
        assert_eq!(
            a.data_hora.as_ref().unwrap().original,
            "9223372036854775807"
        );
        assert!(a.metadados.get("duracao_s").is_none());
        let b = &g.0[1];
        assert_eq!(b.data_hora, None);
        assert_eq!(b.subtipo.as_deref(), Some("desconhecido"));
        assert_eq!(b.de_mim, None);
        assert_eq!(g.1.len(), 3, "{:?}", g.1);
    }

    #[test]
    fn copia_nao_e_alterada_pela_leitura() {
        let (d, e) = entrada(FIXTURE);
        let antes = std::fs::read(d.path().join("calllog.db")).unwrap();
        ParserChamadasAndroid
            .processar(&e, &mut Guarda::default())
            .unwrap();
        assert_eq!(std::fs::read(d.path().join("calllog.db")).unwrap(), antes);
    }
}
