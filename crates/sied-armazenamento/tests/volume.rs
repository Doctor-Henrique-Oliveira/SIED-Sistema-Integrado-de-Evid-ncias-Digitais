//! Consultas para volume real: filtros de conversa no banco (caso inteiro), posição
//! de uma mensagem na conversa, localizar dentro da conversa e o arquivo com a pasta
//! de origem (miniaturas). Extração **SINTÉTICA** montada no teste: SMS inventados em
//! quantidade e os bancos do WhatsApp de `testes/fixtures/whatsapp`.

use rusqlite::Connection;
use sied_armazenamento::conversas::FiltroConversas;
use sied_armazenamento::{BancoCaso, Sessao};
use sied_dominio::{Dispositivo, NovoCaso, NovoDispositivo, SistemaOperacional};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::importador::Importador;
use sied_ingestao::importadores::generico::ImportadorGenerico;
use sied_ingestao::pipeline::{self, Controle, Etapa};
use sied_ingestao::Registro;
use std::fs;
use std::path::Path;

const NOVO: &str = include_str!("../../../testes/fixtures/whatsapp/msgstore_novo.sql");
const WA: &str = include_str!("../../../testes/fixtures/whatsapp/wa.sql");
const BANCOS: &str = "dados/com.whatsapp/databases";

/// Mensagens da conversa grande (thread 1), 3 delas sem data (uma é marcador).
const GRANDE: i64 = 1200;
const BASE_MS: i64 = 1_767_225_600_000;

fn caso(raiz: &Path) -> (BancoCaso, Dispositivo) {
    let mut b = BancoCaso::criar(
        &raiz.join("Casos/IP-3"),
        NovoCaso {
            numero_procedimento: Some("IP 3/2026".into()),
            responsavel_analise: "Analista".into(),
            unidade: "DP".into(),
            ..Default::default()
        },
        Sessao {
            usuario: "analista.teste".into(),
            maquina: "ESTACAO-TESTE".into(),
        },
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

fn extracao(dir: &Path) {
    fs::create_dir_all(dir.join("telefonia")).unwrap();
    fs::write(dir.join("SINTETICO.txt"), "Extração SINTÉTICA de teste.").unwrap();
    let conn = Connection::open(dir.join("telefonia/mmssms.db")).unwrap();
    conn.execute_batch(
        "CREATE TABLE sms(_id INTEGER PRIMARY KEY, thread_id, address, date, type, body);",
    )
    .unwrap();
    let mut ins = conn
        .prepare("INSERT INTO sms(_id, thread_id, address, date, type, body) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
        .unwrap();
    for i in 1..=GRANDE {
        // Datas fora de ordem de id, para a posição depender da data, e de três em
        // três iguais, para o desempate pelo id contar.
        let data = if i == 7 || i == 8 || i == 300 {
            None
        } else {
            Some(BASE_MS + ((i * 37) % GRANDE) / 3 * 60_000)
        };
        let corpo = if i % 100 == 0 {
            format!("marcador sintético {i}")
        } else {
            format!("mensagem sintética {i}")
        };
        ins.execute(rusqlite::params![
            i,
            1,
            "+5511900000001",
            data,
            1 + i % 2,
            corpo
        ])
        .unwrap();
    }
    for (id, thread, numero) in [
        (5001, 2, "+5521900000002"),
        (5002, 3, "+5531900000003"),
        (5003, 4, "(61) 90000-0004"),
    ] {
        ins.execute(rusqlite::params![
            id,
            thread,
            numero,
            BASE_MS,
            1,
            "oi (sintético)"
        ])
        .unwrap();
    }
    drop(ins);
    drop(conn);
    fs::create_dir_all(dir.join(BANCOS)).unwrap();
    Connection::open(dir.join(BANCOS).join("msgstore.db"))
        .unwrap()
        .execute_batch(NOVO)
        .unwrap();
    Connection::open(dir.join(BANCOS).join("wa.db"))
        .unwrap()
        .execute_batch(WA)
        .unwrap();
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

fn preparado() -> (
    tempfile::TempDir,
    BancoCaso,
    sied_armazenamento::extracoes::Extracao,
) {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    extracao(&origem);
    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem, &Controle::default());
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    (raiz, banco, x)
}

fn titulos(banco: &BancoCaso, f: FiltroConversas) -> Vec<String> {
    let p = banco.conversas_filtradas(&f, 0, 100).unwrap();
    assert_eq!(p.total as usize, p.itens.len());
    let mut v: Vec<String> = p
        .itens
        .into_iter()
        .map(|c| c.titulo.unwrap_or(c.id_externo))
        .collect();
    v.sort();
    v
}

fn texto(t: &str) -> FiltroConversas {
    FiltroConversas {
        texto: Some(t.into()),
        ..Default::default()
    }
}

#[test]
fn filtros_de_conversa_valem_para_o_caso_inteiro() {
    let (_raiz, banco, x) = preparado();
    // 4 do SMS + 3 do WhatsApp.
    assert_eq!(titulos(&banco, FiltroConversas::default()).len(), 7);

    let apps = banco.apps_das_conversas(None).unwrap();
    let apps: Vec<(String, i64)> = apps.into_iter().map(|a| (a.app, a.conversas)).collect();
    assert_eq!(apps, vec![("sms".into(), 4), ("whatsapp".into(), 3)]);
    assert_eq!(banco.apps_das_conversas(Some(x.id)).unwrap().len(), 2);

    // Nome do contato no app (participante do grupo e conversa individual).
    assert_eq!(
        titulos(&banco, texto("bruno")),
        vec!["Bruno Sintético", "Grupo sintético de teste"]
    );
    // Título do grupo, sem diferenciar maiúsculas.
    assert_eq!(
        titulos(&banco, texto("GRUPO SINT")),
        vec!["Grupo sintético de teste"]
    );
    // Telefone digitado com máscara acha pelos dígitos; o número gravado com máscara
    // também é achado pelos dígitos.
    assert_eq!(titulos(&banco, texto("(21) 90000-0002")).len(), 1);
    assert_eq!(titulos(&banco, texto("61900000004")).len(), 1);
    // Trecho do identificador.
    assert_eq!(
        titulos(&banco, texto("@g.us")),
        vec!["Grupo sintético de teste"]
    );
    assert!(titulos(&banco, texto("ninguém com esse nome")).is_empty());

    let so = |f: FiltroConversas| banco.conversas_filtradas(&f, 0, 100).unwrap().total;
    assert_eq!(
        so(FiltroConversas {
            app: Some("sms".into()),
            ..Default::default()
        }),
        4
    );
    assert_eq!(
        titulos(
            &banco,
            FiltroConversas {
                tipo: Some("grupo".into()),
                ..Default::default()
            }
        ),
        vec!["Grupo sintético de teste"]
    );
    assert_eq!(
        so(FiltroConversas {
            tipo: Some("individual".into()),
            ..Default::default()
        }),
        6
    );
    // Só a conversa da Ana tem anexos na fixture; o grupo também tem documento.
    let com_anexo = so(FiltroConversas {
        com_anexo: Some(true),
        ..Default::default()
    });
    assert!((1..=3).contains(&com_anexo));
    assert_eq!(
        so(FiltroConversas {
            app: Some("whatsapp".into()),
            texto: Some("bruno".into()),
            tipo: Some("individual".into()),
            extracao_id: Some(x.id),
            ..Default::default()
        }),
        1
    );
    // Paginação: total igual, páginas sem repetição.
    let a = banco
        .conversas_filtradas(&FiltroConversas::default(), 0, 4)
        .unwrap();
    let b = banco
        .conversas_filtradas(&FiltroConversas::default(), 4, 4)
        .unwrap();
    assert_eq!((a.total, a.itens.len(), b.itens.len()), (7, 4, 3));
    assert!(a.itens.iter().all(|c| b.itens.iter().all(|d| d.id != c.id)));
}

#[test]
fn posicao_e_localizar_seguem_a_ordem_da_conversa() {
    let (_raiz, banco, _x) = preparado();
    let grande = banco
        .conversas_filtradas(&texto("+5511900000001"), 0, 10)
        .unwrap()
        .itens
        .remove(0);
    assert_eq!(grande.total_mensagens, GRANDE);

    // A posição de cada mensagem é o índice dela na ordem das páginas.
    let todas = banco
        .mensagens_conversa(grande.id, 0, GRANDE)
        .unwrap()
        .itens;
    assert_eq!(todas.len() as i64, GRANDE);
    for (i, m) in todas
        .iter()
        .enumerate()
        .step_by(97)
        .chain(todas.iter().enumerate().rev().take(3))
    {
        assert_eq!(
            banco.posicao_mensagem(grande.id, m.id).unwrap(),
            Some(i as i64),
            "registro {}",
            m.id
        );
    }
    // Sem data vão para o fim.
    assert!(todas[todas.len() - 3..]
        .iter()
        .all(|m| m.data_hora_ms.is_none()));
    // Registro de outra conversa não tem posição nesta.
    let outra = banco
        .conversas_filtradas(&texto("21900000002"), 0, 1)
        .unwrap()
        .itens[0]
        .id;
    let de_outra = banco.mensagens_conversa(outra, 0, 1).unwrap().itens[0].id;
    assert_eq!(banco.posicao_mensagem(grande.id, de_outra).unwrap(), None);

    // Localizar: 12 marcadores (a cada 100), em ordem, com a posição certa.
    let achados = banco
        .localizar_na_conversa(grande.id, "MARCADOR", 5)
        .unwrap();
    assert_eq!(achados.total, GRANDE / 100);
    assert_eq!(achados.itens.len(), 5);
    let achados = banco
        .localizar_na_conversa(grande.id, "marcador", 100)
        .unwrap();
    assert!(achados
        .itens
        .windows(2)
        .all(|w| w[0].posicao < w[1].posicao));
    for a in &achados.itens {
        assert_eq!(todas[a.posicao as usize].id, a.id);
        assert!(todas[a.posicao as usize]
            .conteudo
            .as_deref()
            .unwrap()
            .contains("marcador"));
    }
    assert_eq!(
        banco
            .localizar_na_conversa(grande.id, "  ", 10)
            .unwrap()
            .total,
        0
    );
}

#[test]
fn paginas_posicoes_e_trecho_batem_com_a_ordem_calculada_a_parte() {
    let (_raiz, banco, _x) = preparado();
    let grande = banco
        .conversas_filtradas(&texto("+5511900000001"), 0, 10)
        .unwrap()
        .itens
        .remove(0);
    let todas = banco
        .mensagens_conversa(grande.id, 0, GRANDE)
        .unwrap()
        .itens;

    // A ordem esperada, calculada aqui: datadas por (data, id), depois sem data por id.
    let mut esperada = todas.clone();
    esperada.sort_by_key(|m| (m.data_hora_ms.is_none(), m.data_hora_ms, m.id));
    let ids = |v: &[sied_armazenamento::extracoes::ArtefatoResumo]| {
        v.iter().map(|m| m.id).collect::<Vec<_>>()
    };
    assert_eq!(ids(&todas), ids(&esperada));
    assert!(todas
        .windows(2)
        .any(|w| w[0].data_hora_ms.is_some() && w[0].data_hora_ms == w[1].data_hora_ms));

    // Páginas de 7, cruzando a divisa entre datadas e sem data, e além do fim.
    let mut paginado = Vec::new();
    let mut desloc = 0;
    loop {
        let p = banco.mensagens_conversa(grande.id, desloc, 7).unwrap();
        assert_eq!(p.total, GRANDE);
        if p.itens.is_empty() {
            break;
        }
        desloc += p.itens.len() as i64;
        paginado.extend(p.itens);
    }
    assert_eq!(ids(&paginado), ids(&esperada));
    let fim = banco.mensagens_conversa(grande.id, GRANDE - 2, 50).unwrap();
    assert_eq!(ids(&fim.itens), ids(&esperada[esperada.len() - 2..]));

    // Posição de toda mensagem, inclusive empatadas e sem data.
    for (i, m) in esperada.iter().enumerate() {
        assert_eq!(
            banco.posicao_mensagem(grande.id, m.id).unwrap(),
            Some(i as i64)
        );
    }

    // O marcador sem data é achado na posição dele, no fim.
    let achados = banco
        .localizar_na_conversa(grande.id, "marcador sintético 300", 10)
        .unwrap();
    assert_eq!(achados.total, 1);
    assert!(achados.itens[0].posicao >= GRANDE - 3);
    assert_eq!(
        esperada[achados.itens[0].posicao as usize].id,
        achados.itens[0].id
    );

    // Trecho cruzando a divisa (informado de trás para frente): as mesmas mensagens,
    // na mesma ordem, com o hash de cada uma.
    let (a, b) = (GRANDE as usize - 10, GRANDE as usize - 2);
    let t = banco.trecho(esperada[b].id, esperada[a].id).unwrap();
    assert_eq!(t.conversa_id, Some(grande.id));
    let do_trecho: Vec<i64> = t.mensagens.iter().map(|(m, _)| m.id).collect();
    assert_eq!(do_trecho, ids(&esperada[a..=b]));
    assert!(t.mensagens.iter().all(|(_, h)| h.len() == 64));
}

/// Mensagem cujo próprio texto traz os caracteres usados como marcadores do destaque:
/// a busca devolve o texto original e destaca só o termo.
#[test]
fn destaque_da_busca_nao_se_confunde_com_o_texto_da_mensagem() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    fs::create_dir_all(origem.join("telefonia")).unwrap();
    fs::write(origem.join("SINTETICO.txt"), "Extração SINTÉTICA de teste.").unwrap();
    let conn = Connection::open(origem.join("telefonia/mmssms.db")).unwrap();
    conn.execute_batch(
        "CREATE TABLE sms(_id INTEGER PRIMARY KEY, thread_id, address, date, type, body);",
    )
    .unwrap();
    let corpo = "inicio \u{2}falso\u{1} combinado \u{1}aberto amanha";
    conn.execute(
        "INSERT INTO sms VALUES (1, 1, '+5511900000001', ?1, 1, ?2)",
        rusqlite::params![BASE_MS, corpo],
    )
    .unwrap();
    drop(conn);
    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem, &Controle::default());
    assert_eq!(x.status, "concluida", "{:?}", x.erro);

    let r = banco
        .buscar(
            &sied_armazenamento::busca::FiltroBusca {
                texto: "combinado".into(),
                ..Default::default()
            },
            0,
            10,
        )
        .unwrap();
    assert_eq!(r.artefatos.total, 1);
    let trecho = r.artefatos.itens[0].trecho.clone().unwrap();
    let junto: String = trecho.iter().map(|p| p.texto.as_str()).collect();
    assert_eq!(junto, corpo);
    let destacados: Vec<&str> = trecho
        .iter()
        .filter(|p| p.destaque)
        .map(|p| p.texto.as_str())
        .collect();
    assert_eq!(destacados, ["combinado"]);
}

#[test]
fn arquivo_com_a_pasta_de_origem_e_progresso_final() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    extracao(&origem);
    let (banco, d) = caso(raiz.path());
    let controle = Controle::default();
    let x = importar(&banco, &d, &origem, &controle);

    let p = controle.progresso();
    assert_eq!(p.etapa, Etapa::Concluida);
    let inicio = p.inicio_ms.expect("início registrado");
    assert!(p.etapa_inicio_ms.unwrap() >= inicio);
    assert_eq!(p.tarefas_feitas, p.tarefas_total);
    assert!(p.fase_tarefa.is_none() && p.copia_bytes_total.is_none());

    let db = banco
        .arquivos(x.id, None, Some("mmssms.db"), 0, 1)
        .unwrap()
        .itens
        .remove(0);
    let a = banco.arquivo_com_origem(db.id).unwrap();
    assert_eq!(a.arquivo.caminho, "telefonia/mmssms.db");
    assert_eq!(a.arquivo.sha256, db.sha256);
    assert_eq!(a.origem_caminho, x.origem_caminho);
    assert!(Path::new(&a.origem_caminho)
        .join(&a.arquivo.caminho)
        .is_file());
    assert!(banco.arquivo_com_origem(999_999).is_err());
}
