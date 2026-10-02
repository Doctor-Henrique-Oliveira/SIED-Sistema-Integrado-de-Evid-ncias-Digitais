//! Controle cooperativo de uma única descoberta, sem timers ou acesso à conexão.
//!
//! O prazo começa na criação (inclui espera pelo serviço). Não é um limite rígido
//! de I/O ou de espera por locks. `concluir` é o gate imediatamente anterior à
//! persistência: depois dele o cancelamento não pode invalidar a confirmação.

use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use rusqlite::Connection;

use crate::{ErroArmazenamento, Resultado};

const EXECUTANDO: u8 = 0;
const CANCELADO: u8 = 1;
const FINALIZANDO: u8 = 2;
const LIMITE: u8 = 3;
const PRAZO_PADRAO: Duration = Duration::from_secs(20);

#[derive(Debug)]
pub struct ControleDescoberta {
    estado: AtomicU8,
    prazo: Instant,
}

impl ControleDescoberta {
    pub fn novo() -> Arc<Self> {
        Self::com_prazo(PRAZO_PADRAO)
    }

    fn com_prazo(prazo: Duration) -> Arc<Self> {
        Arc::new(Self {
            estado: AtomicU8::new(EXECUTANDO),
            prazo: Instant::now() + prazo,
        })
    }

    /// Sinaliza sem adquirir o mutex do serviço. Só o primeiro gate vence.
    pub fn cancelar(&self) -> bool {
        self.estado
            .compare_exchange(EXECUTANDO, CANCELADO, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn verificar(&self) -> Resultado<()> {
        loop {
            match self.estado.load(Ordering::Acquire) {
                CANCELADO => return Err(ErroArmazenamento::DescobertaCancelada),
                LIMITE => return Err(ErroArmazenamento::DescobertaLimite),
                FINALIZANDO => {
                    return Err(ErroArmazenamento::Recusado(
                        "o controle da descoberta já foi finalizado; inicie uma nova operação"
                            .into(),
                    ));
                }
                _ => {
                    if Instant::now() < self.prazo {
                        return Ok(());
                    }
                    if self
                        .estado
                        .compare_exchange(EXECUTANDO, LIMITE, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        return Err(ErroArmazenamento::DescobertaLimite);
                    }
                }
            }
        }
    }

    /// Uso único. O chamador chama imediatamente antes de salvar snapshot/página.
    pub fn concluir(&self) -> Resultado<()> {
        self.verificar()?;
        match self.estado.compare_exchange(
            EXECUTANDO,
            FINALIZANDO,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => Ok(()),
            Err(_) => self.verificar(),
        }
    }

    /// Só traduz interrupções produzidas por este controle, nunca outros erros.
    pub(crate) fn traduzir_sqlite(&self, erro: ErroArmazenamento) -> ErroArmazenamento {
        if let ErroArmazenamento::Sqlite(ref sqlite) = erro {
            if sqlite.sqlite_error_code() == Some(rusqlite::ErrorCode::OperationInterrupted) {
                if let Err(motivo) = self.verificar() {
                    return motivo;
                }
            }
        }
        erro
    }
}

/// Instalada somente durante `buscar_com_controle`. A conexão permanece sob a
/// exclusão do serviço; o callback usa apenas o estado/relógio, nunca SQL/locks.
pub(crate) struct GuardaDescoberta<'a> {
    conn: &'a Connection,
    indicador_registrado: bool,
}

impl<'a> GuardaDescoberta<'a> {
    pub(crate) fn instalar(conn: &'a Connection, controle: &Arc<ControleDescoberta>) -> Self {
        let controle = Arc::clone(controle);
        conn.progress_handler(1_000, Some(move || controle.verificar().is_err()));
        Self {
            conn,
            indicador_registrado: false,
        }
    }

    pub(crate) fn indicador_registrado(&mut self) {
        self.indicador_registrado = true;
    }

    pub(crate) fn limpar(&mut self) -> rusqlite::Result<()> {
        self.conn.progress_handler(0, None::<fn() -> bool>);
        if self.indicador_registrado {
            self.conn.remove_function("sied_ai0_indicador", 2)?;
            self.indicador_registrado = false;
        }
        Ok(())
    }
}

impl Drop for GuardaDescoberta<'_> {
    fn drop(&mut self) {
        // Statements da busca já foram destruídos antes deste guard. Também
        // cobre retornos por erro e unwind sem manter o Arc na conexão seguinte.
        let _ = self.limpar();
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use rusqlite::functions::FunctionFlags;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::Barrier;

    const SOMA: &str = "WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x + 1 FROM n WHERE x < 10000) SELECT sum(x) FROM n";

    #[test]
    fn cancelamento_e_finalizacao_sao_terminais() {
        let cancelado = ControleDescoberta::novo();
        assert!(cancelado.cancelar());
        assert!(!cancelado.cancelar());
        assert!(matches!(
            cancelado.verificar(),
            Err(ErroArmazenamento::DescobertaCancelada)
        ));
        assert!(matches!(
            cancelado.concluir(),
            Err(ErroArmazenamento::DescobertaCancelada)
        ));

        let finalizado = ControleDescoberta::novo();
        finalizado.concluir().unwrap();
        assert!(!finalizado.cancelar());
        assert!(matches!(
            finalizado.verificar(),
            Err(ErroArmazenamento::Recusado(_))
        ));
        assert!(matches!(
            finalizado.concluir(),
            Err(ErroArmazenamento::Recusado(_))
        ));
    }

    #[test]
    fn prazo_zero_sem_sleep() {
        let controle = ControleDescoberta::com_prazo(Duration::ZERO);
        assert!(matches!(
            controle.verificar(),
            Err(ErroArmazenamento::DescobertaLimite)
        ));
        assert!(!controle.cancelar());
        assert!(matches!(
            controle.concluir(),
            Err(ErroArmazenamento::DescobertaLimite)
        ));
    }

    #[test]
    fn prazo_zero_interrompe_sql_e_limpa_hook() {
        let conn = Connection::open_in_memory().unwrap();
        let controle = ControleDescoberta::com_prazo(Duration::ZERO);
        let guarda = GuardaDescoberta::instalar(&conn, &controle);
        let erro = conn
            .query_row(SOMA, [], |r| r.get::<_, i64>(0))
            .unwrap_err();
        assert!(matches!(
            controle.traduzir_sqlite(erro.into()),
            ErroArmazenamento::DescobertaLimite
        ));
        drop(guarda);
        assert_eq!(
            conn.query_row(SOMA, [], |r| r.get::<_, i64>(0)).unwrap(),
            50_005_000
        );
    }

    #[test]
    fn corrida_cancelamento_conclusao_tem_um_vencedor() {
        let controle = ControleDescoberta::novo();
        let barreira = Arc::new(Barrier::new(2));
        let outro = Arc::clone(&controle);
        let inicio = Arc::clone(&barreira);
        let tarefa = std::thread::spawn(move || {
            inicio.wait();
            outro.cancelar()
        });
        barreira.wait();
        let conclusao = controle.concluir();
        let cancelou = tarefa.join().unwrap();
        if cancelou {
            assert!(matches!(
                conclusao,
                Err(ErroArmazenamento::DescobertaCancelada)
            ));
        } else {
            assert!(conclusao.is_ok());
        }
    }

    #[test]
    fn interrompe_durante_sql_e_limpa_para_reuso() {
        let conn = Connection::open_in_memory().unwrap();
        let controle = ControleDescoberta::novo();
        let durante_sql = Arc::clone(&controle);
        conn.create_scalar_function(
            "cancelar_no_sql",
            0,
            FunctionFlags::SQLITE_UTF8,
            move |_| {
                durante_sql.cancelar();
                Ok(0)
            },
        )
        .unwrap();
        let guarda = GuardaDescoberta::instalar(&conn, &controle);
        let erro = conn.query_row(
            "WITH RECURSIVE n(x) AS (VALUES(cancelar_no_sql() + 1) UNION ALL SELECT x + 1 FROM n WHERE x < 10000) SELECT sum(x) FROM n",
            [],
            |r| r.get::<_, i64>(0),
        ).unwrap_err();
        assert_eq!(
            erro.sqlite_error_code(),
            Some(rusqlite::ErrorCode::OperationInterrupted)
        );
        assert!(matches!(
            controle.traduzir_sqlite(erro.into()),
            ErroArmazenamento::DescobertaCancelada
        ));
        drop(guarda);
        assert_eq!(
            conn.query_row(SOMA, [], |r| r.get::<_, i64>(0)).unwrap(),
            50_005_000
        );
    }

    #[test]
    fn limpa_hook_e_funcao_no_unwind_sem_reter_controle() {
        let conn = Connection::open_in_memory().unwrap();
        let controle = ControleDescoberta::novo();
        let fraco = Arc::downgrade(&controle);
        let capturado = Arc::clone(&controle);
        let panico = catch_unwind(AssertUnwindSafe(|| {
            let mut guarda = GuardaDescoberta::instalar(&conn, &controle);
            conn.create_scalar_function(
                "sied_ai0_indicador",
                2,
                FunctionFlags::SQLITE_UTF8,
                move |_| {
                    capturado.verificar().map_err(|_| {
                        rusqlite::Error::SqliteFailure(
                            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERRUPT),
                            None,
                        )
                    })?;
                    Ok(false)
                },
            )
            .unwrap();
            guarda.indicador_registrado();
            panic!("unwind de teste");
        }));
        assert!(panico.is_err());
        assert!(controle.cancelar());
        assert_eq!(
            conn.query_row(SOMA, [], |r| r.get::<_, i64>(0)).unwrap(),
            50_005_000
        );
        assert!(conn
            .query_row("SELECT sied_ai0_indicador(0, '')", [], |r| r
                .get::<_, bool>(0))
            .is_err());
        drop(controle);
        assert!(fraco.upgrade().is_none());
    }

    #[test]
    fn cancelamento_nao_mascara_erro_sql_genuino() {
        let controle = ControleDescoberta::novo();
        controle.cancelar();
        let erro = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CORRUPT),
            Some("corrupção de teste".into()),
        );
        assert!(matches!(
            controle.traduzir_sqlite(erro.into()),
            ErroArmazenamento::Sqlite(_)
        ));
    }
}
