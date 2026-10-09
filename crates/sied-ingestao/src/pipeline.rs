//! Pipeline de importação, igual para todas as ferramentas:
//!
//! 1. **Contar** arquivos e bytes (para o progresso).
//! 2. **Inventariar**: ler cada arquivo uma vez, calcular SHA-256, classificar pela
//!    assinatura e gravar; ao mesmo tempo, escrever o `manifesto.sha256`.
//! 3. **Planejar**: o importador da ferramenta diz quais arquivos cada parser lê.
//! 4. **Processar**: para cada tarefa, copiar os arquivos para a área de trabalho do
//!    caso conferindo o hash com o inventário, rodar o parser sobre a cópia e gravar
//!    artefatos com a referência de origem completa. O anexo que o parser informa
//!    (caminho gravado pelo app) é ligado ao arquivo do inventário, ver [`escolher_anexo`].
//!
//! A extração é acessada só por [`FonteSomenteLeitura`]. Onde gravar é decidido por
//! [`Destino`] (implementado pelo banco do caso), o que mantém este módulo testável.

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sied_dominio::artefato::{NovoArtefato, ReferenciaRegistro};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use ts_rs::TS;

use crate::eventos::{numero_br, tamanho_br, EventoImportacao, NivelEvento, RegistroEventos};
use crate::fonte::{CaminhoFonte, FonteSomenteLeitura};
use crate::importador::{Importador, PlanoImportacao, TarefaParser};
use crate::inventario::{self, ArquivoInventariado};
use crate::parser::{ArquivoDeTrabalho, AvisoParser, EntradaParser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Registro, Resultado};

/// Arquivos auxiliares do SQLite copiados junto com o banco.
pub const AUXILIARES_SQLITE: &[&str] = &["-wal", "-journal"];
pub const NOME_MANIFESTO: &str = "manifesto.sha256";
pub const NOME_LOG: &str = "log-importacao.jsonl";
pub const NOME_CHECKPOINT: &str = "checkpoint-retomada.json";
const LOTE: usize = 500;
const MAX_OBSERVACOES: usize = 50;
/// Bancos SQLite anunciados um a um no console durante o inventário.
const MAX_BANCOS_ANUNCIADOS: u64 = 30;
/// Avisos de cada parser anunciados um a um no console (todos ficam no banco do caso).
const MAX_AVISOS_ANUNCIADOS: u64 = 20;
/// Intervalo, em registros gravados, dos anúncios de gravação de cada parser.
const ANUNCIO_REGISTROS: u64 = 5_000;

/// Limites operacionais da ingestão. Podem ser reduzidos por ambiente de laboratório,
/// mas nunca ampliados acima dos tetos seguros do SIED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitesRecursos {
    pub max_arquivos: u64,
    pub max_bytes: u64,
    pub workers_hash: usize,
}

const MAX_ARQUIVOS_ABSOLUTO: u64 = 2_000_000;
const MAX_BYTES_ABSOLUTO: u64 = 4 * 1024 * 1024 * 1024 * 1024;

fn limite_env(nome: &str, padrao: u64, teto: u64) -> u64 {
    std::env::var(nome)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(1, teto))
        .unwrap_or(padrao)
}

/// Resolve limites uma vez por execução, sem depender de configuração remota.
pub fn limites_recursos() -> LimitesRecursos {
    LimitesRecursos {
        max_arquivos: limite_env(
            "SIED_IMPORT_MAX_FILES",
            MAX_ARQUIVOS_ABSOLUTO,
            MAX_ARQUIVOS_ABSOLUTO,
        ),
        max_bytes: limite_env(
            "SIED_IMPORT_MAX_BYTES",
            MAX_BYTES_ABSOLUTO,
            MAX_BYTES_ABSOLUTO,
        ),
        workers_hash: limite_env("SIED_IMPORT_HASH_THREADS", 1, 8) as usize,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRetomada {
    pub schema: u8,
    pub estado: String,
    pub fonte: String,
    pub atualizado_em: String,
    pub arquivos_inventariados: u64,
    pub bytes_lidos: u64,
    pub workers_hash: usize,
    pub limite_arquivos: u64,
    pub limite_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvaliacaoRetomada {
    SemCheckpoint,
    FonteDivergente {
        registrada: String,
        atual: String,
    },
    /// A fonte já terminou; este checkpoint não é candidato a retomada.
    Concluida(CheckpointRetomada),
    ConferenciaObrigatoria(CheckpointRetomada),
}

/// Lê um checkpoint derivado sem tratá-lo como autorização para concluir a importação.
/// Ausência é normal para casos antigos; formato inválido é reportado para recuperação
/// manual, nunca convertido em sucesso silencioso.
pub fn ler_checkpoint(caminho: &Path) -> Resultado<Option<CheckpointRetomada>> {
    if !caminho.is_file() {
        return Ok(None);
    }
    let dados = fs::read(caminho)?;
    let checkpoint: CheckpointRetomada = serde_json::from_slice(&dados)
        .map_err(|e| ErroIngestao::Interno(format!("checkpoint inválido: {e}")))?;
    if checkpoint.schema != 1
        || !matches!(
            checkpoint.estado.as_str(),
            "iniciada"
                | "inventariando"
                | "inventario_concluido"
                | "concluida"
                | "cancelada"
                | "interrompida"
        )
        || checkpoint.workers_hash == 0
        || checkpoint.workers_hash > 8
        || checkpoint.limite_arquivos == 0
        || checkpoint.limite_bytes == 0
        || checkpoint.arquivos_inventariados > checkpoint.limite_arquivos
        || checkpoint.bytes_lidos > checkpoint.limite_bytes
    {
        return Err(ErroIngestao::Interno(
            "checkpoint fora do contrato de recuperação".into(),
        ));
    }
    Ok(Some(checkpoint))
}

/// Avalia se um checkpoint pertence à mesma fonte antes de qualquer tentativa manual
/// de recuperação. O resultado nunca executa a retomada nem libera gravações existentes.
pub fn avaliar_retomada(caminho: &Path, fonte_atual: &str) -> Resultado<AvaliacaoRetomada> {
    let Some(checkpoint) = ler_checkpoint(caminho)? else {
        return Ok(AvaliacaoRetomada::SemCheckpoint);
    };
    if checkpoint.fonte != fonte_atual {
        return Ok(AvaliacaoRetomada::FonteDivergente {
            registrada: checkpoint.fonte,
            atual: fonte_atual.into(),
        });
    }
    if checkpoint.estado == "concluida" {
        return Ok(AvaliacaoRetomada::Concluida(checkpoint));
    }
    Ok(AvaliacaoRetomada::ConferenciaObrigatoria(checkpoint))
}

fn gravar_checkpoint(
    ctx: &ContextoImportacao,
    fonte: &str,
    estado: &str,
    arquivos: u64,
    bytes: u64,
    limites: LimitesRecursos,
) -> Resultado<()> {
    let checkpoint = CheckpointRetomada {
        schema: 1,
        estado: estado.into(),
        fonte: fonte.into(),
        atualizado_em: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        arquivos_inventariados: arquivos,
        bytes_lidos: bytes,
        workers_hash: limites.workers_hash,
        limite_arquivos: limites.max_arquivos,
        limite_bytes: limites.max_bytes,
    };
    let temporario = ctx.pasta_importacao.join(format!("{NOME_CHECKPOINT}.tmp"));
    let destino = ctx.pasta_importacao.join(NOME_CHECKPOINT);
    let conteudo = serde_json::to_vec_pretty(&checkpoint)
        .map_err(|e| ErroIngestao::Interno(format!("checkpoint inválido: {e}")))?;
    let mut arquivo = File::create(&temporario)?;
    arquivo.write_all(&conteudo)?;
    arquivo.sync_all()?;
    drop(arquivo);
    // O arquivo temporário evita checkpoint truncado mesmo em queda; em Windows a
    // substituição exige remover o destino existente, deixando no máximo uma janela
    // curta sem o derivado (nunca altera a evidência).
    let _ = fs::remove_file(&destino);
    fs::rename(temporario, destino)?;
    Ok(())
}

// ---------- Progresso e cancelamento ----------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Etapa {
    #[default]
    Preparando,
    Contando,
    Inventariando,
    Planejando,
    Processando,
    Concluida,
    Cancelada,
    Falhou,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct Progresso {
    pub etapa: Etapa,
    #[ts(type = "number")]
    pub arquivos_total: u64,
    #[ts(type = "number")]
    pub arquivos_feitos: u64,
    #[ts(type = "number")]
    pub bytes_total: u64,
    #[ts(type = "number")]
    pub bytes_feitos: u64,
    #[ts(type = "number")]
    pub tarefas_total: u64,
    #[ts(type = "number")]
    pub tarefas_feitas: u64,
    #[ts(type = "number")]
    pub artefatos: u64,
    #[ts(type = "number")]
    pub avisos: u64,
    pub item_atual: Option<String>,
    pub mensagem: Option<String>,
    /// Início da importação e da etapa atual (ms desde 1970, UTC): a tela mostra o
    /// tempo decorrido e a taxa sem inventar estimativa.
    #[ts(optional, type = "number")]
    pub inicio_ms: Option<i64>,
    #[ts(optional, type = "number")]
    pub etapa_inicio_ms: Option<i64>,
    /// No processamento: o que a tarefa atual está fazendo.
    #[ts(optional)]
    pub fase_tarefa: Option<FaseTarefa>,
    /// Cópia de trabalho em curso (bytes copiados e conferidos pelo SHA-256).
    #[ts(optional, type = "number")]
    pub copia_bytes_total: Option<u64>,
    #[ts(optional, type = "number")]
    pub copia_bytes_feitos: Option<u64>,
}

/// Passos de cada tarefa do processamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FaseTarefa {
    /// Copiando o banco para a área de trabalho do caso e conferindo o SHA-256.
    CopiandoVerificando,
    /// Parser lendo a cópia verificada e gravando registros.
    Interpretando,
}

fn agora_ms() -> i64 {
    Utc::now().timestamp_millis()
}

impl Progresso {
    /// Muda de etapa e marca quando ela começou.
    pub fn entrar(&mut self, etapa: Etapa) {
        self.etapa = etapa;
        self.etapa_inicio_ms = Some(agora_ms());
        if matches!(etapa, Etapa::Concluida | Etapa::Cancelada | Etapa::Falhou) {
            self.fase_tarefa = None;
            self.copia_bytes_total = None;
            self.copia_bytes_feitos = None;
        }
    }
}

/// Compartilhado entre a tarefa e quem a acompanha (interface).
#[derive(Default)]
pub struct Controle {
    cancelar: AtomicBool,
    progresso: Mutex<Progresso>,
    eventos: Mutex<RegistroEventos>,
}

impl Controle {
    pub fn pedir_cancelamento(&self) {
        self.cancelar.store(true, Ordering::SeqCst);
    }

    pub fn cancelado(&self) -> bool {
        self.cancelar.load(Ordering::SeqCst)
    }

    pub fn progresso(&self) -> Progresso {
        self.progresso.lock().map(|p| p.clone()).unwrap_or_default()
    }

    pub fn atualizar(&self, f: impl FnOnce(&mut Progresso)) {
        if let Ok(mut p) = self.progresso.lock() {
            f(&mut p);
        }
    }

    /// Acrescenta um evento ao console da importação (só em memória; o pipeline também
    /// o grava no log da importação).
    pub fn anunciar(&self, nivel: NivelEvento, mensagem: impl Into<String>) -> EventoImportacao {
        let mensagem = mensagem.into();
        match self.eventos.lock() {
            Ok(mut e) => e.adicionar(agora_ms(), nivel, mensagem),
            Err(_) => EventoImportacao {
                seq: 0,
                ts_ms: agora_ms(),
                nivel,
                mensagem,
            },
        }
    }

    /// Eventos do console depois da sequência `seq`.
    pub fn eventos_desde(&self, seq: u64) -> Vec<EventoImportacao> {
        self.eventos
            .lock()
            .map(|e| e.desde(seq))
            .unwrap_or_default()
    }

    fn verificar(&self) -> Resultado<()> {
        if self.cancelado() {
            Err(ErroIngestao::Cancelado)
        } else {
            Ok(())
        }
    }
}

// ---------- Destino (banco do caso) ----------

/// Um arquivo já gravado no inventário.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArquivoRegistrado {
    pub id: i64,
    pub sha256: Option<String>,
    pub tamanho: u64,
}

/// Referência de origem com o que o parser não sabe: qual arquivo do inventário e seu hash.
#[derive(Debug, Clone, PartialEq)]
pub struct OrigemResolvida {
    pub referencia: ReferenciaRegistro,
    pub principal: bool,
    pub arquivo_id: i64,
    pub arquivo_sha256: String,
}

/// Como o anexo informado pelo parser foi ligado (ou não) a um arquivo do inventário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum VinculoAnexo {
    /// O caminho gravado pelo app é exatamente o caminho na extração.
    CaminhoExato,
    /// Um único arquivo termina com as mesmas pastas e nome (2+ componentes) e o
    /// tamanho confere (quando o app informa o tamanho).
    SufixoCaminho,
    /// Um único arquivo com o mesmo nome e exatamente o tamanho informado pelo app.
    NomeETamanho,
    /// Mais de um arquivo serve, ou só o nome coincide e o app não informa o
    /// tamanho; nenhum é escolhido.
    Ambiguo,
    /// O único candidato tem tamanho diferente do informado pelo app.
    TamanhoDiferente,
    /// Nenhum arquivo com esse nome na extração.
    NaoEncontrado,
}

impl VinculoAnexo {
    pub fn como_str(self) -> &'static str {
        match self {
            VinculoAnexo::CaminhoExato => "caminho_exato",
            VinculoAnexo::SufixoCaminho => "sufixo_caminho",
            VinculoAnexo::NomeETamanho => "nome_e_tamanho",
            VinculoAnexo::Ambiguo => "ambiguo",
            VinculoAnexo::TamanhoDiferente => "tamanho_diferente",
            VinculoAnexo::NaoEncontrado => "nao_encontrado",
        }
    }
}

/// Arquivo do inventário com o mesmo nome do anexo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatoAnexo {
    pub id: i64,
    pub caminho: String,
    pub tamanho: u64,
}

fn componentes(caminho: &str) -> Vec<&str> {
    caminho
        .split(['/', '\\'])
        .filter(|p| !p.is_empty() && *p != ".")
        .collect()
}

/// Escolhe, entre os arquivos do inventário com o mesmo nome, o que corresponde ao
/// caminho gravado pelo app. O app guarda caminhos relativos à pasta dele
/// (`Media/WhatsApp Images/x.jpg`) ou absolutos no aparelho
/// (`/storage/emulated/0/...`); a extração os coloca sob outra raiz. Vence o
/// candidato que compartilha mais componentes finais com o caminho do app, desde que
/// seja o único com essa pontuação. Só o nome em comum exige tamanho igual.
pub fn escolher_anexo(
    caminho_app: &str,
    tamanho_app: Option<u64>,
    candidatos: &[CandidatoAnexo],
) -> (Option<i64>, VinculoAnexo) {
    let app = componentes(caminho_app);
    let comum = |c: &CandidatoAnexo| {
        componentes(&c.caminho)
            .iter()
            .rev()
            .zip(app.iter().rev())
            .take_while(|(a, b)| a == b)
            .count()
    };
    let pontos: Vec<(usize, &CandidatoAnexo)> = candidatos
        .iter()
        .map(|c| (comum(c), c))
        .filter(|(n, _)| *n > 0)
        .collect();
    let Some(melhor) = pontos.iter().map(|(n, _)| *n).max() else {
        return (None, VinculoAnexo::NaoEncontrado);
    };
    let empatados: Vec<&CandidatoAnexo> = pontos
        .iter()
        .filter(|(n, _)| *n == melhor)
        .map(|(_, c)| *c)
        .collect();
    let [unico] = empatados.as_slice() else {
        return (None, VinculoAnexo::Ambiguo);
    };
    match (melhor >= 2, tamanho_app) {
        (_, Some(t)) if t != unico.tamanho => (None, VinculoAnexo::TamanhoDiferente),
        (true, _) => (Some(unico.id), VinculoAnexo::SufixoCaminho),
        (false, Some(_)) => (Some(unico.id), VinculoAnexo::NomeETamanho),
        (false, None) => (None, VinculoAnexo::Ambiguo),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtefatoParaGravar {
    pub artefato: NovoArtefato,
    pub origens: Vec<OrigemResolvida>,
    pub anexo_arquivo_id: Option<i64>,
    /// Presente quando o artefato informa um anexo.
    pub anexo_vinculo: Option<VinculoAnexo>,
    /// SHA-256 do artefato normalizado (JSON canônico), para detectar alteração.
    pub hash_registro: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EstadoExecucao {
    Ok,
    /// Terminou, mas com avisos (registros não interpretados).
    Parcial,
    Falhou,
    Cancelada,
}

impl EstadoExecucao {
    pub fn como_str(self) -> &'static str {
        match self {
            EstadoExecucao::Ok => "ok",
            EstadoExecucao::Parcial => "parcial",
            EstadoExecucao::Falhou => "falhou",
            EstadoExecucao::Cancelada => "cancelada",
        }
    }
}

pub trait Destino {
    fn gravar_arquivos(&mut self, lote: &[ArquivoInventariado]) -> Resultado<()>;
    fn arquivo(&mut self, caminho: &CaminhoFonte) -> Resultado<Option<ArquivoRegistrado>>;
    /// Arquivos legíveis (com hash) desta extração com exatamente este nome. Usado para
    /// ligar anexos cujo caminho no app não é o caminho na extração.
    fn arquivos_com_nome(&mut self, _nome: &str) -> Resultado<Vec<CandidatoAnexo>> {
        Ok(Vec::new())
    }
    fn iniciar_execucao(
        &mut self,
        parser: &InfoComponente,
        arquivo_principal: Option<i64>,
        copia_sha256: Option<&str>,
    ) -> Resultado<i64>;
    fn gravar_artefatos(&mut self, execucao: i64, lote: &[ArtefatoParaGravar]) -> Resultado<()>;
    fn gravar_aviso(&mut self, execucao: i64, aviso: &AvisoParser) -> Resultado<()>;
    fn concluir_execucao(
        &mut self,
        execucao: i64,
        estado: EstadoExecucao,
        resumo: &ResumoParser,
        erro: Option<&str>,
    ) -> Resultado<()>;
}

// ---------- Resultado ----------

/// Onde a importação grava seus arquivos (na pasta do caso, nunca na extração) e o
/// fuso do caso.
#[derive(Debug, Clone)]
pub struct ContextoImportacao {
    /// `importacoes/<id>/`: manifesto e log.
    pub pasta_importacao: PathBuf,
    /// `cache/bancos/<id>/`: cópias de trabalho verificadas.
    pub pasta_copias: PathBuf,
    pub fuso_horario: String,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct ResumoImportacao {
    #[ts(type = "number")]
    pub arquivos: u64,
    #[ts(type = "number")]
    pub diretorios: u64,
    #[ts(type = "number")]
    pub bytes: u64,
    /// Arquivos listados mas que não puderam ser lidos (sem hash).
    #[ts(type = "number")]
    pub ilegiveis: u64,
    /// Itens não inventariados (links simbólicos, nomes inválidos...).
    #[ts(type = "number")]
    pub ignorados: u64,
    pub hash_manifesto: String,
    #[ts(type = "number")]
    pub execucoes: u64,
    #[ts(type = "number")]
    pub execucoes_com_falha: u64,
    #[ts(type = "number")]
    pub artefatos: u64,
    #[ts(type = "number")]
    pub avisos: u64,
    pub ferramenta_versao: Option<String>,
    pub tipo_extracao: Option<String>,
    pub observacoes: Vec<String>,
}

// ---------- Log da importação (JSON por linha) ----------

struct Log {
    arquivo: BufWriter<File>,
}

impl Log {
    fn criar(pasta: &Path) -> Resultado<Self> {
        Ok(Log {
            arquivo: BufWriter::new(File::create(pasta.join(NOME_LOG))?),
        })
    }

    fn registrar(&mut self, nivel: &str, evento: &str, dados: serde_json::Value) {
        let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let linha = json!({ "ts": ts, "nivel": nivel, "evento": evento, "dados": dados });
        // O log detalhado é auxiliar: falha ao escrever não interrompe a importação.
        let _ = writeln!(self.arquivo, "{linha}");
    }

    /// Evento que aparece no console da interface e fica no log com a mesma mensagem.
    fn anunciar(
        &mut self,
        controle: &Controle,
        nivel: NivelEvento,
        evento: &str,
        mensagem: String,
        dados: serde_json::Value,
    ) {
        let e = controle.anunciar(nivel, mensagem);
        let ts = chrono::DateTime::<Utc>::from_timestamp_millis(e.ts_ms)
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let nivel_log = match nivel {
            NivelEvento::Info | NivelEvento::Ok => "info",
            NivelEvento::Aviso => "aviso",
            NivelEvento::Erro => "erro",
        };
        let linha = json!({
            "ts": ts, "nivel": nivel_log, "evento": evento, "dados": dados,
            "mensagem": e.mensagem, "tom": nivel.codigo(),
        });
        let _ = writeln!(self.arquivo, "{linha}");
    }
}

impl Drop for Log {
    fn drop(&mut self) {
        let _ = self.arquivo.flush();
    }
}

// ---------- Execução ----------

pub fn executar(
    fonte: &dyn FonteSomenteLeitura,
    importador: &dyn Importador,
    registro: &Registro,
    destino: &mut dyn Destino,
    controle: &Controle,
    ctx: &ContextoImportacao,
) -> Resultado<ResumoImportacao> {
    controle.atualizar(|p| {
        p.inicio_ms = Some(agora_ms());
        p.etapa_inicio_ms = p.inicio_ms;
    });
    fs::create_dir_all(&ctx.pasta_importacao)?;
    let limites = limites_recursos();
    let fonte_descricao = fonte.descricao();
    gravar_checkpoint(ctx, &fonte_descricao, "iniciada", 0, 0, limites)?;
    let mut log = Log::criar(&ctx.pasta_importacao)?;
    let info = importador.info();
    log.anunciar(
        controle,
        NivelEvento::Info,
        "inicio",
        format!(
            "Iniciando importação com {} {}. Origem aberta somente para leitura: {}",
            info.nome, info.versao, fonte_descricao
        ),
        json!({ "fonte": fonte_descricao.clone(), "importador": info, "limites": limites }),
    );
    let mut r = executar_etapas(
        fonte, importador, registro, destino, controle, ctx, &mut log,
    );
    if r.is_ok() {
        let progresso = controle.progresso();
        if let Err(e) = gravar_checkpoint(
            ctx,
            &fonte_descricao,
            "concluida",
            progresso.arquivos_feitos,
            progresso.bytes_feitos,
            limites,
        ) {
            if let Ok(resumo) = &mut r {
                resumo.avisos = resumo.avisos.saturating_add(1);
                observar(
                    resumo,
                    format!("checkpoint final não persistido; a importação exige conferência: {e}"),
                );
            }
            log.anunciar(
                controle,
                NivelEvento::Aviso,
                "checkpoint_final",
                format!("Importação concluída, mas não foi possível atualizar o checkpoint: {e}"),
                json!({ "erro": e.to_string() }),
            );
        }
    }
    match &r {
        Ok(resumo) => log.anunciar(
            controle,
            if resumo.execucoes_com_falha > 0 || resumo.avisos > 0 {
                NivelEvento::Aviso
            } else {
                NivelEvento::Ok
            },
            "fim",
            format!(
                "Importação concluída: {} arquivo(s) inventariados, {} registro(s) de {} leitura(s) de banco, {} aviso(s){}.",
                numero_br(resumo.arquivos),
                numero_br(resumo.artefatos),
                numero_br(resumo.execucoes),
                numero_br(resumo.avisos),
                if resumo.execucoes_com_falha > 0 {
                    format!(", {} leitura(s) com falha", numero_br(resumo.execucoes_com_falha))
                } else {
                    String::new()
                }
            ),
            json!(resumo),
        ),
        Err(ErroIngestao::Cancelado) => log.anunciar(
            controle,
            NivelEvento::Aviso,
            "fim",
            "Importação cancelada pelo usuário. O que já foi gravado continua no caso, marcado como cancelado.".into(),
            json!({ "erro": ErroIngestao::Cancelado.to_string() }),
        ),
        Err(e) => log.anunciar(
            controle,
            NivelEvento::Erro,
            "fim",
            format!("Importação interrompida: {e}"),
            json!({ "erro": e.to_string() }),
        ),
    }
    if let Err(e) = &r {
        let estado = if matches!(e, &ErroIngestao::Cancelado) {
            "cancelada"
        } else {
            "interrompida"
        };
        let progresso = controle.progresso();
        let _ = gravar_checkpoint(
            ctx,
            &fonte_descricao,
            estado,
            progresso.arquivos_feitos,
            progresso.bytes_feitos,
            limites,
        );
    }
    r
}

fn executar_etapas(
    fonte: &dyn FonteSomenteLeitura,
    importador: &dyn Importador,
    registro: &Registro,
    destino: &mut dyn Destino,
    controle: &Controle,
    ctx: &ContextoImportacao,
    log: &mut Log,
) -> Resultado<ResumoImportacao> {
    let mut resumo = ResumoImportacao::default();
    let limites = limites_recursos();

    // 1. Contar
    controle.atualizar(|p| p.entrar(Etapa::Contando));
    let (mut n, mut bytes) = (0u64, 0u64);
    for e in fonte.listar().flatten() {
        if !e.diretorio {
            n += 1;
            bytes += e.tamanho;
            if n > limites.max_arquivos {
                return Err(ErroIngestao::LimiteRecursos {
                    recurso: "arquivos".into(),
                    limite: limites.max_arquivos,
                    observado: n,
                });
            }
            if bytes > limites.max_bytes {
                return Err(ErroIngestao::LimiteRecursos {
                    recurso: "bytes".into(),
                    limite: limites.max_bytes,
                    observado: bytes,
                });
            }
            if n % 1000 == 0 {
                controle.verificar()?;
                controle.atualizar(|p| (p.arquivos_total, p.bytes_total) = (n, bytes));
            }
        }
    }
    controle.atualizar(|p| (p.arquivos_total, p.bytes_total) = (n, bytes));
    log.anunciar(
        controle,
        NivelEvento::Info,
        "contagem",
        format!(
            "{} arquivo(s) encontrados, {} no total.",
            numero_br(n),
            tamanho_br(bytes)
        ),
        json!({ "arquivos": n, "bytes": bytes }),
    );

    // 2. Inventariar + manifesto
    controle.atualizar(|p| p.entrar(Etapa::Inventariando));
    log.anunciar(
        controle,
        NivelEvento::Info,
        "inventario_inicio",
        "Calculando o SHA-256 de cada arquivo e montando o inventário...".into(),
        json!({}),
    );
    inventariar(fonte, destino, controle, ctx, log, &mut resumo, n)?;

    // 3. Planejar
    controle.atualizar(|p| {
        p.entrar(Etapa::Planejando);
        p.item_atual = None;
    });
    let plano = importador.planejar(fonte)?;
    log.registrar("info", "plano", json!(plano));
    log.anunciar(
        controle,
        NivelEvento::Info,
        "plano_resumo",
        if plano.tarefas.is_empty() {
            "Nenhum banco de aplicativo reconhecido para interpretar nesta extração.".into()
        } else {
            format!(
                "Plano do importador: {} leitura(s) de banco por parser.",
                numero_br(plano.tarefas.len() as u64)
            )
        },
        json!({ "tarefas": plano.tarefas.len() }),
    );
    let PlanoImportacao {
        ferramenta_versao,
        tipo_extracao,
        tarefas,
        observacoes,
        ..
    } = plano;
    resumo.ferramenta_versao = ferramenta_versao;
    resumo.tipo_extracao = tipo_extracao;
    for o in observacoes {
        observar(&mut resumo, o);
    }

    // 4. Processar
    controle.atualizar(|p| {
        p.entrar(Etapa::Processando);
        p.tarefas_total = tarefas.len() as u64;
    });
    for tarefa in &tarefas {
        controle.verificar()?;
        processar_tarefa(
            fonte,
            registro,
            destino,
            controle,
            ctx,
            log,
            tarefa,
            &mut resumo,
        )?;
        controle.atualizar(|p| p.tarefas_feitas += 1);
    }
    controle.atualizar(|p| {
        p.entrar(Etapa::Concluida);
        p.item_atual = None;
    });
    Ok(resumo)
}

fn observar(resumo: &mut ResumoImportacao, texto: String) {
    if resumo.observacoes.len() < MAX_OBSERVACOES {
        resumo.observacoes.push(texto);
    } else if resumo.observacoes.len() == MAX_OBSERVACOES {
        resumo
            .observacoes
            .push("… demais observações no log da importação".into());
    }
}

fn transformar_leitura(
    entrada: EntradaFonte,
    leitura: Resultado<inventario::Leitura>,
    manifesto: &mut BufWriter<File>,
    controle: &Controle,
    log: &mut Log,
    resumo: &mut ResumoImportacao,
    limites: LimitesRecursos,
) -> Resultado<ArquivoInventariado> {
    match leitura {
        Ok(l) => {
            let k = inventario::classificar(&entrada.caminho, l.bytes, &l.cabecalho);
            manifesto
                .write_all(inventario::linha_manifesto(&l.sha256, &entrada.caminho).as_bytes())?;
            resumo.bytes = resumo.bytes.saturating_add(l.bytes);
            if resumo.bytes > limites.max_bytes {
                return Err(ErroIngestao::LimiteRecursos {
                    recurso: "bytes".into(),
                    limite: limites.max_bytes,
                    observado: resumo.bytes,
                });
            }
            let mut entrada = entrada;
            entrada.tamanho = l.bytes;
            Ok(ArquivoInventariado {
                entrada,
                sha256: Some(l.sha256),
                categoria: k.categoria,
                assinatura: k.assinatura,
                mime: k.mime,
                erro: None,
            })
        }
        Err(ErroIngestao::Cancelado) => Err(ErroIngestao::Cancelado),
        Err(e) => {
            resumo.ilegiveis += 1;
            let caminho = entrada.caminho.to_string();
            log.anunciar(
                controle,
                NivelEvento::Erro,
                "leitura",
                format!(
                    "Não foi possível ler {caminho}: {e}. O arquivo fica no inventário sem hash."
                ),
                json!({ "caminho": caminho.clone(), "erro": e.to_string() }),
            );
            observar(resumo, format!("não foi possível ler {caminho}: {e}"));
            let k = inventario::classificar(&entrada.caminho, 1, &[]);
            Ok(ArquivoInventariado {
                entrada,
                sha256: None,
                categoria: k.categoria,
                assinatura: None,
                mime: None,
                erro: Some(e.to_string()),
            })
        }
    }
}

fn registrar_arquivo(
    arq: ArquivoInventariado,
    destino: &mut dyn Destino,
    controle: &Controle,
    log: &mut Log,
    resumo: &mut ResumoImportacao,
    lote: &mut Vec<ArquivoInventariado>,
    bancos: &mut u64,
    total: u64,
    passo: u64,
) -> Resultado<()> {
    resumo.arquivos += 1;
    if arq.categoria == inventario::Categoria::BancoSqlite {
        *bancos += 1;
        if *bancos <= MAX_BANCOS_ANUNCIADOS {
            log.anunciar(
                controle,
                NivelEvento::Info,
                "banco_localizado",
                format!("Banco SQLite localizado: {}", arq.entrada.caminho),
                json!({ "caminho": arq.entrada.caminho, "sha256": arq.sha256 }),
            );
        }
    }
    lote.push(arq);
    controle.atualizar(|p| p.arquivos_feitos += 1);
    if resumo.arquivos.is_multiple_of(passo) && resumo.arquivos < total {
        log.anunciar(
            controle,
            NivelEvento::Info,
            "inventario_progresso",
            format!(
                "{} de {} arquivo(s) inventariados ({}%).",
                numero_br(resumo.arquivos),
                numero_br(total),
                resumo.arquivos * 100 / total.max(1)
            ),
            json!({ "feitos": resumo.arquivos, "total": total }),
        );
    }
    if lote.len() >= LOTE {
        destino.gravar_arquivos(lote)?;
        lote.clear();
    }
    Ok(())
}

fn processar_lote_hash(
    fonte: &dyn FonteSomenteLeitura,
    entradas: Vec<EntradaFonte>,
    limites: LimitesRecursos,
    destino: &mut dyn Destino,
    controle: &Controle,
    manifesto: &mut BufWriter<File>,
    log: &mut Log,
    resumo: &mut ResumoImportacao,
    lote: &mut Vec<ArquivoInventariado>,
    bancos: &mut u64,
    total: u64,
    passo: u64,
) -> Resultado<()> {
    let resultados =
        inventario::ler_varios_com_hash(fonte, entradas, limites.workers_hash, &controle.cancelar);
    for (entrada, leitura) in resultados {
        controle.verificar()?;
        let arq = transformar_leitura(entrada, leitura, manifesto, controle, log, resumo, limites)?;
        registrar_arquivo(
            arq, destino, controle, log, resumo, lote, bancos, total, passo,
        )?;
    }
    Ok(())
}

fn inventariar(
    fonte: &dyn FonteSomenteLeitura,
    destino: &mut dyn Destino,
    controle: &Controle,
    ctx: &ContextoImportacao,
    log: &mut Log,
    resumo: &mut ResumoImportacao,
    total: u64,
) -> Resultado<()> {
    let caminho_manifesto = ctx.pasta_importacao.join(NOME_MANIFESTO);
    let limites = limites_recursos();
    let mut bancos = 0u64;
    // Marco de progresso a cada 10% dos arquivos (ao menos a cada arquivo).
    let passo = (total / 10).max(1);
    let mut manifesto = BufWriter::new(File::create(&caminho_manifesto)?);
    let mut lote: Vec<ArquivoInventariado> = Vec::with_capacity(LOTE);
    let janela_hash = (limites.workers_hash * 4).max(1);
    let mut pendentes_hash = Vec::with_capacity(janela_hash);

    for item in fonte.listar() {
        controle.verificar()?;
        let entrada = match item {
            Ok(e) => e,
            Err(ErroIngestao::ItemIgnorado { caminho, motivo }) => {
                resumo.ignorados += 1;
                log.anunciar(
                    controle,
                    NivelEvento::Aviso,
                    "item_ignorado",
                    format!("Item não inventariado: {caminho} ({motivo})."),
                    json!({ "caminho": caminho, "motivo": motivo }),
                );
                observar(resumo, format!("ignorado: {caminho} — {motivo}"));
                continue;
            }
            Err(e) => return Err(e),
        };
        if entrada.diretorio {
            resumo.diretorios += 1;
            continue;
        }
        if resumo.arquivos + 1 > limites.max_arquivos {
            return Err(ErroIngestao::LimiteRecursos {
                recurso: "arquivos".into(),
                limite: limites.max_arquivos,
                observado: resumo.arquivos + 1,
            });
        }
        controle.atualizar(|p| p.item_atual = Some(entrada.caminho.to_string()));
        if limites.workers_hash > 1 {
            pendentes_hash.push(entrada);
            if pendentes_hash.len() >= janela_hash {
                processar_lote_hash(
                    fonte,
                    std::mem::take(&mut pendentes_hash),
                    limites,
                    destino,
                    controle,
                    &mut manifesto,
                    log,
                    resumo,
                    &mut lote,
                    &mut bancos,
                    total,
                    passo,
                )?;
            }
            continue;
        }
        let leitura = fonte.abrir(&entrada.caminho).and_then(|mut l| {
            inventario::ler_com_hash(&mut l, None, |b| {
                controle.atualizar(|p| p.bytes_feitos += b);
                !controle.cancelado()
            })
        });
        let arq = transformar_leitura(
            entrada,
            leitura,
            &mut manifesto,
            controle,
            log,
            resumo,
            limites,
        )?;
        registrar_arquivo(
            arq,
            destino,
            controle,
            log,
            resumo,
            &mut lote,
            &mut bancos,
            total,
            passo,
        )?;
        if resumo.arquivos.is_multiple_of(LOTE as u64) {
            gravar_checkpoint(
                ctx,
                &fonte.descricao(),
                "inventariando",
                resumo.arquivos,
                resumo.bytes,
                limites,
            )?;
        }
    }
    if !pendentes_hash.is_empty() {
        processar_lote_hash(
            fonte,
            pendentes_hash,
            limites,
            destino,
            controle,
            &mut manifesto,
            log,
            resumo,
            &mut lote,
            &mut bancos,
            total,
            passo,
        )?;
    }
    if !lote.is_empty() {
        destino.gravar_arquivos(&lote)?;
    }
    manifesto.flush()?;
    manifesto.get_ref().sync_all()?;
    drop(manifesto);
    gravar_checkpoint(
        ctx,
        &fonte.descricao(),
        "inventario_concluido",
        resumo.arquivos,
        resumo.bytes,
        limites,
    )?;

    let mut f = File::open(&caminho_manifesto)?;
    resumo.hash_manifesto = inventario::ler_com_hash(&mut f, None, |_| true)?.sha256;
    if bancos > MAX_BANCOS_ANUNCIADOS {
        log.anunciar(
            controle,
            NivelEvento::Info,
            "bancos_localizados",
            format!(
                "… e mais {} banco(s) SQLite (todos no inventário).",
                numero_br(bancos - MAX_BANCOS_ANUNCIADOS)
            ),
            json!({ "bancos": bancos }),
        );
    }
    log.anunciar(
        controle,
        if resumo.ilegiveis > 0 { NivelEvento::Aviso } else { NivelEvento::Ok },
        "manifesto",
        format!(
            "Inventário concluído: {} arquivo(s), {} ({} ilegível(is), {} ignorado(s)). Manifesto gravado, SHA-256 {}.",
            numero_br(resumo.arquivos),
            tamanho_br(resumo.bytes),
            numero_br(resumo.ilegiveis),
            numero_br(resumo.ignorados),
            resumo.hash_manifesto
        ),
        json!({ "arquivo": caminho_manifesto, "sha256": resumo.hash_manifesto }),
    );
    Ok(())
}

/// Papel do arquivo → (id no inventário, SHA-256).
type PorPapel = HashMap<String, (i64, String)>;

/// Copia um arquivo da extração para `pasta`, conferindo que o hash da cópia é o do
/// inventário. Se o original mudou desde o inventário, a cópia é descartada.
fn copiar_verificado(
    fonte: &dyn FonteSomenteLeitura,
    controle: &Controle,
    caminho: &CaminhoFonte,
    esperado: &str,
    pasta: &Path,
) -> Resultado<PathBuf> {
    fs::create_dir_all(pasta)?;
    let destino = pasta.join(caminho.nome());
    let mut f = File::create(&destino)?;
    let mut leitor = fonte.abrir(caminho)?;
    let total = fonte.entrada(caminho)?.map(|e| e.tamanho);
    controle.atualizar(|p| {
        p.fase_tarefa = Some(FaseTarefa::CopiandoVerificando);
        p.copia_bytes_total = total;
        p.copia_bytes_feitos = Some(0);
    });
    let lido = inventario::ler_com_hash(&mut leitor, Some(&mut f), |b| {
        controle.atualizar(|p| p.copia_bytes_feitos = Some(p.copia_bytes_feitos.unwrap_or(0) + b));
        !controle.cancelado()
    })?;
    f.sync_all()?;
    drop(f);
    if lido.sha256 != esperado {
        let _ = fs::remove_file(&destino);
        return Err(ErroIngestao::OrigemAlterada {
            caminho: caminho.to_string(),
            esperado: esperado.into(),
            lido: lido.sha256,
        });
    }
    Ok(destino)
}

/// Prepara as cópias de trabalho de uma tarefa. Devolve os arquivos para o parser,
/// por papel (id no inventário, sha256), e avisos sobre auxiliares que não puderam
/// ser lidos.
fn preparar_entrada(
    fonte: &dyn FonteSomenteLeitura,
    destino: &mut dyn Destino,
    controle: &Controle,
    ctx: &ContextoImportacao,
    log: &mut Log,
    tarefa: &TarefaParser,
) -> Resultado<(Vec<ArquivoDeTrabalho>, PorPapel, Vec<AvisoParser>)> {
    let mut arquivos = Vec::new();
    let mut por_papel = HashMap::new();
    let mut avisos = Vec::new();
    for a in &tarefa.arquivos {
        let reg = destino
            .arquivo(&a.caminho)?
            .ok_or_else(|| ErroIngestao::Parser {
                parser: tarefa.parser_id.clone(),
                mensagem: format!("{} não está no inventário", a.caminho),
            })?;
        let sha = reg.sha256.clone().ok_or_else(|| ErroIngestao::Parser {
            parser: tarefa.parser_id.clone(),
            mensagem: format!("{} não pôde ser lido no inventário", a.caminho),
        })?;
        let pasta = ctx.pasta_copias.join(reg.id.to_string());
        log.anunciar(
            controle,
            NivelEvento::Info,
            "copia_inicio",
            format!(
                "Copiando {} para a área de trabalho do caso ({}).",
                a.caminho,
                tamanho_br(reg.tamanho)
            ),
            json!({ "caminho": a.caminho }),
        );
        let copia = copiar_verificado(fonte, controle, &a.caminho, &sha, &pasta)?;
        log.anunciar(
            controle,
            NivelEvento::Ok,
            "copia_conferida",
            format!("Cópia conferida: SHA-256 igual ao do inventário ({sha})."),
            json!({ "caminho": a.caminho, "sha256": sha }),
        );
        for sufixo in AUXILIARES_SQLITE {
            let aux = CaminhoFonte::novo(&format!("{}{sufixo}", a.caminho))?;
            match destino.arquivo(&aux)? {
                Some(ArquivoRegistrado {
                    sha256: Some(s), ..
                }) => {
                    copiar_verificado(fonte, controle, &aux, &s, &pasta)?;
                    log.anunciar(
                        controle,
                        NivelEvento::Ok,
                        "copia_conferida",
                        format!("Arquivo auxiliar {aux} copiado junto e conferido pelo SHA-256."),
                        json!({ "caminho": aux, "sha256": s }),
                    );
                }
                // Consta na extração mas não pôde ser lido no inventário: o banco é
                // interpretado sem ele, e isso tem de aparecer (registros recentes que
                // só estão no -wal/-journal ficariam de fora).
                Some(_) => avisos.push(AvisoParser {
                    mensagem: format!(
                        "{aux} consta na extração mas não pôde ser lido; {} foi interpretado \
                         sem ele e registros que só estejam nele podem faltar",
                        a.caminho
                    ),
                    papel_arquivo: Some(a.papel.clone()),
                    id_registro: None,
                }),
                None => {}
            }
        }
        por_papel.insert(a.papel.clone(), (reg.id, sha.clone()));
        arquivos.push(ArquivoDeTrabalho {
            papel: a.papel.clone(),
            origem: a.caminho.clone(),
            sha256: sha,
            copia_verificada: copia,
        });
    }
    Ok((arquivos, por_papel, avisos))
}

#[allow(clippy::too_many_arguments)]
fn processar_tarefa(
    fonte: &dyn FonteSomenteLeitura,
    registro: &Registro,
    destino: &mut dyn Destino,
    controle: &Controle,
    ctx: &ContextoImportacao,
    log: &mut Log,
    tarefa: &TarefaParser,
    resumo: &mut ResumoImportacao,
) -> Resultado<()> {
    let Some(parser) = registro.parser(&tarefa.parser_id) else {
        let msg = format!("parser '{}' não disponível nesta versão", tarefa.parser_id);
        log.anunciar(
            controle,
            NivelEvento::Aviso,
            "parser_ausente",
            format!("Leitura não executada: {msg}."),
            json!({ "tarefa": tarefa, "mensagem": msg }),
        );
        observar(resumo, msg);
        return Ok(());
    };
    let info = parser.info();
    let principal = tarefa
        .arquivos
        .first()
        .map(|a| a.caminho.to_string())
        .unwrap_or_default();
    controle.atualizar(|p| p.item_atual = Some(format!("{} — {principal}", info.nome)));
    resumo.execucoes += 1;

    let (arquivos, por_papel, avisos_entrada) =
        match preparar_entrada(fonte, destino, controle, ctx, log, tarefa) {
            Ok(x) => x,
            Err(ErroIngestao::Cancelado) => return Err(ErroIngestao::Cancelado),
            Err(e) => {
                // Registra a execução como falha (por exemplo, original alterado) e segue.
                let principal_id = match tarefa.arquivos.first() {
                    Some(a) => destino.arquivo(&a.caminho)?.map(|r| r.id),
                    None => None,
                };
                let exec = destino.iniciar_execucao(&info, principal_id, None)?;
                destino.concluir_execucao(
                    exec,
                    EstadoExecucao::Falhou,
                    &ResumoParser::default(),
                    Some(&e.to_string()),
                )?;
                resumo.execucoes_com_falha += 1;
                log.anunciar(
                    controle,
                    NivelEvento::Erro,
                    "copia_de_trabalho",
                    format!("{} não foi executado: {e}", info.nome),
                    json!({ "parser": info, "erro": e.to_string() }),
                );
                observar(resumo, format!("{}: {e}", info.nome));
                return Ok(());
            }
        };
    let principal = arquivos
        .first()
        .and_then(|a| por_papel.get(&a.papel).cloned());
    let exec = destino.iniciar_execucao(
        &info,
        principal.as_ref().map(|p| p.0),
        principal.as_ref().map(|p| p.1.as_str()),
    )?;
    log.anunciar(
        controle,
        NivelEvento::Info,
        "parser_inicio",
        format!(
            "Executando {} {} sobre a cópia verificada de {}.",
            info.nome,
            info.versao,
            arquivos
                .first()
                .map(|a| a.origem.to_string())
                .unwrap_or_default()
        ),
        json!({ "parser": info, "execucao": exec, "arquivos": arquivos }),
    );

    controle.atualizar(|p| {
        p.fase_tarefa = Some(FaseTarefa::Interpretando);
        p.copia_bytes_total = None;
        p.copia_bytes_feitos = None;
    });
    let entrada = EntradaParser {
        arquivos,
        fuso_horario: ctx.fuso_horario.clone(),
    };
    let mut coletor = Coletor {
        destino,
        controle,
        log: &mut *log,
        parser_nome: info.nome.clone(),
        execucao: exec,
        por_papel: &por_papel,
        lote: Vec::with_capacity(LOTE),
        falha_gravacao: None,
        emitidos: 0,
        avisos: 0,
    };
    for aviso in avisos_entrada {
        coletor.avisar(aviso);
    }
    // Um parser que entra em pânico com dados hostis não derruba a importação: a
    // execução fica registrada como falha e os demais parsers seguem.
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        parser.processar(&entrada, &mut coletor)
    }))
    .unwrap_or_else(|p| {
        Err(ErroIngestao::Parser {
            parser: info.id.clone(),
            mensagem: format!(
                "falha inesperada ao interpretar os dados ({})",
                texto_do_panico(p.as_ref())
            ),
        })
    });
    // O que o parser emitiu antes de uma falha também é gravado.
    let descarga = coletor.descarregar();
    let r = match (coletor.falha_gravacao.take(), descarga) {
        (Some(e), _) | (None, Err(e)) => Err(e),
        (None, Ok(())) => r,
    };
    let (emitidos, avisos) = (coletor.emitidos, coletor.avisos);
    resumo.artefatos += emitidos;
    resumo.avisos += avisos;
    let contagem = ResumoParser {
        artefatos: emitidos,
        avisos,
    };
    match r {
        Ok(_) => {
            let estado = if avisos > 0 {
                EstadoExecucao::Parcial
            } else {
                EstadoExecucao::Ok
            };
            destino.concluir_execucao(exec, estado, &contagem, None)?;
            log.anunciar(
                controle,
                if avisos > 0 {
                    NivelEvento::Aviso
                } else {
                    NivelEvento::Ok
                },
                "parser_fim",
                format!(
                    "{} concluído: {} registro(s) gravados e indexados, {} aviso(s).",
                    info.nome,
                    numero_br(emitidos),
                    numero_br(avisos)
                ),
                json!({ "execucao": exec, "estado": estado, "resumo": contagem }),
            );
            Ok(())
        }
        Err(ErroIngestao::Cancelado) => {
            destino.concluir_execucao(exec, EstadoExecucao::Cancelada, &contagem, None)?;
            Err(ErroIngestao::Cancelado)
        }
        Err(e @ ErroIngestao::Gravacao(_)) => {
            let _ = destino.concluir_execucao(
                exec,
                EstadoExecucao::Falhou,
                &contagem,
                Some(&e.to_string()),
            );
            Err(e)
        }
        Err(e) => {
            destino.concluir_execucao(
                exec,
                EstadoExecucao::Falhou,
                &contagem,
                Some(&e.to_string()),
            )?;
            resumo.execucoes_com_falha += 1;
            log.anunciar(
                controle,
                NivelEvento::Erro,
                "parser_fim",
                format!(
                    "{} terminou com falha depois de {} registro(s): {e}",
                    info.nome,
                    numero_br(emitidos)
                ),
                json!({ "execucao": exec, "erro": e.to_string() }),
            );
            observar(resumo, format!("{}: {e}", info.nome));
            Ok(())
        }
    }
}

/// Mensagem de um `panic!` capturado (texto fixo quando não é uma string).
pub fn texto_do_panico(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "sem detalhe".into())
}

/// Recebe artefatos do parser, completa a origem e grava em lotes.
struct Coletor<'a> {
    destino: &'a mut dyn Destino,
    controle: &'a Controle,
    log: &'a mut Log,
    parser_nome: String,
    execucao: i64,
    por_papel: &'a PorPapel,
    lote: Vec<ArtefatoParaGravar>,
    falha_gravacao: Option<ErroIngestao>,
    emitidos: u64,
    avisos: u64,
}

impl Coletor<'_> {
    fn resolver(&self, r: ReferenciaRegistro, principal: bool) -> Result<OrigemResolvida, String> {
        match self.por_papel.get(&r.papel_arquivo) {
            Some((id, sha)) => Ok(OrigemResolvida {
                referencia: r,
                principal,
                arquivo_id: *id,
                arquivo_sha256: sha.clone(),
            }),
            None => Err(format!(
                "papel de arquivo desconhecido: {}",
                r.papel_arquivo
            )),
        }
    }

    fn vincular_anexo(
        &mut self,
        caminho: &str,
        tamanho: Option<u64>,
    ) -> Resultado<(Option<i64>, VinculoAnexo)> {
        if let Ok(c) = CaminhoFonte::novo(caminho) {
            if let Some(a) = self.destino.arquivo(&c)? {
                return Ok((Some(a.id), VinculoAnexo::CaminhoExato));
            }
        }
        let Some(nome) = componentes(caminho).last().map(|n| n.to_string()) else {
            return Ok((None, VinculoAnexo::NaoEncontrado));
        };
        let candidatos = self.destino.arquivos_com_nome(&nome)?;
        Ok(escolher_anexo(caminho, tamanho, &candidatos))
    }

    fn descarregar(&mut self) -> Resultado<()> {
        if !self.lote.is_empty() {
            self.destino.gravar_artefatos(self.execucao, &self.lote)?;
            let n = self.lote.len() as u64;
            self.controle.atualizar(|p| p.artefatos += n);
            self.lote.clear();
            // `emitidos` já conta o lote gravado agora.
            if self.emitidos / ANUNCIO_REGISTROS > (self.emitidos - n) / ANUNCIO_REGISTROS {
                self.log.anunciar(
                    self.controle,
                    NivelEvento::Info,
                    "gravacao",
                    format!(
                        "{}: {} registro(s) gravados e indexados até agora.",
                        self.parser_nome,
                        numero_br(self.emitidos)
                    ),
                    json!({ "execucao": self.execucao, "registros": self.emitidos }),
                );
            }
        }
        Ok(())
    }
}

impl SaidaParser for Coletor<'_> {
    fn emitir(&mut self, mut artefato: NovoArtefato) -> Resultado<()> {
        // O que é gravado é exatamente o que entra no hash (recalculável depois).
        artefato.normalizar();
        let hash_registro = artefato.hash_registro();
        let mut origens = Vec::with_capacity(1 + artefato.origens_adicionais.len());
        let refs = std::iter::once((artefato.origem.clone(), true)).chain(
            artefato
                .origens_adicionais
                .iter()
                .cloned()
                .map(|r| (r, false)),
        );
        for (r, principal) in refs {
            match self.resolver(r, principal) {
                Ok(o) => origens.push(o),
                Err(msg) => {
                    self.avisar(AvisoParser {
                        mensagem: format!("artefato descartado: {msg}"),
                        papel_arquivo: None,
                        id_registro: artefato.origem.id_registro.clone(),
                    });
                    return Ok(());
                }
            }
        }
        let (anexo_arquivo_id, anexo_vinculo) = match artefato.anexo_caminho.as_deref() {
            Some(c) => {
                let (id, v) = self.vincular_anexo(c, artefato.tamanho)?;
                (id, Some(v))
            }
            None => (None, None),
        };
        self.lote.push(ArtefatoParaGravar {
            artefato,
            origens,
            anexo_arquivo_id,
            anexo_vinculo,
            hash_registro,
        });
        self.emitidos += 1;
        if self.lote.len() >= LOTE {
            self.descarregar()?;
        }
        Ok(())
    }

    fn avisar(&mut self, aviso: AvisoParser) {
        self.avisos += 1;
        self.controle.atualizar(|p| p.avisos += 1);
        if self.avisos <= MAX_AVISOS_ANUNCIADOS {
            let onde = aviso
                .id_registro
                .as_deref()
                .map(|r| format!(" (registro {r})"))
                .unwrap_or_default();
            self.log.anunciar(
                self.controle,
                NivelEvento::Aviso,
                "aviso",
                format!("{}{onde}: {}", self.parser_nome, aviso.mensagem),
                json!({ "execucao": self.execucao, "aviso": aviso }),
            );
        } else if self.avisos == MAX_AVISOS_ANUNCIADOS + 1 {
            self.log.anunciar(
                self.controle,
                NivelEvento::Aviso,
                "aviso",
                format!(
                    "{}: demais avisos só na lista de avisos da extração (total no fim da leitura).",
                    self.parser_nome
                ),
                json!({ "execucao": self.execucao }),
            );
        }
        if let Err(e) = self.destino.gravar_aviso(self.execucao, &aviso) {
            self.falha_gravacao.get_or_insert(e);
        }
    }

    fn cancelado(&self) -> bool {
        self.controle.cancelado()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn checkpoint_e_lido_sem_autorizar_sucesso() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = ContextoImportacao {
            pasta_importacao: dir.path().join("importacao"),
            pasta_copias: dir.path().join("copias"),
            fuso_horario: "America/Sao_Paulo".into(),
        };
        std::fs::create_dir_all(&ctx.pasta_importacao).unwrap();
        let limites = LimitesRecursos {
            max_arquivos: 100,
            max_bytes: 10_000,
            workers_hash: 2,
        };
        gravar_checkpoint(&ctx, "FIXTURE E12", "interrompida", 3, 120, limites).unwrap();
        let checkpoint = ler_checkpoint(&ctx.pasta_importacao.join(NOME_CHECKPOINT))
            .unwrap()
            .unwrap();
        assert_eq!(checkpoint.estado, "interrompida");
        assert_eq!(checkpoint.arquivos_inventariados, 3);
        assert_ne!(checkpoint.estado, "concluida");
        assert!(!ctx
            .pasta_importacao
            .join(format!("{NOME_CHECKPOINT}.tmp"))
            .exists());
    }

    #[test]
    fn checkpoint_invalido_nao_e_aceito() {
        let dir = tempfile::tempdir().unwrap();
        let caminho = dir.path().join(NOME_CHECKPOINT);
        std::fs::write(
            &caminho,
            r#"{"schema":1,"estado":"desconhecida","fonte":"fixture","atualizado_em":"agora","arquivos_inventariados":0,"bytes_lidos":0,"workers_hash":1,"limite_arquivos":1,"limite_bytes":1}"#,
        )
        .unwrap();
        assert!(matches!(
            ler_checkpoint(&caminho),
            Err(ErroIngestao::Interno(m)) if m.contains("fora do contrato")
        ));
    }

    #[test]
    fn avaliacao_exige_mesma_fonte_e_conferencia_humana() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = ContextoImportacao {
            pasta_importacao: dir.path().join("importacao"),
            pasta_copias: dir.path().join("copias"),
            fuso_horario: "America/Sao_Paulo".into(),
        };
        std::fs::create_dir_all(&ctx.pasta_importacao).unwrap();
        gravar_checkpoint(
            &ctx,
            "FONTE ORIGINAL",
            "interrompida",
            1,
            10,
            LimitesRecursos {
                max_arquivos: 10,
                max_bytes: 100,
                workers_hash: 1,
            },
        )
        .unwrap();
        let caminho = ctx.pasta_importacao.join(NOME_CHECKPOINT);
        assert!(matches!(
            avaliar_retomada(&caminho, "FONTE ORIGINAL").unwrap(),
            AvaliacaoRetomada::ConferenciaObrigatoria(c) if c.estado == "interrompida"
        ));
        assert!(matches!(
            avaliar_retomada(&caminho, "OUTRA FONTE").unwrap(),
            AvaliacaoRetomada::FonteDivergente { .. }
        ));
        gravar_checkpoint(
            &ctx,
            "FONTE ORIGINAL",
            "concluida",
            1,
            10,
            LimitesRecursos {
                max_arquivos: 10,
                max_bytes: 100,
                workers_hash: 1,
            },
        )
        .unwrap();
        assert!(matches!(
            avaliar_retomada(&caminho, "FONTE ORIGINAL").unwrap(),
            AvaliacaoRetomada::Concluida(c) if c.estado == "concluida"
        ));
    }

    fn cand(id: i64, caminho: &str, tamanho: u64) -> CandidatoAnexo {
        CandidatoAnexo {
            id,
            caminho: caminho.into(),
            tamanho,
        }
    }

    const APP: &str = "Media/WhatsApp Images/IMG-1.jpg";

    #[test]
    fn anexo_pelo_sufixo_do_caminho() {
        let c = [
            cand(1, "sdcard/WhatsApp/Media/WhatsApp Images/IMG-1.jpg", 64),
            cand(2, "outra/IMG-1.jpg", 64),
        ];
        assert_eq!(
            escolher_anexo(APP, Some(64), &c),
            (Some(1), VinculoAnexo::SufixoCaminho)
        );
        // Caminho absoluto no aparelho também serve.
        let abs = "/storage/emulated/0/WhatsApp/Media/WhatsApp Images/IMG-1.jpg";
        assert_eq!(
            escolher_anexo(abs, None, &c),
            (Some(1), VinculoAnexo::SufixoCaminho)
        );
    }

    #[test]
    fn anexo_ambiguo_nao_e_escolhido() {
        let c = [
            cand(1, "a/Media/WhatsApp Images/IMG-1.jpg", 64),
            cand(2, "b/Media/WhatsApp Images/IMG-1.jpg", 64),
        ];
        assert_eq!(
            escolher_anexo(APP, Some(64), &c),
            (None, VinculoAnexo::Ambiguo)
        );
    }

    #[test]
    fn tamanho_diferente_nao_e_vinculado() {
        let c = [cand(1, "x/Media/WhatsApp Images/IMG-1.jpg", 10)];
        assert_eq!(
            escolher_anexo(APP, Some(64), &c),
            (None, VinculoAnexo::TamanhoDiferente)
        );
    }

    #[test]
    fn so_o_nome_exige_tamanho_igual() {
        let c = [cand(7, "copiados/IMG-1.jpg", 64)];
        assert_eq!(
            escolher_anexo(APP, Some(64), &c),
            (Some(7), VinculoAnexo::NomeETamanho)
        );
        assert_eq!(escolher_anexo(APP, None, &c), (None, VinculoAnexo::Ambiguo));
    }

    #[test]
    fn sem_candidatos_nao_encontrado() {
        assert_eq!(
            escolher_anexo(APP, Some(1), &[]),
            (None, VinculoAnexo::NaoEncontrado)
        );
    }
}
