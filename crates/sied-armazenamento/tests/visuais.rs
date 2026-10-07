//! Persistência e busca visual sobre extração exclusivamente SINTÉTICA.
//! As assinaturas deste teste isolam a camada de armazenamento; decodificação
//! real PNG/JPEG é exercitada separadamente pelos testes do motor sied-documentos.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use rusqlite::Connection;
use sied_armazenamento::caso::NOME_ARQUIVO_CASO;
use sied_armazenamento::visuais::VisualIndexado;
use sied_armazenamento::{BancoCaso, ControleDescoberta, ErroArmazenamento};

fn preparado() -> (
    tempfile::TempDir,
    BancoCaso,
    VisualIndexado,
    VisualIndexado,
    i64,
) {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("VISUAL-SINTETICA");
    comum::extracao(&origem);
    std::fs::write(origem.join("copia-sintetica.png"), comum::FOTO).unwrap();
    let (banco, dispositivo) = comum::caso(raiz.path());
    let extracao = comum::importar(&banco, &dispositivo, &origem);
    let arquivos = banco
        .arquivos(extracao.id, None, None, 0, 500)
        .unwrap()
        .itens;
    let procurar = |caminho: &str| arquivos.iter().find(|a| a.caminho == caminho).unwrap();
    let foto = procurar(comum::CAMINHO_FOTO);
    let jpeg = procurar(&format!(
        "{}/WhatsApp Images/IMG-20260102-WA0001.jpg",
        comum::MIDIA
    ));
    let copia = procurar("copia-sintetica.png");
    assert_eq!(foto.sha256, copia.sha256);
    let montar = |arquivo: &sied_armazenamento::extracoes::ArquivoExtracao| VisualIndexado {
        id: 0,
        caso_id: banco.caso().id,
        extracao_id: extracao.id,
        arquivo_id: arquivo.id,
        caminho: arquivo.caminho.clone(),
        sha256_origem: arquivo.sha256.clone().unwrap(),
        dhash: "0000000000000000".into(),
        largura: 2,
        altura: 2,
        luminancia_media: 32.0,
        motor_id: "teste_armazenamento_sintetico".into(),
        motor_versao: "1".into(),
        executado_em: chrono::Utc::now().to_rfc3339(),
        hash_derivado: String::new(),
    };
    let foto_descritor = montar(foto);
    let outro_descritor = montar(jpeg);
    let copia_id = copia.id;
    (raiz, banco, foto_descritor, outro_descritor, copia_id)
}

#[test]
fn cache_versionado_e_busca_priorizam_hash_exato_sem_alterar_extracao() {
    let (raiz, mut banco, foto, outro, copia_id) = preparado();
    let origem = raiz
        .path()
        .join("VISUAL-SINTETICA")
        .join(comum::CAMINHO_FOTO);
    let antes = std::fs::read(&origem).unwrap();
    let salvo = banco.guardar_visual(foto.clone()).unwrap();
    let repetido = banco.guardar_visual(foto).unwrap();
    assert_eq!(salvo.id, repetido.id);
    assert_eq!(salvo.hash_derivado, repetido.hash_derivado);
    let proxima_versao = VisualIndexado {
        motor_versao: "2".into(),
        ..outro
    };
    let semelhante = banco.guardar_visual(proxima_versao.clone()).unwrap();
    assert!(semelhante.id > salvo.id);
    // O mesmo dHash sintético demonstra empate técnico; hash idêntico deve vencer.
    let outra_versao_foto = VisualIndexado {
        motor_versao: "2".into(),
        ..salvo.clone()
    };
    let referencia = banco.guardar_visual(outra_versao_foto).unwrap();
    let resultados = banco
        .semelhantes_visuais(referencia.id, 0, 2, &ControleDescoberta::novo())
        .unwrap();
    assert_eq!(resultados.itens.len(), 2);
    assert_eq!(resultados.itens[0].arquivo_id, copia_id);
    assert_eq!(resultados.itens[0].mecanismo, "sha256_exato");
    assert_eq!(resultados.itens[0].score_tecnico, None);
    assert_eq!(resultados.itens[1].arquivo_id, semelhante.arquivo_id);
    assert_eq!(resultados.itens[1].mecanismo, "dhash64_candidato");
    assert_eq!(resultados.itens[1].distancia_bits, Some(0));
    assert_eq!(std::fs::read(origem).unwrap(), antes);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn descritor_adulterado_ou_sem_origem_confere_erro_e_nao_vira_candidato() {
    let (raiz, mut banco, mut foto, outro, _) = preparado();
    foto.sha256_origem = "00".repeat(32);
    assert!(banco.guardar_visual(foto.clone()).is_err());
    foto.caso_id = sied_dominio::CasoId::novo();
    assert!(banco.guardar_visual(foto).is_err());
    let salvo = banco.guardar_visual(outro).unwrap();
    let conn = Connection::open(raiz.path().join("Casos/IP-7").join(NOME_ARQUIVO_CASO)).unwrap();
    assert!(conn
        .execute(
            "UPDATE visual_derivado SET dhash='ffffffffffffffff' WHERE id=?1",
            [salvo.id]
        )
        .is_err());
    assert!(conn
        .execute("DELETE FROM visual_derivado WHERE id=?1", [salvo.id])
        .is_err());
    assert!(matches!(
        banco.semelhantes_visuais(salvo.id, 0, 5, &{
            let c = ControleDescoberta::novo();
            c.cancelar();
            c
        }),
        Err(ErroArmazenamento::DescobertaCancelada)
    ));
    banco.verificar_auditoria().unwrap();
}
