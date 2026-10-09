//! Gravação de uma importação no banco do caso.
//!
//! A importação roda em segundo plano com a sua própria conexão ao `caso.sied.db`
//! (modo WAL: a interface continua lendo enquanto ela grava). Toda escrita que toca a
//! auditoria usa transação `IMMEDIATE`, para que as duas conexões nunca calculem o
//! próximo elo da cadeia ao mesmo tempo.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::json;
use sied_dominio::{DispositivoId, ExtracaoId};
use sied_ingestao::fonte::CaminhoFonte;
use sied_ingestao::inventario::ArquivoInventariado;
use sied_ingestao::parser::{AvisoParser, ResumoParser};
use sied_ingestao::pipeline::{
    ArquivoRegistrado, ArtefatoParaGravar, CandidatoAnexo, ContextoImportacao, Destino,
    EstadoExecucao, ResumoImportacao,
};
use sied_ingestao::{ErroIngestao, InfoComponente};
use std::path::{Path, PathBuf};

use crate::conexao::{self, Abertura};
use crate::extracoes::{self, Extracao};
use crate::migracoes::BANCO_CASO;
use crate::{auditoria, ErroArmazenamento, Resultado, Sessao};

pub const PASTA_IMPORTACOES: &str = "importacoes";
pub const PASTA_COPIAS: &str = "cache/bancos";

/// Uma importação em andamento, com conexão própria ao banco do caso.
pub struct Importacao {
    conn: Connection,
    extracao_id: ExtracaoId,
    sessao: Sessao,
    contexto: ContextoImportacao,
    dispositivo_id: DispositivoId,
}

fn gravacao(e: rusqlite::Error) -> ErroIngestao {
    ErroIngestao::Gravacao(e.to_string())
}

fn texto_json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("JSON serializável")
}

fn enum_str<T: serde::Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        _ => String::new(),
    }
}

/// Recusa importar a própria pasta do caso, ou uma pasta que a contenha: a extração e a
/// área de trabalho do SIED nunca se misturam.
pub fn validar_origem(origem: &Path, pasta_caso: &Path) -> Resultado<PathBuf> {
    let origem = origem
        .canonicalize()
        .map_err(|e| ErroArmazenamento::OrigemInvalida(format!("{}: {e}", origem.display())))?;
    if !origem.is_dir() {
        return Err(ErroArmazenamento::OrigemInvalida(format!(
            "{} não é uma pasta",
            origem.display()
        )));
    }
    if origem.starts_with(pasta_caso) || pasta_caso.starts_with(&origem) {
        return Err(ErroArmazenamento::OrigemInvalida(format!(
            "a pasta da extração ({}) e a pasta do caso ({}) não podem estar uma dentro da outra",
            origem.display(),
            pasta_caso.display()
        )));
    }
    Ok(origem)
}

impl Importacao {
    /// Registra a extração (status `em_andamento`) e a auditoria do início.
    pub(crate) fn iniciar(
        caminho_banco: &Path,
        pasta_caso: &Path,
        fuso_horario: &str,
        sessao: Sessao,
        dispositivo_id: DispositivoId,
        importador: &InfoComponente,
        origem: &Path,
    ) -> Resultado<Self> {
        let origem = validar_origem(origem, pasta_caso)?;
        let mut conn = conexao::abrir(caminho_banco, &BANCO_CASO, Abertura::Existente)?;
        let existe: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM dispositivo WHERE id = ?1)",
            [dispositivo_id.to_string()],
            |r| r.get(0),
        )?;
        if !existe {
            return Err(ErroArmazenamento::NaoEncontrado("dispositivo".into()));
        }
        // A verificação e o registro precisam compartilhar a mesma transação
        // IMMEDIATE; sem isso, duas janelas poderiam passar no SELECT ao mesmo
        // tempo e criar execuções concorrentes.
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Uma execução que ficou aberta após queda/cancelamento do processo não pode
        // ser duplicada silenciosamente. A retomada automática continua desativada;
        // o operador deve recuperar a execução existente ou iniciar uma nova depois
        // que ela tenha um estado final explícito.
        let origem_texto = origem.display().to_string();
        let em_andamento: bool = tx.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM extracao
                WHERE dispositivo_id = ?1 AND origem_caminho = ?2 AND importador = ?3
                  AND status = 'em_andamento'
            )",
            params![dispositivo_id.to_string(), &origem_texto, importador.id],
            |r| r.get(0),
        )?;
        if em_andamento {
            return Err(ErroArmazenamento::Recusado(
                "já existe uma importação desta origem em andamento; recupere ou finalize a execução existente antes de iniciar outra".into(),
            ));
        }
        let id = ExtracaoId::novo();
        let agora = Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO extracao(id, dispositivo_id, importador, importador_nome, importador_versao,
                                  origem_caminho, sied_versao, usuario, maquina, iniciada_em, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'em_andamento')",
            params![
                id.to_string(),
                dispositivo_id.to_string(),
                importador.id,
                importador.nome,
                importador.versao,
                origem_texto,
                env!("CARGO_PKG_VERSION"),
                sessao.usuario,
                sessao.maquina,
                agora
            ],
        )?;
        auditoria::registrar(
            &tx,
            &sessao,
            "importacao_iniciada",
            json!({
                "extracao_id": id,
                "dispositivo_id": dispositivo_id,
                "importador": importador,
                "origem": origem,
            }),
        )?;
        tx.commit()?;
        tracing::info!(extracao = %id, origem = %origem.display(), importador = %importador.id, "importação iniciada");
        let base = id.to_string();
        Ok(Importacao {
            conn,
            extracao_id: id,
            sessao,
            dispositivo_id,
            contexto: ContextoImportacao {
                pasta_importacao: pasta_caso.join(PASTA_IMPORTACOES).join(&base),
                pasta_copias: pasta_caso.join(PASTA_COPIAS).join(&base),
                fuso_horario: fuso_horario.into(),
            },
        })
    }

    pub fn extracao_id(&self) -> ExtracaoId {
        self.extracao_id
    }

    pub fn contexto(&self) -> &ContextoImportacao {
        &self.contexto
    }

    /// Grava o resultado final e a auditoria correspondente.
    pub fn finalizar(
        mut self,
        resultado: &Result<ResumoImportacao, ErroIngestao>,
    ) -> Resultado<Extracao> {
        let (status, acao) = match resultado {
            Ok(_) => ("concluida", "importacao_concluida"),
            Err(ErroIngestao::Cancelado) => ("cancelada", "importacao_cancelada"),
            Err(_) => ("falhou", "importacao_falhou"),
        };
        let erro = resultado.as_ref().err().map(|e| e.to_string());
        let id = self.extracao_id.to_string();
        // Em cancelamento ou falha, os totais refletem o que foi gravado até ali.
        let (arquivos, bytes): (i64, i64) = self.conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(tamanho), 0) FROM arquivo WHERE extracao_id = ?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let (artefatos, avisos): (i64, i64) = self.conn.query_row(
            "SELECT COALESCE(SUM(artefatos), 0), COALESCE(SUM(avisos), 0) FROM execucao_parser WHERE extracao_id = ?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let resumo = resultado.as_ref().ok();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE execucao_parser SET status = 'interrompida', concluida_em = ?2
             WHERE extracao_id = ?1 AND status = 'em_andamento'",
            params![id, Utc::now().to_rfc3339()],
        )?;
        tx.execute(
            "UPDATE extracao SET status = ?2, concluida_em = ?3, total_arquivos = ?4, total_bytes = ?5,
                    arquivos_ilegiveis = ?6, itens_ignorados = ?7, hash_manifesto = ?8,
                    total_artefatos = ?9, total_avisos = ?10, erro = ?11, observacoes = ?12,
                    ferramenta_versao = ?13, tipo_extracao = ?14
             WHERE id = ?1",
            params![
                id,
                status,
                Utc::now().to_rfc3339(),
                arquivos,
                bytes,
                resumo.map(|r| r.ilegiveis as i64),
                resumo.map(|r| r.ignorados as i64),
                resumo.map(|r| r.hash_manifesto.clone()),
                artefatos,
                avisos,
                erro,
                resumo.map(|r| texto_json(&r.observacoes)),
                resumo.and_then(|r| r.ferramenta_versao.clone()),
                resumo.and_then(|r| r.tipo_extracao.clone()),
            ],
        )?;
        auditoria::registrar(
            &tx,
            &self.sessao,
            acao,
            json!({
                "extracao_id": self.extracao_id,
                "dispositivo_id": self.dispositivo_id,
                "arquivos": arquivos,
                "bytes": bytes,
                "hash_manifesto": resumo.map(|r| &r.hash_manifesto),
                "manifesto": self.contexto.pasta_importacao.join(sied_ingestao::pipeline::NOME_MANIFESTO),
                "execucoes": resumo.map(|r| r.execucoes),
                "execucoes_com_falha": resumo.map(|r| r.execucoes_com_falha),
                "artefatos": artefatos,
                "avisos": avisos,
                "erro": erro,
            }),
        )?;
        tx.commit()?;
        tracing::info!(extracao = %id, status, arquivos, artefatos, "importação finalizada");
        let extracao = extracoes::obter(&self.conn, self.extracao_id)?;
        let _ = self.conn.close();
        Ok(extracao)
    }
}

impl Destino for Importacao {
    fn gravar_arquivos(&mut self, lote: &[ArquivoInventariado]) -> Result<(), ErroIngestao> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(gravacao)?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO arquivo(extracao_id, caminho, nome, extensao, tamanho, modificado_em, criado_em,
                                         sha256, assinatura, mime, categoria, erro)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                )
                .map_err(gravacao)?;
            let id = self.extracao_id.to_string();
            for a in lote {
                let e = &a.entrada;
                stmt.execute(params![
                    id,
                    e.caminho.como_str(),
                    e.caminho.nome(),
                    e.caminho.extensao(),
                    e.tamanho as i64,
                    e.modificado_em,
                    e.criado_em,
                    a.sha256,
                    a.assinatura,
                    a.mime,
                    a.categoria.como_str(),
                    a.erro,
                ])
                .map_err(gravacao)?;
            }
        }
        tx.commit().map_err(gravacao)
    }

    fn arquivo(
        &mut self,
        caminho: &CaminhoFonte,
    ) -> Result<Option<ArquivoRegistrado>, ErroIngestao> {
        self.conn
            .prepare_cached(
                "SELECT id, sha256, tamanho FROM arquivo WHERE extracao_id = ?1 AND caminho = ?2",
            )
            .and_then(|mut s| {
                s.query_row(
                    params![self.extracao_id.to_string(), caminho.como_str()],
                    |r| {
                        Ok(ArquivoRegistrado {
                            id: r.get(0)?,
                            sha256: r.get(1)?,
                            tamanho: r.get::<_, i64>(2)? as u64,
                        })
                    },
                )
                .optional()
            })
            .map_err(gravacao)
    }

    fn arquivos_com_nome(&mut self, nome: &str) -> Result<Vec<CandidatoAnexo>, ErroIngestao> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, caminho, tamanho FROM arquivo
                 WHERE extracao_id = ?1 AND nome = ?2 AND sha256 IS NOT NULL ORDER BY id",
            )
            .map_err(gravacao)?;
        let v = stmt
            .query_map(params![self.extracao_id.to_string(), nome], |r| {
                Ok(CandidatoAnexo {
                    id: r.get(0)?,
                    caminho: r.get(1)?,
                    tamanho: r.get::<_, i64>(2)? as u64,
                })
            })
            .map_err(gravacao)?
            .collect::<Result<_, _>>()
            .map_err(gravacao)?;
        Ok(v)
    }

    fn iniciar_execucao(
        &mut self,
        parser: &InfoComponente,
        arquivo_principal: Option<i64>,
        copia_sha256: Option<&str>,
    ) -> Result<i64, ErroIngestao> {
        self.conn
            .execute(
                "INSERT INTO execucao_parser(extracao_id, parser, parser_nome, parser_versao, arquivo_id,
                                             copia_sha256, iniciada_em, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'em_andamento')",
                params![
                    self.extracao_id.to_string(),
                    parser.id,
                    parser.nome,
                    parser.versao,
                    arquivo_principal,
                    copia_sha256,
                    Utc::now().to_rfc3339()
                ],
            )
            .map_err(gravacao)?;
        Ok(self.conn.last_insert_rowid())
    }

    fn gravar_artefatos(
        &mut self,
        execucao: i64,
        lote: &[ArtefatoParaGravar],
    ) -> Result<(), ErroIngestao> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(gravacao)?;
        {
            let mut art = tx
                .prepare_cached(
                    "INSERT INTO artefato(extracao_id, dispositivo_id, execucao_id, tipo, subtipo, app, conta,
                         remetente, destinatarios, de_mim, data_hora_ms, data_hora_original, formato_data,
                         conteudo, id_externo, resposta_a_id_externo, conversa_id_externo, anexo_caminho,
                         anexo_arquivo_id, mime, nome_arquivo, tamanho, latitude, longitude, estado_registro,
                         confianca, metadados, hash_registro, anexo_vinculo)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                             ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)",
                )
                .map_err(gravacao)?;
            let mut orig = tx
                .prepare_cached(
                    "INSERT INTO referencia_origem(artefato_id, principal, papel_arquivo, arquivo_id,
                         arquivo_sha256, tabela, id_registro, consulta, deslocamento, valores_brutos)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )
                .map_err(gravacao)?;
            let extracao = self.extracao_id.to_string();
            let dispositivo = self.dispositivo_id.to_string();
            for g in lote {
                let a = &g.artefato;
                let data = a.data_hora.as_ref();
                art.execute(params![
                    extracao,
                    dispositivo,
                    execucao,
                    enum_str(&a.tipo),
                    a.subtipo,
                    a.app,
                    a.conta,
                    a.remetente,
                    texto_json(&a.destinatarios),
                    a.de_mim,
                    data.and_then(|d| d.utc_ms),
                    data.map(|d| d.original.clone()),
                    data.map(|d| enum_str(&d.formato)),
                    a.conteudo,
                    a.id_externo,
                    a.resposta_a_id_externo,
                    a.conversa_id_externo,
                    a.anexo_caminho,
                    g.anexo_arquivo_id,
                    a.mime,
                    a.nome_arquivo,
                    a.tamanho.map(|t| t as i64),
                    a.latitude,
                    a.longitude,
                    enum_str(&a.estado),
                    enum_str(&a.confianca),
                    texto_json(&a.metadados),
                    g.hash_registro,
                    g.anexo_vinculo.map(|v| v.como_str()),
                ])
                .map_err(gravacao)?;
                let artefato_id = tx.last_insert_rowid();
                for o in &g.origens {
                    let r = &o.referencia;
                    orig.execute(params![
                        artefato_id,
                        o.principal,
                        r.papel_arquivo,
                        o.arquivo_id,
                        o.arquivo_sha256,
                        r.tabela,
                        r.id_registro,
                        r.consulta,
                        r.deslocamento.map(|d| d as i64),
                        texto_json(&r.valores_brutos),
                    ])
                    .map_err(gravacao)?;
                }
            }
        }
        tx.commit().map_err(gravacao)
    }

    fn gravar_aviso(&mut self, execucao: i64, aviso: &AvisoParser) -> Result<(), ErroIngestao> {
        self.conn
            .execute(
                "INSERT INTO aviso_parser(execucao_id, mensagem, papel_arquivo, id_registro) VALUES (?1, ?2, ?3, ?4)",
                params![execucao, aviso.mensagem, aviso.papel_arquivo, aviso.id_registro],
            )
            .map(|_| ())
            .map_err(gravacao)
    }

    fn concluir_execucao(
        &mut self,
        execucao: i64,
        estado: EstadoExecucao,
        resumo: &ResumoParser,
        erro: Option<&str>,
    ) -> Result<(), ErroIngestao> {
        self.conn
            .execute(
                "UPDATE execucao_parser SET status = ?2, concluida_em = ?3, artefatos = ?4, avisos = ?5, erro = ?6
                 WHERE id = ?1",
                params![
                    execucao,
                    estado.como_str(),
                    Utc::now().to_rfc3339(),
                    resumo.artefatos as i64,
                    resumo.avisos as i64,
                    erro
                ],
            )
            .map(|_| ())
            .map_err(gravacao)
    }
}
