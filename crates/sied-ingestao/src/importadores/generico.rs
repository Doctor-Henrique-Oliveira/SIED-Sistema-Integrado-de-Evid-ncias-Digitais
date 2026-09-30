//! Importador "Pasta genérica": aceita qualquer pasta.
//!
//! Não conhece nenhuma ferramenta. Serve de alternativa quando a ferramenta não é
//! reconhecida e de base de testes do pipeline. O inventário (hashes, manifesto,
//! classificação) é completo; a interpretação se limita a bancos com nome padrão do
//! Android e do WhatsApp, localizados em qualquer ponto da árvore.
//!
//! WhatsApp: cada `msgstore.db` vira uma tarefa junto com o `wa.db` da mesma pasta
//! (é assim que o app grava os dois). Um `wa.db` sozinho vira tarefa só de contatos.
//! Backups cifrados (`msgstore*.db.crypt*`) são inventariados e anotados, mas não
//! interpretados: exigem a chave do aparelho.
//!
//! Agenda e chamadas do Android: `contacts2.db` vai para a agenda; `calllog.db`, para o
//! registro de chamadas. Até o Android 6 as chamadas ficavam no próprio `contacts2.db`:
//! sem `calllog.db` na mesma pasta, o `contacts2.db` também vai para o leitor de
//! chamadas (que avisa, sem falhar, se a tabela não estiver lá).
//!
//! SMS e MMS: o `mmssms.db` vai para o leitor de SMS (tabela `sms`) e para o de MMS
//! (tabelas `pdu`, `addr`, `part`).

use crate::fonte::CaminhoFonte;
use crate::fonte::FonteSomenteLeitura;
use crate::importador::{
    ArquivoParaParser, Deteccao, Importador, PlanoImportacao, Suporte, TarefaParser,
};
use crate::parsers::{
    android_chamadas, android_contatos, android_mms, android_sms, whatsapp_android,
};
use crate::{InfoComponente, Resultado};

pub const ID: &str = "generico";

/// Nome de arquivo → (parser, papel). Nomes definidos pelo próprio Android (AOSP),
/// não por uma ferramenta de extração.
const CONHECIDOS: &[(&str, &str, &str)] = &[
    ("mmssms.db", android_sms::ID, android_sms::PAPEL),
    (
        "contacts2.db",
        android_contatos::ID,
        android_contatos::PAPEL,
    ),
    ("calllog.db", android_chamadas::ID, android_chamadas::PAPEL),
];

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
                "interpreta apenas bancos com nome padrão do Android e do WhatsApp \
                 (mmssms.db, contacts2.db, calllog.db, msgstore.db, wa.db)"
                    .into(),
            ],
        })
    }

    fn planejar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<PlanoImportacao> {
        let mut plano = PlanoImportacao {
            tipo_extracao: Some("pasta".into()),
            ..Default::default()
        };
        let mut arquivos: Vec<CaminhoFonte> = Vec::new();
        for e in fonte.listar() {
            let Ok(e) = e else { continue }; // já registrado no inventário
            if !e.diretorio {
                arquivos.push(e.caminho);
            }
        }
        let nome = |c: &CaminhoFonte| c.nome().to_ascii_lowercase();
        let pasta = |c: &CaminhoFonte| {
            c.como_str()
                .rsplit_once('/')
                .map_or(String::new(), |(p, _)| p.to_string())
        };

        for c in &arquivos {
            let n = nome(c);
            if let Some((_, parser, papel)) = CONHECIDOS.iter().find(|(x, _, _)| *x == n) {
                plano
                    .observacoes
                    .push(format!("{c} → parser {parser} (nome padrão do Android)"));
                plano.tarefas.push(TarefaParser {
                    parser_id: (*parser).into(),
                    arquivos: vec![ArquivoParaParser {
                        papel: (*papel).into(),
                        caminho: c.clone(),
                    }],
                });
                if n == "mmssms.db" {
                    plano.observacoes.push(format!(
                        "{c} → parser {} (MMS do mesmo banco)",
                        android_mms::ID
                    ));
                    plano.tarefas.push(TarefaParser {
                        parser_id: android_mms::ID.into(),
                        arquivos: vec![ArquivoParaParser {
                            papel: android_mms::PAPEL.into(),
                            caminho: c.clone(),
                        }],
                    });
                }
                if n == "contacts2.db"
                    && !arquivos
                        .iter()
                        .any(|o| nome(o) == "calllog.db" && pasta(o) == pasta(c))
                {
                    plano.observacoes.push(format!(
                        "{c} → parser {} (sem calllog.db na mesma pasta: até o Android 6 \
                         as chamadas ficavam no contacts2.db)",
                        android_chamadas::ID
                    ));
                    plano.tarefas.push(TarefaParser {
                        parser_id: android_chamadas::ID.into(),
                        arquivos: vec![ArquivoParaParser {
                            papel: android_chamadas::PAPEL.into(),
                            caminho: c.clone(),
                        }],
                    });
                }
            } else if n == "msgstore.db" {
                let wa = arquivos
                    .iter()
                    .find(|o| nome(o) == "wa.db" && pasta(o) == pasta(c));
                let mut tarefa = TarefaParser {
                    parser_id: whatsapp_android::ID.into(),
                    arquivos: vec![ArquivoParaParser {
                        papel: whatsapp_android::PAPEL_MSGSTORE.into(),
                        caminho: c.clone(),
                    }],
                };
                match wa {
                    Some(w) => {
                        plano.observacoes.push(format!(
                            "{c} + {w} → parser {} (nomes padrão do WhatsApp)",
                            whatsapp_android::ID
                        ));
                        tarefa.arquivos.push(ArquivoParaParser {
                            papel: whatsapp_android::PAPEL_WA.into(),
                            caminho: w.clone(),
                        });
                    }
                    None => plano.observacoes.push(format!(
                        "{c} → parser {} (sem wa.db na mesma pasta: conversas sem nomes)",
                        whatsapp_android::ID
                    )),
                }
                plano.tarefas.push(tarefa);
            } else if n == "wa.db"
                && !arquivos
                    .iter()
                    .any(|o| nome(o) == "msgstore.db" && pasta(o) == pasta(c))
            {
                plano.observacoes.push(format!(
                    "{c} → parser {} (só contatos: sem msgstore.db na mesma pasta)",
                    whatsapp_android::ID
                ));
                plano.tarefas.push(TarefaParser {
                    parser_id: whatsapp_android::ID.into(),
                    arquivos: vec![ArquivoParaParser {
                        papel: whatsapp_android::PAPEL_WA.into(),
                        caminho: c.clone(),
                    }],
                });
            } else if n.starts_with("msgstore") && n.contains(".crypt") {
                plano.observacoes.push(format!(
                    "{c}: backup cifrado do WhatsApp, inventariado com hash mas não interpretado"
                ));
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

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fonte::pasta::PastaSomenteLeitura;
    use std::fs;

    /// Pasta SINTÉTICA: só os nomes de arquivo importam para o plano.
    #[test]
    fn plano_junta_msgstore_e_wa_da_mesma_pasta() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        for p in [
            "a/com.whatsapp/databases/msgstore.db",
            "a/com.whatsapp/databases/wa.db",
            "b/com.whatsapp.w4b/databases/msgstore.db",
            "c/wa.db",
            "d/Databases/msgstore-2026-01-01.1.db.crypt14",
            "e/mmssms.db",
            "f/com.android.providers.contacts/databases/contacts2.db",
            "f/com.android.providers.contacts/databases/calllog.db",
            "g/contacts2.db",
        ] {
            fs::create_dir_all(r.join(p).parent().unwrap()).unwrap();
            fs::write(r.join(p), b"sintetico").unwrap();
        }
        let fonte = PastaSomenteLeitura::abrir(r).unwrap();
        let plano = ImportadorGenerico.planejar(&fonte).unwrap();
        let mut tarefas: Vec<(String, Vec<(String, String)>)> = plano
            .tarefas
            .iter()
            .map(|t| {
                (
                    t.parser_id.clone(),
                    t.arquivos
                        .iter()
                        .map(|a| (a.papel.clone(), a.caminho.to_string()))
                        .collect(),
                )
            })
            .collect();
        tarefas.sort();
        let par = |p: &str, c: &str| (p.to_string(), c.to_string());
        assert_eq!(
            tarefas,
            vec![
                (
                    "android_chamadas".into(),
                    vec![par(
                        "chamadas",
                        "f/com.android.providers.contacts/databases/calllog.db"
                    )]
                ),
                // Sem calllog.db na pasta: chamadas procuradas no próprio contacts2.db.
                (
                    "android_chamadas".into(),
                    vec![par("chamadas", "g/contacts2.db")]
                ),
                (
                    "android_contatos".into(),
                    vec![par(
                        "contacts2",
                        "f/com.android.providers.contacts/databases/contacts2.db"
                    )]
                ),
                (
                    "android_contatos".into(),
                    vec![par("contacts2", "g/contacts2.db")]
                ),
                ("android_mms".into(), vec![par("mmssms", "e/mmssms.db")]),
                ("android_sms".into(), vec![par("mmssms", "e/mmssms.db")]),
                (
                    "whatsapp_android".into(),
                    vec![
                        par("msgstore", "a/com.whatsapp/databases/msgstore.db"),
                        par("wa", "a/com.whatsapp/databases/wa.db"),
                    ]
                ),
                (
                    "whatsapp_android".into(),
                    vec![par("msgstore", "b/com.whatsapp.w4b/databases/msgstore.db")]
                ),
                ("whatsapp_android".into(), vec![par("wa", "c/wa.db")]),
            ]
        );
        assert!(plano
            .observacoes
            .iter()
            .any(|o| o.contains("crypt14") && o.contains("não interpretado")));
    }
}
