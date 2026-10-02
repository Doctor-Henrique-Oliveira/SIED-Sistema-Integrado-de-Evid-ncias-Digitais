//! Regressões AI-0, exclusivamente sobre extração **SINTÉTICA** importada pelo
//! pipeline normal. As buscas usam conexão somente leitura ao banco do caso;
//! não há UPDATE no caso nem edição da fonte depois da importação.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use comum::*;
use rusqlite::{functions::FunctionFlags, params, Connection, OpenFlags};
use sied_armazenamento::busca::FiltroBusca;
use sied_armazenamento::descoberta::{self, ModoDescoberta, PedidoDescoberta, TipoIndicador};
use sied_armazenamento::{normalizar_busca, so_digitos, BancoCaso};
use std::collections::BTreeSet;
use std::fs;

const TEXTOS: &[&str] = &[
    "alpha sintético",
    "beta sintético",
    "alpha beta sintético",
    "Café ☕️ sintético",
    "Café ☕ sintético",
    "Café sintético",
    "👨‍👩‍👧‍👦 sintético",
    "👨 👩 👧 👦 sintético",
    "1️⃣ teste sintético",
    "TUDO 3 sintético",
    "TUDO 2 sintético",
    "encontro OR NEAR sintético",
    "encontro sintético",
    "192.0.2.1 teste@example.invalid https://example.invalid/a sintético",
    "999.1.2.3 err@@example.invalid javascript:alert(1) sintético",
    "📍-23.55052, -46.633308 sintético",
    "999.0, 99.0 sintético",
    "0123456789abcdef0123456789abcdef sintético",
    "Amélia SINTÉTICA",
];

fn preparar(volume: usize) -> (tempfile::TempDir, std::path::PathBuf, BancoCaso, Connection) {
    preparar_com_destinatario(volume, None)
}

fn preparar_com_destinatario(
    volume: usize,
    destinatario: Option<&str>,
) -> (tempfile::TempDir, std::path::PathBuf, BancoCaso, Connection) {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA-AI0");
    extracao(&origem);
    fs::create_dir_all(origem.join("sms")).unwrap();
    let fonte = Connection::open(origem.join("sms/mmssms.db")).unwrap();
    fonte
        .execute_batch(
            "CREATE TABLE sms(_id INTEGER PRIMARY KEY, thread_id, address, date, type, body);",
        )
        .unwrap();
    let mut ins = fonte
        .prepare("INSERT INTO sms VALUES (?1, 1, '+5511900000999', 1767225600000, 1, ?2)")
        .unwrap();
    for (i, t) in TEXTOS.iter().enumerate() {
        ins.execute(params![i as i64 + 1, t]).unwrap();
    }
    for i in 0..volume {
        ins.execute(params![
            i as i64 + 100,
            format!("alpha beta registro SINTÉTICO {i}")
        ])
        .unwrap();
    }
    drop(ins);
    if let Some(destinatario) = destinatario {
        // A fonte sintética é construída ANTES da importação. Tipo 2 é enviada:
        // o parser normal preserva `address` em `destinatarios`, serializado JSON.
        fonte
            .execute(
                "INSERT INTO sms VALUES (90, 2, ?1, 1767225600000, 2, ?2)",
                params![
                    destinatario,
                    "Mensagem SINTÉTICA enviada, com endereço de teste preservado."
                ],
            )
            .unwrap();
    }
    drop(fonte);
    fs::write(
        origem.join("alpha-beta-SINTETICO.txt"),
        "Conteúdo fora do escopo textual do inventário: segredo_sintetico_nao_indexado",
    )
    .unwrap();
    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    let conn = Connection::open_with_flags(
        raiz.path().join("Casos/IP-7/caso.sied.db"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    // As funções normalmente são registradas pelo BancoCaso; a conexão separada
    // comprova que o motor funciona sem qualquer permissão de escrita no SQLite.
    conn.create_scalar_function(
        "sied_normal",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |c| c.get::<String>(0).map(|v| normalizar_busca(&v)),
    )
    .unwrap();
    conn.create_scalar_function(
        "sied_digitos",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |c| Ok(c.get::<Option<String>>(0)?.and_then(|v| so_digitos(&v))),
    )
    .unwrap();
    (raiz, origem, banco, conn)
}

fn pedido(modo: ModoDescoberta, texto: &str) -> PedidoDescoberta {
    PedidoDescoberta {
        modo,
        filtro: FiltroBusca {
            texto: texto.into(),
            app: Some("sms".into()),
            tipo: Some("mensagem".into()),
            ..Default::default()
        },
        termos: Vec::new(),
        indicador: None,
    }
}

#[test]
fn uniao_explicita_sem_duplicatas_com_paginacao_estavel_e_filtros() {
    let (_raiz, _origem, _banco, conn) = preparar(400);
    let mut p = pedido(ModoDescoberta::Expansao, "alpha");
    p.termos = vec!["beta".into(), "alpha".into()];
    let r = descoberta::buscar(&conn, &p, 0, 200).unwrap();
    assert_eq!(r.termos_efetivos, ["alpha", "beta"]);
    assert_eq!(r.artefatos.total, 403);
    assert_eq!(r.artefatos.itens.len(), 200);
    assert_eq!(r.arquivos.total, 0, "app/tipo não se aplicam ao inventário");
    assert!(r.artefatos.itens.iter().all(|a| a.motivos.len() == 2));
    let pagina2 = descoberta::buscar(&conn, &p, 200, 200).unwrap();
    let pagina3 = descoberta::buscar(&conn, &p, 400, 200).unwrap();
    let ids: Vec<_> = r
        .artefatos
        .itens
        .iter()
        .chain(pagina2.artefatos.itens.iter())
        .chain(pagina3.artefatos.itens.iter())
        .map(|a| a.achado.artefato.id)
        .collect();
    assert_eq!(ids.len(), 403);
    assert!(
        ids.windows(2).all(|p| p[0] > p[1]),
        "datas empatadas devem desempatar pelo id"
    );
    assert_eq!(
        descoberta::buscar(&conn, &p, 0, 200)
            .unwrap()
            .artefatos
            .itens[0]
            .achado
            .artefato
            .id,
        ids[0]
    );
    p.filtro.de_ms = Some(1767225600001);
    assert_eq!(
        descoberta::buscar(&conn, &p, 0, 10)
            .unwrap()
            .artefatos
            .total,
        0
    );
    p.filtro.de_ms = None;
    p.filtro.app = Some("app_inexistente".into());
    assert_eq!(
        descoberta::buscar(&conn, &p, 0, 10)
            .unwrap()
            .artefatos
            .total,
        0
    );
}

#[test]
fn literal_preserva_emoji_zwj_variation_selector_caixa_e_consecutividade() {
    let (_raiz, _origem, _banco, conn) = preparar(0);
    for (texto, total) in [
        ("Café ☕️", 1),
        ("Café ☕", 2),
        ("Café", 3),
        ("café", 0),
        ("👨‍👩‍👧‍👦", 1),
        ("1️⃣", 1),
        ("TUDO 3", 1),
        ("TUDO 2", 1),
    ] {
        let r = descoberta::buscar(&conn, &pedido(ModoDescoberta::Literal, texto), 0, 20).unwrap();
        assert_eq!(r.artefatos.total, total, "{texto}");
    }
    let r = descoberta::buscar(&conn, &pedido(ModoDescoberta::Expansao, "Café ☕️"), 0, 20).unwrap();
    assert_eq!(r.artefatos.total, 1, "expansão não pode descartar o emoji");
    assert!(r.artefatos.itens[0].motivos[0].contains("literal exato"));
    assert_eq!(
        descoberta::buscar(
            &conn,
            &pedido(ModoDescoberta::Expansao, "encontro OR"),
            0,
            20
        )
        .unwrap()
        .artefatos
        .total,
        1,
        "OR é texto, não operador FTS"
    );
    assert_eq!(
        descoberta::buscar(
            &conn,
            &pedido(ModoDescoberta::Literal, "'; DROP TABLE artefato; --"),
            0,
            20
        )
        .unwrap()
        .artefatos
        .total,
        0
    );
}

#[test]
fn expansao_nfd_acentuada_continua_fts_sem_virar_literal() {
    let (_raiz, origem, banco, conn) = preparar(0);
    let antes = retrato(&origem);
    let texto_nfd = "cafe\u{0301}";
    let r = descoberta::buscar(&conn, &pedido(ModoDescoberta::Expansao, texto_nfd), 0, 20).unwrap();
    assert_eq!(r.termos_efetivos, [texto_nfd]);
    assert_eq!(r.artefatos.total, 3);
    assert_eq!(r.artefatos.total as usize, r.artefatos.itens.len());
    for a in &r.artefatos.itens {
        assert!(a.motivos.iter().all(|m| m.starts_with("FTS5 por prefixo")));
        assert!(a
            .achado
            .artefato
            .conteudo
            .as_deref()
            .unwrap()
            .contains("Café"));
        assert!(banco
            .conferir_registro(a.achado.artefato.id)
            .unwrap()
            .confere());
    }
    let sem_acento =
        descoberta::buscar(&conn, &pedido(ModoDescoberta::Expansao, "cafe"), 0, 20).unwrap();
    assert_eq!(
        r.artefatos
            .itens
            .iter()
            .map(|a| a.achado.artefato.id)
            .collect::<Vec<_>>(),
        sem_acento
            .artefatos
            .itens
            .iter()
            .map(|a| a.achado.artefato.id)
            .collect::<Vec<_>>()
    );
    let acento_interno = descoberta::buscar(
        &conn,
        &pedido(ModoDescoberta::Expansao, "ame\u{0301}lia"),
        0,
        20,
    )
    .unwrap();
    assert_eq!(acento_interno.artefatos.total, 1);
    assert!(acento_interno.artefatos.itens[0].motivos[0].starts_with("FTS5 por prefixo"));
    assert_eq!(
        acento_interno.artefatos.itens[0]
            .achado
            .artefato
            .conteudo
            .as_deref(),
        Some("Amélia SINTÉTICA")
    );
    // Marcas combinantes não são símbolos, mas VS16 e keycap continuam
    // significativos: essas sequências nunca podem virar apenas a palavra.
    let com_emoji =
        descoberta::buscar(&conn, &pedido(ModoDescoberta::Expansao, "Café ☕️"), 0, 20).unwrap();
    assert_eq!(com_emoji.artefatos.total, 1);
    assert!(com_emoji.artefatos.itens[0].motivos[0].contains("literal exato"));
    assert_eq!(antes, retrato(&origem));
    assert!(banco.conferir_derivados().unwrap().consistente);
}

#[test]
fn destinatario_json_escapado_tem_contagem_pagina_e_indicador_coerentes() {
    const DESTINATARIO: &str =
        "Revisor \"SINTÉTICO\"\\laboratório\n192.0.2.77\nteste-destinatario@example.invalid\n🧑‍🔬🔎";
    let (_raiz, origem, banco, conn) = preparar_com_destinatario(0, Some(DESTINATARIO));
    let antes = retrato(&origem);
    let mut p = pedido(ModoDescoberta::Indicadores, "");
    p.indicador = Some(TipoIndicador::Ip);
    let todos = descoberta::buscar(&conn, &p, 0, 20).unwrap();
    assert_eq!(todos.artefatos.total, 2);
    assert_eq!(todos.artefatos.itens.len(), 2);
    let enviado = todos
        .artefatos
        .itens
        .iter()
        .find(|a| {
            a.achado
                .artefato
                .destinatarios
                .iter()
                .any(|d| d == DESTINATARIO)
        })
        .unwrap();
    assert_eq!(enviado.achado.artefato.remetente, None);
    assert_eq!(enviado.achado.artefato.destinatarios, [DESTINATARIO]);
    assert!(enviado.indicadores.iter().any(|i| {
        i.tipo == TipoIndicador::Ip && i.valor == "192.0.2.77" && i.campo == "destinatarios"
    }));
    // A coluna contém escapes de armazenamento; o motor deve procurar o valor
    // decodificado (json_each), exatamente o mesmo valor entregue à interface.
    let armazenado: String = conn
        .query_row(
            "SELECT destinatarios FROM artefato WHERE id = ?1",
            [enviado.achado.artefato.id],
            |r| r.get(0),
        )
        .unwrap();
    assert!(armazenado.contains("\\n192.0.2.77\\n"));
    assert!(armazenado.contains("\\\"SINTÉTICO\\\""));
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&armazenado).unwrap(),
        [DESTINATARIO]
    );
    for (indice, esperado) in todos.artefatos.itens.iter().enumerate() {
        let pagina = descoberta::buscar(&conn, &p, indice as i64, 1).unwrap();
        assert_eq!(pagina.artefatos.total, 2);
        assert_eq!(pagina.artefatos.itens.len(), 1);
        assert_eq!(
            pagina.artefatos.itens[0].achado.artefato.id,
            esperado.achado.artefato.id
        );
        assert!(!pagina.artefatos.itens[0].indicadores.is_empty());
    }
    for (tipo, texto) in [
        (TipoIndicador::Ip, "192.0.2.77"),
        (TipoIndicador::Email, "teste-destinatario@example.invalid"),
    ] {
        p.indicador = Some(tipo);
        p.filtro.texto = texto.into();
        let filtrado = descoberta::buscar(&conn, &p, 0, 20).unwrap();
        assert_eq!(filtrado.artefatos.total, 1);
        assert_eq!(filtrado.artefatos.itens.len(), 1);
        assert!(filtrado.artefatos.itens[0]
            .indicadores
            .iter()
            .any(|i| i.tipo == tipo && i.valor == texto && i.campo == "destinatarios"));
    }
    for trecho in ["\"SINTÉTICO\"\\laboratório\n192.0.2.77", "🧑‍🔬🔎"] {
        let literal =
            descoberta::buscar(&conn, &pedido(ModoDescoberta::Literal, trecho), 0, 20).unwrap();
        assert_eq!(literal.artefatos.total, 1);
        assert_eq!(
            literal.artefatos.itens[0].achado.artefato.destinatarios,
            [DESTINATARIO]
        );
    }
    assert!(banco
        .conferir_registro(enviado.achado.artefato.id)
        .unwrap()
        .confere());
    assert_eq!(antes, retrato(&origem));
    assert!(banco.conferir_derivados().unwrap().consistente);
}

#[test]
fn inventario_busca_so_caminhos_sem_ler_conteudo_e_sem_ignorar_filtros() {
    let (_raiz, _origem, _banco, conn) = preparar(0);
    let mut p = pedido(ModoDescoberta::Expansao, "alpha");
    p.filtro.app = None;
    p.filtro.tipo = None;
    p.termos = vec!["beta".into()];
    let r = descoberta::buscar(&conn, &p, 0, 20).unwrap();
    assert_eq!(r.arquivos.total, 1);
    assert_eq!(r.arquivos.itens[0].caminho, "alpha-beta-SINTETICO.txt");
    p.filtro.extracao_id = Some(r.arquivos.itens[0].extracao_id);
    assert_eq!(
        descoberta::buscar(&conn, &p, 0, 20).unwrap().arquivos.total,
        1
    );
    p.filtro.extracao_id = Some(sied_dominio::ExtracaoId::novo());
    assert_eq!(
        descoberta::buscar(&conn, &p, 0, 20).unwrap().arquivos.total,
        0
    );
    p.filtro.extracao_id = None;
    p.filtro.texto = "segredo_sintetico_nao_indexado".into();
    p.termos.clear();
    assert_eq!(
        descoberta::buscar(&conn, &p, 0, 20).unwrap().arquivos.total,
        0
    );
    p.filtro.texto = "alpha".into();
    p.filtro.de_ms = Some(0);
    assert_eq!(
        descoberta::buscar(&conn, &p, 0, 20).unwrap().arquivos.total,
        0
    );
}

#[test]
fn indicadores_sao_candidatos_com_campo_e_nao_aceitam_dados_implausiveis() {
    let (_raiz, _origem, _banco, conn) = preparar(0);
    for (tipo, esperado) in [
        (TipoIndicador::Ip, "192.0.2.1"),
        (TipoIndicador::Email, "teste@example.invalid"),
        (TipoIndicador::Url, "https://example.invalid/a"),
        (TipoIndicador::Dominio, "example.invalid"),
        (TipoIndicador::Hash, "0123456789abcdef0123456789abcdef"),
        (TipoIndicador::Coordenada, "-23.55052, -46.633308"),
    ] {
        let mut p = pedido(ModoDescoberta::Indicadores, "");
        p.indicador = Some(tipo);
        let r = descoberta::buscar(&conn, &p, 0, 20).unwrap();
        assert_eq!(r.artefatos.total, 1, "{tipo:?}");
        assert!(r.artefatos.itens[0]
            .indicadores
            .iter()
            .any(|i| i.tipo == tipo && i.valor == esperado && i.campo == "conteudo"));
        assert!(r.artefatos.itens[0].motivos[0].contains("candidato"));
        assert_eq!(r.arquivos.total, 0);
    }
    let mut p = pedido(ModoDescoberta::Indicadores, "");
    p.indicador = Some(TipoIndicador::Telefone);
    let r = descoberta::buscar(&conn, &p, 0, 200).unwrap();
    assert!(r
        .artefatos
        .itens
        .iter()
        .all(|a| a.indicadores.iter().any(|i| i.campo == "remetente")));
    p.indicador = Some(TipoIndicador::Ip);
    p.filtro.texto = "999.1.2.3".into();
    assert_eq!(
        descoberta::buscar(&conn, &p, 0, 20)
            .unwrap()
            .artefatos
            .total,
        0
    );
}

#[test]
fn consultas_nao_modificam_evidencia_hashes_fts_ou_auditoria() {
    let (_raiz, origem, banco, conn) = preparar(0);
    let antes = retrato(&origem);
    let retrato_db = || {
        conn.prepare("SELECT id, hash_registro FROM artefato ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    };
    let hashes = retrato_db();
    let auditoria_antes = banco.auditoria(10000).unwrap().len();
    for modo in [
        ModoDescoberta::Expansao,
        ModoDescoberta::Literal,
        ModoDescoberta::Indicadores,
    ] {
        descoberta::buscar(
            &conn,
            &pedido(
                modo,
                if modo == ModoDescoberta::Indicadores {
                    ""
                } else {
                    "alpha"
                },
            ),
            0,
            20,
        )
        .unwrap();
    }
    assert_eq!(antes, retrato(&origem));
    assert_eq!(hashes, retrato_db());
    for (id, _) in hashes {
        assert!(banco.conferir_registro(id).unwrap().confere());
    }
    assert!(banco.conferir_derivados().unwrap().consistente);
    assert_eq!(
        auditoria_antes,
        banco.auditoria(10000).unwrap().len(),
        "motor de consulta não escreve auditoria; snapshot fica em contrato separado"
    );
}

#[test]
fn limites_recusam_consultas_nao_limitadas() {
    let (_raiz, _origem, _banco, conn) = preparar(0);
    let mut p = pedido(ModoDescoberta::Expansao, "alpha");
    assert!(descoberta::buscar(&conn, &p, -1, 20).is_err());
    assert!(descoberta::buscar(&conn, &p, 0, 0).is_err());
    assert!(descoberta::buscar(&conn, &p, 0, 201).is_err());
    p.termos = (0..32).map(|i| format!("termo{i}")).collect();
    assert!(descoberta::buscar(&conn, &p, 0, 20).is_err());
    p.termos.clear();
    p.filtro.texto = "🔎".repeat(201);
    assert!(descoberta::buscar(&conn, &p, 0, 20).is_err());
    p.filtro.texto = "alpha\0".into();
    assert!(descoberta::buscar(&conn, &p, 0, 20).is_err());
    p.filtro.texto = "alpha".into();
    p.filtro.de_ms = Some(10);
    p.filtro.ate_ms = Some(9);
    assert!(descoberta::buscar(&conn, &p, 0, 20).is_err());
}

// Ground truth limitado à gramática conservadora documentada da AI-0, não a
// identidades nem à validade externa desses dados. Nenhum endereço é consultado.
// Domínios .invalid e IPs de documentação são strings exclusivamente SINTÉTICAS.
const CORPUS_INDICADORES: &[(TipoIndicador, &str, &[&str])] = &[
    (
        TipoIndicador::Ip,
        "192.0.2.45 [2001:db8::45]",
        &["192.0.2.45", "2001:db8::45"],
    ),
    (TipoIndicador::Ip, "999.1.2.3 192.0.2.256 2001:db8::zz", &[]),
    (
        TipoIndicador::Email,
        "teste+laboratorio@example.invalid analista.teste@sub.example.invalid",
        &["teste+laboratorio@example.invalid", "analista.teste@sub.example.invalid"],
    ),
    (
        TipoIndicador::Email,
        "nome..teste@example.invalid err@@example.invalid nome@-example.invalid",
        &[],
    ),
    (
        TipoIndicador::Url,
        "http://192.0.2.5:8080/a https://example.invalid/arquivo?q=1",
        &["http://192.0.2.5:8080/a", "https://example.invalid/arquivo?q=1"],
    ),
    (
        TipoIndicador::Url,
        "javascript:alert(1) ftp://example.invalid/a https://example.invalid:abc/a https://user@example.invalid/a",
        &[],
    ),
    (
        TipoIndicador::Dominio,
        "example.invalid sub.example.invalid",
        &["example.invalid", "sub.example.invalid"],
    ),
    (
        TipoIndicador::Dominio,
        "-example.invalid example-.invalid example..invalid 192.0.2.1",
        &[],
    ),
    (
        TipoIndicador::Hash,
        "0123456789abcdef0123456789abcdef ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789",
        &["0123456789abcdef0123456789abcdef", "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789"],
    ),
    (
        TipoIndicador::Hash,
        "0123456789abcdef0123456789abcde 0123456789abcdef0123456789abcdef0 0123456789abcdef0123456789abcdeg",
        &[],
    ),
    (
        TipoIndicador::Telefone,
        "+55-11-90000-0045\n11900000045",
        &["+55-11-90000-0045", "11900000045"],
    ),
    (TipoIndicador::Telefone, "1234567\n1234567890123456\n12.345.678", &[]),
    (
        TipoIndicador::Coordenada,
        "📍 -23.55052, -46.633308; 0.0, 180.0",
        &["-23.55052, -46.633308", "0.0, 180.0"],
    ),
    (
        TipoIndicador::Coordenada,
        "91.0, 0.0; -23.0, 181.0; 12, 21; abc12.3, 21.4; 12.3, 21.4e400",
        &[],
    ),
];

#[test]
fn corpus_rotulado_confere_conjuntos_exatos_das_sete_classes_sem_extrapolar_recall() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA-GROUND-TRUTH-AI0");
    extracao(&origem);
    fs::create_dir_all(origem.join("sms")).unwrap();
    let mut fonte = Connection::open(origem.join("sms/mmssms.db")).unwrap();
    fonte
        .execute_batch(
            "CREATE TABLE sms(_id INTEGER PRIMARY KEY, thread_id, address, date, type, body);",
        )
        .unwrap();
    let tx = fonte.transaction().unwrap();
    for (indice, (_, texto, _)) in CORPUS_INDICADORES.iter().enumerate() {
        // Rótulos e fonte são construídos ANTES da importação. O remetente
        // neutro impede que telefones da fixture virem positivos por outro campo.
        tx.execute(
            "INSERT INTO sms VALUES (?1, 1, 'REMETENTE-SINTETICO', 1767225600000, 1, ?2)",
            params![
                indice as i64 + 1,
                format!("GT-SINTETICO-{indice:02}-FIM {texto}")
            ],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    drop(fonte);
    let origem_antes = retrato(&origem);
    let (banco, dispositivo) = caso(raiz.path());
    let x = importar(&banco, &dispositivo, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    let conn = Connection::open_with_flags(
        banco.pasta().join("caso.sied.db"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let auditoria_antes = banco.verificar_auditoria().unwrap();

    for (indice, (tipo, _, esperados)) in CORPUS_INDICADORES.iter().enumerate() {
        let rotulo = format!("GT-SINTETICO-{indice:02}-FIM");
        let mut p = pedido(ModoDescoberta::Indicadores, &rotulo);
        p.indicador = Some(*tipo);
        let resultado = descoberta::buscar(&conn, &p, 0, 20).unwrap();
        let total_esperado = if esperados.is_empty() { 0_i64 } else { 1 };
        assert_eq!(
            resultado.artefatos.total, total_esperado,
            "{rotulo}: {tipo:?}"
        );
        assert_eq!(resultado.artefatos.itens.len() as i64, total_esperado);
        assert_eq!(resultado.arquivos.total, 0);
        let mut obtidos = BTreeSet::new();
        for item in &resultado.artefatos.itens {
            assert!(item
                .achado
                .artefato
                .conteudo
                .as_ref()
                .unwrap()
                .starts_with(&rotulo));
            assert!(banco
                .conferir_registro(item.achado.artefato.id)
                .unwrap()
                .confere());
            for indicador in &item.indicadores {
                assert_eq!(indicador.tipo, *tipo);
                assert_eq!(indicador.campo, "conteudo");
                assert!(
                    obtidos.insert(indicador.valor.as_str()),
                    "indicador duplicado"
                );
            }
        }
        let esperados: BTreeSet<_> = esperados.iter().copied().collect();
        let falsos_positivos: Vec<_> = obtidos.difference(&esperados).copied().collect();
        let falsos_negativos: Vec<_> = esperados.difference(&obtidos).copied().collect();
        assert!(
            falsos_positivos.is_empty(),
            "{rotulo}: FP={falsos_positivos:?}"
        );
        assert!(
            falsos_negativos.is_empty(),
            "{rotulo}: FN={falsos_negativos:?}"
        );
    }
    let auditoria_depois = banco.verificar_auditoria().unwrap();
    assert_eq!(auditoria_depois.registros, auditoria_antes.registros);
    assert_eq!(auditoria_depois.hash_final, auditoria_antes.hash_final);
    assert_eq!(retrato(&origem), origem_antes);
    assert!(banco.conferir_derivados().unwrap().consistente);
}
