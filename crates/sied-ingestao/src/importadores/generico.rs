//! Importador "Pasta genérica": aceita qualquer pasta.
//!
//! Não conhece nenhuma ferramenta. Serve de alternativa quando a ferramenta não é
//! reconhecida e de base de testes do pipeline. O inventário (hashes, manifesto,
//! classificação) é completo; a interpretação se limita a bancos com nome padrão do
//! Android, localizados em qualquer ponto da árvore.

use crate::fonte::FonteSomenteLeitura;
use crate::importador::{
    ArquivoParaParser, Deteccao, Importador, PlanoImportacao, Suporte, TarefaParser,
};
use crate::parsers::android_sms;
use crate::{InfoComponente, Resultado};

pub const ID: &str = "generico";

/// Nome de arquivo → (parser, papel). Nomes definidos pelo próprio Android (AOSP),
/// não por uma ferramenta de extração.
const CONHECIDOS: &[(&str, &str, &str)] = &[("mmssms.db", android_sms::ID, android_sms::PAPEL)];

pub struct ImportadorGenerico;

impl Importador for ImportadorGenerico {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "Pasta genérica".into(),
            versao: "1.0.0".into(),
        }
    }

    fn detectar(&self, _fonte: &dyn FonteSomenteLeitura) -> Resultado<Deteccao> {
        Ok(Deteccao {
            importador: self.info(),
            confianca: 0.1,
            suporte: Suporte::Parcial,
            motivos: vec![
                "aceita qualquer pasta: inventário, classificação e SHA-256 de todos os arquivos"
                    .into(),
                "interpreta apenas bancos com nome padrão do Android (mmssms.db)".into(),
            ],
        })
    }

    fn planejar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<PlanoImportacao> {
        let mut plano = PlanoImportacao {
            tipo_extracao: Some("pasta".into()),
            ..Default::default()
        };
        for e in fonte.listar() {
            let Ok(e) = e else { continue }; // já registrado no inventário
            if e.diretorio {
                continue;
            }
            let nome = e.caminho.nome().to_ascii_lowercase();
            if let Some((_, parser, papel)) = CONHECIDOS.iter().find(|(n, _, _)| *n == nome) {
                plano.observacoes.push(format!(
                    "{} → parser {parser} (nome padrão do Android)",
                    e.caminho
                ));
                plano.tarefas.push(TarefaParser {
                    parser_id: (*parser).into(),
                    arquivos: vec![ArquivoParaParser {
                        papel: (*papel).into(),
                        caminho: e.caminho,
                    }],
                });
            }
        }
        if plano.tarefas.is_empty() {
            plano
                .observacoes
                .push("nenhum banco conhecido encontrado; apenas o inventário foi feito".into());
        }
        Ok(plano)
    }
}
