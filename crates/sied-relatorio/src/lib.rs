//! Relatório PDF do SIED.
//!
//! Reúne as evidências selecionadas no caso, confere o hash de cada uma (relendo os
//! arquivos na pasta da extração, somente leitura), diagrama com Typst embutido e
//! grava o PDF em `<pasta do caso>/relatorios/`, com um arquivo `.sha256` ao lado.
//! A geração é registrada no banco do caso e na trilha de auditoria. Tudo offline:
//! o modelo, as fontes e as imagens estão embutidos ou vêm do próprio caso.

pub mod modelo;
pub mod montagem;
pub mod motor;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use sied_armazenamento::relatorios::{NovoRelatorio, RelatorioRegistrado, PASTA_RELATORIOS};
use sied_armazenamento::{BancoCaso, ErroArmazenamento};

pub use modelo::{PedidoRelatorio, SecoesRelatorio};
pub use motor::ErroTypst;

/// Identifica o modelo (vai para o registro do relatório).
pub const MODELO_PADRAO: &str = "padrao-v1";

#[derive(Debug, thiserror::Error)]
pub enum ErroRelatorio {
    #[error(transparent)]
    Armazenamento(#[from] ErroArmazenamento),
    #[error("a trilha de auditoria do caso não confere ({0}); o relatório não foi gerado")]
    Auditoria(String),
    #[error(transparent)]
    Typst(#[from] ErroTypst),
    #[error("não foi possível gravar o relatório: {0}")]
    Io(#[from] std::io::Error),
}

/// Gera o relatório do caso aberto e o registra. Devolve o registro gravado.
pub fn gerar(
    banco: &mut BancoCaso,
    pedido: PedidoRelatorio,
) -> Result<RelatorioRegistrado, ErroRelatorio> {
    gerar_em(banco, pedido, Utc::now())
}

/// Como [`gerar`], com o instante da geração informado (testes).
pub fn gerar_em(
    banco: &mut BancoCaso,
    pedido: PedidoRelatorio,
    agora: DateTime<Utc>,
) -> Result<RelatorioRegistrado, ErroRelatorio> {
    // A trilha precisa estar íntegra: o relatório cita o último hash dela.
    let auditoria = match banco.verificar_auditoria() {
        Ok(v) => v,
        Err(ErroArmazenamento::AuditoriaCorrompida { seq, motivo }) => {
            return Err(ErroRelatorio::Auditoria(format!(
                "registro {seq}: {motivo}"
            )))
        }
        Err(e) => return Err(e.into()),
    };
    let montado = montagem::montar(banco, &pedido, agora, &auditoria)?;
    let dados = serde_json::to_string(&montado.documento).expect("documento serializável");
    let caso = banco.caso();
    let ident = format!("SIED-{}-{}", caso.id, agora.timestamp_millis());
    let pdf = motor::pdf(&dados, &montado.imagens, agora, &ident)?;

    let fuso = montagem::fuso_do_caso(&caso.fuso_horario);
    let pasta = banco.pasta().join(PASTA_RELATORIOS);
    fs::create_dir_all(&pasta)?;
    let base = fuso
        .from_utc_datetime(&agora.naive_utc())
        .format("%Y-%m-%d_%H%M%S")
        .to_string();
    let (nome, destino) = livre(&pasta, &base);
    let sha = hex::encode(Sha256::digest(&pdf));
    gravar(&destino, &pdf)?;
    let arquivo_sha = pasta.join(format!("{nome}.sha256"));
    if let Err(e) = gravar(&arquivo_sha, format!("{sha}  {nome}\n").as_bytes()) {
        let _ = fs::remove_file(&destino);
        return Err(e.into());
    }

    let evidencias = montado
        .conferencias
        .iter()
        .map(|c| {
            json!({
                "numero": c.numero, "evidencia_id": c.evidencia_id, "tipo": c.tipo,
                "hash_no_momento": c.hash_no_momento, "hash_conferido": c.hash_conferido,
                "situacao": c.situacao,
            })
        })
        .collect::<Vec<_>>();
    let novo = NovoRelatorio {
        modelo: MODELO_PADRAO.into(),
        secoes: serde_json::to_value(&montado.documento.secoes).expect("seções serializáveis"),
        evidencias: json!(evidencias),
        gerado_em: agora.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        caminho: format!("{PASTA_RELATORIOS}/{nome}"),
        sha256: sha,
        tamanho: pdf.len() as i64,
        auditoria_ate_seq: auditoria.registros,
        auditoria_hash: auditoria.hash_final,
    };
    match banco.registrar_relatorio(novo) {
        Ok(r) => Ok(r),
        Err(e) => {
            // Sem registro, o PDF não fica: um relatório sempre tem a sua linha na auditoria.
            let _ = fs::remove_file(&destino);
            let _ = fs::remove_file(&arquivo_sha);
            Err(e.into())
        }
    }
}

/// Nome ainda não usado na pasta (`<base>_relatorio.pdf`, `<base>_relatorio-2.pdf`...).
fn livre(pasta: &Path, base: &str) -> (String, PathBuf) {
    let mut n = 1;
    loop {
        let nome = if n == 1 {
            format!("{base}_relatorio.pdf")
        } else {
            format!("{base}_relatorio-{n}.pdf")
        };
        let caminho = pasta.join(&nome);
        if !caminho.exists() {
            return (nome, caminho);
        }
        n += 1;
    }
}

/// Grava num arquivo temporário e renomeia: nunca fica um PDF pela metade. Se algo
/// falhar (disco cheio, sem permissão), o temporário é apagado.
fn gravar(destino: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = destino.with_extension("parcial");
    let r = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, destino)
    })();
    if r.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    r
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn falha_ao_gravar_nao_deixa_temporario() {
        let dir = tempfile::tempdir().unwrap();
        // Um diretório no lugar do destino faz a renomeação falhar.
        let destino = dir.path().join("x_relatorio.pdf");
        fs::create_dir(&destino).unwrap();
        assert!(gravar(&destino, b"%PDF-sintetico").is_err());
        assert!(!dir.path().join("x_relatorio.parcial").exists());

        let bom = dir.path().join("y_relatorio.pdf");
        gravar(&bom, b"%PDF-sintetico").unwrap();
        assert_eq!(fs::read(&bom).unwrap(), b"%PDF-sintetico");
        assert!(!dir.path().join("y_relatorio.parcial").exists());
    }
}
