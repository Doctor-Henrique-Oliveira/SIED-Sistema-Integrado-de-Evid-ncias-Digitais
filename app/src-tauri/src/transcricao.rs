//! Orquestração da transcrição local. Nenhum serviço remoto é chamado.
//!
//! O fluxo suporta sidecars SRT/VTT e um provedor de fala local opt-in. O
//! provedor configurado em `SIED_TRANSCRITOR_LOCAL` recebe somente o caminho
//! físico de um arquivo regular e deve emitir SRT ou VTT em stdout. Assim, um
//! wrapper local de Whisper/Vosk pode ser usado sem enviar conteúdo para a
//! rede. Quando nenhum provedor é configurado, o resultado é `sem_suporte`;
//! texto nunca é inventado a partir de áudio/vídeo.

use sha2::{Digest, Sha256};
use sied_armazenamento::transcricoes::{
    EstadoTranscricao, NovaTranscricao, NovoSegmento, TranscricaoLocal,
};
use sied_documentos::ControleDocumento;
use sied_dominio::CasoId;
use sied_ingestao::fonte::CaminhoFonte;
use std::io::{self, BufRead, BufReader, Cursor, Read};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};
use crate::servico::{LeituraTranscricao, Servico};

pub struct OperacoesTranscricao {
    ativa: Mutex<Option<Arc<Operacao>>>,
}
struct Operacao {
    caso_id: CasoId,
    controle: ControleDocumento,
}
pub struct GuardaTranscricao<'a> {
    gerente: &'a OperacoesTranscricao,
    operacao: Arc<Operacao>,
}

impl OperacoesTranscricao {
    pub const fn novo() -> Self {
        Self {
            ativa: Mutex::new(None),
        }
    }
    pub fn iniciar(&self, caso_id: CasoId) -> ResultadoApp<GuardaTranscricao<'_>> {
        let mut ativa = self.ativa.lock()?;
        if ativa.is_some() {
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "há uma transcrição local em curso; aguarde ou cancele a operação",
            ));
        }
        let op = Arc::new(Operacao {
            caso_id,
            controle: ControleDocumento::novo(),
        });
        *ativa = Some(op.clone());
        Ok(GuardaTranscricao {
            gerente: self,
            operacao: op,
        })
    }
    pub fn cancelar(&self, caso_id: CasoId) -> ResultadoApp<bool> {
        let ativa = self.ativa.lock()?;
        if let Some(op) = ativa.as_ref().filter(|x| x.caso_id == caso_id) {
            op.controle.cancelar();
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
impl GuardaTranscricao<'_> {
    pub fn controle(&self) -> &ControleDocumento {
        &self.operacao.controle
    }
}
impl Drop for GuardaTranscricao<'_> {
    fn drop(&mut self) {
        if let Ok(mut ativa) = self.gerente.ativa.lock() {
            if ativa
                .as_ref()
                .is_some_and(|x| Arc::ptr_eq(x, &self.operacao))
            {
                *ativa = None;
            }
        }
    }
}

pub static TRANSCRICOES: OperacoesTranscricao = OperacoesTranscricao::novo();

fn cancelar(controle: &ControleDocumento) -> ResultadoApp<()> {
    if controle.esta_cancelado() {
        Err(ErroApp::novo(
            CodigoErro::Conflito,
            "transcrição local cancelada; nenhum derivado foi gravado",
        ))
    } else {
        Ok(())
    }
}

fn ler_hash(mut leitor: impl Read, controle: &ControleDocumento) -> ResultadoApp<String> {
    let mut h = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        cancelar(controle)?;
        let n = leitor
            .read(&mut buf)
            .map_err(|e| ErroApp::novo(CodigoErro::Interno, format!("falha ao ler mídia: {e}")))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

fn parse_ms(v: &str) -> Option<i64> {
    let v = v.trim().replace(',', ".");
    let mut p = v.split(':');
    let h: i64 = p.next()?.parse().ok()?;
    let m: i64 = p.next()?.parse().ok()?;
    let s: f64 = p.next()?.parse().ok()?;
    if p.next().is_some() || h < 0 || !(0..60).contains(&m) || !(0.0..60.0).contains(&s) {
        return None;
    }
    Some((h * 3600_000 + m * 60_000) + (s * 1000.0).round() as i64)
}

fn parse_sidecar_reader<R: BufRead>(
    linhas: R,
    controle: &ControleDocumento,
) -> ResultadoApp<Vec<NovoSegmento>> {
    let mut linhas = linhas.lines();
    let mut itens = Vec::new();
    let mut ordem = 0u32;
    let mut pendente = None;
    let mut texto = Vec::new();
    while let Some(linha) = linhas.next() {
        cancelar(controle)?;
        let linha = linha.map_err(|e| {
            ErroApp::novo(CodigoErro::Interno, format!("falha ao ler sidecar: {e}"))
        })?;
        if let Some((a, b)) = linha.split_once("-->") {
            if let (Some(a), Some(b)) = (
                a.trim().split_whitespace().next(),
                b.trim().split_whitespace().next(),
            ) {
                if let (Some(inicio_ms), Some(fim_ms)) = (parse_ms(a), parse_ms(b)) {
                    if let Some((ini, fim)) = pendente.take() {
                        let t = texto.join(" ").trim().to_string();
                        if !t.is_empty() {
                            itens.push(NovoSegmento {
                                ordem,
                                inicio_ms: ini,
                                fim_ms: fim,
                                texto: t,
                                confianca: None,
                                falante: None,
                            });
                            ordem += 1;
                        }
                    }
                    pendente = Some((inicio_ms, fim_ms));
                    texto.clear();
                }
            }
        } else if linha.trim().is_empty() {
            if let Some((ini, fim)) = pendente.take() {
                let t = texto.join(" ").trim().to_string();
                if !t.is_empty() {
                    itens.push(NovoSegmento {
                        ordem,
                        inicio_ms: ini,
                        fim_ms: fim,
                        texto: t,
                        confianca: None,
                        falante: None,
                    });
                    ordem += 1;
                }
            }
            texto.clear();
        } else if !linha.trim().eq_ignore_ascii_case("WEBVTT")
            && linha.chars().all(|c| c.is_ascii_digit()) == false
        {
            texto.push(linha);
        }
    }
    if let Some((ini, fim)) = pendente {
        let t = texto.join(" ").trim().to_string();
        if !t.is_empty() {
            itens.push(NovoSegmento {
                ordem,
                inicio_ms: ini,
                fim_ms: fim,
                texto: t,
                confianca: None,
                falante: None,
            });
        }
    }
    Ok(itens)
}

fn parse_sidecar(
    leitura: &LeituraTranscricao,
    caminho: &CaminhoFonte,
    controle: &ControleDocumento,
) -> ResultadoApp<Vec<NovoSegmento>> {
    let leitor = leitura.abrir_caminho(caminho)?;
    parse_sidecar_reader(BufReader::new(leitor), controle)
}

const LIMITE_SAIDA_PROVEDOR: usize = 8 * 1024 * 1024;

fn ler_limitado(mut leitor: impl Read) -> io::Result<Vec<u8>> {
    let mut saida = Vec::new();
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let n = leitor.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        if saida.len().saturating_add(n) > LIMITE_SAIDA_PROVEDOR {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "saída do provedor local excede o limite de 8 MiB",
            ));
        }
        saida.extend_from_slice(&buffer[..n]);
    }
    Ok(saida)
}

fn provedor_local() -> Option<std::ffi::OsString> {
    std::env::var_os("SIED_TRANSCRITOR_LOCAL").filter(|v| !v.is_empty())
}

fn executar_provedor_configurado(
    leitura: &LeituraTranscricao,
    controle: &ControleDocumento,
) -> ResultadoApp<Option<NovaTranscricao>> {
    let Some(provedor) = provedor_local() else {
        return Ok(None);
    };
    let caminho = leitura.caminho_absoluto().ok_or_else(|| {
        ErroApp::novo(
            CodigoErro::NaoSuportado,
            "o provedor local só pode ler arquivos regulares fora de contêineres",
        )
    })?;
    let mut processo = Command::new(&provedor)
        .arg(&caminho)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            ErroApp::novo(
                CodigoErro::NaoSuportado,
                format!("não foi possível iniciar SIED_TRANSCRITOR_LOCAL: {e}"),
            )
        })?;
    let stdout = processo.stdout.take().ok_or_else(|| {
        ErroApp::novo(
            CodigoErro::Interno,
            "o provedor local não expôs a saída da transcrição",
        )
    })?;
    let leitor = thread::spawn(move || ler_limitado(stdout));
    loop {
        if controle.esta_cancelado() {
            let _ = processo.kill();
            let _ = processo.wait();
            let _ = leitor.join();
            return Err(ErroApp::novo(
                CodigoErro::Conflito,
                "transcrição local cancelada; nenhum derivado foi gravado",
            ));
        }
        match processo.try_wait() {
            Ok(Some(status)) => {
                let bytes = leitor
                    .join()
                    .map_err(|_| {
                        ErroApp::novo(CodigoErro::Interno, "falha ao ler saída do provedor local")
                    })?
                    .map_err(|e| ErroApp::novo(CodigoErro::NaoSuportado, e.to_string()))?;
                if !status.success() {
                    return Err(ErroApp::novo(
                        CodigoErro::NaoSuportado,
                        format!(
                            "o provedor local encerrou com {status}; nenhum derivado foi gravado"
                        ),
                    ));
                }
                let segmentos = parse_sidecar_reader(BufReader::new(Cursor::new(bytes)), controle)?;
                if segmentos.is_empty() {
                    return Err(ErroApp::novo(
                        CodigoErro::NaoSuportado,
                        "o provedor local não retornou segmentos SRT/VTT válidos",
                    ));
                }
                return Ok(Some(NovaTranscricao {
                    caso_id: leitura.caso_id,
                    extracao_id: leitura.extracao_id,
                    arquivo_id: leitura.arquivo_id,
                    caminho: leitura.caminho.to_string(),
                    sha256_origem: leitura.sha256.clone(),
                    estado: EstadoTranscricao::Pronta,
                    idioma: None,
                    motor_id: "local-command-srt".into(),
                    motor_versao: "1".into(),
                    erro: None,
                    segmentos,
                }));
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                let _ = processo.kill();
                let _ = processo.wait();
                let _ = leitor.join();
                return Err(ErroApp::novo(
                    CodigoErro::NaoSuportado,
                    format!("falha ao aguardar o provedor local: {e}"),
                ));
            }
        }
    }
}

fn sidecar_candidates(caminho: &CaminhoFonte) -> Vec<CaminhoFonte> {
    let s = caminho.como_str();
    let base = s.rsplit_once('.').map(|(a, _)| a).unwrap_or(s);
    ["srt", "vtt"]
        .into_iter()
        .filter_map(|ext| CaminhoFonte::novo(&format!("{base}.{ext}")).ok())
        .collect()
}

pub fn executar(
    leitura: LeituraTranscricao,
    controle: &ControleDocumento,
) -> ResultadoApp<NovaTranscricao> {
    cancelar(controle)?;
    let hash = ler_hash(leitura.abrir()?, controle)?;
    if hash != leitura.sha256 {
        return Err(ErroApp::novo(
            CodigoErro::Integridade,
            "o SHA-256 da mídia mudou desde o inventário",
        ));
    }
    if let Some(novo) = executar_provedor_configurado(&leitura, controle)? {
        return Ok(novo);
    }
    for candidato in sidecar_candidates(&leitura.caminho) {
        if let Ok(segmentos) = parse_sidecar(&leitura, &candidato, controle) {
            if segmentos.is_empty() {
                continue;
            }
            return Ok(NovaTranscricao {
                caso_id: leitura.caso_id,
                extracao_id: leitura.extracao_id,
                arquivo_id: leitura.arquivo_id,
                caminho: leitura.caminho.to_string(),
                sha256_origem: leitura.sha256,
                estado: EstadoTranscricao::Pronta,
                idioma: None,
                motor_id: "sidecar-srt-vtt".into(),
                motor_versao: "1".into(),
                erro: None,
                segmentos,
            });
        }
    }
    Ok(NovaTranscricao { caso_id: leitura.caso_id, extracao_id: leitura.extracao_id, arquivo_id: leitura.arquivo_id, caminho: leitura.caminho.to_string(), sha256_origem: leitura.sha256, estado: EstadoTranscricao::SemSuporte, idioma: None, motor_id: "nenhum-provedor-local".into(), motor_versao: "1".into(), erro: Some("nenhum sidecar SRT/VTT encontrado; motor de fala local não está instalado/configurado nesta estação".into()), segmentos: Vec::new() })
}

pub fn persistir(
    servico: &mut Servico,
    novo: NovaTranscricao,
    guarda: &GuardaTranscricao<'_>,
) -> ResultadoApp<TranscricaoLocal> {
    cancelar(guarda.controle())?;
    servico.guardar_transcricao(&novo, guarda.controle())
}

#[cfg(test)]
mod testes {
    use super::parse_ms;
    #[test]
    fn converte_timestamps_srt_e_vtt() {
        assert_eq!(parse_ms("00:01:02,500"), Some(62_500));
        assert_eq!(parse_ms("01:02:03.250"), Some(3_723_250));
        assert_eq!(parse_ms("00:99:00"), None);
    }
}
