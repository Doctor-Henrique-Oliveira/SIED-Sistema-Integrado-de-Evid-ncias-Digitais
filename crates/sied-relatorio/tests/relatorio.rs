//! Relatório PDF sobre uma extração **SINTÉTICA** (ver `testes/rust/caso_sintetico.rs`).

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use chrono::{TimeZone, Utc};
use comum::*;
use rusqlite::Connection;
use sied_armazenamento::caso::NOME_ARQUIVO_CASO;
use sied_armazenamento::evidencias::NovaEvidencia;
use sied_armazenamento::extracoes::Extracao;
use sied_armazenamento::BancoCaso;
use sied_relatorio::montagem;
use sied_relatorio::{ErroRelatorio, PedidoRelatorio};
use std::fs;
use std::path::Path;

/// Registro (foto sem imagem válida), trecho de 5 mensagens e a foto da câmera.
fn marcar(banco: &mut BancoCaso, x: &Extracao) {
    let msgs = mensagens_ana(banco);
    let foto = mensagem(&msgs, "SINT0000000003").id;
    banco
        .adicionar_evidencia(NovaEvidencia {
            artefato_id: Some(foto),
            categoria: Some("Mensagens".into()),
            observacao: Some("Observação sintética do analista.".into()),
            ..Default::default()
        })
        .unwrap();
    banco
        .adicionar_evidencia(NovaEvidencia {
            artefato_id: Some(msgs[0].id),
            trecho_fim_id: Some(msgs[4].id),
            descricao_contextual: Some("Trecho sintético usado no teste do relatório.".into()),
            ..Default::default()
        })
        .unwrap();
    banco
        .adicionar_evidencia(NovaEvidencia {
            arquivo_id: Some(id_arquivo(banco, x, CAMINHO_FOTO)),
            titulo: Some("Foto sintética da câmera".into()),
            ..Default::default()
        })
        .unwrap();
}

/// Para inspecionar o PDF à mão: `SIED_RELATORIO_SAIDA=/pasta cargo test -p sied-relatorio`.
fn guardar(pdf: &Path, nome: &str) {
    if let Ok(dir) = std::env::var("SIED_RELATORIO_SAIDA") {
        fs::create_dir_all(&dir).unwrap();
        fs::copy(pdf, Path::new(&dir).join(nome)).unwrap();
    }
}

fn situacoes(banco: &BancoCaso) -> Vec<String> {
    let auditoria = banco.verificar_auditoria().unwrap();
    montagem::montar(banco, &PedidoRelatorio::default(), Utc::now(), &auditoria)
        .unwrap()
        .conferencias
        .into_iter()
        .map(|c| c.situacao)
        .collect()
}

#[test]
fn gera_pdf_com_sha256_registro_e_auditoria_sem_tocar_na_extracao() {
    let (_raiz, origem, mut banco, x) = preparado();
    marcar(&mut banco, &x);
    let antes = retrato(&origem);
    let seq_antes = banco.verificar_auditoria().unwrap();

    let agora = Utc.with_ymd_and_hms(2026, 9, 29, 3, 4, 5).unwrap();
    let r = sied_relatorio::gerar_em(
        &mut banco,
        PedidoRelatorio {
            titulo: Some("Relatório sintético de teste".into()),
            observacoes: Some("Observações gerais sintéticas.".into()),
            conclusao: Some("Conclusão sintética.".into()),
            ..Default::default()
        },
        agora,
    )
    .unwrap();

    // Nome no fuso do caso (America/Sao_Paulo, UTC-3).
    assert_eq!(r.caminho, "relatorios/2026-09-29_000405_relatorio.pdf");
    let pdf = banco.pasta().join(&r.caminho);
    let bytes = fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert_eq!(r.sha256, sha256(&pdf));
    assert_eq!(r.tamanho, bytes.len() as i64);
    assert_eq!(
        fs::read_to_string(pdf.with_extension("pdf.sha256")).unwrap(),
        format!("{}  2026-09-29_000405_relatorio.pdf\n", r.sha256)
    );
    assert_eq!(r.gerado_por, "analista.teste");
    assert_eq!(r.auditoria_ate_seq, seq_antes.registros);
    assert_eq!(r.auditoria_hash, seq_antes.hash_final);
    let evs = r.evidencias.as_array().unwrap();
    assert_eq!(evs.len(), 3);
    assert!(evs.iter().all(|e| e["situacao"] == "conferida"), "{evs:?}");
    guardar(&pdf, "relatorio-sintetico.pdf");

    // Registrado e auditado; a trilha continua íntegra.
    assert_eq!(banco.relatorios().unwrap().len(), 1);
    let ultimo = &banco.auditoria(1).unwrap()[0];
    assert_eq!(ultimo.acao, "relatorio_gerado");
    assert_eq!(ultimo.detalhes["sha256"], r.sha256.as_str());
    assert_eq!(ultimo.seq, seq_antes.registros + 1);
    banco.verificar_auditoria().unwrap();

    // Mesmo segundo: não sobrescreve o anterior.
    let r2 = sied_relatorio::gerar_em(&mut banco, PedidoRelatorio::default(), agora).unwrap();
    assert_eq!(r2.caminho, "relatorios/2026-09-29_000405_relatorio-2.pdf");
    assert!(pdf.exists());
    assert_eq!(banco.relatorios().unwrap().len(), 2);

    // Nada gravado na extração; sem arquivos temporários na pasta de relatórios.
    assert_eq!(retrato(&origem), antes, "a extração não pode mudar");
    let nomes: Vec<String> = fs::read_dir(banco.pasta().join("relatorios"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(nomes.len(), 4, "{nomes:?}");
    assert!(nomes
        .iter()
        .all(|n| n.ends_with(".pdf") || n.ends_with(".pdf.sha256")));

    // O registro do relatório não muda nem some.
    let c = Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap();
    assert!(c.execute("UPDATE relatorio SET sha256 = 'x'", []).is_err());
    assert!(c.execute("DELETE FROM relatorio", []).is_err());
}

#[test]
fn documento_traz_origem_custodia_e_figura_conferida() {
    let (_raiz, _origem, mut banco, x) = preparado();
    marcar(&mut banco, &x);
    let auditoria = banco.verificar_auditoria().unwrap();
    let m = montagem::montar(&banco, &PedidoRelatorio::default(), Utc::now(), &auditoria).unwrap();
    let d = &m.documento;
    assert_eq!(d.titulo, montagem::TITULO_PADRAO);
    assert_eq!(d.identificacao, "IP 7/2026 (sintético)");
    assert!(
        d.fuso.starts_with("America/Sao_Paulo, UTC-03:00"),
        "{}",
        d.fuso
    );
    assert_eq!(d.dispositivos.len(), 1);
    assert_eq!(d.extracoes.len(), 1);
    assert!(d.extracoes[0]
        .campos
        .iter()
        .any(|c| c.rotulo == "Hash do manifesto (SHA-256)"));
    assert_eq!(d.evidencias.len(), 3);

    let json = serde_json::to_value(d).unwrap();
    let e = &json["evidencias"];
    assert_eq!(e[0]["referencia"], "E-1");
    assert_eq!(e[0]["conteudo"]["tipo"], "conversa");
    // A foto da mensagem tem 64 bytes que não formam imagem: fica o motivo, sem figura.
    let msg = &e[0]["conteudo"]["mensagens"][0];
    assert!(msg["figura"].is_null());
    let detalhes = msg["detalhes"].to_string();
    assert!(detalhes.contains("não pôde ser decodificada"), "{detalhes}");
    // Trecho: 5 mensagens, apêndice com uma linha por mensagem e o hash do trecho.
    assert_eq!(e[1]["conteudo"]["mensagens"].as_array().unwrap().len(), 5);
    assert_eq!(e[1]["origem"][0]["linhas"].as_array().unwrap().len(), 5);
    assert!(e[1]["origem"][0]["campos"]
        .to_string()
        .contains("Hash do trecho"));
    // Origem do registro: arquivo, tabela, parser e valores brutos.
    let ref0 = &e[0]["origem"][0]["referencias"][0];
    assert!(ref0["campos"].to_string().contains("msgstore.db"), "{ref0}");
    assert!(ref0["valores_brutos"]
        .as_str()
        .unwrap()
        .contains("key_id = \"SINT0000000003\""));
    // Arquivo: figura conferida.
    assert_eq!(e[2]["conteudo"]["tipo"], "arquivo");
    assert!(e[2]["conteudo"]["figura"]["arquivo"].is_string());
    assert_eq!(m.imagens.len(), 1);
    assert!(m
        .imagens
        .values()
        .next()
        .unwrap()
        .starts_with(&[0xFF, 0xD8]));
    assert!(m.conferencias.iter().all(|c| c.situacao == "conferida"));
}

#[test]
fn arquivo_alterado_diverge_e_pasta_ausente_fica_sem_conferencia() {
    let (_raiz, origem, mut banco, x) = preparado();
    marcar(&mut banco, &x);

    // Alguém troca a foto na pasta da extração depois da importação.
    let foto = origem.join(CAMINHO_FOTO);
    let mut bytes = fs::read(&foto).unwrap();
    let n = bytes.len();
    bytes[n - 20] ^= 0xFF;
    fs::write(&foto, &bytes).unwrap();
    assert_eq!(situacoes(&banco), ["conferida", "conferida", "divergente"]);

    let r = sied_relatorio::gerar(&mut banco, PedidoRelatorio::default()).unwrap();
    assert_eq!(r.evidencias[2]["situacao"], "divergente");
    guardar(&banco.pasta().join(&r.caminho), "relatorio-divergente.pdf");

    // Disco da extração desconectado: registros conferem pelo banco do caso, o
    // arquivo fica sem conferência (vale o SHA-256 do inventário).
    fs::rename(&origem, origem.with_file_name("desconectada")).unwrap();
    assert_eq!(
        situacoes(&banco),
        ["conferida", "conferida", "indisponivel"]
    );
}

#[test]
fn trilha_de_auditoria_adulterada_impede_o_relatorio() {
    let (_raiz, _origem, mut banco, x) = preparado();
    marcar(&mut banco, &x);
    let pasta = banco.pasta().to_path_buf();
    let c = Connection::open(pasta.join(NOME_ARQUIVO_CASO)).unwrap();
    c.execute_batch(
        "DROP TRIGGER auditoria_sem_update;
         UPDATE auditoria SET detalhes = '{\"adulterado\":true}' WHERE seq = 2;",
    )
    .unwrap();
    drop(c);
    match sied_relatorio::gerar(&mut banco, PedidoRelatorio::default()) {
        Err(ErroRelatorio::Auditoria(m)) => assert!(m.contains("registro 2"), "{m}"),
        outro => panic!("esperava recusa pela auditoria, veio {outro:?}"),
    }
    assert!(!pasta.join("relatorios").exists());
    assert!(banco.relatorios().unwrap().is_empty());
}

/// Recalcula a cadeia inteira como faria quem a reescrevesse fora do SIED.
fn reescrever_trilha(c: &Connection) {
    use sha2::{Digest, Sha256};
    let linhas: Vec<(i64, String, String, String, String, String)> = c
        .prepare("SELECT seq, ts, usuario, maquina, acao, detalhes FROM auditoria ORDER BY seq")
        .unwrap()
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut anterior = sied_armazenamento::auditoria::HASH_GENESE.to_string();
    for (seq, ts, usuario, maquina, acao, detalhes) in linhas {
        let hash = hex::encode(Sha256::digest(
            format!(
                "{seq}\x1f{ts}\x1f{usuario}\x1f{maquina}\x1f{acao}\x1f{detalhes}\x1f{anterior}"
            )
            .as_bytes(),
        ));
        c.execute(
            "UPDATE auditoria SET hash_anterior = ?1, hash = ?2 WHERE seq = ?3",
            rusqlite::params![anterior, hash, seq],
        )
        .unwrap();
        anterior = hash;
    }
}

/// Apagar os registros finais da trilha, ou reescrevê-la inteira, deixa uma cadeia que
/// sozinha parece íntegra; a âncora gravada pelo relatório denuncia.
#[test]
fn ancora_do_relatorio_denuncia_trilha_truncada_ou_reescrita() {
    let agora = Utc.with_ymd_and_hms(2026, 3, 4, 15, 16, 17).unwrap();
    for truncar in [true, false] {
        let (_raiz, _origem, mut banco, x) = preparado();
        marcar(&mut banco, &x);
        let r = sied_relatorio::gerar_em(&mut banco, PedidoRelatorio::default(), agora).unwrap();
        banco.verificar_auditoria().unwrap();
        let c = Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap();
        if truncar {
            c.execute_batch("DROP TRIGGER auditoria_sem_delete;")
                .unwrap();
            c.execute(
                "DELETE FROM auditoria WHERE seq >= ?1",
                [r.auditoria_ate_seq],
            )
            .unwrap();
        } else {
            c.execute_batch(
                "DROP TRIGGER auditoria_sem_update;
                 UPDATE auditoria SET detalhes = '{\"adulterado\":true}' WHERE seq = 2;",
            )
            .unwrap();
            reescrever_trilha(&c);
        }
        drop(c);
        match banco.verificar_auditoria() {
            Err(sied_armazenamento::ErroArmazenamento::AuditoriaCorrompida { seq, motivo }) => {
                assert_eq!(seq, r.auditoria_ate_seq);
                let esperado = if truncar {
                    "não existe mais"
                } else {
                    "reescrita"
                };
                assert!(motivo.contains(esperado), "{motivo}");
            }
            outro => panic!("esperava trilha corrompida (truncar={truncar}), veio {outro:?}"),
        }
    }
}

#[test]
fn caso_sem_evidencias_gera_relatorio_so_com_as_secoes_escolhidas() {
    let (_raiz, _origem, mut banco, _x) = preparado();
    let r = sied_relatorio::gerar(
        &mut banco,
        PedidoRelatorio {
            secoes: Some(sied_relatorio::SecoesRelatorio {
                capa: false,
                apendice_tecnico: false,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(r.secoes["capa"], false);
    assert!(r.evidencias.as_array().unwrap().is_empty());
    assert!(fs::read(banco.pasta().join(&r.caminho))
        .unwrap()
        .starts_with(b"%PDF-"));
}

/// O banco do caso é alterado com outra ferramenta (gatilhos desligados): o conteúdo da
/// mensagem muda, mas a coluna hash_registro e o hash da evidência continuam os mesmos.
/// A conferência recalcula o hash do conteúdo e acusa a divergência.
#[test]
fn conteudo_do_registro_alterado_no_banco_do_caso_diverge() {
    let (_raiz, _origem, mut banco, x) = preparado();
    marcar(&mut banco, &x);
    assert_eq!(situacoes(&banco), ["conferida", "conferida", "conferida"]);
    let msgs = mensagens_ana(&banco);
    // A mensagem 2 da conversa está no trecho (E-2), mas não é o registro de E-1.
    let alvo = msgs[1].id;
    let c = Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap();
    c.execute_batch("DROP TRIGGER artefato_sem_update;")
        .unwrap();
    c.execute(
        "UPDATE artefato SET conteudo = 'texto trocado depois' WHERE id = ?1",
        [alvo],
    )
    .unwrap();
    drop(c);
    assert_eq!(situacoes(&banco), ["conferida", "divergente", "conferida"]);

    let auditoria = banco.verificar_auditoria().unwrap();
    let m = montagem::montar(&banco, &PedidoRelatorio::default(), Utc::now(), &auditoria).unwrap();
    let e2 = &m.documento.evidencias[1];
    assert_eq!(e2.situacao, "divergente");
    assert!(
        e2.situacao_texto.contains(&format!("registro {alvo}")),
        "{}",
        e2.situacao_texto
    );

    // O registro de E-1 também é adulterado: diverge mesmo com hash e coluna intactos.
    let foto = mensagem(&msgs, "SINT0000000003").id;
    let c = Connection::open(banco.pasta().join(NOME_ARQUIVO_CASO)).unwrap();
    c.execute(
        "UPDATE artefato SET remetente = 'outra pessoa' WHERE id = ?1",
        [foto],
    )
    .unwrap();
    drop(c);
    assert_eq!(situacoes(&banco), ["divergente", "divergente", "conferida"]);
    let r = sied_relatorio::gerar(&mut banco, PedidoRelatorio::default()).unwrap();
    assert_eq!(r.evidencias[0]["situacao"], "divergente");
    assert_eq!(r.evidencias[1]["situacao"], "divergente");
}

/// Todo texto do documento diagramado, com a borda direita de cada trecho (em pt).
fn textos_diagramados(doc: &typst_layout::PagedDocument) -> Vec<(usize, String, f64, f64)> {
    use typst::layout::{Frame, FrameItem, Point};
    fn visitar(
        f: &Frame,
        origem: Point,
        pagina: usize,
        largura: f64,
        saida: &mut Vec<(usize, String, f64, f64)>,
    ) {
        for (pos, item) in f.items() {
            let p = origem + *pos;
            match item {
                FrameItem::Group(g) => {
                    let t = Point::new(g.transform.tx, g.transform.ty);
                    visitar(&g.frame, p + t, pagina, largura, saida)
                }
                FrameItem::Text(t) => saida.push((
                    pagina,
                    // Glifo 0 (".notdef") é o quadrado vazio de caractere sem desenho.
                    if t.glyphs.iter().any(|g| g.id == 0) {
                        format!("<SEM DESENHO>{}", t.text)
                    } else {
                        t.text.to_string()
                    },
                    (p.x + t.width()).to_pt(),
                    largura,
                )),
                _ => {}
            }
        }
    }
    let mut v = Vec::new();
    for (i, pg) in doc.pages().iter().enumerate() {
        let largura = pg.frame.width().to_pt();
        visitar(&pg.frame, Point::zero(), i + 1, largura, &mut v);
    }
    v
}

/// Textos digitados pelo analista (e conteúdo vindo da extração) com sintaxe do Typst,
/// caracteres de controle, emojis, direção invertida e palavras enormes: tudo sai como
/// texto literal, dentro da página, sem derrubar a geração.
#[test]
fn textos_hostis_viram_texto_literal_dentro_da_pagina() {
    use sied_armazenamento::evidencias::TextosEvidencia;
    let (_raiz, _origem, mut banco, x) = preparado();
    marcar(&mut banco, &x);
    let sintaxe = r#"#panic("injetado") #import "/etc/passwd" $x^2$ <rotulo> @rotulo ] [ *negrito* _it_ `raw` \ // comentário /* bloco"#;
    let controles = "NUL\u{0}BEL\u{7}ESC\u{1b}[31m DEL\u{7f} TAB\t CR\r fim";
    let palavra = "A".repeat(3000);
    let url = format!("https://exemplo.invalid/{}", "caminho/".repeat(300));
    let emojis = "👩‍👩‍👧‍👦 🇧🇷 ✅ 🔒 \u{202E}texto invertido\u{202C} zero\u{200B}width ﷽";
    let evs = banco.evidencias().unwrap();
    banco
        .alterar_evidencia(
            evs[0].id,
            TextosEvidencia {
                titulo: Some(sintaxe.into()),
                observacao: Some(format!("{controles} {palavra}")),
                descricao_contextual: Some(emojis.into()),
                categoria: Some(url.clone()),
            },
        )
        .unwrap();
    banco
        .alterar_evidencia(
            evs[1].id,
            TextosEvidencia {
                observacao: Some("linha\n".repeat(3000)),
                ..Default::default()
            },
        )
        .unwrap();
    let pedido = PedidoRelatorio {
        titulo: Some(format!("Relatório {sintaxe}")),
        observacoes: Some(format!("{emojis}\n\n{palavra}")),
        conclusao: Some(controles.into()),
        ..Default::default()
    };
    let auditoria = banco.verificar_auditoria().unwrap();
    let agora = Utc::now();
    let m = montagem::montar(&banco, &pedido, agora, &auditoria).unwrap();
    let dados = serde_json::to_string(&m.documento).unwrap();
    let doc = sied_relatorio::motor::paginar(&dados, &m.imagens, agora).expect("diagrama");

    let textos = textos_diagramados(&doc);
    let tudo: String = textos.iter().map(|t| t.1.as_str()).collect();
    assert!(
        tudo.contains("#panic(\"injetado\")"),
        "sintaxe do Typst virou texto"
    );
    assert!(tudo.contains("#import"));
    // Emojis e controles aparecem pelo código; nenhum quadrado vazio no documento.
    assert!(tudo.contains("[U+1F469 U+200D U+1F469 U+200D U+1F467 U+200D U+1F466]"));
    assert!(tudo.contains("NUL[U+0000]BEL[U+0007]"));
    assert!(tudo.contains("[U+202E]texto invertido[U+202C]"));
    let sem_desenho: Vec<_> = textos
        .iter()
        .filter(|t| t.1.starts_with("<SEM DESENHO>"))
        .map(|t| t.1.clone())
        .collect();
    assert!(sem_desenho.is_empty(), "{sem_desenho:?}");
    // Nada passa da margem direita (2,1 cm) além de uma pequena tolerância.
    let fora: Vec<_> = textos
        .iter()
        .filter(|(_, _, direita, largura)| *direita > largura - 50.0)
        .map(|(p, t, d, l)| {
            format!(
                "pág. {p}: {:.0} > {:.0}: {}",
                d,
                l - 59.5,
                &t[..t.len().min(40)]
            )
        })
        .collect();
    assert!(
        fora.is_empty(),
        "texto fora da página:\n{}",
        fora.join("\n")
    );

    // E o PDF é gerado e registrado.
    let r = sied_relatorio::gerar(&mut banco, pedido).unwrap();
    assert!(fs::read(banco.pasta().join(&r.caminho))
        .unwrap()
        .starts_with(b"%PDF-"));
    guardar(
        &banco.pasta().join(&r.caminho),
        "relatorio-textos-hostis.pdf",
    );
}
