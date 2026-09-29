//! Caso com uma extração **SINTÉTICA** de WhatsApp, para os testes de evidências e
//! do relatório (incluído com `#[path]` pelos testes de mais de um crate).
//!
//! ATENÇÃO: os bancos vêm de `testes/fixtures/whatsapp/*.sql` (inventados) e a imagem
//! de `testes/fixtures/imagens/foto-sintetica.png` (gerada, sem nenhum conteúdo real).
//! As pastas abaixo não representam nenhuma ferramenta de extração.

#![allow(dead_code)]

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use sied_armazenamento::extracoes::{ArtefatoResumo, Extracao};
use sied_armazenamento::{BancoCaso, Sessao};
use sied_dominio::{Dispositivo, NovoCaso, NovoDispositivo, SistemaOperacional};
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::importador::Importador;
use sied_ingestao::importadores::generico::ImportadorGenerico;
use sied_ingestao::pipeline::{self, Controle};
use sied_ingestao::Registro;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const NOVO: &str = include_str!("../fixtures/whatsapp/msgstore_novo.sql");
const WA: &str = include_str!("../fixtures/whatsapp/wa.sql");
pub const FOTO: &[u8] = include_bytes!("../fixtures/imagens/foto-sintetica.png");

pub const ANA: &str = "5511900000011@s.whatsapp.net";
pub const GRUPO: &str = "120363000000000001@g.us";
pub const BANCOS: &str = "dados/data/com.whatsapp/databases";
pub const MIDIA: &str = "armazenamento/Android/media/com.whatsapp/WhatsApp/Media";
pub const CAMINHO_FOTO: &str = "armazenamento/DCIM/Camera/foto-sintetica.png";

pub fn sessao() -> Sessao {
    Sessao {
        usuario: "analista.teste".into(),
        maquina: "ESTACAO-TESTE".into(),
    }
}

fn banco_de(sql: &str, destino: &Path) {
    fs::create_dir_all(destino.parent().unwrap()).unwrap();
    let conn = Connection::open(destino).unwrap();
    conn.execute_batch(sql).unwrap();
}

fn arquivo(dir: &Path, rel: &str, inicio: &[u8], tamanho: usize) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut v = inicio.to_vec();
    v.resize(tamanho.max(inicio.len()), 0);
    fs::write(p, v).unwrap();
}

/// Extração sintética: `msgstore.db` (esquema novo), `wa.db`, a foto da mensagem 3
/// (64 bytes que não formam uma imagem), um documento em duas cópias e uma foto PNG
/// de verdade na pasta da câmera.
pub fn extracao(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SINTETICO-LEIAME.txt"),
        "Extração SINTÉTICA gerada por teste automatizado. Nenhum dado é real.",
    )
    .unwrap();
    banco_de(NOVO, &dir.join(BANCOS).join("msgstore.db"));
    banco_de(WA, &dir.join(BANCOS).join("wa.db"));
    arquivo(
        dir,
        &format!("{MIDIA}/WhatsApp Images/IMG-20260102-WA0001.jpg"),
        &[0xFF, 0xD8, 0xFF, 0xE0],
        64,
    );
    for raiz in [MIDIA, "copia/WhatsApp/Media"] {
        arquivo(
            dir,
            &format!("{raiz}/WhatsApp Documents/relatorio-sintetico.pdf"),
            b"%PDF-1.4\n% SINTETICO\n",
            128,
        );
    }
    arquivo(dir, CAMINHO_FOTO, FOTO, FOTO.len());
}

pub fn caso(raiz: &Path) -> (BancoCaso, Dispositivo) {
    let mut b = BancoCaso::criar(
        &raiz.join("Casos/IP-7"),
        NovoCaso {
            procedimento: Some("Inquérito Policial".into()),
            numero_procedimento: Some("IP 7/2026 (sintético)".into()),
            responsavel_analise: "Analista de teste".into(),
            unidade: "Unidade sintética".into(),
            ..Default::default()
        },
        sessao(),
    )
    .unwrap();
    let d = b
        .cadastrar_dispositivo(NovoDispositivo {
            apelido: "Celular sintético".into(),
            fabricante: Some("Fabricante fictício".into()),
            sistema: SistemaOperacional::Android,
            imei1: Some("000000000000018".into()),
            ..Default::default()
        })
        .unwrap();
    (b, d)
}

pub fn importar(banco: &BancoCaso, d: &Dispositivo, origem: &Path) -> Extracao {
    let importador = ImportadorGenerico;
    let mut imp = banco
        .iniciar_importacao(d.id, &importador.info(), origem)
        .unwrap();
    let fonte = PastaSomenteLeitura::abrir(origem).unwrap();
    let ctx = imp.contexto().clone();
    let r = pipeline::executar(
        &fonte,
        &importador,
        &Registro::padrao(),
        &mut imp,
        &Controle::default(),
        &ctx,
    );
    imp.finalizar(&r).unwrap()
}

/// Caso pronto: pasta temporária, extração sintética importada, banco aberto.
pub fn preparado() -> (tempfile::TempDir, PathBuf, BancoCaso, Extracao) {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    extracao(&origem);
    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    (raiz, origem, banco, x)
}

/// Mensagens da conversa com a Ana, na ordem.
pub fn mensagens_ana(banco: &BancoCaso) -> Vec<ArtefatoResumo> {
    let c = banco
        .conversas(None, 0, 50)
        .unwrap()
        .itens
        .into_iter()
        .find(|c| c.id_externo == ANA)
        .unwrap();
    banco.mensagens_conversa(c.id, 0, 50).unwrap().itens
}

pub fn mensagem<'a>(msgs: &'a [ArtefatoResumo], key_id: &str) -> &'a ArtefatoResumo {
    msgs.iter()
        .find(|m| m.id_externo.as_deref() == Some(key_id))
        .unwrap_or_else(|| panic!("{key_id} ausente"))
}

/// Id no inventário de um caminho da extração.
pub fn id_arquivo(banco: &BancoCaso, x: &Extracao, caminho: &str) -> i64 {
    banco
        .arquivos(x.id, None, None, 0, 500)
        .unwrap()
        .itens
        .into_iter()
        .find(|a| a.caminho == caminho)
        .unwrap_or_else(|| panic!("{caminho} fora do inventário"))
        .id
}

pub fn sha256(p: &Path) -> String {
    hex::encode(Sha256::digest(fs::read(p).unwrap()))
}

/// SHA-256 e data de modificação de cada arquivo (para provar que nada mudou).
pub fn retrato(dir: &Path) -> BTreeMap<PathBuf, (String, std::time::SystemTime)> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let m = e.metadata().unwrap().modified().unwrap();
            (e.path().to_path_buf(), (sha256(e.path()), m))
        })
        .collect()
}
