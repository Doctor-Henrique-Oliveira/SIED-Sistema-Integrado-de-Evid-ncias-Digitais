//! Histórico local de navegadores SQLite.
//!
//! Suporta somente os esquemas públicos identificáveis por conteúdo: Chrome/Chromium/Edge
//! (`urls` + `visits`), Firefox (`moz_places` + `moz_historyvisits`) e Safari/WebKit
//! (`history_items` + `history_visits`). O nome do arquivo é apenas uma pista do importador;
//! o parser exige as tabelas e colunas antes de ler.

use rusqlite::Connection;
use serde_json::json;
use sied_dominio::artefato::{DataHora, FormatoData, NovoArtefato, TipoArtefato};

use super::comum::{inteiro, referencia, texto, Emissor};
use super::sqlite::{abrir_copia, colunas, linha_bruta};
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "historico_navegador";
pub const PAPEL: &str = "historico_navegador";

pub struct ParserHistoricoNavegador;

fn webkit(micros: i64) -> DataHora {
    let ms = micros
        .checked_div(1_000)
        .and_then(|x| x.checked_add(11_644_473_600_000));
    DataHora {
        utc_ms: ms.and_then(DataHora::ms_plausivel),
        original: micros.to_string(),
        formato: FormatoData::Webkit,
    }
}

fn unix_micros(micros: i64) -> DataHora {
    DataHora {
        utc_ms: DataHora::ms_plausivel(micros.checked_div(1_000).unwrap_or_default()),
        original: micros.to_string(),
        formato: FormatoData::Texto,
    }
}

fn cocoa_seconds(seconds: i64) -> DataHora {
    let ms = seconds
        .checked_mul(1_000)
        .and_then(|x| x.checked_add(978_307_200_000));
    DataHora {
        utc_ms: ms.and_then(DataHora::ms_plausivel),
        original: seconds.to_string(),
        formato: FormatoData::Cocoa,
    }
}

fn tem(cols: &[String], names: &[&str]) -> bool {
    names.iter().all(|n| cols.iter().any(|c| c == *n))
}

impl Parser for ParserHistoricoNavegador {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "Histórico de navegador".into(),
            versao: "1.0.0".into(),
        }
    }

    fn papeis(&self) -> &'static [&'static str] {
        &[PAPEL]
    }

    fn processar(
        &self,
        entrada: &EntradaParser,
        saida: &mut dyn SaidaParser,
    ) -> Resultado<ResumoParser> {
        let arq = entrada.arquivo(PAPEL).ok_or_else(|| ErroIngestao::Parser {
            parser: ID.into(),
            mensagem: format!("arquivo com papel '{PAPEL}' não informado"),
        })?;
        let conn = abrir_copia(&arq.copia_verificada)?;
        let urls = colunas(&conn, "urls")?;
        let visits = colunas(&conn, "visits")?;
        let places = colunas(&conn, "moz_places")?;
        let history = colunas(&conn, "moz_historyvisits")?;
        let safari_items = colunas(&conn, "history_items")?;
        let safari_visits = colunas(&conn, "history_visits")?;
        let mut e = Emissor::novo(saida);
        if tem(&urls, &["id", "url"]) && tem(&visits, &["id", "url", "visit_time"]) {
            let sql = "SELECT visits.id AS __id, visits.visit_time AS __time, urls.url AS __url, urls.title AS __title, visits.transition AS __transition FROM visits JOIN urls ON urls.id = visits.url ORDER BY visits.id";
            let cols = ["__id", "__time", "__url", "__title", "__transition"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            let mut stmt = conn.prepare(sql)?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let l = linha_bruta(row, &cols)?;
                emitir_visita(&mut e, &l, &cols, "chrome", webkit)?;
            }
            return Ok(e.resumo);
        }
        if tem(&safari_items, &["id", "url"])
            && tem(&safari_visits, &["id", "history_item", "visit_time"])
        {
            let titulo = if safari_visits.iter().any(|c| c == "title") {
                "history_visits.title"
            } else {
                "NULL"
            };
            let sql = format!(
                "SELECT history_visits.id AS __id, history_visits.visit_time AS __time, history_items.url AS __url, {titulo} AS __title FROM history_visits JOIN history_items ON history_items.id = history_visits.history_item ORDER BY history_visits.id"
            );
            let cols = ["__id", "__time", "__url", "__title"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let l = linha_bruta(row, &cols)?;
                emitir_visita(&mut e, &l, &cols, "safari", cocoa_seconds)?;
            }
            return Ok(e.resumo);
        }
        if tem(&places, &["id", "url"]) && tem(&history, &["id", "place_id", "visit_date"]) {
            let sql = "SELECT moz_historyvisits.id AS __id, moz_historyvisits.visit_date AS __time, moz_places.url AS __url, moz_places.title AS __title, moz_historyvisits.visit_type AS __transition FROM moz_historyvisits JOIN moz_places ON moz_places.id = moz_historyvisits.place_id ORDER BY moz_historyvisits.id";
            let cols = ["__id", "__time", "__url", "__title", "__transition"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            let mut stmt = conn.prepare(sql)?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let l = linha_bruta(row, &cols)?;
                emitir_visita(&mut e, &l, &cols, "firefox", unix_micros)?;
            }
            return Ok(e.resumo);
        }
        Err(ErroIngestao::FormatoNaoReconhecido(format!(
            "{}: não é um histórico Chrome/Chromium/Edge, Firefox ou Safari reconhecido",
            arq.origem
        )))
    }
}

fn emitir_visita(
    e: &mut Emissor<'_>,
    l: &serde_json::Map<String, serde_json::Value>,
    _cols: &[String],
    navegador: &str,
    data: fn(i64) -> DataHora,
) -> Resultado<()> {
    let Some(id) = inteiro(l, "__id") else {
        e.avisar("visita sem id legível".into(), PAPEL, None);
        return Ok(());
    };
    let mut a = NovoArtefato::novo(
        TipoArtefato::EventoNavegador,
        referencia(
            PAPEL,
            if matches!(navegador, "chrome" | "safari") {
                "visits"
            } else {
                "moz_historyvisits"
            },
            "__id",
            l,
        ),
    );
    a.app = Some(navegador.into());
    a.subtipo = Some("visita".into());
    a.id_externo = Some(format!("{navegador}:visit={id}"));
    a.conteudo = texto(l, "__url");
    a.nome_arquivo = texto(l, "__title");
    a.data_hora = inteiro(l, "__time").map(data);
    a.metadados.insert("navegador".into(), json!(navegador));
    if let Some(v) = l.get("__transition").filter(|v| !v.is_null()) {
        a.metadados.insert("transicao".into(), v.clone());
    }
    e.emitir(a)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::parser::AvisoParser;
    use std::path::PathBuf;

    #[derive(Default)]
    struct Saida(Vec<NovoArtefato>);
    impl SaidaParser for Saida {
        fn emitir(&mut self, a: NovoArtefato) -> Resultado<()> {
            self.0.push(a);
            Ok(())
        }
        fn avisar(&mut self, _: AvisoParser) {}
    }

    #[test]
    fn fixture_chrome_sintetica() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("History");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT);\
             CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, transition INTEGER);\
             INSERT INTO urls VALUES(1, 'https://exemplo.test/pagina', 'Página sintética');\
             INSERT INTO visits VALUES(2, 1, 13300000000000000, 1);",
        )
        .unwrap();
        let entrada = EntradaParser {
            arquivos: vec![crate::parser::ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: crate::fonte::CaminhoFonte::novo("Chrome/History").unwrap(),
                sha256: "0".repeat(64),
                copia_verificada: PathBuf::from(db),
            }],
            fuso_horario: "UTC".into(),
        };
        let mut s = Saida::default();
        ParserHistoricoNavegador
            .processar(&entrada, &mut s)
            .unwrap();
        assert_eq!(s.0.len(), 1);
        assert_eq!(s.0[0].tipo, TipoArtefato::EventoNavegador);
        assert_eq!(s.0[0].app.as_deref(), Some("chrome"));
        assert_eq!(
            s.0[0].conteudo.as_deref(),
            Some("https://exemplo.test/pagina")
        );
    }

    #[test]
    fn fixture_safari_sintetica() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("History.db");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE history_items(id INTEGER PRIMARY KEY, url TEXT);\
             CREATE TABLE history_visits(id INTEGER PRIMARY KEY, history_item INTEGER, visit_time INTEGER, title TEXT);\
             INSERT INTO history_items VALUES(1, 'https://safari.test/pagina');\
             INSERT INTO history_visits VALUES(2, 1, 757382400, 'Safari sintético');",
        )
        .unwrap();
        let entrada = EntradaParser {
            arquivos: vec![crate::parser::ArquivoDeTrabalho {
                papel: PAPEL.into(),
                origem: crate::fonte::CaminhoFonte::novo("Safari/History.db").unwrap(),
                sha256: "0".repeat(64),
                copia_verificada: PathBuf::from(db),
            }],
            fuso_horario: "UTC".into(),
        };
        let mut s = Saida::default();
        ParserHistoricoNavegador
            .processar(&entrada, &mut s)
            .unwrap();
        assert_eq!(s.0.len(), 1);
        assert_eq!(s.0[0].app.as_deref(), Some("safari"));
    }
}
