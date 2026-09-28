//! Comandos chamados pelo frontend (`invoke`). Camada fina: trava o estado,
//! delega ao [`Servico`] e converte erros.

use sied_armazenamento::auditoria::{RegistroAuditoria, VerificacaoAuditoria};
use sied_armazenamento::CasoCatalogado;
use sied_dominio::{CasoId, Dispositivo, NovoCaso, NovoDispositivo};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};
use crate::servico::{CasoAberto, InfoApp, Servico};

pub struct Estado {
    pub servico: Mutex<Option<Servico>>,
    /// Motivo da falha ao iniciar (ex.: banco global de versão mais nova).
    pub falha_inicial: Option<String>,
}

impl Estado {
    fn com<T>(&self, f: impl FnOnce(&mut Servico) -> ResultadoApp<T>) -> ResultadoApp<T> {
        let mut guarda = self.servico.lock()?;
        match guarda.as_mut() {
            Some(s) => f(s),
            None => Err(ErroApp::novo(
                CodigoErro::Interno,
                self.falha_inicial
                    .clone()
                    .unwrap_or_else(|| "SIED não iniciado".into()),
            )),
        }
    }
}

#[tauri::command]
pub fn info_app(estado: State<'_, Estado>) -> ResultadoApp<InfoApp> {
    estado.com(|s| s.info())
}

#[tauri::command]
pub fn listar_casos(estado: State<'_, Estado>) -> ResultadoApp<Vec<CasoCatalogado>> {
    estado.com(|s| s.listar_casos())
}

#[tauri::command]
pub fn criar_caso(
    estado: State<'_, Estado>,
    dados: NovoCaso,
    pasta_base: Option<String>,
) -> ResultadoApp<CasoAberto> {
    estado.com(|s| s.criar_caso(dados, pasta_base))
}

#[tauri::command]
pub fn abrir_caso(estado: State<'_, Estado>, caso_id: CasoId) -> ResultadoApp<CasoAberto> {
    estado.com(|s| s.abrir_caso(caso_id))
}

#[tauri::command]
pub fn abrir_pasta_caso(estado: State<'_, Estado>, pasta: String) -> ResultadoApp<CasoAberto> {
    estado.com(|s| s.abrir_pasta(&PathBuf::from(pasta)))
}

#[tauri::command]
pub fn fechar_caso(estado: State<'_, Estado>) -> ResultadoApp<()> {
    estado.com(|s| s.fechar_caso())
}

#[tauri::command]
pub fn caso_atual(estado: State<'_, Estado>) -> ResultadoApp<Option<CasoAberto>> {
    estado.com(|s| Ok(s.caso_atual()))
}

#[tauri::command]
pub fn cadastrar_dispositivo(
    estado: State<'_, Estado>,
    dados: NovoDispositivo,
) -> ResultadoApp<Dispositivo> {
    estado.com(|s| s.cadastrar_dispositivo(dados))
}

#[tauri::command]
pub fn listar_dispositivos(estado: State<'_, Estado>) -> ResultadoApp<Vec<Dispositivo>> {
    estado.com(|s| s.listar_dispositivos())
}

#[tauri::command]
pub fn listar_auditoria(
    estado: State<'_, Estado>,
    limite: i64,
) -> ResultadoApp<Vec<RegistroAuditoria>> {
    estado.com(|s| s.listar_auditoria(limite))
}

#[tauri::command]
pub fn verificar_auditoria(estado: State<'_, Estado>) -> ResultadoApp<VerificacaoAuditoria> {
    estado.com(|s| s.verificar_auditoria())
}
