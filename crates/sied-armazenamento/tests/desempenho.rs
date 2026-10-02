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
//!
//! AI-0 isolada (sem Tauri/Typst), sempre em TempDir e sem `SIED_MANTER`:
//! ```text
//! SIED_VOLUME_AI0=50000 SIED_REPETICOES_AI0=3 cargo test --release \
//!     -p sied-armazenamento --test desempenho medir_ai0_com_volume \
//!     -- --ignored --exact --nocapture
//! ```
//! Os tempos são observações, não SLO/recall de dados reais. O cache do SO não
//! é controlado. Payload JSON não é uma medição de RSS nem de toda a memória.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use rusqlite::Connection;
use sied_armazenamento::busca::FiltroBusca;
use sied_armazenamento::conversas::FiltroConversas;
use sied_armazenamento::descoberta::{
    self, ModoDescoberta, PedidoDescoberta, ResultadoDescoberta, TipoIndicador,
};
use sied_armazenamento::evidencias::NovaEvidencia;
use sied_armazenamento::{BancoCaso, ErroArmazenamento, Resultado, Sessao};
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
    medir("conferir dados derivados (conversas e índices)", || {
        assert!(banco.conferir_derivados().unwrap().consistente)
    });
    medir("reconstruir dados derivados", || {
        assert!(banco.reconstruir_derivados().unwrap().depois.consistente)
    });
    if std::env::var_os("SIED_MANTER").is_some() {
        let pasta = banco.pasta().to_path_buf();
        drop(banco);
        #[allow(deprecated)]
        let _ = raiz.into_path();
        println!("caso mantido em {}", pasta.display());
    }
}

fn observar_ai0(
    nome: &str,
    total_esperado: i64,
    repeticoes: usize,
    mut consultar: impl FnMut() -> Resultado<ResultadoDescoberta>,
) {
    let mut tempos = Vec::new();
    let mut payload_maximo = 0;
    let mut limites = 0;
    for _ in 0..repeticoes {
        let inicio = Instant::now();
        let resultado = consultar();
        let tempo = inicio.elapsed();
        match resultado {
            Ok(resultado) => {
                assert_eq!(resultado.artefatos.total, total_esperado, "{nome}");
                assert_eq!(
                    resultado.artefatos.itens.len() as i64,
                    total_esperado.min(50)
                );
                assert_eq!(resultado.arquivos.total, 0);
                let bytes = serde_json::to_vec(&resultado).unwrap().len();
                payload_maximo = payload_maximo.max(bytes);
                tempos.push(tempo);
            }
            Err(ErroArmazenamento::DescobertaLimite) => {
                limites += 1;
                println!(
                    "{nome}: prazo cooperativo atingido em {:.1} ms; sem página parcial",
                    tempo.as_secs_f64() * 1000.0
                );
            }
            Err(erro) => panic!("{nome}: falha inesperada na medição: {erro}"),
        }
    }
    tempos.sort();
    if !tempos.is_empty() {
        let p50 = tempos[(tempos.len() - 1) / 2].as_secs_f64() * 1000.0;
        let p95 = tempos[(tempos.len() * 95).div_ceil(100) - 1].as_secs_f64() * 1000.0;
        println!("{nome:<38} concluídas={}/{} p50={p50:.1} ms p95={p95:.1} ms payload_max={payload_maximo} bytes", tempos.len(), repeticoes);
    }
    if limites > 0 {
        println!("MEDIÇÃO INCOMPLETA {nome}: {limites} consulta(s) atingiram o prazo; não extrapolar esses tempos como desempenho aprovado.");
    }
    assert_eq!(
        limites, 0,
        "{nome}: medição incompleta não aprova a validação"
    );
}

#[test]
#[ignore = "medição AI-0 sintética: release, --ignored --exact --nocapture"]
fn medir_ai0_com_volume() {
    let total: i64 = std::env::var("SIED_VOLUME_AI0")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50_000);
    let repeticoes: usize = std::env::var("SIED_REPETICOES_AI0")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);
    assert!(
        (1..=1_000_000).contains(&total),
        "volume AI-0 deve ficar entre 1 e 1.000.000"
    );
    assert!(
        (1..=20).contains(&repeticoes),
        "repetições AI-0 devem ficar entre 1 e 20"
    );
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA-AI0-VOLUME");
    medir(
        "gerar SMS SINTÉTICOS usando gerador de volume existente",
        || extracao(&origem, total),
    );
    // Sete sementes explícitas, construídas na fonte ANTES da importação. O
    // remetente neutro não adiciona indicadores de telefone nesses sete registros.
    let sementes = [
        "192.0.2.1 [2001:db8::1]",
        "teste@example.invalid",
        "https://example.invalid/amostra",
        "sub.example.invalid",
        "0123456789abcdef0123456789abcdef",
        "+5511900000045",
        "-23.55052, -46.633308",
    ];
    let fonte = Connection::open(origem.join("telefonia/mmssms.db")).unwrap();
    for (indice, semente) in sementes.iter().enumerate() {
        fonte
            .execute(
                "INSERT INTO sms VALUES (?1, 99999999, 'REMETENTE-SINTETICO', ?2, 1, ?3)",
                rusqlite::params![
                    total + indice as i64 + 1,
                    BASE_MS + (total + indice as i64 + 1) * 30_000,
                    format!("AI0-MARCADOR-SINTETICO-{indice}-FIM {semente}")
                ],
            )
            .unwrap();
    }
    drop(fonte);
    let origem_antes = comum::retrato(&origem);
    let (mut banco, dispositivo) = comum::caso(raiz.path());
    let importada = medir("importar volume AI-0 pelo pipeline normal", || {
        comum::importar(&banco, &dispositivo, &origem)
    });
    assert_eq!(importada.status, "concluida", "{:?}", importada.erro);
    assert_eq!(
        importada.total_artefatos,
        Some(total + sementes.len() as i64)
    );
    let conn = Connection::open_with_flags(
        banco.pasta().join("caso.sied.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let auditoria_antes = banco.verificar_auditoria().unwrap();
    println!("--- AI-0: {total} SMS + 7 sementes SINTÉTICAS; {repeticoes} repetições; cache do SO não controlado ---");
    let literal = PedidoDescoberta {
        filtro: FiltroBusca {
            texto: "Mensagem sintética".into(),
            app: Some("sms".into()),
            tipo: Some("mensagem".into()),
            ..Default::default()
        },
        modo: ModoDescoberta::Literal,
        termos: vec![],
        indicador: None,
    };
    observar_ai0(
        "literal / página inicial",
        total - total / 1000,
        repeticoes,
        || descoberta::buscar(&conn, &literal, 0, 50),
    );
    let expansao = PedidoDescoberta {
        filtro: FiltroBusca {
            texto: "ligação".into(),
            ..literal.filtro.clone()
        },
        modo: ModoDescoberta::Expansao,
        termos: vec!["marcador".into()],
        indicador: None,
    };
    let esperados_expansao = total / 1000 + sementes.len() as i64;
    observar_ai0(
        "expansão / união sem duplicação",
        esperados_expansao,
        repeticoes,
        || descoberta::buscar(&conn, &expansao, 0, 50),
    );
    for (tipo, esperado) in [
        (TipoIndicador::Ip, 1),
        (TipoIndicador::Email, 1),
        (TipoIndicador::Url, 1),
        (TipoIndicador::Dominio, 2),
        (TipoIndicador::Hash, 1),
        // O hash sintático também contém uma sequência numérica reconhecida
        // como candidata a telefone. Candidato não prova identidade/validade.
        (TipoIndicador::Telefone, total + 2),
        (TipoIndicador::Coordenada, 1),
    ] {
        let pedido = PedidoDescoberta {
            filtro: FiltroBusca {
                texto: String::new(),
                ..literal.filtro.clone()
            },
            modo: ModoDescoberta::Indicadores,
            termos: vec![],
            indicador: Some(tipo),
        };
        observar_ai0(
            &format!("indicadores / {tipo:?}"),
            esperado,
            repeticoes,
            || descoberta::buscar(&conn, &pedido, 0, 50),
        );
    }
    // Motor de leitura não escreve auditoria. Snapshot tem uma transação distinta.
    let auditoria_motor = banco.verificar_auditoria().unwrap();
    assert_eq!(auditoria_motor.registros, auditoria_antes.registros);
    assert_eq!(auditoria_motor.hash_final, auditoria_antes.hash_final);
    let exec = medir("AI-0 snapshot / expansão + transação auditada", || {
        banco.executar_descoberta(expansao, None)
    });
    match exec {
        Ok(exec) => {
            assert_eq!(exec.consulta.total_artefatos, esperados_expansao);
            let offset = (esperados_expansao - 50).max(0);
            observar_ai0("snapshot / última página", esperados_expansao, 1, || {
                banco.paginar_descoberta(exec.consulta.id, offset, 50)
            });
            assert_eq!(
                banco.verificar_auditoria().unwrap().registros,
                auditoria_antes.registros + 1
            );
        }
        Err(ErroArmazenamento::DescobertaLimite) => {
            println!("MEDIÇÃO INCOMPLETA snapshot: prazo cooperativo; nenhum snapshot publicado.");
            assert!(banco.listar_consultas_descoberta().unwrap().is_empty());
            assert_eq!(
                banco.verificar_auditoria().unwrap().registros,
                auditoria_antes.registros
            );
            panic!("snapshot: medição incompleta não aprova a validação");
        }
        Err(erro) => panic!("snapshot: falha inesperada na medição: {erro}"),
    }
    assert_eq!(comum::retrato(&origem), origem_antes);
    // Sem manter/copiar o caso, mesmo que SIED_MANTER esteja definido no ambiente.
}
