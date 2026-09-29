//! O `hash_registro` de cada artefato é recalculado a partir do banco do caso (é o que
//! o relatório confere). Extrações **SINTÉTICAS**: bancos dos fixtures inventados e um
//! `mmssms.db` montado aqui com valores hostis. Nada disto é dado real.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use comum::*;
use rusqlite::{params, Connection};
use sied_armazenamento::caso::NOME_ARQUIVO_CASO;
use std::fs;
use std::path::Path;

const ANTIGO: &str = include_str!("../../../testes/fixtures/whatsapp/msgstore_antigo.sql");
const NOVO: &str = include_str!("../../../testes/fixtures/whatsapp/msgstore_novo.sql");

/// `mmssms.db` com textos, números e tipos que costumam quebrar ida e volta.
fn sms_hostil(destino: &Path) {
    fs::create_dir_all(destino.parent().unwrap()).unwrap();
    let c = Connection::open(destino).unwrap();
    c.execute_batch(
        "CREATE TABLE sms(_id INTEGER PRIMARY KEY, thread_id, address, date, type, body, extra_real, extra);",
    )
    .unwrap();
    let mut ins = c
        .prepare("INSERT INTO sms VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7)")
        .unwrap();
    let textos = [
        "emoji 👩‍👩‍👧 e acentuação: ação, ü, ñ",
        "com NUL\u{0}no meio e \u{1}\u{2} controles",
        "\"aspas\" \\ barra #let x = 1 [colchetes] *negrito* $math$ <tag> </tag>",
        "direção \u{202E}invertida\u{202C} e zero-width\u{200B}",
        "",
    ];
    let reais = [0.1 + 0.2, 5e-324, f64::MAX, -1.5e-300, 1.767225600123e12];
    for (i, (t, r)) in textos.iter().zip(reais).enumerate() {
        let data: rusqlite::types::Value = match i {
            0 => 1767225600000i64.into(),
            1 => 1.767225600999e12f64.into(),
            2 => "2026-01-01".to_string().into(),
            3 => rusqlite::types::Value::Null,
            _ => (-62135596800000i64).into(),
        };
        let extra: rusqlite::types::Value = match i {
            0 => vec![0u8, 255, 1].into(),
            1 => i64::MIN.into(),
            2 => i64::MAX.into(),
            _ => rusqlite::types::Value::Null,
        };
        ins.execute(params![
            i as i64 + 1,
            format!("+55 11 9000{i}-000{i}"),
            data,
            [1i64, 2, 99, 5, 6][i],
            t,
            r,
            extra
        ])
        .unwrap();
    }
    // Texto que não é UTF-8 válido (gravado como blob e convertido para TEXT).
    c.execute(
        "INSERT INTO sms VALUES (99, 1, CAST(X'FF00FE' AS TEXT), 1767225600000, 1, CAST(X'C328' AS TEXT), NULL, NULL)",
        [],
    )
    .unwrap();
}

/// Segunda extração: esquema antigo do WhatsApp, localização com -0.0 numa coluna sem
/// tipo (o SQLite preserva o sinal) e o SMS hostil.
fn extracao_hostil(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SINTETICO-LEIAME.txt"),
        "Extração SINTÉTICA de teste.",
    )
    .unwrap();
    let antigo = dir.join("wa-antigo/databases/msgstore.db");
    fs::create_dir_all(antigo.parent().unwrap()).unwrap();
    Connection::open(&antigo)
        .unwrap()
        .execute_batch(ANTIGO)
        .unwrap();
    let novo = dir.join("wa-novo/databases/msgstore.db");
    fs::create_dir_all(novo.parent().unwrap()).unwrap();
    let sql = NOVO
        .replace("latitude REAL", "latitude")
        .replace("longitude REAL", "longitude");
    let c = Connection::open(&novo).unwrap();
    c.execute_batch(&sql).unwrap();
    c.execute(
        "UPDATE message_location SET latitude = ?1, longitude = ?2",
        params![-0.0f64, 0.1f64 + 0.2],
    )
    .unwrap();
    sms_hostil(&dir.join("telefonia/mmssms.db"));
}

#[test]
fn todo_registro_importado_confere_ao_recalcular() {
    let (raiz, _origem, banco, x) = preparado();
    let origem2 = raiz.path().join("Extracao-HOSTIL");
    extracao_hostil(&origem2);
    let d = banco.listar_dispositivos().unwrap().remove(0);
    let x2 = importar(&banco, &d, &origem2);
    assert_eq!(x2.status, "concluida", "{:?}", x2.erro);
    assert!(x.total_artefatos.unwrap() > 10);
    assert!(x2.total_artefatos.unwrap() > 20, "{:?}", x2.total_artefatos);

    let ruins = banco.registros_que_nao_conferem(None).unwrap();
    assert!(ruins.is_empty(), "{ruins:#?}");

    // A localização com -0.0 na origem foi gravada (e entrou no hash) como 0.0.
    let c = Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap();
    let n: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM artefato WHERE extracao_id = ?1 AND latitude = 0",
            [x2.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert!(n >= 1);
}

#[test]
fn alteracao_fora_do_sied_e_detectada() {
    let (_raiz, _origem, banco, _x) = preparado();
    let ids: Vec<i64> = {
        let c = Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap();
        let mut s = c
            .prepare("SELECT id FROM artefato ORDER BY id LIMIT 5")
            .unwrap();
        let v = s
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        v
    };
    assert!(banco.registros_que_nao_conferem(None).unwrap().is_empty());

    // Alguém desliga os gatilhos e muda o banco do caso com outra ferramenta.
    let c = Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap();
    c.execute_batch("DROP TRIGGER artefato_sem_update; DROP TRIGGER referencia_sem_update;")
        .unwrap();
    c.execute(
        "UPDATE artefato SET conteudo = 'adulterado' WHERE id = ?1",
        [ids[0]],
    )
    .unwrap();
    c.execute(
        "UPDATE artefato SET metadados = '{quebrado' WHERE id = ?1",
        [ids[1]],
    )
    .unwrap();
    c.execute(
        "UPDATE referencia_origem SET valores_brutos = '{\"_id\":1}' WHERE artefato_id = ?1",
        [ids[2]],
    )
    .unwrap();
    // Troca só o hash gravado (o conteúdo continua o original).
    c.execute(
        "UPDATE artefato SET hash_registro = ?2 WHERE id = ?1",
        params![ids[3], "0".repeat(64)],
    )
    .unwrap();
    c.execute("UPDATE artefato SET de_mim = 7 WHERE id = ?1", [ids[4]])
        .unwrap();
    drop(c);

    let ruins = banco.registros_que_nao_conferem(None).unwrap();
    let achados: Vec<i64> = ruins.iter().map(|r| r.artefato_id).collect();
    assert_eq!(achados, ids, "{ruins:#?}");
    let quebrado = &ruins[1];
    assert!(quebrado
        .recalculado
        .as_ref()
        .unwrap_err()
        .contains("metadados"));
    let um = banco.conferir_registro(ids[0]).unwrap();
    assert!(!um.confere());
    assert_ne!(um.recalculado.unwrap(), um.gravado);
}
