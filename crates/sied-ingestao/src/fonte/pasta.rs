//! Extração em uma pasta do disco, lida somente para leitura.
//!
//! Arquivos são abertos com `File::open` (somente leitura). Links simbólicos não são
//! seguidos: um link poderia apontar para fora da extração.

use chrono::{DateTime, SecondsFormat, Utc};
use std::fs::{self, File, Metadata};
use std::io::BufReader;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

use super::{CaminhoFonte, EntradaFonte, FonteSomenteLeitura, LeitorFonte, SEPARADOR_CONTEINER};
use crate::{ErroIngestao, Resultado};

pub struct PastaSomenteLeitura {
    raiz: PathBuf,
}

impl PastaSomenteLeitura {
    pub fn abrir(raiz: &Path) -> Resultado<Self> {
        let raiz = raiz.canonicalize()?;
        if !raiz.is_dir() {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{} não é uma pasta",
                raiz.display()
            )));
        }
        Ok(PastaSomenteLeitura { raiz })
    }

    pub fn raiz(&self) -> &Path {
        &self.raiz
    }

    fn no_disco(&self, caminho: &CaminhoFonte) -> Resultado<PathBuf> {
        if caminho.como_str().contains(SEPARADOR_CONTEINER) {
            return Err(ErroIngestao::NaoImplementado {
                componente: "PastaSomenteLeitura".into(),
                motivo: "leitura dentro de contêineres (.tar, .zip, .ab)".into(),
            });
        }
        // Segunda barreira, com as regras de caminho da plataforma: cada parte tem de ser
        // um nome comum (nunca unidade, raiz, `.` ou `..`), senão o destino sairia da raiz.
        let mut p = self.raiz.clone();
        for parte in caminho.como_str().split('/') {
            let mut c = Path::new(parte).components();
            if !matches!((c.next(), c.next()), (Some(Component::Normal(_)), None)) {
                return Err(ErroIngestao::CaminhoInvalido(caminho.to_string()));
            }
            p.push(parte);
        }
        Ok(p)
    }

    fn relativo(&self, p: &Path) -> Resultado<CaminhoFonte> {
        let rel = p
            .strip_prefix(&self.raiz)
            .map_err(|_| ErroIngestao::CaminhoInvalido(p.display().to_string()))?;
        let texto = rel.to_str().ok_or_else(|| ErroIngestao::ItemIgnorado {
            caminho: rel.to_string_lossy().into(),
            motivo: "nome com caracteres inválidos (não UTF-8)".into(),
        })?;
        // Um nome que o SIED não consegue endereçar com segurança (ex.: pasta "C:" ou
        // "..." numa extração lida no Linux) vira item ignorado, com aviso, em vez de
        // interromper a importação inteira.
        CaminhoFonte::novo(texto).map_err(|_| ErroIngestao::ItemIgnorado {
            caminho: texto.into(),
            motivo: "nome que não pode ser endereçado com segurança \
                     (unidade, \".\", só pontos ou só espaços)"
                .into(),
        })
    }
}

fn data(t: std::io::Result<SystemTime>) -> Option<String> {
    t.ok()
        .map(|t| DateTime::<Utc>::from(t).to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn entrada(caminho: CaminhoFonte, m: &Metadata) -> EntradaFonte {
    EntradaFonte {
        caminho,
        diretorio: m.is_dir(),
        tamanho: if m.is_dir() { 0 } else { m.len() },
        modificado_em: data(m.modified()),
        criado_em: data(m.created()),
    }
}

impl FonteSomenteLeitura for PastaSomenteLeitura {
    fn descricao(&self) -> String {
        self.raiz.display().to_string()
    }

    fn listar(&self) -> Box<dyn Iterator<Item = Resultado<EntradaFonte>> + '_> {
        let iter = WalkDir::new(&self.raiz)
            .follow_links(false)
            .sort_by_file_name()
            .min_depth(1)
            .into_iter()
            .map(move |item| {
                let item = item.map_err(|e| {
                    let caminho = e
                        .path()
                        .and_then(|p| p.strip_prefix(&self.raiz).ok())
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                    ErroIngestao::ItemIgnorado {
                        caminho,
                        motivo: format!("não foi possível ler: {e}"),
                    }
                })?;
                let caminho = self.relativo(item.path())?;
                if item.path_is_symlink() {
                    return Err(ErroIngestao::ItemIgnorado {
                        caminho: caminho.to_string(),
                        motivo: "link simbólico (não seguido)".into(),
                    });
                }
                let m = item.metadata().map_err(|e| ErroIngestao::ItemIgnorado {
                    caminho: caminho.to_string(),
                    motivo: format!("sem metadados: {e}"),
                })?;
                Ok(entrada(caminho, &m))
            });
        Box::new(iter)
    }

    fn entrada(&self, caminho: &CaminhoFonte) -> Resultado<Option<EntradaFonte>> {
        let p = self.no_disco(caminho)?;
        match fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => Ok(None),
            Ok(m) => Ok(Some(entrada(caminho.clone(), &m))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn abrir(&self, caminho: &CaminhoFonte) -> Resultado<Box<dyn LeitorFonte>> {
        let p = self.no_disco(caminho)?;
        if fs::symlink_metadata(&p)?.file_type().is_symlink() {
            return Err(ErroIngestao::ItemIgnorado {
                caminho: caminho.to_string(),
                motivo: "link simbólico (não seguido)".into(),
            });
        }
        Ok(Box::new(BufReader::with_capacity(1 << 20, File::open(p)?)))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn lista_em_ordem_e_le() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("b/c")).unwrap();
        fs::write(dir.path().join("b/c/x.txt"), "abc").unwrap();
        fs::write(dir.path().join("a.txt"), "1").unwrap();
        let f = PastaSomenteLeitura::abrir(dir.path()).unwrap();
        let itens: Vec<_> = f.listar().map(|e| e.unwrap()).collect();
        let nomes: Vec<_> = itens.iter().map(|e| e.caminho.como_str()).collect();
        assert_eq!(nomes, ["a.txt", "b", "b/c", "b/c/x.txt"]);
        assert!(itens[1].diretorio);
        assert_eq!(itens[3].tamanho, 3);
        assert!(itens[3].modificado_em.is_some());

        let c = CaminhoFonte::novo("b/c/x.txt").unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f.abrir(&c).unwrap(), &mut s).unwrap();
        assert_eq!(s, "abc");
        assert!(f
            .entrada(&CaminhoFonte::novo("nao/existe").unwrap())
            .unwrap()
            .is_none());
    }

    #[cfg(unix)]
    #[test]
    fn nao_segue_links() {
        let dir = tempfile::tempdir().unwrap();
        let fora = tempfile::tempdir().unwrap();
        fs::write(fora.path().join("segredo"), "x").unwrap();
        std::os::unix::fs::symlink(fora.path().join("segredo"), dir.path().join("link")).unwrap();
        let f = PastaSomenteLeitura::abrir(dir.path()).unwrap();
        let itens: Vec<_> = f.listar().collect();
        assert!(matches!(itens[0], Err(ErroIngestao::ItemIgnorado { .. })));
        assert!(f.abrir(&CaminhoFonte::novo("link").unwrap()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn nome_perigoso_vira_item_ignorado_sem_interromper() {
        let dir = tempfile::tempdir().unwrap();
        for nome in ["...", " ", "C:"] {
            fs::create_dir_all(dir.path().join(nome)).unwrap();
            fs::write(dir.path().join(nome).join("x.txt"), "x").unwrap();
        }
        fs::write(dir.path().join("bom.txt"), "1").unwrap();
        let f = PastaSomenteLeitura::abrir(dir.path()).unwrap();
        let mut bons = Vec::new();
        let mut ignorados = 0;
        for item in f.listar() {
            match item {
                Ok(e) => bons.push(e.caminho.to_string()),
                Err(ErroIngestao::ItemIgnorado { .. }) => ignorados += 1,
                Err(e) => panic!("erro inesperado: {e}"),
            }
        }
        assert_eq!(bons, ["bom.txt"]);
        assert_eq!(ignorados, 6, "3 pastas e 3 arquivos dentro delas");
    }

    /// Mesmo que um caminho perigoso chegasse por outro meio (banco do caso adulterado),
    /// a leitura recusaria em vez de abrir fora da raiz.
    #[test]
    fn leitura_recusa_parte_que_nao_e_nome_comum() {
        let dir = tempfile::tempdir().unwrap();
        let f = PastaSomenteLeitura::abrir(dir.path()).unwrap();
        let forjado = CaminhoFonte("a/../../fora".into());
        assert!(matches!(
            f.abrir(&forjado),
            Err(ErroIngestao::CaminhoInvalido(_))
        ));
        assert!(matches!(
            f.entrada(&forjado),
            Err(ErroIngestao::CaminhoInvalido(_))
        ));
    }
}
