//! Backup `.ab` do Android **SINTÉTICO** para os testes (incluído com `#[path]`).
//!
//! Monta o tar (ustar) e o cabeçalho do `adb backup` como descritos no código público
//! do Android. Os bancos vêm das fixtures sintéticas de `testes/fixtures/`.

#![allow(dead_code)]

use flate2::write::ZlibEncoder;
use flate2::Compression;
use rusqlite::Connection;
use std::io::Write;
use std::path::Path;

const SMS: &str = "CREATE TABLE sms (_id INTEGER PRIMARY KEY, thread_id INTEGER, address TEXT,
    date INTEGER, type INTEGER, body TEXT);
    INSERT INTO sms VALUES (1, 1, '+5511900000001', 1767225600000, 1, 'SMS sintético no backup');";
const MMS: &str = include_str!("../fixtures/android/mms.sql");
const CONTATOS: &str = include_str!("../fixtures/android/contacts2.sql");
const CHAMADAS: &str = include_str!("../fixtures/android/calllog.sql");
const MSGSTORE: &str = include_str!("../fixtures/whatsapp/msgstore_novo.sql");
const WA: &str = include_str!("../fixtures/whatsapp/wa.sql");

fn cabecalho(nome: &str, tamanho: usize) -> [u8; 512] {
    let mut h = [0u8; 512];
    h[..nome.len()].copy_from_slice(nome.as_bytes());
    h[100..107].copy_from_slice(b"0000600");
    h[124..135].copy_from_slice(format!("{tamanho:011o}").as_bytes());
    h[136..147].copy_from_slice(b"15125334400");
    h[156] = b'0';
    h[257..263].copy_from_slice(b"ustar\0");
    h[263..265].copy_from_slice(b"00");
    h[148..156].copy_from_slice(b"        ");
    let soma: u32 = h.iter().map(|&b| u32::from(b)).sum();
    h[148..155].copy_from_slice(format!("{soma:06o}\0").as_bytes());
    h
}

pub fn tar(itens: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut t = Vec::new();
    for (nome, dados) in itens {
        t.extend_from_slice(&cabecalho(nome, dados.len()));
        t.extend_from_slice(dados);
        t.resize(t.len().div_ceil(512) * 512, 0);
    }
    t.extend_from_slice(&[0u8; 1024]);
    t
}

pub fn ab(tar: &[u8]) -> Vec<u8> {
    let mut v = b"ANDROID BACKUP\n5\n1\nnone\n".to_vec();
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(tar).unwrap();
    v.extend(z.finish().unwrap());
    v
}

fn banco(sql: &str) -> Vec<u8> {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("b.db");
    Connection::open(&p).unwrap().execute_batch(sql).unwrap();
    std::fs::read(p).unwrap()
}

fn manifesto(pacote: &str) -> Vec<u8> {
    format!("1\n{pacote}\n34\n34\n\n0\n0\n").into_bytes()
}

/// Itens do backup: bancos do Android e do WhatsApp em `apps/<pacote>/db/`.
pub fn itens() -> Vec<(String, Vec<u8>)> {
    let tel = "apps/com.android.providers.telephony";
    let cont = "apps/com.android.providers.contacts";
    let wa = "apps/com.whatsapp";
    vec![
        (
            format!("{tel}/_manifest"),
            manifesto("com.android.providers.telephony"),
        ),
        (
            format!("{tel}/db/mmssms.db"),
            banco(&format!("{SMS}\n{MMS}")),
        ),
        (
            format!("{cont}/_manifest"),
            manifesto("com.android.providers.contacts"),
        ),
        (format!("{cont}/db/contacts2.db"), banco(CONTATOS)),
        (format!("{cont}/db/calllog.db"), banco(CHAMADAS)),
        (format!("{wa}/_manifest"), manifesto("com.whatsapp")),
        (format!("{wa}/db/msgstore.db"), banco(MSGSTORE)),
        (format!("{wa}/db/wa.db"), banco(WA)),
    ]
}

/// Grava `backup.ab` na pasta e devolve os itens gravados dentro dele.
pub fn gravar(dir: &Path) -> Vec<(String, Vec<u8>)> {
    std::fs::create_dir_all(dir).unwrap();
    let i = itens();
    std::fs::write(dir.join("backup.ab"), ab(&tar(&i))).unwrap();
    std::fs::write(
        dir.join("LEIAME-SINTETICO.txt"),
        "Backup SINTÉTICO para testes do SIED. Nenhum dado é real.\n",
    )
    .unwrap();
    i
}
