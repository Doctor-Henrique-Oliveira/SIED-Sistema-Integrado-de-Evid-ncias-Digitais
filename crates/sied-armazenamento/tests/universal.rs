//! Pesquisa universal sobre extrações e textos exclusivamente SINTÉTICOS.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use sha2::{Digest, Sha256};
use sied_armazenamento::busca::FiltroBusca;
use sied_armazenamento::documentos::{NovaEntidadeDocumento, NovoDocumento};
use sied_armazenamento::universal::{MecanismoUniversal as M, PedidoUniversal};
use sied_armazenamento::{BancoCaso, ControleDescoberta, ErroArmazenamento};

const TEXTO: &str = "SINTÉTICO: demo@example.invalid. Nenhum dado é real.";

fn preparado() -> (tempfile::TempDir, BancoCaso, NovoDocumento) {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("UNIVERSAL-SINTETICO");
    comum::extracao(&origem);
    std::fs::write(origem.join("example-sintetico.txt"), TEXTO).unwrap();
    let (mut banco, dispositivo) = comum::caso(raiz.path());
    let extracao = comum::importar(&banco, &dispositivo, &origem);
    let arquivo = banco
        .arquivos(extracao.id, None, None, 0, 500)
        .unwrap()
        .itens
        .into_iter()
        .find(|a| a.caminho == "example-sintetico.txt")
        .unwrap();
    let inicio = TEXTO.find("demo@example.invalid").unwrap();
    let novo = NovoDocumento {
        caso_id: banco.caso().id,
        extracao_id: extracao.id,
        arquivo_id: arquivo.id,
        caminho: arquivo.caminho,
        sha256_origem: arquivo.sha256.unwrap(),
        sha256_texto: hex::encode(Sha256::digest(TEXTO.as_bytes())),
        texto: TEXTO.into(),
        pagina: None,
        total_paginas: None,
        motor_id: "texto_utf8_nativo".into(),
        motor_versao: "1".into(),
        executado_em: chrono::Utc::now().to_rfc3339(),
        entidades_limitadas: false,
        entidades_motor_versao: "regra-sintetica/1".into(),
        entidades: vec![NovaEntidadeDocumento {
            tipo: "email".into(),
            inicio_utf8: inicio as u32,
            fim_utf8: (inicio + "demo@example.invalid".len()) as u32,
            literal: "demo@example.invalid".into(),
            regra_versao: "email-sintetico/1".into(),
        }],
    };
    banco.guardar_documento(&novo).unwrap();
    (raiz, banco, novo)
}

fn pedido(texto: &str) -> PedidoUniversal {
    PedidoUniversal {
        filtro: FiltroBusca {
            texto: texto.into(),
            ..Default::default()
        },
        mecanismos: vec![M::Registros, M::Arquivos, M::Documentos, M::Entidades],
    }
}

#[test]
fn recuperacao_mescla_sinais_e_duplicatas_de_snapshot_nao_multiplicam_votos() {
    let (_raiz, mut banco, mut novo) = preparado();
    for versao in ["2", "3", "4"] {
        novo.motor_versao = versao.into();
        banco.guardar_documento(&novo).unwrap();
    }
    let r = banco
        .pesquisar_universal(&pedido("example"), 0, 50, &ControleDescoberta::novo())
        .unwrap();
    assert_eq!(r.caso_id, novo.caso_id);
    let resultados: Vec<_> = r
        .itens
        .iter()
        .filter(|i| i.arquivo_id == Some(novo.arquivo_id))
        .collect();
    assert_eq!(resultados.len(), 1);
    let arquivo = resultados[0];
    assert_eq!(arquivo.sinais.len(), 3);
    let mecanismos: std::collections::BTreeSet<_> =
        arquivo.sinais.iter().map(|s| s.mecanismo).collect();
    assert_eq!(mecanismos.len(), arquivo.sinais.len());
    assert!(mecanismos.contains(&M::Arquivos));
    assert!(mecanismos.contains(&M::Documentos));
    assert!(mecanismos.contains(&M::Entidades));
    let esperado = arquivo
        .sinais
        .iter()
        .map(|s| s.contribuicao_rrf)
        .sum::<f64>()
        / (4.0 / 61.0);
    assert!((arquivo.score_ranking - esperado).abs() < 1e-12);
    assert!((0.0..=1.0).contains(&arquivo.score_ranking));
    assert!(arquivo
        .sinais
        .iter()
        .filter(|s| matches!(s.mecanismo, M::Documentos | M::Entidades))
        .all(|s| s.documento_id.is_some()
            && s.hash_derivado.as_deref() == Some(novo.sha256_texto.as_str())));
}

#[test]
fn filtros_globais_nao_fabricam_app_tipo_ou_data_para_documentos_e_arquivos() {
    let (_raiz, banco, novo) = preparado();
    for (app, tipo) in [
        (Some("whatsapp".into()), None),
        (None, Some("mensagem".into())),
    ] {
        let mut p = pedido("example");
        p.filtro.app = app;
        p.filtro.tipo = tipo;
        let r = banco
            .pesquisar_universal(&p, 0, 50, &ControleDescoberta::novo())
            .unwrap();
        assert!(!r
            .itens
            .iter()
            .any(|i| i.arquivo_id == Some(novo.arquivo_id)));
    }
    let mut p = pedido("example");
    p.filtro.extracao_id = Some(sied_dominio::ExtracaoId::novo());
    assert!(banco
        .pesquisar_universal(&p, 0, 50, &ControleDescoberta::novo())
        .unwrap()
        .itens
        .is_empty());
    let mut p = pedido("example");
    p.filtro.de_ms = Some(chrono::Utc::now().timestamp_millis() + 86_400_000);
    assert!(banco
        .pesquisar_universal(&p, 0, 50, &ControleDescoberta::novo())
        .unwrap()
        .itens
        .is_empty());
}

#[test]
fn cancelamento_nao_publica_resultado_e_nao_contamina_consulta_seguinte() {
    let (_raiz, banco, _novo) = preparado();
    let controle = ControleDescoberta::novo();
    assert!(controle.cancelar());
    assert!(matches!(
        banco.pesquisar_universal(&pedido("example"), 0, 50, &controle),
        Err(ErroArmazenamento::DescobertaCancelada)
    ));
    let r = banco
        .pesquisar_universal(&pedido("example"), 0, 50, &ControleDescoberta::novo())
        .unwrap();
    assert!(!r.itens.is_empty());
    assert!(banco.conferir_derivados().unwrap().consistente);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn entradas_invalidas_rejeitadas_e_janela_paginada_sem_contagem_global_falsa() {
    let (_raiz, banco, _novo) = preparado();
    let mut p = pedido("example");
    p.mecanismos.push(M::Documentos);
    assert!(banco
        .pesquisar_universal(&p, 0, 50, &ControleDescoberta::novo())
        .is_err());
    let mut p = pedido("example");
    p.filtro.de_ms = Some(2);
    p.filtro.ate_ms = Some(1);
    assert!(banco
        .pesquisar_universal(&p, 0, 50, &ControleDescoberta::novo())
        .is_err());
    assert!(banco
        .pesquisar_universal(&pedido("example"), -1, 50, &ControleDescoberta::novo())
        .is_err());
    assert!(banco
        .pesquisar_universal(&pedido("example"), 0, 51, &ControleDescoberta::novo())
        .is_err());
    let primeiro = banco
        .pesquisar_universal(&pedido("example"), 0, 1, &ControleDescoberta::novo())
        .unwrap();
    let depois = banco
        .pesquisar_universal(
            &pedido("example"),
            primeiro.recuperados_na_janela as i64,
            1,
            &ControleDescoberta::novo(),
        )
        .unwrap();
    assert!(depois.itens.is_empty());
    assert_eq!(depois.recuperados_na_janela, primeiro.recuperados_na_janela);
    assert!(primeiro.avisos.iter().any(|a| a.contains("não confiança")));
}
