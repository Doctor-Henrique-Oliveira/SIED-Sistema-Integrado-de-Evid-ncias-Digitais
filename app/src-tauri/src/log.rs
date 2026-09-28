//! Logs técnicos do aplicativo (diferentes da auditoria do caso).
//!
//! Arquivo diário em `<pasta de dados>/logs/sied.log.AAAA-MM-DD` e saída no console
//! em desenvolvimento. Nível ajustável pela variável `SIED_LOG` (ex.: `SIED_LOG=debug`).
//! Logs nunca contêm conteúdo de evidências, só operações e erros.

use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

pub fn iniciar(pasta_logs: &Path) -> Option<WorkerGuard> {
    let filtro = || EnvFilter::try_from_env("SIED_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let (arquivo, guarda) = match std::fs::create_dir_all(pasta_logs) {
        Ok(()) => {
            let (nb, g) = tracing_appender::non_blocking(tracing_appender::rolling::daily(
                pasta_logs, "sied.log",
            ));
            (
                Some(
                    fmt::layer()
                        .with_ansi(false)
                        .with_writer(nb)
                        .with_filter(filtro()),
                ),
                Some(g),
            )
        }
        Err(e) => {
            eprintln!(
                "não foi possível criar a pasta de logs {}: {e}",
                pasta_logs.display()
            );
            (None, None)
        }
    };
    let _ = tracing_subscriber::registry()
        .with(arquivo)
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(filtro()),
        )
        .try_init();

    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(%info, "falha inesperada (panic)");
        anterior(info);
    }));
    guarda
}
