//! Conferência e reconstrução dos dados derivados (conversas e índices de busca) sobre
//! uma extração **SINTÉTICA** (ver `testes/rust/caso_sintetico.rs`).

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use comum::*;
use rusqlite::Connection;
use sied_armazenamento::busca::FiltroBusca;
use sied_armazenamento::caso::NOME_ARQUIVO_CASO;
use sied_armazenamento::derivados::ConferenciaDerivados;
use sied_armazenamento::BancoCaso;

fn diferenca(c: &ConferenciaDerivados, tabela: &str) -> (i64, i64, bool) {
    let t = c.tabelas.iter().find(|t| t.tabela == tabela).unwrap();
    (t.faltando, t.sobrando, t.indice_integro)
}

fn conversas(banco: &BancoCaso) -> Vec<(i64, String, i64, i64)> {
    let mut v: Vec<_> = banco
        .conversas(None, 0, 100)
        .unwrap()
        .itens
        .into_iter()
        .map(|c| (c.id, c.id_externo, c.total_mensagens, c.total_participantes))
        .collect();
    v.sort();
    v
}

fn achados(banco: &BancoCaso, texto: &str) -> (i64, i64) {
    let r = banco
        .buscar(
            &FiltroBusca {
                texto: texto.into(),
                ..Default::default()
            },
            0,
            50,
        )
        .unwrap();
    (r.artefatos.total, r.arquivos.total)
}

/// Contagem e hash de todas as linhas das tabelas de origem, para provar que a
/// reconstrução não as toca.
fn origem(c: &Connection) -> String {
    c.query_row(
        "SELECT (SELECT COUNT(*) || ':' || group_concat(hash_registro) FROM artefato)
             || '|' || (SELECT COUNT(*) || ':' || group_concat(sha256) FROM arquivo)",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn caso_recem_importado_confere() {
    let (_raiz, _origem, banco, _x) = preparado();
    let c = banco.conferir_derivados().unwrap();
    assert!(c.consistente, "{c:?}");
    assert_eq!(c.tabelas.len(), 4);
    assert!(c.tabelas.iter().all(|t| t.esperadas > 0), "{c:?}");
}

#[test]
fn divergencias_sao_achadas_e_a_reconstrucao_corrige_sem_tocar_na_origem() {
    let (_raiz, _origem, mut banco, _x) = preparado();
    let pasta = banco.pasta().to_path_buf();
    let c = Connection::open(pasta.join(NOME_ARQUIVO_CASO)).unwrap();
    let antes_conversas = conversas(&banco);
    let antes_busca = achados(&banco, "sintetic");
    let antes_origem = origem(&c);
    assert!(antes_busca.0 > 0 && antes_busca.1 > 0);

    // Estragos que só as tabelas derivadas admitem (as de origem recusam por gatilho).
    c.execute_batch(
        "UPDATE conversa SET total_mensagens = total_mensagens + 5
          WHERE id = (SELECT MIN(id) FROM conversa);
         INSERT INTO conversa (extracao_id, app, id_externo, total_mensagens, enviadas,
                               com_anexo, primeira_ms, ultima_ms)
         SELECT extracao_id, app, 'inexistente@s.whatsapp.net', 1, 0, 0, NULL, NULL
         FROM conversa LIMIT 1;
         INSERT INTO conversa_participante (conversa_id, identificador, mensagens)
         SELECT MAX(id), 'ninguem', 1 FROM conversa;
         DELETE FROM conversa_participante
          WHERE rowid = (SELECT MIN(rowid) FROM conversa_participante);
         DELETE FROM busca_artefato WHERE rowid IN (SELECT rowid FROM busca_artefato LIMIT 3);
         INSERT INTO busca_arquivo (rowid, caminho) VALUES (999999, 'nao/existe.db');",
    )
    .unwrap();

    let conf = banco.conferir_derivados().unwrap();
    assert!(!conf.consistente);
    // Conversa alterada: falta a certa e sobra a errada; a inventada só sobra.
    assert_eq!(diferenca(&conf, "conversa"), (1, 2, true));
    assert_eq!(diferenca(&conf, "conversa_participante"), (1, 1, true));
    assert_eq!(diferenca(&conf, "busca_artefato"), (3, 0, true));
    assert_eq!(diferenca(&conf, "busca_arquivo"), (0, 1, true));

    let r = banco.reconstruir_derivados().unwrap();
    assert_eq!(r.antes, conf);
    assert!(r.depois.consistente, "{:?}", r.depois);
    assert!(banco.conferir_derivados().unwrap().consistente);

    // Mesmas conversas, com os mesmos ids; a inventada sumiu.
    assert_eq!(conversas(&banco), antes_conversas);
    assert_eq!(achados(&banco, "sintetic"), antes_busca);
    assert_eq!(achados(&banco, "existe").1, 0);
    assert_eq!(origem(&c), antes_origem);

    let reg = banco
        .auditoria(5)
        .unwrap()
        .into_iter()
        .find(|r| r.acao == "derivados_reconstruidos")
        .expect("reconstrução auditada");
    assert_eq!(reg.detalhes["antes"][0]["tabela"], "conversa");
    assert_eq!(reg.detalhes["antes"][0]["sobrando"], 2);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn indice_de_busca_danificado_e_achado_e_refeito() {
    let (_raiz, _origem, mut banco, _x) = preparado();
    let antes = achados(&banco, "sintetic");
    let c = Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap();
    // Apaga os segmentos do índice e mantém o texto: só o integrity-check percebe.
    c.execute("DELETE FROM busca_artefato_data WHERE id > 10", [])
        .unwrap();

    let conf = banco.conferir_derivados().unwrap();
    assert!(!conf.consistente);
    assert_eq!(diferenca(&conf, "busca_artefato"), (0, 0, false));
    assert_eq!(diferenca(&conf, "busca_arquivo"), (0, 0, true));

    banco.reconstruir_derivados().unwrap();
    assert!(banco.conferir_derivados().unwrap().consistente);
    assert_eq!(achados(&banco, "sintetic"), antes);
}
