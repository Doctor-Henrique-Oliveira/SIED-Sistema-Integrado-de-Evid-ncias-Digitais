//! Configurações e consultas AI-0, somente com extração SINTÉTICA temporária.
use rusqlite::Connection;
use sied_armazenamento::busca::FiltroBusca;
use sied_armazenamento::controle_descoberta::ControleDescoberta;
use sied_armazenamento::descoberta::{ModoDescoberta, PedidoDescoberta};
use sied_armazenamento::pesquisa_local::{RamoVocabulario, VocabularioPesquisa};
use sied_armazenamento::{BancoCaso, ErroArmazenamento};

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod sintetico;

fn vocabulario() -> VocabularioPesquisa {
    VocabularioPesquisa {
        nome: "Vocabulário SINTÉTICO".into(),
        origem: "Criado para teste; sem significado criminal".into(),
        ramos: vec![RamoVocabulario {
            nome: "Grafias de teste".into(),
            habilitado: true,
            termos: vec!["amanhã".into()],
        }],
    }
}

fn pedido() -> PedidoDescoberta {
    PedidoDescoberta {
        filtro: FiltroBusca {
            texto: "encontro".into(),
            ..Default::default()
        },
        modo: ModoDescoberta::Expansao,
        termos: vec!["amanhã".into()],
        indicador: None,
    }
}

#[test]
fn versoes_consultas_e_exportacao_sao_auditadas_sem_expor_termos_no_log() {
    let (_raiz, origem, mut banco, _x) = sintetico::preparado();
    let antes = sintetico::retrato(&origem);
    let original = banco.buscar(&pedido().filtro, 0, 50).unwrap();
    let v1 = banco.salvar_vocabulario_pesquisa(vocabulario()).unwrap();
    let v2 = banco.salvar_vocabulario_pesquisa(vocabulario()).unwrap();
    assert_eq!(v1.versao, 1);
    assert_eq!(v2.versao, 2);
    assert_ne!(v1.id, v2.id);
    assert_ne!(v1.sha256, v2.sha256); // A versão e sua proveniência entram no hash.
    let exportado: serde_json::Value =
        serde_json::from_str(&banco.exportar_vocabulario_pesquisa(v1.id).unwrap()).unwrap();
    assert_eq!(exportado["formato"], "sied-vocabulario-pesquisa-v1");
    assert_eq!(exportado["dados"]["nome"], "Vocabulário SINTÉTICO");
    let exec = banco.executar_descoberta(pedido(), Some(v1.id)).unwrap();
    assert_eq!(exec.consulta.vocabulario_id, Some(v1.id));
    assert_eq!(exec.consulta.versao_indice, 3);
    assert_eq!(
        exec.consulta.termos_efetivos,
        exec.resultado.termos_efetivos
    );
    let eventos_antes = banco.verificar_auditoria().unwrap().registros;
    let pagina = banco.paginar_descoberta(exec.consulta.id, 0, 50).unwrap();
    assert_eq!(pagina.artefatos.total, exec.resultado.artefatos.total);
    assert_eq!(pagina.executada_em, exec.consulta.criada_em);
    assert_eq!(
        banco.verificar_auditoria().unwrap().registros,
        eventos_antes
    );
    assert_eq!(
        banco.listar_consultas_descoberta().unwrap()[0].sha256,
        exec.consulta.sha256
    );
    let detalhes = banco
        .auditoria(20)
        .unwrap()
        .into_iter()
        .filter(|a| {
            a.acao.starts_with("consulta_descoberta") || a.acao.starts_with("vocabulario_pesquisa")
        })
        .map(|a| a.detalhes.to_string())
        .collect::<String>();
    assert!(!detalhes.contains("encontro"));
    assert!(!detalhes.contains("amanhã"));
    assert!(!detalhes.contains("Vocabulário SINTÉTICO"));
    assert_eq!(sintetico::retrato(&origem), antes);
    assert!(banco.conferir_derivados().unwrap().consistente);
    let depois = banco.buscar(&pedido().filtro, 0, 50).unwrap();
    assert_eq!(original.artefatos.total, depois.artefatos.total);
    for a in exec.resultado.artefatos.itens {
        assert!(banco
            .conferir_registro(a.achado.artefato.id)
            .unwrap()
            .confere());
    }
}

#[test]
fn configuracao_invalida_e_vocabulario_incompativel_nao_gravam_nem_auditam() {
    let (_raiz, _origem, mut banco, _x) = sintetico::preparado();
    let v = banco.salvar_vocabulario_pesquisa(vocabulario()).unwrap();
    let n = banco.verificar_auditoria().unwrap().registros;
    let mut invalido = vocabulario();
    invalido.ramos[0].termos = vec!["x".repeat(201)];
    assert!(banco.salvar_vocabulario_pesquisa(invalido).is_err());
    let mut p = pedido();
    p.termos.push("termo não presente na versão".into());
    assert!(banco.executar_descoberta(p, Some(v.id)).is_err());
    assert!(banco.exportar_vocabulario_pesquisa(-1).is_err());
    assert_eq!(banco.verificar_auditoria().unwrap().registros, n);
    assert!(banco.listar_consultas_descoberta().unwrap().is_empty());
}

#[test]
fn descoberta_cancelada_nao_publica_snapshot_nem_auditoria_e_conexao_continua_utilizavel() {
    let (_raiz, origem, mut banco, x) = sintetico::preparado();
    let v = banco.salvar_vocabulario_pesquisa(vocabulario()).unwrap();
    let original = pedido();
    let pedido_antes = serde_json::to_value(&original).unwrap();
    let origem_antes = sintetico::retrato(&origem);
    let extracao_antes = serde_json::to_value(banco.extracao(x.id).unwrap()).unwrap();
    let artefatos = banco.artefatos(x.id, 0, 500).unwrap();
    assert_eq!(artefatos.total, artefatos.itens.len() as i64);
    let artefatos_antes = serde_json::to_value(&artefatos).unwrap();
    let hashes_antes: Vec<_> = artefatos
        .itens
        .iter()
        .map(|a| {
            let conferencia = banco.conferir_registro(a.id).unwrap();
            assert!(conferencia.confere());
            (a.id, conferencia.gravado)
        })
        .collect();
    let auditoria_antes = banco.verificar_auditoria().unwrap();

    let controle = ControleDescoberta::novo();
    assert!(controle.cancelar());
    let cancelada = banco.executar_descoberta_com_controle(original.clone(), Some(v.id), &controle);
    assert!(matches!(
        cancelada,
        Err(ErroArmazenamento::DescobertaCancelada)
    ));
    assert!(banco.listar_consultas_descoberta().unwrap().is_empty());
    let auditoria_depois = banco.verificar_auditoria().unwrap();
    assert_eq!(auditoria_depois.registros, auditoria_antes.registros);
    assert_eq!(auditoria_depois.hash_final, auditoria_antes.hash_final);
    assert_eq!(serde_json::to_value(&original).unwrap(), pedido_antes);
    assert_eq!(sintetico::retrato(&origem), origem_antes);
    assert_eq!(
        serde_json::to_value(banco.extracao(x.id).unwrap()).unwrap(),
        extracao_antes
    );
    assert_eq!(
        serde_json::to_value(banco.artefatos(x.id, 0, 500).unwrap()).unwrap(),
        artefatos_antes
    );
    for (id, hash_antes) in hashes_antes {
        let conferencia = banco.conferir_registro(id).unwrap();
        assert!(conferencia.confere());
        assert_eq!(conferencia.gravado, hash_antes);
    }
    assert!(banco.conferir_derivados().unwrap().consistente);

    // Um controle novo não herda o cancelamento e a transação anterior não fica aberta.
    let nova = banco
        .executar_descoberta_com_controle(original, Some(v.id), &ControleDescoberta::novo())
        .unwrap();
    assert_eq!(banco.listar_consultas_descoberta().unwrap().len(), 1);
    assert_eq!(nova.consulta.vocabulario_id, Some(v.id));
    assert_eq!(
        banco.verificar_auditoria().unwrap().registros,
        auditoria_antes.registros + 1
    );
    assert!(banco.paginar_descoberta(nova.consulta.id, 0, 50).is_ok());
}

#[test]
fn paginacao_cancelada_preserva_snapshot_e_auditoria_sem_contaminar_nova_pagina() {
    let (_raiz, origem, mut banco, _x) = sintetico::preparado();
    let exec = banco.executar_descoberta(pedido(), None).unwrap();
    let origem_antes = sintetico::retrato(&origem);
    let consultas_antes =
        serde_json::to_value(banco.listar_consultas_descoberta().unwrap()).unwrap();
    let auditoria_antes = banco.verificar_auditoria().unwrap();
    let controle = ControleDescoberta::novo();
    assert!(controle.cancelar());

    assert!(matches!(
        banco.paginar_descoberta_com_controle(exec.consulta.id, 0, 50, &controle),
        Err(ErroArmazenamento::DescobertaCancelada)
    ));
    assert_eq!(
        serde_json::to_value(banco.listar_consultas_descoberta().unwrap()).unwrap(),
        consultas_antes
    );
    let auditoria_depois = banco.verificar_auditoria().unwrap();
    assert_eq!(auditoria_depois.registros, auditoria_antes.registros);
    assert_eq!(auditoria_depois.hash_final, auditoria_antes.hash_final);
    assert_eq!(sintetico::retrato(&origem), origem_antes);

    let pagina = banco
        .paginar_descoberta_com_controle(exec.consulta.id, 0, 50, &ControleDescoberta::novo())
        .unwrap();
    assert_eq!(
        serde_json::to_value(pagina).unwrap(),
        serde_json::to_value(exec.resultado).unwrap()
    );
    let auditoria_final = banco.verificar_auditoria().unwrap();
    assert_eq!(auditoria_final.registros, auditoria_antes.registros);
    assert_eq!(auditoria_final.hash_final, auditoria_antes.hash_final);
    assert!(banco.conferir_derivados().unwrap().consistente);
}

#[test]
fn snapshots_e_versoes_sao_somente_inclusao_e_persistem_ao_reabrir() {
    let (_raiz, _origem, mut banco, _x) = sintetico::preparado();
    let v = banco.salvar_vocabulario_pesquisa(vocabulario()).unwrap();
    let exec = banco.executar_descoberta(pedido(), Some(v.id)).unwrap();
    let pasta = banco.pasta().to_path_buf();
    banco.fechar().unwrap();
    let conn = Connection::open(pasta.join("caso.sied.db")).unwrap();
    for tabela in ["vocabulario_pesquisa", "consulta_descoberta"] {
        assert!(conn
            .execute(
                &format!("UPDATE {tabela} SET sha256 = ?1"),
                ["0".repeat(64)]
            )
            .is_err());
        assert!(conn.execute(&format!("DELETE FROM {tabela}"), []).is_err());
    }
    drop(conn);
    let banco = BancoCaso::abrir(&pasta, sintetico::sessao()).unwrap();
    assert_eq!(banco.listar_vocabularios_pesquisa().unwrap()[0].id, v.id);
    assert_eq!(
        banco.listar_consultas_descoberta().unwrap()[0].id,
        exec.consulta.id
    );
    assert!(banco.paginar_descoberta(exec.consulta.id, 0, 50).is_ok());
    banco.verificar_auditoria().unwrap();
}

#[test]
fn migracao_v6_cria_backup_sem_editar_fatos_ou_indice() {
    let (_raiz, _origem, mut banco, _x) = sintetico::preparado();
    let v = banco.salvar_vocabulario_pesquisa(vocabulario()).unwrap();
    let pasta = banco.pasta().to_path_buf();
    let encontrados = banco
        .buscar(&pedido().filtro, 0, 50)
        .unwrap()
        .artefatos
        .total;
    banco.fechar().unwrap();
    let conn = Connection::open(pasta.join("caso.sied.db")).unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = OFF;
        DROP TABLE visual_derivado;
        DROP TABLE amostra_supervisionada;
        DROP TABLE dataset_supervisionado;
        DROP TABLE revisao_entidade_documental;
        DROP TABLE busca_entidade_documental;
        DROP TABLE entidade_documental;
        DROP TABLE busca_documento;
        DROP TABLE documento_derivado;
        DROP TABLE consulta_descoberta;
        DROP TABLE vocabulario_pesquisa;
        DELETE FROM schema_migracao WHERE versao >= 7;
        PRAGMA user_version = 6;
        PRAGMA foreign_keys = ON;",
    )
    .unwrap();
    drop(conn);
    let banco = BancoCaso::abrir(&pasta, sintetico::sessao()).unwrap();
    assert!(pasta.join("caso.sied.db.antes-v7.bak").is_file());
    assert!(banco.listar_vocabularios_pesquisa().unwrap().is_empty());
    assert!(banco.paginar_descoberta(v.id, 0, 50).is_err());
    assert_eq!(
        banco
            .buscar(&pedido().filtro, 0, 50)
            .unwrap()
            .artefatos
            .total,
        encontrados
    );
    banco.verificar_auditoria().unwrap();
}

#[test]
fn adulteracao_de_derivado_e_denunciada_sem_confiar_so_no_hash_armazenado() {
    let (_raiz, _origem, mut banco, _x) = sintetico::preparado();
    banco.salvar_vocabulario_pesquisa(vocabulario()).unwrap();
    let pasta = banco.pasta().to_path_buf();
    banco.fechar().unwrap();
    let conn = Connection::open(pasta.join("caso.sied.db")).unwrap();
    conn.execute_batch(
        "DROP TRIGGER vocabulario_pesquisa_sem_update;
        UPDATE vocabulario_pesquisa SET dados = '{}' WHERE id = 1;",
    )
    .unwrap();
    drop(conn);
    let banco = BancoCaso::abrir(&pasta, sintetico::sessao()).unwrap();
    assert!(banco.listar_vocabularios_pesquisa().is_err());
}

#[test]
fn mudar_ramos_exige_uma_nova_versao_e_nao_muda_a_consulta_anterior() {
    let (_raiz, _origem, mut banco, _x) = sintetico::preparado();
    let v1 = banco.salvar_vocabulario_pesquisa(vocabulario()).unwrap();
    let primeira = banco.executar_descoberta(pedido(), Some(v1.id)).unwrap();
    let mut p = pedido();
    p.termos.clear();
    let antes = banco.verificar_auditoria().unwrap().registros;
    assert!(banco.executar_descoberta(p.clone(), Some(v1.id)).is_err());
    assert_eq!(banco.verificar_auditoria().unwrap().registros, antes);
    let mut dados = vocabulario();
    dados.ramos[0].habilitado = false;
    let v2 = banco.salvar_vocabulario_pesquisa(dados).unwrap();
    let segunda = banco.executar_descoberta(p, Some(v2.id)).unwrap();
    assert_eq!(segunda.consulta.vocabulario_id, Some(v2.id));
    assert_eq!(primeira.consulta.pedido.termos, vec!["amanhã"]);
    assert!(segunda.consulta.pedido.termos.is_empty());
    assert_eq!(banco.listar_consultas_descoberta().unwrap().len(), 2);
    assert!(banco
        .paginar_descoberta(primeira.consulta.id, 0, 50)
        .is_ok());
}

#[test]
fn hash_do_vocabulario_confere_tambem_a_proveniencia() {
    let (_raiz, _origem, mut banco, _x) = sintetico::preparado();
    banco.salvar_vocabulario_pesquisa(vocabulario()).unwrap();
    let pasta = banco.pasta().to_path_buf();
    banco.fechar().unwrap();
    // Somente banco temporário SINTÉTICO: simula alteração fora dos controles do SIED.
    let conn = Connection::open(pasta.join("caso.sied.db")).unwrap();
    conn.execute_batch(
        "DROP TRIGGER vocabulario_pesquisa_sem_update;
        UPDATE vocabulario_pesquisa SET autor = 'Outro autor SINTÉTICO' WHERE id = 1;",
    )
    .unwrap();
    drop(conn);
    let banco = BancoCaso::abrir(&pasta, sintetico::sessao()).unwrap();
    assert!(banco.listar_vocabularios_pesquisa().is_err());
}
