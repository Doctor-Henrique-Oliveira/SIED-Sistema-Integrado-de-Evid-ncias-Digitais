//! Imagens geradas exclusivamente em memória, todas SINTÉTICAS.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use image::codecs::{jpeg::JpegEncoder, png::PngEncoder};
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
use sha2::{Digest, Sha256};
use sied_documentos::visao::{
    analisar, distancia_dhash, AssinaturaVisual, ResultadoVisual, MAX_LADO, MOTOR_ID, MOTOR_VERSAO,
};
use sied_documentos::{
    ControleDocumento, EstadoDocumento, ModoDocumento, PedidoDocumento, TAMANHO_MAXIMO,
};
use sied_dominio::{CasoId, ExtracaoId};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::fonte::{CaminhoFonte, EntradaFonte, FonteSomenteLeitura, LeitorFonte};
use sied_ingestao::Resultado;
use std::io::{self, Cursor, Read};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

struct FonteSintetica {
    dados: Vec<u8>,
    aberturas: AtomicUsize,
    leituras: Arc<AtomicUsize>,
    cancelar: Option<Arc<ControleDocumento>>,
}

impl FonteSintetica {
    fn nova(dados: Vec<u8>) -> Self {
        Self {
            dados,
            aberturas: AtomicUsize::new(0),
            leituras: Arc::new(AtomicUsize::new(0)),
            cancelar: None,
        }
    }

    fn pedido(&self, nome: &str) -> PedidoDocumento {
        PedidoDocumento {
            caso_id: CasoId::novo(),
            extracao_id: ExtracaoId::novo(),
            arquivo_id: 7,
            caminho: CaminhoFonte::novo(nome).unwrap(),
            sha256: Some(hex::encode(Sha256::digest(&self.dados))),
            tamanho: self.dados.len() as u64,
            modo: ModoDocumento::Automatico,
        }
    }
}

impl FonteSomenteLeitura for FonteSintetica {
    fn descricao(&self) -> String {
        panic!("motor não deve consultar descrição da fonte")
    }
    fn listar(&self) -> Box<dyn Iterator<Item = Resultado<EntradaFonte>> + '_> {
        panic!("motor não deve percorrer a fonte")
    }
    fn entrada(&self, _: &CaminhoFonte) -> Resultado<Option<EntradaFonte>> {
        panic!("motor usa apenas o inventário já resolvido")
    }
    fn abrir(&self, _: &CaminhoFonte) -> Resultado<Box<dyn LeitorFonte>> {
        self.aberturas.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(LeitorSintetico {
            cursor: Cursor::new(self.dados.clone()),
            leituras: self.leituras.clone(),
            cancelar: self.cancelar.clone(),
        }))
    }
}

struct LeitorSintetico {
    cursor: Cursor<Vec<u8>>,
    leituras: Arc<AtomicUsize>,
    cancelar: Option<Arc<ControleDocumento>>,
}

impl Read for LeitorSintetico {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let lidos = self.cursor.read(bytes)?;
        self.leituras.fetch_add(lidos, Ordering::SeqCst);
        if lidos > 0 {
            if let Some(controle) = &self.cancelar {
                controle.cancelar();
            }
        }
        Ok(lidos)
    }
}

fn png(imagem: &RgbImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            imagem.as_raw(),
            imagem.width(),
            imagem.height(),
            ExtendedColorType::Rgb8,
        )
        .unwrap();
    bytes
}

fn jpeg(imagem: &RgbImage, qualidade: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, qualidade)
        .encode(
            imagem.as_raw(),
            imagem.width(),
            imagem.height(),
            ExtendedColorType::Rgb8,
        )
        .unwrap();
    bytes
}

fn gradiente(largura: u32, altura: u32) -> RgbImage {
    RgbImage::from_fn(largura, altura, |x, _| {
        let valor = (255 * (largura - 1 - x) / (largura - 1)) as u8;
        Rgb([valor, valor, valor])
    })
}

fn pronta(fonte: &FonteSintetica, nome: &str) -> AssinaturaVisual {
    let pedido = fonte.pedido(nome);
    let resultado = analisar(fonte, &pedido, &ControleDocumento::novo());
    assert_eq!(resultado.estado, EstadoDocumento::Pronto, "{resultado:?}");
    assert!(resultado.motivo.as_ref().is_some_and(
        |motivo| motivo.contains("não identidade") && motivo.contains("indisponíveis")
    ));
    let assinatura = resultado.assinatura.unwrap();
    assert_eq!(assinatura.caso_id, pedido.caso_id);
    assert_eq!(assinatura.extracao_id, pedido.extracao_id);
    assert_eq!(assinatura.arquivo_id, pedido.arquivo_id);
    assert_eq!(assinatura.caminho, pedido.caminho);
    assert_eq!(Some(assinatura.sha256.as_str()), pedido.sha256.as_deref());
    assert_eq!(assinatura.motor_id, MOTOR_ID);
    assert_eq!(assinatura.motor_versao, MOTOR_VERSAO);
    assert_eq!(assinatura.dhash.len(), 16);
    assert!(assinatura
        .dhash
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    assert!(assinatura.luminancia_media.is_finite());
    assert!((0.0..=255.0).contains(&assinatura.luminancia_media));
    assert_eq!(fonte.aberturas.load(Ordering::SeqCst), 1);
    assinatura
}

fn sem_assinatura(resultado: ResultadoVisual, estado: EstadoDocumento) {
    assert_eq!(resultado.estado, estado, "{resultado:?}");
    assert!(resultado.assinatura.is_none());
    assert!(resultado.motivo.is_some_and(|motivo| !motivo.is_empty()));
}

#[test]
fn png_real_preserva_proveniencia_e_mede_dimensoes_luminancia_dhash() {
    let bytes = png(&gradiente(36, 24));
    let antes = bytes.clone();
    let fonte = FonteSintetica::nova(bytes);
    let assinatura = pronta(&fonte, "IMAGEM-SINTETICA.png");
    assert_eq!((assinatura.largura, assinatura.altura), (36, 24));
    assert_eq!(assinatura.dhash, "ffffffffffffffff");
    assert!((assinatura.luminancia_media - 127.0).abs() < 1.0);
    assert_eq!(fonte.dados, antes);
}

#[test]
fn resize_e_recompressao_sinteticos_medem_semelhanca_nao_identidade() {
    let pequena = FonteSintetica::nova(png(&gradiente(36, 24)));
    let grande = FonteSintetica::nova(png(&gradiente(72, 48)));
    let recomprimida = FonteSintetica::nova(jpeg(&gradiente(72, 48), 80));
    let a = pronta(&pequena, "MENOR-SINTETICA.png");
    let b = pronta(&grande, "MAIOR-SINTETICA.png");
    let c = pronta(&recomprimida, "JPEG-SINTETICO.jpg");
    assert_ne!(a.sha256, b.sha256);
    assert_ne!(b.sha256, c.sha256);
    assert_eq!(distancia_dhash(&a.dhash, &b.dhash), Some(0));
    assert!(distancia_dhash(&b.dhash, &c.dhash).unwrap() <= 8);
}

#[test]
fn imagens_uniformes_distintas_colidem_sem_serem_declaradas_iguais() {
    let preta = FonteSintetica::nova(png(&RgbImage::from_pixel(16, 16, Rgb([0, 0, 0]))));
    let branca = FonteSintetica::nova(png(&RgbImage::from_pixel(16, 16, Rgb([255, 255, 255]))));
    let a = pronta(&preta, "PRETA-SINTETICA.png");
    let b = pronta(&branca, "BRANCA-SINTETICA.png");
    assert_ne!(a.sha256, b.sha256);
    assert_eq!(a.dhash, b.dhash);
    assert_eq!(a.dhash, "0000000000000000");
    assert_eq!(a.luminancia_media, 0.0);
    assert_eq!(b.luminancia_media, 255.0);
}

#[test]
fn sha_ou_tamanho_divergente_impede_decodificacao_mesmo_com_imagem_real() {
    let fonte = FonteSintetica::nova(png(&gradiente(18, 16)));
    let mut pedido = fonte.pedido("SINTETICA.png");
    pedido.sha256 = Some("0".repeat(64));
    sem_assinatura(
        analisar(&fonte, &pedido, &ControleDocumento::novo()),
        EstadoDocumento::OrigemDivergente,
    );
    pedido.sha256 = fonte.pedido("SINTETICA.png").sha256;
    pedido.tamanho += 1;
    sem_assinatura(
        analisar(&fonte, &pedido, &ControleDocumento::novo()),
        EstadoDocumento::OrigemDivergente,
    );
}

#[test]
fn cancelamento_antes_e_durante_leitura_nao_publica_descritor_parcial() {
    let mut fonte = FonteSintetica::nova(png(&gradiente(18, 16)));
    let pedido = fonte.pedido("SINTETICA.png");
    let controle = Arc::new(ControleDocumento::novo());
    controle.cancelar();
    sem_assinatura(
        analisar(&fonte, &pedido, &controle),
        EstadoDocumento::Cancelado,
    );
    assert_eq!(fonte.aberturas.load(Ordering::SeqCst), 0);
    let controle = Arc::new(ControleDocumento::novo());
    fonte.cancelar = Some(controle.clone());
    sem_assinatura(
        analisar(&fonte, &pedido, &controle),
        EstadoDocumento::Cancelado,
    );
    assert_eq!(fonte.aberturas.load(Ordering::SeqCst), 1);
}

#[test]
fn pedidos_nao_admitidos_hash_ausente_limite_e_prazo_nao_abrem_fonte() {
    let fonte = FonteSintetica::nova(Vec::new());
    for nome in [
        "SINTETICA.gif",
        "SINTETICA.webp",
        "SINTETICA.bmp",
        "SINTETICA.heic",
        "backup.zip!/SINTETICA.png",
    ] {
        sem_assinatura(
            analisar(&fonte, &fonte.pedido(nome), &ControleDocumento::novo()),
            EstadoDocumento::SemSuporte,
        );
    }
    let mut pedido = fonte.pedido("SINTETICA.png");
    pedido.sha256 = None;
    sem_assinatura(
        analisar(&fonte, &pedido, &ControleDocumento::novo()),
        EstadoDocumento::HashAusente,
    );
    pedido = fonte.pedido("SINTETICA.png");
    pedido.tamanho = TAMANHO_MAXIMO + 1;
    sem_assinatura(
        analisar(&fonte, &pedido, &ControleDocumento::novo()),
        EstadoDocumento::LimiteExcedido,
    );
    pedido = fonte.pedido("SINTETICA.png");
    sem_assinatura(
        analisar(
            &fonte,
            &pedido,
            &ControleDocumento::com_prazo(Duration::ZERO),
        ),
        EstadoDocumento::LimiteExcedido,
    );
    pedido.modo = ModoDocumento::Ocr;
    sem_assinatura(
        analisar(&fonte, &pedido, &ControleDocumento::novo()),
        EstadoDocumento::SemSuporte,
    );
    assert_eq!(fonte.aberturas.load(Ordering::SeqCst), 0);
}

#[test]
fn origem_excedente_e_lida_somente_ate_um_byte_sentinela() {
    let fonte = FonteSintetica::nova(vec![0; TAMANHO_MAXIMO as usize + 128]);
    let mut pedido = fonte.pedido("SINTETICA.png");
    pedido.tamanho = 1;
    sem_assinatura(
        analisar(&fonte, &pedido, &ControleDocumento::novo()),
        EstadoDocumento::LimiteExcedido,
    );
    assert_eq!(
        fonte.leituras.load(Ordering::SeqCst),
        TAMANHO_MAXIMO as usize + 1
    );
}

#[test]
fn assinatura_incompativel_e_imagem_truncada_nao_sao_mascaradas() {
    let png_bytes = png(&gradiente(18, 16));
    let renomeada = FonteSintetica::nova(png_bytes.clone());
    sem_assinatura(
        analisar(
            &renomeada,
            &renomeada.pedido("RENOMEADA-SINTETICA.jpg"),
            &ControleDocumento::novo(),
        ),
        EstadoDocumento::SemSuporte,
    );
    let sem_iend = FonteSintetica::nova(png_bytes[..png_bytes.len() - 12].to_vec());
    sem_assinatura(
        analisar(
            &sem_iend,
            &sem_iend.pedido("TRUNCADA-SINTETICA.png"),
            &ControleDocumento::novo(),
        ),
        EstadoDocumento::Falhou,
    );
    let jpeg_bytes = jpeg(&gradiente(18, 16), 90);
    let sem_eoi = FonteSintetica::nova(jpeg_bytes[..jpeg_bytes.len() - 2].to_vec());
    sem_assinatura(
        analisar(
            &sem_eoi,
            &sem_eoi.pedido("TRUNCADA-SINTETICA.jpeg"),
            &ControleDocumento::novo(),
        ),
        EstadoDocumento::Falhou,
    );
}

#[test]
fn dimensao_acima_do_limite_falha_antes_de_alocar_raster() {
    let fonte = FonteSintetica::nova(png(&RgbImage::from_pixel(MAX_LADO + 1, 1, Rgb([0, 0, 0]))));
    sem_assinatura(
        analisar(
            &fonte,
            &fonte.pedido("LARGA-SINTETICA.png"),
            &ControleDocumento::novo(),
        ),
        EstadoDocumento::LimiteExcedido,
    );
}

#[test]
fn png_animado_real_nao_e_interpretado_apenas_como_primeiro_frame() {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_animated(2, 0).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[0; 12]).unwrap();
        writer.write_image_data(&[255; 12]).unwrap();
        writer.finish().unwrap();
    }
    let fonte = FonteSintetica::nova(bytes);
    sem_assinatura(
        analisar(
            &fonte,
            &fonte.pedido("ANIMADA-SINTETICA.png"),
            &ControleDocumento::novo(),
        ),
        EstadoDocumento::SemSuporte,
    );
}

#[test]
fn hamming_valida_hash_exato_sem_aceitar_score_ou_inventar_equivalencia() {
    assert_eq!(
        distancia_dhash("0000000000000000", "ffffffffffffffff"),
        Some(64)
    );
    assert_eq!(
        distancia_dhash("0000000000000000", "0000000000000001"),
        Some(1)
    );
    for invalido in [
        "0",
        "000000000000000g",
        "FFFFFFFFFFFFFFFF",
        "00000000000000000",
    ] {
        assert_eq!(distancia_dhash(invalido, "0000000000000000"), None);
    }
}

#[test]
fn analise_de_png_inventariado_preserva_origem_auditoria_e_fatos() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-VISUAL-SINTETICA");
    comum::extracao(&origem);
    std::fs::create_dir_all(origem.join("visual")).unwrap();
    let caminho = origem.join("visual/GRADIENTE-SINTETICO.png");
    std::fs::write(&caminho, png(&gradiente(36, 24))).unwrap();
    let bytes_antes = std::fs::read(&caminho).unwrap();
    let mtime_antes = std::fs::metadata(&caminho).unwrap().modified().unwrap();
    let origem_antes = comum::retrato(&origem);
    let (banco, dispositivo) = comum::caso(raiz.path());
    let extracao = comum::importar(&banco, &dispositivo, &origem);
    assert_eq!(extracao.status, "concluida");
    let inventario_antes =
        serde_json::to_value(banco.arquivos(extracao.id, None, None, 0, 500).unwrap()).unwrap();
    let auditoria_antes = serde_json::to_value(banco.auditoria(500).unwrap()).unwrap();
    let cadeia_antes = banco.verificar_auditoria().unwrap();
    let arquivo = banco
        .arquivos(extracao.id, None, None, 0, 500)
        .unwrap()
        .itens
        .into_iter()
        .find(|arquivo| arquivo.caminho == "visual/GRADIENTE-SINTETICO.png")
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
    let resultado = analisar(&fonte, &pedido, &ControleDocumento::novo());
    assert_eq!(resultado.estado, EstadoDocumento::Pronto, "{resultado:?}");
    let assinatura = resultado.assinatura.unwrap();
    assert_eq!(assinatura.caso_id, banco.caso().id);
    assert_eq!(assinatura.extracao_id, extracao.id);
    assert_eq!(assinatura.arquivo_id, arquivo.id);
    assert_eq!(assinatura.sha256, hex::encode(Sha256::digest(&bytes_antes)));
    assert_eq!(std::fs::read(&caminho).unwrap(), bytes_antes);
    assert_eq!(
        std::fs::metadata(&caminho).unwrap().modified().unwrap(),
        mtime_antes
    );
    assert_eq!(comum::retrato(&origem), origem_antes);
    assert_eq!(
        serde_json::to_value(banco.arquivos(extracao.id, None, None, 0, 500).unwrap()).unwrap(),
        inventario_antes
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
fn png_16bits_e_recusado_sem_conversao_oculta_para_8bits() {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[0, 0, 0, 1, 0x7f, 0xff, 0xff, 0xff])
            .unwrap();
        writer.finish().unwrap();
    }
    let fonte = FonteSintetica::nova(bytes);
    sem_assinatura(
        analisar(
            &fonte,
            &fonte.pedido("16BITS-SINTETICA.png"),
            &ControleDocumento::novo(),
        ),
        EstadoDocumento::SemSuporte,
    );
}

#[test]
fn pixels_totais_do_cabecalho_sao_limitados_antes_de_decodificar_dados() {
    let pequeno = png(&RgbImage::from_pixel(1, 1, Rgb([0, 0, 0])));
    let mut cabecalho = Vec::new();
    {
        // Cabeçalho novo criado pelo encoder oficial, antes do inventário.
        // O payload mínimo é deliberadamente incompatível: o teste demonstra
        // que o limite de pixels vence antes de tentar produzir o raster.
        let mut encoder = png::Encoder::new(&mut cabecalho, 4_001, 4_000);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let _writer = encoder.write_header().unwrap();
    }
    // PNG: assinatura 8 + chunk IHDR completo 25; o decoder continua oficial.
    let mut bytes = cabecalho[..33].to_vec();
    bytes.extend_from_slice(&pequeno[33..]);
    let fonte = FonteSintetica::nova(bytes);
    sem_assinatura(
        analisar(
            &fonte,
            &fonte.pedido("CABECALHO-EXCESSO-SINTETICO.png"),
            &ControleDocumento::novo(),
        ),
        EstadoDocumento::LimiteExcedido,
    );
}

#[test]
fn jpeg_concatenado_nao_e_apresentado_como_o_primeiro_arquivo_completo() {
    let uma = jpeg(&gradiente(18, 16), 90);
    let mut duas = uma.clone();
    duas.extend_from_slice(&uma);
    let fonte = FonteSintetica::nova(duas);
    sem_assinatura(
        analisar(
            &fonte,
            &fonte.pedido("CONCATENADO-SINTETICO.jpg"),
            &ControleDocumento::novo(),
        ),
        EstadoDocumento::Falhou,
    );
}
