//! Agenda e registro de chamadas do Android de ponta a ponta, sobre extrações
//! **SINTÉTICAS** montadas no teste.
//!
//! ATENÇÃO: os bancos vêm de `testes/fixtures/android/*.sql` e
//! `testes/fixtures/whatsapp/wa.sql` (inventados, com nomes de tabela da documentação
//! pública do Android e do app). As pastas abaixo não representam nenhuma ferramenta de
//! extração. Nenhum dado é real.

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use sied_armazenamento::agenda::{FiltroChamadas, FiltroContatos};
use sied_armazenamento::busca::FiltroBusca;
use sied_armazenamento::evidencias::NovaEvidencia;
use sied_armazenamento::extracoes::Extracao;
use sied_armazenamento::{BancoCaso, Sessao};
use sied_dominio::{Dispositivo, NovoCaso, NovoDispositivo, SistemaOperacional};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::importador::Importador;
use sied_ingestao::importadores::generico::ImportadorGenerico;
use sied_ingestao::pipeline::{self, Controle};
use sied_ingestao::Registro;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CONTACTS2: &str = include_str!("../../../testes/fixtures/android/contacts2.sql");
const CALLLOG: &str = include_str!("../../../testes/fixtures/android/calllog.sql");
const WA: &str = include_str!("../../../testes/fixtures/whatsapp/wa.sql");

const CONTATOS: &str = "dados/data/com.android.providers.contacts/databases";
const TELEFONIA: &str = "dados/data/com.android.providers.telephony/databases";
const WHATSAPP: &str = "dados/data/com.whatsapp/databases";

fn sessao() -> Sessao {
    Sessao {
        usuario: "analista.teste".into(),
        maquina: "ESTACAO-TESTE".into(),
    }
}

fn caso(raiz: &Path) -> (BancoCaso, Dispositivo, Dispositivo) {
    let mut b = BancoCaso::criar(
        &raiz.join("Casos/IP-3"),
        NovoCaso {
            numero_procedimento: Some("IP 3/2026".into()),
            responsavel_analise: "Analista".into(),
            unidade: "DP".into(),
            ..Default::default()
        },
        sessao(),
    )
    .unwrap();
    let mut novo = |apelido: &str| {
        b.cadastrar_dispositivo(NovoDispositivo {
            apelido: apelido.into(),
            sistema: SistemaOperacional::Android,
            ..Default::default()
        })
        .unwrap()
    };
    let (d1, d2) = (novo("Celular sintético A"), novo("Celular sintético B"));
    (b, d1, d2)
}

fn banco_de(sql: &str, destino: &Path) {
    fs::create_dir_all(destino.parent().unwrap()).unwrap();
    Connection::open(destino)
        .unwrap()
        .execute_batch(sql)
        .unwrap();
}

/// Agenda, chamadas, um SMS do número da Ana e os contatos do WhatsApp.
fn extracao_completa(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SINTETICO-LEIAME.txt"),
        "Extração SINTÉTICA gerada por teste automatizado. Nenhum dado é real.",
    )
    .unwrap();
    banco_de(CONTACTS2, &dir.join(CONTATOS).join("contacts2.db"));
    banco_de(CALLLOG, &dir.join(CONTATOS).join("calllog.db"));
    banco_de(WA, &dir.join(WHATSAPP).join("wa.db"));
    banco_de(
        "CREATE TABLE sms (_id INTEGER PRIMARY KEY, thread_id INTEGER, address TEXT, date INTEGER,
             type INTEGER, body TEXT);
         INSERT INTO sms VALUES (1, 1, '+5511900000011', 1767290000000, 1, 'SMS sintético');",
        &dir.join(TELEFONIA).join("mmssms.db"),
    );
}

fn sha256(p: &Path) -> String {
    hex::encode(Sha256::digest(fs::read(p).unwrap()))
}

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

fn importar(banco: &BancoCaso, d: &Dispositivo, origem: &Path) -> Extracao {
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
        &Controle::default(),
        &ctx,
    );
    imp.finalizar(&r).unwrap()
}

fn texto(t: &str) -> Option<String> {
    Some(t.into())
}

#[test]
fn agenda_e_chamadas_importadas_e_ligadas_pelo_mesmo_numero() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA-A");
    extracao_completa(&origem);
    let antes = retrato(&origem);
    let (mut banco, d1, d2) = caso(raiz.path());

    let x = importar(&banco, &d1, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    // 6 contatos da agenda + 9 chamadas + 1 SMS + 4 contatos do wa.db.
    assert_eq!(x.total_artefatos, Some(20));
    // A linha órfã de `data` na agenda.
    assert_eq!(x.total_avisos, Some(1));
    assert_eq!(retrato(&origem), antes, "a extração não pode mudar");

    let execucoes: BTreeMap<String, String> = banco
        .execucoes(x.id)
        .unwrap()
        .into_iter()
        .map(|e| (e.parser, e.status))
        .collect();
    assert_eq!(execucoes["android_contatos"], "parcial");
    assert_eq!(execucoes["android_chamadas"], "ok");
    // Com calllog.db na pasta, o contacts2.db não vai para o leitor de chamadas.
    assert_eq!(execucoes.len(), 4);

    // Todo registro importado confere com o próprio hash (inclusive as origens
    // adicionais da agenda: conta, contato agrupado e linhas de `data`).
    assert!(banco
        .registros_que_nao_conferem(Some(x.id))
        .unwrap()
        .is_empty());

    // Segunda extração, de outro aparelho, só com a mesma agenda: nada dela pode ser
    // ligado às chamadas da primeira.
    let origem_b = raiz.path().join("Extracao-SINTETICA-B");
    banco_de(CONTACTS2, &origem_b.join(CONTATOS).join("contacts2.db"));
    let xb = importar(&banco, &d2, &origem_b);
    assert_eq!(xb.status, "concluida");
    let execucoes_b: BTreeMap<String, String> = banco
        .execucoes(xb.id)
        .unwrap()
        .into_iter()
        .map(|e| (e.parser, e.status))
        .collect();
    // Sem calllog.db: as chamadas são procuradas no contacts2.db, que não tem a tabela.
    assert_eq!(execucoes_b["android_chamadas"], "parcial");

    // ---------------------------------------------------------------- contatos
    let todos = banco.contatos(&FiltroContatos::default(), 0, 100).unwrap();
    // 6 + 6 da agenda (duas extrações) + 3 do WhatsApp (o grupo fica de fora).
    assert_eq!(todos.total, 15);
    assert!(todos
        .itens
        .iter()
        .all(|c| c.nome.as_deref() != Some("Grupo sintético de teste")));
    // Sem nome por último.
    assert_eq!(todos.itens.last().unwrap().nome, None);

    let so_a = |f: FiltroContatos| {
        banco
            .contatos(
                &FiltroContatos {
                    extracao_id: Some(x.id),
                    ..f
                },
                0,
                100,
            )
            .unwrap()
    };
    let dora = so_a(FiltroContatos {
        texto: texto("DÓRA"),
        ..Default::default()
    });
    assert_eq!(dora.total, 1);
    let dora = &dora.itens[0];
    assert_eq!(dora.app.as_deref(), Some("contatos_android"));
    assert_eq!(dora.empresa.as_deref(), Some("Empresa Sintética Ltda"));

    let bruno = so_a(FiltroContatos {
        texto: texto("90000-0012"),
        ..Default::default()
    });
    let apps: Vec<_> = bruno.itens.iter().map(|c| c.app.clone().unwrap()).collect();
    assert_eq!(apps, vec!["contatos_android", "whatsapp"]);
    let tel = &bruno.itens[0].telefones;
    assert_eq!(tel.len(), 2);
    assert_eq!(tel[0].numero, "11 90000-0012");
    assert_eq!(tel[0].tipo.as_deref(), Some("trabalho"));
    assert_eq!(tel[1].rotulo.as_deref(), Some("Escritório sintético"));
    // Contato do WhatsApp: o número do wa.db.
    assert_eq!(bruno.itens[1].telefones[0].numero, "+55 11 90000-0012");

    let ana = so_a(FiltroContatos {
        texto: texto("ana"),
        app: texto("contatos_android"),
        ..Default::default()
    });
    assert_eq!(ana.total, 2);
    assert!(ana.itens[0].favorito);
    assert_eq!(
        ana.itens[0].conta.as_deref(),
        Some("analista.sintetico@example.com")
    );
    assert_eq!(ana.itens[1].tipo_conta.as_deref(), Some("com.whatsapp"));
    let ids_ana: Vec<i64> = ana.itens.iter().map(|c| c.id).collect();

    let apps = banco.apps_contatos().unwrap();
    let apps: Vec<(String, i64)> = apps.into_iter().map(|a| (a.app, a.total)).collect();
    assert_eq!(
        apps,
        vec![("contatos_android".into(), 12), ("whatsapp".into(), 3)]
    );

    // ---------------------------------------------------------------- chamadas
    let todas = banco.chamadas(&FiltroChamadas::default(), 0, 100).unwrap();
    assert_eq!(todas.total, 9);
    let ids: Vec<Option<i64>> = todas.itens.iter().map(|c| c.data_hora_ms).collect();
    let mut ordenado = ids.clone();
    ordenado.sort_by(|a, b| b.cmp(a));
    assert_eq!(ids, ordenado, "mais recente primeiro");

    // A mais antiga com o número (a Ana aparece em duas chamadas).
    let por_numero = |n: &str| {
        todas
            .itens
            .iter()
            .rev()
            .find(|c| c.numero.as_deref() == Some(n))
            .unwrap()
    };
    let c1 = por_numero("+5511900000011");
    assert_eq!(c1.subtipo.as_deref(), Some("recebida"));
    assert_eq!(c1.duracao_s, Some(125));
    assert_eq!(c1.nome_no_registro.as_deref(), Some("Ana Sintética"));
    // As duas linhas da Ana na agenda da MESMA extração (Google e WhatsApp); nenhuma
    // da extração B.
    let agenda: Vec<i64> = c1.agenda.iter().map(|c| c.id).collect();
    assert_eq!(agenda, ids_ana);

    // Número gravado com máscara: liga pelo normalizado do aparelho.
    let c2 = por_numero("(11) 90000-0012");
    assert_eq!(c2.subtipo.as_deref(), Some("efetuada"));
    assert_eq!(c2.de_mim, Some(true));
    assert_eq!(c2.recursos, vec!["video", "wifi"]);
    assert_eq!(c2.agenda.len(), 1);
    assert_eq!(c2.agenda[0].nome.as_deref(), Some("Bruno Sintético"));

    // Fora da agenda.
    assert!(por_numero("+5521900000013").agenda.is_empty());
    // Número gravado diferente, normalizado idêntico ao do contato apagado.
    let c9 = por_numero("071900000017");
    assert_eq!(c9.agenda.len(), 1);
    assert_eq!(c9.agenda[0].nome.as_deref(), Some("Eduardo Sintético"));
    assert_eq!(c9.agenda[0].estado_registro, "apagado");
    // Restrito: sem número, sem vínculo.
    let restrita = todas
        .itens
        .iter()
        .find(|c| c.apresentacao.as_deref() == Some("restrito"))
        .unwrap();
    assert_eq!(restrita.numero, None);
    assert!(restrita.agenda.is_empty());
    // Código antigo no lugar do número: não vira telefone, mas chega à tela.
    let especial = todas
        .itens
        .iter()
        .find(|c| c.numero_especial.is_some())
        .unwrap();
    assert_eq!(especial.numero_especial.as_deref(), Some("-2"));
    assert_eq!(especial.numero, None);
    assert!(especial.agenda.is_empty());

    let filtrar = |f: FiltroChamadas| banco.chamadas(&f, 0, 100).unwrap();
    let fone = filtrar(FiltroChamadas {
        texto: texto("(11) 90000-0011"),
        ..Default::default()
    });
    assert_eq!(fone.total, 2);
    // Nome só pela agenda (a chamada não guardou nome).
    let eduardo = filtrar(FiltroChamadas {
        texto: texto("eduardo"),
        ..Default::default()
    });
    assert_eq!(eduardo.total, 1);
    assert_eq!(eduardo.itens[0].numero.as_deref(), Some("071900000017"));
    // Nome guardado na chamada, sem acento.
    assert_eq!(
        filtrar(FiltroChamadas {
            texto: texto("SINTETICO"),
            ..Default::default()
        })
        .total,
        // Bruno (nome na chamada) e Eduardo (pela agenda).
        2
    );
    let perdidas = filtrar(FiltroChamadas {
        subtipo: texto("perdida"),
        ..Default::default()
    });
    assert_eq!(perdidas.total, 2);
    assert_eq!(
        filtrar(FiltroChamadas {
            extracao_id: Some(xb.id),
            ..Default::default()
        })
        .total,
        0
    );

    let resumo = banco.resumo_chamadas().unwrap();
    assert_eq!(resumo.total, 9);
    assert_eq!(resumo.duracao_total_s, 212);
    assert_eq!(resumo.numeros_distintos, 6);
    let por_subtipo: BTreeMap<String, i64> = resumo
        .por_subtipo
        .into_iter()
        .map(|s| (s.subtipo, s.total))
        .collect();
    assert_eq!(por_subtipo["recebida"], 3);
    assert_eq!(por_subtipo["perdida"], 2);
    assert_eq!(por_subtipo["desconhecido"], 1);

    // ---------------------------------------------------------------- identificadores
    let idents = banco.identificadores(None, 0, 100).unwrap();
    let sms = idents
        .itens
        .iter()
        .find(|i| i.identificador == "+5511900000011")
        .unwrap();
    assert_eq!(sms.app, "sms");
    let agenda: Vec<i64> = sms.agenda.iter().map(|c| c.id).collect();
    assert_eq!(agenda, ids_ana);

    // ---------------------------------------------------------------- busca
    let achados = banco
        .buscar(
            &FiltroBusca {
                texto: "90000-0017".into(),
                ..Default::default()
            },
            0,
            50,
        )
        .unwrap();
    let tipos: Vec<&str> = achados
        .artefatos
        .itens
        .iter()
        .map(|a| a.artefato.tipo.as_str())
        .collect();
    // A chamada e o contato apagado, nas duas extrações.
    assert_eq!(tipos.iter().filter(|t| **t == "chamada").count(), 1);
    assert_eq!(tipos.iter().filter(|t| **t == "contato").count(), 2);

    // ---------------------------------------------------------------- evidências
    let e = banco
        .adicionar_evidencia(NovaEvidencia {
            artefato_id: Some(c1.id),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(e.numero, 1);
    let e = banco
        .adicionar_evidencia(NovaEvidencia {
            artefato_id: Some(dora.id),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(e.numero, 2);
}
