//! Onde fica uma pasta: disco local ou compartilhamento de rede.
//!
//! O SQLite em modo WAL e a trava do caso só são confiáveis em disco local. Pelo
//! caminho a interface reconhece `\\servidor\pasta` e pastas sincronizadas (OneDrive,
//! Dropbox...), mas não uma unidade de rede mapeada (`Z:`): isso só o sistema
//! operacional sabe.
//!
//! - Windows: `GetDriveTypeW` na raiz da unidade (`DRIVE_REMOTE`).
//! - Linux: tipo do sistema de arquivos do ponto de montagem mais próximo, em
//!   `/proc/self/mountinfo` (NFS, SMB/CIFS, SSHFS...).
//! - Outros: só o caminho UNC.

use std::path::Path;

/// `true` se a pasta (ou, se ela ainda não existe, a pasta existente mais próxima
/// acima dela) está num compartilhamento de rede.
pub fn em_rede(caminho: &Path) -> bool {
    sistema::em_rede(caminho)
}

#[cfg(windows)]
mod sistema {
    use std::path::{Component, Path, Prefix};

    const DRIVE_REMOTE: u32 = 4;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetDriveTypeW(raiz: *const u16) -> u32;
    }

    pub fn em_rede(caminho: &Path) -> bool {
        let Some(Component::Prefix(p)) = caminho.components().next() else {
            return false;
        };
        match p.kind() {
            Prefix::UNC(..) | Prefix::VerbatimUNC(..) => true,
            Prefix::Disk(letra) | Prefix::VerbatimDisk(letra) => {
                let raiz: Vec<u16> = format!("{}:\\", letra as char)
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                // SAFETY: `raiz` é uma cadeia UTF-16 terminada em zero, viva durante a
                // chamada; a função só lê.
                unsafe { GetDriveTypeW(raiz.as_ptr()) == DRIVE_REMOTE }
            }
            _ => false,
        }
    }
}

#[cfg(target_os = "linux")]
mod sistema {
    use std::path::{Path, PathBuf};

    /// Sistemas de arquivos em que o dado está em outra máquina.
    const REMOTOS: &[&str] = &[
        "nfs",
        "nfs4",
        "cifs",
        "smb3",
        "smbfs",
        "fuse.sshfs",
        "afs",
        "ceph",
        "9p",
        "davfs",
        "fuse.davfs2",
        "fuse.rclone",
        "glusterfs",
        "fuse.glusterfs",
    ];

    /// Pasta existente mais próxima (a pasta base de um caso novo pode ainda não existir).
    fn existente(caminho: &Path) -> Option<PathBuf> {
        caminho
            .ancestors()
            .find_map(|a| std::fs::canonicalize(a).ok())
    }

    /// Ponto de montagem e tipo de cada linha do mountinfo (`man 5 proc`): o ponto é o
    /// 5º campo e o tipo vem logo depois do separador `-`.
    pub(super) fn montagens(mountinfo: &str) -> Vec<(PathBuf, String)> {
        mountinfo
            .lines()
            .filter_map(|l| {
                let campos: Vec<&str> = l.split(' ').collect();
                let ponto = campos.get(4)?;
                let sep = campos.iter().position(|c| *c == "-")?;
                let tipo = campos.get(sep + 1)?;
                // Espaços e afins vêm em octal (\040).
                let ponto = ponto
                    .replace("\\040", " ")
                    .replace("\\011", "\t")
                    .replace("\\012", "\n")
                    .replace("\\134", "\\");
                Some((PathBuf::from(ponto), tipo.to_string()))
            })
            .collect()
    }

    pub(super) fn remoto_em(montagens: &[(PathBuf, String)], pasta: &Path) -> bool {
        montagens
            .iter()
            .filter(|(ponto, _)| pasta.starts_with(ponto))
            .max_by_key(|(ponto, _)| ponto.components().count())
            .is_some_and(|(_, tipo)| REMOTOS.contains(&tipo.as_str()))
    }

    pub fn em_rede(caminho: &Path) -> bool {
        let Some(pasta) = existente(caminho) else {
            return false;
        };
        match std::fs::read_to_string("/proc/self/mountinfo") {
            Ok(m) => remoto_em(&montagens(&m), &pasta),
            Err(_) => false,
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod sistema {
    use std::path::Path;

    pub fn em_rede(caminho: &Path) -> bool {
        let c = caminho.to_string_lossy();
        c.starts_with("\\\\") || c.starts_with("//")
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn pasta_temporaria_e_local() {
        let d = tempfile::tempdir().unwrap();
        assert!(!em_rede(d.path()));
        // Pasta que ainda não existe: vale a existente mais próxima.
        assert!(!em_rede(&d.path().join("ainda/nao/existe")));
    }

    #[cfg(windows)]
    #[test]
    fn caminho_unc_e_rede() {
        assert!(em_rede(Path::new(r"\\servidor\casos\IP-1")));
        assert!(!em_rede(Path::new(r"C:\SIED\Casos")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn montagem_mais_proxima_decide() {
        use std::path::PathBuf;
        // Linhas no formato do /proc/self/mountinfo (SINTÉTICAS).
        let m = sistema::montagens(
            "22 1 8:1 / / rw,relatime - ext4 /dev/sda1 rw\n\
             40 22 0:50 / /mnt/casos rw - cifs //servidor/casos rw\n\
             41 40 8:2 / /mnt/casos/local rw - ext4 /dev/sdb1 rw\n\
             42 22 0:51 / /mnt/com\\040espaco rw - nfs4 srv:/x rw\n",
        );
        assert_eq!(m.len(), 4);
        assert_eq!(m[3].0, PathBuf::from("/mnt/com espaco"));
        let r = |p: &str| sistema::remoto_em(&m, Path::new(p));
        assert!(!r("/home/analista/SIED"));
        assert!(r("/mnt/casos/IP-1"));
        assert!(!r("/mnt/casos/local/IP-2"));
        assert!(r("/mnt/com espaco/IP-3"));
        // Prefixo de texto não é prefixo de caminho.
        assert!(!r("/mnt/casosX"));
    }
}
