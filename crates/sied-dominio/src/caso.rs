//! Caso / procedimento. Toda análise pertence a um caso.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::CasoId;
use crate::validacao::{limpar, ErrosValidacao, Validador};

pub const FUSO_PADRAO: &str = "America/Sao_Paulo";

/// Dados informados pelo analista ao criar um caso.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NovoCaso {
    /// Tipo/descrição do procedimento (ex.: "Inquérito Policial").
    pub procedimento: Option<String>,
    pub numero_procedimento: Option<String>,
    pub numero_bo: Option<String>,
    pub investigado: Option<String>,
    pub vitima: Option<String>,
    pub delegado: Option<String>,
    pub responsavel_analise: String,
    pub unidade: String,
    pub observacoes: Option<String>,
}

impl NovoCaso {
    /// Remove espaços e converte campos vazios em ausentes.
    pub fn normalizado(self) -> Self {
        NovoCaso {
            procedimento: limpar(self.procedimento),
            numero_procedimento: limpar(self.numero_procedimento),
            numero_bo: limpar(self.numero_bo),
            investigado: limpar(self.investigado),
            vitima: limpar(self.vitima),
            delegado: limpar(self.delegado),
            responsavel_analise: self.responsavel_analise.trim().to_string(),
            unidade: self.unidade.trim().to_string(),
            observacoes: limpar(self.observacoes),
        }
    }

    pub fn validar(&self) -> Result<(), ErrosValidacao> {
        let mut v = Validador::default();
        v.obrigatorio("responsavel_analise", &self.responsavel_analise);
        v.obrigatorio("unidade", &self.unidade);
        if self.numero_procedimento.is_none() && self.numero_bo.is_none() {
            v.erro(
                "numero_procedimento",
                "informe o número do procedimento ou do BO",
            );
        }
        for (campo, valor) in [
            ("procedimento", &self.procedimento),
            ("numero_procedimento", &self.numero_procedimento),
            ("numero_bo", &self.numero_bo),
            ("investigado", &self.investigado),
            ("vitima", &self.vitima),
            ("delegado", &self.delegado),
        ] {
            v.tamanho_max(campo, valor.as_deref(), 200);
        }
        v.tamanho_max("responsavel_analise", Some(&self.responsavel_analise), 200);
        v.tamanho_max("unidade", Some(&self.unidade), 200);
        v.tamanho_max("observacoes", self.observacoes.as_deref(), 10_000);
        v.concluir()
    }
}

/// Caso gravado no banco do caso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Caso {
    pub id: CasoId,
    #[serde(flatten)]
    #[ts(flatten)]
    pub dados: NovoCaso,
    /// Fuso usado para exibir datas no caso e no relatório.
    pub fuso_horario: String,
    #[ts(type = "string")]
    pub criado_em: DateTime<Utc>,
    #[ts(type = "string")]
    pub atualizado_em: DateTime<Utc>,
}

impl Caso {
    /// Identificação curta para listas e títulos.
    pub fn identificacao(&self) -> String {
        match (&self.dados.numero_procedimento, &self.dados.numero_bo) {
            (Some(p), _) => p.clone(),
            (None, Some(bo)) => format!("BO {bo}"),
            (None, None) => self.id.to_string(),
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn valido() -> NovoCaso {
        NovoCaso {
            numero_procedimento: Some("IP 123/2026".into()),
            responsavel_analise: "Analista".into(),
            unidade: "DP Central".into(),
            ..Default::default()
        }
    }

    #[test]
    fn aceita_caso_minimo() {
        assert!(valido().normalizado().validar().is_ok());
    }

    #[test]
    fn exige_procedimento_ou_bo() {
        let mut c = valido();
        c.numero_procedimento = Some("   ".into());
        let erros = c.normalizado().validar().unwrap_err();
        assert_eq!(erros.erros[0].campo, "numero_procedimento");
    }

    #[test]
    fn aceita_apenas_bo() {
        let mut c = valido();
        c.numero_procedimento = None;
        c.numero_bo = Some("4567/2026".into());
        assert!(c.normalizado().validar().is_ok());
    }

    #[test]
    fn exige_responsavel_e_unidade() {
        let c = NovoCaso {
            numero_bo: Some("1".into()),
            ..Default::default()
        };
        let campos: Vec<_> = c
            .validar()
            .unwrap_err()
            .erros
            .into_iter()
            .map(|e| e.campo)
            .collect();
        assert_eq!(campos, vec!["responsavel_analise", "unidade"]);
    }
}
