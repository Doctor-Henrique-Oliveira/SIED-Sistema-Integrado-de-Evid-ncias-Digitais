//! SMS do Android: tabela `sms` do `mmssms.db` (provedor de telefonia do AOSP).
//!
//! Colunas usadas: `_id`, `thread_id`, `address`, `date` (ms UTC), `date_sent`,
//! `type` (1 recebida, 2 enviada, 3 rascunho, 4 caixa de saída, 5 falha, 6 na fila),
//! `body`, `read`, `seen`, `status`. A linha inteira vai para `valores_brutos`.
//! Os MMS do mesmo banco (tabelas `pdu`, `addr`, `part`) são lidos por
//! [`super::android_mms`].

use serde_json::{json, Map, Value};
use sied_dominio::artefato::{
    DataHora, FormatoData, NovoArtefato, ReferenciaRegistro, TipoArtefato,
};

use super::sqlite::{abrir_copia, colunas, linha_bruta};
use crate::parser::{AvisoParser, EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "android_sms";
pub const PAPEL: &str = "mmssms";
/// Valor de `app` das mensagens (SMS e MMS dividem as conversas).
pub const APP: &str = "sms";
const CONSULTA: &str = "SELECT * FROM sms ORDER BY _id";
const OBRIGATORIAS: &[&str] = &["_id", "address", "date", "type", "body"];

pub struct ParserSmsAndroid;

fn descricao_tipo(t: i64) -> &'static str {
    match t {
        1 => "recebida",
        2 => "enviada",
        3 => "rascunho",
        4 => "caixa_de_saida",
        5 => "falha_no_envio",
        6 => "na_fila",
        _ => "desconhecido",
    }
}

fn inteiro(m: &Map<String, Value>, c: &str) -> Option<i64> {
    m.get(c).and_then(Value::as_i64)
}

fn texto(m: &Map<String, Value>, c: &str) -> Option<String> {
    match m.get(c) {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

impl Parser for ParserSmsAndroid {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "SMS (Android)".into(),
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
        let cols = colunas(&conn, "sms")?;
        if cols.is_empty() {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{}: tabela 'sms' não encontrada",
                arq.origem
            )));
        }
        let faltando: Vec<_> = OBRIGATORIAS
            .iter()
            .filter(|c| !cols.iter().any(|x| x == *c))
            .collect();
        if !faltando.is_empty() {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{}: tabela 'sms' sem as colunas {faltando:?}",
                arq.origem
            )));
        }

        let mut resumo = ResumoParser::default();
        let mut stmt = conn.prepare(CONSULTA)?;
        let mut linhas = stmt.query([])?;
        while let Some(r) = linhas.next()? {
            if resumo.artefatos % 1000 == 0 && saida.cancelado() {
                return Err(ErroIngestao::Cancelado);
            }
            let brutos = linha_bruta(r, &cols)?;
            let id = brutos.get("_id").cloned().unwrap_or(Value::Null);
            let id_registro = format!("_id={id}");
            let Some(tipo) = inteiro(&brutos, "type") else {
                saida.avisar(AvisoParser {
                    mensagem: "coluna 'type' vazia ou não numérica; registro ignorado".into(),
                    papel_arquivo: Some(PAPEL.into()),
                    id_registro: Some(id_registro),
                });
                resumo.avisos += 1;
                continue;
            };
            let endereco = texto(&brutos, "address");

            let mut a = NovoArtefato::novo(
                TipoArtefato::Mensagem,
                ReferenciaRegistro {
                    papel_arquivo: PAPEL.into(),
                    tabela: Some("sms".into()),
                    id_registro: Some(id_registro.clone()),
                    consulta: Some(CONSULTA.into()),
                    deslocamento: None,
                    valores_brutos: brutos.clone(),
                },
            );
            a.subtipo = Some("sms".into());
            a.app = Some(APP.into());
            a.id_externo = Some(id.to_string());
            a.conversa_id_externo = inteiro(&brutos, "thread_id").map(|t| t.to_string());
            a.conteudo = texto(&brutos, "body");
            let recebida = tipo == 1;
            a.de_mim = match tipo {
                1 => Some(false),
                2..=6 => Some(true),
                _ => None,
            };
            if recebida {
                a.remetente = endereco;
            } else if let Some(e) = endereco {
                a.destinatarios.push(e);
            }
            a.data_hora = match brutos.get("date") {
                Some(Value::Number(n)) => Some(DataHora {
                    utc_ms: n.as_i64().and_then(DataHora::ms_plausivel),
                    original: n.to_string(),
                    formato: FormatoData::UnixMilissegundos,
                }),
                Some(v) if !v.is_null() => {
                    saida.avisar(AvisoParser {
                        mensagem: format!("coluna 'date' não numérica: {v}"),
                        papel_arquivo: Some(PAPEL.into()),
                        id_registro: Some(id_registro.clone()),
                    });
                    resumo.avisos += 1;
                    None
                }
                _ => None,
            };
            let mut meta = Map::new();
            meta.insert("tipo_android".into(), json!(tipo));
            meta.insert("situacao".into(), json!(descricao_tipo(tipo)));
            for c in [
                "read",
                "seen",
                "status",
                "date_sent",
                "sub_id",
                "service_center",
            ] {
                if let Some(v) = brutos.get(c).filter(|v| !v.is_null()) {
                    meta.insert(c.into(), v.clone());
                }
            }
            a.metadados = meta;
            saida.emitir(a)?;
            resumo.artefatos += 1;
        }
        Ok(resumo)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fonte::CaminhoFonte;
    use crate::parser::ArquivoDeTrabalho;

    struct Nada;
    impl SaidaParser for Nada {
        fn emitir(&mut self, _: NovoArtefato) -> Resultado<()> {
            Ok(())
        }
        fn avisar(&mut self, _: AvisoParser) {}
    }

    fn entrada(sql: &str) -> (tempfile::TempDir, EntradaParser) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("x.db");
        rusqlite::Connection::open(&db)
            .unwrap()
            .execute_batch(sql)
            .unwrap();
        let e = EntradaParser {
            arquivos: vec![ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: CaminhoFonte::novo("x.db").unwrap(),
                sha256: "sintetico".into(),
                copia_verificada: db,
            }],
            fuso_horario: "America/Sao_Paulo".into(),
        };
        (dir, e)
    }

    #[test]
    fn recusa_banco_sem_tabela_ou_colunas() {
        let (_d, e) = entrada("CREATE TABLE outra(x);");
        let r = ParserSmsAndroid.processar(&e, &mut Nada);
        assert!(
            matches!(r, Err(ErroIngestao::FormatoNaoReconhecido(m)) if m.contains("não encontrada"))
        );
        let (_d, e) = entrada("CREATE TABLE sms(_id, address, date);");
        let r = ParserSmsAndroid.processar(&e, &mut Nada);
        assert!(matches!(r, Err(ErroIngestao::FormatoNaoReconhecido(m)) if m.contains("type")));
    }

    struct Guarda(Vec<NovoArtefato>);
    impl SaidaParser for Guarda {
        fn emitir(&mut self, a: NovoArtefato) -> Resultado<()> {
            self.0.push(a);
            Ok(())
        }
        fn avisar(&mut self, _: AvisoParser) {}
    }

    /// Data absurda (lixo ou valor hostil) não vira data: fica só o valor original.
    #[test]
    fn data_fora_do_intervalo_fica_so_no_original() {
        let (_d, e) = entrada(
            "CREATE TABLE sms(_id INTEGER PRIMARY KEY, address, date, type, body);
             INSERT INTO sms VALUES (1, '1', 9000000000000000, 1, 'sintético');
             INSERT INTO sms VALUES (2, '1', 9223372036854775807, 1, 'sintético');
             INSERT INTO sms VALUES (3, '1', 1767225600000, 1, 'sintético');",
        );
        let mut g = Guarda(Vec::new());
        ParserSmsAndroid.processar(&e, &mut g).unwrap();
        let datas: Vec<_> = g.0.iter().map(|a| a.data_hora.clone().unwrap()).collect();
        assert_eq!(datas[0].utc_ms, None);
        assert_eq!(datas[0].original, "9000000000000000");
        assert_eq!(datas[1].utc_ms, None);
        assert_eq!(datas[1].original, "9223372036854775807");
        assert_eq!(datas[2].utc_ms, Some(1_767_225_600_000));
    }

    #[test]
    fn copia_nao_e_alterada_pela_leitura() {
        let (d, e) = entrada(
            "CREATE TABLE sms(_id INTEGER PRIMARY KEY, address, date, type, body);
             INSERT INTO sms VALUES (1, '1', 1, 1, 'sintético');",
        );
        let antes = std::fs::read(d.path().join("x.db")).unwrap();
        let r = ParserSmsAndroid.processar(&e, &mut Nada).unwrap();
        assert_eq!(r.artefatos, 1);
        assert_eq!(std::fs::read(d.path().join("x.db")).unwrap(), antes);
    }
}
