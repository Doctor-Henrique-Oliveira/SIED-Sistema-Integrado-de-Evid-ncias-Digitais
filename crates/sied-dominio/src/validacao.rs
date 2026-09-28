use serde::Serialize;
use std::fmt;
use ts_rs::TS;

/// Erro de validação ligado a um campo do formulário.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ErroCampo {
    pub campo: String,
    pub mensagem: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, thiserror::Error)]
#[ts(export)]
pub struct ErrosValidacao {
    pub erros: Vec<ErroCampo>,
}

impl fmt::Display for ErrosValidacao {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let partes: Vec<String> = self
            .erros
            .iter()
            .map(|e| format!("{}: {}", e.campo, e.mensagem))
            .collect();
        write!(f, "{}", partes.join("; "))
    }
}

/// Acumula erros de vários campos para devolver todos de uma vez à UI.
#[derive(Default)]
pub(crate) struct Validador {
    erros: Vec<ErroCampo>,
}

impl Validador {
    pub fn erro(&mut self, campo: &str, mensagem: impl Into<String>) {
        self.erros.push(ErroCampo {
            campo: campo.into(),
            mensagem: mensagem.into(),
        });
    }

    pub fn obrigatorio(&mut self, campo: &str, valor: &str) {
        if valor.trim().is_empty() {
            self.erro(campo, "campo obrigatório");
        }
    }

    pub fn tamanho_max(&mut self, campo: &str, valor: Option<&str>, max: usize) {
        if let Some(v) = valor {
            if v.chars().count() > max {
                self.erro(campo, format!("máximo de {max} caracteres"));
            }
        }
    }

    pub fn concluir(self) -> Result<(), ErrosValidacao> {
        if self.erros.is_empty() {
            Ok(())
        } else {
            Err(ErrosValidacao { erros: self.erros })
        }
    }
}

/// Remove espaços nas pontas e transforma texto vazio em `None`.
pub fn limpar(valor: Option<String>) -> Option<String> {
    valor
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
