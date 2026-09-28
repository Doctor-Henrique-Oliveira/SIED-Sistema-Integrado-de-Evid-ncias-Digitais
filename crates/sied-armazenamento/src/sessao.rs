use serde::{Deserialize, Serialize};

/// Quem está operando o SIED. Gravado em cada registro de auditoria.
///
/// Nesta etapa vem do usuário do sistema operacional; o login local do SIED
/// substituirá a origem do `usuario` sem mudar o restante.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sessao {
    pub usuario: String,
    pub maquina: String,
}

impl Sessao {
    pub fn do_sistema() -> Self {
        let usuario = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "desconhecido".into());
        let maquina = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
            .unwrap_or_else(|_| "desconhecida".into());
        Sessao { usuario, maquina }
    }
}
