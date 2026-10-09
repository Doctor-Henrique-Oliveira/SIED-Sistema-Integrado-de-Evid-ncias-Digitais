//! Aplicativo desktop do SIED (Tauri).
//!
//! Pastas usadas (Windows, padrão):
//! - dados do aplicativo: `%LOCALAPPDATA%\br.sied.desktop\` (banco global `sied-global.db`)
//! - logs: `<dados>\logs\`
//! - casos: `Documentos\SIED\Casos\` (cada caso em sua própria pasta)
//!
//! `SIED_PASTA_DADOS` e `SIED_PASTA_CASOS` substituem os padrões (modo portátil e testes).

mod comandos;
mod controle_documental;
mod descobertas;
mod disco;
mod documentos;
mod erro;
mod log;
mod servico;
mod transcricao;
mod visao_local;

use sied_armazenamento::Sessao;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

use comandos::Estado;
use servico::Servico;

fn pasta_env(nome: &str) -> Option<PathBuf> {
    std::env::var_os(nome)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

pub fn executar() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let caminhos = app.path();
            let pasta_dados = pasta_env("SIED_PASTA_DADOS")
                .or_else(|| caminhos.app_local_data_dir().ok())
                .unwrap_or_else(|| PathBuf::from("sied-dados"));
            let pasta_logs = pasta_dados.join("logs");
            let guarda_log = log::iniciar(&pasta_logs);
            let pasta_casos = pasta_env("SIED_PASTA_CASOS")
                .or_else(|| {
                    caminhos
                        .document_dir()
                        .ok()
                        .map(|d| d.join("SIED").join("Casos"))
                })
                .unwrap_or_else(|| pasta_dados.join("Casos"));

            let sessao = Sessao::do_sistema();
            tracing::info!(
                versao = env!("CARGO_PKG_VERSION"),
                dados = %pasta_dados.display(),
                usuario = %sessao.usuario,
                maquina = %sessao.maquina,
                "SIED iniciado"
            );
            let (servico, falha_inicial) =
                match Servico::iniciar(&pasta_dados, &pasta_logs, &pasta_casos, sessao) {
                    Ok(s) => (Some(s), None),
                    Err(e) => {
                        tracing::error!(erro = %e.mensagem, "falha ao iniciar o banco global");
                        (None, Some(e.mensagem))
                    }
                };
            app.manage(Estado {
                servico: Mutex::new(servico),
                descobertas: Arc::new(descobertas::RegistroDescobertas::default()),
                falha_inicial,
            });
            if let Some(g) = guarda_log {
                app.manage(g);
            }
            Ok(())
        })
        // Fechar a janela principal encerra o caso aberto (registra "caso_fechado").
        .on_window_event(|janela, evento| {
            if let WindowEvent::CloseRequested { .. } = evento {
                encerrar(janela.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            comandos::info_app,
            comandos::listar_casos,
            comandos::remover_caso_catalogo,
            comandos::criar_caso,
            comandos::abrir_caso,
            comandos::abrir_pasta_caso,
            comandos::fechar_caso,
            comandos::caso_atual,
            comandos::cadastrar_dispositivo,
            comandos::listar_dispositivos,
            comandos::listar_auditoria,
            comandos::verificar_auditoria,
            comandos::pasta_em_rede,
            comandos::conferir_derivados,
            comandos::reconstruir_derivados,
            comandos::detectar_fonte,
            comandos::iniciar_importacao,
            comandos::estado_importacao,
            comandos::cancelar_importacao,
            comandos::eventos_importacao,
            comandos::eventos_da_extracao,
            comandos::listar_extracoes,
            comandos::obter_extracao,
            comandos::totais_por_categoria,
            comandos::listar_arquivos,
            comandos::listar_execucoes,
            comandos::listar_artefatos,
            comandos::detalhe_artefato,
            comandos::listar_conversas,
            comandos::apps_das_conversas,
            comandos::posicao_mensagem,
            comandos::localizar_na_conversa,
            comandos::miniatura,
            comandos::ler_documento,
            comandos::indexar_documento,
            comandos::cancelar_documento,
            comandos::transcrever_local,
            comandos::cancelar_transcricao,
            comandos::listar_transcricoes,
            comandos::segmentos_transcricao,
            comandos::caminho_midia_transcricao,
            comandos::buscar_documentos,
            comandos::pesquisar_universal,
            comandos::contexto_relacoes,
            comandos::indexar_visual,
            comandos::analisar_visual_local,
            comandos::semelhantes_visuais,
            comandos::obter_documento,
            comandos::regioes_documento,
            comandos::localizar_regiao_documento,
            comandos::obter_arquivo,
            comandos::entidades_documento,
            comandos::criar_dataset,
            comandos::listar_datasets,
            comandos::adicionar_amostra,
            comandos::amostras_dataset,
            comandos::avaliar_dataset,
            comandos::revisar_entidade,
            comandos::revisoes_entidade,
            comandos::obter_conversa,
            comandos::mensagens_conversa,
            comandos::conversa_do_artefato,
            comandos::buscar,
            comandos::listar_vocabularios_pesquisa,
            comandos::salvar_vocabulario_pesquisa,
            comandos::exportar_vocabulario_pesquisa,
            comandos::executar_descoberta,
            comandos::paginar_descoberta,
            comandos::cancelar_descoberta,
            comandos::indexar_embeddings,
            comandos::buscar_hibrida,
            comandos::listar_consultas_descoberta,
            comandos::resumo_caso,
            comandos::listar_identificadores,
            comandos::listar_contatos,
            comandos::apps_dos_contatos,
            comandos::listar_chamadas,
            comandos::resumo_chamadas,
            comandos::linha_do_tempo,
            comandos::resumo_linha_do_tempo,
            comandos::listar_evidencias,
            comandos::marcacoes_evidencias,
            comandos::adicionar_evidencia,
            comandos::alterar_evidencia,
            comandos::mover_evidencia,
            comandos::remover_evidencia,
            comandos::listar_relatorios,
            comandos::conferir_pasta_relatorios,
            comandos::gerar_relatorio,
        ])
        .build(tauri::generate_context!())
        .expect("falha ao montar o aplicativo");

    app.run(|app, evento| {
        if let RunEvent::Exit = evento {
            encerrar(app);
            tracing::info!("SIED encerrado");
        }
    });
}

/// Fecha o caso aberto, se houver. Pode ser chamado mais de uma vez.
fn encerrar(app: &AppHandle) {
    if let Some(estado) = app.try_state::<Estado>() {
        estado.descobertas.cancelar_ativa();
        if let Ok(mut s) = estado.servico.lock() {
            if let Some(s) = s.as_mut() {
                s.encerrar();
            }
        }
    }
}
