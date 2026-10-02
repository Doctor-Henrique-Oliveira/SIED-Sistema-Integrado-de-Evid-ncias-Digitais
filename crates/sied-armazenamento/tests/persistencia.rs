//! Fluxo da fundação: criar caso, cadastrar dispositivo, fechar, reabrir e conferir.

use rusqlite::Connection;
use sied_armazenamento::caso::{nome_pasta_sugerido, NOME_ARQUIVO_CASO};
use sied_armazenamento::migracoes::{BANCO_CASO, BANCO_GLOBAL};
use sied_armazenamento::{BancoCaso, CatalogoGlobal, ErroArmazenamento, Sessao};
use sied_dominio::{NovoCaso, NovoDispositivo, SistemaOperacional};

fn sessao() -> Sessao {
    Sessao {
        usuario: "analista.teste".into(),
        maquina: "ESTACAO-TESTE".into(),
    }
}

fn novo_caso() -> NovoCaso {
    NovoCaso {
        procedimento: Some("Inquérito Policial".into()),
        numero_procedimento: Some("IP 0123/2026".into()),
        numero_bo: Some("BO 4567/2026".into()),
        investigado: Some("Fulano (dado sintético)".into()),
        vitima: None,
        delegado: Some("Dra. Exemplo".into()),
        responsavel_analise: "Analista Teste".into(),
        unidade: "DP Central".into(),
        observacoes: Some("Caso sintético de teste".into()),
    }
}

fn novo_dispositivo() -> NovoDispositivo {
    NovoDispositivo {
        apelido: "Celular 1 – investigado".into(),
        fabricante: Some("Samsung".into()),
        modelo: Some("SM-A546E".into()),
        nome_comercial: Some("Galaxy A54".into()),
        sistema: SistemaOperacional::Android,
        versao_sistema: Some("14".into()),
        imei1: Some("490154203237518".into()),
        capacidade_bytes: Some(128_000_000_000),
        ..Default::default()
    }
}

#[test]
fn criar_fechar_reabrir_preserva_dados() {
    let raiz = tempfile::tempdir().unwrap();
    let dados_app = raiz.path().join("appdata");
    let pasta_caso = raiz
        .path()
        .join("Casos")
        .join(nome_pasta_sugerido(&novo_caso()));

    // Sessão 1: cria caso e dispositivo.
    let (caso_id, disp_id) = {
        let catalogo = CatalogoGlobal::abrir(&dados_app).unwrap();
        let mut banco = BancoCaso::criar(&pasta_caso, novo_caso(), sessao()).unwrap();
        catalogo
            .registrar_caso(banco.caso(), banco.pasta())
            .unwrap();
        let disp = banco.cadastrar_dispositivo(novo_dispositivo()).unwrap();
        let ids = (banco.caso().id, disp.id);
        banco.fechar().unwrap();
        ids
    }; // catálogo e banco liberados, como ao fechar o programa

    // Nenhum arquivo WAL deve sobrar depois de fechar (pasta copiável).
    assert!(!pasta_caso.join(format!("{NOME_ARQUIVO_CASO}-wal")).exists());

    // Sessão 2: reabre tudo.
    let catalogo = CatalogoGlobal::abrir(&dados_app).unwrap();
    let casos = catalogo.listar_casos().unwrap();
    assert_eq!(casos.len(), 1);
    assert_eq!(casos[0].caso_id, caso_id);
    assert!(casos[0].disponivel);
    assert_eq!(
        casos[0].numero_procedimento.as_deref(),
        Some("IP 0123/2026")
    );

    let banco =
        BancoCaso::abrir(&catalogo.pasta_do_caso(caso_id).unwrap().unwrap(), sessao()).unwrap();
    assert_eq!(banco.caso().id, caso_id);
    assert_eq!(banco.caso().dados, novo_caso());
    let disps = banco.listar_dispositivos().unwrap();
    assert_eq!(disps.len(), 1);
    assert_eq!(disps[0].id, disp_id);
    assert_eq!(disps[0].dados, novo_dispositivo());

    // Auditoria: criado, dispositivo, fechado, aberto — cadeia íntegra.
    let acoes: Vec<String> = banco
        .auditoria(10)
        .unwrap()
        .into_iter()
        .rev()
        .map(|r| r.acao)
        .collect();
    assert_eq!(
        acoes,
        [
            "caso_criado",
            "dispositivo_cadastrado",
            "caso_fechado",
            "caso_aberto"
        ]
    );
    assert_eq!(banco.verificar_auditoria().unwrap().registros, 4);
}

#[test]
fn pasta_do_caso_copiada_continua_valida() {
    let raiz = tempfile::tempdir().unwrap();
    let original = raiz.path().join("original");
    let mut banco = BancoCaso::criar(&original, novo_caso(), sessao()).unwrap();
    banco.cadastrar_dispositivo(novo_dispositivo()).unwrap();
    let id = banco.caso().id;
    banco.fechar().unwrap();

    let copia = raiz.path().join("copia");
    std::fs::create_dir(&copia).unwrap();
    std::fs::copy(
        original.join(NOME_ARQUIVO_CASO),
        copia.join(NOME_ARQUIVO_CASO),
    )
    .unwrap();

    let banco = BancoCaso::abrir(&copia, sessao()).unwrap();
    assert_eq!(banco.caso().id, id);
    assert_eq!(banco.listar_dispositivos().unwrap().len(), 1);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn validacao_impede_gravacao() {
    let raiz = tempfile::tempdir().unwrap();
    let invalido = NovoCaso {
        unidade: "DP".into(),
        ..Default::default()
    };
    let erro = BancoCaso::criar(&raiz.path().join("c"), invalido, sessao())
        .err()
        .unwrap();
    assert!(matches!(erro, ErroArmazenamento::Validacao(_)));
    assert!(
        !raiz.path().join("c").exists(),
        "não deve criar pasta para caso inválido"
    );

    let mut banco = BancoCaso::criar(&raiz.path().join("ok"), novo_caso(), sessao()).unwrap();
    let mut d = novo_dispositivo();
    d.imei1 = Some("490154203237519".into());
    assert!(matches!(
        banco.cadastrar_dispositivo(d),
        Err(ErroArmazenamento::Validacao(_))
    ));
    assert!(banco.listar_dispositivos().unwrap().is_empty());
    assert_eq!(
        banco.auditoria(10).unwrap().len(),
        1,
        "falha não gera registro de auditoria"
    );
}

#[test]
fn recusa_pasta_nao_vazia_e_caso_existente() {
    let raiz = tempfile::tempdir().unwrap();
    let pasta = raiz.path().join("c");
    std::fs::create_dir(&pasta).unwrap();
    std::fs::write(pasta.join("outro.txt"), "x").unwrap();
    assert!(matches!(
        BancoCaso::criar(&pasta, novo_caso(), sessao()),
        Err(ErroArmazenamento::PastaNaoVazia(_))
    ));

    let pasta2 = raiz.path().join("c2");
    BancoCaso::criar(&pasta2, novo_caso(), sessao())
        .unwrap()
        .fechar()
        .unwrap();
    assert!(matches!(
        BancoCaso::criar(&pasta2, novo_caso(), sessao()),
        Err(ErroArmazenamento::CasoJaExiste(_))
    ));
}

#[test]
fn recusa_banco_de_outro_tipo() {
    let raiz = tempfile::tempdir().unwrap();
    // Um banco global não pode ser aberto como caso.
    CatalogoGlobal::abrir(raiz.path()).unwrap();
    std::fs::rename(
        raiz.path().join("sied-global.db"),
        raiz.path().join(NOME_ARQUIVO_CASO),
    )
    .unwrap();
    assert!(matches!(
        BancoCaso::abrir(raiz.path(), sessao()),
        Err(ErroArmazenamento::TipoBancoIncorreto { .. })
    ));

    // Um SQLite qualquer (ex.: um banco da evidência) também não.
    let outro = raiz.path().join("x");
    std::fs::create_dir(&outro).unwrap();
    let c = Connection::open(outro.join(NOME_ARQUIVO_CASO)).unwrap();
    c.execute_batch("CREATE TABLE message(_id INTEGER);")
        .unwrap();
    drop(c);
    let antes = std::fs::read(outro.join(NOME_ARQUIVO_CASO)).unwrap();
    assert!(matches!(
        BancoCaso::abrir(&outro, sessao()),
        Err(ErroArmazenamento::TipoBancoIncorreto { .. })
    ));
    // Recusado sem escrever nada: nem WAL, nem cabeçalho novo.
    assert_eq!(std::fs::read(outro.join(NOME_ARQUIVO_CASO)).unwrap(), antes);
    assert!(!outro.join(format!("{NOME_ARQUIVO_CASO}-wal")).exists());
}

/// Arquivo do caso vazio, truncado ou com lixo: erro claro, e o arquivo fica como está.
#[test]
fn banco_vazio_truncado_ou_invalido_e_recusado_sem_alteracao() {
    let raiz = tempfile::tempdir().unwrap();
    let pasta = raiz.path().join("c");
    BancoCaso::criar(&pasta, novo_caso(), sessao())
        .unwrap()
        .fechar()
        .unwrap();
    let arquivo = pasta.join(NOME_ARQUIVO_CASO);
    let inteiro = std::fs::read(&arquivo).unwrap();

    let casos: [(&str, Vec<u8>); 4] = [
        ("vazio", Vec::new()),
        (
            "lixo",
            b"isto nao e um banco sqlite, e so texto".repeat(200),
        ),
        ("cabecalho truncado", inteiro[..50].to_vec()),
        ("metade das paginas", inteiro[..inteiro.len() / 2].to_vec()),
    ];
    for (nome, bytes) in casos {
        std::fs::write(&arquivo, &bytes).unwrap();
        let _ = std::fs::remove_file(pasta.join(format!("{NOME_ARQUIVO_CASO}-wal")));
        let _ = std::fs::remove_file(pasta.join(format!("{NOME_ARQUIVO_CASO}-shm")));
        match BancoCaso::abrir(&pasta, sessao()) {
            Err(ErroArmazenamento::Inconsistente(m)) => {
                assert!(
                    m.contains("vazio") || m.contains("corrompido"),
                    "{nome}: {m}"
                )
            }
            Err(ErroArmazenamento::Sqlite(e)) => panic!("{nome}: erro sem tradução: {e}"),
            Err(e) => panic!("{nome}: erro inesperado: {e}"),
            Ok(_) => panic!("{nome}: abriu um banco inválido"),
        }
        assert_eq!(
            std::fs::read(&arquivo).unwrap(),
            bytes,
            "{nome}: arquivo alterado"
        );
    }
}

/// Uma cópia de segurança anterior com o mesmo nome (ex.: de uma migração que falhou)
/// não impede o caso de abrir e não é sobrescrita.
#[test]
fn copia_de_seguranca_existente_nao_bloqueia_a_migracao() {
    let raiz = tempfile::tempdir().unwrap();
    let pasta = raiz.path().join("c");
    BancoCaso::criar(&pasta, novo_caso(), sessao())
        .unwrap()
        .fechar()
        .unwrap();
    // Volta o caso para a versão 4 (sem as tabelas da migração 5).
    let c = Connection::open(pasta.join(NOME_ARQUIVO_CASO)).unwrap();
    c.execute_batch(
        "DROP TABLE consulta_descoberta; DROP TABLE vocabulario_pesquisa;
         DELETE FROM schema_migracao WHERE versao = 7;
         DROP INDEX idx_artefato_conversa_ordem;
         CREATE INDEX idx_artefato_conversa ON artefato(app, conversa_id_externo, data_hora_ms);
         DELETE FROM schema_migracao WHERE versao = 6;
         DROP TABLE evidencia; DROP TABLE relatorio; DELETE FROM schema_migracao WHERE versao = 5;
         PRAGMA user_version = 4;",
    )
    .unwrap();
    drop(c);
    let anterior = pasta.join(format!("{NOME_ARQUIVO_CASO}.antes-v5.bak"));
    std::fs::write(&anterior, b"copia anterior").unwrap();

    let banco = BancoCaso::abrir(&pasta, sessao()).unwrap();
    assert!(banco.evidencias().unwrap().is_empty());
    assert_eq!(std::fs::read(&anterior).unwrap(), b"copia anterior");
    let nova = pasta.join(format!("{NOME_ARQUIVO_CASO}.antes-v5-2.bak"));
    let v: i64 = Connection::open(&nova)
        .unwrap()
        .query_row("SELECT MAX(versao) FROM schema_migracao", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 4);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn recusa_banco_mais_novo_que_o_sied() {
    let raiz = tempfile::tempdir().unwrap();
    let pasta = raiz.path().join("c");
    BancoCaso::criar(&pasta, novo_caso(), sessao())
        .unwrap()
        .fechar()
        .unwrap();
    let c = Connection::open(pasta.join(NOME_ARQUIVO_CASO)).unwrap();
    c.execute(
        "INSERT INTO schema_migracao VALUES (?1, 'futura', 'x', '2030-01-01', '9.9.9')",
        [BANCO_CASO.versao_atual() + 1],
    )
    .unwrap();
    drop(c);
    assert!(matches!(
        BancoCaso::abrir(&pasta, sessao()),
        Err(ErroArmazenamento::VersaoMaisNova { .. })
    ));
}

#[test]
fn detecta_adulteracao_da_auditoria() {
    let raiz = tempfile::tempdir().unwrap();
    let pasta = raiz.path().join("c");
    let mut banco = BancoCaso::criar(&pasta, novo_caso(), sessao()).unwrap();
    banco.cadastrar_dispositivo(novo_dispositivo()).unwrap();
    banco.fechar().unwrap();

    // Alguém remove os gatilhos e edita a auditoria por fora do SIED.
    let c = Connection::open(pasta.join(NOME_ARQUIVO_CASO)).unwrap();
    c.execute_batch(
        "DROP TRIGGER auditoria_sem_update;
         UPDATE auditoria SET detalhes = '{\"adulterado\":true}' WHERE seq = 2;",
    )
    .unwrap();
    drop(c);

    let banco = BancoCaso::abrir(&pasta, sessao()).unwrap();
    match banco.verificar_auditoria() {
        Err(ErroArmazenamento::AuditoriaCorrompida { seq, .. }) => assert_eq!(seq, 2),
        outro => panic!("esperava auditoria corrompida, veio {outro:?}"),
    }
}

#[test]
fn auditoria_nao_aceita_update_nem_delete() {
    let raiz = tempfile::tempdir().unwrap();
    let pasta = raiz.path().join("c");
    BancoCaso::criar(&pasta, novo_caso(), sessao())
        .unwrap()
        .fechar()
        .unwrap();
    let c = Connection::open(pasta.join(NOME_ARQUIVO_CASO)).unwrap();
    assert!(c.execute("UPDATE auditoria SET acao = 'x'", []).is_err());
    assert!(c.execute("DELETE FROM auditoria", []).is_err());
}

#[test]
fn migracoes_idempotentes_e_versionadas() {
    let raiz = tempfile::tempdir().unwrap();
    for _ in 0..3 {
        CatalogoGlobal::abrir(raiz.path()).unwrap();
    }
    let c = Connection::open(raiz.path().join("sied-global.db")).unwrap();
    let (n, versao): (i64, i64) = c
        .query_row(
            "SELECT COUNT(*), (SELECT user_version FROM pragma_user_version) FROM schema_migracao",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(n, BANCO_GLOBAL.migracoes.len() as i64);
    assert_eq!(versao, BANCO_GLOBAL.versao_atual());
}

#[test]
fn nome_de_pasta_seguro() {
    let mut c = novo_caso();
    assert_eq!(nome_pasta_sugerido(&c), "IP-0123-2026");
    c.numero_procedimento = None;
    c.numero_bo = Some("  BO: 45/2026 \\ <x> ".into());
    assert_eq!(nome_pasta_sugerido(&c), "BO-45-2026-x");
    // Nomes reservados pelo Windows não viram pasta.
    for (numero, pasta) in [
        ("con", "caso-con"),
        ("NUL", "caso-NUL"),
        ("Com1", "caso-Com1"),
        ("LPT9", "caso-LPT9"),
        ("COM10", "COM10"),
        ("CONSULTA", "CONSULTA"),
    ] {
        c.numero_bo = Some(numero.into());
        assert_eq!(nome_pasta_sugerido(&c), pasta, "{numero}");
    }
}

/// Duas janelas do SIED (ou duas estações na mesma pasta de rede) não abrem o mesmo
/// caso: a segunda abertura marcaria como "interrompida" a importação que a primeira
/// ainda está fazendo.
#[test]
fn caso_aberto_nao_abre_em_outra_janela() {
    let raiz = tempfile::tempdir().unwrap();
    let pasta = raiz.path().join("Casos/IP-trava");
    let origem = raiz.path().join("Extracao-SINTETICA");
    std::fs::create_dir_all(&origem).unwrap();
    std::fs::write(origem.join("SINTETICO.txt"), "sintético").unwrap();

    let mut banco = BancoCaso::criar(&pasta, novo_caso(), sessao()).unwrap();
    let d = banco.cadastrar_dispositivo(novo_dispositivo()).unwrap();
    let info = sied_ingestao::InfoComponente {
        id: "generico".into(),
        nome: "Pasta genérica".into(),
        versao: "0".into(),
    };
    let imp = banco.iniciar_importacao(d.id, &info, &origem).unwrap();

    match BancoCaso::abrir(&pasta, sessao()) {
        Err(ErroArmazenamento::CasoEmUso(p)) => assert_eq!(p, pasta.canonicalize().unwrap()),
        Err(e) => panic!("esperava caso em uso, veio {e}"),
        Ok(_) => panic!("o mesmo caso abriu duas vezes"),
    }
    // A importação da primeira janela continua em andamento, sem registro de interrupção.
    assert_eq!(banco.extracoes().unwrap()[0].status, "em_andamento");
    assert!(banco
        .auditoria(100)
        .unwrap()
        .iter()
        .all(|r| r.acao != "importacao_interrompida"));
    let x = imp
        .finalizar(&Err(sied_ingestao::ErroIngestao::Cancelado))
        .unwrap();
    assert_eq!(x.status, "cancelada");

    // Fechado na primeira, abre na segunda.
    banco.fechar().unwrap();
    let banco = BancoCaso::abrir(&pasta, sessao()).unwrap();
    assert_eq!(banco.extracoes().unwrap()[0].status, "cancelada");
    banco.verificar_auditoria().unwrap();
    // Sem fechar (queda do programa): a trava some com o processo/arquivo.
    drop(banco);
    BancoCaso::abrir(&pasta, sessao()).unwrap();
}
