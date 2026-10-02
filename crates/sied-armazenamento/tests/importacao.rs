//! Importação de ponta a ponta sobre uma extração **SINTÉTICA** gerada no teste.
//!
//! ATENÇÃO: a pasta de extração abaixo é inventada. Só o nome `mmssms.db` e o esquema
//! da tabela `sms` seguem o Android (AOSP); nenhum dado é real e a organização das
//! pastas não representa nenhuma ferramenta de extração.

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use sied_armazenamento::busca::FiltroBusca;
use sied_armazenamento::{BancoCaso, ErroArmazenamento, Sessao};
use sied_dominio::{Dispositivo, NovoCaso, NovoDispositivo, SistemaOperacional};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::importador::Importador;
use sied_ingestao::importadores::generico::ImportadorGenerico;
use sied_ingestao::pipeline::{self, Controle};
use sied_ingestao::Registro;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn sessao() -> Sessao {
    Sessao {
        usuario: "analista.teste".into(),
        maquina: "ESTACAO-TESTE".into(),
    }
}

fn caso(raiz: &Path) -> (BancoCaso, Dispositivo) {
    let mut b = BancoCaso::criar(
        &raiz.join("Casos/IP-1"),
        NovoCaso {
            numero_procedimento: Some("IP 1/2026".into()),
            responsavel_analise: "Analista".into(),
            unidade: "DP".into(),
            ..Default::default()
        },
        sessao(),
    )
    .unwrap();
    let d = b
        .cadastrar_dispositivo(NovoDispositivo {
            apelido: "Celular sintético".into(),
            sistema: SistemaOperacional::Android,
            ..Default::default()
        })
        .unwrap();
    (b, d)
}

/// Cria a extração sintética. O `mmssms.db` fica com uma mensagem só no `-wal`,
/// como acontece em aparelhos reais quando o banco não foi consolidado.
fn extracao_sintetica(dir: &Path) {
    fs::create_dir_all(dir.join("dados/telefonia")).unwrap();
    fs::create_dir_all(dir.join("midia")).unwrap();
    fs::write(
        dir.join("SINTETICO-LEIAME.txt"),
        "Extração SINTÉTICA gerada por teste automatizado. Nenhum dado é real.",
    )
    .unwrap();
    fs::write(
        dir.join("midia/foto.bin"),
        [0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3],
    )
    .unwrap();
    fs::write(dir.join("vazio.txt"), "").unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("mmssms.db");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE sms (_id INTEGER PRIMARY KEY, thread_id INTEGER, address TEXT, person INTEGER,
             date INTEGER, date_sent INTEGER, protocol INTEGER, read INTEGER, status INTEGER, type INTEGER,
             body TEXT, service_center TEXT, seen INTEGER);
         INSERT INTO sms VALUES (1, 7, '+5511900000001', NULL, 1767225600000, 1767225590000, 0, 1, -1, 1,
             'Mensagem sintética recebida', NULL, 1);
         INSERT INTO sms VALUES (2, 7, '+5511900000001', NULL, 1767225660000, 0, NULL, 1, -1, 2,
             'Resposta sintética enviada', NULL, 1);
         PRAGMA wal_checkpoint(TRUNCATE);
         PRAGMA wal_autocheckpoint=0;
         INSERT INTO sms VALUES (3, 8, '+5511900000002', NULL, 1767225720000, 0, 0, 0, -1, 1,
             'Só no WAL (sintético)', NULL, 0);
         INSERT INTO sms VALUES (4, 8, '+5511900000002', NULL, 1767225780000, 0, 0, 0, -1, NULL,
             'Sem tipo: deve virar aviso', NULL, 0);",
    )
    .unwrap();
    // Copia banco e -wal com a conexão ainda aberta (WAL não consolidado).
    fs::copy(&db, dir.join("dados/telefonia/mmssms.db")).unwrap();
    fs::copy(
        tmp.path().join("mmssms.db-wal"),
        dir.join("dados/telefonia/mmssms.db-wal"),
    )
    .unwrap();
    drop(conn);
}

fn sha256(p: &Path) -> String {
    hex::encode(Sha256::digest(fs::read(p).unwrap()))
}

/// Hash e data de modificação de tudo na pasta, para provar que nada mudou.
fn retrato(dir: &Path) -> BTreeMap<PathBuf, (String, std::time::SystemTime)> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let m = e.metadata().unwrap().modified().unwrap();
            (e.path().to_path_buf(), (sha256(e.path()), m))
        })
        .collect()
}

fn importar(
    banco: &BancoCaso,
    d: &Dispositivo,
    origem: &Path,
    controle: &Controle,
) -> sied_armazenamento::extracoes::Extracao {
    let importador = ImportadorGenerico;
    let mut imp = banco
        .iniciar_importacao(d.id, &importador.info(), origem)
        .unwrap();
    let fonte = PastaSomenteLeitura::abrir(origem).unwrap();
    let ctx = imp.contexto().clone();
    let r = pipeline::executar(
        &fonte,
        &importador,
        &Registro::padrao(),
        &mut imp,
        controle,
        &ctx,
    );
    imp.finalizar(&r).unwrap()
}

#[test]
fn importa_inventaria_e_rastreia_ate_a_origem() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    extracao_sintetica(&origem);
    let antes = retrato(&origem);
    let (banco, d) = caso(raiz.path());

    let x = importar(&banco, &d, &origem, &Controle::default());
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    assert_eq!(x.total_arquivos, Some(5));
    assert_eq!(x.arquivos_ilegiveis, Some(0));
    assert_eq!(x.total_artefatos, Some(3));
    assert_eq!(x.total_avisos, Some(1));
    assert_eq!(x.dispositivo_id, d.id);

    // A extração não mudou (conteúdo e datas).
    assert_eq!(retrato(&origem), antes);

    // Manifesto: hash gravado confere e cada linha confere com o arquivo real.
    let manifesto = banco
        .pasta()
        .join("importacoes")
        .join(x.id.to_string())
        .join("manifesto.sha256");
    assert_eq!(
        x.hash_manifesto.as_deref(),
        Some(sha256(&manifesto).as_str())
    );
    let texto = fs::read_to_string(&manifesto).unwrap();
    assert_eq!(texto.lines().count(), 5);
    for linha in texto.lines() {
        let (hash, caminho) = linha.split_once(" *").unwrap();
        assert_eq!(hash, sha256(&origem.join(caminho)), "{caminho}");
    }
    assert!(banco
        .pasta()
        .join("importacoes")
        .join(x.id.to_string())
        .join("log-importacao.jsonl")
        .is_file());

    // Inventário e classificação pela assinatura.
    let cats = banco.totais_por_categoria(x.id).unwrap();
    let qtd = |c: &str| cats.iter().find(|t| t.categoria == c).map(|t| t.arquivos);
    assert_eq!(qtd("banco_sqlite"), Some(1));
    assert_eq!(qtd("imagem"), Some(1), "foto.bin reconhecida como JPEG");
    assert_eq!(qtd("vazio"), Some(1));
    let busca = banco.arquivos(x.id, None, Some("MMSSMS"), 0, 10).unwrap();
    assert_eq!(busca.total, 2); // banco e -wal
    let midias = banco.arquivos(x.id, Some("midia"), None, 0, 10).unwrap();
    assert_eq!(
        midias.total, 1,
        "\"midia\" = imagens, vídeos, áudios e documentos"
    );
    assert_eq!(midias.itens[0].categoria, "imagem");

    // Execução do parser.
    let ex = banco.execucoes(x.id).unwrap();
    // O mmssms.db vai também ao leitor de MMS; sem tabela pdu, nada a ler e sem aviso.
    assert_eq!(ex.len(), 2);
    assert_eq!(ex[1].parser, "android_mms");
    assert_eq!(ex[1].status, "ok");
    assert_eq!(ex[1].artefatos, Some(0));
    assert_eq!(ex[1].avisos, Some(0));
    assert_eq!(ex[0].parser, "android_sms");
    assert_eq!(ex[0].status, "parcial");
    assert_eq!(
        ex[0].arquivo_caminho.as_deref(),
        Some("dados/telefonia/mmssms.db")
    );
    assert_eq!(
        ex[0].copia_sha256.as_deref(),
        Some(sha256(&origem.join("dados/telefonia/mmssms.db")).as_str())
    );

    // Artefatos, inclusive o registro que só existia no WAL.
    let arts = banco.artefatos(x.id, 0, 10).unwrap();
    assert_eq!(arts.total, 3);
    let textos: Vec<_> = arts
        .itens
        .iter()
        .map(|a| a.conteudo.clone().unwrap())
        .collect();
    assert_eq!(
        textos,
        [
            "Mensagem sintética recebida",
            "Resposta sintética enviada",
            "Só no WAL (sintético)"
        ]
    );
    assert_eq!(arts.itens[0].remetente.as_deref(), Some("+5511900000001"));
    assert_eq!(arts.itens[0].de_mim, Some(false));
    assert_eq!(arts.itens[1].de_mim, Some(true));
    assert_eq!(arts.itens[1].destinatarios, ["+5511900000001"]);
    assert_eq!(arts.itens[0].data_hora_ms, Some(1_767_225_600_000));

    // Visão técnica: de onde o dado veio, com todos os elos.
    let det = banco.artefato(arts.itens[1].id).unwrap();
    assert_eq!(det.importador, "generico");
    assert_eq!(det.parser, "android_sms");
    assert_eq!(det.parser_versao, "1.0.0");
    assert_eq!(det.data_hora_original.as_deref(), Some("1767225660000"));
    assert_eq!(det.formato_data.as_deref(), Some("unix_milissegundos"));
    assert_eq!(det.hash_manifesto, x.hash_manifesto);
    assert_eq!(
        det.origem_caminho,
        origem.canonicalize().unwrap().display().to_string()
    );
    let r = &det.referencias[0];
    assert!(r.principal);
    assert_eq!(r.arquivo_caminho, "dados/telefonia/mmssms.db");
    assert_eq!(r.arquivo_sha256, ex[0].copia_sha256.clone().unwrap());
    assert_eq!(r.tabela.as_deref(), Some("sms"));
    assert_eq!(r.id_registro.as_deref(), Some("_id=2"));
    assert_eq!(
        r.consulta.as_deref(),
        Some("SELECT * FROM sms ORDER BY _id")
    );
    assert_eq!(r.valores_brutos["body"], "Resposta sintética enviada");
    assert_eq!(r.valores_brutos["type"], 2);
    assert_eq!(det.metadados["situacao"], "enviada");

    // A cópia de trabalho ficou na pasta do caso, não na extração.
    let copia = banco.pasta().join("cache/bancos").join(x.id.to_string());
    assert!(copia.is_dir());

    // Inventário e artefatos são somente inclusão.
    let conn = Connection::open(banco.pasta().join("caso.sied.db")).unwrap();
    assert!(conn.execute("UPDATE arquivo SET sha256 = 'x'", []).is_err());
    assert!(conn.execute("DELETE FROM artefato", []).is_err());
    assert!(conn.execute("DELETE FROM referencia_origem", []).is_err());

    // Auditoria íntegra e com o início e o fim da importação.
    let acoes: Vec<_> = banco
        .auditoria(10)
        .unwrap()
        .into_iter()
        .map(|r| r.acao)
        .collect();
    assert_eq!(acoes[0], "importacao_concluida");
    assert_eq!(acoes[1], "importacao_iniciada");
    let fim = &banco.auditoria(1).unwrap()[0];
    assert_eq!(
        fim.detalhes["hash_manifesto"],
        x.hash_manifesto.clone().unwrap()
    );
    banco.verificar_auditoria().unwrap();
}

#[test]
fn cancelamento_fica_registrado() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("ext");
    extracao_sintetica(&origem);
    let (banco, d) = caso(raiz.path());
    let controle = Controle::default();
    controle.pedir_cancelamento();
    let x = importar(&banco, &d, &origem, &controle);
    assert_eq!(x.status, "cancelada");
    assert_eq!(banco.auditoria(1).unwrap()[0].acao, "importacao_cancelada");
    banco.verificar_auditoria().unwrap();
}

#[test]
fn importacao_interrompida_e_detectada_ao_reabrir() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("ext");
    extracao_sintetica(&origem);
    let (banco, d) = caso(raiz.path());
    let pasta = banco.pasta().to_path_buf();
    let imp = banco
        .iniciar_importacao(d.id, &ImportadorGenerico.info(), &origem)
        .unwrap();
    drop(imp); // programa encerrado no meio
    banco.fechar().unwrap();

    let banco = BancoCaso::abrir(&pasta, sessao()).unwrap();
    let x = &banco.extracoes().unwrap()[0];
    assert_eq!(x.status, "interrompida");
    assert!(banco
        .auditoria(5)
        .unwrap()
        .iter()
        .any(|r| r.acao == "importacao_interrompida"));
    banco.verificar_auditoria().unwrap();
}

#[test]
fn recusa_extracao_dentro_do_caso_e_vice_versa() {
    let raiz = tempfile::tempdir().unwrap();
    let (banco, d) = caso(raiz.path());
    let info = ImportadorGenerico.info();
    let dentro = banco.pasta().join("sub");
    fs::create_dir_all(&dentro).unwrap();
    for origem in [dentro.as_path(), raiz.path()] {
        assert!(matches!(
            banco.iniciar_importacao(d.id, &info, origem),
            Err(ErroArmazenamento::OrigemInvalida(_))
        ));
    }
    assert!(matches!(
        banco.iniciar_importacao(d.id, &info, &raiz.path().join("nao-existe")),
        Err(ErroArmazenamento::OrigemInvalida(_))
    ));
}

#[test]
fn caso_v1_e_migrado_com_copia_de_seguranca() {
    // Um caso criado pela versão da fundação (só migração 1) abre e ganha as tabelas novas.
    let raiz = tempfile::tempdir().unwrap();
    let (banco, _) = caso(raiz.path());
    let pasta = banco.pasta().to_path_buf();
    banco.fechar().unwrap();
    let conn = Connection::open(pasta.join("caso.sied.db")).unwrap();
    conn.execute_batch(
        "DROP TABLE consulta_descoberta; DROP TABLE vocabulario_pesquisa;
         DELETE FROM schema_migracao WHERE versao = 7;
         DROP INDEX idx_artefato_conversa_ordem;
         CREATE INDEX idx_artefato_conversa ON artefato(app, conversa_id_externo, data_hora_ms);
         DELETE FROM schema_migracao WHERE versao = 6;
         DROP TABLE evidencia; DROP TABLE relatorio; DELETE FROM schema_migracao WHERE versao = 5;
         DELETE FROM schema_migracao WHERE versao = 4;
         DROP TABLE busca_artefato; DROP TABLE busca_arquivo; DROP TABLE conversa_participante;
         DROP TABLE conversa; DELETE FROM schema_migracao WHERE versao = 3;
         DROP TABLE referencia_origem; DROP TABLE artefato; DROP TABLE aviso_parser;
         DROP TABLE execucao_parser; DROP TABLE arquivo; DROP TABLE extracao;
         DELETE FROM schema_migracao WHERE versao = 2; PRAGMA user_version = 1;",
    )
    .unwrap();
    drop(conn);
    let banco = BancoCaso::abrir(&pasta, sessao()).unwrap();
    assert!(banco.extracoes().unwrap().is_empty());
    assert!(pasta.join("caso.sied.db.antes-v2.bak").is_file());
    banco.verificar_auditoria().unwrap();
}

fn conversas_e_busca_de(banco: &BancoCaso) -> (String, String) {
    let conversas = banco.conversas(None, 0, 50).unwrap();
    let busca = banco
        .buscar(
            &FiltroBusca {
                texto: "sintetica".into(),
                ..Default::default()
            },
            0,
            50,
        )
        .unwrap();
    (
        serde_json::to_string(&conversas).unwrap(),
        serde_json::to_string(&busca).unwrap(),
    )
}

#[test]
fn conversas_sao_reconstruidas_a_partir_dos_artefatos() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("ext");
    extracao_sintetica(&origem);
    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem, &Controle::default());

    let lista = banco.conversas(Some(x.id), 0, 10).unwrap();
    assert_eq!(lista.total, 2);
    // Mais recente primeiro: a conversa 8 (só no WAL) é a última.
    let (c8, c7) = (&lista.itens[0], &lista.itens[1]);
    assert_eq!((c8.app.as_str(), c8.id_externo.as_str()), ("sms", "8"));
    assert_eq!(c8.total_mensagens, 1);
    assert_eq!(c8.ultima_mensagem.as_deref(), Some("Só no WAL (sintético)"));
    assert_eq!(c7.id_externo, "7");
    assert_eq!(c7.total_mensagens, 2);
    assert_eq!(c7.enviadas, 1);
    assert_eq!(c7.primeira_ms, Some(1_767_225_600_000));
    assert_eq!(c7.ultima_ms, Some(1_767_225_660_000));
    assert_eq!(c7.participantes.len(), 1);
    assert_eq!(c7.participantes[0].identificador, "+5511900000001");
    assert_eq!(c7.participantes[0].mensagens, 2);

    // Mensagens em ordem cronológica, e o caminho de volta artefato → conversa.
    let msgs = banco.mensagens_conversa(c7.id, 0, 10).unwrap();
    assert_eq!(msgs.total, 2);
    assert_eq!(
        msgs.itens[0].conteudo.as_deref(),
        Some("Mensagem sintética recebida")
    );
    assert_eq!(msgs.itens[1].de_mim, Some(true));
    assert_eq!(
        banco.conversa_do_artefato(msgs.itens[1].id).unwrap(),
        Some(c7.id)
    );
    assert!(matches!(
        banco.conversa(9999),
        Err(ErroArmazenamento::NaoEncontrado(_))
    ));
}

#[test]
fn busca_global_por_texto_telefone_e_arquivo() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("ext");
    extracao_sintetica(&origem);
    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem, &Controle::default());
    let buscar = |texto: &str| {
        banco
            .buscar(
                &FiltroBusca {
                    texto: texto.into(),
                    ..Default::default()
                },
                0,
                10,
            )
            .unwrap()
    };

    // Sem acento e por prefixo ("sintetica" encontra "sintética"/"sintético").
    let r = buscar("SINTETIC");
    assert_eq!(r.modo, "texto");
    assert_eq!(r.artefatos.total, 3);
    // Duas palavras: as duas precisam aparecer.
    let r = buscar("resposta enviada");
    assert_eq!(r.artefatos.total, 1);
    let a = &r.artefatos.itens[0];
    assert_eq!(a.extracao_id, x.id);
    assert!(a.conversa_id.is_some());
    let trecho = a.trecho.as_deref().unwrap();
    assert!(
        trecho.iter().any(|p| p.destaque && p.texto == "Resposta"),
        "{trecho:?}"
    );
    let texto: String = trecho.iter().map(|p| p.texto.as_str()).collect();
    assert!(!texto.contains(['\u{1}', '\u{2}']), "{texto:?}");

    // Telefone digitado de outro jeito encontra remetente e destinatário.
    let r = buscar("(11) 90000-0001");
    assert_eq!(r.modo, "telefone");
    assert_eq!(r.artefatos.total, 2);
    // O número inteiro, só com dígitos.
    assert_eq!(buscar("5511900000002").artefatos.total, 1);

    // Nomes de arquivo do inventário.
    let r = buscar("mmssms");
    assert_eq!(r.arquivos.total, 2);
    assert!(r.arquivos.itens.iter().all(|f| f.extracao_id == x.id));

    // Texto com sintaxe de FTS não quebra a consulta.
    assert_eq!(buscar("\"OR (NEAR -").artefatos.total, 0);
    assert_eq!(buscar("   ").modo, "vazio");

    // Filtros.
    let filtrado = |f: FiltroBusca| banco.buscar(&f, 0, 10).unwrap().artefatos.total;
    let base = || FiltroBusca {
        texto: "sintetic".into(),
        ..Default::default()
    };
    assert_eq!(
        filtrado(FiltroBusca {
            app: Some("whatsapp".into()),
            ..base()
        }),
        0
    );
    assert_eq!(
        filtrado(FiltroBusca {
            de_ms: Some(1_767_225_650_000),
            ate_ms: Some(1_767_225_700_000),
            ..base()
        }),
        1
    );
    assert_eq!(
        filtrado(FiltroBusca {
            extracao_id: Some(x.id),
            tipo: Some("mensagem".into()),
            ..base()
        }),
        3
    );
}

#[test]
fn caso_v2_ganha_conversas_e_busca_do_que_ja_foi_importado() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("ext");
    extracao_sintetica(&origem);
    let (banco, d) = caso(raiz.path());
    importar(&banco, &d, &origem, &Controle::default());
    let esperado = conversas_e_busca_de(&banco);
    let pasta = banco.pasta().to_path_buf();
    banco.fechar().unwrap();

    // Volta o banco para a versão 2 (sem as tabelas derivadas).
    let conn = Connection::open(pasta.join("caso.sied.db")).unwrap();
    conn.execute_batch(
        "DROP TABLE consulta_descoberta; DROP TABLE vocabulario_pesquisa;
         DELETE FROM schema_migracao WHERE versao = 7;
         DROP INDEX idx_artefato_conversa_ordem;
         CREATE INDEX idx_artefato_conversa ON artefato(app, conversa_id_externo, data_hora_ms);
         DELETE FROM schema_migracao WHERE versao = 6;
         DROP TABLE evidencia; DROP TABLE relatorio; DELETE FROM schema_migracao WHERE versao = 5;
         DROP INDEX idx_arquivo_nome; DROP INDEX idx_artefato_id_externo;
         DELETE FROM schema_migracao WHERE versao = 4;
         DROP TRIGGER artefato_indexa_busca; DROP TRIGGER artefato_agrega_conversa;
         DROP TRIGGER arquivo_indexa_busca;
         ALTER TABLE artefato DROP COLUMN anexo_vinculo;
         DROP TABLE busca_artefato; DROP TABLE busca_arquivo; DROP TABLE conversa_participante;
         DROP TABLE conversa; DELETE FROM schema_migracao WHERE versao = 3;
         PRAGMA user_version = 2;",
    )
    .unwrap();
    drop(conn);

    let banco = BancoCaso::abrir(&pasta, sessao()).unwrap();
    assert!(pasta.join("caso.sied.db.antes-v3.bak").is_file());
    assert_eq!(conversas_e_busca_de(&banco), esperado);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn resumo_do_caso_e_identificadores_contam_o_que_foi_gravado() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("ext");
    extracao_sintetica(&origem);
    let (banco, d) = caso(raiz.path());

    // Caso sem extração: tudo zero, exceto o dispositivo.
    let vazio = banco.resumo().unwrap();
    assert_eq!(
        (vazio.dispositivos, vazio.extracoes, vazio.registros),
        (1, 0, 0)
    );
    assert!(vazio.arquivos_por_categoria.is_empty());

    let x = importar(&banco, &d, &origem, &Controle::default());
    let r = banco.resumo().unwrap();
    assert_eq!(r.extracoes, 1);
    assert_eq!(r.extracoes_concluidas, 1);
    // Os totais batem com os da própria extração.
    assert_eq!(Some(r.arquivos), x.total_arquivos);
    assert_eq!(Some(r.bytes), x.total_bytes);
    assert_eq!(Some(r.registros), x.total_artefatos);
    assert_eq!(Some(r.avisos_parser), x.total_avisos);
    assert_eq!(
        r.arquivos_por_categoria
            .iter()
            .map(|c| c.arquivos)
            .sum::<i64>(),
        r.arquivos
    );
    assert_eq!(
        r.registros_por_tipo.iter().map(|t| t.total).sum::<i64>(),
        r.registros
    );
    assert_eq!(r.conversas, banco.conversas(None, 0, 1).unwrap().total);
    assert_eq!(r.identificadores, 2);
    assert_eq!(r.execucoes_parser, 2); // SMS e MMS do mesmo mmssms.db
    assert_eq!(r.execucoes_com_problema, 0);
    assert_eq!(r.localizacoes, 0);

    let ids = banco.identificadores(None, 0, 10).unwrap();
    assert_eq!(ids.total, 2);
    let um = ids
        .itens
        .iter()
        .find(|i| i.identificador == "+5511900000001")
        .unwrap();
    assert_eq!((um.app.as_str(), um.conversas, um.mensagens), ("sms", 1, 2));
    assert_eq!(um.primeira_ms, Some(1_767_225_600_000));
    assert_eq!(um.ultima_ms, Some(1_767_225_660_000));
    // Filtro por trecho, sem normalizar o número.
    let filtrado = banco.identificadores(Some("0002"), 0, 10).unwrap();
    assert_eq!(filtrado.total, 1);
    assert_eq!(filtrado.itens[0].identificador, "+5511900000002");
    assert_eq!(
        banco.identificadores(Some("99999"), 0, 10).unwrap().total,
        0
    );
}
