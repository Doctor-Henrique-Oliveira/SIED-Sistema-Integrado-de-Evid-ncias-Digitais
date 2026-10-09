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
    android_chamadas, android_contatos, android_mms, android_sms, browser_history, email_eml,
    ios_contatos, ios_sms, telegram_json, whatsapp_android,
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
                "abre .tar, .zip e backup .ab sem cifra (itens de dentro também entram no inventário; \
                 ZIP tem limite de 200.000 entradas e 512 MiB por item)"
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
            let caminho_lower = c.como_str().to_ascii_lowercase();
            if n == "sms.db" {
                plano.observacoes.push(format!(
                    "{c} → parser {} (banco message do iOS; esquema conferido pelo conteúdo)",
                    ios_sms::ID
                ));
                plano.tarefas.push(TarefaParser {
                    parser_id: ios_sms::ID.into(),
                    arquivos: vec![ArquivoParaParser {
                        papel: ios_sms::PAPEL.into(),
                        caminho: c.clone(),
                    }],
                });
                continue;
            }
            if n == "addressbook.sqlitedb" {
                plano.observacoes.push(format!(
                    "{c} → parser {} (contatos iOS; só aceita tabela ABPerson e preserva colunas brutas)",
                    ios_contatos::ID
                ));
                plano.tarefas.push(TarefaParser {
                    parser_id: ios_contatos::ID.into(),
                    arquivos: vec![ArquivoParaParser {
                        papel: ios_contatos::PAPEL.into(),
                        caminho: c.clone(),
                    }],
                });
                continue;
            }
            if n == "history" || n == "places.sqlite" || n == "history.db" {
                plano.observacoes.push(format!(
                    "{c} → parser {} (histórico SQLite; esquema Chrome/Firefox/Safari conferido)",
                    browser_history::ID
                ));
                plano.tarefas.push(TarefaParser {
                    parser_id: browser_history::ID.into(),
                    arquivos: vec![ArquivoParaParser {
                        papel: browser_history::PAPEL.into(),
                        caminho: c.clone(),
                    }],
                });
                continue;
            }
            if n == "result.json" && caminho_lower.contains("telegram") {
                plano.observacoes.push(format!(
                    "{c} → parser {} (exportação JSON oficial do Telegram)",
                    telegram_json::ID
                ));
                plano.tarefas.push(TarefaParser {
                    parser_id: telegram_json::ID.into(),
                    arquivos: vec![ArquivoParaParser {
                        papel: telegram_json::PAPEL.into(),
                        caminho: c.clone(),
                    }],
                });
                continue;
            }
            if n.ends_with(".eml") {
                plano.observacoes.push(format!(
                    "{c} → parser {} (mensagem RFC 5322; MIME complexo permanece nos brutos)",
                    email_eml::ID
                ));
                plano.tarefas.push(TarefaParser {
                    parser_id: email_eml::ID.into(),
                    arquivos: vec![ArquivoParaParser {
                        papel: email_eml::PAPEL.into(),
                        caminho: c.clone(),
                    }],
                });
                continue;
            }
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

    #[test]
    fn plano_reconhece_fontes_documentadas_adicionais() {
        let d = tempfile::tempdir().unwrap();
        for p in [
            "Library/SMS/sms.db",
            "Library/AddressBook/AddressBook.sqlitedb",
            "Chrome/Default/History",
            "Firefox/places.sqlite",
            "Telegram/result.json",
            "mail/arquivo.eml",
        ] {
            fs::create_dir_all(d.path().join(p).parent().unwrap()).unwrap();
            fs::write(d.path().join(p), b"sintetico").unwrap();
        }
        let fonte = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let plano = ImportadorGenerico.planejar(&fonte).unwrap();
        let ids: Vec<_> = plano.tarefas.iter().map(|t| t.parser_id.as_str()).collect();
        assert!(ids.contains(&"ios_sms"));
        assert!(ids.contains(&"ios_contatos"));
        assert_eq!(
            ids.iter()
                .filter(|id| **id == "historico_navegador")
                .count(),
            2
        );
        assert!(ids.contains(&"telegram_json"));
        assert!(ids.contains(&"email_eml"));
    }
}
