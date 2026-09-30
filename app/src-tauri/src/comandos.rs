//! Comandos chamados pelo frontend (`invoke`). Camada fina: trava o estado,
//! delega ao [`Servico`] e converte erros.
//!
//! Todo comando roda numa thread de trabalho ([`em_segundo_plano`]), nunca na thread
//! da janela: um comando síncrono rodaria nela e, enquanto outro segura a trava do
//! estado (ex.: um relatório que leva segundos), ficaria esperando ali e a janela
//! congelaria ("Não respondendo" no Windows). As threads de trabalho são as de tarefas
//! bloqueantes do runtime, para que a espera pela trava ou pela decodificação de
//! imagens não ocupe as threads assíncronas do Tauri.

use sied_armazenamento::agenda::{
    Chamada, ContatoAgenda, FiltroChamadas, FiltroContatos, ResumoChamadas, TotalApp,
};
use sied_armazenamento::auditoria::{RegistroAuditoria, VerificacaoAuditoria};
use sied_armazenamento::busca::{AchadoArtefato, FiltroBusca, ResultadoBusca};
use sied_armazenamento::conversas::{AppConversas, Conversa, FiltroConversas, PosicaoMensagem};
use sied_armazenamento::evidencias::{Evidencia, Marcacoes, NovaEvidencia, TextosEvidencia};
use sied_armazenamento::extracoes::{
    ArquivoExtracao, ArtefatoDetalhe, ArtefatoResumo, ExecucaoParser, Extracao, Pagina,
    TotalCategoria,
};
use sied_armazenamento::linha_tempo::{FiltroLinhaTempo, ResumoLinhaTempo};
use sied_armazenamento::relatorios::RelatorioRegistrado;
use sied_armazenamento::resumo::{Identificador, ResumoCaso};
use sied_armazenamento::CasoCatalogado;
use sied_dominio::{CasoId, Dispositivo, DispositivoId, ExtracaoId, NovoCaso, NovoDispositivo};
use sied_ingestao::importador::Deteccao;
use sied_midia::Miniatura;
use sied_relatorio::PedidoRelatorio;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};
use crate::servico::{CasoAberto, EstadoImportacao, InfoApp, Servico};

pub struct Estado {
    pub servico: Mutex<Option<Servico>>,
    /// Motivo da falha ao iniciar (ex.: banco global de versão mais nova).
    pub falha_inicial: Option<String>,
}

impl Estado {
    pub(crate) fn com<T>(
        &self,
        f: impl FnOnce(&mut Servico) -> ResultadoApp<T>,
    ) -> ResultadoApp<T> {
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

/// Executa `f` numa thread de tarefas bloqueantes com acesso ao estado.
async fn em_segundo_plano<T: Send + 'static>(
    app_handle: AppHandle,
    f: impl FnOnce(&Estado) -> ResultadoApp<T> + Send + 'static,
) -> ResultadoApp<T> {
    tauri::async_runtime::spawn_blocking(move || f(&app_handle.state::<Estado>()))
        .await
        .map_err(|e| {
            tracing::error!(erro = %e, "comando interrompido");
            ErroApp::novo(
                CodigoErro::Interno,
                "a operação foi interrompida por uma falha interna; tente de novo",
            )
        })?
}

#[tauri::command]
pub async fn info_app(app_handle: AppHandle) -> ResultadoApp<InfoApp> {
    em_segundo_plano(app_handle, move |estado| estado.com(|s| s.info())).await
}

#[tauri::command]
pub async fn listar_casos(app_handle: AppHandle) -> ResultadoApp<Vec<CasoCatalogado>> {
    em_segundo_plano(app_handle, move |estado| estado.com(|s| s.listar_casos())).await
}

#[tauri::command]
pub async fn criar_caso(
    app_handle: AppHandle,
    dados: NovoCaso,
    pasta_base: Option<String>,
) -> ResultadoApp<CasoAberto> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.criar_caso(dados, pasta_base))
    })
    .await
}

#[tauri::command]
pub async fn abrir_caso(app_handle: AppHandle, caso_id: CasoId) -> ResultadoApp<CasoAberto> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.abrir_caso(caso_id))
    })
    .await
}

#[tauri::command]
pub async fn abrir_pasta_caso(app_handle: AppHandle, pasta: String) -> ResultadoApp<CasoAberto> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.abrir_pasta(&PathBuf::from(pasta)))
    })
    .await
}

#[tauri::command]
pub async fn fechar_caso(app_handle: AppHandle) -> ResultadoApp<()> {
    em_segundo_plano(app_handle, move |estado| estado.com(|s| s.fechar_caso())).await
}

#[tauri::command]
pub async fn caso_atual(app_handle: AppHandle) -> ResultadoApp<Option<CasoAberto>> {
    em_segundo_plano(app_handle, move |estado| estado.com(|s| Ok(s.caso_atual()))).await
}

#[tauri::command]
pub async fn cadastrar_dispositivo(
    app_handle: AppHandle,
    dados: NovoDispositivo,
) -> ResultadoApp<Dispositivo> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.cadastrar_dispositivo(dados))
    })
    .await
}

#[tauri::command]
pub async fn listar_dispositivos(app_handle: AppHandle) -> ResultadoApp<Vec<Dispositivo>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_dispositivos())
    })
    .await
}

#[tauri::command]
pub async fn listar_auditoria(
    app_handle: AppHandle,
    limite: i64,
) -> ResultadoApp<Vec<RegistroAuditoria>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_auditoria(limite))
    })
    .await
}

#[tauri::command]
pub async fn verificar_auditoria(app_handle: AppHandle) -> ResultadoApp<VerificacaoAuditoria> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.verificar_auditoria())
    })
    .await
}

// ---------- Importação ----------

#[tauri::command]
pub async fn detectar_fonte(app_handle: AppHandle, pasta: String) -> ResultadoApp<Vec<Deteccao>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.detectar_fonte(&PathBuf::from(pasta)))
    })
    .await
}

#[tauri::command]
pub async fn iniciar_importacao(
    app_handle: AppHandle,
    dispositivo_id: DispositivoId,
    pasta: String,
    importador: String,
) -> ResultadoApp<EstadoImportacao> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.iniciar_importacao(dispositivo_id, &PathBuf::from(pasta), &importador))
    })
    .await
}

#[tauri::command]
pub async fn estado_importacao(app_handle: AppHandle) -> ResultadoApp<Option<EstadoImportacao>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| Ok(s.estado_importacao()))
    })
    .await
}

#[tauri::command]
pub async fn cancelar_importacao(app_handle: AppHandle) -> ResultadoApp<()> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| {
            s.cancelar_importacao();
            Ok(())
        })
    })
    .await
}

#[tauri::command]
pub async fn listar_extracoes(app_handle: AppHandle) -> ResultadoApp<Vec<Extracao>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_extracoes())
    })
    .await
}

#[tauri::command]
pub async fn obter_extracao(
    app_handle: AppHandle,
    extracao_id: ExtracaoId,
) -> ResultadoApp<Extracao> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.obter_extracao(extracao_id))
    })
    .await
}

#[tauri::command]
pub async fn totais_por_categoria(
    app_handle: AppHandle,
    extracao_id: ExtracaoId,
) -> ResultadoApp<Vec<TotalCategoria>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.totais_por_categoria(extracao_id))
    })
    .await
}

#[tauri::command]
pub async fn listar_arquivos(
    app_handle: AppHandle,
    extracao_id: ExtracaoId,
    categoria: Option<String>,
    texto: Option<String>,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<ArquivoExtracao>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_arquivos(extracao_id, categoria, texto, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn listar_execucoes(
    app_handle: AppHandle,
    extracao_id: ExtracaoId,
) -> ResultadoApp<Vec<ExecucaoParser>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_execucoes(extracao_id))
    })
    .await
}

#[tauri::command]
pub async fn listar_artefatos(
    app_handle: AppHandle,
    extracao_id: ExtracaoId,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<ArtefatoResumo>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_artefatos(extracao_id, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn detalhe_artefato(
    app_handle: AppHandle,
    artefato_id: i64,
) -> ResultadoApp<ArtefatoDetalhe> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.detalhe_artefato(artefato_id))
    })
    .await
}

/// Filtros opcionais (texto, app, tipo, com anexo) valem para o caso inteiro.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn listar_conversas(
    app_handle: AppHandle,
    extracao_id: Option<ExtracaoId>,
    deslocamento: i64,
    limite: i64,
    texto: Option<String>,
    app: Option<String>,
    tipo: Option<String>,
    com_anexo: Option<bool>,
) -> ResultadoApp<Pagina<Conversa>> {
    em_segundo_plano(app_handle, move |estado| {
        let filtro = FiltroConversas {
            extracao_id,
            texto,
            app,
            tipo,
            com_anexo,
        };
        estado.com(|s| s.listar_conversas_filtradas(&filtro, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn apps_das_conversas(
    app_handle: AppHandle,
    extracao_id: Option<ExtracaoId>,
) -> ResultadoApp<Vec<AppConversas>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.apps_das_conversas(extracao_id))
    })
    .await
}

#[tauri::command]
pub async fn posicao_mensagem(
    app_handle: AppHandle,
    conversa_id: i64,
    artefato_id: i64,
) -> ResultadoApp<Option<i64>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.posicao_mensagem(conversa_id, artefato_id))
    })
    .await
}

#[tauri::command]
pub async fn localizar_na_conversa(
    app_handle: AppHandle,
    conversa_id: i64,
    termo: String,
    limite: i64,
) -> ResultadoApp<Pagina<PosicaoMensagem>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.localizar_na_conversa(conversa_id, &termo, limite))
    })
    .await
}

/// A leitura da evidência e a decodificação acontecem depois de soltar a trava do
/// estado: a grade de miniaturas não segura os outros comandos.
#[tauri::command]
pub async fn miniatura(app_handle: AppHandle, arquivo_id: i64) -> ResultadoApp<Miniatura> {
    em_segundo_plano(app_handle, move |estado| {
        let pedido = estado.com(|s| s.pedido_miniatura(arquivo_id))?;
        Ok(sied_midia::obter(&pedido))
    })
    .await
}

#[tauri::command]
pub async fn obter_conversa(app_handle: AppHandle, conversa_id: i64) -> ResultadoApp<Conversa> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.obter_conversa(conversa_id))
    })
    .await
}

#[tauri::command]
pub async fn mensagens_conversa(
    app_handle: AppHandle,
    conversa_id: i64,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<ArtefatoResumo>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.mensagens_conversa(conversa_id, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn conversa_do_artefato(
    app_handle: AppHandle,
    artefato_id: i64,
) -> ResultadoApp<Option<i64>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.conversa_do_artefato(artefato_id))
    })
    .await
}

#[tauri::command]
pub async fn buscar(
    app_handle: AppHandle,
    filtro: FiltroBusca,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<ResultadoBusca> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.buscar(&filtro, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn resumo_caso(app_handle: AppHandle) -> ResultadoApp<ResumoCaso> {
    em_segundo_plano(app_handle, move |estado| estado.com(|s| s.resumo_caso())).await
}

#[tauri::command]
pub async fn listar_identificadores(
    app_handle: AppHandle,
    texto: Option<String>,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<Identificador>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_identificadores(texto, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn listar_contatos(
    app_handle: AppHandle,
    filtro: FiltroContatos,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<ContatoAgenda>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_contatos(&filtro, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn apps_dos_contatos(app_handle: AppHandle) -> ResultadoApp<Vec<TotalApp>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.apps_dos_contatos())
    })
    .await
}

#[tauri::command]
pub async fn listar_chamadas(
    app_handle: AppHandle,
    filtro: FiltroChamadas,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<Chamada>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_chamadas(&filtro, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn resumo_chamadas(app_handle: AppHandle) -> ResultadoApp<ResumoChamadas> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.resumo_chamadas())
    })
    .await
}

#[tauri::command]
pub async fn linha_do_tempo(
    app_handle: AppHandle,
    filtro: FiltroLinhaTempo,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<AchadoArtefato>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.linha_do_tempo(&filtro, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn resumo_linha_do_tempo(
    app_handle: AppHandle,
    filtro: FiltroLinhaTempo,
) -> ResultadoApp<ResumoLinhaTempo> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.resumo_linha_do_tempo(&filtro))
    })
    .await
}

#[tauri::command]
pub async fn listar_evidencias(app_handle: AppHandle) -> ResultadoApp<Vec<Evidencia>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_evidencias())
    })
    .await
}

#[tauri::command]
pub async fn marcacoes_evidencias(app_handle: AppHandle) -> ResultadoApp<Marcacoes> {
    em_segundo_plano(app_handle, move |estado| estado.com(|s| s.marcacoes())).await
}

#[tauri::command]
pub async fn adicionar_evidencia(
    app_handle: AppHandle,
    nova: NovaEvidencia,
) -> ResultadoApp<Evidencia> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.adicionar_evidencia(nova))
    })
    .await
}

#[tauri::command]
pub async fn alterar_evidencia(
    app_handle: AppHandle,
    id: i64,
    textos: TextosEvidencia,
) -> ResultadoApp<Evidencia> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.alterar_evidencia(id, textos))
    })
    .await
}

#[tauri::command]
pub async fn mover_evidencia(
    app_handle: AppHandle,
    id: i64,
    numero: i64,
) -> ResultadoApp<Vec<Evidencia>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.mover_evidencia(id, numero))
    })
    .await
}

#[tauri::command]
pub async fn remover_evidencia(
    app_handle: AppHandle,
    id: i64,
    motivo: Option<String>,
) -> ResultadoApp<()> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.remover_evidencia(id, motivo))
    })
    .await
}

#[tauri::command]
pub async fn listar_relatorios(app_handle: AppHandle) -> ResultadoApp<Vec<RelatorioRegistrado>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_relatorios())
    })
    .await
}

/// Conferir os hashes, reler os arquivos e diagramar o PDF pode levar alguns
/// segundos; a janela continua respondendo (ver o início do módulo).
#[tauri::command]
pub async fn gerar_relatorio(
    app_handle: AppHandle,
    pedido: PedidoRelatorio,
) -> ResultadoApp<RelatorioRegistrado> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.gerar_relatorio(pedido))
    })
    .await
}
