//! Dados do relatório como o modelo Typst os lê (`/dados.json`).
//!
//! Tudo chega pronto para exibir: datas já no fuso do caso, rótulos em português e
//! caminhos e hashes com pontos de quebra. O modelo só diagrama; nada do que vem
//! daqui é interpretado como marcação.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Seções que o analista escolhe incluir.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SecoesRelatorio {
    pub capa: bool,
    pub procedimento: bool,
    pub dispositivos: bool,
    pub aquisicao: bool,
    pub metodologia: bool,
    pub evidencias: bool,
    pub observacoes: bool,
    pub conclusao: bool,
    pub apendice_tecnico: bool,
    pub integridade: bool,
}

impl Default for SecoesRelatorio {
    fn default() -> Self {
        SecoesRelatorio {
            capa: true,
            procedimento: true,
            dispositivos: true,
            aquisicao: true,
            metodologia: true,
            evidencias: true,
            observacoes: true,
            conclusao: true,
            apendice_tecnico: true,
            integridade: true,
        }
    }
}

/// Pedido de geração vindo da tela.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct PedidoRelatorio {
    #[ts(optional)]
    pub secoes: Option<SecoesRelatorio>,
    /// Título na capa (padrão: "Relatório de análise de evidências digitais").
    #[ts(optional)]
    pub titulo: Option<String>,
    #[ts(optional)]
    pub observacoes: Option<String>,
    #[ts(optional)]
    pub conclusao: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Campo {
    pub rotulo: String,
    pub valor: String,
}

impl Campo {
    pub fn novo(rotulo: &str, valor: impl Into<String>) -> Self {
        Campo {
            rotulo: rotulo.into(),
            valor: valor.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Documento {
    pub titulo: String,
    pub identificacao: String,
    pub gerado_em: String,
    pub fuso: String,
    pub sied_versao: String,
    pub analista: String,
    pub estacao: String,
    pub secoes: SecoesRelatorio,
    /// Campos da capa (parte da identificação do procedimento).
    pub capa: Vec<Campo>,
    pub procedimento: Vec<Campo>,
    pub dispositivos: Vec<DispositivoDoc>,
    pub extracoes: Vec<ExtracaoDoc>,
    pub metodologia: Vec<String>,
    pub transcricoes: Vec<TranscricaoDoc>,
    pub derivados: DerivadosDoc,
    pub evidencias: Vec<EvidenciaDoc>,
    pub observacoes: Option<String>,
    pub conclusao: Option<String>,
    pub integridade: IntegridadeDoc,
}

/// Índices e correlações já produzidos pelo caso, apresentados como material
/// derivado. Nada aqui substitui a evidência original ou afirma causalidade.
#[derive(Debug, Clone, Serialize)]
pub struct DerivadosDoc {
    pub documentos: Vec<DocumentoDerivadoDoc>,
    pub linha_tempo: LinhaTempoDoc,
    pub relacoes: Vec<RelacaoDoc>,
    pub limites: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentoDerivadoDoc {
    pub id: i64,
    pub caminho: String,
    pub pagina: Option<u32>,
    pub total_paginas: Option<u32>,
    pub motor: String,
    pub entidades_motor: String,
    pub regioes: i64,
    pub entidades: i64,
    pub sha256_origem: String,
    pub sha256_texto: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinhaTempoDoc {
    pub total: i64,
    pub primeira: Option<String>,
    pub ultima: Option<String>,
    pub por_tipo: Vec<Campo>,
    pub apps: Vec<Campo>,
    pub sem_data: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelacaoDoc {
    pub de: String,
    pub para: String,
    pub classe: String,
    pub motivo: String,
    pub mecanismo: String,
    pub score: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DispositivoDoc {
    pub nome: String,
    pub campos: Vec<Campo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtracaoDoc {
    pub nome: String,
    pub campos: Vec<Campo>,
}

/// Resultado derivado de fala local, preservado no relatório com origem, hash,
/// motor e timestamps. Não é uma conclusão pericial nem altera a evidência.
#[derive(Debug, Clone, Serialize)]
pub struct TranscricaoDoc {
    pub origem: String,
    pub estado: String,
    pub motor: String,
    pub hash_origem: String,
    pub segmentos: Vec<SegmentoTranscricaoDoc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SegmentoTranscricaoDoc {
    pub inicio: String,
    pub fim: String,
    pub texto: String,
    pub falante: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenciaDoc {
    pub numero: i64,
    /// "E-3": como citar a evidência no texto.
    pub referencia: String,
    pub tipo: String,
    pub titulo: String,
    pub categoria: Option<String>,
    pub observacao: Option<String>,
    pub descricao: Option<String>,
    /// Quem marcou, quando, hash registrado e hash conferido agora.
    pub custodia: Vec<Campo>,
    /// "conferida", "divergente" ou "indisponivel".
    pub situacao: String,
    pub situacao_texto: String,
    pub conteudo: ConteudoDoc,
    pub origem: Vec<OrigemDoc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum ConteudoDoc {
    Conversa {
        cabecalho: Vec<Campo>,
        mensagens: Vec<MensagemDoc>,
    },
    Registro {
        campos: Vec<Campo>,
    },
    Arquivo {
        campos: Vec<Campo>,
        figura: Option<FiguraDoc>,
        sem_figura: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct MensagemDoc {
    pub id: i64,
    pub de_mim: bool,
    pub sistema: bool,
    pub autor: String,
    pub dia: String,
    pub hora: String,
    pub texto: Option<String>,
    pub detalhes: Vec<String>,
    pub figura: Option<FiguraDoc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FiguraDoc {
    /// Caminho virtual da imagem no Typst (`/img/…`).
    pub arquivo: String,
    pub legenda: String,
}

/// Origem técnica de um registro (apêndice).
#[derive(Debug, Clone, Serialize)]
pub struct OrigemDoc {
    pub titulo: String,
    pub campos: Vec<Campo>,
    pub referencias: Vec<ReferenciaDoc>,
    /// Trecho: uma linha por mensagem (id, data, tabela/registro, hash).
    pub linhas: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReferenciaDoc {
    pub campos: Vec<Campo>,
    pub valores_brutos: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntegridadeDoc {
    pub campos: Vec<Campo>,
    pub evidencias: Vec<Vec<String>>,
}
