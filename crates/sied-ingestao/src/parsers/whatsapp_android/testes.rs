//! Testes do parser WhatsApp sobre bancos **SINTÉTICOS** criados a partir de
//! `testes/fixtures/whatsapp/*.sql`. Nenhum dado é real.

use super::*;
use crate::fonte::CaminhoFonte;
use crate::parser::{ArquivoDeTrabalho, AvisoParser};
use serde_json::json;
use sied_dominio::artefato::{Confianca, FormatoData, NovoArtefato, TipoArtefato};
use std::path::{Path, PathBuf};

const NOVO: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testes/fixtures/whatsapp/msgstore_novo.sql"
));
const ANTIGO: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testes/fixtures/whatsapp/msgstore_antigo.sql"
));
const WA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testes/fixtures/whatsapp/wa.sql"
));

const ANA: &str = "5511900000011@s.whatsapp.net";
const BRUNO: &str = "5511900000012@s.whatsapp.net";
const CARLA: &str = "5521900000013@s.whatsapp.net";
const GRUPO: &str = "120363000000000001@g.us";

#[derive(Default)]
struct Coletor {
    artefatos: Vec<NovoArtefato>,
    avisos: Vec<AvisoParser>,
}

impl SaidaParser for Coletor {
    fn emitir(&mut self, a: NovoArtefato) -> Resultado<()> {
        self.artefatos.push(a);
        Ok(())
    }
    fn avisar(&mut self, a: AvisoParser) {
        self.avisos.push(a);
    }
}

impl Coletor {
    fn do_tipo(&self, t: TipoArtefato) -> Vec<&NovoArtefato> {
        self.artefatos.iter().filter(|a| a.tipo == t).collect()
    }
    fn mensagem(&self, key_id: &str) -> &NovoArtefato {
        self.artefatos
            .iter()
            .find(|a| a.tipo == TipoArtefato::Mensagem && a.id_externo.as_deref() == Some(key_id))
            .unwrap_or_else(|| panic!("mensagem {key_id} não emitida"))
    }
    fn com_id(&self, t: TipoArtefato, id: &str) -> &NovoArtefato {
        self.artefatos
            .iter()
            .find(|a| a.tipo == t && a.id_externo.as_deref() == Some(id))
            .unwrap_or_else(|| panic!("{t:?} {id} não emitido"))
    }
}

fn banco(dir: &Path, nome: &str, sql: &str) -> PathBuf {
    let p = dir.join(nome);
    rusqlite::Connection::open(&p)
        .unwrap()
        .execute_batch(sql)
        .unwrap();
    p
}

fn arquivo(papel: &str, origem: &str, copia: PathBuf) -> ArquivoDeTrabalho {
    ArquivoDeTrabalho {
        papel: papel.into(),
        origem: CaminhoFonte::novo(origem).unwrap(),
        sha256: "sintetico".into(),
        copia_verificada: copia,
    }
}

fn entrada(arquivos: Vec<ArquivoDeTrabalho>) -> EntradaParser {
    EntradaParser {
        arquivos,
        fuso_horario: "America/Sao_Paulo".into(),
    }
}

fn rodar(e: &EntradaParser) -> (Coletor, ResumoParser) {
    let mut c = Coletor::default();
    let r = ParserWhatsAppAndroid.processar(e, &mut c).unwrap();
    (c, r)
}

fn completo(dir: &Path) -> EntradaParser {
    entrada(vec![
        arquivo(
            PAPEL_MSGSTORE,
            "dados/com.whatsapp/databases/msgstore.db",
            banco(dir, "msgstore.db", NOVO),
        ),
        arquivo(
            PAPEL_WA,
            "dados/com.whatsapp/databases/wa.db",
            banco(dir, "wa.db", WA),
        ),
    ])
}

#[test]
fn esquema_novo_emite_conversas_mensagens_e_contatos() {
    let d = tempfile::tempdir().unwrap();
    let (c, r) = rodar(&completo(d.path()));
    assert_eq!(c.do_tipo(TipoArtefato::Conversa).len(), 3);
    assert_eq!(c.do_tipo(TipoArtefato::Mensagem).len(), 13);
    assert_eq!(c.do_tipo(TipoArtefato::Contato).len(), 4);
    assert_eq!(r.artefatos, 20);
    assert!(c.avisos.is_empty(), "avisos inesperados: {:?}", c.avisos);
    assert!(c
        .artefatos
        .iter()
        .all(|a| a.app.as_deref() == Some("whatsapp")));

    // Recebida em conversa individual: remetente é o jid da conversa.
    let m = c.mensagem("SINT0000000001");
    assert_eq!(m.subtipo.as_deref(), Some("texto"));
    assert_eq!(m.de_mim, Some(false));
    assert_eq!(m.remetente.as_deref(), Some(ANA));
    assert_eq!(m.conversa_id_externo.as_deref(), Some(ANA));
    assert_eq!(
        m.conteudo.as_deref(),
        Some("Oi! Mensagem sintética do WhatsApp")
    );
    let dh = m.data_hora.as_ref().unwrap();
    assert_eq!(dh.utc_ms, Some(1767355200000));
    assert_eq!(dh.formato, FormatoData::UnixMilissegundos);
    assert_eq!(m.origem.tabela.as_deref(), Some("message"));
    assert_eq!(m.origem.id_registro.as_deref(), Some("_id=1"));
    assert_eq!(m.origem.valores_brutos["key_id"], json!("SINT0000000001"));
    assert_eq!(m.origem.valores_brutos["sort_id"], json!(1)); // coluna não interpretada, preservada
    assert_eq!(m.metadados["esquema"], json!("message"));

    // Enviada: destinatário é a conversa; situação pelo status.
    let m = c.mensagem("SINT0000000002");
    assert_eq!(m.de_mim, Some(true));
    assert_eq!(m.remetente, None);
    assert_eq!(m.destinatarios, vec![ANA.to_string()]);
    assert_eq!(m.metadados["situacao"], json!("lida"));

    // Imagem: legenda, caminho do app, tipo, tamanho e a linha de message_media.
    let m = c.mensagem("SINT0000000003");
    assert_eq!(m.subtipo.as_deref(), Some("imagem"));
    assert_eq!(m.conteudo.as_deref(), Some("Foto sintética"));
    assert_eq!(
        m.anexo_caminho.as_deref(),
        Some("Media/WhatsApp Images/IMG-20260102-WA0001.jpg")
    );
    assert_eq!(m.nome_arquivo.as_deref(), Some("IMG-20260102-WA0001.jpg"));
    assert_eq!(m.mime.as_deref(), Some("image/jpeg"));
    assert_eq!(m.tamanho, Some(64));
    assert!(m
        .origens_adicionais
        .iter()
        .any(|o| o.tabela.as_deref() == Some("message_media")
            && o.id_registro.as_deref() == Some("message_row_id=3")));

    // Resposta: aponta o key_id da mensagem citada e guarda o texto copiado pelo app.
    let m = c.mensagem("SINT0000000004");
    assert_eq!(m.resposta_a_id_externo.as_deref(), Some("SINT0000000003"));
    assert_eq!(m.metadados["citacao"]["texto"], json!("Foto sintética"));

    // Localização.
    let m = c.mensagem("SINT0000000005");
    assert_eq!(m.subtipo.as_deref(), Some("localizacao"));
    assert_eq!(m.latitude, Some(-23.55052));
    assert_eq!(m.longitude, Some(-46.633308));
    assert_eq!(m.conteudo.as_deref(), Some("Local sintético"));

    // Apagada para todos.
    let m = c.mensagem("SINT0000000006");
    assert_eq!(m.subtipo.as_deref(), Some("apagada"));
    assert_eq!(m.metadados["apagada_para_todos"], json!(true));

    // Cartão de contato.
    let m = c.mensagem("SINT0000000007");
    assert_eq!(m.subtipo.as_deref(), Some("contato"));
    assert!(m.metadados["vcards"][0]
        .as_str()
        .unwrap()
        .contains("FN:Contato Sintético"));

    // Código desconhecido: não é inventado, fica bruto e com confiança média.
    let m = c.mensagem("SINT0000000008");
    assert_eq!(m.subtipo.as_deref(), Some("desconhecido"));
    assert_eq!(m.metadados["tipo_mensagem"], json!(99));
    assert_eq!(m.confianca, Confianca::Media);

    // Grupo: remetente pelo sender_jid_row_id; evento de sistema; documento; encaminhada.
    let m = c.mensagem("SINT0000000009");
    assert_eq!(m.subtipo.as_deref(), Some("sistema"));
    assert_eq!(m.metadados["acao_sistema"], json!(11));
    let m = c.mensagem("SINT0000000010");
    assert_eq!(m.conversa_id_externo.as_deref(), Some(GRUPO));
    assert_eq!(m.remetente.as_deref(), Some(BRUNO));
    let m = c.mensagem("SINT0000000011");
    assert_eq!(m.remetente.as_deref(), Some(CARLA));
    assert_eq!(m.subtipo.as_deref(), Some("documento"));
    assert_eq!(m.nome_arquivo.as_deref(), Some("relatorio-sintetico.pdf"));
    let m = c.mensagem("SINT0000000012");
    assert_eq!(m.de_mim, Some(true));
    assert!(m.destinatarios.is_empty());
    assert_eq!(m.metadados["encaminhada"], json!(true));

    // Registro da conversa do grupo: título, tipo e membros, com as linhas de origem.
    let g = c.com_id(TipoArtefato::Conversa, GRUPO);
    assert_eq!(g.conteudo.as_deref(), Some("Grupo sintético de teste"));
    assert_eq!(g.subtipo.as_deref(), Some("grupo"));
    assert_eq!(g.conversa_id_externo, None, "não pode contar como mensagem");
    assert_eq!(g.metadados["participantes"].as_array().unwrap().len(), 2);
    assert_eq!(g.origem.tabela.as_deref(), Some("chat"));
    let tabelas: Vec<_> = g
        .origens_adicionais
        .iter()
        .filter_map(|o| o.tabela.as_deref())
        .collect();
    assert_eq!(
        tabelas,
        [
            "jid",
            "group_participant_user",
            "jid",
            "group_participant_user",
            "jid"
        ]
    );
    let ind = c.com_id(TipoArtefato::Conversa, ANA);
    assert_eq!(ind.subtipo.as_deref(), Some("individual"));
    assert_eq!(ind.conteudo, None);

    // Contatos: nome da agenda preferido; sem ele, nome de perfil, dizendo a fonte.
    let ana = c.com_id(TipoArtefato::Contato, ANA);
    assert_eq!(ana.conteudo.as_deref(), Some("Ana Sintética"));
    assert_eq!(ana.metadados["fonte_do_nome"], json!("agenda"));
    assert_eq!(ana.metadados["nome_perfil"], json!("Ana S."));
    let carla = c.com_id(TipoArtefato::Contato, CARLA);
    assert_eq!(carla.conteudo.as_deref(), Some("Carla (perfil sintético)"));
    assert_eq!(carla.metadados["fonte_do_nome"], json!("perfil_whatsapp"));
    assert_eq!(carla.origem.tabela.as_deref(), Some("wa_contacts"));
    assert_eq!(
        c.com_id(TipoArtefato::Contato, GRUPO).subtipo.as_deref(),
        Some("grupo")
    );
}

#[test]
fn esquema_antigo_emite_conversas_e_mensagens() {
    let d = tempfile::tempdir().unwrap();
    let e = entrada(vec![arquivo(
        PAPEL_MSGSTORE,
        "msgstore.db",
        banco(d.path(), "msgstore.db", ANTIGO),
    )]);
    let (c, r) = rodar(&e);
    assert_eq!(c.do_tipo(TipoArtefato::Conversa).len(), 2);
    assert_eq!(c.do_tipo(TipoArtefato::Mensagem).len(), 8);
    assert_eq!(r.artefatos, 10);
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);

    let m = c.mensagem("ANTIGO000000001");
    assert_eq!(m.remetente.as_deref(), Some(ANA));
    assert_eq!(m.metadados["esquema"], json!("messages"));
    assert_eq!(m.origem.tabela.as_deref(), Some("messages"));

    let m = c.mensagem("ANTIGO000000002");
    assert_eq!(m.resposta_a_id_externo.as_deref(), Some("ANTIGO000000001"));
    assert_eq!(m.destinatarios, vec![ANA.to_string()]);
    assert_eq!(m.metadados["situacao"], json!("lida"));

    // Imagem: legenda da própria linha, caminho de message_media.
    let m = c.mensagem("ANTIGO000000003");
    assert_eq!(m.subtipo.as_deref(), Some("imagem"));
    assert_eq!(m.conteudo.as_deref(), Some("Imagem sintética antiga"));
    assert_eq!(
        m.anexo_caminho.as_deref(),
        Some("Media/WhatsApp Images/IMG-20260102-WA0009.jpg")
    );
    assert_eq!(m.tamanho, Some(64));

    let m = c.mensagem("ANTIGO000000004");
    assert_eq!(m.latitude, Some(-22.906847));
    assert_eq!(m.conteudo.as_deref(), Some("Local sintético"));

    assert_eq!(
        c.mensagem("ANTIGO000000005").subtipo.as_deref(),
        Some("apagada")
    );
    // status 6 = mensagem de controle/sistema, mesmo com media_wa_type 0
    assert_eq!(
        c.mensagem("ANTIGO000000006").subtipo.as_deref(),
        Some("sistema")
    );
    let m = c.mensagem("ANTIGO000000007");
    assert_eq!(m.remetente.as_deref(), Some(CARLA));
    assert_eq!(m.conversa_id_externo.as_deref(), Some(GRUPO));
    assert_eq!(
        c.mensagem("ANTIGO000000008").metadados["encaminhada"],
        json!(true)
    );

    let g = c.com_id(TipoArtefato::Conversa, GRUPO);
    assert_eq!(
        g.conteudo.as_deref(),
        Some("Grupo sintético (esquema antigo)")
    );
    assert_eq!(g.metadados["participantes"].as_array().unwrap().len(), 2);
    assert_eq!(g.origens_adicionais.len(), 2);
}

#[test]
fn le_mensagem_que_so_existe_no_wal() {
    let d = tempfile::tempdir().unwrap();
    let origem = d.path().join("origem");
    std::fs::create_dir(&origem).unwrap();
    let db = origem.join("msgstore.db");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA journal_mode=WAL; {NOVO}
         PRAGMA wal_checkpoint(TRUNCATE);
         PRAGMA wal_autocheckpoint=0;
         INSERT INTO message (_id, chat_row_id, from_me, key_id, sender_jid_row_id, status,
                              timestamp, message_type, text_data)
         VALUES (50, 1, 0, 'SINTWAL0000001', 0, 0, 1767356400000, 0,
                 'Mensagem só no WAL (sintética)');"
    ))
    .unwrap();
    // Copia banco e -wal com a conexão aberta (WAL não consolidado), como na extração.
    let copia = d.path().join("copia");
    std::fs::create_dir(&copia).unwrap();
    std::fs::copy(&db, copia.join("msgstore.db")).unwrap();
    std::fs::copy(
        origem.join("msgstore.db-wal"),
        copia.join("msgstore.db-wal"),
    )
    .unwrap();
    drop(conn);

    let e = entrada(vec![arquivo(
        PAPEL_MSGSTORE,
        "msgstore.db",
        copia.join("msgstore.db"),
    )]);
    let (c, _) = rodar(&e);
    let m = c.mensagem("SINTWAL0000001");
    assert_eq!(
        m.conteudo.as_deref(),
        Some("Mensagem só no WAL (sintética)")
    );
    assert_eq!(c.do_tipo(TipoArtefato::Mensagem).len(), 14);
}

#[test]
fn copias_nao_sao_alteradas_pela_leitura() {
    let d = tempfile::tempdir().unwrap();
    let e = completo(d.path());
    let antes: Vec<_> = e
        .arquivos
        .iter()
        .map(|a| std::fs::read(&a.copia_verificada).unwrap())
        .collect();
    rodar(&e);
    for (a, b) in e.arquivos.iter().zip(antes) {
        assert_eq!(std::fs::read(&a.copia_verificada).unwrap(), b);
    }
}

#[test]
fn recusa_banco_sem_esquema_conhecido() {
    let d = tempfile::tempdir().unwrap();
    let e = entrada(vec![arquivo(
        PAPEL_MSGSTORE,
        "msgstore.db",
        banco(
            d.path(),
            "x.db",
            "CREATE TABLE messages(_id, key_remote_jid);",
        ),
    )]);
    let r = ParserWhatsAppAndroid.processar(&e, &mut Coletor::default());
    match r {
        Err(ErroIngestao::FormatoNaoReconhecido(m)) => {
            assert!(m.contains("'message' ausente"), "{m}");
            assert!(m.contains("key_from_me"), "{m}");
        }
        outro => panic!("esperava formato não reconhecido: {outro:?}"),
    }
}

#[test]
fn prefere_esquema_novo_salvo_se_estiver_vazio() {
    let d = tempfile::tempdir().unwrap();
    // Os dois esquemas no mesmo banco, com `message` vazia: vale `messages`.
    let sql = format!(
        "{ANTIGO}
         CREATE TABLE message (_id INTEGER PRIMARY KEY, chat_row_id INTEGER, from_me INTEGER,
                               key_id TEXT, timestamp INTEGER);"
    );
    let e = entrada(vec![arquivo(
        PAPEL_MSGSTORE,
        "msgstore.db",
        banco(d.path(), "a.db", &sql),
    )]);
    let (c, _) = rodar(&e);
    assert_eq!(c.do_tipo(TipoArtefato::Mensagem).len(), 8);
    assert_eq!(
        c.do_tipo(TipoArtefato::Mensagem)[0].metadados["esquema"],
        json!("messages")
    );

    // Com linhas nas duas, vale `message`.
    let sql = format!(
        "{NOVO}
         CREATE TABLE messages (_id INTEGER PRIMARY KEY, key_remote_jid TEXT, key_from_me INTEGER,
                                key_id TEXT, timestamp INTEGER);
         INSERT INTO messages VALUES (1, 'x@s.whatsapp.net', 0, 'LEGADO', 1);"
    );
    let e = entrada(vec![arquivo(
        PAPEL_MSGSTORE,
        "msgstore.db",
        banco(d.path(), "b.db", &sql),
    )]);
    let (c, _) = rodar(&e);
    assert_eq!(c.do_tipo(TipoArtefato::Mensagem).len(), 13);
    assert!(c
        .do_tipo(TipoArtefato::Mensagem)
        .iter()
        .all(|m| m.metadados["esquema"] == json!("message")));
}

#[test]
fn so_wa_db_emite_apenas_contatos() {
    let d = tempfile::tempdir().unwrap();
    let e = entrada(vec![arquivo(
        PAPEL_WA,
        "wa.db",
        banco(d.path(), "wa.db", WA),
    )]);
    let (c, _) = rodar(&e);
    assert_eq!(c.artefatos.len(), 4);
    assert!(c.artefatos.iter().all(|a| a.tipo == TipoArtefato::Contato));
}

#[test]
fn wa_db_ilegivel_vira_aviso_e_mensagens_continuam() {
    let d = tempfile::tempdir().unwrap();
    let e = entrada(vec![
        arquivo(
            PAPEL_MSGSTORE,
            "msgstore.db",
            banco(d.path(), "msgstore.db", NOVO),
        ),
        arquivo(
            PAPEL_WA,
            "wa.db",
            banco(d.path(), "wa.db", "CREATE TABLE outra(x);"),
        ),
    ]);
    let (c, r) = rodar(&e);
    assert_eq!(c.do_tipo(TipoArtefato::Mensagem).len(), 13);
    assert_eq!(c.avisos.len(), 1);
    assert!(c.avisos[0].mensagem.contains("wa_contacts"));
    assert_eq!(r.avisos, 1);
}

#[test]
fn mensagem_de_grupo_sem_remetente_vira_aviso() {
    let d = tempfile::tempdir().unwrap();
    let sql = format!(
        "{NOVO}
         INSERT INTO message (_id, chat_row_id, from_me, key_id, sender_jid_row_id, status,
                              timestamp, message_type, text_data)
         VALUES (60, 2, 0, 'SINTSEMREM0001', 77, 0, 1767356500000, 0, 'Sem remetente');"
    );
    let e = entrada(vec![arquivo(
        PAPEL_MSGSTORE,
        "msgstore.db",
        banco(d.path(), "m.db", &sql),
    )]);
    let (c, _) = rodar(&e);
    let m = c.mensagem("SINTSEMREM0001");
    assert_eq!(m.remetente, None);
    assert_eq!(m.confianca, Confianca::Media);
    assert_eq!(c.avisos.len(), 1);
    assert_eq!(c.avisos[0].id_registro.as_deref(), Some("_id=60"));
}

#[test]
fn sem_arquivos_de_entrada_e_erro() {
    let r = ParserWhatsAppAndroid.processar(&entrada(vec![]), &mut Coletor::default());
    assert!(matches!(r, Err(ErroIngestao::Parser { .. })));
}

/// "Prefira não sei a inventar": coordenada fora do intervalo (ou infinita) não é
/// interpretada; o valor continua nos valores brutos da origem.
#[test]
fn coordenadas_invalidas_nao_sao_interpretadas() {
    let dir = tempfile::tempdir().unwrap();
    let e = completo(dir.path());
    let msgstore = &e.arquivos[0].copia_verificada;
    let c = rusqlite::Connection::open(msgstore).unwrap();
    for (lat, lon) in [("123.0", "-46.6"), ("9e999", "-46.6"), ("-23.5", "NULL")] {
        c.execute_batch(&format!(
            "UPDATE message_location SET latitude = {lat}, longitude = {lon} WHERE message_row_id = 5"
        ))
        .unwrap();
        let (col, _) = rodar(&e);
        let m = col.mensagem("SINT0000000005");
        assert_eq!((m.latitude, m.longitude), (None, None), "{lat} {lon}");
        assert_eq!(
            m.metadados["coordenadas_invalidas"],
            json!(true),
            "{lat} {lon}"
        );
        let bruto = &m
            .origens_adicionais
            .iter()
            .find(|o| o.tabela.as_deref() == Some("message_location"))
            .unwrap()
            .valores_brutos;
        assert_ne!(
            bruto["latitude"],
            json!(null),
            "{lat}: o valor bruto fica preservado"
        );
    }
}
