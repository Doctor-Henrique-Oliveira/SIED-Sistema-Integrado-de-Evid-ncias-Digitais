//! Pipeline de importação, igual para todas as ferramentas:
//!
//! 1. **Contar** arquivos e bytes (para o progresso).
//! 2. **Inventariar**: ler cada arquivo uma vez, calcular SHA-256, classificar pela
//!    assinatura e gravar; ao mesmo tempo, escrever o `manifesto.sha256`.
//! 3. **Planejar**: o importador da ferramenta diz quais arquivos cada parser lê.
//! 4. **Processar**: para cada tarefa, copiar os arquivos para a área de trabalho do
//!    caso conferindo o hash com o inventário, rodar o parser sobre a cópia e gravar
//!    artefatos com a referência de origem completa.
//!
//! A extração é acessada só por [`FonteSomenteLeitura`]. Onde gravar é decidido por
//! [`Destino`] (implementado pelo banco do caso), o que mantém este módulo testável.

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sied_dominio::artefato::{NovoArtefato, ReferenciaRegistro};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use ts_rs::TS;

use crate::fonte::{CaminhoFonte, FonteSomenteLeitura};
use crate::importador::{Importador, PlanoImportacao, TarefaParser};
use crate::inventario::{self, ArquivoInventariado};
use crate::parser::{ArquivoDeTrabalho, AvisoParser, EntradaParser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Registro, Resultado};

/// Arquivos auxiliares do SQLite copiados junto com o banco.
pub const AUXILIARES_SQLITE: &[&str] = &["-wal", "-journal"];
pub const NOME_MANIFESTO: &str = "manifesto.sha256";
pub const NOME_LOG: &str = "log-importacao.jsonl";
const LOTE: usize = 500;
const MAX_OBSERVACOES: usize = 50;

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
}

/// Compartilhado entre a tarefa e quem a acompanha (interface).
#[derive(Default)]
pub struct Controle {
    cancelar: AtomicBool,
    progresso: Mutex<Progresso>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct ArtefatoParaGravar {
    pub artefato: NovoArtefato,
    pub origens: Vec<OrigemResolvida>,
    pub anexo_arquivo_id: Option<i64>,
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
    fs::create_dir_all(&ctx.pasta_importacao)?;
    let mut log = Log::criar(&ctx.pasta_importacao)?;
    log.registrar(
        "info",
        "inicio",
        json!({ "fonte": fonte.descricao(), "importador": importador.info() }),
    );
    let r = executar_etapas(
        fonte, importador, registro, destino, controle, ctx, &mut log,
    );
    match &r {
        Ok(resumo) => log.registrar("info", "fim", json!(resumo)),
        Err(e) => log.registrar("erro", "fim", json!({ "erro": e.to_string() })),
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

    // 1. Contar
    controle.atualizar(|p| p.etapa = Etapa::Contando);
    let (mut n, mut bytes) = (0u64, 0u64);
    for e in fonte.listar().flatten() {
        if !e.diretorio {
            n += 1;
            bytes += e.tamanho;
            if n % 1000 == 0 {
                controle.verificar()?;
                controle.atualizar(|p| (p.arquivos_total, p.bytes_total) = (n, bytes));
            }
        }
    }
    controle.atualizar(|p| (p.arquivos_total, p.bytes_total) = (n, bytes));
    log.registrar("info", "contagem", json!({ "arquivos": n, "bytes": bytes }));

    // 2. Inventariar + manifesto
    controle.atualizar(|p| p.etapa = Etapa::Inventariando);
    inventariar(fonte, destino, controle, ctx, log, &mut resumo)?;

    // 3. Planejar
    controle.atualizar(|p| {
        p.etapa = Etapa::Planejando;
        p.item_atual = None;
    });
    let plano = importador.planejar(fonte)?;
    log.registrar("info", "plano", json!(plano));
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
        p.etapa = Etapa::Processando;
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
        p.etapa = Etapa::Concluida;
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

fn inventariar(
    fonte: &dyn FonteSomenteLeitura,
    destino: &mut dyn Destino,
    controle: &Controle,
    ctx: &ContextoImportacao,
    log: &mut Log,
    resumo: &mut ResumoImportacao,
) -> Resultado<()> {
    let caminho_manifesto = ctx.pasta_importacao.join(NOME_MANIFESTO);
    let mut manifesto = BufWriter::new(File::create(&caminho_manifesto)?);
    let mut lote: Vec<ArquivoInventariado> = Vec::with_capacity(LOTE);

    for item in fonte.listar() {
        controle.verificar()?;
        let entrada = match item {
            Ok(e) => e,
            Err(ErroIngestao::ItemIgnorado { caminho, motivo }) => {
                resumo.ignorados += 1;
                log.registrar(
                    "aviso",
                    "item_ignorado",
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
        controle.atualizar(|p| p.item_atual = Some(entrada.caminho.to_string()));
        let leitura = fonte.abrir(&entrada.caminho).and_then(|mut l| {
            inventario::ler_com_hash(&mut l, None, |b| {
                controle.atualizar(|p| p.bytes_feitos += b);
                !controle.cancelado()
            })
        });
        let arq = match leitura {
            Ok(l) => {
                let k = inventario::classificar(&entrada.caminho, l.bytes, &l.cabecalho);
                manifesto.write_all(
                    inventario::linha_manifesto(&l.sha256, &entrada.caminho).as_bytes(),
                )?;
                resumo.bytes += l.bytes;
                let mut entrada = entrada;
                entrada.tamanho = l.bytes; // tamanho efetivamente lido e coberto pelo hash
                ArquivoInventariado {
                    entrada,
                    sha256: Some(l.sha256),
                    categoria: k.categoria,
                    assinatura: k.assinatura,
                    mime: k.mime,
                    erro: None,
                }
            }
            Err(ErroIngestao::Cancelado) => return Err(ErroIngestao::Cancelado),
            Err(e) => {
                resumo.ilegiveis += 1;
                log.registrar(
                    "erro",
                    "leitura",
                    json!({ "caminho": entrada.caminho, "erro": e.to_string() }),
                );
                observar(
                    resumo,
                    format!("não foi possível ler {}: {e}", entrada.caminho),
                );
                let k = inventario::classificar(&entrada.caminho, 1, &[]);
                ArquivoInventariado {
                    entrada,
                    sha256: None,
                    categoria: k.categoria,
                    assinatura: None,
                    mime: None,
                    erro: Some(e.to_string()),
                }
            }
        };
        resumo.arquivos += 1;
        lote.push(arq);
        controle.atualizar(|p| p.arquivos_feitos += 1);
        if lote.len() >= LOTE {
            destino.gravar_arquivos(&lote)?;
            lote.clear();
        }
    }
    if !lote.is_empty() {
        destino.gravar_arquivos(&lote)?;
    }
    manifesto.flush()?;
    manifesto.get_ref().sync_all()?;
    drop(manifesto);

    let mut f = File::open(&caminho_manifesto)?;
    resumo.hash_manifesto = inventario::ler_com_hash(&mut f, None, |_| true)?.sha256;
    log.registrar(
        "info",
        "manifesto",
        json!({ "arquivo": caminho_manifesto, "sha256": resumo.hash_manifesto }),
    );
    Ok(())
}

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
    let lido = inventario::ler_com_hash(&mut leitor, Some(&mut f), |_| !controle.cancelado())?;
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

/// Prepara as cópias de trabalho de uma tarefa. Devolve os arquivos para o parser e,
/// por papel, (id no inventário, sha256).
fn preparar_entrada(
    fonte: &dyn FonteSomenteLeitura,
    destino: &mut dyn Destino,
    controle: &Controle,
    ctx: &ContextoImportacao,
    tarefa: &TarefaParser,
) -> Resultado<(Vec<ArquivoDeTrabalho>, HashMap<String, (i64, String)>)> {
    let mut arquivos = Vec::new();
    let mut por_papel = HashMap::new();
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
        let copia = copiar_verificado(fonte, controle, &a.caminho, &sha, &pasta)?;
        for sufixo in AUXILIARES_SQLITE {
            let aux = CaminhoFonte::novo(&format!("{}{sufixo}", a.caminho))?;
            if let Some(ArquivoRegistrado {
                sha256: Some(s), ..
            }) = destino.arquivo(&aux)?
            {
                copiar_verificado(fonte, controle, &aux, &s, &pasta)?;
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
    Ok((arquivos, por_papel))
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
        log.registrar(
            "aviso",
            "parser_ausente",
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

    let (arquivos, por_papel) = match preparar_entrada(fonte, destino, controle, ctx, tarefa) {
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
            log.registrar(
                "erro",
                "copia_de_trabalho",
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
    log.registrar(
        "info",
        "parser_inicio",
        json!({ "parser": info, "execucao": exec, "arquivos": arquivos }),
    );

    let entrada = EntradaParser {
        arquivos,
        fuso_horario: ctx.fuso_horario.clone(),
    };
    let mut coletor = Coletor {
        destino,
        controle,
        execucao: exec,
        por_papel: &por_papel,
        lote: Vec::with_capacity(LOTE),
        falha_gravacao: None,
        emitidos: 0,
        avisos: 0,
    };
    let r = parser.processar(&entrada, &mut coletor);
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
            log.registrar(
                "info",
                "parser_fim",
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
            log.registrar(
                "erro",
                "parser_fim",
                json!({ "execucao": exec, "erro": e.to_string() }),
            );
            observar(resumo, format!("{}: {e}", info.nome));
            Ok(())
        }
    }
}

/// Recebe artefatos do parser, completa a origem e grava em lotes.
struct Coletor<'a> {
    destino: &'a mut dyn Destino,
    controle: &'a Controle,
    execucao: i64,
    por_papel: &'a HashMap<String, (i64, String)>,
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

    fn descarregar(&mut self) -> Resultado<()> {
        if !self.lote.is_empty() {
            self.destino.gravar_artefatos(self.execucao, &self.lote)?;
            let n = self.lote.len() as u64;
            self.controle.atualizar(|p| p.artefatos += n);
            self.lote.clear();
        }
        Ok(())
    }
}

impl SaidaParser for Coletor<'_> {
    fn emitir(&mut self, artefato: NovoArtefato) -> Resultado<()> {
        let json =
            serde_json::to_vec(&artefato).map_err(|e| ErroIngestao::Gravacao(e.to_string()))?;
        let hash_registro = hex::encode(Sha256::digest(&json));
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
        let anexo_arquivo_id = match artefato.anexo_caminho.as_deref().map(CaminhoFonte::novo) {
            Some(Ok(c)) => self.destino.arquivo(&c)?.map(|a| a.id),
            _ => None,
        };
        self.lote.push(ArtefatoParaGravar {
            artefato,
            origens,
            anexo_arquivo_id,
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
        if let Err(e) = self.destino.gravar_aviso(self.execucao, &aviso) {
            self.falha_gravacao.get_or_insert(e);
        }
    }

    fn cancelado(&self) -> bool {
        self.controle.cancelado()
    }
}
