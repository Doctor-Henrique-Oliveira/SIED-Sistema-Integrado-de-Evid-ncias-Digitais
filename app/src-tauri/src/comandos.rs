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
use sied_armazenamento::derivados::{ConferenciaDerivados, ReconstrucaoDerivados};
use sied_armazenamento::descoberta::{PedidoDescoberta, ResultadoDescoberta};
use sied_armazenamento::documentos::{
    AchadoDocumento, DocumentoIndexado, EntidadeDocumento, EstadoRevisaoEntidade, RegiaoDocumento,
    RevisaoEntidade,
};
use sied_armazenamento::embeddings::{ResultadoBuscaHibrida, ResumoIndexacaoEmbeddings};
use sied_armazenamento::evidencias::{Evidencia, Marcacoes, NovaEvidencia, TextosEvidencia};
use sied_armazenamento::extracoes::{
    ArquivoExtracao, ArtefatoDetalhe, ArtefatoResumo, ExecucaoParser, Extracao, Pagina,
    TotalCategoria,
};
use sied_armazenamento::linha_tempo::{FiltroLinhaTempo, ResumoLinhaTempo};
use sied_armazenamento::pesquisa_local::{
    ConsultaDescobertaSalva, ExecucaoDescoberta, VersaoVocabularioPesquisa, VocabularioPesquisa,
};
use sied_armazenamento::relatorios::{ConferenciaPastaRelatorios, RelatorioRegistrado};
use sied_armazenamento::resumo::{Identificador, ResumoCaso};
use sied_armazenamento::transcricoes::{SegmentoTranscricao, TranscricaoLocal};
use sied_armazenamento::CasoCatalogado;
use sied_documentos::{ModoDocumento, ResultadoDocumento};
use sied_dominio::{CasoId, Dispositivo, DispositivoId, ExtracaoId, NovoCaso, NovoDispositivo};
use sied_ingestao::eventos::EventoImportacao;
use sied_ingestao::importador::Deteccao;
use sied_midia::Miniatura;
use sied_relatorio::PedidoRelatorio;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};
use crate::servico::{CasoAberto, EstadoImportacao, InfoApp, Servico};

/// Leitura e indexação compartilham uma única vaga e cancelamento cooperativo.
static DOCUMENTOS: crate::controle_documental::OperacoesDocumentais =
    crate::controle_documental::OperacoesDocumentais::novo();

pub struct Estado {
    pub servico: Mutex<Option<Servico>>,
    pub(crate) descobertas: Arc<crate::descobertas::RegistroDescobertas>,
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

/// Pasta num compartilhamento de rede (inclusive unidade mapeada, que o caminho não
/// revela). Fora da thread da janela: uma unidade de rede desconectada pode demorar.
#[tauri::command]
pub async fn pasta_em_rede(caminho: String) -> bool {
    tauri::async_runtime::spawn_blocking(move || {
        crate::disco::em_rede(std::path::Path::new(&caminho))
    })
    .await
    .unwrap_or(false)
}

#[tauri::command]
pub async fn listar_casos(app_handle: AppHandle) -> ResultadoApp<Vec<CasoCatalogado>> {
    em_segundo_plano(app_handle, move |estado| estado.com(|s| s.listar_casos())).await
}

#[tauri::command]
pub async fn remover_caso_catalogo(app_handle: AppHandle, caso_id: CasoId) -> ResultadoApp<bool> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.remover_caso_catalogo(caso_id))
    })
    .await
}

#[tauri::command]
pub async fn criar_caso(
    app_handle: AppHandle,
    dados: NovoCaso,
    pasta_base: Option<String>,
) -> ResultadoApp<CasoAberto> {
    app_handle.state::<Estado>().descobertas.cancelar_ativa();
    DOCUMENTOS.cancelar_ativa();
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.criar_caso(dados, pasta_base))
    })
    .await
}

#[tauri::command]
pub async fn abrir_caso(app_handle: AppHandle, caso_id: CasoId) -> ResultadoApp<CasoAberto> {
    app_handle.state::<Estado>().descobertas.cancelar_ativa();
    DOCUMENTOS.cancelar_ativa();
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.abrir_caso(caso_id))
    })
    .await
}

#[tauri::command]
pub async fn abrir_pasta_caso(app_handle: AppHandle, pasta: String) -> ResultadoApp<CasoAberto> {
    app_handle.state::<Estado>().descobertas.cancelar_ativa();
    DOCUMENTOS.cancelar_ativa();
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.abrir_pasta(&PathBuf::from(pasta)))
    })
    .await
}

#[tauri::command]
pub async fn fechar_caso(app_handle: AppHandle) -> ResultadoApp<()> {
    app_handle.state::<Estado>().descobertas.cancelar_ativa();
    DOCUMENTOS.cancelar_ativa();
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

/// Percorre todos os artefatos: fora da thread da janela.
#[tauri::command]
pub async fn conferir_derivados(app_handle: AppHandle) -> ResultadoApp<ConferenciaDerivados> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.conferir_derivados())
    })
    .await
}

/// Refaz conversas e índices de busca de todo o caso: pode levar alguns segundos.
#[tauri::command]
pub async fn reconstruir_derivados(app_handle: AppHandle) -> ResultadoApp<ReconstrucaoDerivados> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.reconstruir_derivados())
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
pub async fn eventos_importacao(
    app_handle: AppHandle,
    desde: u64,
) -> ResultadoApp<Vec<EventoImportacao>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| Ok(s.eventos_importacao(desde)))
    })
    .await
}

#[tauri::command]
pub async fn eventos_da_extracao(
    app_handle: AppHandle,
    id: ExtracaoId,
) -> ResultadoApp<Vec<EventoImportacao>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.eventos_da_extracao(id))
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

/// Resolve o inventário sob a trava; abre e interpreta a origem após soltá-la.
/// O caso esperado é somente uma guarda contra troca/fechamento durante a navegação.
#[tauri::command]
pub async fn ler_documento(
    app_handle: AppHandle,
    arquivo_id: i64,
    caso_id: CasoId,
    pagina: Option<u32>,
    modo: Option<ModoDocumento>,
) -> ResultadoApp<ResultadoDocumento> {
    let guarda = DOCUMENTOS.iniciar(caso_id)?;
    em_segundo_plano(app_handle, move |estado| {
        // A guarda permanece na thread mesmo se o frontend descartar a requisição.
        let pedido = estado
            .com(|s| s.pedido_documento(arquivo_id, caso_id))?
            .com_modo(modo.unwrap_or_default());
        Ok(pedido.executar_controlado(pagina, guarda.controle()))
    })
    .await
}

/// Reabre/confere a origem; texto e candidatos não vêm do IPC. A escrita curta
/// ocorre só após a leitura fora da trava e a conferência do caso ainda ativo.
#[tauri::command]
pub async fn indexar_documento(
    app_handle: AppHandle,
    arquivo_id: i64,
    caso_id: CasoId,
    pagina: Option<u32>,
    modo: Option<ModoDocumento>,
) -> ResultadoApp<DocumentoIndexado> {
    let guarda = DOCUMENTOS.iniciar(caso_id)?;
    em_segundo_plano(app_handle, move |estado| {
        let pedido = estado
            .com(|s| s.pedido_documento(arquivo_id, caso_id))?
            .com_modo(modo.unwrap_or_default());
        let leitura = pedido.executar_controlado(pagina, guarda.controle());
        let novo = crate::documentos::preparar_derivado(leitura, guarda.controle())?;
        estado.com(|s| {
            // Confere caso antes do gate: uma operação enfileirada não pode
            // registrar resultado no caso aberto depois dela.
            s.banco_documental(caso_id)?;
            guarda.finalizar()?;
            s.guardar_documento(novo, guarda.controle())
        })
    })
    .await
}

/// Pedido de cancelamento, não uma afirmação de término ou aborto preemptivo.
#[tauri::command]
pub async fn cancelar_documento(caso_id: CasoId) -> ResultadoApp<bool> {
    DOCUMENTOS.cancelar(caso_id)
}

/// Executa o provedor local sobre uma mídia do inventário. O comando não recebe
/// caminho, texto ou hash do frontend; tudo é resolvido no caso aberto.
#[tauri::command]
pub async fn transcrever_local(
    app_handle: AppHandle,
    arquivo_id: i64,
    caso_id: CasoId,
) -> ResultadoApp<TranscricaoLocal> {
    let guarda = crate::transcricao::TRANSCRICOES.iniciar(caso_id)?;
    em_segundo_plano(app_handle, move |estado| {
        let leitura = estado.com(|s| s.pedido_transcricao(arquivo_id, caso_id))?;
        let novo = crate::transcricao::executar(leitura, guarda.controle())?;
        estado.com(|s| crate::transcricao::persistir(s, novo, &guarda))
    })
    .await
}

#[tauri::command]
pub async fn cancelar_transcricao(caso_id: CasoId) -> ResultadoApp<bool> {
    crate::transcricao::TRANSCRICOES.cancelar(caso_id)
}

#[tauri::command]
pub async fn listar_transcricoes(
    app_handle: AppHandle,
    caso_id: CasoId,
    arquivo_id: i64,
) -> ResultadoApp<Vec<TranscricaoLocal>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_transcricoes(caso_id, arquivo_id))
    })
    .await
}

#[tauri::command]
pub async fn segmentos_transcricao(
    app_handle: AppHandle,
    caso_id: CasoId,
    transcricao_id: i64,
    deslocamento: i64,
    limite: i64,
    termo: Option<String>,
) -> ResultadoApp<Pagina<SegmentoTranscricao>> {
    em_segundo_plano(app_handle, move |estado| {
        estado
            .com(|s| s.segmentos_transcricao(caso_id, transcricao_id, deslocamento, limite, termo))
    })
    .await
}

/// Entrega somente o caminho resolvido pelo inventário para que o WebView
/// possa usar o protocolo `asset:` do Tauri na reprodução local.
#[tauri::command]
pub async fn caminho_midia_transcricao(
    app_handle: AppHandle,
    caso_id: CasoId,
    arquivo_id: i64,
) -> ResultadoApp<String> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.caminho_midia_transcricao(caso_id, arquivo_id))
    })
    .await
}

#[tauri::command]
pub async fn buscar_documentos(
    app_handle: AppHandle,
    caso_id: CasoId,
    termo: String,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<AchadoDocumento>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.buscar_documentos(caso_id, &termo, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn pesquisar_universal(
    app_handle: AppHandle,
    caso_id: CasoId,
    operacao_id: String,
    pedido: sied_armazenamento::universal::PedidoUniversal,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<sied_armazenamento::universal::ResultadoUniversal> {
    let operacao = app_handle
        .state::<Estado>()
        .descobertas
        .iniciar(operacao_id)?;
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| {
            let resultado = s.banco_documental(caso_id)?.pesquisar_universal(
                &pedido,
                deslocamento,
                limite,
                &operacao.controle,
            )?;
            operacao.controle.concluir()?;
            Ok(resultado)
        })
    })
    .await
}

#[tauri::command]
pub async fn contexto_relacoes(
    app_handle: AppHandle,
    caso_id: CasoId,
    operacao_id: String,
    artefato_id: i64,
    janela_minutos: u32,
    limite: i64,
) -> ResultadoApp<sied_armazenamento::relacoes::ContextoRelacoes> {
    let operacao = app_handle
        .state::<Estado>()
        .descobertas
        .iniciar(operacao_id)?;
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| {
            s.banco_documental(caso_id)?
                .contexto_relacoes(artefato_id, janela_minutos, limite, &operacao.controle)
                .map_err(Into::into)
        })
    })
    .await
}

#[tauri::command]
pub async fn indexar_visual(
    app_handle: AppHandle,
    caso_id: CasoId,
    arquivo_id: i64,
) -> ResultadoApp<sied_armazenamento::visuais::VisualIndexado> {
    let guarda = DOCUMENTOS.iniciar(caso_id)?;
    em_segundo_plano(app_handle, move |estado| {
        // Cache é histórico: a origem atual não é certificada só por reutilizá-lo.
        if let Some(d) = estado.com(|s| {
            Ok(s.banco_documental(caso_id)?.visual_existente(
                arquivo_id,
                sied_documentos::visao::MOTOR_ID,
                sied_documentos::visao::MOTOR_VERSAO,
            )?)
        })? {
            return Ok(d);
        }
        let leitura = estado
            .com(|s| s.pedido_documento(arquivo_id, caso_id))?
            .executar_visual(guarda.controle());
        if leitura.estado != sied_documentos::EstadoDocumento::Pronto {
            let codigo = match leitura.estado {
                sied_documentos::EstadoDocumento::OrigemDivergente
                | sied_documentos::EstadoDocumento::HashAusente => CodigoErro::Integridade,
                sied_documentos::EstadoDocumento::SemSuporte => CodigoErro::NaoSuportado,
                sied_documentos::EstadoDocumento::Cancelado => CodigoErro::Conflito,
                sied_documentos::EstadoDocumento::LimiteExcedido => CodigoErro::Validacao,
                _ => CodigoErro::Interno,
            };
            return Err(ErroApp::novo(
                codigo,
                leitura
                    .motivo
                    .unwrap_or_else(|| "a análise visual não publicou descritor".into()),
            ));
        }
        let a = leitura.assinatura.ok_or_else(|| {
            ErroApp::novo(
                CodigoErro::Integridade,
                leitura
                    .motivo
                    .unwrap_or_else(|| "a análise visual pronta não contém descritor".into()),
            )
        })?;
        crate::documentos::conferir_cancelamento(guarda.controle())?;
        let d = sied_armazenamento::visuais::VisualIndexado {
            id: 0,
            caso_id: a.caso_id,
            extracao_id: a.extracao_id,
            arquivo_id: a.arquivo_id,
            caminho: a.caminho.to_string(),
            sha256_origem: a.sha256,
            dhash: a.dhash,
            largura: a.largura,
            altura: a.altura,
            luminancia_media: a.luminancia_media,
            motor_id: a.motor_id,
            motor_versao: a.motor_versao,
            executado_em: a.executado_em.to_rfc3339(),
            hash_derivado: String::new(),
        };
        estado.com(|s| {
            let b = s.banco_documental(caso_id)?;
            guarda.finalizar()?;
            Ok(b.guardar_visual(d)?)
        })
    })
    .await
}

/// Executa um provider de visão configurado localmente. O provider recebe
/// somente a origem resolvida pelo inventário e devolve candidatos com regiões;
/// nenhum resultado é tratado como conclusão pericial.
#[tauri::command]
pub async fn analisar_visual_local(
    app_handle: AppHandle,
    caso_id: CasoId,
    arquivo_id: i64,
) -> ResultadoApp<crate::visao_local::ResultadoVisaoLocal> {
    let guarda = DOCUMENTOS.iniciar(caso_id)?;
    em_segundo_plano(app_handle, move |estado| {
        let leitura = estado.com(|s| s.pedido_documento(arquivo_id, caso_id))?;
        let caminho = leitura.caminho_absoluto().ok_or_else(|| {
            ErroApp::novo(
                CodigoErro::NaoSuportado,
                "o motor visual local só pode ler uma imagem regular fora de contêineres",
            )
        })?;
        if !leitura.tem_hash_origem() {
            return Err(ErroApp::novo(
                CodigoErro::Integridade,
                "o inventário não contém SHA-256 conferível para esta imagem",
            ));
        }
        if leitura.tamanho_origem() > 16 * 1024 * 1024 {
            return Err(ErroApp::novo(
                CodigoErro::Validacao,
                "a imagem excede o limite de 16 MiB do provider visual local",
            ));
        }
        let extensao = caminho
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(extensao.as_str(), "png" | "jpg" | "jpeg") {
            return Err(ErroApp::novo(
                CodigoErro::NaoSuportado,
                "o motor visual local admite somente PNG e JPEG neste estágio",
            ));
        }
        let resultado = crate::visao_local::executar(&caminho, guarda.controle())?;
        guarda.finalizar()?;
        Ok(resultado)
    })
    .await
}

#[tauri::command]
pub async fn semelhantes_visuais(
    app_handle: AppHandle,
    caso_id: CasoId,
    operacao_id: String,
    visual_id: i64,
    max_bits: u32,
    limite: usize,
) -> ResultadoApp<sied_armazenamento::visuais::ResultadoSimilarVisual> {
    let op = app_handle
        .state::<Estado>()
        .descobertas
        .iniciar(operacao_id)?;
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| {
            let r = s.banco_documental(caso_id)?.semelhantes_visuais(
                visual_id,
                max_bits,
                limite,
                &op.controle,
            )?;
            op.controle.concluir()?;
            Ok(r)
        })
    })
    .await
}

#[tauri::command]
pub async fn obter_documento(
    app_handle: AppHandle,
    caso_id: CasoId,
    documento_id: i64,
) -> ResultadoApp<DocumentoIndexado> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.obter_documento(caso_id, documento_id))
    })
    .await
}

#[tauri::command]
pub async fn regioes_documento(
    app_handle: AppHandle,
    caso_id: CasoId,
    documento_id: i64,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<RegiaoDocumento>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.regioes_documento(caso_id, documento_id, deslocamento, limite))
    })
    .await
}

#[tauri::command]
pub async fn localizar_regiao_documento(
    app_handle: AppHandle,
    caso_id: CasoId,
    documento_id: i64,
    inicio_utf8: u32,
    fim_utf8: u32,
) -> ResultadoApp<Option<u32>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.localizar_regiao_documento(caso_id, documento_id, inicio_utf8, fim_utf8))
    })
    .await
}

#[tauri::command]
pub async fn obter_arquivo(
    app_handle: AppHandle,
    caso_id: CasoId,
    extracao_id: ExtracaoId,
    arquivo_id: i64,
) -> ResultadoApp<ArquivoExtracao> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| {
            let a = s
                .banco_documental(caso_id)?
                .arquivo_com_origem(arquivo_id)?;
            if a.extracao_id != extracao_id {
                return Err(ErroApp::novo(
                    CodigoErro::Conflito,
                    "arquivo não pertence à extração solicitada",
                ));
            }
            Ok(a.arquivo)
        })
    })
    .await
}

#[tauri::command]
pub async fn entidades_documento(
    app_handle: AppHandle,
    caso_id: CasoId,
    documento_id: i64,
) -> ResultadoApp<Vec<EntidadeDocumento>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.entidades_documento(caso_id, documento_id))
    })
    .await
}

#[tauri::command]
pub async fn criar_dataset(
    app_handle: AppHandle,
    caso_id: CasoId,
    nome: String,
    autorizacao: String,
    retencao: String,
) -> ResultadoApp<sied_armazenamento::supervisao::DatasetSupervisionado> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| {
            Ok(s.banco_documental(caso_id)?
                .criar_dataset(&nome, &autorizacao, &retencao)?)
        })
    })
    .await
}
#[tauri::command]
pub async fn listar_datasets(
    app_handle: AppHandle,
    caso_id: CasoId,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<sied_armazenamento::supervisao::DatasetSupervisionado>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| {
            Ok(s.banco_documental(caso_id)?
                .listar_datasets(deslocamento, limite)?)
        })
    })
    .await
}
#[tauri::command]
pub async fn adicionar_amostra(
    app_handle: AppHandle,
    caso_id: CasoId,
    dataset_id: i64,
    pedido: sied_armazenamento::supervisao::PedidoAmostraSupervisionada,
) -> ResultadoApp<sied_armazenamento::supervisao::AmostraSupervisionada> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| {
            Ok(s.banco_documental(caso_id)?
                .adicionar_amostra(dataset_id, &pedido)?)
        })
    })
    .await
}
#[tauri::command]
pub async fn amostras_dataset(
    app_handle: AppHandle,
    caso_id: CasoId,
    dataset_id: i64,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<sied_armazenamento::supervisao::AmostraSupervisionada>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| {
            Ok(s.banco_documental(caso_id)?
                .amostras_dataset(dataset_id, deslocamento, limite)?)
        })
    })
    .await
}
#[tauri::command]
pub async fn avaliar_dataset(
    app_handle: AppHandle,
    caso_id: CasoId,
    dataset_id: i64,
    operacao_id: String,
) -> ResultadoApp<sied_armazenamento::supervisao::AvaliacaoSupervisionada> {
    let op = app_handle
        .state::<Estado>()
        .descobertas
        .iniciar(operacao_id)?;
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| {
            let r = s
                .banco_documental(caso_id)?
                .avaliar_dataset_com_controle(dataset_id, &op.controle)?;
            op.controle.concluir()?;
            Ok(r)
        })
    })
    .await
}

#[tauri::command]
pub async fn revisar_entidade(
    app_handle: AppHandle,
    caso_id: CasoId,
    entidade_id: i64,
    estado: EstadoRevisaoEntidade,
    nota: Option<String>,
) -> ResultadoApp<EntidadeDocumento> {
    em_segundo_plano(app_handle, move |servico| {
        servico.com(|s| s.revisar_entidade(caso_id, entidade_id, estado, nota))
    })
    .await
}

#[tauri::command]
pub async fn revisoes_entidade(
    app_handle: AppHandle,
    caso_id: CasoId,
    entidade_id: i64,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<Pagina<RevisaoEntidade>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.revisoes_entidade(caso_id, entidade_id, deslocamento, limite))
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
pub async fn listar_vocabularios_pesquisa(
    app_handle: AppHandle,
    caso_id: CasoId,
) -> ResultadoApp<Vec<VersaoVocabularioPesquisa>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_vocabularios_pesquisa(caso_id))
    })
    .await
}

#[tauri::command]
pub async fn salvar_vocabulario_pesquisa(
    app_handle: AppHandle,
    caso_id: CasoId,
    dados: VocabularioPesquisa,
) -> ResultadoApp<VersaoVocabularioPesquisa> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.salvar_vocabulario_pesquisa(caso_id, dados))
    })
    .await
}

#[tauri::command]
pub async fn exportar_vocabulario_pesquisa(
    app_handle: AppHandle,
    caso_id: CasoId,
    id: i64,
) -> ResultadoApp<String> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.exportar_vocabulario_pesquisa(caso_id, id))
    })
    .await
}

#[tauri::command]
pub async fn executar_descoberta(
    app_handle: AppHandle,
    caso_id: CasoId,
    operacao_id: String,
    pedido: PedidoDescoberta,
    vocabulario_id: Option<i64>,
) -> ResultadoApp<ExecucaoDescoberta> {
    let operacao = app_handle
        .state::<Estado>()
        .descobertas
        .iniciar(operacao_id)?;
    em_segundo_plano(app_handle, move |estado| {
        operacao.controle.verificar()?;
        estado.com(|s| s.executar_descoberta(caso_id, pedido, vocabulario_id, &operacao.controle))
    })
    .await
}

#[tauri::command]
pub async fn paginar_descoberta(
    app_handle: AppHandle,
    caso_id: CasoId,
    operacao_id: String,
    consulta_id: i64,
    deslocamento: i64,
    limite: i64,
) -> ResultadoApp<ResultadoDescoberta> {
    let operacao = app_handle
        .state::<Estado>()
        .descobertas
        .iniciar(operacao_id)?;
    em_segundo_plano(app_handle, move |estado| {
        operacao.controle.verificar()?;
        estado.com(|s| {
            s.paginar_descoberta(
                caso_id,
                consulta_id,
                deslocamento,
                limite,
                &operacao.controle,
            )
        })
    })
    .await
}

/// A sinalização não adquire a trava do serviço ocupada pela consulta.
#[tauri::command]
pub async fn cancelar_descoberta(app_handle: AppHandle, operacao_id: String) -> ResultadoApp<bool> {
    app_handle
        .state::<Estado>()
        .descobertas
        .cancelar(&operacao_id)
}

#[tauri::command]
pub async fn indexar_embeddings(
    app_handle: AppHandle,
    caso_id: CasoId,
    operacao_id: String,
) -> ResultadoApp<ResumoIndexacaoEmbeddings> {
    let operacao = app_handle
        .state::<Estado>()
        .descobertas
        .iniciar(operacao_id)?;
    em_segundo_plano(app_handle, move |estado| {
        operacao.controle.verificar()?;
        estado.com(|s| s.indexar_embeddings(caso_id, &operacao.controle))
    })
    .await
}

#[tauri::command]
pub async fn buscar_hibrida(
    app_handle: AppHandle,
    caso_id: CasoId,
    operacao_id: String,
    texto: String,
    deslocamento: i64,
    limite: i64,
    peso_semantico: f64,
) -> ResultadoApp<ResultadoBuscaHibrida> {
    let operacao = app_handle
        .state::<Estado>()
        .descobertas
        .iniciar(operacao_id)?;
    em_segundo_plano(app_handle, move |estado| {
        operacao.controle.verificar()?;
        estado.com(|s| {
            s.buscar_hibrida(
                caso_id,
                texto,
                deslocamento,
                limite,
                peso_semantico,
                &operacao.controle,
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn listar_consultas_descoberta(
    app_handle: AppHandle,
    caso_id: CasoId,
) -> ResultadoApp<Vec<ConsultaDescobertaSalva>> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.listar_consultas_descoberta(caso_id))
    })
    .await
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

/// Lê cada PDF registrado para conferir o SHA-256: fora da thread da janela.
#[tauri::command]
pub async fn conferir_pasta_relatorios(
    app_handle: AppHandle,
) -> ResultadoApp<ConferenciaPastaRelatorios> {
    em_segundo_plano(app_handle, move |estado| {
        estado.com(|s| s.conferir_pasta_relatorios())
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
