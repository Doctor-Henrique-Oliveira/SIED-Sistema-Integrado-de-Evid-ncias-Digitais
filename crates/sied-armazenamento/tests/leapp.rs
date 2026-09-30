//! Relatórios do ALEAPP e do iLEAPP de ponta a ponta, sobre relatórios **SINTÉTICOS**.
//!
//! ATENÇÃO: `testes/fixtures/leapp/` foi montado por `testes/fixtures/leapp/montar.py` para os
//! testes. Só o que o código-fonte público do ALEAPP e do iLEAPP define foi imitado (a pasta
//! `_TSV Exports` gravada pelo `csv.writer` do Python, o projeto `_lava_data.lava`, os nomes
//! dos artefatos e das colunas); os valores são inventados.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod caso_sintetico;

use caso_sintetico::{caso, retrato};
use sied_armazenamento::extracoes::{ArtefatoDetalhe, Extracao};
use sied_armazenamento::BancoCaso;
use sied_dominio::Dispositivo;
use sied_ingestao::eventos::{EventoImportacao, NivelEvento};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::importador::{Importador, Suporte};
use sied_ingestao::importadores::leapp::LeappImporter;
use sied_ingestao::pipeline::{self, Controle};
use sied_ingestao::Registro;
use std::fs;
use std::path::Path;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testes/fixtures/leapp");

fn copiar(de: &Path, para: &Path) {
    fs::create_dir_all(para).unwrap();
    for e in fs::read_dir(de).unwrap() {
        let e = e.unwrap();
        let destino = para.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copiar(&e.path(), &destino);
        } else {
            fs::copy(e.path(), destino).unwrap();
        }
    }
}

fn importar(
    banco: &BancoCaso,
    d: &Dispositivo,
    origem: &Path,
) -> (Extracao, Vec<EventoImportacao>) {
    let importador = LeappImporter;
    let mut imp = banco
        .iniciar_importacao(d.id, &importador.info(), origem)
        .unwrap();
    let fonte = PastaSomenteLeitura::abrir(origem).unwrap();
    let ctx = imp.contexto().clone();
    let controle = Controle::default();
    let r = pipeline::executar(
        &fonte,
        &importador,
        &Registro::padrao(),
        &mut imp,
        &controle,
        &ctx,
    );
    let x = imp.finalizar(&r).unwrap();
    (x, controle.eventos_desde(0))
}

fn detalhes(banco: &BancoCaso, x: &Extracao) -> Vec<ArtefatoDetalhe> {
    banco
        .artefatos(x.id, 0, 100)
        .unwrap()
        .itens
        .iter()
        .map(|a| banco.artefato(a.id).unwrap())
        .collect()
}

fn do_artefato<'a>(v: &'a [ArtefatoDetalhe], nome: &str) -> Vec<&'a ArtefatoDetalhe> {
    let mut r: Vec<&ArtefatoDetalhe> = v
        .iter()
        .filter(|a| a.resumo.subtipo.as_deref() == Some(nome))
        .collect();
    r.sort_by(|a, b| {
        a.referencias[0]
            .id_registro
            .cmp(&b.referencias[0].id_registro)
    });
    r
}

#[test]
fn relatorio_do_aleapp() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("ALEAPP_Reports_SINTETICO");
    copiar(&Path::new(FIXTURE).join("aleapp"), &origem);
    let antes = retrato(&origem);

    let fonte = PastaSomenteLeitura::abrir(&origem).unwrap();
    let det = Registro::padrao().detectar(&fonte).unwrap();
    assert_eq!(det[0].importador.id, "leapp");
    assert_eq!(det[0].suporte, Suporte::Parcial);

    let (banco, d) = caso(raiz.path());
    let (x, ev) = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    assert_eq!(x.tipo_extracao.as_deref(), Some("relatório do ALEAPP"));
    assert_eq!(x.ferramenta_versao.as_deref(), Some("ALEAPP 2026.4.2"));
    assert_eq!(x.total_artefatos, Some(7));
    assert_eq!(x.total_avisos, Some(1), "{ev:#?}");

    let v = detalhes(&banco, &x);
    assert!(v.iter().all(|a| a.resumo.tipo == "outro"
        && a.resumo.app.as_deref() == Some("aleapp")
        && a.parser == "aleapp_tsv"));

    // SMS: fuso explícito convertido; campo com TAB, aspas e quebra de linha inteiro.
    let sms = do_artefato(&v, "SMS Messages");
    assert_eq!(sms.len(), 3);
    assert_eq!(sms[0].resumo.data_hora_ms, Some(1_767_358_800_000));
    assert_eq!(sms[1].resumo.data_hora_ms, Some(1_767_359_100_250));
    assert_eq!(sms[2].resumo.data_hora_ms, Some(1_767_443_400_000)); // 09:30 -03:00
    assert_eq!(
        sms[1].referencias[0].valores_brutos["Body"],
        "Com\ttab, \"aspas\" e\nduas linhas"
    );
    assert_eq!(
        sms[1].referencias[0].id_registro.as_deref(),
        Some("linha=3")
    );
    assert_eq!(
        sms[1].referencias[0].tabela.as_deref(),
        Some("SMS Messages")
    );
    assert!(sms[0]
        .resumo
        .conteudo
        .as_deref()
        .unwrap()
        .contains("Body: Mensagem sintética 1"));

    // Call logs: o cabeçalho repetido (com outro BOM) não vira registro; data sem fuso fica
    // só no original, com um aviso.
    let chamadas = do_artefato(&v, "Call logs ");
    assert_eq!(chamadas.len(), 2);
    assert_eq!(chamadas[0].resumo.data_hora_ms, Some(1_767_549_600_000));
    assert_eq!(chamadas[1].resumo.data_hora_ms, None);
    assert_eq!(
        chamadas[1].data_hora_original.as_deref(),
        Some("2026-01-04 19:00:00")
    );
    assert!(
        ev.iter().any(|e| e.nivel == NivelEvento::Aviso
            && e.mensagem.contains("Call Date")
            && e.mensagem.contains("sem fuso")),
        "{ev:#?}"
    );

    // Cache Location: data na coluna Readtime; coordenadas; 0,0 não é posição.
    let locais = do_artefato(&v, "Cache Location");
    assert_eq!(locais.len(), 2);
    assert_eq!(locais[0].resumo.latitude, Some(-15.7939));
    assert_eq!(locais[0].resumo.longitude, Some(-47.8828));
    assert_eq!(locais[0].resumo.data_hora_ms, Some(1_767_607_200_000));
    assert_eq!(locais[0].metadados["coluna_data"], "Readtime");
    assert_eq!(locais[1].resumo.latitude, None);
    assert_eq!(locais[1].resumo.data_hora_ms, None);

    for a in &v {
        assert!(banco.conferir_registro(a.resumo.id).unwrap().confere());
    }
    assert_eq!(banco.execucoes(x.id).unwrap().len(), 3);
    assert!(x.observacoes.iter().any(|o| o.contains("ALEAPP 2026.4.2")));
    assert_eq!(retrato(&origem), antes);
}

#[test]
fn relatorio_do_ileapp() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("iLEAPP_Reports_SINTETICO");
    copiar(&Path::new(FIXTURE).join("ileapp"), &origem);
    let (banco, d) = caso(raiz.path());
    let (x, _) = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    assert_eq!(x.tipo_extracao.as_deref(), Some("relatório do iLEAPP"));
    let v = detalhes(&banco, &x);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].resumo.app.as_deref(), Some("ileapp"));
    assert_eq!(v[0].parser, "ileapp_tsv");
    assert_eq!(v[0].resumo.data_hora_ms, Some(1_767_708_000_000));
}
