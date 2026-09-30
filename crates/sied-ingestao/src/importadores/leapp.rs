//! Importador "ALEAPP/iLEAPP": pasta do relatório gerado pelo ALEAPP (Android) ou pelo
//! iLEAPP (iOS), de Alexis Brignoni e colaboradores.
//!
//! Estrutura conferida no código-fonte público dos dois projetos (abrignoni/ALEAPP e
//! abrignoni/iLEAPP: `scripts/ilapfuncs.py`, `scripts/lavafuncs.py`, `aleapp.py`):
//! - `_TSV Exports/<artefato>.tsv`: um arquivo por artefato encontrado (lido aqui);
//! - `_lava_data.lava`: projeto em JSON com `parser_info.leapp_name` e `leapp_version`
//!   (identifica a ferramenta), a pasta de entrada e o tipo de entrada;
//! - `_lava_artifacts.db`, `_HTML/`, `_Timeline/tl.db`, `_KML Exports/`, `data/`
//!   (evidência preparada pela ferramenta) e `media/`: inventariados com SHA-256, mas não
//!   interpretados.
//!
//! As exportações TSV são o formato de troca que os dois projetos mantêm há mais tempo e
//! do mesmo jeito; o banco LAVA muda com a versão do esquema e não é lido nesta versão.

use crate::fonte::{CaminhoFonte, FonteSomenteLeitura};
use crate::importador::{
    ArquivoParaParser, Deteccao, Importador, PlanoImportacao, Suporte, TarefaParser,
};
use crate::parsers::leapp_tsv::{self, Ferramenta};
use crate::{InfoComponente, Resultado};
use std::io::Read;

pub const ID: &str = "leapp";
const PROJETO: &str = "_lava_data.lava";
/// Arquivos cuja presença na raiz indica um relatório do xLEAPP.
const MARCAS: &[&str] = &["_lava_artifacts.db", "_HTML", "_Timeline"];
/// O projeto cresce com o número de artefatos; acima disso não é lido.
const MAX_PROJETO: u64 = 64 * 1024 * 1024;

pub struct LeappImporter;

/// O que o projeto `_lava_data.lava` diz sobre a execução.
#[derive(Debug, Default, PartialEq)]
struct Projeto {
    ferramenta: Option<String>,
    versao: Option<String>,
    entrada: Option<String>,
    tipo_entrada: Option<String>,
}

fn ler_projeto(texto: &str) -> Option<Projeto> {
    let v: serde_json::Value = serde_json::from_str(texto).ok()?;
    let s = |x: &serde_json::Value| x.as_str().map(String::from);
    let info = v.get("parser_info")?;
    Some(Projeto {
        ferramenta: info.get("leapp_name").and_then(s),
        versao: info.get("leapp_version").and_then(s),
        entrada: v.get("param_input").and_then(s),
        tipo_entrada: v.get("param_type").and_then(s),
    })
}

impl LeappImporter {
    fn projeto(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Option<Projeto>> {
        let c = CaminhoFonte::novo(PROJETO)?;
        match fonte.entrada(&c)? {
            Some(e) if !e.diretorio && e.tamanho <= MAX_PROJETO => {}
            _ => return Ok(None),
        }
        let mut texto = String::new();
        let lido = fonte
            .abrir(&c)
            .and_then(|l| Ok(l.take(MAX_PROJETO).read_to_string(&mut texto)?));
        Ok(lido.ok().and_then(|_| ler_projeto(&texto)))
    }

    fn tem_pasta_tsv(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<bool> {
        Ok(fonte
            .entrada(&CaminhoFonte::novo(leapp_tsv::PASTA_TSV)?)?
            .is_some_and(|e| e.diretorio))
    }

    /// Arquivos `.tsv` logo dentro de `_TSV Exports/`, em ordem de nome.
    fn tsvs(&self, fonte: &dyn FonteSomenteLeitura) -> Vec<CaminhoFonte> {
        let prefixo = format!("{}/", leapp_tsv::PASTA_TSV);
        let mut v: Vec<CaminhoFonte> = fonte
            .listar_sem_conteineres()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let s = e.caminho.como_str();
                !e.diretorio
                    && s.strip_prefix(&prefixo)
                        .is_some_and(|resto| !resto.contains('/'))
                    && s.to_ascii_lowercase().ends_with(".tsv")
            })
            .map(|e| e.caminho)
            .collect();
        v.sort_by(|a, b| a.como_str().cmp(b.como_str()));
        v
    }

    fn marcas(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Vec<&'static str>> {
        let mut r = Vec::new();
        for m in MARCAS {
            if fonte.entrada(&CaminhoFonte::novo(m)?)?.is_some() {
                r.push(*m);
            }
        }
        Ok(r)
    }
}

fn ferramenta(p: Option<&Projeto>) -> Ferramenta {
    p.and_then(|p| p.ferramenta.as_deref())
        .map(Ferramenta::pelo_nome)
        .unwrap_or(Ferramenta::Desconhecida)
}

impl Importador for LeappImporter {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "ALEAPP / iLEAPP".into(),
            versao: "1.0.0".into(),
        }
    }

    fn detectar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Deteccao> {
        let projeto = self.projeto(fonte)?;
        let tsv = self.tem_pasta_tsv(fonte)?;
        let marcas = self.marcas(fonte)?;
        let mut motivos = Vec::new();
        if let Some(p) = &projeto {
            motivos.push(format!(
                "{PROJETO}: relatório do {} {}",
                p.ferramenta.as_deref().unwrap_or("xLEAPP"),
                p.versao.as_deref().unwrap_or("(versão não informada)")
            ));
        }
        motivos.extend(marcas.iter().map(|m| format!("encontrado {m}")));
        if tsv {
            motivos.push(format!("encontrada a pasta {}", leapp_tsv::PASTA_TSV));
        }
        let reconhecido = projeto.is_some() || !marcas.is_empty();
        let (confianca, suporte) = match (reconhecido, tsv) {
            (true, true) => (0.9, Suporte::Parcial),
            (false, true) => (0.6, Suporte::Parcial),
            (true, false) => {
                motivos.push(format!(
                    "sem a pasta {}: gere o relatório com a saída TSV para os artefatos \
                     serem lidos",
                    leapp_tsv::PASTA_TSV
                ));
                (0.5, Suporte::Parcial)
            }
            (false, false) => (0.0, Suporte::NaoReconhecido),
        };
        if suporte != Suporte::NaoReconhecido {
            motivos.push(
                "lê as exportações TSV; o banco LAVA, o HTML e as cópias da evidência \
                 entram só no inventário"
                    .into(),
            );
        }
        Ok(Deteccao {
            importador: self.info(),
            confianca,
            suporte,
            motivos,
        })
    }

    fn planejar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<PlanoImportacao> {
        let projeto = self.projeto(fonte)?;
        let f = ferramenta(projeto.as_ref());
        let mut plano = PlanoImportacao {
            tipo_extracao: Some(format!("relatório do {}", f.nome())),
            ..Default::default()
        };
        // Ferramenta e versão como o próprio projeto declara (`_lava_data.lava`).
        plano.ferramenta_versao = projeto.as_ref().and_then(|p| {
            let nome = p.ferramenta.as_deref()?;
            Some(match p.versao.as_deref() {
                Some(v) => format!("{nome} {v}"),
                None => nome.to_string(),
            })
        });
        match &projeto {
            Some(p) => {
                plano.observacoes.push(format!(
                    "{PROJETO}: {} {}{}{}",
                    p.ferramenta
                        .as_deref()
                        .unwrap_or("ferramenta não informada"),
                    p.versao.as_deref().unwrap_or("(versão não informada)"),
                    p.tipo_entrada
                        .as_deref()
                        .map(|t| format!(", entrada do tipo \"{t}\""))
                        .unwrap_or_default(),
                    p.entrada
                        .as_deref()
                        .map(|e| format!(", entrada original: {e}"))
                        .unwrap_or_default()
                ));
                if f == Ferramenta::Desconhecida {
                    plano.observacoes.push(
                        "o projeto não diz se é ALEAPP ou iLEAPP: registros marcados como \
                         ALEAPP/iLEAPP"
                            .into(),
                    );
                }
            }
            None => plano.observacoes.push(format!(
                "sem {PROJETO} legível: a ferramenta (ALEAPP ou iLEAPP) não é identificada"
            )),
        }
        let tsvs = self.tsvs(fonte);
        if tsvs.is_empty() {
            plano.observacoes.push(format!(
                "nenhum arquivo .tsv em {}: os artefatos não serão lidos",
                leapp_tsv::PASTA_TSV
            ));
        } else {
            plano.observacoes.push(format!(
                "{} artefato(s) exportado(s) em {} → parser {}",
                tsvs.len(),
                leapp_tsv::PASTA_TSV,
                f.id_parser()
            ));
        }
        for c in tsvs {
            plano.tarefas.push(TarefaParser {
                parser_id: f.id_parser().into(),
                arquivos: vec![ArquivoParaParser {
                    papel: leapp_tsv::PAPEL.into(),
                    caminho: c,
                }],
            });
        }
        plano.observacoes.push(
            "_lava_artifacts.db, _HTML/, _Timeline/, _KML Exports/, data/ e media/: \
             inventariados com hash, não interpretados"
                .into(),
        );
        Ok(plano)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fonte::pasta::PastaSomenteLeitura;
    use std::fs;

    /// Relatório SINTÉTICO: só os nomes das pastas e dos campos seguem o xLEAPP.
    fn relatorio(r: &std::path::Path, nome: Option<&str>) {
        for (p, c) in [
            ("_TSV Exports/SMS messages.tsv", "\u{feff}A\tB\r\n1\t2\r\n"),
            ("_TSV Exports/Call logs.tsv", "\u{feff}A\r\n1\r\n"),
            ("_TSV Exports/sub/Outro.tsv", "A\r\n"),
            ("_TSV Exports/leia.txt", "x"),
            ("_HTML/index.html", "<html></html>"),
            ("data/mmssms.db", "sintetico"),
        ] {
            fs::create_dir_all(r.join(p).parent().unwrap()).unwrap();
            fs::write(r.join(p), c).unwrap();
        }
        if let Some(n) = nome {
            fs::write(
                r.join(PROJETO),
                format!(
                    r#"{{"parser_info":{{"leapp_name":"{n}","leapp_version":"2026.4.2"}},
                       "param_input":"C:\\extracao.zip","param_type":"zip"}}"#
                ),
            )
            .unwrap();
        }
    }

    #[test]
    fn projeto_do_lava() {
        let p = ler_projeto(
            r#"{"parser_info":{"leapp_name":"iLEAPP","leapp_version":"2026.1"},"param_type":"tar"}"#,
        )
        .unwrap();
        assert_eq!(p.ferramenta.as_deref(), Some("iLEAPP"));
        assert_eq!(p.tipo_entrada.as_deref(), Some("tar"));
        assert_eq!(ler_projeto("{}"), None);
        assert_eq!(ler_projeto("não é json"), None);
    }

    #[test]
    fn detecta_e_planeja_por_ferramenta() {
        let d = tempfile::tempdir().unwrap();
        relatorio(d.path(), Some("ALEAPP"));
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let det = LeappImporter.detectar(&f).unwrap();
        assert_eq!(det.suporte, Suporte::Parcial);
        assert!(det.confianca > 0.8);
        assert!(det.motivos.iter().any(|m| m.contains("ALEAPP 2026.4.2")));

        let plano = LeappImporter.planejar(&f).unwrap();
        let t: Vec<(String, String)> = plano
            .tarefas
            .iter()
            .map(|t| (t.parser_id.clone(), t.arquivos[0].caminho.to_string()))
            .collect();
        assert_eq!(
            t,
            vec![
                ("aleapp_tsv".into(), "_TSV Exports/Call logs.tsv".into()),
                ("aleapp_tsv".into(), "_TSV Exports/SMS messages.tsv".into()),
            ]
        );
        assert_eq!(plano.tipo_extracao.as_deref(), Some("relatório do ALEAPP"));
        assert_eq!(plano.ferramenta_versao.as_deref(), Some("ALEAPP 2026.4.2"));
        assert!(plano.observacoes.iter().any(|o| o.contains("extracao.zip")));

        let d = tempfile::tempdir().unwrap();
        relatorio(d.path(), Some("iLEAPP"));
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let plano = LeappImporter.planejar(&f).unwrap();
        assert_eq!(plano.tarefas[0].parser_id, "ileapp_tsv");
    }

    #[test]
    fn so_tsv_sem_projeto_e_pasta_comum() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("_TSV Exports")).unwrap();
        fs::write(d.path().join("_TSV Exports/X.tsv"), "A\r\n").unwrap();
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let det = LeappImporter.detectar(&f).unwrap();
        assert_eq!(det.suporte, Suporte::Parcial);
        assert_eq!(
            LeappImporter.planejar(&f).unwrap().tarefas[0].parser_id,
            "xleapp_tsv"
        );

        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("a.tsv"), "A\r\n").unwrap();
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        assert_eq!(
            LeappImporter.detectar(&f).unwrap().suporte,
            Suporte::NaoReconhecido
        );
    }
}
