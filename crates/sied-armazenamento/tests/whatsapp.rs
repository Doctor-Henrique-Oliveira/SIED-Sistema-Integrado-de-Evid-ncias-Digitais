//! WhatsApp de ponta a ponta sobre uma extração **SINTÉTICA** montada no teste.
//!
//! ATENÇÃO: os bancos vêm de `testes/fixtures/whatsapp/*.sql` (inventados, com nomes
//! de tabela da referência pública do app) e as pastas abaixo não representam nenhuma
//! ferramenta de extração. Nenhum dado é real.

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use sied_armazenamento::busca::FiltroBusca;
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

const NOVO: &str = include_str!("../../../testes/fixtures/whatsapp/msgstore_novo.sql");
const ANTIGO: &str = include_str!("../../../testes/fixtures/whatsapp/msgstore_antigo.sql");
const WA: &str = include_str!("../../../testes/fixtures/whatsapp/wa.sql");

const ANA: &str = "5511900000011@s.whatsapp.net";
const BRUNO: &str = "5511900000012@s.whatsapp.net";
const CARLA: &str = "5521900000013@s.whatsapp.net";
const GRUPO: &str = "120363000000000001@g.us";

const BANCOS: &str = "dados/data/com.whatsapp/databases";
const MIDIA: &str = "armazenamento/Android/media/com.whatsapp/WhatsApp/Media";

fn sessao() -> Sessao {
    Sessao {
        usuario: "analista.teste".into(),
        maquina: "ESTACAO-TESTE".into(),
    }
}

fn caso(raiz: &Path) -> (BancoCaso, Dispositivo) {
    let mut b = BancoCaso::criar(
        &raiz.join("Casos/IP-2"),
        NovoCaso {
            numero_procedimento: Some("IP 2/2026".into()),
            responsavel_analise: "Analista".into(),
            unidade: "DP".into(),
            ..Default::default()
        },
        sessao(),
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

fn banco_de(sql: &str, destino: &Path) {
    fs::create_dir_all(destino.parent().unwrap()).unwrap();
    let conn = Connection::open(destino).unwrap();
    conn.execute_batch(sql).unwrap();
}

fn arquivo(dir: &Path, rel: &str, inicio: &[u8], tamanho: usize) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut v = inicio.to_vec();
    v.resize(tamanho, 0);
    fs::write(p, v).unwrap();
}

/// Extração sintética com o esquema novo do `msgstore.db`, o `wa.db` e mídias:
/// - a foto da mensagem 3 existe uma vez (liga pelo sufixo do caminho);
/// - o documento da mensagem 11 existe em duas cópias (ambíguo, não liga);
/// - o áudio da mensagem 13 não existe.
fn extracao_whatsapp(dir: &Path) {
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

fn importar(
    banco: &BancoCaso,
    d: &Dispositivo,
    origem: &Path,
) -> sied_armazenamento::extracoes::Extracao {
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

fn mensagem<'a>(
    msgs: &'a [sied_armazenamento::extracoes::ArtefatoResumo],
    key_id: &str,
) -> &'a sied_armazenamento::extracoes::ArtefatoResumo {
    msgs.iter()
        .find(|m| m.id_externo.as_deref() == Some(key_id))
        .unwrap_or_else(|| panic!("{key_id} ausente"))
}

#[test]
fn whatsapp_importado_com_conversas_nomes_e_anexos() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA-WhatsApp");
    extracao_whatsapp(&origem);
    let antes = retrato(&origem);
    let (banco, d) = caso(raiz.path());

    let x = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    // 3 registros de conversa + 13 mensagens + 4 contatos.
    assert_eq!(x.total_artefatos, Some(20));
    assert_eq!(x.total_avisos, Some(0));
    assert_eq!(retrato(&origem), antes, "a extração não pode mudar");

    // Uma execução só, lendo msgstore.db e wa.db juntos.
    let ex = banco.execucoes(x.id).unwrap();
    assert_eq!(ex.len(), 1);
    assert_eq!(ex[0].parser, "whatsapp_android");
    assert_eq!(ex[0].status, "ok");

    // Conversas: título do assunto do grupo ou do contato, tipo e nomes.
    let conversas = banco.conversas(Some(x.id), 0, 50).unwrap();
    assert_eq!(conversas.total, 3);
    let conv = |jid: &str| {
        let c = conversas
            .itens
            .iter()
            .find(|c| c.id_externo == jid)
            .unwrap();
        banco.conversa(c.id).unwrap()
    };
    let ana = conv(ANA);
    assert_eq!(ana.app, "whatsapp");
    assert_eq!(ana.titulo.as_deref(), Some("Ana Sintética"));
    assert_eq!(ana.tipo.as_deref(), Some("individual"));
    assert_eq!(ana.total_mensagens, 8);
    let registro = banco.artefato(ana.registro_id.unwrap()).unwrap();
    assert_eq!(registro.resumo.tipo, "conversa");

    let grupo = conv(GRUPO);
    assert_eq!(grupo.titulo.as_deref(), Some("Grupo sintético de teste"));
    assert_eq!(grupo.tipo.as_deref(), Some("grupo"));
    let nomes: BTreeMap<&str, Option<&str>> = grupo
        .participantes
        .iter()
        .map(|p| (p.identificador.as_str(), p.nome.as_deref()))
        .collect();
    assert_eq!(
        nomes,
        BTreeMap::from([
            (BRUNO, Some("Bruno Sintético")),
            (CARLA, Some("Carla (perfil sintético)")),
        ])
    );
    assert_eq!(conv(BRUNO).titulo.as_deref(), Some("Bruno Sintético"));

    // Mensagens da conversa trazem anexo, localização e resposta.
    let msgs = banco.mensagens_conversa(ana.id, 0, 50).unwrap().itens;
    let foto = mensagem(&msgs, "SINT0000000003");
    assert_eq!(foto.subtipo.as_deref(), Some("imagem"));
    assert_eq!(foto.anexo_vinculo.as_deref(), Some("sufixo_caminho"));
    assert_eq!(foto.tamanho, Some(64));
    let det = banco.artefato(foto.id).unwrap();
    assert_eq!(
        det.anexo_caminho.as_deref(),
        Some("Media/WhatsApp Images/IMG-20260102-WA0001.jpg")
    );
    let anexo = det.anexo_arquivo.expect("foto ligada ao inventário");
    assert_eq!(
        anexo.caminho,
        format!("{MIDIA}/WhatsApp Images/IMG-20260102-WA0001.jpg")
    );
    assert_eq!(
        anexo.sha256.as_deref(),
        Some(sha256(&origem.join(&anexo.caminho)).as_str())
    );
    assert_eq!(anexo.categoria, "imagem");
    assert_eq!(foto.anexo_arquivo_id, Some(anexo.id));

    let local = mensagem(&msgs, "SINT0000000005");
    assert_eq!(
        (local.latitude, local.longitude),
        (Some(-23.55052), Some(-46.633308))
    );
    assert_eq!(
        mensagem(&msgs, "SINT0000000004")
            .resposta_a_id_externo
            .as_deref(),
        Some("SINT0000000003")
    );

    let doc = mensagem(
        &banco.mensagens_conversa(grupo.id, 0, 50).unwrap().itens,
        "SINT0000000011",
    )
    .clone();
    assert_eq!(doc.anexo_vinculo.as_deref(), Some("ambiguo"));
    assert_eq!(doc.anexo_arquivo_id, None);
    assert_eq!(doc.remetente.as_deref(), Some(CARLA));

    let audio = mensagem(
        &banco
            .mensagens_conversa(conv(BRUNO).id, 0, 50)
            .unwrap()
            .itens,
        "SINT0000000013",
    )
    .clone();
    assert_eq!(audio.anexo_vinculo.as_deref(), Some("nao_encontrado"));
    assert_eq!(audio.anexo_arquivo_id, None);

    // Busca por telefone acha o contato (pelo jid) e as mensagens com a Ana.
    let r = banco
        .buscar(
            &FiltroBusca {
                texto: "+55 11 90000-0011".into(),
                ..Default::default()
            },
            0,
            50,
        )
        .unwrap();
    assert_eq!(r.modo, "telefone");
    let tipos: Vec<&str> = r
        .artefatos
        .itens
        .iter()
        .map(|a| a.artefato.tipo.as_str())
        .collect();
    assert!(tipos.contains(&"contato"), "{tipos:?}");
    assert!(tipos.contains(&"mensagem"), "{tipos:?}");
    assert!(r
        .artefatos
        .itens
        .iter()
        .filter(|a| a.artefato.tipo == "mensagem")
        .all(|a| a.conversa_id == Some(ana.id)));

    // Identificadores com nome, filtráveis pelo nome.
    let ids = banco.identificadores(Some("carla"), 0, 50).unwrap();
    assert_eq!(ids.total, 1);
    assert_eq!(ids.itens[0].identificador, CARLA);
    assert_eq!(
        ids.itens[0].nome.as_deref(),
        Some("Carla (perfil sintético)")
    );

    banco.verificar_auditoria().unwrap();
}

/// Os filtros de texto (conversas, localizar na conversa, arquivos e identificadores)
/// não diferenciam maiúsculas nem acentos: antes, `lower()` do SQLite só convertia
/// ASCII e "SINTÉTICA" não achava "Sintética".
#[test]
fn filtros_de_texto_ignoram_maiusculas_e_acentos() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("Extracao-SINTETICA-WhatsApp");
    extracao_whatsapp(&origem);
    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);

    let filtrar = |texto: &str| -> Vec<String> {
        let f = sied_armazenamento::conversas::FiltroConversas {
            extracao_id: Some(x.id),
            texto: Some(texto.into()),
            ..Default::default()
        };
        let mut v: Vec<String> = banco
            .conversas_filtradas(&f, 0, 50)
            .unwrap()
            .itens
            .into_iter()
            .map(|c| c.id_externo)
            .collect();
        v.sort();
        v
    };
    for termo in [
        "ANA SINTÉTICA",
        "ana sintetica",
        "Ana Sintetica",
        "ÁNA SÍNTÉTICA",
    ] {
        assert_eq!(filtrar(termo), vec![ANA.to_string()], "{termo}");
    }
    // "sintético" casa com os nomes de Bruno, Carla e do grupo, com ou sem acento.
    assert_eq!(filtrar("SINTETICO"), filtrar("sintético"));
    assert_eq!(filtrar("SINTETICO").len(), 2, "{:?}", filtrar("SINTETICO"));

    let ana = banco
        .conversas(Some(x.id), 0, 50)
        .unwrap()
        .itens
        .into_iter()
        .find(|c| c.id_externo == ANA)
        .unwrap();
    let achadas = banco
        .localizar_na_conversa(ana.id, "MENSAGEM SINTETICA", 10)
        .unwrap();
    assert_eq!(achadas.total, 1);
    let msgs = banco.mensagens_conversa(ana.id, 0, 50).unwrap().itens;
    assert_eq!(achadas.itens[0].id, mensagem(&msgs, "SINT0000000001").id);

    let arquivos = banco
        .arquivos(x.id, None, Some("whatsapp images/img-20260102"), 0, 50)
        .unwrap();
    assert_eq!(arquivos.total, 1);

    let ids = banco
        .identificadores(Some("CARLA (PERFIL SINTETICO)"), 0, 50)
        .unwrap();
    assert_eq!(ids.total, 1);
    assert_eq!(ids.itens[0].identificador, CARLA);
}

#[test]
fn esquema_antigo_e_caminho_exato_na_raiz_da_extracao() {
    // Extração que já começa na pasta do WhatsApp: o caminho gravado pelo app é o
    // caminho na extração.
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("WhatsApp-SINTETICO");
    banco_de(ANTIGO, &origem.join("Databases/msgstore.db"));
    arquivo(
        &origem,
        "Media/WhatsApp Images/IMG-20260102-WA0009.jpg",
        &[0xFF, 0xD8, 0xFF, 0xE0],
        64,
    );
    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem);
    assert_eq!(x.status, "concluida", "{:?}", x.erro);
    assert_eq!(x.total_artefatos, Some(10)); // 2 conversas + 8 mensagens, sem wa.db

    let conversas = banco.conversas(Some(x.id), 0, 50).unwrap();
    assert_eq!(conversas.total, 2);
    let grupo = conversas
        .itens
        .iter()
        .find(|c| c.id_externo == GRUPO)
        .unwrap();
    assert_eq!(
        grupo.titulo.as_deref(),
        Some("Grupo sintético (esquema antigo)")
    );
    // Sem wa.db, a conversa individual fica sem título e os participantes sem nome.
    let ana = conversas
        .itens
        .iter()
        .find(|c| c.id_externo == ANA)
        .unwrap();
    assert_eq!(ana.titulo, None);
    assert_eq!(ana.tipo.as_deref(), Some("individual"));
    assert!(ana.participantes.iter().all(|p| p.nome.is_none()));

    let msgs = banco.mensagens_conversa(ana.id, 0, 50).unwrap().itens;
    let foto = mensagem(&msgs, "ANTIGO000000003");
    assert_eq!(foto.anexo_vinculo.as_deref(), Some("caminho_exato"));
    assert!(foto.anexo_arquivo_id.is_some());
}

#[test]
fn caso_v3_ganha_vinculo_de_anexo_sem_perder_registros() {
    let raiz = tempfile::tempdir().unwrap();
    let origem = raiz.path().join("ext");
    extracao_whatsapp(&origem);
    let (banco, d) = caso(raiz.path());
    let x = importar(&banco, &d, &origem);
    let conversas_antes = banco.conversas(None, 0, 50).unwrap();
    let pasta = banco.pasta().to_path_buf();
    banco.fechar().unwrap();

    // Volta o banco para a versão 3 (sem a coluna e os índices da migração 4).
    let conn = Connection::open(pasta.join("caso.sied.db")).unwrap();
    conn.execute_batch(
        "DROP INDEX idx_artefato_conversa_ordem;
         CREATE INDEX idx_artefato_conversa ON artefato(app, conversa_id_externo, data_hora_ms);
         DELETE FROM schema_migracao WHERE versao = 6;
         DROP TABLE evidencia; DROP TABLE relatorio; DELETE FROM schema_migracao WHERE versao = 5;
         DROP INDEX idx_arquivo_nome; DROP INDEX idx_artefato_id_externo;
         ALTER TABLE artefato DROP COLUMN anexo_vinculo;
         DELETE FROM schema_migracao WHERE versao = 4; PRAGMA user_version = 3;",
    )
    .unwrap();
    drop(conn);

    let banco = BancoCaso::abrir(&pasta, sessao()).unwrap();
    assert!(pasta.join("caso.sied.db.antes-v4.bak").is_file());
    let depois = banco.conversas(None, 0, 50).unwrap();
    assert_eq!(depois.total, conversas_antes.total);
    for (a, b) in conversas_antes.itens.iter().zip(&depois.itens) {
        assert_eq!(
            (&a.id_externo, &a.titulo, a.total_mensagens),
            (&b.id_externo, &b.titulo, b.total_mensagens)
        );
    }
    // O que foi importado antes da migração não tem o vínculo registrado; o
    // arquivo ligado continua o mesmo.
    let todos = banco.artefatos(x.id, 0, 100).unwrap().itens;
    assert!(todos.iter().all(|a| a.anexo_vinculo.is_none()));
    assert_eq!(
        todos
            .iter()
            .filter(|a| a.anexo_arquivo_id.is_some())
            .count(),
        1
    );
    banco.verificar_auditoria().unwrap();
}
