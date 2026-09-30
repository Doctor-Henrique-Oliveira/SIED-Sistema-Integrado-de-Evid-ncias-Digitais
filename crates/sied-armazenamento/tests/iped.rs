//! Caso do IPED de ponta a ponta, sobre um caso **SINTÉTICO**.
//!
//! ATENÇÃO: `testes/fixtures/iped/` foi montado para os testes. Só o que o código-fonte
//! público do IPED define foi imitado (pasta `iped/` com `IPEDConfig.txt`, `conf/` e
//! `index/`; a lista "Lista de Arquivos.csv"/"FileList.csv" com as 15 colunas; a pasta
//! "Exportados/arquivos" com arquivos nomeados pelo hash). Nomes, caminhos, hashes
//! fictícios e datas são inventados; o índice Lucene não existe.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod caso_sintetico;

use caso_sintetico::{caso, retrato};
use sied_armazenamento::extracoes::{ArtefatoDetalhe, Extracao};
use sied_armazenamento::BancoCaso;
use sied_dominio::Dispositivo;
use sied_ingestao::eventos::{EventoImportacao, NivelEvento};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::importador::{Importador, Suporte};
use sied_ingestao::importadores::iped::IpedImporter;
use sied_ingestao::pipeline::{self, Controle};
use sied_ingestao::Registro;
use std::fs;
use std::path::Path;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testes/fixtures/iped");

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
    let importador = IpedImporter;
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

fn por_nome<'a>(v: &'a [ArtefatoDetalhe], nome: &str) -> &'a ArtefatoDetalhe {
    v.iter()
        .find(|a| a.resumo.nome_arquivo.as_deref() == Some(nome))
        .unwrap_or_else(|| panic!("{nome} não encontrado"))
}

#[test]
fn caso_iped_em_portugues() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Caso-IPED-SINTETICO");
    copiar(&Path::new(FIXTURE).join("caso"), &origem);
    let antes = retrato(&origem);

    // Detecção pelo registro: o IPED vem antes da pasta genérica.
    let fonte = PastaSomenteLeitura::abrir(&origem).unwrap();
    let det = Registro::padrao().detectar(&fonte).unwrap();
    assert_eq!(det[0].importador.id, "iped");
    assert_eq!(det[0].suporte, Suporte::Parcial);

    let (banco, d) = caso(raiz.path());
    let (x, ev) = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    assert_eq!(x.tipo_extracao.as_deref(), Some("caso IPED"));
    assert_eq!(x.total_artefatos, Some(5));

    let v = detalhes(&banco, &x);
    assert_eq!(v.len(), 5);
    assert!(v.iter().all(|a| a.resumo.tipo == "arquivo"
        && a.resumo.app.as_deref() == Some("iped")
        && a.parser == "iped_lista_arquivos"));

    // Arquivo exportado: ligado pelo caminho exato do atalho, hashes do IPED guardados.
    let foto = por_nome(&v, "foto-sintetica.png");
    let anexo = foto
        .anexo_arquivo
        .as_ref()
        .expect("exportado ligado ao inventário");
    assert_eq!(
        anexo.caminho,
        "Exportados/arquivos/E/0/E062575EBA498606B50E1F64198BBF42.png"
    );
    assert_eq!(foto.resumo.anexo_vinculo.as_deref(), Some("caminho_exato"));
    assert_eq!(foto.resumo.subtipo.as_deref(), Some("item"));
    assert_eq!(foto.resumo.estado_registro, "ativo");
    assert_eq!(foto.metadados["md5"], "E062575EBA498606B50E1F64198BBF42");
    assert_eq!(
        foto.metadados["marcadores"],
        serde_json::json!(["Relevante", "Revisar"])
    );
    assert_eq!(
        foto.metadados["categorias"],
        serde_json::json!(["Other Images"])
    );
    // "Fri Jan 02 10:00:00 BRT 2026" = 13:00 UTC.
    assert_eq!(foto.resumo.data_hora_ms, Some(1_767_358_800_000));
    assert_eq!(
        foto.data_hora_original.as_deref(),
        Some("Fri Jan 02 10:00:00 BRT 2026")
    );
    assert_eq!(
        foto.id_externo.as_deref(),
        Some("0A1B2C3D4E5F60718293A4B5C6D7E8F9")
    );
    // A linha inteira fica como veio, pelos nomes do cabeçalho em português.
    let brutos = &foto.referencias[0].valores_brutos;
    assert_eq!(brutos["Modificação"], "Fri Jan 02 10:00:00 BRT 2026");
    assert_eq!(foto.referencias[0].id_registro.as_deref(), Some("linha=2"));

    let txt = por_nome(&v, "anotacoes-sinteticas.txt");
    assert!(txt.anexo_arquivo.is_some());
    assert_eq!(txt.resumo.data_hora_ms, Some(1_767_638_730_000)); // 18:45:30 UTC

    let apagada = por_nome(&v, "apagada-sintetica.jpg");
    assert_eq!(apagada.resumo.estado_registro, "apagado");
    assert!(apagada.anexo_arquivo.is_none());

    let recuperado = por_nome(&v, "Carved-000123.jpg");
    assert_eq!(recuperado.resumo.subtipo.as_deref(), Some("recuperado"));
    assert_eq!(recuperado.resumo.data_hora_ms, None);

    // Aspas e separador dentro do nome; fuso ambíguo (CET) fica só no original.
    let doc = por_nome(&v, "relatório \"final\"; v2.docx");
    assert_eq!(doc.resumo.data_hora_ms, None);
    assert_eq!(
        doc.data_hora_original.as_deref(),
        Some("Tue Jan 06 14:00:00 CET 2026")
    );
    assert!(
        ev.iter()
            .any(|e| e.nivel == NivelEvento::Aviso && e.mensagem.contains("CET")),
        "{ev:#?}"
    );

    // Registros conferem pelo hash; nada da pasta iped/ foi interpretado; origem intacta.
    for a in &v {
        assert!(banco.conferir_registro(a.resumo.id).unwrap().confere());
    }
    let execucoes = banco.execucoes(x.id).unwrap();
    assert_eq!(execucoes.len(), 1, "{execucoes:#?}");
    assert!(x.observacoes.iter().any(|o| o.contains("iped/")));
    assert_eq!(retrato(&origem), antes);
}

#[test]
fn lista_em_ingles_sem_pasta_do_caso() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Lista-SINTETICA");
    fs::create_dir_all(&origem).unwrap();
    fs::copy(
        Path::new(FIXTURE).join("FileList.csv"),
        origem.join("FileList.csv"),
    )
    .unwrap();
    let fonte = PastaSomenteLeitura::abrir(&origem).unwrap();
    assert_eq!(
        IpedImporter.detectar(&fonte).unwrap().suporte,
        Suporte::Parcial
    );

    let (banco, d) = caso(raiz.path());
    let (x, _) = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    let v = detalhes(&banco, &x);
    assert_eq!(v.len(), 2);
    // "GMT-03:00" explícito e "GMT".
    assert_eq!(
        por_nome(&v, "photo.png").resumo.data_hora_ms,
        Some(1_767_798_000_000)
    );
    let notas = por_nome(&v, "notes, draft.txt");
    assert_eq!(notas.resumo.data_hora_ms, Some(1_767_859_200_000));
    assert_eq!(
        notas.referencias[0].valores_brutos["Modified"],
        "Thu Jan 08 08:00:00 GMT 2026"
    );
}

#[test]
fn linhas_com_problema_viram_aviso_sem_parar_a_leitura() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Lista-SINTETICA");
    fs::create_dir_all(origem.join("iped")).unwrap();
    fs::write(origem.join("iped/IPEDConfig.txt"), "SINTETICO").unwrap();
    let cab = "\u{feff}\"Nome\";\"Atalho\";\"Tamanho\";\"Ext\";\"Marcador\";\"Categoria\";\"MD5\";\
               \"SHA1\";\"Deletado\";\"Recuperado\";\"Acesso\";\"Modificação\";\"Criação\";\
               \"Caminho\";\"TrackId\"\r\n";
    let ok = "\"a.txt\";\"\";\"1\";\"txt\";\"\";\"\";\"\";\"\";\"false\";\"false\";\"\";\"\";\"\";\"x/a.txt\";\"T1\"\r\n";
    let curta = "\"b.txt\";\"\";\"1\"\r\n";
    let ruim = "\"c.txt\";\"\";\"mil\";\"txt\";\"\";\"\";\"\";\"\";\"false\";\"false\";\"\";\"ontem\";\"\";\"x/c.txt\";\"T3\"\r\n";
    fs::write(
        origem.join("Lista de Arquivos.csv"),
        format!("{cab}{ok}{curta}{ruim}"),
    )
    .unwrap();

    let (banco, d) = caso(raiz.path());
    let (x, ev) = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    assert_eq!(x.total_artefatos, Some(2));
    assert_eq!(x.total_avisos, Some(3), "{ev:#?}");
    let avisos: Vec<&str> = ev
        .iter()
        .filter(|e| e.nivel == NivelEvento::Aviso)
        .map(|e| e.mensagem.as_str())
        .collect();
    assert!(
        avisos
            .iter()
            .any(|m| m.contains("linha=3") && m.contains("3 coluna(s)")),
        "{avisos:#?}"
    );
    assert!(
        avisos.iter().any(|m| m.contains("não numérico: mil")),
        "{avisos:#?}"
    );
    assert!(avisos.iter().any(|m| m.contains("ontem")), "{avisos:#?}");
}
