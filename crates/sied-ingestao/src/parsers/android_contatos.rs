//! Agenda de contatos do Android: `contacts2.db` do provedor de contatos (AOSP).
//!
//! Um registro por **contato bruto** (`raw_contacts`): é assim que o Android guarda a
//! agenda, uma linha por conta que sincroniza o contato (Google, aparelho, WhatsApp...).
//! O agrupamento que o Android faz na tela (`contacts`) não é refeito: o identificador
//! do contato agrupado vai nos metadados (`contato_id`) e a linha dele entra como
//! origem adicional.
//!
//! Tabelas lidas (documentação pública `ContactsContract`):
//! - `raw_contacts`: `_id`, `contact_id`, `account_id` (ou `account_name`/`account_type`
//!   nas versões antigas), `deleted`, `display_name`, `starred`, `times_contacted`,
//!   `last_time_contacted`;
//! - `accounts`: `account_name`, `account_type` (conta que sincroniza o contato);
//! - `contacts`: contato agrupado (`contact_last_updated_timestamp`);
//! - `data` + `mimetypes`: telefones (`phone_v2`: `data1` número, `data2` tipo, `data3`
//!   rótulo, `data4` número normalizado), e-mails, nome estruturado, apelido, empresa,
//!   observação, endereço, site; outros tipos (inclusive de apps, como o perfil do
//!   WhatsApp) vão em `outros_dados` com o mimetype e os campos `data1`–`data3`.
//!
//! Cada linha de `data` interpretada entra como origem adicional, com a linha inteira.
//! A foto (`photo`) não é interpretada nem copiada para os valores brutos (é uma
//! imagem em BLOB); o id da linha fica em `foto_registro`. `deleted_contacts` e a
//! tabela `calls` (registro de chamadas das versões antigas, lido por
//! `android_chamadas`) não são lidos aqui.

use serde_json::{json, Map, Value};
use sied_dominio::artefato::{DataHora, EstadoRegistro, NovoArtefato, TipoArtefato};
use std::collections::{HashMap, HashSet};

use super::comum::{faltando, inteiro, referencia, texto, Auxiliar, Emissor, Linha};
use super::sqlite::{abrir_copia, colunas, linha_bruta};
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "android_contatos";
pub const PAPEL: &str = "contacts2";
/// Valor de `app` dos registros emitidos.
pub const APP: &str = "contatos_android";
const CONSULTA: &str = "SELECT * FROM raw_contacts ORDER BY _id";

const MIME: &str = "vnd.android.cursor.item/";

pub struct ParserContatosAndroid;

/// `data2` de um telefone (`Phone.TYPE_*`).
fn tipo_telefone(t: i64) -> Option<&'static str> {
    Some(match t {
        0 => "personalizado",
        1 => "residencial",
        2 => "celular",
        3 => "trabalho",
        4 => "fax_trabalho",
        5 => "fax_residencial",
        6 => "pager",
        7 => "outro",
        8 => "retorno",
        9 => "carro",
        10 => "empresa_principal",
        11 => "isdn",
        12 => "principal",
        13 => "outro_fax",
        14 => "radio",
        15 => "telex",
        16 => "tty_tdd",
        17 => "celular_trabalho",
        18 => "pager_trabalho",
        19 => "assistente",
        20 => "mms",
        _ => return None,
    })
}

/// `data2` de um e-mail (`Email.TYPE_*`).
fn tipo_email(t: i64) -> Option<&'static str> {
    Some(match t {
        0 => "personalizado",
        1 => "residencial",
        2 => "trabalho",
        3 => "outro",
        4 => "celular",
        _ => return None,
    })
}

/// Endereço, e-mail, telefone...: valor em `data1`, tipo em `data2`, rótulo em `data3`.
fn item(l: &Linha, chave: &str, tipo_nome: fn(i64) -> Option<&'static str>) -> Option<Value> {
    let valor = texto(l, "data1")?;
    let mut m = Map::new();
    m.insert(chave.into(), json!(valor));
    if let Some(t) = inteiro(l, "data2") {
        m.insert("tipo".into(), json!(t));
        if let Some(n) = tipo_nome(t) {
            m.insert("tipo_nome".into(), json!(n));
        }
    }
    if let Some(r) = texto(l, "data3") {
        m.insert("rotulo".into(), json!(r));
    }
    if inteiro(l, "is_primary") == Some(1) {
        m.insert("principal".into(), json!(true));
    }
    Some(Value::Object(m))
}

fn id_registro(l: &Linha) -> Option<String> {
    l.get("_id")
        .filter(|v| !v.is_null())
        .map(|v| format!("_id={v}"))
}

impl Parser for ParserContatosAndroid {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "Agenda de contatos (Android)".into(),
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
        let cols = colunas(&conn, "raw_contacts")?;
        if cols.is_empty() {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{}: tabela 'raw_contacts' não encontrada",
                arq.origem
            )));
        }
        for (tabela, obrigatorias) in [
            ("raw_contacts", &["_id"][..]),
            (
                "data",
                &["_id", "raw_contact_id", "mimetype_id", "data1"][..],
            ),
            ("mimetypes", &["_id", "mimetype"][..]),
        ] {
            let falta = faltando(&colunas(&conn, tabela)?, obrigatorias);
            if !falta.is_empty() {
                return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                    "{}: tabela '{tabela}' ausente ou sem as colunas {falta:?}",
                    arq.origem
                )));
            }
        }

        let mut mimetypes: HashMap<i64, String> = HashMap::new();
        {
            let mut stmt = conn.prepare("SELECT _id, mimetype FROM mimetypes")?;
            let mut rs = stmt.query([])?;
            while let Some(r) = rs.next()? {
                if let (Ok(id), Ok(m)) = (r.get::<_, i64>(0), r.get::<_, String>(1)) {
                    mimetypes.insert(id, m);
                }
            }
        }
        let contas = Auxiliar::carregar(&conn, "accounts", "_id")?;
        let agrupados = Auxiliar::carregar(&conn, "contacts", "_id")?;
        let dados = Auxiliar::carregar(&conn, "data", "raw_contact_id")?;

        let mut e = Emissor::novo(saida);
        let mut vistos: HashSet<i64> = HashSet::new();
        let mut stmt = conn.prepare(CONSULTA)?;
        let mut rs = stmt.query([])?;
        while let Some(r) = rs.next()? {
            let l = linha_bruta(r, &cols)?;
            let mut origem = referencia(PAPEL, "raw_contacts", "_id", &l);
            origem.consulta = Some(CONSULTA.into());
            let Some(id) = inteiro(&l, "_id") else {
                e.avisar(
                    "contato sem '_id' numérico; registro ignorado".into(),
                    PAPEL,
                    origem.id_registro,
                );
                continue;
            };
            vistos.insert(id);

            let mut a = NovoArtefato::novo(TipoArtefato::Contato, origem);
            a.app = Some(APP.into());
            a.subtipo = Some("contato".into());
            a.id_externo = Some(id.to_string());
            if inteiro(&l, "deleted") == Some(1) {
                // Apagado no aparelho, ainda não removido pela sincronização da conta.
                a.estado = EstadoRegistro::Apagado;
            }
            let mut m = Map::new();
            m.insert("raw_contact_id".into(), json!(id));

            // Conta que sincroniza o contato.
            let conta = match inteiro(&l, "account_id").and_then(|c| contas.primeira(c)) {
                Some(c) => {
                    a.origens_adicionais.push(contas.referencia(PAPEL, c));
                    (texto(c, "account_name"), texto(c, "account_type"))
                }
                None => (texto(&l, "account_name"), texto(&l, "account_type")),
            };
            a.conta = conta.0;
            if let Some(t) = conta.1 {
                m.insert("tipo_conta".into(), json!(t));
            }

            if let Some(c) = inteiro(&l, "contact_id") {
                m.insert("contato_id".into(), json!(c));
                if let Some(g) = agrupados.primeira(c) {
                    a.origens_adicionais.push(agrupados.referencia(PAPEL, g));
                    if let Some(ms) = inteiro(g, "contact_last_updated_timestamp")
                        .and_then(DataHora::ms_plausivel)
                    {
                        m.insert("ultima_atualizacao_ms".into(), json!(ms));
                    }
                }
            }
            if let Some(v) = inteiro(&l, "starred") {
                m.insert("favorito".into(), json!(v == 1));
            }
            if let Some(v) = inteiro(&l, "times_contacted").filter(|&v| v > 0) {
                m.insert("vezes_contatado".into(), json!(v));
            }
            if let Some(v) = inteiro(&l, "last_time_contacted").and_then(DataHora::ms_plausivel) {
                m.insert("ultimo_contato_ms".into(), json!(v));
            }

            let mut nome_estruturado = None;
            let mut telefones = Vec::new();
            let mut emails = Vec::new();
            let mut enderecos = Vec::new();
            let mut sites = Vec::new();
            let mut outros = Vec::new();
            let mut linhas: Vec<&Linha> = dados.todas(id).iter().collect();
            linhas.sort_by_key(|d| inteiro(d, "_id"));
            for d in linhas {
                let mime = inteiro(d, "mimetype_id").and_then(|i| mimetypes.get(&i));
                let curto = mime.and_then(|x| x.strip_prefix(MIME));
                if curto == Some("photo") {
                    if let Some(r) = id_registro(d) {
                        m.insert("foto_registro".into(), json!(r));
                    }
                    continue;
                }
                let mut ref_dado = referencia(PAPEL, "data", "_id", d);
                ref_dado.consulta = Some(format!(
                    "SELECT * FROM data WHERE raw_contact_id = {id} ORDER BY _id"
                ));
                match curto {
                    Some("phone_v2") => {
                        if let Some(Value::Object(mut t)) = item(d, "numero", tipo_telefone) {
                            if let Some(n) = texto(d, "data4") {
                                t.insert("normalizado".into(), json!(n));
                            }
                            if let Some(r) = &ref_dado.id_registro {
                                t.insert("registro".into(), json!(r));
                            }
                            telefones.push(Value::Object(t));
                        }
                    }
                    Some("email_v2") => emails.extend(item(d, "endereco", tipo_email)),
                    Some("name") => {
                        nome_estruturado = texto(d, "data1");
                        for (coluna, chave) in [("data2", "nome"), ("data3", "sobrenome")] {
                            if let Some(v) = texto(d, coluna) {
                                m.insert(chave.into(), json!(v));
                            }
                        }
                    }
                    Some("nickname") => {
                        if let Some(v) = texto(d, "data1") {
                            m.insert("apelido".into(), json!(v));
                        }
                    }
                    Some("organization") => {
                        for (coluna, chave) in [("data1", "empresa"), ("data4", "cargo")] {
                            if let Some(v) = texto(d, coluna) {
                                m.insert(chave.into(), json!(v));
                            }
                        }
                    }
                    Some("note") => {
                        if let Some(v) = texto(d, "data1") {
                            m.insert("observacao".into(), json!(v));
                        }
                    }
                    Some("postal-address_v2") => {
                        enderecos.extend(texto(d, "data1").map(Value::from))
                    }
                    Some("website") => sites.extend(texto(d, "data1").map(Value::from)),
                    _ => {
                        let mut o = Map::new();
                        o.insert("mimetype".into(), json!(mime));
                        for c in ["data1", "data2", "data3"] {
                            if let Some(v) = texto(d, c) {
                                o.insert(c.into(), json!(v));
                            }
                        }
                        if mime.is_none() {
                            e.avisar(
                                format!(
                                    "linha de 'data' com mimetype_id {} sem correspondência em \
                                     'mimetypes'; guardada sem interpretação",
                                    d.get("mimetype_id").cloned().unwrap_or(Value::Null)
                                ),
                                PAPEL,
                                ref_dado.id_registro.clone(),
                            );
                        }
                        outros.push(Value::Object(o));
                    }
                }
                a.origens_adicionais.push(ref_dado);
            }
            a.conteudo = texto(&l, "display_name").or(nome_estruturado);
            for (chave, lista) in [
                ("telefones", telefones),
                ("emails", emails),
                ("enderecos", enderecos),
                ("sites", sites),
                ("outros_dados", outros),
            ] {
                if !lista.is_empty() {
                    m.insert(chave.into(), Value::Array(lista));
                }
            }
            a.metadados = m;
            e.emitir(a)?;
        }

        for rc in dados.chaves() {
            if !vistos.contains(&rc) {
                e.avisar(
                    format!(
                        "{} linha(s) de 'data' com raw_contact_id={rc} sem contato em \
                         'raw_contacts'; não interpretadas",
                        dados.todas(rc).len()
                    ),
                    PAPEL,
                    Some(format!("raw_contact_id={rc}")),
                );
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

    const FIXTURE: &str = include_str!("../../../../testes/fixtures/android/contacts2.sql");

    fn entrada(sql: &str) -> (tempfile::TempDir, EntradaParser) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("contacts2.db");
        rusqlite::Connection::open(&db)
            .unwrap()
            .execute_batch(sql)
            .unwrap();
        let e = EntradaParser {
            arquivos: vec![ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: CaminhoFonte::novo("contacts2.db").unwrap(),
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
        ParserContatosAndroid.processar(&e, &mut g).unwrap();
        g
    }

    #[test]
    fn le_a_fixture_sintetica() {
        let g = ler(FIXTURE);
        assert_eq!(g.0.len(), 6);
        let por_id = |id: &str| {
            g.0.iter()
                .find(|a| a.id_externo.as_deref() == Some(id))
                .unwrap()
        };

        let ana = por_id("1");
        assert_eq!(ana.tipo, TipoArtefato::Contato);
        assert_eq!(ana.app.as_deref(), Some(APP));
        assert_eq!(ana.conteudo.as_deref(), Some("Ana Sintética"));
        assert_eq!(ana.conta.as_deref(), Some("analista.sintetico@example.com"));
        assert_eq!(ana.metadados["tipo_conta"], json!("com.google"));
        assert_eq!(ana.metadados["favorito"], json!(true));
        assert_eq!(ana.metadados["vezes_contatado"], json!(5));
        assert_eq!(ana.metadados["contato_id"], json!(1));
        assert_eq!(
            ana.metadados["telefones"],
            json!([{
                "numero": "+55 11 90000-0011", "tipo": 2, "tipo_nome": "celular",
                "principal": true, "normalizado": "+5511900000011", "registro": "_id=2"
            }])
        );
        assert_eq!(
            ana.metadados["emails"],
            json!([{"endereco": "ana.sintetica@example.com", "tipo": 1, "tipo_nome": "residencial"}])
        );
        // A foto não é interpretada nem copiada: fica só a referência da linha.
        assert_eq!(ana.metadados["foto_registro"], json!("_id=4"));
        let tabelas: Vec<_> = ana
            .origens_adicionais
            .iter()
            .map(|o| (o.tabela.clone().unwrap(), o.id_registro.clone().unwrap()))
            .collect();
        assert_eq!(
            tabelas,
            vec![
                ("accounts".into(), "_id=1".into()),
                ("contacts".into(), "_id=1".into()),
                ("data".into(), "_id=1".into()),
                ("data".into(), "_id=2".into()),
                ("data".into(), "_id=3".into()),
            ]
        );

        let bruno = por_id("2");
        let tels = bruno.metadados["telefones"].as_array().unwrap();
        assert_eq!(tels.len(), 2);
        assert_eq!(tels[1]["tipo_nome"], json!("personalizado"));
        assert_eq!(tels[1]["rotulo"], json!("Escritório sintético"));

        let dora = por_id("3");
        assert_eq!(dora.conta, None);
        assert_eq!(dora.metadados["empresa"], json!("Empresa Sintética Ltda"));
        assert_eq!(dora.metadados["cargo"], json!("Analista"));
        assert_eq!(dora.metadados["apelido"], json!("Dorinha"));
        assert_eq!(
            dora.metadados["observacao"],
            json!("Observação sintética de teste")
        );
        assert!(dora.metadados.get("telefones").is_none());

        assert_eq!(por_id("4").estado, EstadoRegistro::Apagado);

        // Contato da conta do WhatsApp: o perfil do app fica em outros_dados.
        let wa = por_id("5");
        assert_eq!(wa.metadados["tipo_conta"], json!("com.whatsapp"));
        assert_eq!(
            wa.metadados["outros_dados"][0]["mimetype"],
            json!("vnd.android.cursor.item/vnd.com.whatsapp.profile")
        );

        // Sem nome: nenhum nome é inventado.
        assert_eq!(por_id("6").conteudo, None);

        // A linha de data sem contato vira aviso.
        assert_eq!(g.1.len(), 1, "{:?}", g.1);
        assert!(g.1[0].mensagem.contains("raw_contact_id=999"));
    }

    #[test]
    fn esquema_antigo_com_conta_na_propria_linha() {
        let g = ler(
            "CREATE TABLE raw_contacts(_id INTEGER PRIMARY KEY, account_name TEXT, account_type TEXT,
                 deleted INTEGER, display_name TEXT);
             CREATE TABLE mimetypes(_id INTEGER PRIMARY KEY, mimetype TEXT);
             CREATE TABLE data(_id INTEGER PRIMARY KEY, raw_contact_id INTEGER, mimetype_id INTEGER,
                 data1 TEXT, data2 TEXT, data3 TEXT, data4 TEXT);
             INSERT INTO mimetypes VALUES (1, 'vnd.android.cursor.item/phone_v2');
             INSERT INTO raw_contacts VALUES (1, 'conta.sintetica', 'com.google', 0, 'Sintético');
             INSERT INTO data VALUES (1, 1, 1, '11900000001', '2', NULL, NULL);
             INSERT INTO data VALUES (2, 1, 7, 'x', NULL, NULL, NULL);",
        );
        assert_eq!(g.0.len(), 1);
        let a = &g.0[0];
        assert_eq!(a.conta.as_deref(), Some("conta.sintetica"));
        assert_eq!(a.metadados["telefones"][0]["numero"], json!("11900000001"));
        // mimetype_id desconhecido: guardado sem interpretação, com aviso.
        assert_eq!(a.metadados["outros_dados"][0]["mimetype"], Value::Null);
        assert_eq!(g.1.len(), 1);
    }

    #[test]
    fn recusa_banco_sem_as_tabelas_da_agenda() {
        let (_d, e) = entrada("CREATE TABLE calls(_id INTEGER PRIMARY KEY);");
        let r = ParserContatosAndroid.processar(&e, &mut Guarda::default());
        assert!(
            matches!(r, Err(ErroIngestao::FormatoNaoReconhecido(m)) if m.contains("raw_contacts"))
        );
        let (_d, e) = entrada("CREATE TABLE raw_contacts(_id INTEGER PRIMARY KEY);");
        let r = ParserContatosAndroid.processar(&e, &mut Guarda::default());
        assert!(matches!(r, Err(ErroIngestao::FormatoNaoReconhecido(m)) if m.contains("'data'")));
    }

    #[test]
    fn copia_nao_e_alterada_pela_leitura() {
        let (d, e) = entrada(FIXTURE);
        let antes = std::fs::read(d.path().join("contacts2.db")).unwrap();
        ParserContatosAndroid
            .processar(&e, &mut Guarda::default())
            .unwrap();
        assert_eq!(std::fs::read(d.path().join("contacts2.db")).unwrap(), antes);
    }
}
