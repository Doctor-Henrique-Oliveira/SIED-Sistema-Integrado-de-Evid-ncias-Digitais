// Imagens SINTÉTICAS geradas no próprio teste.

use super::*;
use image::{ImageEncoder, Rgb, RgbImage};
use sha2::{Digest, Sha256};

fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn jpeg(largura: u32, altura: u32) -> Vec<u8> {
    let img = RgbImage::from_fn(largura, altura, |x, y| {
        Rgb([(x % 256) as u8, (y % 256) as u8, 90])
    });
    let mut v = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut v, 90)
        .encode_image(&img)
        .unwrap();
    v
}

fn png(largura: u32, altura: u32) -> Vec<u8> {
    let img = RgbImage::from_fn(largura, altura, |x, _| Rgb([200, (x % 256) as u8, 40]));
    let mut v = Vec::new();
    image::codecs::png::PngEncoder::new(&mut v)
        .write_image(
            img.as_raw(),
            largura,
            altura,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    v
}

struct Cenario {
    _raiz: tempfile::TempDir,
    extracao: PathBuf,
    caso: PathBuf,
}

fn cenario(arquivos: &[(&str, &[u8])]) -> Cenario {
    let raiz = tempfile::tempdir().unwrap();
    let extracao = raiz.path().join("Extracao");
    for (rel, bytes) in arquivos {
        let p = extracao.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }
    fs::create_dir_all(&extracao).unwrap();
    let caso = raiz.path().join("Caso");
    fs::create_dir_all(&caso).unwrap();
    Cenario {
        extracao,
        caso,
        _raiz: raiz,
    }
}

fn pedido(c: &Cenario, caminho: &str, bytes: &[u8], categoria: &str) -> Pedido {
    Pedido {
        pasta_extracao: c.extracao.clone(),
        caminho: caminho.into(),
        sha256: Some(sha(bytes)),
        tamanho: bytes.len() as u64,
        categoria: categoria.into(),
        pasta_caso: c.caso.clone(),
    }
}

/// Conteúdo e datas de modificação de todos os arquivos, para provar que a
/// evidência não mudou.
fn retrato(pasta: &Path) -> Vec<(PathBuf, String, std::time::SystemTime)> {
    let mut v: Vec<_> = walk(pasta)
        .into_iter()
        .map(|p| {
            let m = fs::metadata(&p).unwrap().modified().unwrap();
            (p.clone(), sha(&fs::read(&p).unwrap()), m)
        })
        .collect();
    v.sort();
    v
}

fn walk(p: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for e in fs::read_dir(p).unwrap() {
        let e = e.unwrap().path();
        if e.is_dir() {
            v.extend(walk(&e));
        } else {
            v.push(e);
        }
    }
    v
}

fn decodificar(m: &Miniatura) -> DynamicImage {
    let dados = m.dados.as_deref().expect("miniatura pronta");
    let b64 = dados.strip_prefix("data:image/jpeg;base64,").unwrap();
    let jpeg = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .unwrap();
    image::load_from_memory(&jpeg).unwrap()
}

#[test]
fn gera_miniatura_reduzida_e_guarda_no_cache_do_caso() {
    let foto = jpeg(1600, 900);
    let c = cenario(&[("DCIM/IMG_0001.jpg", &foto)]);
    let antes = retrato(&c.extracao);
    let p = pedido(&c, "DCIM/IMG_0001.jpg", &foto, "imagem");

    let m = obter(&p);
    assert_eq!(m.estado, EstadoMiniatura::Pronta, "{:?}", m.motivo);
    let img = decodificar(&m);
    assert_eq!((img.width(), img.height()), (LADO_MAXIMO, 180));
    let cache = caminho_cache(&c.caso, p.sha256.as_deref().unwrap());
    assert!(cache.starts_with(c.caso.join(PASTA_MINIATURAS)));
    assert!(cache.is_file());
    assert_eq!(retrato(&c.extracao), antes, "a extração não pode mudar");

    // Com a miniatura no cache, a evidência nem é lida de novo.
    fs::remove_dir_all(&c.extracao).unwrap();
    assert_eq!(obter(&p).estado, EstadoMiniatura::Pronta);
}

#[test]
fn png_pequeno_nao_e_ampliado() {
    let img = png(40, 30);
    let c = cenario(&[("a/b.png", &img)]);
    let m = obter(&pedido(&c, "a/b.png", &img, "imagem"));
    assert_eq!(m.estado, EstadoMiniatura::Pronta);
    let d = decodificar(&m);
    assert_eq!((d.width(), d.height()), (40, 30));
}

#[test]
fn recusa_quando_o_arquivo_difere_do_inventario() {
    let original = jpeg(64, 64);
    let alterado = jpeg(65, 64);
    let c = cenario(&[("DCIM/x.jpg", &alterado)]);
    let p = pedido(&c, "DCIM/x.jpg", &original, "imagem");
    let m = obter(&p);
    assert_eq!(m.estado, EstadoMiniatura::OrigemDivergente);
    assert!(m.dados.is_none());
    assert!(m.motivo.unwrap().contains("difere do inventário"));
    assert!(!caminho_cache(&c.caso, p.sha256.as_deref().unwrap()).exists());
}

#[test]
fn origem_indisponivel_e_tipos_sem_suporte() {
    let foto = jpeg(8, 8);
    let c = cenario(&[("f.jpg", &foto)]);

    let mut sem_pasta = pedido(&c, "f.jpg", &foto, "imagem");
    sem_pasta.pasta_extracao = c.extracao.join("nao-existe");
    assert_eq!(
        obter(&sem_pasta).estado,
        EstadoMiniatura::OrigemIndisponivel
    );

    let sem_arquivo = pedido(&c, "outro.jpg", &foto, "imagem");
    assert_eq!(
        obter(&sem_arquivo).estado,
        EstadoMiniatura::OrigemIndisponivel
    );

    let video = pedido(&c, "f.jpg", &foto, "video");
    let m = obter(&video);
    assert_eq!(m.estado, EstadoMiniatura::SemSuporte);
    assert!(m.motivo.unwrap().contains("vídeos"));

    let mut ilegivel = pedido(&c, "f.jpg", &foto, "imagem");
    ilegivel.sha256 = None;
    assert_eq!(obter(&ilegivel).estado, EstadoMiniatura::Falhou);
}

#[test]
fn imagem_corrompida_ou_formato_desconhecido() {
    let truncado = jpeg(32, 32)[..40].to_vec();
    let heic = b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00mif1heic".to_vec();
    let c = cenario(&[("t.jpg", &truncado), ("h.heic", &heic)]);
    let m = obter(&pedido(&c, "t.jpg", &truncado, "imagem"));
    assert_eq!(m.estado, EstadoMiniatura::Falhou, "{:?}", m.motivo);
    let m = obter(&pedido(&c, "h.heic", &heic, "imagem"));
    assert_eq!(m.estado, EstadoMiniatura::SemSuporte, "{:?}", m.motivo);
}

#[test]
fn pedidos_simultaneos_da_mesma_imagem() {
    let foto = jpeg(800, 600);
    let c = cenario(&[("p.jpg", &foto)]);
    let p = pedido(&c, "p.jpg", &foto, "imagem");
    let estados: Vec<_> = std::thread::scope(|s| {
        let h: Vec<_> = (0..8).map(|_| s.spawn(|| obter(&p).estado)).collect();
        h.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert!(estados.iter().all(|e| *e == EstadoMiniatura::Pronta));
    let pasta = caminho_cache(&c.caso, p.sha256.as_deref().unwrap());
    let sobras: Vec<_> = fs::read_dir(pasta.parent().unwrap()).unwrap().collect();
    assert_eq!(sobras.len(), 1, "sem temporários esquecidos");
}

/// Insere um segmento EXIF mínimo (só a tag Orientation) logo após o SOI do JPEG.
fn com_orientacao(jpeg: &[u8], orientacao: u8) -> Vec<u8> {
    let mut app1 = vec![0xFF, 0xE1, 0x00, 0x22];
    app1.extend_from_slice(b"Exif\0\0MM\0\x2A\0\0\0\x08");
    app1.extend_from_slice(&[0x00, 0x01, 0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01]);
    app1.extend_from_slice(&[0x00, orientacao, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let mut v = jpeg[..2].to_vec();
    v.extend_from_slice(&app1);
    v.extend_from_slice(&jpeg[2..]);
    v
}

#[test]
fn orientacao_exif_e_aplicada_depois_de_reduzir() {
    // Paisagem gravada com "girar 90°": a miniatura sai em retrato.
    let foto = com_orientacao(&jpeg(800, 400), 6);
    let c = cenario(&[("DCIM/girada.jpg", &foto)]);
    let m = obter(&pedido(&c, "DCIM/girada.jpg", &foto, "imagem"));
    assert_eq!(m.estado, EstadoMiniatura::Pronta, "{:?}", m.motivo);
    let img = decodificar(&m);
    assert_eq!((img.width(), img.height()), (160, LADO_MAXIMO));
    // Pequena, sem redução: gira do mesmo jeito.
    let pequena = com_orientacao(&jpeg(40, 20), 8);
    let c = cenario(&[("p.jpg", &pequena)]);
    let img = decodificar(&obter(&pedido(&c, "p.jpg", &pequena, "imagem")));
    assert_eq!((img.width(), img.height()), (20, 40));
}

fn crc32(dados: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in dados {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// PNG cujo cabeçalho declara `largura` x `altura` com poucos bytes de dados.
fn png_declarando(largura: u32, altura: u32) -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut pedaco = |tipo: &[u8], dados: &[u8]| {
        v.extend_from_slice(&(dados.len() as u32).to_be_bytes());
        let mut c = tipo.to_vec();
        c.extend_from_slice(dados);
        v.extend_from_slice(&c);
        v.extend_from_slice(&crc32(&c).to_be_bytes());
    };
    let mut ihdr = largura.to_be_bytes().to_vec();
    ihdr.extend_from_slice(&altura.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    pedaco(b"IHDR", &ihdr);
    pedaco(b"IDAT", &[0x78, 0x9C, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01]);
    pedaco(b"IEND", &[]);
    v
}

#[test]
fn imagem_hostil_nao_derruba_nem_esgota_a_memoria() {
    // 60.000 x 60.000 declarados em ~70 bytes (seriam 10 GB decodificados).
    let bomba = png_declarando(60_000, 60_000);
    // Dentro do limite de dimensões, mas sem dados: falha limpa.
    let vazia = png_declarando(4_000, 4_000);
    let c = cenario(&[("bomba.png", &bomba), ("vazia.png", &vazia)]);
    let inicio = std::time::Instant::now();
    for (nome, bytes) in [("bomba.png", &bomba), ("vazia.png", &vazia)] {
        let m = obter(&pedido(&c, nome, bytes, "imagem"));
        assert_eq!(m.estado, EstadoMiniatura::Falhou, "{nome}: {:?}", m.motivo);
        assert!(m.motivo.unwrap().contains("não pôde ser decodificada"));
        let r = imagem_para_relatorio(&pedido(&c, nome, bytes, "imagem"));
        assert_eq!(r.unwrap_err().estado, EstadoMiniatura::Falhou);
    }
    assert!(inicio.elapsed() < std::time::Duration::from_secs(10));
}

/// Pico de memória (VmHWM) ao gerar a miniatura de uma foto enorme girada por EXIF.
/// Medição manual: `cargo test --release -p sied-midia medir_memoria -- --ignored --nocapture`.
#[test]
#[ignore]
fn medir_memoria_foto_enorme() {
    fn pico_kb() -> u64 {
        fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VmHWM:"))
                    .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
            })
            .unwrap_or(0)
    }
    // 16.000 x 12.000 (192 MP, ~576 MB decodificada), cor lisa: o JPEG fica pequeno.
    let (l, a) = (16_000, 12_000);
    let img = RgbImage::from_pixel(l, a, Rgb([120, 130, 140]));
    let mut j = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut j, 70)
        .encode_image(&img)
        .unwrap();
    drop(img);
    let foto = com_orientacao(&j, 6);
    let c = cenario(&[("enorme.jpg", &foto)]);
    let antes = pico_kb();
    let inicio = std::time::Instant::now();
    let m = obter(&pedido(&c, "enorme.jpg", &foto, "imagem"));
    println!(
        "JPEG {} KB; miniatura {:?} em {:?}; pico de memória antes {} MB, depois {} MB",
        foto.len() / 1024,
        m.estado,
        inicio.elapsed(),
        antes / 1024,
        pico_kb() / 1024
    );
}
