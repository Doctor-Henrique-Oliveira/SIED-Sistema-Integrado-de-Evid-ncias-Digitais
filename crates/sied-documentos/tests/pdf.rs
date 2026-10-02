//! PDFs exclusivamente SINTÉTICOS, produzidos pelo writer oficial de `lopdf`.
//! Os arquivos da origem são criados antes da importação. Depois disso, apenas
//! pedidos incorretos e fontes simuladas exercitam falhas; nenhuma evidência é
//! adulterada, nem há escrita direta no banco do caso.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use lopdf::content::{Content, Operation};
use lopdf::xref::XrefType;
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use sha2::{Digest, Sha256};
use sied_documentos::{
    analisar, analisar_pagina, ControleDocumento, EstadoDocumento, ModoDocumento, PedidoDocumento,
    ResultadoDocumento, TAMANHO_MAXIMO,
};
use sied_dominio::{CasoId, ExtracaoId};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::fonte::{CaminhoFonte, EntradaFonte, FonteSomenteLeitura, LeitorFonte};
use sied_ingestao::Resultado;
use std::fs;
use std::io::{self, Cursor, Read};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn fonte_base() -> Dictionary {
    dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    }
}

fn operacoes_texto(bytes: &[u8]) -> Vec<Operation> {
    vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 12.into()]),
        Operation::new("Td", vec![70.into(), 700.into()]),
        Operation::new("Tj", vec![Object::string_literal(bytes)]),
        Operation::new("ET", vec![]),
    ]
}

fn conteudo(bytes: &[u8]) -> Vec<u8> {
    Content {
        operations: operacoes_texto(bytes),
    }
    .encode()
    .unwrap()
}

struct PdfSintetico {
    documento: Document,
    paginas: Vec<ObjectId>,
    fonte_id: ObjectId,
    recursos_id: ObjectId,
    paginas_id: ObjectId,
}

impl PdfSintetico {
    /// Cada página pode conter zero, um ou vários streams reais de conteúdo.
    fn novo(paginas: Vec<Vec<Vec<u8>>>, comprimir: bool) -> Self {
        let mut documento = Document::with_version("1.4");
        documento.reference_table.cross_reference_type = XrefType::CrossReferenceTable;
        let paginas_id = documento.new_object_id();
        let fonte_id = documento.add_object(fonte_base());
        let recursos_id = documento.add_object(dictionary! {
            "Font" => dictionary! { "F1" => fonte_id },
        });
        let mut ids = Vec::new();
        for streams in paginas {
            let conteudos: Vec<Object> = streams
                .into_iter()
                .map(|bytes| {
                    let mut stream = Stream::new(dictionary! {}, bytes);
                    if comprimir {
                        stream.compress().unwrap();
                    }
                    documento.add_object(stream).into()
                })
                .collect();
            let mut pagina = dictionary! {
                "Type" => "Page",
                "Parent" => paginas_id,
                "Resources" => recursos_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            };
            match conteudos.as_slice() {
                [] => {}
                [unico] => pagina.set("Contents", unico.clone()),
                _ => pagina.set("Contents", conteudos),
            }
            ids.push(documento.add_object(pagina));
        }
        documento.set_object(
            paginas_id,
            dictionary! {
                "Type" => "Pages",
                "Kids" => ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>(),
                "Count" => ids.len() as i64,
            },
        );
        let catalogo = documento.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => paginas_id,
        });
        documento.trailer.set("Root", catalogo);
        Self {
            documento,
            paginas: ids,
            fonte_id,
            recursos_id,
            paginas_id,
        }
    }

    fn bytes(mut self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.documento.save_to(&mut bytes).unwrap();
        bytes
    }
}

fn duas_paginas() -> Vec<u8> {
    PdfSintetico::novo(
        vec![
            vec![conteudo(b"Documento SINTETICO 1: a\xe7\xe3o - caf\xe9")],
            vec![conteudo(b"Documento SINTETICO 2: outra pagina")],
        ],
        false,
    )
    .bytes()
}

struct FonteMemoria {
    caminho: CaminhoFonte,
    bytes: Arc<[u8]>,
    aberturas: AtomicUsize,
    cancelar_ao_ler: Option<Arc<ControleDocumento>>,
}

impl FonteMemoria {
    fn nova(caminho: &str, bytes: Vec<u8>) -> Self {
        Self {
            caminho: CaminhoFonte::novo(caminho).unwrap(),
            bytes: bytes.into(),
            aberturas: AtomicUsize::new(0),
            cancelar_ao_ler: None,
        }
    }

    fn pedido(&self) -> PedidoDocumento {
        PedidoDocumento {
            caso_id: CasoId::novo(),
            extracao_id: ExtracaoId::novo(),
            arquivo_id: 17,
            caminho: self.caminho.clone(),
            sha256: Some(hash(&self.bytes)),
            tamanho: self.bytes.len() as u64,
            modo: ModoDocumento::Automatico,
        }
    }

    fn sem_abertura(&self) {
        assert_eq!(self.aberturas.load(Ordering::SeqCst), 0);
    }
}

impl FonteSomenteLeitura for FonteMemoria {
    fn descricao(&self) -> String {
        panic!("o motor não precisa consultar descrição da fonte")
    }

    fn listar(&self) -> Box<dyn Iterator<Item = Resultado<EntradaFonte>> + '_> {
        panic!("o motor não pode percorrer a fonte")
    }

    fn entrada(&self, _: &CaminhoFonte) -> Resultado<Option<EntradaFonte>> {
        panic!("metadados devem vir do inventário, não de uma nova consulta")
    }

    fn abrir(&self, caminho: &CaminhoFonte) -> Resultado<Box<dyn LeitorFonte>> {
        assert_eq!(caminho, &self.caminho);
        self.aberturas.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(LeitorMemoria {
            cursor: Cursor::new(Arc::clone(&self.bytes)),
            cancelar_ao_ler: self.cancelar_ao_ler.clone(),
        }))
    }
}

struct LeitorMemoria {
    cursor: Cursor<Arc<[u8]>>,
    cancelar_ao_ler: Option<Arc<ControleDocumento>>,
}

impl Read for LeitorMemoria {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let lido = self.cursor.read(buffer)?;
        if lido > 0 {
            if let Some(controle) = &self.cancelar_ao_ler {
                controle.cancelar();
            }
        }
        Ok(lido)
    }
}

fn recusa(resultado: ResultadoDocumento, esperado: EstadoDocumento) {
    assert_eq!(resultado.estado, esperado, "{resultado:?}");
    assert!(resultado.texto.is_none(), "texto parcial não deve sair");
    assert!(resultado.motivo.is_some_and(|motivo| !motivo.is_empty()));
}

#[test]
fn pdf_nativo_seleciona_pagina_real_e_preserva_proveniencia() {
    let fonte = FonteMemoria::nova("documentos/SINTETICO.PDF", duas_paginas());
    let pedido = fonte.pedido();
    let original = fonte.bytes.to_vec();
    for (pagina, esperado) in [
        (None, "Documento SINTETICO 1: ação - café\n"),
        (Some(1), "Documento SINTETICO 1: ação - café\n"),
        (Some(2), "Documento SINTETICO 2: outra pagina\n"),
    ] {
        let inicio = chrono::Utc::now();
        let resultado = analisar_pagina(&fonte, &pedido, &ControleDocumento::novo(), pagina);
        let fim = chrono::Utc::now();
        assert_eq!(resultado.estado, EstadoDocumento::Pronto, "{resultado:?}");
        assert!(resultado.motivo.is_none());
        let texto = resultado.texto.unwrap();
        assert_eq!(texto.texto, esperado);
        assert_eq!(texto.pagina, Some(pagina.unwrap_or(1)));
        assert_eq!(texto.total_paginas, Some(2));
        assert_eq!(texto.caso_id, pedido.caso_id);
        assert_eq!(texto.extracao_id, pedido.extracao_id);
        assert_eq!(texto.arquivo_id, pedido.arquivo_id);
        assert_eq!(texto.caminho, pedido.caminho);
        assert_eq!(Some(texto.sha256.clone()), pedido.sha256);
        assert_eq!(texto.sha256_texto, hash(texto.texto.as_bytes()));
        assert_ne!(texto.sha256_texto, texto.sha256);
        assert_eq!(texto.motor_id, "pdf_nativo_lopdf");
        assert!(!texto.motor_versao.is_empty());
        assert!(texto.executado_em >= inicio && texto.executado_em <= fim);
        let serializado = serde_json::to_value(&texto).unwrap();
        assert!(serializado.get("regiao").is_none());
        assert!(serializado.get("confianca").is_none());
    }
    assert_eq!(&*fonte.bytes, original);
    assert_eq!(fonte.aberturas.load(Ordering::SeqCst), 3);
}

#[test]
fn pdf_importado_nao_altera_origem_inventario_auditoria_ou_hashes_forenses() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-PDF-SINTETICA");
    comum::extracao(&origem);
    fs::create_dir_all(origem.join("documentos")).unwrap();
    fs::write(
        origem.join("documentos/nativo-sintetico.pdf"),
        duas_paginas(),
    )
    .unwrap();
    let origem_antes = comum::retrato(&origem);
    let (banco, dispositivo) = comum::caso(raiz.path());
    let extracao = comum::importar(&banco, &dispositivo, &origem);
    assert_eq!(extracao.status, "concluida", "{:?}", extracao.erro);
    assert_eq!(comum::retrato(&origem), origem_antes);
    let inventario_antes =
        serde_json::to_value(banco.arquivos(extracao.id, None, None, 0, 500).unwrap()).unwrap();
    let extracao_antes = serde_json::to_value(banco.extracao(extracao.id).unwrap()).unwrap();
    let auditoria_antes = serde_json::to_value(banco.auditoria(500).unwrap()).unwrap();
    let cadeia_antes = banco.verificar_auditoria().unwrap();
    let arquivo = banco
        .arquivos(extracao.id, None, None, 0, 500)
        .unwrap()
        .itens
        .into_iter()
        .find(|arquivo| arquivo.caminho == "documentos/nativo-sintetico.pdf")
        .unwrap();
    let pedido = PedidoDocumento {
        caso_id: banco.caso().id,
        extracao_id: extracao.id,
        arquivo_id: arquivo.id,
        caminho: CaminhoFonte::novo(&arquivo.caminho).unwrap(),
        sha256: arquivo.sha256,
        tamanho: arquivo.tamanho.try_into().unwrap(),
        modo: ModoDocumento::Automatico,
    };
    let fonte = PastaSomenteLeitura::abrir(&origem).unwrap();
    for pagina in [1, 2] {
        let resultado = analisar_pagina(&fonte, &pedido, &ControleDocumento::novo(), Some(pagina));
        assert_eq!(resultado.estado, EstadoDocumento::Pronto, "{resultado:?}");
        let texto = resultado.texto.unwrap();
        assert_eq!(texto.caso_id, banco.caso().id);
        assert_eq!(texto.extracao_id, extracao.id);
        assert_eq!(texto.arquivo_id, arquivo.id);
        assert_eq!(texto.pagina, Some(pagina));
        assert_eq!(texto.total_paginas, Some(2));
        assert_eq!(
            texto.sha256,
            comum::sha256(&origem.join(arquivo.caminho.clone()))
        );
        assert_eq!(texto.sha256_texto, hash(texto.texto.as_bytes()));
    }
    assert_eq!(comum::retrato(&origem), origem_antes);
    assert_eq!(
        serde_json::to_value(banco.arquivos(extracao.id, None, None, 0, 500).unwrap()).unwrap(),
        inventario_antes
    );
    assert_eq!(
        serde_json::to_value(banco.extracao(extracao.id).unwrap()).unwrap(),
        extracao_antes
    );
    assert_eq!(
        serde_json::to_value(banco.auditoria(500).unwrap()).unwrap(),
        auditoria_antes
    );
    let cadeia_depois = banco.verificar_auditoria().unwrap();
    assert_eq!(cadeia_depois.registros, cadeia_antes.registros);
    assert_eq!(cadeia_depois.hash_final, cadeia_antes.hash_final);
    assert!(banco.registros_que_nao_conferem(None).unwrap().is_empty());
}

#[test]
fn pagina_sem_texto_e_vazia_nao_dispara_ocr_nem_inventa_conteudo() {
    let fonte = FonteMemoria::nova(
        "vazio-sintetico.pdf",
        PdfSintetico::novo(vec![vec![]], false).bytes(),
    );
    let resultado = analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo());
    assert!(resultado
        .motivo
        .as_deref()
        .is_some_and(|motivo| motivo.contains("OCR")));
    recusa(resultado, EstadoDocumento::SemSuporte);
}

#[test]
fn hash_e_tamanho_sao_conferidos_antes_de_interpretar_pdf_corrompido() {
    let fonte = FonteMemoria::nova(
        "corrompido-sintetico.pdf",
        b"%PDF-1.4\nINVALIDO SINTETICO".to_vec(),
    );
    let mut pedido = fonte.pedido();
    pedido.sha256 = Some("0".repeat(64));
    recusa(
        analisar(&fonte, &pedido, &ControleDocumento::novo()),
        EstadoDocumento::OrigemDivergente,
    );
    pedido.sha256 = Some(hash(&fonte.bytes));
    pedido.tamanho += 1;
    recusa(
        analisar(&fonte, &pedido, &ControleDocumento::novo()),
        EstadoDocumento::OrigemDivergente,
    );
}

#[test]
fn pdf_invalido_e_nao_pdf_com_hash_conferido_recusam_sem_publicar() {
    for bytes in [
        b"%PDF-1.4\nINVALIDO SINTETICO".to_vec(),
        b"Texto SINTETICO com extensao errada".to_vec(),
        b"%PDF-1.4\n".to_vec(),
        vec![],
    ] {
        let fonte = FonteMemoria::nova("invalido-sintetico.pdf", bytes);
        recusa(
            analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
            EstadoDocumento::Falhou,
        );
    }
}

#[test]
fn pagina_zero_e_fora_do_documento_nao_publicam_texto() {
    let fonte = FonteMemoria::nova("paginas-sinteticas.pdf", duas_paginas());
    for pagina in [0, 3, u32::MAX] {
        recusa(
            analisar_pagina(
                &fonte,
                &fonte.pedido(),
                &ControleDocumento::novo(),
                Some(pagina),
            ),
            EstadoDocumento::Falhou,
        );
    }
}

#[test]
fn cancelamento_e_prazo_zero_recusam_sem_abrir_pdf() {
    let fonte = FonteMemoria::nova("cancelamento-sintetico.pdf", duas_paginas());
    let controle = ControleDocumento::novo();
    controle.cancelar();
    recusa(
        analisar(&fonte, &fonte.pedido(), &controle),
        EstadoDocumento::Cancelado,
    );
    recusa(
        analisar(
            &fonte,
            &fonte.pedido(),
            &ControleDocumento::com_prazo(Duration::ZERO),
        ),
        EstadoDocumento::LimiteExcedido,
    );
    fonte.sem_abertura();
}

#[test]
fn cancelamento_durante_leitura_descarta_todo_texto() {
    let controle = Arc::new(ControleDocumento::novo());
    let mut fonte = FonteMemoria::nova("cancelamento-em-leitura-sintetico.pdf", duas_paginas());
    fonte.cancelar_ao_ler = Some(Arc::clone(&controle));
    recusa(
        analisar(&fonte, &fonte.pedido(), &controle),
        EstadoDocumento::Cancelado,
    );
    assert_eq!(fonte.aberturas.load(Ordering::SeqCst), 1);
}

#[test]
fn ocr_e_pdf_em_conteiner_recusam_sem_abrir() {
    let fonte = FonteMemoria::nova("ocr-sintetico.pdf", duas_paginas());
    let mut pedido = fonte.pedido();
    pedido.modo = ModoDocumento::Ocr;
    recusa(
        analisar(&fonte, &pedido, &ControleDocumento::novo()),
        EstadoDocumento::SemSuporte,
    );
    fonte.sem_abertura();
    let fonte = FonteMemoria::nova("sintetico.zip!/nativo.pdf", duas_paginas());
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::SemSuporte,
    );
    fonte.sem_abertura();
}

#[test]
fn pdf_maior_que_limite_do_inventario_recusa_sem_abrir() {
    let fonte = FonteMemoria::nova("grande-sintetico.pdf", duas_paginas());
    let mut pedido = fonte.pedido();
    pedido.tamanho = TAMANHO_MAXIMO + 1;
    recusa(
        analisar(&fonte, &pedido, &ControleDocumento::novo()),
        EstadoDocumento::LimiteExcedido,
    );
    fonte.sem_abertura();
}

#[test]
fn texto_comprimido_valido_continua_legivel() {
    let texto = b"Conteudo PDF SINTETICO comprimido. ".repeat(32);
    let fonte = FonteMemoria::nova(
        "comprimido-sintetico.pdf",
        PdfSintetico::novo(vec![vec![conteudo(&texto)]], true).bytes(),
    );
    let resultado = analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo());
    assert_eq!(resultado.estado, EstadoDocumento::Pronto, "{resultado:?}");
    let mut esperado = texto;
    esperado.push(b'\n');
    assert_eq!(resultado.texto.unwrap().texto.as_bytes(), esperado);
}

#[test]
fn bomba_de_descompressao_pequena_recusa_sem_texto_parcial() {
    let texto = b"SINTETICO ".repeat(800_000);
    let fonte = FonteMemoria::nova(
        "bomba-de-descompressao-sintetica.pdf",
        PdfSintetico::novo(vec![vec![conteudo(&texto)]], true).bytes(),
    );
    assert!(fonte.bytes.len() < TAMANHO_MAXIMO as usize);
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::LimiteExcedido,
    );
}

#[test]
fn limite_agregado_da_pagina_nao_e_multiplicado_por_stream() {
    let texto = b"SINTETICO ".repeat(3_000);
    let streams = (0..10).map(|_| conteudo(&texto)).collect();
    let fonte = FonteMemoria::nova(
        "multiplos-streams-sinteticos.pdf",
        PdfSintetico::novo(vec![streams], true).bytes(),
    );
    assert!(fonte.bytes.len() < TAMANHO_MAXIMO as usize);
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::LimiteExcedido,
    );
}

#[test]
fn quantidade_de_paginas_e_objetos_tem_limites_reais() {
    let fonte = FonteMemoria::nova(
        "muitas-paginas-sinteticas.pdf",
        PdfSintetico::novo((0..513).map(|_| vec![]).collect(), false).bytes(),
    );
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::LimiteExcedido,
    );
    let mut pdf = PdfSintetico::novo(vec![vec![conteudo(b"SINTETICO")]], false);
    for n in 0..4_097 {
        pdf.documento.add_object(dictionary! { "SINTETICO" => n });
    }
    let fonte = FonteMemoria::nova("muitos-objetos-sinteticos.pdf", pdf.bytes());
    assert!(fonte.bytes.len() < TAMANHO_MAXIMO as usize);
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::LimiteExcedido,
    );
}

#[test]
fn fonte_sem_mapeamento_admitido_recusa_em_vez_de_adivinhar() {
    for alteracao in ["Type0", "ToUnicode", "EncodingDesconhecido"] {
        let mut pdf = PdfSintetico::novo(vec![vec![conteudo(b"SINTETICO")]], false);
        let to_unicode = pdf
            .documento
            .add_object(Stream::new(dictionary! {}, b"SINTETICO".to_vec()));
        let fonte = pdf.documento.get_dictionary_mut(pdf.fonte_id).unwrap();
        match alteracao {
            "Type0" => fonte.set("Subtype", "Type0"),
            "ToUnicode" => fonte.set("ToUnicode", to_unicode),
            _ => fonte.set("Encoding", "CodificacaoSinteticaDesconhecida"),
        }
        let fonte = FonteMemoria::nova("fonte-sem-suporte-sintetica.pdf", pdf.bytes());
        recusa(
            analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
            EstadoDocumento::SemSuporte,
        );
    }
}

#[test]
fn form_xobject_recusa_pagina_inteira_sem_texto_parcial() {
    let mut operacoes = operacoes_texto(b"SINTETICO antes de Form");
    operacoes.push(Operation::new("Do", vec!["FORM_SINTETICO".into()]));
    let mut pdf = PdfSintetico::novo(
        vec![vec![Content {
            operations: operacoes,
        }
        .encode()
        .unwrap()]],
        false,
    );
    let form = pdf.documento.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 10.into(), 10.into()],
        },
        conteudo(b"SINTETICO dentro de Form"),
    ));
    pdf.documento
        .get_dictionary_mut(pdf.recursos_id)
        .unwrap()
        .set("XObject", dictionary! { "FORM_SINTETICO" => form });
    let fonte = FonteMemoria::nova("form-sintetico.pdf", pdf.bytes());
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::SemSuporte,
    );
}

#[test]
fn operador_de_fonte_invalido_nao_publica_prefixo_extraido() {
    let mut operacoes = operacoes_texto(b"Prefixo SINTETICO que nao pode sair");
    operacoes.push(Operation::new("BT", vec![]));
    operacoes.push(Operation::new(
        "Tf",
        vec![Object::string_literal(b"F1"), 12.into()],
    ));
    operacoes.push(Operation::new(
        "Tj",
        vec![Object::string_literal(b"SINTETICO final")],
    ));
    operacoes.push(Operation::new("ET", vec![]));
    let fonte = FonteMemoria::nova(
        "tf-invalido-sintetico.pdf",
        PdfSintetico::novo(
            vec![vec![Content {
                operations: operacoes,
            }
            .encode()
            .unwrap()]],
            false,
        )
        .bytes(),
    );
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::Falhou,
    );
}

#[test]
fn arvore_de_paginas_ciclica_nao_publica_pagina_inventada() {
    let mut pdf = PdfSintetico::novo(vec![vec![conteudo(b"SINTETICO")]], false);
    pdf.documento
        .get_dictionary_mut(pdf.paginas_id)
        .unwrap()
        .set(
            "Kids",
            vec![
                Object::Reference(pdf.paginas_id),
                Object::Reference(pdf.paginas[0]),
            ],
        );
    let fonte = FonteMemoria::nova("arvore-ciclica-sintetica.pdf", pdf.bytes());
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::Falhou,
    );
}

#[test]
fn muitas_fontes_na_pagina_recusam_antes_de_decodificar() {
    let mut pdf = PdfSintetico::novo(vec![vec![conteudo(b"SINTETICO")]], false);
    let mut fontes = Dictionary::new();
    fontes.set("F1", pdf.fonte_id);
    for n in 0..65 {
        let id = pdf.documento.add_object(fonte_base());
        fontes.set(format!("FSINTETICA{n}"), id);
    }
    pdf.documento
        .get_dictionary_mut(pdf.recursos_id)
        .unwrap()
        .set("Font", fontes);
    let fonte = FonteMemoria::nova("muitas-fontes-sinteticas.pdf", pdf.bytes());
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::LimiteExcedido,
    );
}

#[test]
fn count_declarado_divergente_nao_vira_total_de_paginas() {
    let mut pdf = PdfSintetico::novo(vec![vec![conteudo(b"SINTETICO")]], false);
    pdf.documento
        .get_dictionary_mut(pdf.paginas_id)
        .unwrap()
        .set("Count", 200);
    let fonte = FonteMemoria::nova("contagem-divergente-sintetica.pdf", pdf.bytes());
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::Falhou,
    );
}

#[test]
fn filtro_comprimido_corrompido_nao_faz_fallback_para_bytes_crus() {
    let mut pdf = PdfSintetico::novo(vec![vec![]], false);
    let conteudo_id = pdf.documento.add_object(Stream::new(
        dictionary! { "Filter" => "FlateDecode" },
        // Os bytes parecem operadores, mas a declaração exige FlateDecode.
        // O motor não pode tratá-los como conteúdo não comprimido por fallback.
        conteudo(b"SINTETICO que nao pode sair"),
    ));
    pdf.documento
        .get_dictionary_mut(pdf.paginas[0])
        .unwrap()
        .set("Contents", conteudo_id);
    let fonte = FonteMemoria::nova("flate-corrompido-sintetico.pdf", pdf.bytes());
    recusa(
        analisar(&fonte, &fonte.pedido(), &ControleDocumento::novo()),
        EstadoDocumento::Falhou,
    );
}

#[test]
#[ignore = "medição real opcional; executa 20 leituras/hash/cargas de PDF sintético sem SLO inventado"]
fn medicao_20_leituras_reais_de_pdf_sintetico() {
    let fonte = FonteMemoria::nova("medicao-pdf-sintetico.pdf", duas_paginas());
    let pedido = fonte.pedido();
    let mut duracoes = Vec::new();
    for rodada in 0..20 {
        let pagina = (rodada % 2 + 1) as u32;
        let inicio = std::time::Instant::now();
        let resultado = analisar_pagina(&fonte, &pedido, &ControleDocumento::novo(), Some(pagina));
        duracoes.push(inicio.elapsed().as_micros());
        assert_eq!(resultado.estado, EstadoDocumento::Pronto, "{resultado:?}");
        let texto = resultado.texto.unwrap();
        assert_eq!(texto.pagina, Some(pagina));
        assert_eq!(texto.total_paginas, Some(2));
        assert_eq!(texto.sha256_texto, hash(texto.texto.as_bytes()));
    }
    assert_eq!(fonte.aberturas.load(Ordering::SeqCst), 20);
    duracoes.sort_unstable();
    println!(
        "PDF SINTETICO: rodadas=20; origem_bytes={}; p50_us={}; max_us={}; RSS=nao_medido; fonte=memoria; SLO=nao_definido",
        fonte.bytes.len(), duracoes[10], duracoes[19],
    );
}
