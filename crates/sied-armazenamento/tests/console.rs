//! Console da importação sobre uma extração **SINTÉTICA** (WhatsApp de
//! `caso_sintetico.rs`): os eventos são os do pipeline e reabrem iguais pelo log.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod caso_sintetico;

use caso_sintetico::*;
use sied_ingestao::eventos::NivelEvento;
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::importador::Importador;
use sied_ingestao::importadores::generico::ImportadorGenerico;
use sied_ingestao::pipeline::{self, Controle};
use sied_ingestao::Registro;

const MMS: &str = include_str!("../../../testes/fixtures/android/mms.sql");

fn importar_com_console(
    banco: &sied_armazenamento::BancoCaso,
    d: &sied_dominio::Dispositivo,
    origem: &std::path::Path,
) -> (Controle, sied_armazenamento::extracoes::Extracao) {
    let importador = ImportadorGenerico;
    let mut imp = banco
        .iniciar_importacao(d.id, &importador.info(), origem)
        .unwrap();
    let fonte = PastaSomenteLeitura::abrir(origem).unwrap();
    let ctx = imp.contexto().clone();
    let controle = Controle::default();
    let r = pipeline::executar(
        &fonte,
        &importador,
        &Registro::padrao(),
        &mut imp,
        &controle,
        &ctx,
    );
    (controle, imp.finalizar(&r).unwrap())
}

#[test]
fn avisos_do_parser_aparecem_como_aviso() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    let dir = origem.join("dados/data/com.android.providers.telephony/databases");
    std::fs::create_dir_all(&dir).unwrap();
    let conn = rusqlite::Connection::open(dir.join("mmssms.db")).unwrap();
    conn.execute_batch(
        "CREATE TABLE sms (_id INTEGER PRIMARY KEY, thread_id INTEGER, address TEXT, date INTEGER,
             type INTEGER, body TEXT);",
    )
    .unwrap();
    conn.execute_batch(MMS).unwrap();
    drop(conn);
    let (banco, d) = caso(raiz.path());
    let (controle, x) = importar_com_console(&banco, &d, &origem);
    assert_eq!(x.status, "concluida");
    let ev = controle.eventos_desde(0);
    let aviso = ev
        .iter()
        .find(|e| e.nivel == NivelEvento::Aviso && e.mensagem.starts_with("MMS"))
        .unwrap_or_else(|| panic!("{ev:#?}"));
    assert!(aviso.mensagem.contains("999"), "{aviso:?}");
    let fim = ev.last().unwrap();
    assert_eq!(fim.nivel, NivelEvento::Aviso);
    assert!(fim.mensagem.contains("1 aviso(s)"), "{fim:?}");
}

#[test]
fn eventos_reais_em_ordem_e_relidos_do_log() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    extracao(&origem);
    let antes = retrato(&origem);
    let (banco, d) = caso(raiz.path());
    let (controle, x) = importar_com_console(&banco, &d, &origem);
    assert_eq!(x.status, "concluida");

    let ev = controle.eventos_desde(0);
    let seqs: Vec<u64> = ev.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, (1..=ev.len() as u64).collect::<Vec<_>>());
    assert!(ev.windows(2).all(|w| w[0].ts_ms <= w[1].ts_ms));
    let textos: Vec<&str> = ev.iter().map(|e| e.mensagem.as_str()).collect();
    let tem = |trecho: &str| {
        assert!(
            textos.iter().any(|t| t.contains(trecho)),
            "sem \"{trecho}\" em {textos:#?}"
        )
    };
    assert!(textos[0].starts_with("Iniciando importação com"));
    tem("arquivo(s) encontrados");
    tem("Calculando o SHA-256");
    tem(&format!("Banco SQLite localizado: {BANCOS}/msgstore.db"));
    tem("Inventário concluído");
    tem(&x.hash_manifesto.clone().unwrap());
    tem("Plano do importador");
    tem(&format!("Copiando {BANCOS}/msgstore.db"));
    tem("Cópia conferida: SHA-256 igual ao do inventário");
    tem("sobre a cópia verificada");
    tem("registro(s) gravados e indexados");
    let fim = ev.last().unwrap();
    assert!(fim.mensagem.starts_with("Importação concluída"), "{fim:?}");
    // Sem avisos nem falhas nesta extração: termina em "ok".
    assert_eq!(fim.nivel, NivelEvento::Ok);
    assert!(ev.iter().all(|e| e.nivel != NivelEvento::Erro));

    // Consulta incremental.
    let depois = controle.eventos_desde(3);
    assert_eq!(depois.first().unwrap().seq, 4);
    assert!(controle.eventos_desde(fim.seq).is_empty());

    // Reaberto pelo log: as mesmas mensagens, na mesma ordem.
    let relidos = banco.eventos_da_importacao(x.id).unwrap();
    assert_eq!(
        relidos
            .iter()
            .map(|e| (e.nivel, e.mensagem.clone()))
            .collect::<Vec<_>>(),
        ev.iter()
            .map(|e| (e.nivel, e.mensagem.clone()))
            .collect::<Vec<_>>()
    );
    // Extração inexistente é erro; nada foi escrito na origem.
    assert!(banco
        .eventos_da_importacao(sied_dominio::ExtracaoId::novo())
        .is_err());
    assert_eq!(retrato(&origem), antes);
}
