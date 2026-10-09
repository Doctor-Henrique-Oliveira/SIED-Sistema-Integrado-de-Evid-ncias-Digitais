//! Importador "IPED": pasta de um caso processado pelo IPED (Polícia Federal).
//!
//! Estrutura conferida no código-fonte público do IPED (projeto sepinf-inc/IPED):
//! - `<caso>/iped/` guarda o que o IPED gera para a análise: `IPEDConfig.txt`,
//!   `LocalConfig.txt`, `conf/`, `index/` (índice Lucene), `data/`, `lib/`, `jre/`;
//! - `<caso>/` guarda o aplicativo de análise (`IPED-SearchApp.exe`), a lista de itens
//!   (`FileList.csv`, ou "Lista de Arquivos.csv" em português, entre outras traduções)
//!   e, se o caso foi exportado, a pasta dos arquivos exportados.
//!
//! O que é interpretado nesta versão:
//! - a lista de itens (um registro "arquivo" por linha, com hashes MD5/SHA1 que o IPED
//!   calculou, caminho na evidência, marcadores, categoria e datas);
//! - fora de `iped/`, os mesmos bancos com nome padrão do Android e do WhatsApp que a
//!   pasta genérica reconhece (por exemplo, bancos exportados pelo IPED).
//!
//! O índice Lucene e os bancos internos do IPED **não** são lidos: o formato deles muda
//! com a versão e não é um formato de troca. Todos os arquivos, inclusive os de `iped/`,
//! entram no inventário com SHA-256, como em qualquer extração.

use crate::fonte::{CaminhoFonte, FonteSomenteLeitura};
use crate::importador::{
    ArquivoParaParser, Deteccao, Importador, PlanoImportacao, Suporte, TarefaParser,
};
use crate::importadores::generico::ImportadorGenerico;
use crate::parsers::iped_lista;
use crate::{InfoComponente, Resultado};
use std::io::{BufRead, BufReader, Read};

pub const ID: &str = "iped";
/// Pasta de saída do IPED dentro do caso.
const PASTA_IPED: &str = "iped";
/// Arquivos cuja presença em `iped/` identifica o caso.
const MARCAS: &[&str] = &["iped/IPEDConfig.txt", "iped/index", "iped/conf"];
/// Bytes lidos do começo de uma lista para conferir o cabeçalho.
const MAX_CABECALHO: u64 = 16 * 1024;
/// Configurações são apenas metadados do plano; nunca carregamos arquivos inteiros
/// nem tentamos interpretar o índice Lucene ou bancos internos do IPED.
const MAX_CONFIGURACAO: u64 = 64 * 1024;
const CONFIGURACOES_CONHECIDAS: &[&str] = &[
    "iped/IPEDConfig.txt",
    "iped/LocalConfig.txt",
    "iped/conf/OCRConfig.txt",
    "iped/conf/AudioTranscriptConfig.txt",
];
const CHAVES_CONFIGURACAO: &[&str] = &[
    "enableHash",
    "ignoreDuplicates",
    "enableOCR",
    "enableAudioTranscription",
    "enableImageSimilarity",
    "enableFaceRecognition",
    "enableGraphGeneration",
    "enableQRCode",
    "enableCarving",
    "indexFileContents",
    "expandContainers",
    "processEmbeddedDisks",
    "locale",
    "numThreads",
    "commitIntervalSeconds",
    "implementationClass",
    "OCRLanguage",
];

pub struct IpedImporter;

impl IpedImporter {
    /// Listas de itens na raiz do caso, com cabeçalho reconhecido.
    fn listas(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Vec<CaminhoFonte>> {
        let mut r = Vec::new();
        for nome in iped_lista::NOMES_CSV {
            let c = CaminhoFonte::novo(nome)?;
            let Some(e) = fonte.entrada(&c)? else {
                continue;
            };
            if e.diretorio {
                continue;
            }
            let mut linha = String::new();
            let lido = fonte
                .abrir(&c)
                .map(|l| BufReader::new(l.take(MAX_CABECALHO)).read_line(&mut linha));
            if matches!(lido, Ok(Ok(_))) && iped_lista::ler_cabecalho(&linha).is_some() {
                r.push(c);
            }
        }
        Ok(r)
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

    /// Lê somente arquivos de configuração conhecidos e em limite pequeno. Os valores
    /// entram no log do plano para explicar quais derivados podem ser reaproveitados;
    /// não são usados para habilitar processamento nem para alterar a extração.
    fn observacoes_configuracao(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Vec<String>> {
        let mut observacoes = Vec::new();
        for caminho in CONFIGURACOES_CONHECIDAS {
            let c = CaminhoFonte::novo(caminho)?;
            let Some(entrada) = fonte.entrada(&c)? else {
                continue;
            };
            if entrada.diretorio {
                continue;
            }
            let mut bytes = Vec::new();
            BufReader::new(fonte.abrir(&c)?.take(MAX_CONFIGURACAO)).read_to_end(&mut bytes)?;
            let texto = String::from_utf8_lossy(&bytes);
            let mut achados = Vec::new();
            for linha in texto.lines() {
                let linha = linha.trim();
                if linha.is_empty() || linha.starts_with('#') || linha.starts_with(';') {
                    continue;
                }
                let Some((chave, valor)) = linha.split_once('=') else {
                    continue;
                };
                let chave = chave.trim();
                let valor = valor.trim();
                if !CHAVES_CONFIGURACAO.contains(&chave) || valor.is_empty() {
                    continue;
                }
                // Chaves permitidas são deliberadamente restritas e o valor é truncado
                // para impedir que o plano replique endpoints ou conteúdo inesperado.
                let valor: String = valor.chars().take(120).collect();
                achados.push(format!("{chave}={valor}"));
            }
            if !achados.is_empty() {
                observacoes.push(format!(
                    "{caminho}: configuração declarada ({})",
                    achados.join(", ")
                ));
            } else {
                observacoes.push(format!(
                    "{caminho}: arquivo de configuração presente (sem chaves deriváveis)",
                ));
            }
        }
        Ok(observacoes)
    }

    /// Obtém a versão somente quando ela está explicitamente registrada em um arquivo
    /// do caso. A ausência do arquivo não é preenchida com a versão do instalador local.
    fn versao_declarada(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Option<String>> {
        for caminho in ["iped/version.txt", "iped/ReleaseNotes.txt"] {
            let c = CaminhoFonte::novo(caminho)?;
            let Some(entrada) = fonte.entrada(&c)? else {
                continue;
            };
            if entrada.diretorio {
                continue;
            }
            let mut bytes = Vec::new();
            BufReader::new(fonte.abrir(&c)?.take(MAX_CABECALHO)).read_to_end(&mut bytes)?;
            let texto = String::from_utf8_lossy(&bytes);
            let tokens: Vec<&str> = texto.split_whitespace().collect();
            for (indice, token) in tokens.iter().enumerate() {
                if let Some(versao) = Self::token_versao(token) {
                    return Ok(Some(versao));
                }
                if token.eq_ignore_ascii_case("IPED") {
                    if let Some(proximo) = tokens.get(indice + 1).and_then(|t| Self::versao_bare(t))
                    {
                        return Ok(Some(format!("IPED-{proximo}")));
                    }
                }
            }
        }
        Ok(None)
    }

    fn token_versao(token: &str) -> Option<String> {
        let token =
            token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-');
        let sem_prefixo = token
            .strip_prefix("IPED-")
            .or_else(|| token.strip_prefix("iped-"))
            .or_else(|| token.strip_prefix("IPED_"))
            .or_else(|| token.strip_prefix("iped_"))?;
        if Self::versao_bare(sem_prefixo).is_some() {
            Some(format!("IPED-{sem_prefixo}"))
        } else {
            None
        }
    }

    fn versao_bare(token: &str) -> Option<String> {
        let token = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.');
        if token.chars().next().is_some_and(|c| c.is_ascii_digit()) && token.contains('.') {
            Some(token.to_string())
        } else {
            None
        }
    }
}

impl Importador for IpedImporter {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "IPED".into(),
            versao: "1.0.0".into(),
        }
    }

    fn detectar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Deteccao> {
        let marcas = self.marcas(fonte)?;
        let listas = self.listas(fonte)?;
        let mut motivos: Vec<String> = marcas
            .iter()
            .map(|m| format!("encontrado {m} (pasta de saída do IPED)"))
            .collect();
        motivos.extend(
            listas
                .iter()
                .map(|l| format!("encontrada a lista de itens do IPED: {l}")),
        );
        let (confianca, suporte) = match (marcas.is_empty(), listas.is_empty()) {
            (true, true) => (0.0, Suporte::NaoReconhecido),
            (false, false) => (0.9, Suporte::Parcial),
            (true, false) => (0.7, Suporte::Parcial),
            (false, true) => {
                motivos.push(
                    "sem lista de itens (FileList.csv / Lista de Arquivos.csv) na raiz: \
                     o caso precisa ser exportado em CSV pelo IPED para os itens serem lidos"
                        .into(),
                );
                (0.6, Suporte::Parcial)
            }
        };
        if suporte != Suporte::NaoReconhecido {
            motivos.push(
                "o índice e os bancos internos do IPED não são lidos; entram só no inventário"
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
        let mut plano = PlanoImportacao {
            tipo_extracao: Some("caso IPED".into()),
            ..Default::default()
        };
        plano.ferramenta_versao = self.versao_declarada(fonte)?;
        if plano.ferramenta_versao.is_none() {
            plano.observacoes.push(
                "versão do IPED não declarada nos arquivos do caso; não foi inferida do ambiente local"
                    .into(),
            );
        } else if let Some(versao) = &plano.ferramenta_versao {
            plano
                .observacoes
                .push(format!("versão declarada do IPED: {versao}"));
        }
        plano
            .observacoes
            .extend(self.observacoes_configuracao(fonte)?);
        let listas = self.listas(fonte)?;
        for l in &listas {
            plano.observacoes.push(format!(
                "{l} → parser {} (lista de itens do IPED)",
                iped_lista::ID
            ));
            plano.tarefas.push(TarefaParser {
                parser_id: iped_lista::ID.into(),
                arquivos: vec![ArquivoParaParser {
                    papel: iped_lista::PAPEL.into(),
                    caminho: l.clone(),
                }],
            });
        }
        if listas.is_empty() {
            plano.observacoes.push(
                "nenhuma lista de itens do IPED na raiz do caso (FileList.csv, \
                 Lista de Arquivos.csv...): os itens do caso não serão lidos"
                    .into(),
            );
        }
        plano.observacoes.push(format!(
            "{PASTA_IPED}/: índice e configuração do IPED, inventariados com hash mas não interpretados"
        ));

        // Fora de iped/: bancos com nome padrão (ex.: exportados pelo IPED).
        let generico = ImportadorGenerico.planejar(fonte)?;
        let dentro_iped = |c: &CaminhoFonte| {
            c.como_str()
                .split('/')
                .next()
                .is_some_and(|p| p.eq_ignore_ascii_case(PASTA_IPED))
        };
        let fora = generico
            .tarefas
            .into_iter()
            .filter(|t| !t.arquivos.iter().any(|a| dentro_iped(&a.caminho)));
        for t in fora {
            let arquivos: Vec<String> = t.arquivos.iter().map(|a| a.caminho.to_string()).collect();
            plano.observacoes.push(format!(
                "{} → parser {} (nome padrão do Android/WhatsApp)",
                arquivos.join(" + "),
                t.parser_id
            ));
            plano.tarefas.push(t);
        }
        Ok(plano)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fonte::pasta::PastaSomenteLeitura;
    use std::fs;

    const CABECALHO_PT: &str = "\u{feff}\"Nome\";\"Atalho\";\"Tamanho\";\"Ext\";\"Marcador\";\
        \"Categoria\";\"MD5\";\"SHA1\";\"Deletado\";\"Recuperado\";\"Acesso\";\"Modificação\";\
        \"Criação\";\"Caminho\";\"TrackId\"\r\n";

    /// Pasta SINTÉTICA com a estrutura do IPED; os arquivos não têm conteúdo real.
    fn caso(r: &std::path::Path, com_lista: bool) {
        for p in [
            "iped/IPEDConfig.txt",
            "iped/index/segments_1",
            "iped/conf/LocalConfig.txt",
            "iped/lib/x/mmssms.db",
            "Exportados/arquivos/com.android.providers.telephony/databases/mmssms.db",
        ] {
            fs::create_dir_all(r.join(p).parent().unwrap()).unwrap();
            fs::write(r.join(p), b"sintetico").unwrap();
        }
        if com_lista {
            fs::write(r.join("Lista de Arquivos.csv"), CABECALHO_PT).unwrap();
        }
    }

    #[test]
    fn detecta_caso_iped_com_e_sem_lista() {
        let d = tempfile::tempdir().unwrap();
        caso(d.path(), true);
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let det = IpedImporter.detectar(&f).unwrap();
        assert_eq!(det.suporte, Suporte::Parcial);
        assert!(det.confianca > 0.8);
        assert!(det
            .motivos
            .iter()
            .any(|m| m.contains("Lista de Arquivos.csv")));

        let d = tempfile::tempdir().unwrap();
        caso(d.path(), false);
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let det = IpedImporter.detectar(&f).unwrap();
        assert_eq!(det.suporte, Suporte::Parcial);
        assert!(det.motivos.iter().any(|m| m.contains("sem lista de itens")));
    }

    #[test]
    fn nao_reconhece_pasta_comum_nem_csv_qualquer() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("FileList.csv"), "\"a\",\"b\"\r\n").unwrap();
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let det = IpedImporter.detectar(&f).unwrap();
        assert_eq!(det.suporte, Suporte::NaoReconhecido);
        assert_eq!(det.confianca, 0.0);
    }

    #[test]
    fn plano_le_a_lista_e_ignora_bancos_dentro_de_iped() {
        let d = tempfile::tempdir().unwrap();
        caso(d.path(), true);
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let plano = IpedImporter.planejar(&f).unwrap();
        let t: Vec<(String, String)> = plano
            .tarefas
            .iter()
            .map(|t| (t.parser_id.clone(), t.arquivos[0].caminho.to_string()))
            .collect();
        let banco = "Exportados/arquivos/com.android.providers.telephony/databases/mmssms.db";
        assert_eq!(
            t,
            vec![
                ("iped_lista_arquivos".into(), "Lista de Arquivos.csv".into()),
                ("android_sms".into(), banco.into()),
                ("android_mms".into(), banco.into()),
            ]
        );
        assert_eq!(plano.tipo_extracao.as_deref(), Some("caso IPED"));
    }

    #[test]
    fn plano_preserva_versao_e_capacidades_declaradas_sem_inferir() {
        let d = tempfile::tempdir().unwrap();
        caso(d.path(), false);
        fs::write(
            d.path().join("iped/IPEDConfig.txt"),
            "enableHash=true\nenableOCR=false\nenableGraphGeneration=true\nendpoint=https://nao-deve-aparecer\n",
        )
        .unwrap();
        fs::write(d.path().join("iped/version.txt"), "IPED-4.3.1\n").unwrap();
        fs::write(
            d.path().join("iped/conf/OCRConfig.txt"),
            "OCRLanguage=por\n",
        )
        .unwrap();

        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let plano = IpedImporter.planejar(&f).unwrap();
        assert_eq!(plano.ferramenta_versao.as_deref(), Some("IPED-4.3.1"));
        assert!(plano
            .observacoes
            .iter()
            .any(|o| o.contains("enableHash=true") && o.contains("enableOCR=false")));
        assert!(plano
            .observacoes
            .iter()
            .any(|o| o.contains("OCRLanguage=por")));
        assert!(!plano
            .observacoes
            .iter()
            .any(|o| o.contains("endpoint=") || o.contains("nao-deve-aparecer")));
    }

    #[test]
    fn plano_nao_inventa_versao_ausente() {
        let d = tempfile::tempdir().unwrap();
        caso(d.path(), false);
        let f = PastaSomenteLeitura::abrir(d.path()).unwrap();
        let plano = IpedImporter.planejar(&f).unwrap();
        assert_eq!(plano.ferramenta_versao, None);
        assert!(plano
            .observacoes
            .iter()
            .any(|o| o.contains("versão do IPED não declarada")));
    }
}
