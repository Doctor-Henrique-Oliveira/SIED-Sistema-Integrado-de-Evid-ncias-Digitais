//! MMS do Android: tabelas `pdu`, `addr` e `part` do `mmssms.db` (provedor de
//! telefonia do AOSP, documentação pública `Telephony.Mms`).
//!
//! Um artefato `mensagem` (app `sms`) por linha de `pdu`, na mesma conversa
//! (`thread_id`) dos SMS: o Android guarda SMS e MMS de um contato no mesmo fio.
//!
//! - `pdu`: `_id`, `thread_id`, `date` e `date_sent` (**segundos** UTC, ao contrário
//!   do `sms`), `msg_box` (1 recebida, 2 enviada, 3 rascunho, 4 caixa de saída,
//!   5 falha), `m_type` (128 envio, 132 recebida, 130 notificação de MMS ainda não
//!   baixado), `sub` (assunto), `m_id`, `m_size`, `read`, `seen`, `exp`, `ct_l`.
//! - `addr` (por `msg_id`): `address` e `type` (137 de, 151 para, 130 cópia,
//!   129 cópia oculta). `insert-address-token` é o próprio aparelho: não é telefone.
//! - `part` (por `mid`): `ct` (tipo), `text` (texto das partes `text/plain`), `_data`
//!   (caminho do arquivo em `app_parts`), `cl`/`name`/`fn` (nome). A parte
//!   `application/smil` é só o layout.
//!
//! Texto = partes de texto na ordem de `seq` (sem texto, o assunto). Anexo = a
//! primeira parte de mídia; todas as partes ficam em `metadados.partes`, e cada
//! linha de `addr` e `part` entra como origem adicional com a linha inteira.
//! Banco sem `pdu` (só SMS): nada a ler, sem aviso.

use serde_json::{json, Map, Value};
use sied_dominio::artefato::{DataHora, FormatoData, NovoArtefato, TipoArtefato};
use std::collections::HashSet;

use super::android_sms;
use super::comum::{faltando, inteiro, referencia, texto, Auxiliar, Emissor, Linha};
use super::sqlite::{abrir_copia, colunas, linha_bruta};
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "android_mms";
/// Mesmo banco e mesmo papel do parser de SMS.
pub const PAPEL: &str = android_sms::PAPEL;
const TABELA: &str = "pdu";
const CONSULTA: &str = "SELECT * FROM pdu ORDER BY _id";
const OBRIGATORIAS: &[&str] = &["_id", "date", "msg_box"];
/// Endereço que o Android grava no lugar do número do próprio aparelho.
const PROPRIO: &str = "insert-address-token";

pub struct ParserMmsAndroid;

fn situacao(caixa: i64) -> &'static str {
    match caixa {
        1 => "recebida",
        2 => "enviada",
        3 => "rascunho",
        4 => "caixa_de_saida",
        5 => "falha_no_envio",
        _ => "desconhecido",
    }
}

fn papel_endereco(t: i64) -> Option<&'static str> {
    Some(match t {
        137 => "de",
        151 => "para",
        130 => "copia",
        129 => "copia_oculta",
        _ => return None,
    })
}

fn e_texto(ct: &str) -> bool {
    ct.eq_ignore_ascii_case("text/plain")
}

fn e_layout(ct: &str) -> bool {
    ct.eq_ignore_ascii_case("application/smil")
}

/// Subtipo da mensagem pela parte de mídia principal.
fn subtipo_midia(ct: &str) -> &'static str {
    let ct = ct.to_ascii_lowercase();
    if ct == "image/gif" {
        "gif"
    } else if ct.starts_with("image/") {
        "imagem"
    } else if ct.starts_with("video/") {
        "video"
    } else if ct.starts_with("audio/") {
        "audio"
    } else if ct == "text/x-vcard" || ct == "text/vcard" {
        "contato"
    } else {
        "documento"
    }
}

fn nome_do_caminho(c: &str) -> Option<String> {
    c.rsplit(['/', '\\'])
        .next()
        .filter(|n| !n.is_empty())
        .map(str::to_string)
}

impl Parser for ParserMmsAndroid {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "MMS (Android)".into(),
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
            return Ok(e.resumo);
        }
        let falta = faltando(&cols, OBRIGATORIAS);
        if !falta.is_empty() {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{}: tabela 'pdu' sem as colunas {falta:?}",
                arq.origem
            )));
        }
        let enderecos = Auxiliar::carregar(&conn, "addr", "msg_id")?;
        let partes = Auxiliar::carregar(&conn, "part", "mid")?;

        let mut lidas = HashSet::new();
        let mut stmt = conn.prepare(CONSULTA)?;
        let mut linhas = stmt.query([])?;
        while let Some(r) = linhas.next()? {
            let l = linha_bruta(r, &cols)?;
            let mut origem = referencia(PAPEL, TABELA, "_id", &l);
            origem.consulta = Some(CONSULTA.into());
            let id_registro = origem.id_registro.clone();
            let id = inteiro(&l, "_id");
            if let Some(i) = id {
                lidas.insert(i);
            }
            let caixa = inteiro(&l, "msg_box");
            let tipo_pdu = inteiro(&l, "m_type");

            let mut a = NovoArtefato::novo(TipoArtefato::Mensagem, origem);
            a.app = Some(android_sms::APP.into());
            a.id_externo = id.map(|i| i.to_string());
            a.conversa_id_externo = inteiro(&l, "thread_id").map(|t| t.to_string());
            a.de_mim = match caixa {
                Some(1) => Some(false),
                Some(2..=5) => Some(true),
                _ => None,
            };

            // Endereços, na ordem do banco.
            let mut por_papel: Map<String, Value> = Map::new();
            let mut de = None;
            for ad in id.map(|i| enderecos.todas(i)).unwrap_or(&[]) {
                a.origens_adicionais.push(enderecos.referencia(PAPEL, ad));
                let (Some(endereco), Some(p)) = (
                    texto(ad, "address"),
                    inteiro(ad, "type").and_then(papel_endereco),
                ) else {
                    continue;
                };
                let proprio = endereco.trim() == PROPRIO;
                let lista = por_papel
                    .entry(p)
                    .or_insert_with(|| Value::Array(vec![]))
                    .as_array_mut()
                    .expect("lista");
                lista.push(json!(if proprio {
                    "(este aparelho)"
                } else {
                    &endereco
                }));
                if proprio {
                    continue;
                }
                if p == "de" {
                    de.get_or_insert(endereco);
                } else {
                    a.destinatarios.push(endereco);
                }
            }
            if a.de_mim == Some(false) {
                a.remetente = de;
            }

            // Partes, na ordem de seq.
            let mut ps: Vec<&Linha> = id.map(|i| partes.todas(i)).unwrap_or(&[]).iter().collect();
            ps.sort_by_key(|p| (inteiro(p, "seq").unwrap_or(0), inteiro(p, "_id")));
            let mut textos = Vec::new();
            let mut lista_partes = Vec::new();
            let mut principal: Option<&Linha> = None;
            for p in &ps {
                a.origens_adicionais.push(partes.referencia(PAPEL, p));
                let ct = texto(p, "ct").unwrap_or_default();
                let caminho = texto(p, "_data");
                let nome = ["cl", "name", "fn"]
                    .iter()
                    .find_map(|c| texto(p, c))
                    .or_else(|| caminho.as_deref().and_then(nome_do_caminho));
                if e_texto(&ct) {
                    if let Some(t) = texto(p, "text") {
                        textos.push(t);
                    }
                } else if !e_layout(&ct) && principal.is_none() {
                    principal = Some(p);
                }
                let mut d = Map::new();
                d.insert(
                    "registro".into(),
                    p.get("_id").cloned().unwrap_or(Value::Null),
                );
                d.insert("seq".into(), p.get("seq").cloned().unwrap_or(Value::Null));
                d.insert("tipo".into(), json!(ct));
                if let Some(n) = nome {
                    d.insert("nome".into(), json!(n));
                }
                if let Some(c) = caminho {
                    d.insert("caminho".into(), json!(c));
                }
                lista_partes.push(Value::Object(d));
            }
            let assunto = texto(&l, "sub");
            a.conteudo = if textos.is_empty() {
                assunto.clone()
            } else {
                Some(textos.join("\n"))
            };
            a.subtipo = Some(
                if tipo_pdu == Some(130) {
                    "mms_nao_baixado"
                } else if let Some(p) = principal {
                    subtipo_midia(&texto(p, "ct").unwrap_or_default())
                } else {
                    "texto"
                }
                .into(),
            );
            if let Some(p) = principal {
                a.mime = texto(p, "ct");
                a.anexo_caminho = texto(p, "_data");
                a.nome_arquivo = ["cl", "name", "fn"]
                    .iter()
                    .find_map(|c| texto(p, c))
                    .or_else(|| a.anexo_caminho.as_deref().and_then(nome_do_caminho));
            }

            a.data_hora = match l.get("date") {
                Some(Value::Number(n)) => Some(DataHora {
                    utc_ms: n
                        .as_i64()
                        .and_then(|s| s.checked_mul(1000))
                        .and_then(DataHora::ms_plausivel),
                    original: n.to_string(),
                    formato: FormatoData::UnixSegundos,
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
            m.insert("formato".into(), json!("mms"));
            if let Some(c) = caixa {
                m.insert("tipo_android".into(), json!(c));
                m.insert("situacao".into(), json!(situacao(c)));
            }
            if let Some(t) = tipo_pdu {
                m.insert("tipo_pdu".into(), json!(t));
            }
            if let Some(s) = assunto {
                m.insert("assunto".into(), json!(s));
            }
            if !por_papel.is_empty() {
                m.insert("enderecos".into(), Value::Object(por_papel));
            }
            if !lista_partes.is_empty() {
                m.insert("partes".into(), Value::Array(lista_partes));
            }
            let midias = ps
                .iter()
                .filter(|p| {
                    let ct = texto(p, "ct").unwrap_or_default();
                    !e_texto(&ct) && !e_layout(&ct)
                })
                .count();
            if midias > 1 {
                m.insert("outras_midias".into(), json!(midias - 1));
            }
            for (coluna, chave) in [("m_id", "mms_id"), ("ct_l", "endereco_do_conteudo")] {
                if let Some(v) = texto(&l, coluna) {
                    m.insert(chave.into(), json!(v));
                }
            }
            for (coluna, chave) in [
                ("m_size", "tamanho_declarado"),
                ("sub_id", "sub_id"),
                ("locked", "bloqueada"),
            ] {
                if let Some(v) = inteiro(&l, coluna) {
                    m.insert(chave.into(), json!(v));
                }
            }
            for (coluna, chave) in [("read", "lida"), ("seen", "vista")] {
                if let Some(v) = inteiro(&l, coluna) {
                    m.insert(chave.into(), json!(v == 1));
                }
            }
            for (coluna, chave) in [("date_sent", "enviada_em_ms"), ("exp", "expira_em_ms")] {
                if let Some(ms) = inteiro(&l, coluna)
                    .filter(|&s| s > 0)
                    .and_then(|s| s.checked_mul(1000))
                    .and_then(DataHora::ms_plausivel)
                {
                    m.insert(chave.into(), json!(ms));
                }
            }
            a.metadados = m;
            e.emitir(a)?;
        }

        for (aux, rotulo) in [(&enderecos, "endereço"), (&partes, "parte")] {
            for chave in aux.chaves() {
                if !lidas.contains(&chave) {
                    for l in aux.todas(chave) {
                        e.avisar(
                            format!(
                                "{rotulo} de MMS sem mensagem correspondente ({} = {chave}); \
                                 não interpretado",
                                aux.chave
                            ),
                            PAPEL,
                            aux.referencia(PAPEL, l).id_registro,
                        );
                    }
                }
            }
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

    const FIXTURE: &str = include_str!("../../../../testes/fixtures/android/mms.sql");
    const SMS: &str = "CREATE TABLE sms (_id INTEGER PRIMARY KEY, thread_id INTEGER, address TEXT,
                           date INTEGER, type INTEGER, body TEXT);";

    fn entrada(sql: &str) -> (tempfile::TempDir, EntradaParser) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("mmssms.db");
        rusqlite::Connection::open(&db)
            .unwrap()
            .execute_batch(sql)
            .unwrap();
        let e = EntradaParser {
            arquivos: vec![ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: CaminhoFonte::novo("mmssms.db").unwrap(),
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
        ParserMmsAndroid.processar(&e, &mut g).unwrap();
        g
    }

    fn por_id<'a>(g: &'a Guarda, id: &str) -> &'a NovoArtefato {
        g.0.iter()
            .find(|a| a.id_externo.as_deref() == Some(id))
            .unwrap()
    }

    #[test]
    fn le_a_fixture_sintetica() {
        let g = ler(&format!("{SMS}{FIXTURE}"));
        assert_eq!(g.0.len(), 5);

        // Recebida com foto e texto, na conversa dos SMS.
        let m1 = por_id(&g, "1");
        assert_eq!(m1.app.as_deref(), Some("sms"));
        assert_eq!(m1.conversa_id_externo.as_deref(), Some("1"));
        assert_eq!(m1.subtipo.as_deref(), Some("imagem"));
        assert_eq!(m1.de_mim, Some(false));
        assert_eq!(m1.remetente.as_deref(), Some("+5511900000001"));
        assert!(
            m1.destinatarios.is_empty(),
            "o próprio aparelho não vira telefone"
        );
        assert_eq!(
            m1.conteudo.as_deref(),
            Some("Foto do local combinado (MMS sintético)")
        );
        assert_eq!(m1.mime.as_deref(), Some("image/png"));
        assert_eq!(m1.nome_arquivo.as_deref(), Some("IMG_0002.png"));
        assert_eq!(
            m1.anexo_caminho.as_deref(),
            Some("/data/user_de/0/com.android.providers.telephony/app_parts/PART_1767232800000_IMG_0002.png")
        );
        let dh = m1.data_hora.as_ref().unwrap();
        assert_eq!(dh.utc_ms, Some(1_767_232_800_000));
        assert_eq!(dh.formato, FormatoData::UnixSegundos);
        assert_eq!(m1.origem.tabela.as_deref(), Some("pdu"));
        assert_eq!(m1.origem.id_registro.as_deref(), Some("_id=1"));
        // 2 endereços + 3 partes (inclusive o layout) como origens adicionais.
        assert_eq!(m1.origens_adicionais.len(), 5);
        assert_eq!(m1.metadados["partes"].as_array().unwrap().len(), 3);
        assert_eq!(
            m1.metadados["enderecos"]["para"],
            json!(["(este aparelho)"])
        );
        assert_eq!(m1.metadados["enviada_em_ms"], json!(1_767_232_790_000_i64));
        assert_eq!(m1.metadados["formato"], json!("mms"));

        // Enviada em grupo, só texto, com assunto.
        let m2 = por_id(&g, "2");
        assert_eq!(m2.de_mim, Some(true));
        assert_eq!(m2.remetente, None);
        assert_eq!(
            m2.destinatarios,
            vec!["+5521900000002", "+5531900000003", "+5541900000015"]
        );
        assert_eq!(
            m2.metadados["enderecos"]["copia"],
            json!(["+5541900000015"])
        );
        assert_eq!(m2.subtipo.as_deref(), Some("texto"));
        assert_eq!(
            m2.conteudo.as_deref(),
            Some("Mensagem em grupo (MMS sintético)")
        );
        assert_eq!(m2.metadados["assunto"], json!("Aviso sintético"));
        assert_eq!(m2.metadados["situacao"], json!("enviada"));
        assert_eq!(m2.anexo_caminho, None);

        // Duas mídias: a primeira (seq) é o anexo; a outra fica listada.
        let m3 = por_id(&g, "3");
        assert_eq!(m3.subtipo.as_deref(), Some("audio"));
        assert_eq!(m3.nome_arquivo.as_deref(), Some("audio_sintetico.amr"));
        assert_eq!(m3.metadados["outras_midias"], json!(1));
        assert!(m3.destinatarios.is_empty());
        assert_eq!(m3.conteudo, None);
        assert_eq!(m3.metadados["lida"], json!(false));

        // Notificação de MMS não baixado: o assunto é o único texto.
        let m4 = por_id(&g, "4");
        assert_eq!(m4.subtipo.as_deref(), Some("mms_nao_baixado"));
        assert_eq!(
            m4.conteudo.as_deref(),
            Some("Mensagem multimídia sintética")
        );
        assert_eq!(
            m4.metadados["endereco_do_conteudo"],
            json!("http://mmsc.invalido/SINTETICO")
        );
        assert_eq!(m4.metadados["expira_em_ms"], json!(1_767_848_400_000_i64));

        // Data absurda: só o valor original.
        let m5 = por_id(&g, "5");
        let dh = m5.data_hora.as_ref().unwrap();
        assert_eq!(dh.utc_ms, None);
        assert_eq!(dh.original, "99999999999999");

        // Parte órfã vira aviso.
        assert_eq!(g.1.len(), 1);
        assert!(g.1[0].mensagem.contains("sem mensagem correspondente"));
        assert_eq!(g.1[0].id_registro.as_deref(), Some("_id=7"));
    }

    #[test]
    fn banco_so_com_sms_nao_tem_nada_a_ler() {
        let g = ler(SMS);
        assert!(g.0.is_empty());
        assert!(g.1.is_empty());
    }

    #[test]
    fn recusa_pdu_sem_colunas_obrigatorias() {
        let (_d, e) = entrada("CREATE TABLE pdu (_id INTEGER PRIMARY KEY, thread_id INTEGER);");
        let erro = ParserMmsAndroid
            .processar(&e, &mut Guarda::default())
            .unwrap_err();
        assert!(erro.to_string().contains("date"), "{erro}");
    }

    #[test]
    fn copia_nao_e_alterada_pela_leitura() {
        let (_d, e) = entrada(&format!("{SMS}{FIXTURE}"));
        let caminho = &e.arquivos[0].copia_verificada;
        let antes = std::fs::read(caminho).unwrap();
        ParserMmsAndroid
            .processar(&e, &mut Guarda::default())
            .unwrap();
        assert_eq!(std::fs::read(caminho).unwrap(), antes);
    }
}
