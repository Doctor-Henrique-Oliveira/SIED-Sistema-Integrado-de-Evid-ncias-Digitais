//! Importador "ADB": backup gerado por `adb backup` (arquivo `.ab`), sozinho ou numa
//! pasta com outros arquivos puxados do aparelho (`adb pull`).
//!
//! Formato público no código do Android (AOSP, `BackupManagerService`, `FullBackup` e
//! `BackupAgent`): cabeçalho de texto e um tar (comprimido com zlib) com os dados de cada
//! app em `apps/<pacote>/`: `db/` (bancos), `f/` (arquivos), `sp/` (preferências), `r/`
//! (raiz do app), `_manifest` (pacote, versão do app, versão do Android) e, quando pedido,
//! `shared/` (armazenamento compartilhado). A leitura de dentro do `.ab` é feita pela
//! fonte ([`crate::fonte::conteiner`]); este importador reconhece o backup, lê os
//! `_manifest` e manda os bancos de nome padrão para os mesmos leitores da pasta
//! genérica. Backups cifrados são inventariados, mas não abertos.

use crate::fonte::conteiner::{self, CabecalhoBackup};
use crate::fonte::{CaminhoFonte, FonteSomenteLeitura, SEPARADOR_CONTEINER};
use crate::importador::{AtributoDetectado, Deteccao, Importador, PlanoImportacao, Suporte};
use crate::importadores::generico::ImportadorGenerico;
use crate::{InfoComponente, Resultado};
use std::io::{BufReader, Read};

pub const ID: &str = "adb";
/// Backups examinados na detecção (ela precisa ser rápida).
const MAX_BACKUPS_DETECCAO: usize = 20;
/// `_manifest` descritos nas observações (o resto é só contado).
const MAX_MANIFESTOS_OBSERVADOS: usize = 60;
const MAX_MANIFESTO: u64 = 64 * 1024;

pub struct AdbImporter;

/// Um `.ab` encontrado e o seu cabeçalho.
struct Backup {
    caminho: CaminhoFonte,
    cabecalho: CabecalhoBackup,
}

/// Campos do `_manifest` de um app (linhas 1–4 do formato do Android).
#[derive(Debug, PartialEq, Eq)]
struct Manifesto {
    pacote: String,
    versao_app: String,
    api_android: String,
}

fn ler_manifesto(texto: &str) -> Option<Manifesto> {
    let mut l = texto.lines();
    let _versao_formato = l.next()?;
    let pacote = l.next()?.trim().to_string();
    let versao_app = l.next()?.trim().to_string();
    let api_android = l.next()?.trim().to_string();
    let numero = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (!pacote.is_empty() && numero(&versao_app) && numero(&api_android)).then_some(Manifesto {
        pacote,
        versao_app,
        api_android,
    })
}

impl AdbImporter {
    fn backups(&self, fonte: &dyn FonteSomenteLeitura, limite: usize) -> Resultado<Vec<Backup>> {
        let mut r = Vec::new();
        for e in fonte.listar_sem_conteineres() {
            let Ok(e) = e else { continue };
            if e.diretorio
                || conteiner::tipo_por_nome(e.caminho.nome())
                    != Some(conteiner::TipoConteiner::BackupAndroid)
            {
                continue;
            }
            let Ok(leitor) = fonte.abrir(&e.caminho) else {
                continue;
            };
            let mut b = BufReader::new(leitor.take(1024));
            if let Ok(Some(cabecalho)) = conteiner::ler_cabecalho_backup(&mut b) {
                r.push(Backup {
                    caminho: e.caminho,
                    cabecalho,
                });
                if r.len() >= limite {
                    break;
                }
            }
        }
        Ok(r)
    }
}

impl Importador for AdbImporter {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "ADB (backup .ab)".into(),
            versao: "1.0.0".into(),
        }
    }

    fn detectar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Deteccao> {
        let backups = self.backups(fonte, MAX_BACKUPS_DETECCAO)?;
        let abertos = backups.iter().filter(|b| !b.cabecalho.cifrado()).count();
        let mut motivos: Vec<String> = backups
            .iter()
            .map(|b| {
                let c = &b.cabecalho;
                format!(
                    "backup do Android {} (versão {}, {}{})",
                    b.caminho,
                    c.versao,
                    if c.comprimido {
                        "comprimido"
                    } else {
                        "sem compressão"
                    },
                    if c.cifrado() {
                        format!(", cifrado com {}: não será aberto", c.cifra)
                    } else {
                        String::new()
                    }
                )
            })
            .collect();
        let (confianca, suporte) = if backups.is_empty() {
            (0.0, Suporte::NaoReconhecido)
        } else if abertos > 0 {
            motivos.push(
                "interpreta dentro do backup os bancos de nome padrão do Android e do WhatsApp; \
                 o restante é inventariado com SHA-256"
                    .into(),
            );
            (0.85, Suporte::Parcial)
        } else {
            motivos.push("todos os backups estão cifrados: só o inventário será feito".into());
            (0.5, Suporte::Parcial)
        };
        Ok(Deteccao {
            importador: self.info(),
            confianca,
            suporte,
            motivos,
        })
    }

    fn planejar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<PlanoImportacao> {
        let backups = self.backups(fonte, usize::MAX)?;
        let mut plano = PlanoImportacao {
            tipo_extracao: Some("backup ADB (.ab)".into()),
            ..Default::default()
        };
        for b in &backups {
            let c = &b.cabecalho;
            plano.observacoes.push(if c.cifrado() {
                format!(
                    "{}: backup cifrado ({}), inventariado com hash mas não aberto (exige a senha)",
                    b.caminho, c.cifra
                )
            } else {
                format!(
                    "{}: backup do Android versão {}, {}",
                    b.caminho,
                    c.versao,
                    if c.comprimido {
                        "comprimido (zlib)"
                    } else {
                        "sem compressão"
                    }
                )
            });
        }

        // _manifest de cada app dentro dos backups: pacote, versão do app e do Android.
        let mut manifestos = 0usize;
        let mut api: Option<(String, CaminhoFonte)> = None;
        for e in fonte.listar() {
            let Ok(e) = e else { continue };
            let s = e.caminho.como_str();
            let dentro = s
                .split_once(SEPARADOR_CONTEINER)
                .map(|(_, i)| i)
                .unwrap_or("");
            let partes: Vec<&str> = dentro.split('/').collect();
            if e.diretorio || partes.len() != 3 || partes[0] != "apps" || partes[2] != "_manifest" {
                continue;
            }
            let mut texto = String::new();
            let lido = fonte
                .abrir(&e.caminho)
                .and_then(|l| Ok(l.take(MAX_MANIFESTO).read_to_string(&mut texto)?));
            let Some(m) = lido.ok().and_then(|_| ler_manifesto(&texto)) else {
                plano.observacoes.push(format!(
                    "{}: _manifest em formato não reconhecido",
                    e.caminho
                ));
                continue;
            };
            manifestos += 1;
            if manifestos <= MAX_MANIFESTOS_OBSERVADOS {
                plano.observacoes.push(format!(
                    "{}: app {} (versão do app {}, Android API {})",
                    e.caminho, m.pacote, m.versao_app, m.api_android
                ));
            }
            if api.is_none() {
                api = Some((m.api_android, e.caminho.clone()));
            }
        }
        if manifestos > MAX_MANIFESTOS_OBSERVADOS {
            plano.observacoes.push(format!(
                "mais {} app(s) com _manifest no backup (não listados aqui)",
                manifestos - MAX_MANIFESTOS_OBSERVADOS
            ));
        }
        if let Some((valor, origem)) = api {
            plano.atributos_dispositivo.push(AtributoDetectado {
                chave: "api_android".into(),
                valor,
                origem,
            });
        }

        // Bancos de nome padrão, dentro dos backups e fora deles (adb pull).
        let generico = ImportadorGenerico.planejar(fonte)?;
        plano.tarefas = generico.tarefas;
        plano.observacoes.extend(generico.observacoes);
        Ok(plano)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fonte::conteiner::testes::{backup, tar};
    use crate::fonte::pasta::PastaSomenteLeitura;
    use std::fs;

    const MANIFESTO: &[u8] = b"1\ncom.android.providers.telephony\n34\n34\n\n0\n0\n";

    #[test]
    fn manifesto_do_android() {
        assert_eq!(
            ler_manifesto("1\ncom.whatsapp\n231114\n30\ncom.android.vending\n0\n1\nxx\n"),
            Some(Manifesto {
                pacote: "com.whatsapp".into(),
                versao_app: "231114".into(),
                api_android: "30".into(),
            })
        );
        assert_eq!(ler_manifesto("1\ncom.whatsapp\nabc\n30\n"), None);
        assert_eq!(ler_manifesto("1\n"), None);
    }

    /// Backup SINTÉTICO: só os nomes (pacote, db/, _manifest) seguem o Android.
    #[test]
    fn detecta_e_planeja_backup() {
        let d = tempfile::tempdir().unwrap();
        let t = tar(&[
            (
                "apps/com.android.providers.telephony/_manifest",
                MANIFESTO,
                b'0',
            ),
            (
                "apps/com.android.providers.telephony/db/mmssms.db",
                b"SQLite format 3\0",
                b'0',
            ),
        ]);
        fs::write(d.path().join("backup.ab"), backup(&t, true)).unwrap();
        fs::write(
            d.path().join("cifrado.ab"),
            b"ANDROID BACKUP\n5\n1\nAES-256\nxx",
        )
        .unwrap();
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let det = AdbImporter.detectar(&f).unwrap();
        assert_eq!(det.suporte, Suporte::Parcial);
        assert!(det.confianca > 0.8);
        assert!(det.motivos.iter().any(|m| m.contains("AES-256")));

        let plano = AdbImporter.planejar(&f).unwrap();
        let t: Vec<(String, String)> = plano
            .tarefas
            .iter()
            .map(|t| (t.parser_id.clone(), t.arquivos[0].caminho.to_string()))
            .collect();
        let banco = "backup.ab!/apps/com.android.providers.telephony/db/mmssms.db";
        assert_eq!(
            t,
            vec![
                ("android_sms".into(), banco.into()),
                ("android_mms".into(), banco.into()),
            ]
        );
        assert_eq!(plano.atributos_dispositivo[0].valor, "34");
        assert!(plano
            .observacoes
            .iter()
            .any(|o| o.contains("com.android.providers.telephony") && o.contains("API 34")));
        assert!(plano.observacoes.iter().any(|o| o.contains("cifrado")));
    }

    #[test]
    fn pasta_sem_backup_nao_e_adb() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("x.ab"), "texto").unwrap();
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        assert_eq!(
            AdbImporter.detectar(&f).unwrap().suporte,
            Suporte::NaoReconhecido
        );
    }
}
