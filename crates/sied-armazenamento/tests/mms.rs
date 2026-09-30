//! MMS do Android de ponta a ponta, sobre uma extração **SINTÉTICA** montada no teste.
//!
//! ATENÇÃO: o banco vem de `testes/fixtures/android/mms.sql` (inventado, com nomes de
//! tabela da documentação pública do Android) e os arquivos de `app_parts` são bytes
//! inventados. As pastas não representam nenhuma ferramenta de extração.

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use sied_armazenamento::busca::FiltroBusca;
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

const MMS: &str = include_str!("../../../testes/fixtures/android/mms.sql");
const TELEFONIA: &str = "dados/data/user_de/0/com.android.providers.telephony";

fn caso(raiz: &Path) -> (BancoCaso, Dispositivo) {
    let mut b = BancoCaso::criar(
        &raiz.join("Casos/IP-4"),
        NovoCaso {
            numero_procedimento: Some("IP 4/2026".into()),
            responsavel_analise: "Analista".into(),
            unidade: "DP".into(),
            ..Default::default()
        },
        Sessao {
            usuario: "analista.teste".into(),
            maquina: "ESTACAO-TESTE".into(),
        },
    )
    .unwrap();
    let d = b
        .cadastrar_dispositivo(NovoDispositivo {
            apelido: "Celular sintético".into(),
            sistema: SistemaOperacional::Android,
            ..Default::default()
        })
        .unwrap();
    (b, d)
}

fn extracao(dir: &Path) {
    let bancos = dir.join(TELEFONIA).join("databases");
    fs::create_dir_all(&bancos).unwrap();
    Connection::open(bancos.join("mmssms.db"))
        .unwrap()
        .execute_batch(&format!(
            "CREATE TABLE sms (_id INTEGER PRIMARY KEY, thread_id INTEGER, address TEXT,
                 date INTEGER, type INTEGER, body TEXT);
             INSERT INTO sms VALUES (1, 1, '+5511900000001', 1767225600000, 1, 'SMS sintético');
             {MMS}"
        ))
        .unwrap();
    let partes = dir.join(TELEFONIA).join("app_parts");
    fs::create_dir_all(&partes).unwrap();
    fs::write(
        partes.join("PART_1767232800000_IMG_0002.png"),
        [&b"\x89PNG\r\n\x1a\n"[..], &[0u8; 56]].concat(),
    )
    .unwrap();
    fs::write(
        partes.join("PART_1767240000000_audio_sintetico.amr"),
        b"#!AMR\nSINTETICO",
    )
    .unwrap();
}

fn sha256(p: &Path) -> String {
    hex::encode(Sha256::digest(fs::read(p).unwrap()))
}

fn retrato(dir: &Path) -> BTreeMap<PathBuf, (String, std::time::SystemTime)> {
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

fn importar(banco: &BancoCaso, d: &Dispositivo, origem: &Path) -> Extracao {
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

fn mms<'a>(msgs: &'a [ArtefatoResumo], id: &str) -> &'a ArtefatoResumo {
    msgs.iter()
        .find(|m| m.id_externo.as_deref() == Some(id) && m.subtipo.as_deref() != Some("sms"))
        .unwrap()
}

#[test]
fn mms_entram_na_conversa_dos_sms_com_anexo_ligado_ao_inventario() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA");
    extracao(&origem);
    let antes = retrato(&origem);
    let (banco, d) = caso(raiz.path());

    let x = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    // 1 SMS + 5 MMS; a parte órfã vira aviso.
    assert_eq!(x.total_artefatos, Some(6));
    assert_eq!(x.total_avisos, Some(1));
    assert_eq!(retrato(&origem), antes, "a extração não pode mudar");
    let execucoes: BTreeMap<String, String> = banco
        .execucoes(x.id)
        .unwrap()
        .into_iter()
        .map(|e| (e.parser, e.status))
        .collect();
    assert_eq!(execucoes["android_sms"], "ok");
    assert_eq!(execucoes["android_mms"], "parcial");
    assert!(banco
        .registros_que_nao_conferem(Some(x.id))
        .unwrap()
        .is_empty());

    // SMS e MMS do mesmo thread na mesma conversa.
    let conversas = banco.conversas(Some(x.id), 0, 50).unwrap();
    let fio = conversas
        .itens
        .iter()
        .find(|c| c.id_externo == "1")
        .unwrap();
    assert_eq!(fio.app, "sms");
    let msgs = banco.mensagens_conversa(fio.id, 0, 50).unwrap().itens;
    assert_eq!(msgs.len(), 4, "1 SMS + 3 MMS do thread 1");

    // Foto: ligada ao arquivo de app_parts pelo final do caminho.
    let foto = mms(&msgs, "1");
    assert_eq!(foto.subtipo.as_deref(), Some("imagem"));
    assert_eq!(foto.anexo_vinculo.as_deref(), Some("sufixo_caminho"));
    let det = banco.artefato(foto.id).unwrap();
    let anexo = det.anexo_arquivo.expect("foto ligada ao inventário");
    assert_eq!(
        anexo.caminho,
        format!("{TELEFONIA}/app_parts/PART_1767232800000_IMG_0002.png")
    );
    assert_eq!(
        anexo.sha256.as_deref(),
        Some(sha256(&origem.join(&anexo.caminho)).as_str())
    );
    assert_eq!(anexo.categoria, "imagem");
    // Origens: pdu + 2 endereços + 3 partes, todas com o SHA-256 do mmssms.db.
    assert_eq!(det.referencias.len(), 6);
    assert!(det.referencias[0].principal);
    assert_eq!(det.referencias[0].tabela.as_deref(), Some("pdu"));
    assert!(det
        .referencias
        .iter()
        .any(|r| r.tabela.as_deref() == Some("part")));

    // Áudio: primeira mídia ligada; a imagem da mesma mensagem não existe na extração
    // e fica só listada nas partes.
    let audio = mms(&msgs, "3");
    assert_eq!(audio.subtipo.as_deref(), Some("audio"));
    assert!(audio.anexo_arquivo_id.is_some());

    // Data em segundos convertida; data absurda sem conversão.
    assert_eq!(foto.data_hora_ms, Some(1_767_232_800_000));
    assert_eq!(mms(&msgs, "5").data_hora_ms, None);

    // Grupo enviado: destinatários sem o próprio aparelho.
    let grupo = conversas
        .itens
        .iter()
        .find(|c| c.id_externo == "4")
        .unwrap();
    let g = banco.mensagens_conversa(grupo.id, 0, 50).unwrap().itens;
    assert_eq!(g[0].de_mim, Some(true));
    assert_eq!(
        g[0].destinatarios,
        vec!["+5521900000002", "+5531900000003", "+5541900000015"]
    );

    // O texto do MMS entra na busca.
    let r = banco
        .buscar(
            &FiltroBusca {
                texto: "combinado".into(),
                ..Default::default()
            },
            0,
            10,
        )
        .unwrap();
    assert_eq!(r.artefatos.total, 1);
    assert_eq!(r.artefatos.itens[0].artefato.id, foto.id);
}
