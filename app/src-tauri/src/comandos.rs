//! Comandos chamados pelo frontend (`invoke`). Camada fina: trava o estado,
//! delega ao [`Servico`] e converte erros.

use sied_armazenamento::auditoria::{RegistroAuditoria, VerificacaoAuditoria};
use sied_armazenamento::extracoes::{
    ArquivoExtracao, ArtefatoDetalhe, ArtefatoResumo, ExecucaoParser, Extracao, Pagina,
    TotalCategoria,
};
use sied_armazenamento::CasoCatalogado;
use sied_dominio::{CasoId, Dispositivo, DispositivoId, ExtracaoId, NovoCaso, NovoDispositivo};
use sied_ingestao::importador::Deteccao;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};
use crate::servico::{CasoAberto, EstadoImportacao, InfoApp, Servico};

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

// ---------- Importação ----------

#[tauri::command]
pub fn detectar_fonte(estado: State<'_, Estado>, pasta: String) -> ResultadoApp<Vec<Deteccao>> {
    estado.com(|s| s.detectar_fonte(&PathBuf::from(pasta)))
}

#[tauri::command]
pub fn iniciar_importacao(
    estado: State<'_, Estado>,
    dispositivo_id: DispositivoId,
    pasta: String,
    importador: String,
) -> ResultadoApp<EstadoImportacao> {
    estado.com(|s| s.iniciar_importacao(dispositivo_id, &PathBuf::from(pasta), &importador))
}

#[tauri::command]
pub fn estado_importacao(estado: State<'_, Estado>) -> ResultadoApp<Option<EstadoImportacao>> {
    estado.com(|s| Ok(s.estado_importacao()))
}

#[tauri::command]
pub fn cancelar_importacao(estado: State<'_, Estado>) -> ResultadoApp<()> {
    estado.com(|s| {
        s.cancelar_importacao();
        Ok(())
    })
}

#[tauri::command]
pub fn listar_extracoes(estado: State<'_, Estado>) -> ResultadoApp<Vec<Extracao>> {
    estado.com(|s| s.listar_extracoes())
}

#[tauri::command]
pub fn obter_extracao(
    estado: State<'_, Estado>,
    extracao_id: ExtracaoId,
) -> ResultadoApp<Extracao> {
    estado.com(|s| s.obter_extracao(extracao_id))
}

#[tauri::command]
pub fn totais_por_categoria(
    estado: State<'_, Estado>,
    extracao_id: ExtracaoId,
) -> ResultadoApp<Vec<TotalCategoria>> {
    estado.com(|s| s.totais_por_categoria(extracao_id))
}

#[tauri::command]
pub fn listar_arquivos(
    estado: State<'_, Estado>,
    extracao_id: ExtracaoId,
    categoria: Option<String>,
    texto: Option<String>,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<ArquivoExtracao>> {
    estado.com(|s| s.listar_arquivos(extracao_id, categoria, texto, deslocamento, limite))
}

#[tauri::command]
pub fn listar_execucoes(
    estado: State<'_, Estado>,
    extracao_id: ExtracaoId,
) -> ResultadoApp<Vec<ExecucaoParser>> {
    estado.com(|s| s.listar_execucoes(extracao_id))
}

#[tauri::command]
pub fn listar_artefatos(
    estado: State<'_, Estado>,
    extracao_id: ExtracaoId,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<ArtefatoResumo>> {
    estado.com(|s| s.listar_artefatos(extracao_id, deslocamento, limite))
}

#[tauri::command]
pub fn detalhe_artefato(
    estado: State<'_, Estado>,
    artefato_id: i64,
) -> ResultadoApp<ArtefatoDetalhe> {
    estado.com(|s| s.detalhe_artefato(artefato_id))
}
