//! Backup `adb backup` (`.ab`) de ponta a ponta, sobre um backup **SINTÉTICO**
//! (`testes/rust/backup_sintetico.rs`): o SIED lê de dentro do backup sem extrair nada
//! na pasta da extração.

#[path = "../../../testes/rust/backup_sintetico.rs"]
mod backup_sintetico;
#[path = "../../../testes/rust/caso_sintetico.rs"]
mod caso_sintetico;

use caso_sintetico::{caso, retrato};
use sha2::{Digest, Sha256};
use sied_armazenamento::extracoes::Extracao;
use sied_armazenamento::BancoCaso;
use sied_dominio::Dispositivo;
use sied_ingestao::fonte::pasta::PastaSomenteLeitura;
use sied_ingestao::importador::{Importador, Suporte};
use sied_ingestao::importadores::adb::AdbImporter;
use sied_ingestao::pipeline::{self, Controle};
use sied_ingestao::Registro;

fn importar(banco: &BancoCaso, d: &Dispositivo, origem: &std::path::Path) -> Extracao {
    let importador = AdbImporter;
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

#[test]
fn backup_ab_de_ponta_a_ponta() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Backup-SINTETICO");
    let itens = backup_sintetico::gravar(&origem);
    let antes = retrato(&origem);

    let fonte = PastaSomenteLeitura::abrir(&origem).unwrap();
    let det = Registro::padrao().detectar(&fonte).unwrap();
    assert_eq!(det[0].importador.id, "adb");
    assert_eq!(det[0].suporte, Suporte::Parcial);

    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    assert_eq!(x.tipo_extracao.as_deref(), Some("backup ADB (.ab)"));
    // backup.ab, o leiame e os 8 itens de dentro.
    assert_eq!(x.total_arquivos, Some(10));
    assert_eq!(x.itens_ignorados, Some(0));

    // Cada item de dentro tem o SHA-256 do próprio conteúdo.
    let inventario = banco.arquivos(x.id, None, None, 0, 100).unwrap().itens;
    for (nome, dados) in &itens {
        let c = format!("backup.ab!/{nome}");
        let a = inventario
            .iter()
            .find(|a| a.caminho == c)
            .unwrap_or_else(|| panic!("{c} fora do inventário"));
        assert_eq!(
            a.sha256.as_deref(),
            Some(hex::encode(Sha256::digest(dados)).as_str())
        );
    }

    // Os cinco leitores rodaram sobre as cópias conferidas de dentro do backup.
    let mut parsers: Vec<(String, String)> = banco
        .execucoes(x.id)
        .unwrap()
        .into_iter()
        .map(|e| {
            // O MMS sintético traz um aviso de propósito (parcial); nenhum falha.
            assert!(e.erro.is_none() && e.status != "falhou", "{e:?}");
            (e.parser, e.arquivo_caminho.unwrap_or_default())
        })
        .collect();
    parsers.sort();
    let tel = "backup.ab!/apps/com.android.providers.telephony/db/mmssms.db";
    let cont = "backup.ab!/apps/com.android.providers.contacts/db";
    assert_eq!(
        parsers,
        vec![
            ("android_chamadas".into(), format!("{cont}/calllog.db")),
            ("android_contatos".into(), format!("{cont}/contacts2.db")),
            ("android_mms".into(), tel.into()),
            ("android_sms".into(), tel.into()),
            (
                "whatsapp_android".into(),
                "backup.ab!/apps/com.whatsapp/db/msgstore.db".into()
            ),
        ]
    );
    assert!(x.total_artefatos.unwrap() > 20, "{:?}", x.total_artefatos);

    // A origem de um registro aponta para dentro do backup, e ele confere pelo hash.
    let pag = banco.artefatos(x.id, 0, 500).unwrap();
    let sms = pag
        .itens
        .iter()
        .find(|a| a.conteudo.as_deref() == Some("SMS sintético no backup"))
        .expect("SMS do backup");
    let det = banco.artefato(sms.id).unwrap();
    assert_eq!(det.referencias[0].arquivo_caminho, tel);
    assert!(banco.conferir_registro(sms.id).unwrap().confere());
    assert!(x
        .observacoes
        .iter()
        .any(|o| o.contains("com.whatsapp") && o.contains("API 34")));

    assert_eq!(
        retrato(&origem),
        antes,
        "a pasta da extração não pode mudar"
    );
}
