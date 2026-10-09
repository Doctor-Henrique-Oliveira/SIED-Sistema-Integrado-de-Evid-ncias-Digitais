//! Benchmark sintético do núcleo de hashing.
//!
//! Não abre casos reais nem mede parser. O tamanho pode ser ajustado com
//! `SIED_BENCH_BYTES`; o limite evita alocação acidentalmente exagerada.

use sied_ingestao::inventario::ler_com_hash;
use std::io::Cursor;
use std::time::Instant;

fn main() {
    let bytes = std::env::var("SIED_BENCH_BYTES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(64 * 1024 * 1024)
        .clamp(1, 512 * 1024 * 1024);
    let dados = vec![0x5Au8; bytes];
    let inicio = Instant::now();
    let leitura = ler_com_hash(&mut Cursor::new(&dados), None, |_| true)
        .expect("fixture sintética deve ser legível");
    let decorrido = inicio.elapsed();
    let mib = bytes as f64 / (1024.0 * 1024.0);
    let taxa = mib / decorrido.as_secs_f64().max(f64::EPSILON);
    println!(
        "inventario_sintetico bytes={} sha256={} duracao_ms={} taxa_mib_s={:.2}",
        bytes,
        leitura.sha256,
        decorrido.as_millis(),
        taxa
    );
}
