//! Banco autocontido de um caso.
//!
//! ```text
//! <pasta do caso>/
//!   caso.sied.db     procedimento, dispositivos, extrações, inventário, artefatos, auditoria
//!   importacoes/<id>/  manifesto.sha256 e log-importacao.jsonl de cada importação
//!   cache/bancos/<id>/ cópias de trabalho verificadas dos bancos lidos pelos parsers
//! ```
//!
//! Cada alteração é gravada na mesma transação que o seu registro de auditoria:
//! ou as duas coisas ficam gravadas, ou nenhuma.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row, TransactionBehavior};
use serde_json::json;
use sied_dominio::caso::FUSO_PADRAO;
use sied_dominio::{
    Caso, CasoId, Dispositivo, DispositivoId, ExtracaoId, NovoCaso, NovoDispositivo,
    SistemaOperacional,
};
use sied_ingestao::InfoComponente;
use std::path::{Path, PathBuf};

use crate::auditoria::{self, RegistroAuditoria, VerificacaoAuditoria};
use crate::busca::{self, FiltroBusca, ResultadoBusca};
use crate::conversas::{self, Conversa};
use crate::extracoes::{
    self, ArquivoExtracao, ArtefatoDetalhe, ArtefatoResumo, ExecucaoParser, Extracao, Pagina,
    TotalCategoria,
};
use crate::importacao::Importacao;
use crate::migracoes::{self, BANCO_CASO};
use crate::{conexao, ErroArmazenamento, Resultado, Sessao};

pub const NOME_ARQUIVO_CASO: &str = "caso.sied.db";

pub struct BancoCaso {
    conn: Connection,
    pasta: PathBuf,
    sessao: Sessao,
    caso: Caso,
}

impl BancoCaso {
    /// Cria um caso numa pasta nova ou vazia.
    pub fn criar(pasta: &Path, novo: NovoCaso, sessao: Sessao) -> Resultado<Self> {
        let novo = novo.normalizado();
        novo.validar()?;
        if pasta.join(NOME_ARQUIVO_CASO).exists() {
            return Err(ErroArmazenamento::CasoJaExiste(pasta.into()));
        }
        if pasta.exists() && std::fs::read_dir(pasta)?.next().is_some() {
            return Err(ErroArmazenamento::PastaNaoVazia(pasta.into()));
        }
        std::fs::create_dir_all(pasta)?;
        let pasta = pasta.canonicalize()?;
        let caminho = pasta.join(NOME_ARQUIVO_CASO);
        let mut conn = conexao::abrir(&caminho)?;
        migracoes::preparar(&mut conn, &caminho, &BANCO_CASO)?;

        let agora = Utc::now();
        let caso = Caso {
            id: CasoId::novo(),
            dados: novo,
            fuso_horario: FUSO_PADRAO.into(),
            criado_em: agora,
            atualizado_em: agora,
        };
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let d = &caso.dados;
        tx.execute(
            "INSERT INTO caso(id, procedimento, numero_procedimento, numero_bo, investigado, vitima, delegado,
                              responsavel_analise, unidade, observacoes, fuso_horario, criado_em, atualizado_em)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
            params![
                caso.id.to_string(),
                d.procedimento,
                d.numero_procedimento,
                d.numero_bo,
                d.investigado,
                d.vitima,
                d.delegado,
                d.responsavel_analise,
                d.unidade,
                d.observacoes,
                caso.fuso_horario,
                agora.to_rfc3339(),
            ],
        )?;
        auditoria::registrar(
            &tx,
            &sessao,
            "caso_criado",
            json!({ "caso_id": caso.id, "dados": caso.dados, "pasta": pasta, "versao_sied": env!("CARGO_PKG_VERSION") }),
        )?;
        tx.commit()?;
        tracing::info!(caso = %caso.id, pasta = %pasta.display(), "caso criado");
        Ok(BancoCaso {
            conn,
            pasta,
            sessao,
            caso,
        })
    }

    /// Abre um caso existente a partir da sua pasta.
    pub fn abrir(pasta: &Path, sessao: Sessao) -> Resultado<Self> {
        let caminho = pasta.join(NOME_ARQUIVO_CASO);
        if !caminho.is_file() {
            return Err(ErroArmazenamento::CasoNaoEncontrado(pasta.into()));
        }
        let pasta = pasta.canonicalize()?;
        let mut conn = conexao::abrir(&caminho)?;
        migracoes::preparar(&mut conn, &caminho, &BANCO_CASO)?;
        let caso = ler_caso(&conn)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        auditoria::registrar(&tx, &sessao, "caso_aberto", json!({ "pasta": pasta }))?;
        marcar_interrompidas(&tx, &sessao)?;
        tx.commit()?;
        tracing::info!(caso = %caso.id, pasta = %pasta.display(), "caso aberto");
        Ok(BancoCaso {
            conn,
            pasta,
            sessao,
            caso,
        })
    }

    /// Registra o fechamento na auditoria e libera o banco.
    pub fn fechar(mut self) -> Resultado<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        auditoria::registrar(&tx, &self.sessao, "caso_fechado", json!({}))?;
        tx.commit()?;
        tracing::info!(caso = %self.caso.id, "caso fechado");
        self.conn.close().map_err(|(_, e)| e)?;
        Ok(())
    }

    pub fn caso(&self) -> &Caso {
        &self.caso
    }

    pub fn pasta(&self) -> &Path {
        &self.pasta
    }

    pub fn cadastrar_dispositivo(&mut self, novo: NovoDispositivo) -> Resultado<Dispositivo> {
        let novo = novo.normalizado();
        novo.validar()?;
        let agora = Utc::now();
        let disp = Dispositivo {
            id: DispositivoId::novo(),
            caso_id: self.caso.id,
            dados: novo,
            criado_em: agora,
            atualizado_em: agora,
        };
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let d = &disp.dados;
        tx.execute(
            "INSERT INTO dispositivo(id, caso_id, apelido, fabricante, modelo, nome_comercial, sistema, versao_sistema,
                                     imei1, imei2, serial, numero, operadora, capacidade_bytes, observacoes,
                                     criado_em, atualizado_em)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
            params![
                disp.id.to_string(),
                disp.caso_id.to_string(),
                d.apelido,
                d.fabricante,
                d.modelo,
                d.nome_comercial,
                d.sistema.como_str(),
                d.versao_sistema,
                d.imei1,
                d.imei2,
                d.serial,
                d.numero,
                d.operadora,
                d.capacidade_bytes.map(|c| c as i64),
                d.observacoes,
                agora.to_rfc3339(),
            ],
        )?;
        auditoria::registrar(
            &tx,
            &self.sessao,
            "dispositivo_cadastrado",
            json!({ "dispositivo_id": disp.id, "dados": disp.dados }),
        )?;
        tx.commit()?;
        tracing::info!(caso = %self.caso.id, dispositivo = %disp.id, "dispositivo cadastrado");
        Ok(disp)
    }

    pub fn listar_dispositivos(&self) -> Resultado<Vec<Dispositivo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, caso_id, apelido, fabricante, modelo, nome_comercial, sistema, versao_sistema,
                    imei1, imei2, serial, numero, operadora, capacidade_bytes, observacoes, criado_em, atualizado_em
             FROM dispositivo ORDER BY criado_em, id",
        )?;
        let linhas = stmt.query_map([], ler_dispositivo)?;
        Ok(linhas.collect::<Result<_, _>>()?)
    }

    pub fn auditoria(&self, limite: i64) -> Resultado<Vec<RegistroAuditoria>> {
        auditoria::listar(&self.conn, limite)
    }

    pub fn verificar_auditoria(&self) -> Resultado<VerificacaoAuditoria> {
        auditoria::verificar(&self.conn)
    }

    /// Registra o início de uma importação e devolve o gravador, que tem conexão
    /// própria e pode ser levado para outra thread.
    pub fn iniciar_importacao(
        &self,
        dispositivo_id: DispositivoId,
        importador: &InfoComponente,
        origem: &Path,
    ) -> Resultado<Importacao> {
        Importacao::iniciar(
            &self.pasta.join(NOME_ARQUIVO_CASO),
            &self.pasta,
            &self.caso.fuso_horario,
            self.sessao.clone(),
            dispositivo_id,
            importador,
            origem,
        )
    }

    pub fn extracoes(&self) -> Resultado<Vec<Extracao>> {
        extracoes::listar(&self.conn)
    }

    pub fn extracao(&self, id: ExtracaoId) -> Resultado<Extracao> {
        extracoes::obter(&self.conn, id)
    }

    pub fn totais_por_categoria(&self, id: ExtracaoId) -> Resultado<Vec<TotalCategoria>> {
        extracoes::totais_por_categoria(&self.conn, id)
    }

    pub fn arquivos(
        &self,
        id: ExtracaoId,
        categoria: Option<&str>,
        texto: Option<&str>,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<ArquivoExtracao>> {
        extracoes::listar_arquivos(&self.conn, id, categoria, texto, deslocamento, limite)
    }

    pub fn execucoes(&self, id: ExtracaoId) -> Resultado<Vec<ExecucaoParser>> {
        extracoes::listar_execucoes(&self.conn, id)
    }

    pub fn artefatos(
        &self,
        id: ExtracaoId,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<ArtefatoResumo>> {
        extracoes::listar_artefatos(&self.conn, id, deslocamento, limite)
    }

    pub fn artefato(&self, artefato_id: i64) -> Resultado<ArtefatoDetalhe> {
        extracoes::detalhe_artefato(&self.conn, artefato_id)
    }

    pub fn conversas(
        &self,
        extracao: Option<ExtracaoId>,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<Conversa>> {
        conversas::listar(&self.conn, extracao, deslocamento, limite)
    }

    pub fn conversa(&self, conversa_id: i64) -> Resultado<Conversa> {
        conversas::obter(&self.conn, conversa_id)
    }

    pub fn mensagens_conversa(
        &self,
        conversa_id: i64,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<ArtefatoResumo>> {
        conversas::mensagens(&self.conn, conversa_id, deslocamento, limite)
    }

    pub fn conversa_do_artefato(&self, artefato_id: i64) -> Resultado<Option<i64>> {
        conversas::conversa_do_artefato(&self.conn, artefato_id)
    }

    pub fn buscar(
        &self,
        filtro: &FiltroBusca,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<ResultadoBusca> {
        busca::buscar(&self.conn, filtro, deslocamento, limite)
    }
}

/// Importações que ainda constam "em andamento" ao abrir o caso foram interrompidas
/// (programa encerrado ou queda de energia). O que já foi gravado permanece.
fn marcar_interrompidas(conn: &Connection, sessao: &Sessao) -> Resultado<()> {
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM extracao WHERE status = 'em_andamento'")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    for id in ids {
        let agora = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE execucao_parser SET status = 'interrompida', concluida_em = ?2
             WHERE extracao_id = ?1 AND status = 'em_andamento'",
            params![id, agora],
        )?;
        conn.execute(
            "UPDATE extracao SET status = 'interrompida', concluida_em = ?2,
                    erro = 'importação interrompida (o SIED foi encerrado durante a importação)'
             WHERE id = ?1",
            params![id, agora],
        )?;
        auditoria::registrar(
            conn,
            sessao,
            "importacao_interrompida",
            json!({ "extracao_id": id }),
        )?;
        tracing::warn!(extracao = %id, "importação encontrada interrompida");
    }
    Ok(())
}

fn data(texto: String) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&texto)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

fn uuid<T: std::str::FromStr>(texto: String) -> rusqlite::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    texto.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn ler_caso(conn: &Connection) -> Resultado<Caso> {
    let mut stmt = conn.prepare(
        "SELECT id, procedimento, numero_procedimento, numero_bo, investigado, vitima, delegado,
                responsavel_analise, unidade, observacoes, fuso_horario, criado_em, atualizado_em
         FROM caso",
    )?;
    let mut casos: Vec<Caso> = stmt
        .query_map([], |r| {
            Ok(Caso {
                id: uuid(r.get(0)?)?,
                dados: NovoCaso {
                    procedimento: r.get(1)?,
                    numero_procedimento: r.get(2)?,
                    numero_bo: r.get(3)?,
                    investigado: r.get(4)?,
                    vitima: r.get(5)?,
                    delegado: r.get(6)?,
                    responsavel_analise: r.get(7)?,
                    unidade: r.get(8)?,
                    observacoes: r.get(9)?,
                },
                fuso_horario: r.get(10)?,
                criado_em: data(r.get(11)?)?,
                atualizado_em: data(r.get(12)?)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    match casos.len() {
        1 => Ok(casos.remove(0)),
        n => Err(ErroArmazenamento::Inconsistente(format!(
            "esperado 1 caso no banco, encontrados {n}"
        ))),
    }
}

fn ler_dispositivo(r: &Row) -> rusqlite::Result<Dispositivo> {
    let sistema: String = r.get(6)?;
    Ok(Dispositivo {
        id: uuid(r.get(0)?)?,
        caso_id: uuid(r.get(1)?)?,
        dados: NovoDispositivo {
            apelido: r.get(2)?,
            fabricante: r.get(3)?,
            modelo: r.get(4)?,
            nome_comercial: r.get(5)?,
            sistema: SistemaOperacional::de_str(&sistema).unwrap_or_default(),
            versao_sistema: r.get(7)?,
            imei1: r.get(8)?,
            imei2: r.get(9)?,
            serial: r.get(10)?,
            numero: r.get(11)?,
            operadora: r.get(12)?,
            capacidade_bytes: r.get::<_, Option<i64>>(13)?.map(|c| c as u64),
            observacoes: r.get(14)?,
        },
        criado_em: data(r.get(15)?)?,
        atualizado_em: data(r.get(16)?)?,
    })
}

/// Sugere um nome de pasta seguro no Windows a partir do número do procedimento/BO.
pub fn nome_pasta_sugerido(novo: &NovoCaso) -> String {
    let base = novo
        .numero_procedimento
        .as_deref()
        .or(novo.numero_bo.as_deref())
        .unwrap_or("caso");
    let mut nome: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    while nome.contains("--") {
        nome = nome.replace("--", "-");
    }
    let nome = nome.trim_matches('-');
    let nome: String = nome.chars().take(60).collect();
    if nome.is_empty() {
        "caso".into()
    } else {
        nome
    }
}
