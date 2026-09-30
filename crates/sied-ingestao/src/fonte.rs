//! Acesso somente leitura a uma extração.
//!
//! A extração pode ser uma pasta, ou conter contêineres (.tar, .zip, .ab). Todos são
//! vistos pela mesma interface. O contrato **não oferece escrita**: nenhum código de
//! importador ou parser consegue alterar a evidência por meio dele.

use serde::{Deserialize, Serialize};
use std::io::Read;
use ts_rs::TS;

use crate::{ErroIngestao, Resultado};

pub mod conteiner;
pub mod pasta;

/// Separador entre o caminho de um contêiner e o caminho interno.
/// Ex.: `backup.ab!/apps/com.whatsapp/db/msgstore.db`
pub const SEPARADOR_CONTEINER: &str = "!/";

/// Caminho relativo à raiz da extração, sempre com `/`, sem `..` e sem raiz absoluta.
/// É o endereço estável de um arquivo da evidência, independente do disco onde está.
///
/// Recusa também, em qualquer posição, componentes que no Windows mudariam o destino:
/// unidade (`C:`, `c:x`), `.`, e nomes só de pontos e espaços (`...`, `.. `), que o
/// Windows reduz ao remover pontos e espaços finais.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(try_from = "String", into = "String")]
#[ts(export, type = "string")]
pub struct CaminhoFonte(String);

impl CaminhoFonte {
    pub fn novo(caminho: &str) -> Resultado<Self> {
        let normal = caminho.replace('\\', "/");
        let normal = normal.trim_start_matches("./");
        let invalido = normal.is_empty()
            || normal.starts_with('/')
            || normal.split('/').any(componente_invalido);
        if invalido {
            return Err(ErroIngestao::CaminhoInvalido(caminho.into()));
        }
        Ok(CaminhoFonte(normal.to_string()))
    }

    pub fn como_str(&self) -> &str {
        &self.0
    }

    /// Nome do arquivo (último componente).
    pub fn nome(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }

    pub fn extensao(&self) -> Option<String> {
        let nome = self.nome();
        nome.rfind('.')
            .filter(|&i| i > 0)
            .map(|i| nome[i + 1..].to_ascii_lowercase())
    }
}

fn componente_invalido(p: &str) -> bool {
    let b = p.as_bytes();
    p.trim_end_matches(['.', ' ']).is_empty() // "", ".", "..", "...", ".. "
        || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':') // C: C:x
}

impl TryFrom<String> for CaminhoFonte {
    type Error = ErroIngestao;
    fn try_from(s: String) -> Resultado<Self> {
        CaminhoFonte::novo(&s)
    }
}

impl From<CaminhoFonte> for String {
    fn from(c: CaminhoFonte) -> String {
        c.0
    }
}

impl std::fmt::Display for CaminhoFonte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Um item listado pela fonte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EntradaFonte {
    pub caminho: CaminhoFonte,
    pub diretorio: bool,
    #[ts(type = "number")]
    pub tamanho: u64,
    /// Datas como registradas na extração (RFC 3339), quando disponíveis.
    pub modificado_em: Option<String>,
    pub criado_em: Option<String>,
}

pub trait LeitorFonte: Read + Send {}
impl<T: Read + Send> LeitorFonte for T {}

/// Extração vista como árvore de arquivos, somente leitura.
pub trait FonteSomenteLeitura: Send + Sync {
    /// Descrição para logs e auditoria (ex.: caminho absoluto da pasta).
    fn descricao(&self) -> String;

    /// Percorre todos os itens, inclusive os de dentro de contêineres. Deve ser
    /// incremental (não carregar a árvore toda).
    fn listar(&self) -> Box<dyn Iterator<Item = Resultado<EntradaFonte>> + '_>;

    /// Como [`listar`](Self::listar), sem abrir contêineres (rápido, para detecção).
    fn listar_sem_conteineres(&self) -> Box<dyn Iterator<Item = Resultado<EntradaFonte>> + '_> {
        Box::new(
            self.listar().filter(
                |r| !matches!(r, Ok(e) if e.caminho.como_str().contains(SEPARADOR_CONTEINER)),
            ),
        )
    }

    /// Informações de um item, se existir.
    fn entrada(&self, caminho: &CaminhoFonte) -> Resultado<Option<EntradaFonte>>;

    /// Abre um arquivo para leitura.
    fn abrir(&self, caminho: &CaminhoFonte) -> Resultado<Box<dyn LeitorFonte>>;
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn normaliza_caminhos() {
        assert_eq!(
            CaminhoFonte::novo(r"a\b\c.db").unwrap().como_str(),
            "a/b/c.db"
        );
        assert_eq!(CaminhoFonte::novo("./a/b").unwrap().como_str(), "a/b");
        let c = CaminhoFonte::novo("backup.ab!/apps/com.whatsapp/db/msgstore.db").unwrap();
        assert_eq!(c.nome(), "msgstore.db");
        assert_eq!(c.extensao().as_deref(), Some("db"));
    }

    #[test]
    fn aceita_nomes_com_pontos_e_dois_pontos_no_meio() {
        for bom in [
            ".nomedia",
            "a/.thumbnails/x.jpg",
            "a/b.c/..d",
            "a/2026-01-02 10:00:00.jpg",
            "a/arquivo.",
            "a/ab:c",
        ] {
            assert!(CaminhoFonte::novo(bom).is_ok(), "{bom} deveria ser aceito");
        }
    }

    #[test]
    fn recusa_caminhos_que_escapam_da_raiz() {
        for ruim in [
            "",
            "/etc/passwd",
            "C:/Windows",
            r"C:\x",
            "a/../b",
            "a//b",
            "..",
            "a/C:/Windows",
            "a/c:x",
            "a/./b",
            "a/.",
            "a/.../b",
            "a/.. /b",
            "a/ /b",
        ] {
            assert!(
                CaminhoFonte::novo(ruim).is_err(),
                "{ruim} deveria ser recusado"
            );
        }
    }

    #[test]
    fn desserializacao_valida_caminho() {
        assert!(serde_json::from_str::<CaminhoFonte>("\"../x\"").is_err());
        assert!(serde_json::from_str::<CaminhoFonte>("\"a/b\"").is_ok());
    }
}
