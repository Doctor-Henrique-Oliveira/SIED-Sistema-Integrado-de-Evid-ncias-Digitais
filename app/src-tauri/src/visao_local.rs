//! Adaptador opt-in para classificação/detecção visual local.
//!
//! O SIED não instala nem escolhe um modelo neural. Quando
//! `SIED_VISAO_LOCAL` está configurado, o executável recebe o caminho de uma
//! imagem regular e devolve JSON em stdout no formato `{ "regioes": [...] }`.
//! Cada item é candidato para revisão humana; não é conclusão pericial.

use serde::{Deserialize, Serialize};
use sied_documentos::ControleDocumento;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use ts_rs::TS;

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};

const LIMITE_SAIDA: usize = 2 * 1024 * 1024;
const LIMITE_REGIOES: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RegiaoVisualLocal {
    pub rotulo: String,
    pub confianca: f64,
    /// Caixa normalizada em relação à largura/altura da imagem (0..=1).
    pub x: f64,
    pub y: f64,
    pub largura: f64,
    pub altura: f64,
    pub mecanismo: String,
    pub versao: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResultadoVisaoLocal {
    pub motor_id: String,
    pub motor_versao: String,
    pub regioes: Vec<RegiaoVisualLocal>,
    pub observacao: String,
}

#[derive(Debug, Deserialize)]
struct Saida {
    regioes: Vec<RegiaoVisualLocal>,
}

fn ler_limitado(mut leitor: impl Read) -> io::Result<Vec<u8>> {
    let mut saida = Vec::new();
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let n = leitor.read(&mut buffer)?;
        if n == 0 {
            return Ok(saida);
        }
        if saida.len().saturating_add(n) > LIMITE_SAIDA {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "saída do motor visual excede 2 MiB",
            ));
        }
        saida.extend_from_slice(&buffer[..n]);
    }
}

fn validar(regioes: &mut [RegiaoVisualLocal]) -> ResultadoApp<()> {
    if regioes.len() > LIMITE_REGIOES {
        return Err(ErroApp::novo(
            CodigoErro::NaoSuportado,
            "o motor visual retornou mais de 256 regiões candidatas",
        ));
    }
    for r in regioes {
        if r.rotulo.trim().is_empty()
            || r.rotulo.len() > 128
            || r.mecanismo.trim().is_empty()
            || r.versao.trim().is_empty()
            || !r.confianca.is_finite()
            || !(0.0..=1.0).contains(&r.confianca)
            || !r.x.is_finite()
            || !r.y.is_finite()
            || !r.largura.is_finite()
            || !r.altura.is_finite()
            || r.x < 0.0
            || r.y < 0.0
            || r.largura <= 0.0
            || r.altura <= 0.0
            || r.x + r.largura > 1.0
            || r.y + r.altura > 1.0
        {
            return Err(ErroApp::novo(
                CodigoErro::NaoSuportado,
                "o motor visual retornou uma região fora dos limites técnicos",
            ));
        }
        r.rotulo = r.rotulo.trim().to_owned();
        r.mecanismo = r.mecanismo.trim().to_owned();
        r.versao = r.versao.trim().to_owned();
    }
    Ok(())
}

pub fn executar(caminho: &Path, controle: &ControleDocumento) -> ResultadoApp<ResultadoVisaoLocal> {
    let provedor = std::env::var_os("SIED_VISAO_LOCAL").filter(|x| !x.is_empty()).ok_or_else(|| {
        ErroApp::novo(
            CodigoErro::NaoSuportado,
            "nenhum motor visual local está configurado; defina SIED_VISAO_LOCAL para um executável offline",
        )
    })?;
    let mut processo = Command::new(provedor)
        .arg(caminho)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            ErroApp::novo(
                CodigoErro::NaoSuportado,
                format!("não foi possível iniciar SIED_VISAO_LOCAL: {e}"),
            )
        })?;
    let stdout = processo
        .stdout
        .take()
        .ok_or_else(|| ErroApp::novo(CodigoErro::Interno, "o motor visual não expôs stdout"))?;
    let leitor = thread::spawn(move || ler_limitado(stdout));
    loop {
        if controle.esta_cancelado() {
            let _ = processo.kill();
            let _ = processo.wait();
            let _ = leitor.join();
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "análise visual local cancelada; nenhum candidato foi publicado",
            ));
        }
        match processo.try_wait() {
            Ok(Some(status)) => {
                let bytes = leitor
                    .join()
                    .map_err(|_| ErroApp::novo(CodigoErro::Interno, "falha ao ler o motor visual"))?
                    .map_err(|e| ErroApp::novo(CodigoErro::NaoSuportado, e.to_string()))?;
                if !status.success() {
                    return Err(ErroApp::novo(
                        CodigoErro::NaoSuportado,
                        format!("o motor visual encerrou com {status}"),
                    ));
                }
                let mut saida: Saida = serde_json::from_slice(&bytes).map_err(|e| {
                    ErroApp::novo(
                        CodigoErro::NaoSuportado,
                        format!("JSON do motor visual inválido: {e}"),
                    )
                })?;
                validar(&mut saida.regioes)?;
                let motor_id = saida
                    .regioes
                    .first()
                    .map(|r| r.mecanismo.clone())
                    .unwrap_or_else(|| "provedor-visual-local".into());
                let motor_versao = saida
                    .regioes
                    .first()
                    .map(|r| r.versao.clone())
                    .unwrap_or_else(|| "desconhecida".into());
                return Ok(ResultadoVisaoLocal { motor_id, motor_versao, regioes: saida.regioes, observacao: "Candidatos derivados localmente; confiança e região não estabelecem identidade, autoria ou conclusão pericial.".into() });
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                let _ = processo.kill();
                let _ = processo.wait();
                let _ = leitor.join();
                return Err(ErroApp::novo(
                    CodigoErro::NaoSuportado,
                    format!("falha ao aguardar o motor visual: {e}"),
                ));
            }
        }
    }
}
