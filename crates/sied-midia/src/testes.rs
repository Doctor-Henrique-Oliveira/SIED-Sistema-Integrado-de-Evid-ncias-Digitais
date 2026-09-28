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
