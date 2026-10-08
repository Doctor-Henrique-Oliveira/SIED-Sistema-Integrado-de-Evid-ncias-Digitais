//! Avaliação supervisionada exclusivamente SINTÉTICA e sem treinamento.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use sied_armazenamento::caso::NOME_ARQUIVO_CASO;
use sied_armazenamento::documentos::{
    EstadoRevisaoEntidade as Estado, NovaEntidadeDocumento, NovoDocumento,
};
use sied_armazenamento::supervisao::PedidoAmostraSupervisionada;
use sied_armazenamento::{BancoCaso, ControleDescoberta, ErroArmazenamento};

const TEXTO:&str="SINTÉTICO 🧪 tp@example.invalid fp@example.invalid MISSED-SINTETICO NEGATIVO-SINTETICO pending@example.invalid INCERTO-SINTETICO";

fn preparado() -> (tempfile::TempDir, BancoCaso, i64) {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("SUPERVISAO-SINTETICA");
    comum::extracao(&origem);
    std::fs::write(origem.join("avaliacao-sintetica.txt"), TEXTO).unwrap();
    let (mut banco, disp) = comum::caso(raiz.path());
    let x = comum::importar(&banco, &disp, &origem);
    let a = banco
        .arquivos(x.id, None, None, 0, 500)
        .unwrap()
        .itens
        .into_iter()
        .find(|a| a.caminho == "avaliacao-sintetica.txt")
        .unwrap();
    let entidades = [
        "tp@example.invalid",
        "fp@example.invalid",
        "pending@example.invalid",
    ]
    .into_iter()
    .map(|literal| {
        let inicio = TEXTO.find(literal).unwrap();
        NovaEntidadeDocumento {
            tipo: "email".into(),
            inicio_utf8: inicio as u32,
            fim_utf8: (inicio + literal.len()) as u32,
            literal: literal.into(),
            regra_versao: "email-sintetico/1".into(),
        }
    })
    .collect();
    let novo = NovoDocumento {
        caso_id: banco.caso().id,
        extracao_id: x.id,
        arquivo_id: a.id,
        caminho: a.caminho,
        sha256_origem: a.sha256.unwrap(),
        sha256_texto: hex::encode(Sha256::digest(TEXTO.as_bytes())),
        texto: TEXTO.into(),
        pagina: None,
        total_paginas: None,
        motor_id: "texto_utf8_nativo".into(),
        motor_versao: "1".into(),
        executado_em: chrono::Utc::now().to_rfc3339(),
        entidades_limitadas: false,
        entidades_motor_versao: "motor-sintetico/1".into(),
        entidades,
    };
    let d = banco.guardar_documento(&novo).unwrap();
    (raiz, banco, d.id)
}

fn pedido(
    doc: i64,
    literal: &str,
    esperado: bool,
    previsto: bool,
    estado: Estado,
) -> PedidoAmostraSupervisionada {
    let inicio = TEXTO.find(literal).unwrap();
    PedidoAmostraSupervisionada {
        documento_id: doc,
        inicio_utf8: inicio as u32,
        fim_utf8: (inicio + literal.len()) as u32,
        esperado,
        previsto,
        estado_rotulo: estado,
        nota: Some("Anotação SINTÉTICA de teste; nenhum dado real.".into()),
    }
}

#[test]
fn dataset_exige_autorizacao_retencao_e_versiona_sem_coleta_automatica() {
    let (_raiz, mut banco, _doc) = preparado();
    assert!(banco
        .criar_dataset("SINTETICO", "", "apagar após teste")
        .is_err());
    assert!(banco
        .criar_dataset("SINTETICO", "autorizado apenas teste", "")
        .is_err());
    assert!(banco
        .criar_dataset("", "autorizado apenas teste", "apagar após teste")
        .is_err());
    assert_eq!(banco.listar_datasets(0, 50).unwrap().total, 0);
    let d = banco
        .criar_dataset(
            "SINTETICO",
            "Autorização explícita sintética.",
            "Retenção declarada apenas para teste.",
        )
        .unwrap();
    assert_eq!(d.total_amostras, 0);
    assert_eq!(d.total_versoes, 0);
    assert_eq!(d.versao, 1);
    let outro = banco
        .criar_dataset(
            "SINTETICO",
            "Outra versão autorizada sintética.",
            "Retenção de teste.",
        )
        .unwrap();
    assert_eq!(outro.versao, 2);
    assert_ne!(d.id, outro.id);
    assert_eq!(banco.listar_datasets(0, 1).unwrap().total, 2);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn matriz_de_confusao_real_exclui_rotulos_nao_revisados_inconclusivos_e_historico() {
    let (_raiz, mut banco, doc) = preparado();
    let d = banco
        .criar_dataset(
            "SINTETICO-METRICAS",
            "Autorizado apenas teste.",
            "Retenção de teste.",
        )
        .unwrap();
    for (literal, esperado, previsto, estado) in [
        ("tp@example.invalid", true, true, Estado::Confirmado),
        ("fp@example.invalid", false, true, Estado::Confirmado),
        ("MISSED-SINTETICO", true, false, Estado::Confirmado),
        ("NEGATIVO-SINTETICO", false, false, Estado::Confirmado),
        ("pending@example.invalid", true, true, Estado::NaoRevisado),
        ("INCERTO-SINTETICO", true, false, Estado::Inconclusivo),
    ] {
        banco
            .adicionar_amostra(d.id, &pedido(doc, literal, esperado, previsto, estado))
            .unwrap();
    }
    let a = banco.avaliar_dataset(d.id).unwrap();
    assert_eq!((a.tp, a.fp, a.fn_, a.tn), (1, 1, 1, 1));
    assert_eq!(
        (
            a.total_amostras,
            a.total_versoes,
            a.elegiveis,
            a.nao_revisadas,
            a.inconclusivas
        ),
        (6, 6, 4, 1, 1)
    );
    assert_eq!(a.precision, Some(0.5));
    assert_eq!(a.recall, Some(0.5));
    assert_eq!(a.f1, Some(0.5));
    assert_eq!(a.acuracia, Some(0.5));
    assert_eq!(a.cobertura, Some(4.0 / 6.0));
    assert!(a.limites.iter().any(|l| l.contains("não mede")));
    // Revisar o mesmo intervalo cria outra versão, não outra observação estatística.
    let revisada = banco
        .adicionar_amostra(
            d.id,
            &pedido(doc, "tp@example.invalid", true, true, Estado::Inconclusivo),
        )
        .unwrap();
    assert_eq!(revisada.versao, 2);
    let a = banco.avaliar_dataset(d.id).unwrap();
    assert_eq!(a.total_amostras, 6);
    assert_eq!(a.total_versoes, 7);
    assert_eq!((a.tp, a.fp, a.fn_, a.tn), (0, 1, 1, 1));
    assert_eq!(a.inconclusivas, 2);
    assert_eq!(banco.amostras_dataset(d.id, 0, 100).unwrap().total, 7);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn previsao_nao_e_confiada_ao_frontend_e_texto_vem_so_do_snapshot() {
    let (_raiz, mut banco, doc) = preparado();
    let d = banco
        .criar_dataset("SINTETICO", "Autorizado teste.", "Retenção teste.")
        .unwrap();
    assert!(banco
        .adicionar_amostra(
            d.id,
            &pedido(doc, "tp@example.invalid", true, false, Estado::Confirmado)
        )
        .is_err());
    assert!(banco
        .adicionar_amostra(
            d.id,
            &pedido(doc, "MISSED-SINTETICO", true, true, Estado::Confirmado)
        )
        .is_err());
    let mut p = pedido(doc, "tp@example.invalid", true, true, Estado::Confirmado);
    p.inicio_utf8 = 5;
    assert!(banco.adicionar_amostra(d.id, &p).is_err());
    let a = banco
        .adicionar_amostra(
            d.id,
            &pedido(doc, "tp@example.invalid", true, true, Estado::Confirmado),
        )
        .unwrap();
    assert_eq!(a.trecho, "tp@example.invalid");
    assert_eq!(a.tipos_previstos, vec!["email"]);
    assert_eq!(a.sha256_amostra.len(), 64);
    let audit = banco.auditoria(1).unwrap().remove(0);
    assert!(!audit.detalhes.to_string().contains(&a.trecho));
}

#[test]
fn denominador_zero_permanece_desconhecido_em_vez_de_metricas_fabricadas() {
    let (_raiz, mut banco, doc) = preparado();
    let d = banco
        .criar_dataset("SINTETICO", "Autorizado teste.", "Retenção teste.")
        .unwrap();
    let vazio = banco.avaliar_dataset(d.id).unwrap();
    assert_eq!(vazio.precision, None);
    assert_eq!(vazio.recall, None);
    assert_eq!(vazio.f1, None);
    assert_eq!(vazio.acuracia, None);
    assert_eq!(vazio.cobertura, None);
    banco
        .adicionar_amostra(
            d.id,
            &pedido(doc, "tp@example.invalid", true, true, Estado::NaoRevisado),
        )
        .unwrap();
    let a = banco.avaliar_dataset(d.id).unwrap();
    assert_eq!(a.elegiveis, 0);
    assert_eq!(a.precision, None);
    assert_eq!(a.cobertura, Some(0.0));
}

#[test]
fn auditoria_atomicidade_immutabilidade_e_cancelamento_preservados() {
    let (_raiz, mut banco, doc) = preparado();
    let d = banco
        .criar_dataset("SINTETICO", "Autorizado teste.", "Retenção teste.")
        .unwrap();
    let c = Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap();
    c.execute_batch("CREATE TRIGGER falha_sintetica_amostra BEFORE INSERT ON auditoria
        WHEN NEW.acao='amostra_supervisionada_adicionada' BEGIN SELECT RAISE(ABORT,'falha sintética'); END;").unwrap();
    assert!(banco
        .adicionar_amostra(
            d.id,
            &pedido(doc, "tp@example.invalid", true, true, Estado::Confirmado)
        )
        .is_err());
    assert_eq!(banco.amostras_dataset(d.id, 0, 100).unwrap().total, 0);
    c.execute_batch("DROP TRIGGER falha_sintetica_amostra;")
        .unwrap();
    banco
        .adicionar_amostra(
            d.id,
            &pedido(doc, "tp@example.invalid", true, true, Estado::Confirmado),
        )
        .unwrap();
    for tabela in ["dataset_supervisionado", "amostra_supervisionada"] {
        assert!(c
            .execute(&format!("UPDATE {tabela} SET id=id+100"), [])
            .is_err());
        assert!(c.execute(&format!("DELETE FROM {tabela}"), []).is_err());
    }
    let controle = ControleDescoberta::novo();
    controle.cancelar();
    assert!(matches!(
        banco.avaliar_dataset_com_controle(d.id, &controle),
        Err(ErroArmazenamento::DescobertaCancelada)
    ));
    assert_eq!(banco.avaliar_dataset(d.id).unwrap().elegiveis, 1);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn adulteracao_da_amostra_nao_vira_resultado_de_avaliacao() {
    let (_raiz, mut banco, doc) = preparado();
    let d = banco
        .criar_dataset("SINTETICO", "Autorizado teste.", "Retenção teste.")
        .unwrap();
    banco
        .adicionar_amostra(
            d.id,
            &pedido(doc, "tp@example.invalid", true, true, Estado::Confirmado),
        )
        .unwrap();
    let c = Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap();
    // Corrupção deliberada SOMENTE neste banco temporário sintético.
    c.execute_batch("DROP TRIGGER amostra_supervisionada_sem_update; UPDATE amostra_supervisionada SET esperado=0;").unwrap();
    assert!(matches!(
        banco.avaliar_dataset(d.id),
        Err(ErroArmazenamento::Inconsistente(_))
    ));
    assert!(banco.amostras_dataset(d.id, 0, 100).is_err());
}
