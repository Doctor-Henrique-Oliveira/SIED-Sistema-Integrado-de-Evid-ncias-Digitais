//! Extração em uma pasta do disco, lida somente para leitura.
//!
//! Arquivos são abertos com `File::open` (somente leitura). Links simbólicos não são
//! seguidos: um link poderia apontar para fora da extração.
//!
//! Contêineres (`.tar`, `.zip` e backup `.ab` do Android sem cifra) são lidos sem tocar na
//! origem: a listagem traz, logo depois do contêiner, os itens de dentro dele com o endereço
//! `contêiner!/caminho/interno` (ver [`super::conteiner`]). ZIPs só materializam o item
//! solicitado em temporário derivado, com limite de expansão; contêiner dentro de contêiner
//! não é aberto.

use chrono::{DateTime, SecondsFormat, Utc};
use std::collections::HashSet;
use std::fs::{self, File, Metadata};
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use walkdir::WalkDir;

use super::conteiner::{self, Cursor, ItemZip, LeitorTar, TipoConteiner, TipoItem};
use super::{CaminhoFonte, EntradaFonte, FonteSomenteLeitura, LeitorFonte, SEPARADOR_CONTEINER};
use crate::{ErroIngestao, Resultado};

pub struct PastaSomenteLeitura {
    raiz: PathBuf,
    /// Onde parou a leitura do último contêiner aberto (ver [`Cursor`]).
    cursor: Arc<Mutex<Option<Cursor>>>,
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
        Ok(PastaSomenteLeitura {
            raiz,
            cursor: Arc::default(),
        })
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

    /// Separa `contêiner!/interno`: caminho do contêiner no disco, tipo e nome interno.
    fn dentro(
        &self,
        caminho: &CaminhoFonte,
    ) -> Resultado<Option<(PathBuf, TipoConteiner, String)>> {
        let Some((cont, interno)) = caminho.como_str().split_once(SEPARADOR_CONTEINER) else {
            return Ok(None);
        };
        let c = CaminhoFonte::novo(cont)?;
        let tipo = conteiner::tipo_por_nome(c.nome())
            .ok_or_else(|| ErroIngestao::CaminhoInvalido(caminho.to_string()))?;
        if interno.is_empty() || interno.contains(SEPARADOR_CONTEINER) {
            return Err(ErroIngestao::CaminhoInvalido(caminho.to_string()));
        }
        let p = self.no_disco(&c)?;
        if fs::symlink_metadata(&p)?.file_type().is_symlink() {
            return Err(ErroIngestao::ItemIgnorado {
                caminho: c.to_string(),
                motivo: "link simbólico (não seguido)".into(),
            });
        }
        Ok(Some((p, tipo, interno.to_string())))
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

impl PastaSomenteLeitura {
    fn percorrer(&self) -> walkdir::IntoIter {
        WalkDir::new(&self.raiz)
            .follow_links(false)
            .sort_by_file_name()
            .min_depth(1)
            .into_iter()
    }

    /// Item de pasta (fora de contêiner) da listagem.
    fn item_da_pasta(&self, item: walkdir::Result<walkdir::DirEntry>) -> Resultado<EntradaFonte> {
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
    }

    /// Abre um item de dentro de um contêiner, continuando de onde a última leitura do
    /// mesmo contêiner parou quando o item está mais à frente.
    fn abrir_dentro(
        &self,
        caminho: &CaminhoFonte,
        disco: &Path,
        tipo: TipoConteiner,
        interno: &str,
    ) -> Resultado<Box<dyn LeitorFonte>> {
        if tipo == TipoConteiner::Zip {
            return Ok(Box::new(conteiner::abrir_zip_item(disco, interno)?));
        }
        let chave = disco.display().to_string();
        let anterior = self.cursor.lock().unwrap_or_else(|e| e.into_inner()).take();
        let mut cur = match anterior {
            Some(c) if c.conteiner == chave && !c.passados.contains(interno) => c,
            _ => Cursor {
                conteiner: chave,
                tar: conteiner::abrir_tar(disco, tipo)?,
                passados: HashSet::new(),
            },
        };
        while let Some(i) = cur.tar.proximo()? {
            let primeiro = cur.passados.insert(i.nome.clone());
            if i.nome == interno && primeiro {
                if i.tipo != TipoItem::Arquivo {
                    return Err(ErroIngestao::ItemIgnorado {
                        caminho: caminho.to_string(),
                        motivo: "não é um arquivo comum dentro do contêiner".into(),
                    });
                }
                return Ok(Box::new(ItemDoConteiner {
                    cursor: Some(cur),
                    dono: Arc::clone(&self.cursor),
                }));
            }
        }
        Err(ErroIngestao::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{caminho}: item não encontrado dentro do contêiner"),
        )))
    }
}

/// Conteúdo de um item de contêiner. Ao terminar, devolve a posição de leitura para a
/// fonte, para o próximo item não exigir recomeçar do início.
struct ItemDoConteiner {
    cursor: Option<Cursor>,
    dono: Arc<Mutex<Option<Cursor>>>,
}

impl Read for ItemDoConteiner {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.cursor.as_mut() {
            Some(c) => c.tar.read(buf),
            None => Ok(0),
        }
    }
}

impl Drop for ItemDoConteiner {
    fn drop(&mut self) {
        if let Some(mut c) = self.cursor.take() {
            if c.tar.terminar_item().is_ok() {
                *self.dono.lock().unwrap_or_else(|e| e.into_inner()) = Some(c);
            }
        }
    }
}

/// Contêiner sendo percorrido pela listagem.
struct Expansao {
    conteiner: CaminhoFonte,
    tar: Option<LeitorTar>,
    zip: Option<(Vec<ItemZip>, usize)>,
    /// Motivo de não abrir (ex.: backup cifrado), avisado uma vez.
    recusa: Option<String>,
    vistos: HashSet<String>,
}

struct Listagem<'a> {
    fonte: &'a PastaSomenteLeitura,
    walk: walkdir::IntoIter,
    expansao: Option<Expansao>,
    expandir: bool,
}

impl Listagem<'_> {
    fn preparar_expansao(&self, e: &EntradaFonte) -> Option<Expansao> {
        if e.diretorio || e.caminho.como_str().contains(SEPARADOR_CONTEINER) {
            return None;
        }
        let tipo = conteiner::tipo_por_nome(e.caminho.nome())?;
        let disco = self.fonte.no_disco(&e.caminho).ok()?;
        let mut x = Expansao {
            conteiner: e.caminho.clone(),
            tar: None,
            zip: None,
            recusa: None,
            vistos: HashSet::new(),
        };
        if tipo == TipoConteiner::BackupAndroid {
            match conteiner::cabecalho_backup_do_arquivo(&disco) {
                // Extensão .ab sem o cabeçalho de backup: arquivo comum.
                Ok(None) => return None,
                Ok(Some(c)) if c.cifrado() => {
                    x.recusa = Some(format!(
                        "backup .ab cifrado ({}): inventariado com hash, conteúdo não aberto \
                         (exige a senha)",
                        c.cifra
                    ));
                    return Some(x);
                }
                Ok(Some(_)) => {}
                Err(err) => {
                    x.recusa = Some(format!("cabeçalho do backup .ab ilegível: {err}"));
                    return Some(x);
                }
            }
        }
        if tipo == TipoConteiner::Zip {
            match conteiner::listar_zip(&disco) {
                Ok(itens) => x.zip = Some((itens, 0)),
                Err(err) => x.recusa = Some(format!("ZIP não aberto: {err}")),
            }
        } else {
            match conteiner::abrir_tar(&disco, tipo) {
                Ok(t) => x.tar = Some(t),
                Err(err) => x.recusa = Some(format!("contêiner não aberto: {err}")),
            }
        }
        Some(x)
    }

    /// Próximo item de dentro do contêiner; `None` quando ele termina.
    fn proximo_interno(x: &mut Expansao) -> Option<Resultado<EntradaFonte>> {
        let ignorado =
            |caminho: String, motivo: String| ErroIngestao::ItemIgnorado { caminho, motivo };
        if let Some(m) = x.recusa.take() {
            return Some(Err(ignorado(x.conteiner.to_string(), m)));
        }
        if x.zip.is_some() {
            let item = {
                let (itens, indice) = x.zip.as_mut().expect("zip presente");
                let item = itens.get(*indice).cloned();
                *indice += 1;
                item
            };
            let Some(item) = item else {
                x.zip = None;
                return None;
            };
            let texto = format!("{}{SEPARADOR_CONTEINER}{}", x.conteiner, item.nome);
            let Ok(caminho) = CaminhoFonte::novo(&texto) else {
                return Some(Err(ignorado(
                    texto,
                    "nome interno que não pode ser endereçado com segurança".into(),
                )));
            };
            return Some(match item.tipo {
                TipoItem::Outro(t) => Err(ignorado(
                    texto,
                    format!("item especial do ZIP (tipo '{}'), não seguido", t as char),
                )),
                tipo => Ok(EntradaFonte {
                    caminho,
                    diretorio: tipo == TipoItem::Diretorio,
                    tamanho: item.tamanho,
                    modificado_em: None,
                    criado_em: None,
                }),
            });
        }
        let tar = x.tar.as_mut()?;
        loop {
            let item = match tar.proximo() {
                Ok(Some(i)) => i,
                Ok(None) => return None,
                Err(err) => {
                    x.tar = None;
                    return Some(Err(ignorado(
                        x.conteiner.to_string(),
                        format!(
                            "contêiner lido só até um ponto (o resto não foi inventariado): {err}"
                        ),
                    )));
                }
            };
            if item.nome.is_empty() {
                continue;
            }
            let texto = format!("{}{SEPARADOR_CONTEINER}{}", x.conteiner, item.nome);
            if item.nome.contains(SEPARADOR_CONTEINER) {
                return Some(Err(ignorado(
                    texto,
                    "nome interno com \"!/\" (ambíguo com o separador do SIED)".into(),
                )));
            }
            let Ok(caminho) = CaminhoFonte::novo(&texto) else {
                return Some(Err(ignorado(
                    texto,
                    "nome interno que não pode ser endereçado com segurança".into(),
                )));
            };
            if !x.vistos.insert(item.nome.clone()) {
                return Some(Err(ignorado(
                    texto,
                    "nome repetido no contêiner: só a primeira ocorrência é inventariada".into(),
                )));
            }
            return Some(match item.tipo {
                TipoItem::Outro(t) => Err(ignorado(
                    texto,
                    format!(
                        "link ou item especial do tar (tipo '{}'), não seguido",
                        t as char
                    ),
                )),
                tipo => Ok(EntradaFonte {
                    caminho,
                    diretorio: tipo == TipoItem::Diretorio,
                    tamanho: item.tamanho,
                    modificado_em: item.modificado_em,
                    criado_em: None,
                }),
            });
        }
    }
}

impl Iterator for Listagem<'_> {
    type Item = Resultado<EntradaFonte>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(x) = self.expansao.as_mut() {
            match Self::proximo_interno(x) {
                Some(r) => return Some(r),
                None => self.expansao = None,
            }
        }
        let r = self.fonte.item_da_pasta(self.walk.next()?);
        if self.expandir {
            if let Ok(e) = &r {
                self.expansao = self.preparar_expansao(e);
            }
        }
        Some(r)
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
        Box::new(Listagem {
            fonte: self,
            walk: self.percorrer(),
            expansao: None,
            expandir: true,
        })
    }

    fn listar_sem_conteineres(&self) -> Box<dyn Iterator<Item = Resultado<EntradaFonte>> + '_> {
        Box::new(Listagem {
            fonte: self,
            walk: self.percorrer(),
            expansao: None,
            expandir: false,
        })
    }

    fn entrada(&self, caminho: &CaminhoFonte) -> Resultado<Option<EntradaFonte>> {
        if let Some((p, tipo, interno)) = self.dentro(caminho)? {
            if tipo == TipoConteiner::Zip {
                let itens = conteiner::listar_zip(&p).map_err(ErroIngestao::Io)?;
                return Ok(itens
                    .into_iter()
                    .find(|i| i.nome == interno)
                    .map(|i| EntradaFonte {
                        caminho: caminho.clone(),
                        diretorio: i.tipo == TipoItem::Diretorio,
                        tamanho: i.tamanho,
                        modificado_em: None,
                        criado_em: None,
                    }));
            }
            let mut tar = conteiner::abrir_tar(&p, tipo)?;
            while let Some(i) = tar.proximo()? {
                if i.nome == interno {
                    return Ok(Some(EntradaFonte {
                        caminho: caminho.clone(),
                        diretorio: i.tipo == TipoItem::Diretorio,
                        tamanho: i.tamanho,
                        modificado_em: i.modificado_em,
                        criado_em: None,
                    }));
                }
            }
            return Ok(None);
        }
        let p = self.no_disco(caminho)?;
        match fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => Ok(None),
            Ok(m) => Ok(Some(entrada(caminho.clone(), &m))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn abrir(&self, caminho: &CaminhoFonte) -> Resultado<Box<dyn LeitorFonte>> {
        if let Some((p, tipo, interno)) = self.dentro(caminho)? {
            return self.abrir_dentro(caminho, &p, tipo, &interno);
        }
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

    fn ler(f: &PastaSomenteLeitura, c: &str) -> Vec<u8> {
        let mut v = Vec::new();
        f.abrir(&CaminhoFonte::novo(c).unwrap())
            .unwrap()
            .read_to_end(&mut v)
            .unwrap();
        v
    }

    /// Backup `.ab` e `.tar` SINTÉTICOS: itens listados logo depois do contêiner e lidos
    /// em qualquer ordem, sem nada gravado na pasta.
    #[test]
    fn lista_e_le_dentro_de_conteineres() {
        use crate::fonte::conteiner::testes::{backup, tar};
        let dir = tempfile::tempdir().unwrap();
        let t = tar(&[
            ("apps/com.exemplo/db/", b"", b'5'),
            ("apps/com.exemplo/db/a.db", b"banco A", b'0'),
            ("apps/com.exemplo/f/b.txt", b"arquivo B", b'0'),
            ("apps/com.exemplo/db/a.db", b"repetido", b'0'),
            ("apps/../fora", b"x", b'0'),
            ("apps/link", b"", b'2'),
        ]);
        fs::write(dir.path().join("backup.ab"), backup(&t, true)).unwrap();
        fs::write(dir.path().join("pasta.tar"), tar(&[("x.txt", b"X", b'0')])).unwrap();
        fs::write(dir.path().join("falso.ab"), "não é backup").unwrap();
        let f = PastaSomenteLeitura::abrir(dir.path()).unwrap();
        let mut ok = Vec::new();
        let mut ignorados = Vec::new();
        for r in f.listar() {
            match r {
                Ok(e) => ok.push((e.caminho.to_string(), e.diretorio, e.tamanho)),
                Err(ErroIngestao::ItemIgnorado { caminho, .. }) => ignorados.push(caminho),
                Err(e) => panic!("{e}"),
            }
        }
        let tam_ab = fs::metadata(dir.path().join("backup.ab")).unwrap().len();
        let tam_tar = fs::metadata(dir.path().join("pasta.tar")).unwrap().len();
        assert_eq!(
            ok,
            vec![
                ("backup.ab".into(), false, tam_ab),
                ("backup.ab!/apps/com.exemplo/db".into(), true, 0),
                ("backup.ab!/apps/com.exemplo/db/a.db".into(), false, 7),
                ("backup.ab!/apps/com.exemplo/f/b.txt".into(), false, 9),
                ("falso.ab".into(), false, 14),
                ("pasta.tar".into(), false, tam_tar),
                ("pasta.tar!/x.txt".into(), false, 1),
            ]
        );
        assert_eq!(
            ignorados,
            vec![
                "backup.ab!/apps/com.exemplo/db/a.db",
                "backup.ab!/apps/../fora",
                "backup.ab!/apps/link",
            ]
        );
        assert_eq!(f.listar_sem_conteineres().count(), 3);

        // Em ordem, fora de ordem e de volta: sempre a primeira ocorrência.
        assert_eq!(ler(&f, "backup.ab!/apps/com.exemplo/db/a.db"), b"banco A");
        assert_eq!(ler(&f, "backup.ab!/apps/com.exemplo/f/b.txt"), b"arquivo B");
        assert_eq!(ler(&f, "backup.ab!/apps/com.exemplo/db/a.db"), b"banco A");
        assert_eq!(ler(&f, "pasta.tar!/x.txt"), b"X");
        // Leitura parcial devolve a posição sem quebrar a seguinte.
        {
            let mut l = f
                .abrir(&CaminhoFonte::novo("backup.ab!/apps/com.exemplo/db/a.db").unwrap())
                .unwrap();
            let mut um = [0u8; 1];
            l.read_exact(&mut um).unwrap();
        }
        assert_eq!(ler(&f, "backup.ab!/apps/com.exemplo/f/b.txt"), b"arquivo B");
        let e = f
            .entrada(&CaminhoFonte::novo("backup.ab!/apps/com.exemplo/f/b.txt").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(e.tamanho, 9);
        assert!(f
            .abrir(&CaminhoFonte::novo("backup.ab!/nao/existe").unwrap())
            .is_err());
        assert!(f
            .abrir(&CaminhoFonte::novo("backup.ab!/apps/com.exemplo/db").unwrap())
            .is_err());
    }

    #[test]
    fn lista_e_le_dentro_de_zip() {
        let dir = tempfile::tempdir().unwrap();
        let zip = conteiner::testes::zip_sintetico();
        let caminho = dir.path().join("extracao.zip");
        fs::write(&caminho, zip).unwrap();
        let f = PastaSomenteLeitura::abrir(dir.path()).unwrap();
        let itens: Vec<_> = f.listar().map(|e| e.unwrap()).collect();
        let nomes: Vec<_> = itens.iter().map(|e| e.caminho.to_string()).collect();
        assert!(nomes.iter().any(|n| n == "extracao.zip"));
        assert!(nomes
            .iter()
            .any(|n| n == "extracao.zip!/apps/com.exemplo/db/msgstore.db"));
        let c = CaminhoFonte::novo("extracao.zip!/apps/com.exemplo/db/msgstore.db").unwrap();
        let mut s = Vec::new();
        f.abrir(&c).unwrap().read_to_end(&mut s).unwrap();
        assert_eq!(s, b"SQLite format 3\0");
    }

    #[test]
    fn backup_cifrado_e_inventariado_sem_abrir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("cifrado.ab"),
            b"ANDROID BACKUP\n5\n1\nAES-256\nsal-e-dados-sinteticos",
        )
        .unwrap();
        let f = PastaSomenteLeitura::abrir(dir.path()).unwrap();
        let r: Vec<_> = f.listar().collect();
        assert_eq!(r.len(), 2);
        assert!(matches!(&r[0], Ok(e) if e.caminho.como_str() == "cifrado.ab"));
        assert!(
            matches!(&r[1], Err(ErroIngestao::ItemIgnorado { caminho, motivo })
            if caminho == "cifrado.ab" && motivo.contains("cifrado"))
        );
    }
}
