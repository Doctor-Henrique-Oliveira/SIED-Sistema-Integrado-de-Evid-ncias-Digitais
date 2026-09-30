//! Leitura, só para leitura, de dentro de contêineres: `.tar` e backup do Android
//! (`.ab`, gerado por `adb backup`).
//!
//! Formato do `.ab`, público no código do Android (AOSP, `BackupManagerService` /
//! `FullBackup`): quatro linhas de texto (`ANDROID BACKUP`, versão, `1`/`0` para
//! compressão, `none` ou `AES-256` para cifra) seguidas de um arquivo tar, comprimido com
//! zlib quando a terceira linha é `1`. Backups cifrados não são abertos (exigem a senha).
//!
//! O tar é lido em sequência, sem extrair nada para o disco: POSIX ustar, com nomes
//! longos do GNU (`L`) e cabeçalhos pax (`x`, campos `path` e `size`). Links (`1`, `2`)
//! e dispositivos não são seguidos. O conteúdo de cada item é entregue como fluxo.

use chrono::{DateTime, SecondsFormat};
use flate2::read::ZlibDecoder;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

/// Tipo de contêiner reconhecido pelo nome do arquivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoConteiner {
    Tar,
    BackupAndroid,
}

pub fn tipo_por_nome(nome: &str) -> Option<TipoConteiner> {
    let n = nome.to_ascii_lowercase();
    if n.ends_with(".tar") {
        Some(TipoConteiner::Tar)
    } else if n.ends_with(".ab") {
        Some(TipoConteiner::BackupAndroid)
    } else {
        None
    }
}

/// Cabeçalho de texto de um backup `.ab`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CabecalhoBackup {
    pub versao: u32,
    pub comprimido: bool,
    /// `none` ou o nome da cifra (ex.: `AES-256`).
    pub cifra: String,
}

impl CabecalhoBackup {
    pub fn cifrado(&self) -> bool {
        !self.cifra.eq_ignore_ascii_case("none")
    }
}

const MAGICA_AB: &str = "ANDROID BACKUP";

fn linha(r: &mut impl BufRead) -> io::Result<String> {
    let mut v = Vec::new();
    // Linhas do cabeçalho são curtas; um limite evita ler um arquivo inteiro sem '\n'.
    r.take(64).read_until(b'\n', &mut v)?;
    if v.last() != Some(&b'\n') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "cabeçalho de backup .ab incompleto",
        ));
    }
    v.pop();
    String::from_utf8(v)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "cabeçalho .ab não é texto"))
}

/// Lê as quatro linhas do cabeçalho. `Ok(None)` se não for um backup do Android.
pub fn ler_cabecalho_backup(r: &mut impl BufRead) -> io::Result<Option<CabecalhoBackup>> {
    match linha(r) {
        Ok(m) if m == MAGICA_AB => {}
        Ok(_) => return Ok(None),
        Err(e) if e.kind() == io::ErrorKind::InvalidData => return Ok(None),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let versao = linha(r)?
        .trim()
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "versão do backup .ab"))?;
    let comprimido = match linha(r)?.trim() {
        "1" => true,
        "0" => false,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "indicador de compressão do backup .ab",
            ))
        }
    };
    let cifra = linha(r)?.trim().to_string();
    Ok(Some(CabecalhoBackup {
        versao,
        comprimido,
        cifra,
    }))
}

/// Cabeçalho de um backup `.ab` no disco (lê só as primeiras linhas).
pub fn cabecalho_backup_do_arquivo(p: &Path) -> io::Result<Option<CabecalhoBackup>> {
    ler_cabecalho_backup(&mut BufReader::new(File::open(p)?))
}

/// Fluxo tar de um contêiner no disco.
pub fn abrir_tar(p: &Path, tipo: TipoConteiner) -> io::Result<LeitorTar> {
    let mut r = BufReader::with_capacity(1 << 20, File::open(p)?);
    let fluxo: Box<dyn Read + Send> = match tipo {
        TipoConteiner::Tar => Box::new(r),
        TipoConteiner::BackupAndroid => {
            let c = ler_cabecalho_backup(&mut r)?.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "não é um backup do Android (falta \"ANDROID BACKUP\")",
                )
            })?;
            if c.cifrado() {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!(
                        "backup .ab cifrado ({}): conteúdo não aberto (exige a senha)",
                        c.cifra
                    ),
                ));
            }
            if c.comprimido {
                Box::new(BufReader::with_capacity(1 << 20, ZlibDecoder::new(r)))
            } else {
                Box::new(r)
            }
        }
    };
    Ok(LeitorTar::novo(fluxo))
}

/// Tipo de item do tar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoItem {
    Arquivo,
    Diretorio,
    /// Link, dispositivo, FIFO...: listado como ignorado, nunca seguido.
    Outro(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTar {
    /// Caminho como gravado no tar (sem `./` inicial nem `/` final).
    pub nome: String,
    pub tipo: TipoItem,
    pub tamanho: u64,
    /// Data de modificação (RFC 3339), se válida.
    pub modificado_em: Option<String>,
}

/// Leitor sequencial de um fluxo tar.
pub struct LeitorTar {
    r: Box<dyn Read + Send>,
    /// Bytes do conteúdo do item atual ainda não lidos.
    restante: u64,
    /// Preenchimento até o próximo bloco de 512 bytes.
    enchimento: u64,
    fim: bool,
}

const BLOCO: usize = 512;
/// Nome longo (GNU `L`) ou cabeçalho pax maior que isso não é plausível.
const MAX_METADADO: u64 = 1 << 20;

fn octal(campo: &[u8]) -> io::Result<u64> {
    if campo.first().is_some_and(|b| b & 0x80 != 0) {
        // Base 256 (GNU) para tamanhos grandes.
        let mut v: u64 = u64::from(campo[0] & 0x7f);
        for &b in &campo[1..] {
            v = v
                .checked_mul(256)
                .and_then(|x| x.checked_add(u64::from(b)))
                .ok_or_else(|| invalido("tamanho do tar fora do limite"))?;
        }
        return Ok(v);
    }
    let t: String = campo
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect();
    let t = t.trim();
    if t.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(t, 8).map_err(|_| invalido("campo numérico do tar inválido"))
}

fn invalido(m: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, m.to_string())
}

fn texto(campo: &[u8]) -> String {
    let fim = campo.iter().position(|&b| b == 0).unwrap_or(campo.len());
    String::from_utf8_lossy(&campo[..fim]).into_owned()
}

fn normalizar_nome(n: &str) -> String {
    let n = n.trim_start_matches("./").trim_start_matches('/');
    n.trim_end_matches('/').to_string()
}

impl LeitorTar {
    pub fn novo(r: Box<dyn Read + Send>) -> Self {
        LeitorTar {
            r,
            restante: 0,
            enchimento: 0,
            fim: false,
        }
    }

    fn pular(&mut self, mut n: u64) -> io::Result<()> {
        let mut buf = [0u8; 8192];
        while n > 0 {
            let k = (n as usize).min(buf.len());
            self.r.read_exact(&mut buf[..k])?;
            n -= k as u64;
        }
        Ok(())
    }

    /// Descarta o que falta do item atual.
    pub fn terminar_item(&mut self) -> io::Result<()> {
        let n = self.restante + self.enchimento;
        self.restante = 0;
        self.enchimento = 0;
        self.pular(n)
    }

    fn ler_metadado(&mut self, tamanho: u64) -> io::Result<Vec<u8>> {
        if tamanho > MAX_METADADO {
            return Err(invalido("metadado do tar grande demais"));
        }
        let mut v = vec![0u8; tamanho as usize];
        self.r.read_exact(&mut v)?;
        self.pular((BLOCO as u64 - tamanho % BLOCO as u64) % BLOCO as u64)?;
        Ok(v)
    }

    /// Próximo item (arquivo, pasta ou outro), com o conteúdo pronto para [`Read`].
    pub fn proximo(&mut self) -> io::Result<Option<ItemTar>> {
        self.terminar_item()?;
        let mut nome_longo: Option<String> = None;
        let mut tamanho_pax: Option<u64> = None;
        loop {
            if self.fim {
                return Ok(None);
            }
            let mut h = [0u8; BLOCO];
            match self.r.read_exact(&mut h) {
                Ok(()) => {}
                // Tar sem os blocos zerados do fim: termina aqui.
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    self.fim = true;
                    return Ok(None);
                }
                Err(e) => return Err(e),
            }
            if h.iter().all(|&b| b == 0) {
                self.fim = true;
                return Ok(None);
            }
            let soma: u64 = h
                .iter()
                .enumerate()
                .map(|(i, &b)| {
                    if (148..156).contains(&i) {
                        32
                    } else {
                        u64::from(b)
                    }
                })
                .sum();
            if octal(&h[148..156])? != soma {
                return Err(invalido("soma de verificação do cabeçalho tar não confere"));
            }
            let tipo = h[156];
            let tamanho = tamanho_pax.take().map_or_else(|| octal(&h[124..136]), Ok)?;
            match tipo {
                b'L' => {
                    let v = self.ler_metadado(tamanho)?;
                    nome_longo = Some(texto(&v));
                    continue;
                }
                b'x' => {
                    let v = self.ler_metadado(tamanho)?;
                    for (k, val) in pax(&v) {
                        match k.as_str() {
                            "path" => nome_longo = Some(val),
                            "size" => tamanho_pax = val.parse().ok(),
                            _ => {}
                        }
                    }
                    continue;
                }
                b'g' | b'K' => {
                    self.ler_metadado(tamanho)?;
                    continue;
                }
                _ => {}
            }
            let nome = match nome_longo.take() {
                Some(n) => n,
                None => {
                    let base = texto(&h[0..100]);
                    let prefixo = if &h[257..262] == b"ustar" {
                        texto(&h[345..500])
                    } else {
                        String::new()
                    };
                    if prefixo.is_empty() {
                        base
                    } else {
                        format!("{prefixo}/{base}")
                    }
                }
            };
            let tipo_item = match tipo {
                b'0' | 0 | b'7' => TipoItem::Arquivo,
                b'5' => TipoItem::Diretorio,
                outro => TipoItem::Outro(outro),
            };
            // Só arquivos comuns têm conteúdo no tar.
            let conteudo = if tipo_item == TipoItem::Arquivo {
                tamanho
            } else {
                0
            };
            self.restante = conteudo;
            self.enchimento = (BLOCO as u64 - conteudo % BLOCO as u64) % BLOCO as u64;
            let mtime = octal(&h[136..148]).ok().and_then(|s| i64::try_from(s).ok());
            return Ok(Some(ItemTar {
                nome: normalizar_nome(&nome),
                tipo: tipo_item,
                tamanho: conteudo,
                modificado_em: mtime
                    .and_then(|s| DateTime::from_timestamp(s, 0))
                    .map(|d| d.to_rfc3339_opts(SecondsFormat::Millis, true)),
            }));
        }
    }
}

impl Read for LeitorTar {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.restante == 0 {
            return Ok(0);
        }
        let k = (buf.len() as u64).min(self.restante) as usize;
        let n = self.r.read(&mut buf[..k])?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "tar terminou no meio de um item",
            ));
        }
        self.restante -= n as u64;
        Ok(n)
    }
}

/// Registros `"<tamanho> <chave>=<valor>\n"` de um cabeçalho pax.
fn pax(v: &[u8]) -> Vec<(String, String)> {
    let mut r = Vec::new();
    let mut i = 0;
    while i < v.len() {
        let Some(esp) = v[i..].iter().position(|&b| b == b' ') else {
            break;
        };
        let Ok(n) = std::str::from_utf8(&v[i..i + esp])
            .unwrap_or("")
            .parse::<usize>()
        else {
            break;
        };
        if n == 0 || i + n > v.len() {
            break;
        }
        let reg = &v[i + esp + 1..i + n];
        let reg = reg.strip_suffix(b"\n").unwrap_or(reg);
        if let Some(eq) = reg.iter().position(|&b| b == b'=') {
            r.push((
                String::from_utf8_lossy(&reg[..eq]).into_owned(),
                String::from_utf8_lossy(&reg[eq + 1..]).into_owned(),
            ));
        }
        i += n;
    }
    r
}

/// Posição de leitura mantida entre aberturas sucessivas de itens do mesmo contêiner:
/// o inventário lê os itens na ordem da listagem, então cada contêiner é descomprimido
/// uma vez só (voltar atrás reabre do início).
pub struct Cursor {
    pub conteiner: String,
    pub tar: LeitorTar,
    /// Nomes já passados (para achar sempre a primeira ocorrência de um nome).
    pub passados: HashSet<String>,
}

#[cfg(test)]
pub(crate) mod testes {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    /// Cabeçalho ustar mínimo (para montar tar SINTÉTICO nos testes).
    pub fn cabecalho(nome: &str, tamanho: usize, tipo: u8) -> [u8; 512] {
        let mut h = [0u8; 512];
        h[..nome.len()].copy_from_slice(nome.as_bytes());
        h[100..107].copy_from_slice(b"0000644");
        h[124..135].copy_from_slice(format!("{tamanho:011o}").as_bytes());
        h[136..147].copy_from_slice(b"15125334400"); // 2026-01-01 (1767225600 em octal)
        h[156] = tipo;
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        h[148..156].copy_from_slice(b"        ");
        let soma: u32 = h.iter().map(|&b| u32::from(b)).sum();
        h[148..155].copy_from_slice(format!("{soma:06o}\0").as_bytes());
        h
    }

    /// Tar SINTÉTICO com os itens dados (`tipo` b'0' arquivo, b'5' pasta...).
    pub fn tar(itens: &[(&str, &[u8], u8)]) -> Vec<u8> {
        let mut t = Vec::new();
        for (nome, dados, tipo) in itens {
            let dados: &[u8] = if *tipo == b'0' { dados } else { b"" };
            t.extend_from_slice(&cabecalho(nome, dados.len(), *tipo));
            t.extend_from_slice(dados);
            t.resize(t.len().div_ceil(512) * 512, 0);
        }
        t.extend_from_slice(&[0u8; 1024]);
        t
    }

    /// Backup `.ab` SINTÉTICO (versão 5, comprimido, sem cifra) com o tar dado.
    pub fn backup(tar: &[u8], comprimido: bool) -> Vec<u8> {
        let mut v = format!(
            "ANDROID BACKUP\n5\n{}\nnone\n",
            if comprimido { 1 } else { 0 }
        )
        .into_bytes();
        if comprimido {
            let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
            z.write_all(tar).unwrap();
            v.extend(z.finish().unwrap());
        } else {
            v.extend_from_slice(tar);
        }
        v
    }

    fn itens(dados: Vec<u8>) -> Vec<(ItemTar, Vec<u8>)> {
        let mut t = LeitorTar::novo(Box::new(io::Cursor::new(dados)));
        let mut r = Vec::new();
        while let Some(i) = t.proximo().unwrap() {
            let mut c = Vec::new();
            if i.nome.ends_with("b.txt") {
                t.read_to_end(&mut c).unwrap(); // lê só alguns itens; os outros são pulados
            }
            r.push((i, c));
        }
        r
    }

    #[test]
    fn le_ustar_pax_e_nome_longo() {
        let longo = format!("apps/{}/db/x.db", "p".repeat(120));
        let pax_reg = format!("path={longo}\n");
        let pax = format!("{} {pax_reg}", pax_reg.len() + 4); // "NN " + registro
        let mut t = tar(&[
            ("apps/", b"", b'5'),
            ("./apps/a.txt", b"aaaa", b'0'),
            ("apps/b.txt", &[7u8; 700], b'0'),
            ("apps/link", b"", b'2'),
        ]);
        // Acrescenta um item com cabeçalho pax (nome longo) antes dos blocos finais.
        t.truncate(t.len() - 1024);
        t.extend_from_slice(&cabecalho("PaxHeader", pax.len(), b'x'));
        t.extend_from_slice(pax.as_bytes());
        t.resize(t.len().div_ceil(512) * 512, 0);
        t.extend_from_slice(&cabecalho("curto", 3, b'0'));
        t.extend_from_slice(b"xyz");
        t.resize(t.len().div_ceil(512) * 512, 0);
        t.extend_from_slice(&[0u8; 1024]);

        let v = itens(t);
        let nomes: Vec<(&str, TipoItem, u64)> = v
            .iter()
            .map(|(i, _)| (i.nome.as_str(), i.tipo, i.tamanho))
            .collect();
        assert_eq!(
            nomes,
            vec![
                ("apps", TipoItem::Diretorio, 0),
                ("apps/a.txt", TipoItem::Arquivo, 4),
                ("apps/b.txt", TipoItem::Arquivo, 700),
                ("apps/link", TipoItem::Outro(b'2'), 0),
                (longo.as_str(), TipoItem::Arquivo, 3),
            ]
        );
        assert_eq!(v[2].1, vec![7u8; 700]);
        assert_eq!(
            v[1].0.modificado_em.as_deref(),
            Some("2026-01-01T00:00:00.000Z")
        );
    }

    #[test]
    fn cabecalho_do_backup_e_zlib() {
        let t = tar(&[("apps/com.exemplo/db/a.db", b"SQLite format 3\0", b'0')]);
        for comprimido in [true, false] {
            let ab = backup(&t, comprimido);
            let c = ler_cabecalho_backup(&mut &ab[..]).unwrap().unwrap();
            assert_eq!(
                (c.versao, c.comprimido, c.cifrado()),
                (5, comprimido, false)
            );
            let d = tempfile::tempdir().unwrap();
            let p = d.path().join("backup.ab");
            std::fs::write(&p, &ab).unwrap();
            let mut l = abrir_tar(&p, TipoConteiner::BackupAndroid).unwrap();
            let i = l.proximo().unwrap().unwrap();
            assert_eq!(i.nome, "apps/com.exemplo/db/a.db");
            let mut c = Vec::new();
            l.read_to_end(&mut c).unwrap();
            assert_eq!(c, b"SQLite format 3\0");
            assert!(l.proximo().unwrap().is_none());
        }
        assert!(ler_cabecalho_backup(&mut &b"PK\x03\x04"[..])
            .unwrap()
            .is_none());
        let cifrado = b"ANDROID BACKUP\n5\n1\nAES-256\nxxxx";
        let c = ler_cabecalho_backup(&mut &cifrado[..]).unwrap().unwrap();
        assert!(c.cifrado());
    }

    #[test]
    fn recusa_cabecalho_corrompido() {
        let mut t = tar(&[("a.txt", b"abc", b'0')]);
        t[0] = b'b'; // altera o nome sem corrigir a soma
        let mut l = LeitorTar::novo(Box::new(io::Cursor::new(t)));
        assert!(l.proximo().is_err());
    }
}
