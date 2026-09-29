//! Medição: relatório com o limite de figuras (não roda na bateria normal).
//!
//! Extração **SINTÉTICA** (ver `testes/rust/caso_sintetico.rs`) com 61 imagens PNG de
//! 12 megapixels desenhadas aqui (degradês, sem pessoas nem texto). Rodar em release:
//!
//! ```text
//! cargo test --release -p sied-relatorio --test desempenho -- --ignored --nocapture
//! ```

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use comum::*;
use sied_armazenamento::evidencias::NovaEvidencia;
use sied_relatorio::montagem::MAX_FIGURAS;
use sied_relatorio::PedidoRelatorio;
use std::fs;
use std::time::Instant;

fn pico_de_memoria_mb() -> Option<f64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let linha = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    let kb: f64 = linha.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024.0)
}

#[test]
#[ignore = "medição: rodar em release com --ignored --nocapture"]
fn medir_relatorio_com_limite_de_figuras() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    extracao(&origem);
    let total = MAX_FIGURAS + 1;
    let t = Instant::now();
    for n in 0..total {
        let img = image::RgbImage::from_fn(4000, 3000, |x, y| {
            image::Rgb([(x / 16) as u8, (y / 12) as u8, (n * 4) as u8])
        });
        let p = origem.join(format!("DCIM/Sinteticas/SINT_{n:03}.png"));
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        img.save(&p).unwrap();
    }
    println!(
        "gerar {total} PNG de 12 MP: {:.1} s",
        t.elapsed().as_secs_f64()
    );
    let (mut banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    for n in 0..total {
        let id = id_arquivo(&banco, &x, &format!("DCIM/Sinteticas/SINT_{n:03}.png"));
        banco
            .adicionar_evidencia(NovaEvidencia {
                arquivo_id: Some(id),
                ..Default::default()
            })
            .unwrap();
    }
    let antes = pico_de_memoria_mb();
    let t = Instant::now();
    let r = sied_relatorio::gerar(&mut banco, PedidoRelatorio::default()).unwrap();
    let s = t.elapsed().as_secs_f64();
    let pdf = banco.pasta().join(&r.caminho);
    println!(
        "relatório com {total} imagens: {s:.1} s, PDF {:.1} MB, pico de memória {:?} MB (antes {:?} MB)",
        fs::metadata(&pdf).unwrap().len() as f64 / 1e6,
        pico_de_memoria_mb().map(|m| m.round()),
        antes.map(|m| m.round())
    );
    let evidencias = r.evidencias.as_array().unwrap();
    assert_eq!(evidencias.len(), total);
    assert!(evidencias.iter().all(|e| e["situacao"] == "conferida"));
}
