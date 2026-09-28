//! Dispositivo examinado. Um caso pode ter vários dispositivos.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::{CasoId, DispositivoId};
use crate::validacao::{limpar, ErrosValidacao, Validador};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SistemaOperacional {
    Android,
    Ios,
    Outro,
    #[default]
    Desconhecido,
}

impl SistemaOperacional {
    pub fn como_str(self) -> &'static str {
        match self {
            Self::Android => "android",
            Self::Ios => "ios",
            Self::Outro => "outro",
            Self::Desconhecido => "desconhecido",
        }
    }

    pub fn de_str(s: &str) -> Option<Self> {
        Some(match s {
            "android" => Self::Android,
            "ios" => Self::Ios,
            "outro" => Self::Outro,
            "desconhecido" => Self::Desconhecido,
            _ => return None,
        })
    }
}

/// Dados informados pelo analista. Valores detectados automaticamente em extrações
/// ficarão em tabela própria, com a fonte de cada um, e nunca sobrescrevem estes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NovoDispositivo {
    /// Nome de trabalho, ex.: "Celular 1 – investigado".
    pub apelido: String,
    pub fabricante: Option<String>,
    pub modelo: Option<String>,
    pub nome_comercial: Option<String>,
    pub sistema: SistemaOperacional,
    pub versao_sistema: Option<String>,
    pub imei1: Option<String>,
    pub imei2: Option<String>,
    pub serial: Option<String>,
    pub numero: Option<String>,
    pub operadora: Option<String>,
    #[ts(type = "number | null")]
    pub capacidade_bytes: Option<u64>,
    pub observacoes: Option<String>,
}

impl NovoDispositivo {
    pub fn normalizado(self) -> Self {
        NovoDispositivo {
            apelido: self.apelido.trim().to_string(),
            fabricante: limpar(self.fabricante),
            modelo: limpar(self.modelo),
            nome_comercial: limpar(self.nome_comercial),
            sistema: self.sistema,
            versao_sistema: limpar(self.versao_sistema),
            imei1: limpar(self.imei1).map(|i| somente_digitos(&i)),
            imei2: limpar(self.imei2).map(|i| somente_digitos(&i)),
            serial: limpar(self.serial),
            numero: limpar(self.numero),
            operadora: limpar(self.operadora),
            capacidade_bytes: self.capacidade_bytes,
            observacoes: limpar(self.observacoes),
        }
    }

    pub fn validar(&self) -> Result<(), ErrosValidacao> {
        let mut v = Validador::default();
        v.obrigatorio("apelido", &self.apelido);
        v.tamanho_max("apelido", Some(&self.apelido), 120);
        for (campo, imei) in [("imei1", &self.imei1), ("imei2", &self.imei2)] {
            if let Some(i) = imei {
                if let Err(msg) = validar_imei(i) {
                    v.erro(campo, msg);
                }
            }
        }
        if self.imei1.is_some() && self.imei1 == self.imei2 {
            v.erro("imei2", "IMEI 2 igual ao IMEI 1");
        }
        for (campo, valor) in [
            ("fabricante", &self.fabricante),
            ("modelo", &self.modelo),
            ("nome_comercial", &self.nome_comercial),
            ("versao_sistema", &self.versao_sistema),
            ("serial", &self.serial),
            ("numero", &self.numero),
            ("operadora", &self.operadora),
        ] {
            v.tamanho_max(campo, valor.as_deref(), 120);
        }
        v.tamanho_max("observacoes", self.observacoes.as_deref(), 10_000);
        v.concluir()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Dispositivo {
    pub id: DispositivoId,
    pub caso_id: CasoId,
    #[serde(flatten)]
    #[ts(flatten)]
    pub dados: NovoDispositivo,
    #[ts(type = "string")]
    pub criado_em: DateTime<Utc>,
    #[ts(type = "string")]
    pub atualizado_em: DateTime<Utc>,
}

fn somente_digitos(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// IMEI: 15 dígitos com dígito verificador de Luhn.
pub fn validar_imei(imei: &str) -> Result<(), &'static str> {
    if imei.len() != 15 || !imei.bytes().all(|b| b.is_ascii_digit()) {
        return Err("IMEI deve ter 15 dígitos");
    }
    let soma: u32 = imei
        .bytes()
        .rev()
        .enumerate()
        .map(|(i, b)| {
            let d = (b - b'0') as u32;
            if i % 2 == 1 {
                let x = d * 2;
                if x > 9 {
                    x - 9
                } else {
                    x
                }
            } else {
                d
            }
        })
        .sum();
    if soma % 10 == 0 {
        Ok(())
    } else {
        Err("IMEI inválido (dígito verificador não confere)")
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    // IMEI de exemplo com dígito verificador válido (não pertence a um aparelho real conhecido).
    const IMEI_OK: &str = "490154203237518";

    #[test]
    fn imei_luhn() {
        assert!(validar_imei(IMEI_OK).is_ok());
        assert!(validar_imei("490154203237519").is_err());
        assert!(validar_imei("12345").is_err());
    }

    #[test]
    fn normaliza_imei_com_mascara() {
        let d = NovoDispositivo {
            apelido: "Celular 1".into(),
            imei1: Some("49-015420-323751-8".into()),
            ..Default::default()
        }
        .normalizado();
        assert_eq!(d.imei1.as_deref(), Some(IMEI_OK));
        assert!(d.validar().is_ok());
    }

    #[test]
    fn exige_apelido() {
        let d = NovoDispositivo::default().normalizado();
        assert_eq!(d.validar().unwrap_err().erros[0].campo, "apelido");
    }

    #[test]
    fn rejeita_imeis_iguais() {
        let d = NovoDispositivo {
            apelido: "x".into(),
            imei1: Some(IMEI_OK.into()),
            imei2: Some(IMEI_OK.into()),
            ..Default::default()
        };
        assert_eq!(d.validar().unwrap_err().erros[0].campo, "imei2");
    }
}
