//! Evidências sobre uma extração **SINTÉTICA** (ver `testes/rust/caso_sintetico.rs`).

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use comum::*;
use rusqlite::Connection;
use sied_armazenamento::caso::NOME_ARQUIVO_CASO;
use sied_armazenamento::evidencias::{
    hash_do_trecho, NovaEvidencia, TextosEvidencia, TipoEvidencia,
};
use sied_armazenamento::ErroArmazenamento;

fn recusa(r: Result<impl std::fmt::Debug, ErroArmazenamento>) -> String {
    match r {
        Err(ErroArmazenamento::Recusado(m)) => m,
        outro => panic!("esperava recusa, veio {outro:?}"),
    }
}

fn acoes(banco: &sied_armazenamento::BancoCaso) -> Vec<String> {
    let mut a: Vec<String> = banco
        .auditoria(500)
        .unwrap()
        .into_iter()
        .map(|r| r.acao)
        .collect();
    a.sort();
    a
}

#[test]
fn registro_trecho_e_arquivo_guardam_o_hash_e_vao_para_a_auditoria() {
    let (_raiz, origem, mut banco, x) = preparado();
    let antes = retrato(&origem);
    let msgs = mensagens_ana(&banco);
    let foto = mensagem(&msgs, "SINT0000000003").clone();

    // Registro: o hash é o hash_registro do artefato.
    let e1 = banco
        .adicionar_evidencia(NovaEvidencia {
            artefato_id: Some(foto.id),
            titulo: Some("  Foto enviada  ".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(e1.numero, 1);
    assert_eq!(e1.tipo, TipoEvidencia::Registro);
    assert_eq!(e1.titulo.as_deref(), Some("Foto enviada"));
    assert_eq!(
        e1.hash_no_momento,
        banco.artefato(foto.id).unwrap().hash_registro
    );
    assert!(e1.conversa_id.is_some());
    assert_eq!(e1.criada_por, "analista.teste");

    // A mesma coisa não entra duas vezes.
    let m = recusa(banco.adicionar_evidencia(NovaEvidencia {
        artefato_id: Some(foto.id),
        ..Default::default()
    }));
    assert!(m.contains("nº 1"), "{m}");

    // Trecho informado de trás para a frente: vale a ordem da conversa.
    let e2 = banco
        .adicionar_evidencia(NovaEvidencia {
            artefato_id: Some(msgs[4].id),
            trecho_fim_id: Some(msgs[0].id),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(e2.tipo, TipoEvidencia::Trecho);
    assert_eq!(e2.itens, 5);
    assert_eq!(e2.registro.as_ref().unwrap().id, msgs[0].id);
    assert_eq!(e2.registro_fim.as_ref().unwrap().id, msgs[4].id);
    let t = banco.trecho(msgs[0].id, msgs[4].id).unwrap();
    assert_eq!(
        e2.hash_no_momento,
        hash_do_trecho(t.mensagens.iter().map(|(m, h)| (m.id, h.as_str())))
    );

    // Trecho de uma mensagem só vira registro.
    let e3 = banco
        .adicionar_evidencia(NovaEvidencia {
            artefato_id: Some(msgs[5].id),
            trecho_fim_id: Some(msgs[5].id),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(e3.tipo, TipoEvidencia::Registro);

    // Arquivo: o SHA-256 do inventário (igual ao do arquivo na extração).
    let id_foto = id_arquivo(&banco, &x, CAMINHO_FOTO);
    let e4 = banco
        .adicionar_evidencia(NovaEvidencia {
            arquivo_id: Some(id_foto),
            categoria: Some("Imagens".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(e4.tipo, TipoEvidencia::Arquivo);
    assert_eq!(e4.hash_no_momento, sha256(&origem.join(CAMINHO_FOTO)));
    assert_eq!(e4.numero, 4);

    // Marcações para a tela.
    let mc = banco.marcacoes().unwrap();
    assert!(mc.registros.contains(&(foto.id, 1)));
    for m in &msgs[..5] {
        assert!(mc.registros.contains(&(m.id, 2)), "{} fora do trecho", m.id);
    }
    assert!(mc.registros.contains(&(msgs[5].id, 3)));
    assert_eq!(mc.arquivos, vec![(id_foto, 4)]);

    // Textos editáveis; alvo e hash não mudam.
    let e4b = banco
        .alterar_evidencia(
            e4.id,
            TextosEvidencia {
                titulo: Some("Foto da câmera".into()),
                observacao: Some("Observação de teste".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(e4b.titulo.as_deref(), Some("Foto da câmera"));
    assert_eq!(e4b.categoria, None, "vazio apaga");
    assert_eq!(e4b.hash_no_momento, e4.hash_no_momento);

    // Reordenar: a 4 vira a 1.
    let lista = banco.mover_evidencia(e4.id, 1).unwrap();
    let ordem: Vec<i64> = lista.iter().map(|e| e.id).collect();
    assert_eq!(ordem, vec![e4.id, e1.id, e2.id, e3.id]);
    assert_eq!(lista[0].numero, 1);

    // Remover é lógico e pode ser refeito depois.
    banco
        .remover_evidencia(e1.id, Some("marcada por engano".into()))
        .unwrap();
    let lista = banco.evidencias().unwrap();
    assert_eq!(lista.len(), 3);
    assert_eq!(
        lista.iter().map(|e| e.numero).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    let de_novo = banco
        .adicionar_evidencia(NovaEvidencia {
            artefato_id: Some(foto.id),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(de_novo.numero, 4);
    assert_ne!(de_novo.id, e1.id);

    let a = acoes(&banco);
    assert_eq!(a.iter().filter(|a| *a == "evidencia_adicionada").count(), 5);
    assert_eq!(a.iter().filter(|a| *a == "evidencia_alterada").count(), 1);
    assert_eq!(a.iter().filter(|a| *a == "evidencia_reordenada").count(), 1);
    assert_eq!(a.iter().filter(|a| *a == "evidencia_removida").count(), 1);
    banco.verificar_auditoria().unwrap();
    assert_eq!(retrato(&origem), antes, "a extração não pode mudar");
}

#[test]
fn trecho_so_dentro_de_uma_conversa_e_pedido_precisa_de_alvo() {
    let (_raiz, _origem, mut banco, _x) = preparado();
    let ana = mensagens_ana(&banco);
    let grupo = banco
        .conversas(None, 0, 50)
        .unwrap()
        .itens
        .into_iter()
        .find(|c| c.id_externo == GRUPO)
        .unwrap();
    let do_grupo = banco.mensagens_conversa(grupo.id, 0, 50).unwrap().itens;
    let r = banco.adicionar_evidencia(NovaEvidencia {
        artefato_id: Some(ana[0].id),
        trecho_fim_id: Some(do_grupo[0].id),
        ..Default::default()
    });
    assert!(matches!(r, Err(ErroArmazenamento::Recusado(_))), "{r:?}");
    let m = recusa(banco.adicionar_evidencia(NovaEvidencia::default()));
    assert!(m.contains("informe"), "{m}");
    assert!(banco.evidencias().unwrap().is_empty());
}

#[test]
fn alvo_hash_e_evidencia_removida_nao_mudam_nem_somem_do_banco() {
    let (_raiz, _origem, mut banco, _x) = preparado();
    let msgs = mensagens_ana(&banco);
    let e = banco
        .adicionar_evidencia(NovaEvidencia {
            artefato_id: Some(msgs[0].id),
            ..Default::default()
        })
        .unwrap();
    let pasta = banco.pasta().to_path_buf();
    banco.remover_evidencia(e.id, None).unwrap();
    let c = Connection::open(pasta.join(NOME_ARQUIVO_CASO)).unwrap();
    for sql in [
        "UPDATE evidencia SET hash_no_momento = 'x'",
        "UPDATE evidencia SET artefato_id = artefato_id",
        "UPDATE evidencia SET titulo = 'outro'",
        "DELETE FROM evidencia",
    ] {
        assert!(c.execute(sql, []).is_err(), "{sql} deveria falhar");
    }
    let (removida_por, hash): (String, String) = c
        .query_row(
            "SELECT removida_por, hash_no_momento FROM evidencia WHERE id = ?1",
            [e.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(removida_por, "analista.teste");
    assert_eq!(hash, e.hash_no_momento);
}
