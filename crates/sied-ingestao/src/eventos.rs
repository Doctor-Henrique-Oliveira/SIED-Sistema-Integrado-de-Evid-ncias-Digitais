//! Eventos da importação para o console técnico da interface.
//!
//! Cada evento corresponde a algo que o pipeline fez de verdade (inventário, hash,
//! cópia conferida, parser iniciado e concluído, aviso, erro). Eles ficam em memória
//! durante a importação, para a tela buscar de forma incremental (pelo número de
//! sequência), e vão também para o `log-importacao.jsonl` da importação, com o campo
//! `mensagem`: assim o console de uma importação passada pode ser reaberto.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use ts_rs::TS;

/// Eventos guardados em memória durante a importação (os mais antigos saem primeiro;
/// o log em disco guarda todos).
pub const MAX_EVENTOS_MEMORIA: usize = 5_000;
/// Eventos devolvidos por consulta (a tela pede o restante na seguinte).
pub const MAX_EVENTOS_POR_CONSULTA: usize = 1_000;
/// Log lido para reabrir o console de uma importação passada.
const MAX_LOG_LIDO: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NivelEvento {
    Info,
    /// Etapa concluída ou conferência que bateu.
    Ok,
    Aviso,
    Erro,
}

impl NivelEvento {
    pub fn codigo(self) -> &'static str {
        match self {
            NivelEvento::Info => "info",
            NivelEvento::Ok => "ok",
            NivelEvento::Aviso => "aviso",
            NivelEvento::Erro => "erro",
        }
    }

    fn de_codigo(s: &str) -> Option<Self> {
        Some(match s {
            "info" => NivelEvento::Info,
            "ok" => NivelEvento::Ok,
            "aviso" => NivelEvento::Aviso,
            "erro" => NivelEvento::Erro,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct EventoImportacao {
    /// Número de sequência (1, 2, 3...): a tela pede "eventos depois do n".
    #[ts(type = "number")]
    pub seq: u64,
    /// Quando aconteceu (ms desde 1970, UTC).
    #[ts(type = "number")]
    pub ts_ms: i64,
    pub nivel: NivelEvento,
    pub mensagem: String,
}

/// Fila de eventos da importação em curso.
#[derive(Debug, Default)]
pub struct RegistroEventos {
    itens: VecDeque<EventoImportacao>,
    ultimo: u64,
}

impl RegistroEventos {
    pub fn adicionar(
        &mut self,
        ts_ms: i64,
        nivel: NivelEvento,
        mensagem: String,
    ) -> EventoImportacao {
        self.ultimo += 1;
        let e = EventoImportacao {
            seq: self.ultimo,
            ts_ms,
            nivel,
            mensagem,
        };
        self.itens.push_back(e.clone());
        while self.itens.len() > MAX_EVENTOS_MEMORIA {
            self.itens.pop_front();
        }
        e
    }

    /// Eventos com sequência maior que `seq`, em ordem, até o limite por consulta.
    pub fn desde(&self, seq: u64) -> Vec<EventoImportacao> {
        self.itens
            .iter()
            .filter(|e| e.seq > seq)
            .take(MAX_EVENTOS_POR_CONSULTA)
            .cloned()
            .collect()
    }
}

/// Número com separador de milhar (12.483).
pub fn numero_br(n: u64) -> String {
    let s = n.to_string();
    let mut r = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            r.push('.');
        }
        r.push(c);
    }
    r
}

/// Tamanho legível em pt-BR (1,5 MB).
pub fn tamanho_br(bytes: u64) -> String {
    const UNIDADES: [&str; 5] = ["bytes", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!(
            "{} {}",
            numero_br(bytes),
            if bytes == 1 { "byte" } else { "bytes" }
        );
    }
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNIDADES.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    format!("{:.1} {}", v, UNIDADES[u]).replace('.', ",")
}

/// Relê o console de uma importação a partir do seu `log-importacao.jsonl`.
///
/// Linhas gravadas com `mensagem` viram eventos como foram mostrados; linhas de logs
/// anteriores ao console ganham uma descrição mínima pelo nome do evento. Linhas
/// ilegíveis são puladas. Devolve no máximo [`MAX_EVENTOS_MEMORIA`] eventos (com uma
/// linha final avisando quando o restante ficou só no arquivo).
pub fn ler_log(caminho: &Path) -> std::io::Result<Vec<EventoImportacao>> {
    let leitor = BufReader::new(File::open(caminho)?.take(MAX_LOG_LIDO));
    let mut reg = RegistroEventos::default();
    let mut restantes = 0u64;
    let mut ultimo_ts = 0;
    for linha in leitor.lines() {
        let Ok(linha) = linha else { break };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&linha) else {
            continue;
        };
        let ts_ms = v["ts"]
            .as_str()
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.with_timezone(&Utc).timestamp_millis())
            .unwrap_or(ultimo_ts);
        ultimo_ts = ts_ms;
        let Some((nivel, mensagem)) = descrever(&v) else {
            continue;
        };
        if reg.itens.len() + 1 >= MAX_EVENTOS_MEMORIA {
            restantes += 1;
            continue;
        }
        reg.adicionar(ts_ms, nivel, mensagem);
    }
    if restantes > 0 {
        reg.adicionar(
            ultimo_ts,
            NivelEvento::Aviso,
            format!(
                "… mais {} evento(s) no arquivo log-importacao.jsonl da importação",
                numero_br(restantes)
            ),
        );
    }
    Ok(reg.itens.into())
}

fn descrever(v: &serde_json::Value) -> Option<(NivelEvento, String)> {
    let nivel_log = v["nivel"].as_str().unwrap_or("info");
    if let Some(m) = v["mensagem"].as_str() {
        let nivel = v["tom"]
            .as_str()
            .and_then(NivelEvento::de_codigo)
            .or_else(|| NivelEvento::de_codigo(nivel_log))
            .unwrap_or(NivelEvento::Info);
        return Some((nivel, m.to_string()));
    }
    // Logs gravados antes do console: descrição mínima, sem inventar detalhe.
    let d = &v["dados"];
    let texto = match v["evento"].as_str()? {
        "inicio" => "Importação iniciada.".to_string(),
        "contagem" => format!(
            "{} arquivo(s) encontrados.",
            numero_br(d["arquivos"].as_u64().unwrap_or(0))
        ),
        "manifesto" => format!(
            "Manifesto SHA-256 gravado: {}",
            d["sha256"].as_str().unwrap_or("?")
        ),
        "parser_inicio" => format!(
            "Parser {} iniciado.",
            d["parser"]["nome"].as_str().unwrap_or("?")
        ),
        "parser_fim" => match d["erro"].as_str() {
            Some(e) => format!("Parser terminou com falha: {e}"),
            None => format!(
                "Parser concluído: {} registro(s).",
                numero_br(d["resumo"]["artefatos"].as_u64().unwrap_or(0))
            ),
        },
        "fim" => match d["erro"].as_str() {
            Some(e) => format!("Importação terminada com erro: {e}"),
            None => "Importação terminada.".to_string(),
        },
        outro if nivel_log != "info" => format!("{outro}: {d}"),
        _ => return None,
    };
    let nivel = NivelEvento::de_codigo(nivel_log).unwrap_or(NivelEvento::Info);
    Some((nivel, texto))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn numeros_e_tamanhos_em_portugues() {
        assert_eq!(numero_br(0), "0");
        assert_eq!(numero_br(999), "999");
        assert_eq!(numero_br(12_483), "12.483");
        assert_eq!(numero_br(1_234_567), "1.234.567");
        assert_eq!(tamanho_br(1), "1 byte");
        assert_eq!(tamanho_br(512), "512 bytes");
        assert_eq!(tamanho_br(1536), "1,5 KB");
        assert_eq!(tamanho_br(5 * 1024 * 1024), "5,0 MB");
    }

    #[test]
    fn fila_entrega_por_sequencia_e_descarta_os_mais_antigos() {
        let mut r = RegistroEventos::default();
        for i in 0..(MAX_EVENTOS_MEMORIA + 10) {
            r.adicionar(i as i64, NivelEvento::Info, format!("e{i}"));
        }
        assert_eq!(r.itens.len(), MAX_EVENTOS_MEMORIA);
        let a = r.desde(0);
        assert_eq!(a.len(), MAX_EVENTOS_POR_CONSULTA);
        assert_eq!(a[0].seq, 11, "os 10 primeiros saíram da memória");
        let ultimos = r.desde((MAX_EVENTOS_MEMORIA + 7) as u64);
        assert_eq!(
            ultimos.iter().map(|e| e.seq).collect::<Vec<_>>(),
            vec![5_008, 5_009, 5_010]
        );
    }

    #[test]
    fn rele_log_novo_e_antigo() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("log.jsonl");
        std::fs::write(
            &p,
            [
                r#"{"ts":"2026-01-01T10:00:00.000Z","nivel":"info","evento":"inicio","dados":{},"mensagem":"Iniciando.","tom":"info"}"#,
                r#"{"ts":"2026-01-01T10:00:01.000Z","nivel":"info","evento":"copia","dados":{},"mensagem":"Cópia conferida.","tom":"ok"}"#,
                "linha quebrada",
                r#"{"ts":"2026-01-01T10:00:02.000Z","nivel":"info","evento":"plano","dados":{}}"#,
                r#"{"ts":"2026-01-01T10:00:03.000Z","nivel":"info","evento":"contagem","dados":{"arquivos":1500}}"#,
                r#"{"ts":"2026-01-01T10:00:04.000Z","nivel":"erro","evento":"leitura","dados":{"caminho":"a"}}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let e = ler_log(&p).unwrap();
        let m: Vec<_> = e.iter().map(|e| (e.nivel, e.mensagem.as_str())).collect();
        assert_eq!(
            m,
            vec![
                (NivelEvento::Info, "Iniciando."),
                (NivelEvento::Ok, "Cópia conferida."),
                (NivelEvento::Info, "1.500 arquivo(s) encontrados."),
                (NivelEvento::Erro, r#"leitura: {"caminho":"a"}"#),
            ]
        );
        assert_eq!(e[0].ts_ms, 1_767_261_600_000);
        assert_eq!(
            e.iter().map(|e| e.seq).collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
    }
}
