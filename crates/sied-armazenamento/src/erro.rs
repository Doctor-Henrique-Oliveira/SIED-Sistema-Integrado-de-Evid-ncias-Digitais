use sied_dominio::ErrosValidacao;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErroArmazenamento {
    #[error("erro de banco de dados: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("erro de arquivo: {0}")]
    Io(#[from] std::io::Error),

    #[error("dados inválidos: {0}")]
    Validacao(#[from] ErrosValidacao),

    #[error("o arquivo {caminho} não é um banco {esperado} do SIED")]
    TipoBancoIncorreto {
        caminho: PathBuf,
        esperado: &'static str,
    },

    #[error(
        "o banco {tipo} está na versão {encontrada}, mais nova que a suportada por esta versão do SIED ({suportada}); atualize o SIED"
    )]
    VersaoMaisNova {
        tipo: &'static str,
        encontrada: i64,
        suportada: i64,
    },

    #[error("a migração {versao} do banco {tipo} já aplicada difere da embutida no SIED (hash não confere)")]
    MigracaoDivergente { tipo: &'static str, versao: i64 },

    #[error("já existe um caso em {0}")]
    CasoJaExiste(PathBuf),

    #[error("a pasta {0} não está vazia; escolha uma pasta vazia para o novo caso")]
    PastaNaoVazia(PathBuf),

    #[error("nenhum caso do SIED encontrado em {0}")]
    CasoNaoEncontrado(PathBuf),

    #[error("banco do caso inconsistente: {0}")]
    Inconsistente(String),

    #[error("trilha de auditoria inválida no registro {seq}: {motivo}")]
    AuditoriaCorrompida { seq: i64, motivo: String },
}

pub type Resultado<T> = Result<T, ErroArmazenamento>;
