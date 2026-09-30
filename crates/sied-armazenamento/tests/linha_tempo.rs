//! Linha do tempo sobre uma extração **SINTÉTICA** (WhatsApp de `caso_sintetico.rs`
//! mais o registro de chamadas de `testes/fixtures/android/calllog.sql`). Nenhum dado
//! é real; as pastas não representam nenhuma ferramenta de extração.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod caso_sintetico;

use caso_sintetico::*;
use rusqlite::Connection;
use sied_armazenamento::linha_tempo::FiltroLinhaTempo;
use std::fs;

const CALLLOG: &str = include_str!("../../../testes/fixtures/android/calllog.sql");

fn preparar() -> (
    tempfile::TempDir,
    sied_armazenamento::BancoCaso,
    sied_armazenamento::extracoes::Extracao,
) {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    extracao(&origem);
    let dir = origem.join("dados/data/com.android.providers.contacts/databases");
    fs::create_dir_all(&dir).unwrap();
    Connection::open(dir.join("calllog.db"))
        .unwrap()
        .execute_batch(CALLLOG)
        .unwrap();
    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    (raiz, banco, x)
}

fn tipos(v: &[String]) -> Option<Vec<String>> {
    Some(v.to_vec())
}

#[test]
fn ordem_cronologica_de_todos_os_apps() {
    let (_raiz, banco, x) = preparar();
    let f = FiltroLinhaTempo::default();
    let p = banco.linha_do_tempo(&f, 0, 500).unwrap();
    assert_eq!(p.total as usize, p.itens.len());
    assert!(p.itens.iter().all(|a| a.artefato.data_hora_ms.is_some()));
    let datas: Vec<i64> = p
        .itens
        .iter()
        .map(|a| a.artefato.data_hora_ms.unwrap())
        .collect();
    assert!(datas.windows(2).all(|w| w[0] <= w[1]), "fora de ordem");
    // Chamadas (1º/01) vêm antes das mensagens do WhatsApp (02/01).
    assert_eq!(p.itens.first().unwrap().artefato.tipo, "chamada");
    assert_eq!(
        p.itens.last().unwrap().artefato.app.as_deref(),
        Some("whatsapp")
    );
    assert!(p.itens.iter().all(|a| a.extracao_id == x.id));
    // O cabeçalho da conversa não é um evento (a data dele é a da última mensagem).
    assert!(p.itens.iter().all(|a| a.artefato.tipo != "conversa"));
    // Mensagens trazem a conversa, para abrir no contexto.
    let msg = p
        .itens
        .iter()
        .find(|a| a.artefato.tipo == "mensagem")
        .unwrap();
    assert!(msg.conversa_id.is_some());

    // Mais recentes primeiro = a mesma lista invertida.
    let inv = banco
        .linha_do_tempo(
            &FiltroLinhaTempo {
                recentes_primeiro: Some(true),
                ..Default::default()
            },
            0,
            500,
        )
        .unwrap();
    let ids: Vec<i64> = p.itens.iter().rev().map(|a| a.artefato.id).collect();
    assert_eq!(
        inv.itens.iter().map(|a| a.artefato.id).collect::<Vec<_>>(),
        ids
    );

    // Paginação estável.
    let pag = banco.linha_do_tempo(&f, 3, 4).unwrap();
    assert_eq!(pag.total, p.total);
    let esperados: Vec<i64> = p.itens[3..7].iter().map(|a| a.artefato.id).collect();
    assert_eq!(
        pag.itens.iter().map(|a| a.artefato.id).collect::<Vec<_>>(),
        esperados
    );
}

#[test]
fn filtros_de_tipo_app_periodo_e_coordenadas() {
    let (_raiz, banco, x) = preparar();
    let todas = banco
        .linha_do_tempo(&FiltroLinhaTempo::default(), 0, 500)
        .unwrap();

    let so_chamadas = banco
        .linha_do_tempo(
            &FiltroLinhaTempo {
                tipos: tipos(&["chamada".into()]),
                ..Default::default()
            },
            0,
            500,
        )
        .unwrap();
    assert!(so_chamadas.total >= 9);
    assert!(so_chamadas
        .itens
        .iter()
        .all(|a| a.artefato.tipo == "chamada"));

    let wa = banco
        .linha_do_tempo(
            &FiltroLinhaTempo {
                app: Some("whatsapp".into()),
                ..Default::default()
            },
            0,
            500,
        )
        .unwrap();
    assert!(wa
        .itens
        .iter()
        .all(|a| a.artefato.app.as_deref() == Some("whatsapp")));
    assert_eq!(wa.total + so_chamadas.total, todas.total);

    // Período: só o intervalo da conversa com a Ana (mensagens 1 a 8, inclusivo).
    let periodo = banco
        .linha_do_tempo(
            &FiltroLinhaTempo {
                de_ms: Some(1_767_355_200_000),
                ate_ms: Some(1_767_355_620_000),
                ..Default::default()
            },
            0,
            500,
        )
        .unwrap();
    assert_eq!(periodo.total, 8, "{:?}", periodo.itens);

    // Coordenadas: a mensagem de localização.
    let geo = banco
        .linha_do_tempo(
            &FiltroLinhaTempo {
                com_coordenadas: Some(true),
                ..Default::default()
            },
            0,
            500,
        )
        .unwrap();
    assert_eq!(geo.total, 1);
    assert_eq!(geo.itens[0].artefato.latitude, Some(-23.55052));
    assert_eq!(
        geo.itens[0].artefato.id_externo.as_deref(),
        Some("SINT0000000005")
    );

    // Extração inexistente: nada.
    let outra = banco
        .linha_do_tempo(
            &FiltroLinhaTempo {
                extracao_id: Some(sied_dominio::ExtracaoId::novo()),
                ..Default::default()
            },
            0,
            10,
        )
        .unwrap();
    assert_eq!(outra.total, 0);
    let mesma = banco
        .linha_do_tempo(
            &FiltroLinhaTempo {
                extracao_id: Some(x.id),
                ..Default::default()
            },
            0,
            1,
        )
        .unwrap();
    assert_eq!(mesma.total, todas.total);
}

#[test]
fn resumo_conta_tipos_apps_intervalo_e_sem_data() {
    let (_raiz, banco, _x) = preparar();
    let todas = banco
        .linha_do_tempo(&FiltroLinhaTempo::default(), 0, 500)
        .unwrap();
    let r = banco
        .resumo_linha_do_tempo(&FiltroLinhaTempo::default())
        .unwrap();
    assert_eq!(r.por_tipo.iter().map(|t| t.total).sum::<i64>(), todas.total);
    assert!(r.por_tipo.iter().any(|t| t.tipo == "chamada"));
    assert!(r.por_tipo.iter().any(|t| t.tipo == "mensagem"));
    assert!(r.apps.iter().any(|t| t.tipo == "telefone"));
    assert!(r.apps.iter().any(|t| t.tipo == "whatsapp"));
    assert_eq!(
        r.primeira_ms,
        todas.itens.first().unwrap().artefato.data_hora_ms
    );
    assert_eq!(
        r.ultima_ms,
        todas.itens.last().unwrap().artefato.data_hora_ms
    );
    assert!(r.sem_data >= 0);

    // O filtro de tipo não muda a contagem por tipo (para a tela mostrar todos),
    // mas muda os apps e o intervalo.
    let f = FiltroLinhaTempo {
        tipos: tipos(&["chamada".into()]),
        ..Default::default()
    };
    let rc = banco.resumo_linha_do_tempo(&f).unwrap();
    let par = |v: &[sied_armazenamento::resumo::TotalTipo]| {
        v.iter()
            .map(|t| (t.tipo.clone(), t.total))
            .collect::<Vec<_>>()
    };
    assert_eq!(par(&rc.por_tipo), par(&r.por_tipo));
    assert_eq!(rc.apps.len(), 1);
    assert_eq!(rc.apps[0].tipo, "telefone");
    assert!(rc.ultima_ms < r.ultima_ms);
}
