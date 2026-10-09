//! Persistência AI-1 exclusivamente SINTÉTICA: origem criada antes da importação.
//! Nenhuma extração real, banco de produção ou fixture original é alterado.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use sied_armazenamento::caso::NOME_ARQUIVO_CASO;
use sied_armazenamento::documentos::{
    EstadoRevisaoEntidade as Estado, NovaEntidadeDocumento, NovaRegiaoDocumento, NovoDocumento,
    MAX_ENTIDADES_DOCUMENTO, TAMANHO_MAXIMO_TEXTO,
};
use sied_armazenamento::{BancoCaso, ErroArmazenamento};
use std::fs;

const TEXTO: &str =
    "SINTÉTICO 🧪\nContato demo@example.invalid e IP 192.0.2.17.\nNenhum dado é real.";

fn hash(texto: &str) -> String {
    hex::encode(Sha256::digest(texto.as_bytes()))
}

fn candidato(tipo: &str, literal: &str) -> NovaEntidadeDocumento {
    let inicio = TEXTO.find(literal).unwrap();
    NovaEntidadeDocumento {
        tipo: tipo.into(),
        inicio_utf8: inicio.try_into().unwrap(),
        fim_utf8: (inicio + literal.len()).try_into().unwrap(),
        literal: literal.into(),
        regra_versao: "regra-sintetica/1".into(),
    }
}

fn preparado() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    BancoCaso,
    NovoDocumento,
) {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("DOCUMENTOS-SINTETICOS");
    comum::extracao(&origem);
    fs::write(origem.join("documento-sintetico.txt"), TEXTO).unwrap();
    let (banco, dispositivo) = comum::caso(raiz.path());
    let extracao = comum::importar(&banco, &dispositivo, &origem);
    let arquivo = banco
        .arquivos(extracao.id, None, None, 0, 500)
        .unwrap()
        .itens
        .into_iter()
        .find(|a| a.caminho == "documento-sintetico.txt")
        .unwrap();
    let novo = NovoDocumento {
        caso_id: banco.caso().id,
        extracao_id: extracao.id,
        arquivo_id: arquivo.id,
        caminho: arquivo.caminho,
        sha256_origem: arquivo.sha256.unwrap(),
        sha256_texto: hash(TEXTO),
        texto: TEXTO.into(),
        pagina: None,
        total_paginas: None,
        largura_px: None,
        altura_px: None,
        regioes: vec![],
        motor_id: "texto_utf8_nativo".into(),
        motor_versao: "0.1.0".into(),
        executado_em: chrono::Utc::now().to_rfc3339(),
        entidades_limitadas: false,
        entidades_motor_versao: "motor-entidades-sintetico/1".into(),
        entidades: vec![
            candidato("email", "demo@example.invalid"),
            candidato("ip", "192.0.2.17"),
        ],
    };
    (raiz, origem, banco, novo)
}

fn conn(banco: &BancoCaso) -> Connection {
    Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap()
}

fn contar(c: &Connection, tabela: &str) -> i64 {
    c.query_row(&format!("SELECT COUNT(*) FROM {tabela}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn snapshot_pesquisa_candidatos_e_auditoria_preservam_origem_e_fatos() {
    let (_raiz, origem, mut banco, novo) = preparado();
    let retrato = comum::retrato(&origem);
    let inventario = serde_json::to_value(
        banco
            .arquivos(novo.extracao_id, None, None, 0, 500)
            .unwrap(),
    )
    .unwrap();
    let registros = banco.registros_que_nao_conferem(None).unwrap();
    let evidencias = banco.evidencias().unwrap().len();
    let c = conn(&banco);
    let fatos = contar(&c, "artefato");
    let documento = banco.guardar_documento(&novo).unwrap();
    assert_eq!(documento.texto, TEXTO);
    assert_eq!(documento.caso_id, novo.caso_id);
    assert_eq!(documento.sha256_origem, novo.sha256_origem);
    assert_eq!(documento.sha256_texto, hash(&documento.texto));
    assert_eq!(documento.pagina, None);
    assert_eq!(documento.total_paginas, None);
    assert_eq!(banco.obter_documento(documento.id).unwrap().texto, TEXTO);
    let entidades = banco.entidades_documento(documento.id).unwrap();
    assert_eq!(entidades.len(), 2);
    for e in &entidades {
        assert_eq!(e.estado, Estado::NaoRevisado);
        assert_eq!(
            documento
                .texto
                .get(e.inicio_utf8 as usize..e.fim_utf8 as usize),
            Some(e.literal.as_str())
        );
        assert!(e.revisor.is_none());
    }
    let achados = banco.buscar_documentos("SINTETICO", 0, 50).unwrap();
    assert_eq!(achados.total, 1);
    assert_eq!(achados.itens[0].id, documento.id);
    assert!(achados.itens[0].trecho.contains("SINTÉTICO"));
    assert!(achados.itens[0].trecho.chars().count() <= 400);
    assert_eq!(banco.buscar_documentos("example", 0, 50).unwrap().total, 1);
    assert_eq!(comum::retrato(&origem), retrato);
    assert_eq!(
        serde_json::to_value(
            banco
                .arquivos(novo.extracao_id, None, None, 0, 500)
                .unwrap()
        )
        .unwrap(),
        inventario
    );
    assert_eq!(
        banco.registros_que_nao_conferem(None).unwrap().len(),
        registros.len()
    );
    assert_eq!(contar(&c, "artefato"), fatos);
    assert_eq!(banco.evidencias().unwrap().len(), evidencias);
    let audit = banco.auditoria(1).unwrap().remove(0);
    assert_eq!(audit.acao, "documento_derivado_indexado");
    let detalhes = audit.detalhes.to_string();
    assert!(!detalhes.contains(TEXTO));
    assert!(!detalhes.contains("demo@example.invalid"));
    assert_eq!(audit.detalhes["ocorrencias_candidatas"], 2);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn reprocessamento_idempotente_preserva_revisoes_e_nova_versao_preserva_historico() {
    let (_raiz, _origem, mut banco, mut novo) = preparado();
    let documento = banco.guardar_documento(&novo).unwrap();
    let id = banco.entidades_documento(documento.id).unwrap()[0].id;
    let confirmado = banco
        .revisar_entidade(
            id,
            Estado::Confirmado,
            Some("Ocorrência literal conferida; dado sintético."),
        )
        .unwrap();
    assert_eq!(confirmado.estado, Estado::Confirmado);
    assert_eq!(confirmado.literal, "demo@example.invalid");
    assert_eq!(confirmado.revisor.as_deref(), Some("analista.teste"));
    banco
        .revisar_entidade(
            id,
            Estado::Inconclusivo,
            Some("Revisão sintética posterior."),
        )
        .unwrap();
    novo.executado_em = chrono::Utc::now().to_rfc3339();
    let reutilizado = banco.guardar_documento(&novo).unwrap();
    assert_eq!(reutilizado.id, documento.id);
    assert_eq!(reutilizado.executado_em, documento.executado_em);
    assert_eq!(
        banco.entidades_documento(documento.id).unwrap()[0].estado,
        Estado::Inconclusivo
    );
    let revisoes = banco.revisoes_entidade(id, 0, 1).unwrap();
    assert_eq!(revisoes.total, 2);
    assert_eq!(revisoes.itens.len(), 1);
    assert_eq!(revisoes.itens[0].estado, Estado::Inconclusivo);
    assert_eq!(
        banco.revisoes_entidade(id, 1, 1).unwrap().itens[0].estado,
        Estado::Confirmado
    );
    assert_eq!(contar(&conn(&banco), "documento_derivado"), 1);
    novo.entidades_motor_versao = "motor-entidades-sintetico/2".into();
    let outra = banco.guardar_documento(&novo).unwrap();
    assert_ne!(outra.id, documento.id);
    assert_eq!(
        banco.entidades_documento(outra.id).unwrap()[0].estado,
        Estado::NaoRevisado
    );
    assert_eq!(banco.revisoes_entidade(id, 0, 100).unwrap().total, 2);
    assert_eq!(
        banco.buscar_documentos("SINTETICO", 0, 100).unwrap().total,
        2
    );
    banco.verificar_auditoria().unwrap();
}

#[test]
fn erro_de_auditoria_reverte_snapshot_fts_e_ocorrencias_atomicamente() {
    let (_raiz, _origem, mut banco, novo) = preparado();
    let c = conn(&banco);
    let antes = contar(&c, "auditoria");
    c.execute_batch(
        "CREATE TRIGGER falha_sintetica_auditoria BEFORE INSERT ON auditoria
        WHEN NEW.acao = 'documento_derivado_indexado'
        BEGIN SELECT RAISE(ABORT,'falha exclusivamente sintética'); END;",
    )
    .unwrap();
    assert!(banco.guardar_documento(&novo).is_err());
    assert_eq!(contar(&c, "documento_derivado"), 0);
    assert_eq!(contar(&c, "entidade_documental"), 0);
    assert_eq!(contar(&c, "busca_documento"), 0);
    assert_eq!(contar(&c, "auditoria"), antes);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn origem_hash_limites_e_offsets_incorretos_nao_publicam_snapshot() {
    let (_raiz, _origem, mut banco, novo) = preparado();
    let mut pedidos = Vec::new();
    let mut p = novo.clone();
    p.sha256_texto = "0".repeat(64);
    pedidos.push(p);
    let mut p = novo.clone();
    p.sha256_origem = "0".repeat(64);
    pedidos.push(p);
    let mut p = novo.clone();
    p.caso_id = sied_dominio::CasoId::novo();
    pedidos.push(p);
    let mut p = novo.clone();
    p.extracao_id = sied_dominio::ExtracaoId::novo();
    pedidos.push(p);
    let mut p = novo.clone();
    p.arquivo_id = i64::MAX;
    pedidos.push(p);
    let mut p = novo.clone();
    p.caminho = "inventado.txt".into();
    pedidos.push(p);
    let mut p = novo.clone();
    p.entidades[0].inicio_utf8 = 5;
    pedidos.push(p);
    let mut p = novo.clone();
    p.entidades[0].fim_utf8 = u32::MAX;
    pedidos.push(p);
    let mut p = novo.clone();
    p.entidades[0].literal = "dado inventado".into();
    pedidos.push(p);
    let mut p = novo.clone();
    p.entidades[0].tipo = "pessoa_comprovada".into();
    pedidos.push(p);
    let mut p = novo.clone();
    p.entidades.push(p.entidades[0].clone());
    pedidos.push(p);
    let mut p = novo.clone();
    p.entidades = vec![p.entidades[0].clone(); MAX_ENTIDADES_DOCUMENTO + 1];
    pedidos.push(p);
    let mut p = novo.clone();
    p.texto = "X".repeat(TAMANHO_MAXIMO_TEXTO + 1);
    p.sha256_texto = hash(&p.texto);
    pedidos.push(p);
    let mut p = novo.clone();
    p.executado_em = "data inventada".into();
    pedidos.push(p);
    let mut p = novo.clone();
    p.pagina = Some(0);
    p.total_paginas = Some(2);
    pedidos.push(p);
    let mut p = novo.clone();
    p.pagina = Some(2);
    p.total_paginas = Some(1);
    pedidos.push(p);
    let mut p = novo.clone();
    p.pagina = Some(1);
    pedidos.push(p);
    for p in pedidos {
        assert!(
            banco.guardar_documento(&p).is_err(),
            "pedido inválido: {p:?}"
        );
    }
    assert_eq!(contar(&conn(&banco), "documento_derivado"), 0);
    assert_eq!(
        banco.buscar_documentos("SINTETICO", 0, 50).unwrap().total,
        0
    );
}

#[test]
fn candidato_diferente_na_mesma_versao_e_hash_e_recusado_sem_apagar_revisao() {
    let (_raiz, _origem, mut banco, mut novo) = preparado();
    let documento = banco.guardar_documento(&novo).unwrap();
    let id = banco.entidades_documento(documento.id).unwrap()[0].id;
    banco
        .revisar_entidade(
            id,
            Estado::Descartado,
            Some("Descartado apenas como candidato sintético."),
        )
        .unwrap();
    novo.entidades.pop();
    assert!(banco.guardar_documento(&novo).is_err());
    assert_eq!(
        banco.entidades_documento(documento.id).unwrap()[0].estado,
        Estado::Descartado
    );
    assert_eq!(contar(&conn(&banco), "documento_derivado"), 1);
}

#[test]
fn snapshots_ocorrencias_e_revisoes_rejeitam_update_e_delete() {
    let (_raiz, _origem, mut banco, novo) = preparado();
    let documento = banco.guardar_documento(&novo).unwrap();
    let entidade = banco.entidades_documento(documento.id).unwrap()[0].id;
    banco
        .revisar_entidade(entidade, Estado::Confirmado, None)
        .unwrap();
    let c = conn(&banco);
    for tabela in [
        "documento_derivado",
        "entidade_documental",
        "revisao_entidade_documental",
    ] {
        assert!(c.execute(&format!("DELETE FROM {tabela}"), []).is_err());
        assert!(c
            .execute(&format!("UPDATE {tabela} SET id = id + 1000"), [])
            .is_err());
    }
    banco.verificar_auditoria().unwrap();
}

#[test]
fn erro_de_auditoria_na_revisao_nao_grava_decisao_parcial() {
    let (_raiz, _origem, mut banco, novo) = preparado();
    let documento = banco.guardar_documento(&novo).unwrap();
    let entidade = banco.entidades_documento(documento.id).unwrap()[0].id;
    let c = conn(&banco);
    c.execute_batch(
        "CREATE TRIGGER falha_sintetica_revisao BEFORE INSERT ON auditoria
        WHEN NEW.acao = 'entidade_documental_revisada'
        BEGIN SELECT RAISE(ABORT,'falha exclusivamente sintética'); END;",
    )
    .unwrap();
    assert!(banco
        .revisar_entidade(entidade, Estado::Confirmado, Some("Não deve persistir."))
        .is_err());
    assert_eq!(banco.revisoes_entidade(entidade, 0, 100).unwrap().total, 0);
    assert_eq!(
        banco.entidades_documento(documento.id).unwrap()[0].estado,
        Estado::NaoRevisado
    );
}

#[test]
fn conteudo_derivado_adulterado_falha_hash_e_impede_revisao() {
    let (_raiz, _origem, mut banco, novo) = preparado();
    let documento = banco.guardar_documento(&novo).unwrap();
    let entidade = banco.entidades_documento(documento.id).unwrap()[0].id;
    let c = conn(&banco);
    // Corrupção deliberada somente deste banco temporário SINTÉTICO.
    c.execute_batch(
        "DROP TRIGGER documento_sem_update;
        UPDATE documento_derivado SET texto = 'texto sintético adulterado';",
    )
    .unwrap();
    assert!(matches!(
        banco.obter_documento(documento.id),
        Err(ErroArmazenamento::Inconsistente(_))
    ));
    assert!(banco.entidades_documento(documento.id).is_err());
    assert!(banco
        .revisar_entidade(entidade, Estado::Confirmado, None)
        .is_err());
    assert_eq!(contar(&c, "revisao_entidade_documental"), 0);
}

#[test]
fn reconstrucao_corrige_somente_indice_e_preserva_snapshots_e_revisoes() {
    let (_raiz, _origem, mut banco, novo) = preparado();
    let documento = banco.guardar_documento(&novo).unwrap();
    let entidade = banco.entidades_documento(documento.id).unwrap()[0].id;
    banco
        .revisar_entidade(
            entidade,
            Estado::Confirmado,
            Some("Literal sintético conferido."),
        )
        .unwrap();
    let c = conn(&banco);
    c.execute("DELETE FROM busca_documento", []).unwrap();
    c.execute("DELETE FROM busca_entidade_documental", [])
        .unwrap();
    let antes = banco.conferir_derivados().unwrap();
    assert!(!antes.consistente);
    let indice = antes
        .tabelas
        .iter()
        .find(|t| t.tabela == "busca_documento")
        .unwrap();
    assert_eq!(indice.faltando, 1);
    banco.reconstruir_derivados().unwrap();
    assert!(banco.conferir_derivados().unwrap().consistente);
    assert_eq!(banco.buscar_documentos("example", 0, 50).unwrap().total, 1);
    assert_eq!(banco.obter_documento(documento.id).unwrap().texto, TEXTO);
    assert_eq!(
        banco.entidades_documento(documento.id).unwrap()[0].estado,
        Estado::Confirmado
    );
    assert_eq!(banco.revisoes_entidade(entidade, 0, 50).unwrap().total, 1);
    assert_eq!(contar(&c, "documento_derivado"), 1);
    assert_eq!(contar(&c, "entidade_documental"), 2);
    assert_eq!(contar(&c, "busca_entidade_documental"), 2);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn busca_escapa_operadores_paginação_e_limita_trecho_sem_tratar_comandos() {
    let (_raiz, _origem, mut banco, mut novo) = preparado();
    novo.texto = format!("{} SINTÉTICO", "X".repeat(3000));
    novo.sha256_texto = hash(&novo.texto);
    novo.entidades.clear();
    for versao in ["1", "2", "3"] {
        novo.motor_versao = versao.into();
        banco.guardar_documento(&novo).unwrap();
    }
    let r = banco.buscar_documentos("SINTETICO", 0, 1).unwrap();
    assert_eq!(r.total, 3);
    assert_eq!(r.itens.len(), 1);
    assert!(r.itens[0].trecho.chars().count() <= 400);
    assert_eq!(
        banco
            .buscar_documentos("SINTETICO", 1, 1)
            .unwrap()
            .itens
            .len(),
        1
    );
    assert_eq!(
        banco
            .buscar_documentos("SINTETICO", 99, 1)
            .unwrap()
            .itens
            .len(),
        0
    );
    for termo in [
        "\"",
        "OR",
        "SINTETICO OR inexiste",
        "'); DELETE FROM arquivo; --",
        "NEAR(SINTETICO, arquivo)",
    ] {
        assert!(banco.buscar_documentos(termo, 0, 50).is_ok());
    }
    assert!(banco.buscar_documentos("SINTETICO", -1, 50).is_err());
    assert!(banco.buscar_documentos("SINTETICO", 0, 101).is_err());
    assert!(banco.buscar_documentos(&"X".repeat(1025), 0, 50).is_err());
    assert!(banco.conferir_derivados().unwrap().consistente);
}

#[test]
fn migracao_oito_e_nova_faz_backup_e_preserva_hashes_das_anteriores() {
    let (raiz, _origem, banco, _novo) = preparado();
    let pasta = banco.pasta().to_path_buf();
    banco.fechar().unwrap();
    let c = Connection::open(pasta.join(NOME_ARQUIVO_CASO)).unwrap();
    let antigos: Vec<(i64, String)> = c
        .prepare("SELECT versao,sha256 FROM schema_migracao WHERE versao <= 7 ORDER BY versao")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    c.execute_batch(
        "DROP TABLE regiao_documental; DELETE FROM schema_migracao WHERE versao=11;
         DROP TABLE visual_derivado; DELETE FROM schema_migracao WHERE versao=10;
         DROP TABLE amostra_supervisionada; DROP TABLE dataset_supervisionado;
         DELETE FROM schema_migracao WHERE versao=9;
         DROP TABLE revisao_entidade_documental; DROP TABLE entidade_documental;
        DROP TABLE documento_derivado; DROP TABLE busca_documento; DROP TABLE busca_entidade_documental;
        DELETE FROM schema_migracao WHERE versao = 8; PRAGMA user_version = 7;",
    )
    .unwrap();
    drop(c);
    let banco = BancoCaso::abrir(&pasta, comum::sessao()).unwrap();
    assert!(pasta
        .join(format!("{NOME_ARQUIVO_CASO}.antes-v8.bak"))
        .is_file());
    let c = conn(&banco);
    let depois: Vec<(i64, String)> = c
        .prepare("SELECT versao,sha256 FROM schema_migracao WHERE versao <= 7 ORDER BY versao")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(antigos, depois);
    assert_eq!(contar(&c, "documento_derivado"), 0);
    banco.verificar_auditoria().unwrap();
    assert!(raiz.path().is_dir());
}

#[test]
fn regioes_ocr_sao_append_only_pesquisaveis_e_reutilizadas_sem_perder_origem() {
    let (_raiz, _origem, mut banco, mut novo) = preparado();
    novo.motor_id = "windows_media_ocr".into();
    novo.motor_versao = "1.0.0".into();
    novo.pagina = Some(1);
    novo.total_paginas = Some(1);
    novo.largura_px = Some(1200);
    novo.altura_px = Some(800);
    novo.regioes = vec![NovaRegiaoDocumento {
        ordem: 0,
        inicio_utf8: 0,
        fim_utf8: "SINTÉTICO".len() as u32,
        texto: "SINTÉTICO".into(),
        x: 100,
        y: 80,
        largura: 240,
        altura: 52,
        confianca: None,
    }];
    let primeiro = banco.guardar_documento(&novo).unwrap();
    let repetido = banco.guardar_documento(&novo).unwrap();
    assert_eq!(primeiro.id, repetido.id);
    assert_eq!(repetido.largura_px, Some(1200));
    assert_eq!(repetido.altura_px, Some(800));
    assert_eq!(repetido.regioes.len(), 1);
    assert_eq!(repetido.regioes[0].texto, "SINTÉTICO");
    assert_eq!(repetido.regioes[0].confianca, None);
    let encontrados = banco.buscar_documentos("SINTETICO", 0, 20).unwrap();
    assert_eq!(encontrados.total, 1);
    assert_eq!(encontrados.itens[0].motor_id, "windows_media_ocr");
    let c = conn(&banco);
    assert!(c.execute("UPDATE regiao_documental SET x=0", []).is_err());
    assert!(c.execute("DELETE FROM regiao_documental", []).is_err());
}

#[test]
fn filtros_documentais_precedem_janela_e_preservam_data_exata_do_inventario() {
    let (raiz, _origem, mut banco, novo) = preparado();
    banco.guardar_documento(&novo).unwrap();
    let origem = raiz.path().join("OUTRA-EXTRACAO-SINTETICA");
    comum::extracao(&origem);
    std::fs::write(origem.join("documento-sintetico.txt"), TEXTO).unwrap();
    let disp = banco.listar_dispositivos().unwrap().remove(0);
    let x = comum::importar(&banco, &disp, &origem);
    let arquivo = banco
        .arquivos(x.id, None, None, 0, 500)
        .unwrap()
        .itens
        .into_iter()
        .find(|a| a.caminho == "documento-sintetico.txt")
        .unwrap();
    let mut outro = novo.clone();
    outro.extracao_id = x.id;
    outro.arquivo_id = arquivo.id;
    outro.caminho = arquivo.caminho;
    outro.sha256_origem = arquivo.sha256.unwrap();
    for versao in 1..=101 {
        outro.motor_versao = format!("outra-extracao-sintetica/{versao}");
        banco.guardar_documento(&outro).unwrap();
    }
    let mut filtro = sied_armazenamento::busca::FiltroBusca {
        texto: "SINTETICO".into(),
        extracao_id: Some(novo.extracao_id),
        ..Default::default()
    };
    let resultado = banco.buscar_documentos_filtrado(&filtro, 0, 100).unwrap();
    assert_eq!(resultado.total, 1);
    assert_eq!(resultado.itens[0].arquivo_id, novo.arquivo_id);
    let a = banco
        .arquivos(novo.extracao_id, None, None, 0, 500)
        .unwrap()
        .itens
        .into_iter()
        .find(|a| a.id == novo.arquivo_id)
        .unwrap();
    let data = a.criado_em.or(a.modificado_em).unwrap();
    let ms = chrono::DateTime::parse_from_rfc3339(&data)
        .unwrap()
        .timestamp_millis();
    filtro.de_ms = Some(ms);
    filtro.ate_ms = Some(ms);
    assert_eq!(
        banco
            .buscar_documentos_filtrado(&filtro, 0, 100)
            .unwrap()
            .total,
        1
    );
    filtro.de_ms = Some(ms + 1);
    filtro.ate_ms = Some(ms + 1);
    assert_eq!(
        banco
            .buscar_documentos_filtrado(&filtro, 0, 100)
            .unwrap()
            .total,
        0
    );
    filtro.de_ms = None;
    filtro.ate_ms = None;
    filtro.app = Some("whatsapp".into());
    assert_eq!(
        banco
            .buscar_documentos_filtrado(&filtro, 0, 100)
            .unwrap()
            .total,
        0
    );
}
