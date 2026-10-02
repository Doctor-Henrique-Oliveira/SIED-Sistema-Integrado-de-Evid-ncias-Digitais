//! Controle transitório das pesquisas: independente da trava que ocupa o serviço.
//! Não persiste pedidos, resultados ou sinais de cancelamento no caso.

use sied_armazenamento::controle_descoberta::ControleDescoberta;
use std::sync::{Arc, Mutex};

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};

#[derive(Default)]
pub(crate) struct RegistroDescobertas {
    ativa: Mutex<Option<(String, Arc<ControleDescoberta>)>>,
}

pub(crate) struct OperacaoDescoberta {
    registro: Arc<RegistroDescobertas>,
    id: String,
    pub controle: Arc<ControleDescoberta>,
}

impl RegistroDescobertas {
    /// Reserva antes de enfileirar a tarefa bloqueante; no máximo uma pesquisa.
    pub fn iniciar(self: &Arc<Self>, id: String) -> ResultadoApp<OperacaoDescoberta> {
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(ErroApp::novo(
                CodigoErro::Validacao,
                "identificador de operação inválido",
            ));
        }
        let mut ativa = self.ativa.lock()?;
        if ativa.is_some() {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "já existe uma consulta local em execução",
            ));
        }
        let controle = ControleDescoberta::novo();
        *ativa = Some((id.clone(), Arc::clone(&controle)));
        Ok(OperacaoDescoberta {
            registro: Arc::clone(self),
            id,
            controle,
        })
    }

    /// `true` confirma somente o sinal aceito, não o encerramento da tarefa.
    pub fn cancelar(&self, id: &str) -> ResultadoApp<bool> {
        let ativa = self.ativa.lock()?;
        Ok(ativa
            .as_ref()
            .is_some_and(|(atual, controle)| atual == id && controle.cancelar()))
    }

    pub fn cancelar_ativa(&self) {
        if let Ok(ativa) = self.ativa.lock() {
            if let Some((_, controle)) = ativa.as_ref() {
                controle.cancelar();
            }
        }
    }
}

impl Drop for OperacaoDescoberta {
    fn drop(&mut self) {
        if let Ok(mut ativa) = self.registro.ativa.lock() {
            if ativa
                .as_ref()
                .is_some_and(|(id, c)| id == &self.id && Arc::ptr_eq(c, &self.controle))
            {
                *ativa = None;
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cancelar_so_atende_a_operacao_atual_e_libera_reserva_no_drop() {
        let registro = Arc::new(RegistroDescobertas::default());
        let primeira = registro.iniciar("primeira".into()).unwrap();
        assert!(registro.iniciar("segunda".into()).is_err());
        assert!(!registro.cancelar("outra").unwrap());
        assert!(primeira.controle.verificar().is_ok());
        assert!(registro.cancelar("primeira").unwrap());
        assert!(primeira.controle.verificar().is_err());
        drop(primeira);
        let segunda = registro.iniciar("segunda".into()).unwrap();
        assert!(!registro.cancelar("primeira").unwrap());
        assert!(segunda.controle.verificar().is_ok());
        segunda.controle.concluir().unwrap();
        assert!(!registro.cancelar("segunda").unwrap());
        drop(segunda);
        assert!(!registro.cancelar("segunda").unwrap());
    }

    #[test]
    fn sinal_nao_precisa_da_trava_ocupada_pelo_servico() {
        let servico = Mutex::new(());
        let _ocupado = servico.lock().unwrap();
        let registro = Arc::new(RegistroDescobertas::default());
        let operacao = registro.iniciar("sintetica".into()).unwrap();
        registro.cancelar_ativa();
        assert!(operacao.controle.verificar().is_err());
    }

    #[test]
    fn reserva_e_liberada_tambem_no_desempilhamento_de_panic() {
        let registro = Arc::new(RegistroDescobertas::default());
        let r = Arc::clone(&registro);
        let resultado = std::panic::catch_unwind(move || {
            let _operacao = r.iniciar("sintetica".into()).unwrap();
            panic!("falha SINTÉTICA");
        });
        assert!(resultado.is_err());
        assert!(registro.iniciar("nova".into()).is_ok());
    }
}
