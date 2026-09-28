use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErroIngestao {
    #[error("erro de leitura: {0}")]
    Io(#[from] std::io::Error),

    #[error("caminho inválido na fonte: {0}")]
    CaminhoInvalido(String),

    #[error("{componente}: ainda não implementado — {motivo}")]
    NaoImplementado { componente: String, motivo: String },

    #[error("formato não reconhecido: {0}")]
    FormatoNaoReconhecido(String),

    #[error("parser {parser}: {mensagem}")]
    Parser { parser: String, mensagem: String },
}

pub type Resultado<T> = Result<T, ErroIngestao>;
