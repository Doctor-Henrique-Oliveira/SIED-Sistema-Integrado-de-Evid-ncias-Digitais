//! Uma operação documental por aplicativo, cancelamento cooperativo e identidade
//! do caso. A guarda vive na thread de trabalho mesmo se o frontend sair da tela.

use sied_documentos::ControleDocumento;
use sied_dominio::CasoId;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};

pub struct OperacoesDocumentais {
    ativa: Mutex<Option<Arc<Operacao>>>,
}

struct Operacao {
    caso_id: CasoId,
    controle: ControleDocumento,
    finalizando: AtomicBool,
}

pub struct GuardaDocumental<'a> {
    gerente: &'a OperacoesDocumentais,
    operacao: Arc<Operacao>,
}

impl OperacoesDocumentais {
    pub const fn novo() -> Self {
        Self {
            ativa: Mutex::new(None),
        }
    }

    pub fn iniciar(&self, caso_id: CasoId) -> ResultadoApp<GuardaDocumental<'_>> {
        let mut ativa = self.ativa.lock()?;
        if ativa.is_some() {
            return Err(ErroApp::novo(CodigoErro::Conflito,
                "há uma operação documental em curso; aguarde sua conclusão ou solicite o cancelamento"));
        }
        let operacao = Arc::new(Operacao {
            caso_id,
            controle: ControleDocumento::novo(),
            finalizando: AtomicBool::new(false),
        });
        *ativa = Some(operacao.clone());
        Ok(GuardaDocumental {
            gerente: self,
            operacao,
        })
    }

    /// Só cancela uma operação do caso solicitado. Não equivale a confirmação de
    /// término: o leitor deve alcançar um ponto de interrupção e devolver o estado.
    pub fn cancelar(&self, caso_id: CasoId) -> ResultadoApp<bool> {
        let ativa = self.ativa.lock()?;
        if let Some(op) = ativa.as_ref().filter(|op| op.caso_id == caso_id) {
            if op.finalizando.load(Ordering::Acquire) || op.controle.esta_cancelado() {
                return Ok(false);
            }
            op.controle.cancelar();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn cancelar_ativa(&self) {
        if let Ok(ativa) = self.ativa.lock() {
            if let Some(op) = ativa
                .as_ref()
                .filter(|op| !op.finalizando.load(Ordering::Acquire))
            {
                op.controle.cancelar();
            }
        }
    }
}

impl GuardaDocumental<'_> {
    pub fn controle(&self) -> &ControleDocumento {
        &self.operacao.controle
    }

    /// Gate atômico com relação ao pedido de cancelamento. Deve ser chamado sob
    /// a trava do caso, imediatamente antes da transação. Depois dele cancelar
    /// retorna false: a gravação já está finalizando e não promete rollback.
    pub fn finalizar(&self) -> ResultadoApp<()> {
        let ativa = self.gerente.ativa.lock()?;
        if !ativa
            .as_ref()
            .is_some_and(|op| Arc::ptr_eq(op, &self.operacao))
            || self.operacao.controle.esta_cancelado()
            || self.operacao.finalizando.swap(true, Ordering::AcqRel)
        {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "operação documental cancelada ou já finalizada",
            ));
        }
        Ok(())
    }
}

impl Drop for GuardaDocumental<'_> {
    fn drop(&mut self) {
        // Recuperar esta trava não executa código do parser nem publica resultado.
        let mut ativa = self.gerente.ativa.lock().unwrap_or_else(|e| e.into_inner());
        if ativa
            .as_ref()
            .is_some_and(|op| Arc::ptr_eq(op, &self.operacao))
        {
            *ativa = None;
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn limite_global_nao_libera_ate_a_thread_terminar() {
        let gerente = OperacoesDocumentais::novo();
        let caso = CasoId::novo();
        let guarda = gerente.iniciar(caso).unwrap();
        assert!(gerente.iniciar(CasoId::novo()).is_err());
        assert!(!gerente.cancelar(CasoId::novo()).unwrap());
        assert!(gerente.cancelar(caso).unwrap());
        assert!(gerente.iniciar(caso).is_err());
        drop(guarda);
        assert!(!gerente.cancelar(caso).unwrap());
        assert!(gerente.iniciar(caso).is_ok());
    }

    #[test]
    fn panico_da_operacao_libera_vaga() {
        let gerente = OperacoesDocumentais::novo();
        let caso = CasoId::novo();
        let resultado = std::panic::catch_unwind(|| {
            let _guarda = gerente.iniciar(caso).unwrap();
            panic!("falha SINTÉTICA");
        });
        assert!(resultado.is_err());
        assert!(gerente.iniciar(caso).is_ok());
    }

    #[test]
    fn cancelamento_e_finalizacao_tem_gates_mutuamente_exclusivos() {
        let gerente = OperacoesDocumentais::novo();
        let caso = CasoId::novo();
        let guarda = gerente.iniciar(caso).unwrap();
        assert!(gerente.cancelar(caso).unwrap());
        assert!(guarda.finalizar().is_err());
        drop(guarda);
        let guarda = gerente.iniciar(caso).unwrap();
        guarda.finalizar().unwrap();
        assert!(!gerente.cancelar(caso).unwrap());
        gerente.cancelar_ativa();
        assert!(!guarda.controle().esta_cancelado());
        assert!(guarda.finalizar().is_err());
    }
}
