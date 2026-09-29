//! Medição de desempenho com volume (não roda na bateria normal).
//!
//! Extração **SINTÉTICA** montada no teste: SMS inventados em quantidade, numa conversa
//! enorme e em muitas conversas curtas. Mede importação, listas, filtros, busca e
//! trecho, e imprime os tempos. Rodar em release:
//!
//! ```text
//! SIED_VOLUME=200000 cargo test --release -p sied-armazenamento --test desempenho \
//!     -- --ignored --nocapture
//! ```

use rusqlite::Connection;
use sied_armazenamento::busca::FiltroBusca;
use sied_armazenamento::conversas::FiltroConversas;
use sied_armazenamento::evidencias::NovaEvidencia;
use sied_armazenamento::{BancoCaso, Sessao};
use sied_dominio::{NovoCaso, NovoDispositivo, SistemaOperacional};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::importador::Importador;
use sied_ingestao::importadores::generico::ImportadorGenerico;
use sied_ingestao::pipeline::{self, Controle};
use sied_ingestao::Registro;
use std::fs;
use std::path::Path;
use std::time::Instant;

const BASE_MS: i64 = 1_767_225_600_000;

fn extracao(dir: &Path, total: i64) {
    fs::create_dir_all(dir.join("telefonia")).unwrap();
    fs::write(dir.join("SINTETICO.txt"), "Extração SINTÉTICA de teste.").unwrap();
    let mut conn = Connection::open(dir.join("telefonia/mmssms.db")).unwrap();
    conn.execute_batch(
        "CREATE TABLE sms(_id INTEGER PRIMARY KEY, thread_id, address, date, type, body);",
    )
    .unwrap();
    let tx = conn.transaction().unwrap();
    {
        let mut ins = tx
            .prepare("INSERT INTO sms VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
            .unwrap();
        // Metade numa conversa só; o resto em conversas de 20 mensagens.
        for i in 1..=total {
            let (thread, numero) = if i <= total / 2 {
                (1, "+5511900000001".to_string())
            } else {
                let t = 2 + (i - total / 2) / 20;
                (t, format!("+55119{:08}", t))
            };
            let corpo = if i % 1000 == 0 {
                format!(
                    "Ligação combinada às {} horas, marcador sintético {i}",
                    i % 24
                )
            } else {
                format!("Mensagem sintética número {i} sobre o encontro de amanhã")
            };
            ins.execute(rusqlite::params![
                i,
                thread,
                numero,
                BASE_MS + i * 30_000,
                1 + i % 2,
                corpo
            ])
            .unwrap();
        }
    }
    tx.commit().unwrap();
}

fn medir<T>(nome: &str, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let r = f();
    println!("{nome:<58} {:>9.1} ms", t.elapsed().as_secs_f64() * 1000.0);
    r
}

#[test]
#[ignore = "medição: rodar em release com --ignored --nocapture"]
fn medir_desempenho_com_volume() {
    let total: i64 = std::env::var("SIED_VOLUME")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000);
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    medir("gerar extração sintética", || extracao(&origem, total));

    let mut banco = BancoCaso::criar(
        &raiz.path().join("Casos/IP-desempenho"),
        NovoCaso {
            numero_procedimento: Some("IP desempenho".into()),
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
    let d = banco
        .cadastrar_dispositivo(NovoDispositivo {
            apelido: "Celular sintético".into(),
            sistema: SistemaOperacional::Android,
            ..Default::default()
        })
        .unwrap();

    println!("--- {total} mensagens SMS sintéticas ---");
    let x = medir(
        "importar (hash, cópia verificada, parser, gravação)",
        || {
            let importador = ImportadorGenerico;
            let mut imp = banco
                .iniciar_importacao(d.id, &importador.info(), &origem)
                .unwrap();
            let fonte = PastaSomenteLeitura::abrir(&origem).unwrap();
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
        },
    );
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    assert_eq!(x.total_artefatos, Some(total));
    let tamanho = fs::metadata(banco.pasta().join("caso.sied.db"))
        .unwrap()
        .len();
    println!(
        "{:<58} {:>9.1} MB",
        "tamanho do banco do caso",
        tamanho as f64 / 1e6
    );

    let conversas = medir("listar conversas (1ª página de 100)", || {
        banco.conversas(None, 0, 100).unwrap()
    });
    println!("{:<58} {:>9}", "conversas", conversas.total);
    let grande = banco
        .conversas_filtradas(
            &FiltroConversas {
                texto: Some("+5511900000001".into()),
                ..Default::default()
            },
            0,
            10,
        )
        .unwrap()
        .itens
        .into_iter()
        .max_by_key(|c| c.total_mensagens)
        .unwrap();
    println!(
        "{:<58} {:>9}",
        "mensagens na conversa grande", grande.total_mensagens
    );
    medir("filtrar conversas por texto (sem acento)", || {
        banco
            .conversas_filtradas(
                &FiltroConversas {
                    texto: Some("sintetica".into()),
                    ..Default::default()
                },
                0,
                100,
            )
            .unwrap()
    });
    medir("filtrar conversas por telefone", || {
        banco
            .conversas_filtradas(
                &FiltroConversas {
                    texto: Some("11 90000-0001".into()),
                    ..Default::default()
                },
                0,
                100,
            )
            .unwrap()
    });
    medir("mensagens da conversa grande (início)", || {
        banco.mensagens_conversa(grande.id, 0, 200).unwrap()
    });
    let fundo = medir(
        "mensagens da conversa grande (fim, deslocamento alto)",
        || {
            banco
                .mensagens_conversa(grande.id, grande.total_mensagens - 200, 200)
                .unwrap()
        },
    );
    let pequena = conversas.itens[0].clone();
    medir("mensagens de uma conversa curta (20 mensagens)", || {
        banco.mensagens_conversa(pequena.id, 0, 200).unwrap()
    });
    let ultima = fundo.itens.last().unwrap().id;
    medir("posição de uma mensagem na conversa grande", || {
        banco.posicao_mensagem(grande.id, ultima).unwrap()
    });
    let achadas = medir("localizar na conversa grande (sem acento)", || {
        banco
            .localizar_na_conversa(grande.id, "LIGACAO COMBINADA", 50)
            .unwrap()
    });
    println!("{:<58} {:>9}", "ocorrências", achadas.total);
    let r = medir("busca global por palavra (FTS)", || {
        banco
            .buscar(
                &FiltroBusca {
                    texto: "ligação".into(),
                    ..Default::default()
                },
                0,
                50,
            )
            .unwrap()
    });
    println!("{:<58} {:>9}", "achados", r.artefatos.total);
    medir("busca global por telefone", || {
        banco
            .buscar(
                &FiltroBusca {
                    texto: "(11) 90000-0001".into(),
                    ..Default::default()
                },
                0,
                50,
            )
            .unwrap()
    });
    medir("identificadores (1ª página)", || {
        banco.identificadores(None, 0, 100).unwrap()
    });
    medir("resumo do caso", || banco.resumo().unwrap());

    let inicio = banco.mensagens_conversa(grande.id, 0, 1).unwrap().itens[0].id;
    let fim = banco.mensagens_conversa(grande.id, 1999, 1).unwrap().itens[0].id;
    medir("marcar trecho de 2.000 mensagens (limite)", || {
        banco
            .adicionar_evidencia(NovaEvidencia {
                artefato_id: Some(inicio),
                trecho_fim_id: Some(fim),
                ..Default::default()
            })
            .unwrap()
    });
    medir("conferir hash de todos os registros do caso", || {
        assert!(banco.registros_que_nao_conferem(None).unwrap().is_empty())
    });
    medir("verificar a trilha de auditoria", || {
        banco.verificar_auditoria().unwrap()
    });
    if std::env::var_os("SIED_MANTER").is_some() {
        let pasta = banco.pasta().to_path_buf();
        drop(banco);
        #[allow(deprecated)]
        let _ = raiz.into_path();
        println!("caso mantido em {}", pasta.display());
    }
}
