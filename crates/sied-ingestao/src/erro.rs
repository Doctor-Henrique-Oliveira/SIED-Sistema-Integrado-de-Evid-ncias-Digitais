use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErroIngestao {
    #[error("erro de leitura: {0}")]
    Io(#[from] std::io::Error),

    #[error("caminho inválido na fonte: {0}")]
    CaminhoInvalido(String),

    #[error("{componente}: ainda não implementado — {motivo}")]
    NaoImplementado { componente: String, motivo: String },

    #[error("erro ao ler banco SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("formato não reconhecido: {0}")]
    FormatoNaoReconhecido(String),

    #[error("parser {parser}: {mensagem}")]
    Parser { parser: String, mensagem: String },

    #[error("{caminho}: {motivo}")]
    ItemIgnorado { caminho: String, motivo: String },

    #[error("o arquivo {caminho} da extração mudou desde o inventário (SHA-256 esperado {esperado}, lido {lido})")]
    OrigemAlterada {
        caminho: String,
        esperado: String,
        lido: String,
    },

    #[error("importação cancelada pelo usuário")]
    Cancelado,

    #[error("falha ao gravar no caso: {0}")]
    Gravacao(String),

    /// Defeito do próprio SIED (ex.: pânico capturado), registrado em vez de derrubar o programa.
    #[error("falha inesperada: {0}")]
    Interno(String),
}

pub type Resultado<T> = Result<T, ErroIngestao>;
