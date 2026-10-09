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
use sied_ingestao::eventos::{self, EventoImportacao};
use sied_ingestao::pipeline::NOME_LOG;
use sied_ingestao::InfoComponente;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::agenda::{
    self, Chamada, ContatoAgenda, FiltroChamadas, FiltroContatos, ResumoChamadas, TotalApp,
};
use crate::auditoria::{self, RegistroAuditoria, VerificacaoAuditoria};
use crate::busca::AchadoArtefato;
use crate::busca::{self, FiltroBusca, ResultadoBusca};
use crate::conexao::{self, Abertura};
use crate::controle_descoberta::ControleDescoberta;
use crate::conversas::{self, AppConversas, Conversa, FiltroConversas, PosicaoMensagem, Trecho};
use crate::derivados::{self, ConferenciaDerivados, ReconstrucaoDerivados};
use crate::descoberta::{PedidoDescoberta, ResultadoDescoberta};
use crate::documentos::{
    self, AchadoDocumento, DocumentoIndexado, EntidadeDocumento, EstadoRevisaoEntidade,
    NovoDocumento, RegiaoDocumento, ResumoDocumento, RevisaoEntidade,
};
use crate::embeddings::{self, ResultadoBuscaHibrida, ResumoIndexacaoEmbeddings};
use crate::evidencias::{self, Evidencia, Marcacoes, NovaEvidencia, TextosEvidencia};
use crate::extracoes::{
    self, ArquivoComOrigem, ArquivoExtracao, ArtefatoDetalhe, ArtefatoResumo, ExecucaoParser,
    Extracao, Pagina, TotalCategoria,
};
use crate::importacao::{Importacao, PASTA_IMPORTACOES};
use crate::linha_tempo::{self, FiltroLinhaTempo, ResumoLinhaTempo};
use crate::migracoes::{self, BANCO_CASO};
use crate::multimodal::{PedidoMultimodal, ResultadoMultimodal};
use crate::pesquisa_local::{
    self, ConsultaDescobertaSalva, ExecucaoDescoberta, VersaoVocabularioPesquisa,
    VocabularioPesquisa,
};
use crate::relacoes::{self, ContextoRelacoes};
use crate::relatorios::{self, NovoRelatorio, RelatorioRegistrado};
use crate::resumo::{self, Identificador, ResumoCaso};
use crate::supervisao::{
    self, AmostraSupervisionada, AvaliacaoSupervisionada, CandidatoRevisao, DatasetSupervisionado,
    FiltroFilaRevisao, PedidoAmostraSupervisionada,
};
use crate::transcricoes::{self, NovaTranscricao, SegmentoTranscricao, TranscricaoLocal};
use crate::universal::{self, PedidoUniversal, ResultadoUniversal};
use crate::visuais::{self, ResultadoSimilarVisual, VisualIndexado};
use crate::{ErroArmazenamento, Resultado, Sessao};

pub const NOME_ARQUIVO_CASO: &str = "caso.sied.db";

/// Enquanto o caso está aberto, o SIED segura uma trava exclusiva do sistema
/// operacional neste arquivo: outra janela (ou outra estação, na mesma pasta de rede)
/// não abre o mesmo caso ao mesmo tempo. A trava é liberada pelo sistema mesmo se o
/// SIED fechar de forma abrupta; o arquivo vazio pode ficar na pasta, sem efeito.
pub const NOME_ARQUIVO_TRAVA: &str = "caso.sied.lock";

/// Trava exclusiva do caso. `None` quando o sistema de arquivos não oferece travas
/// (registrado no log; o caso abre assim mesmo, como antes).
fn travar(pasta: &Path) -> Resultado<Option<std::fs::File>> {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(pasta.join(NOME_ARQUIVO_TRAVA))?;
    match f.try_lock() {
        Ok(()) => Ok(Some(f)),
        Err(std::fs::TryLockError::WouldBlock) => Err(ErroArmazenamento::CasoEmUso(pasta.into())),
        Err(std::fs::TryLockError::Error(e)) => {
            tracing::warn!(erro = %e, pasta = %pasta.display(), "sistema de arquivos sem trava; o caso abre sem proteção contra uso simultâneo");
            Ok(None)
        }
    }
}

pub struct BancoCaso {
    conn: Connection,
    pasta: PathBuf,
    sessao: Sessao,
    caso: Caso,
    /// Mantida aberta enquanto o caso estiver aberto (ver [`NOME_ARQUIVO_TRAVA`]).
    _trava: Option<std::fs::File>,
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
        let trava = travar(&pasta)?;
        let caminho = pasta.join(NOME_ARQUIVO_CASO);
        let mut conn = conexao::abrir(&caminho, &BANCO_CASO, Abertura::Criar)?;
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
            _trava: trava,
        })
    }

    /// Abre um caso existente a partir da sua pasta.
    pub fn abrir(pasta: &Path, sessao: Sessao) -> Resultado<Self> {
        let caminho = pasta.join(NOME_ARQUIVO_CASO);
        if !caminho.is_file() {
            return Err(ErroArmazenamento::CasoNaoEncontrado(pasta.into()));
        }
        let pasta = pasta.canonicalize()?;
        // Antes de tocar no banco: outra janela com o caso aberto pode estar importando,
        // e marcar_interrompidas trataria a importação dela como interrompida.
        let trava = travar(&pasta)?;
        let mut conn = conexao::abrir(&caminho, &BANCO_CASO, Abertura::Existente)?;
        migracoes::preparar(&mut conn, &caminho, &BANCO_CASO)
            .map_err(|e| conexao::corrompido_em(e, &caminho))?;
        let caso = ler_caso(&conn).map_err(|e| conexao::corrompido_em(e, &caminho))?;
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
            _trava: trava,
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

    /// Confere conversas, participantes e índices de busca contra os registros de
    /// origem (ver [`derivados`]). Só lê.
    pub fn conferir_derivados(&self) -> Resultado<ConferenciaDerivados> {
        derivados::conferir(&self.conn)
    }

    /// Recalcula os dados derivados a partir dos registros de origem (com auditoria).
    pub fn reconstruir_derivados(&mut self) -> Resultado<ReconstrucaoDerivados> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let r = derivados::reconstruir(&tx, &self.sessao)?;
        tx.commit()?;
        tracing::info!(caso = %self.caso.id, antes = ?r.antes.tabelas, "dados derivados reconstruídos");
        Ok(r)
    }

    // ---------- Documentos derivados (AI-1), separados dos fatos ----------

    pub fn guardar_documento(&mut self, novo: &NovoDocumento) -> Resultado<DocumentoIndexado> {
        if novo.caso_id != self.caso.id {
            return Err(ErroArmazenamento::Recusado(
                "snapshot pertence a outro caso".into(),
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let documento = documentos::guardar(&tx, &self.sessao, novo)?;
        tx.commit()?;
        Ok(documento)
    }

    pub fn buscar_documentos(
        &self,
        termo: &str,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<AchadoDocumento>> {
        documentos::buscar(&self.conn, termo, deslocamento, limite)
    }

    pub fn buscar_documentos_filtrado(
        &self,
        filtro: &FiltroBusca,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<AchadoDocumento>> {
        documentos::buscar_filtrado(&self.conn, filtro, deslocamento, limite)
    }

    pub fn pesquisar_universal(
        &self,
        pedido: &PedidoUniversal,
        deslocamento: i64,
        limite: i64,
        controle: &Arc<ControleDescoberta>,
    ) -> Resultado<ResultadoUniversal> {
        universal::pesquisar(
            &self.conn,
            self.caso.id,
            pedido,
            deslocamento,
            limite,
            controle,
        )
    }

    pub fn pesquisar_multimodal(
        &self,
        pedido: &PedidoMultimodal,
        deslocamento: i64,
        limite: i64,
        controle: &Arc<ControleDescoberta>,
    ) -> Resultado<ResultadoMultimodal> {
        crate::multimodal::pesquisar(
            &self.conn,
            self.caso.id,
            pedido,
            deslocamento,
            limite,
            controle,
        )
    }

    pub fn contexto_relacoes(
        &self,
        artefato_id: i64,
        janela_minutos: u32,
        limite: i64,
        controle: &Arc<ControleDescoberta>,
    ) -> Resultado<ContextoRelacoes> {
        relacoes::correlacionar(
            &self.conn,
            &self.caso.id,
            artefato_id,
            janela_minutos,
            limite,
            controle,
        )
    }

    pub fn obter_documento(&self, id: i64) -> Resultado<DocumentoIndexado> {
        documentos::obter(&self.conn, id)
    }

    pub fn resumos_documentos_relatorio(&self) -> Resultado<Pagina<ResumoDocumento>> {
        documentos::listar_resumos(&self.conn, self.caso.id, 0, 200)
    }

    pub fn regioes_documento(
        &self,
        id: i64,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<RegiaoDocumento>> {
        documentos::regioes(&self.conn, id, deslocamento, limite)
    }

    pub fn localizar_regiao_documento(
        &self,
        id: i64,
        inicio_utf8: u32,
        fim_utf8: u32,
    ) -> Resultado<Option<u32>> {
        documentos::localizar_regiao(&self.conn, id, inicio_utf8, fim_utf8)
    }

    pub fn entidades_documento(&self, id: i64) -> Resultado<Vec<EntidadeDocumento>> {
        documentos::entidades(&self.conn, id)
    }

    pub fn guardar_transcricao(&mut self, novo: &NovaTranscricao) -> Resultado<TranscricaoLocal> {
        if novo.caso_id != self.caso.id {
            return Err(ErroArmazenamento::Recusado(
                "transcrição pertence a outro caso".into(),
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let t = transcricoes::guardar(&tx, &self.sessao, novo)?;
        tx.commit()?;
        Ok(t)
    }

    pub fn listar_transcricoes(&self, arquivo_id: i64) -> Resultado<Vec<TranscricaoLocal>> {
        transcricoes::listar(&self.conn, arquivo_id)
    }

    pub fn transcricoes_caso(&self) -> Resultado<Vec<TranscricaoLocal>> {
        transcricoes::listar_caso(&self.conn, self.caso.id)
    }

    pub fn segmentos_transcricao_relatorio(
        &self,
        transcricao_id: i64,
        limite: i64,
    ) -> Resultado<Vec<SegmentoTranscricao>> {
        transcricoes::segmentos_relatorio(&self.conn, transcricao_id, limite)
    }

    pub fn segmentos_transcricao(
        &self,
        id: i64,
        deslocamento: i64,
        limite: i64,
        termo: Option<&str>,
    ) -> Resultado<Pagina<SegmentoTranscricao>> {
        transcricoes::segmentos(&self.conn, id, deslocamento, limite, termo)
    }

    pub fn revisar_entidade(
        &mut self,
        id: i64,
        estado: EstadoRevisaoEntidade,
        nota: Option<&str>,
    ) -> Resultado<EntidadeDocumento> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let entidade = documentos::revisar(&tx, &self.sessao, id, estado, nota)?;
        tx.commit()?;
        Ok(entidade)
    }

    pub fn revisoes_entidade(
        &self,
        id: i64,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<RevisaoEntidade>> {
        documentos::revisoes(&self.conn, id, deslocamento, limite)
    }

    pub fn fila_revisao(
        &self,
        filtro: &FiltroFilaRevisao,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<CandidatoRevisao>> {
        supervisao::fila_revisao(&self.conn, self.caso.id, filtro, deslocamento, limite)
    }

    pub fn criar_dataset(
        &mut self,
        nome: &str,
        autorizacao: &str,
        retencao: &str,
    ) -> Resultado<DatasetSupervisionado> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let dataset = supervisao::criar_dataset(
            &tx,
            &self.sessao,
            self.caso.id,
            nome,
            autorizacao,
            retencao,
        )?;
        tx.commit()?;
        Ok(dataset)
    }

    pub fn listar_datasets(
        &self,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<DatasetSupervisionado>> {
        supervisao::listar_datasets(&self.conn, self.caso.id, deslocamento, limite)
    }

    pub fn adicionar_amostra(
        &mut self,
        dataset_id: i64,
        pedido: &PedidoAmostraSupervisionada,
    ) -> Resultado<AmostraSupervisionada> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let amostra =
            supervisao::adicionar_amostra(&tx, &self.sessao, self.caso.id, dataset_id, pedido)?;
        tx.commit()?;
        Ok(amostra)
    }

    pub fn amostras_dataset(
        &self,
        dataset_id: i64,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<AmostraSupervisionada>> {
        supervisao::amostras_dataset(&self.conn, self.caso.id, dataset_id, deslocamento, limite)
    }

    pub fn avaliar_dataset(&self, dataset_id: i64) -> Resultado<AvaliacaoSupervisionada> {
        self.avaliar_dataset_com_controle(dataset_id, &ControleDescoberta::novo())
    }

    pub fn avaliar_dataset_com_controle(
        &self,
        dataset_id: i64,
        controle: &Arc<ControleDescoberta>,
    ) -> Resultado<AvaliacaoSupervisionada> {
        supervisao::avaliar_dataset(&self.conn, self.caso.id, dataset_id, controle)
    }

    pub fn visual_existente(
        &self,
        arquivo_id: i64,
        motor: &str,
        versao: &str,
    ) -> Resultado<Option<VisualIndexado>> {
        visuais::existente(&self.conn, arquivo_id, motor, versao)
    }

    pub fn guardar_visual(&mut self, dados: VisualIndexado) -> Resultado<VisualIndexado> {
        if dados.caso_id != self.caso.id {
            return Err(ErroArmazenamento::Recusado(
                "descritor visual pertence a outro caso".into(),
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let d = visuais::guardar(&tx, &self.sessao, dados)?;
        tx.commit()?;
        Ok(d)
    }

    pub fn semelhantes_visuais(
        &self,
        id: i64,
        max_bits: u32,
        limite: usize,
        controle: &Arc<ControleDescoberta>,
    ) -> Resultado<ResultadoSimilarVisual> {
        visuais::semelhantes(&self.conn, id, max_bits, limite, controle)
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

    /// Fecha explicitamente uma execução que ficou aberta após uma queda da
    /// estação. Isto não retoma nem reprocessa a extração: apenas registra o
    /// estado interrompido e permite uma nova decisão do analista.
    pub fn marcar_extracao_interrompida(&mut self, id: ExtracaoId) -> Resultado<Extracao> {
        let id_texto = id.to_string();
        let status: String = self.conn.query_row(
            "SELECT status FROM extracao WHERE id = ?1",
            [&id_texto],
            |r| r.get(0),
        )?;
        if status != "em_andamento" {
            return Err(ErroArmazenamento::Recusado(format!(
                "a extração está em '{}'; somente execuções em andamento podem ser marcadas como interrompidas",
                status
            )));
        }
        let agora = Utc::now().to_rfc3339();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE execucao_parser SET status = 'interrompida', concluida_em = ?2,
             erro = COALESCE(erro, 'execução encerrada manualmente após recuperação da estação')
             WHERE extracao_id = ?1 AND status = 'em_andamento'",
            params![&id_texto, &agora],
        )?;
        tx.execute(
            "UPDATE extracao SET status = 'interrompida', concluida_em = ?2,
             erro = COALESCE(erro, 'importação marcada como interrompida pelo analista após recuperação da estação')
             WHERE id = ?1 AND status = 'em_andamento'",
            params![&id_texto, &agora],
        )?;
        auditoria::registrar(
            &tx,
            &self.sessao,
            "importacao_interrompida_manual",
            json!({ "extracao_id": id, "motivo": "recuperacao_da_estacao" }),
        )?;
        tx.commit()?;
        extracoes::obter(&self.conn, id)
    }

    /// Console de uma importação já feita, relido do `log-importacao.jsonl` da pasta
    /// do caso (vazio se o log não existir, por exemplo numa cópia parcial do caso).
    pub fn eventos_da_importacao(&self, id: ExtracaoId) -> Resultado<Vec<EventoImportacao>> {
        self.extracao(id)?;
        let log = self
            .pasta
            .join(PASTA_IMPORTACOES)
            .join(id.to_string())
            .join(NOME_LOG);
        match eventos::ler_log(&log) {
            Ok(v) => Ok(v),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
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

    pub fn arquivo_com_origem(&self, arquivo_id: i64) -> Resultado<ArquivoComOrigem> {
        extracoes::arquivo_com_origem(&self.conn, arquivo_id)
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

    /// Recalcula o `hash_registro` a partir do conteúdo gravado (ver
    /// [`extracoes::conferir_registro`]).
    pub fn conferir_registro(&self, artefato_id: i64) -> Resultado<extracoes::ConferenciaRegistro> {
        extracoes::conferir_registro(&self.conn, artefato_id)
    }

    /// Registros do caso (ou da extração) cujo conteúdo não confere com o hash gravado.
    pub fn registros_que_nao_conferem(
        &self,
        extracao: Option<ExtracaoId>,
    ) -> Resultado<Vec<extracoes::ConferenciaRegistro>> {
        extracoes::registros_que_nao_conferem(&self.conn, extracao)
    }

    pub fn conversas(
        &self,
        extracao: Option<ExtracaoId>,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<Conversa>> {
        let filtro = FiltroConversas {
            extracao_id: extracao,
            ..Default::default()
        };
        conversas::listar(&self.conn, &filtro, deslocamento, limite)
    }

    pub fn conversas_filtradas(
        &self,
        filtro: &FiltroConversas,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<Conversa>> {
        conversas::listar(&self.conn, filtro, deslocamento, limite)
    }

    pub fn apps_das_conversas(&self, extracao: Option<ExtracaoId>) -> Resultado<Vec<AppConversas>> {
        conversas::apps(&self.conn, extracao)
    }

    pub fn posicao_mensagem(&self, conversa_id: i64, artefato_id: i64) -> Resultado<Option<i64>> {
        conversas::posicao_mensagem(&self.conn, conversa_id, artefato_id)
    }

    pub fn localizar_na_conversa(
        &self,
        conversa_id: i64,
        termo: &str,
        limite: i64,
    ) -> Resultado<Pagina<PosicaoMensagem>> {
        conversas::localizar(&self.conn, conversa_id, termo, limite)
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

    pub fn indexar_embeddings(
        &mut self,
        controle: &Arc<ControleDescoberta>,
    ) -> Resultado<ResumoIndexacaoEmbeddings> {
        embeddings::indexar(&mut self.conn, &self.caso.id, controle, &self.sessao)
    }

    pub fn buscar_hibrida(
        &self,
        texto: &str,
        deslocamento: i64,
        limite: i64,
        peso_semantico: f64,
        controle: &Arc<ControleDescoberta>,
    ) -> Resultado<ResultadoBuscaHibrida> {
        embeddings::buscar(
            &self.conn,
            &self.caso.id,
            texto,
            deslocamento,
            limite,
            peso_semantico,
            controle,
        )
    }

    pub fn listar_vocabularios_pesquisa(&self) -> Resultado<Vec<VersaoVocabularioPesquisa>> {
        pesquisa_local::listar_vocabularios(&self.conn)
    }

    pub fn salvar_vocabulario_pesquisa(
        &mut self,
        dados: VocabularioPesquisa,
    ) -> Resultado<VersaoVocabularioPesquisa> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let versao = pesquisa_local::salvar_vocabulario(&tx, &self.sessao, dados)?;
        tx.commit()?;
        Ok(versao)
    }

    pub fn exportar_vocabulario_pesquisa(&mut self, id: i64) -> Resultado<String> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let texto = pesquisa_local::exportar_vocabulario(&tx, &self.sessao, id)?;
        tx.commit()?;
        Ok(texto)
    }

    pub fn executar_descoberta(
        &mut self,
        pedido: PedidoDescoberta,
        vocabulario_id: Option<i64>,
    ) -> Resultado<ExecucaoDescoberta> {
        self.executar_descoberta_com_controle(pedido, vocabulario_id, &ControleDescoberta::novo())
    }

    pub fn executar_descoberta_com_controle(
        &mut self,
        pedido: PedidoDescoberta,
        vocabulario_id: Option<i64>,
        controle: &Arc<ControleDescoberta>,
    ) -> Resultado<ExecucaoDescoberta> {
        controle.verificar()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let resultado =
            pesquisa_local::executar(&tx, &self.sessao, pedido, vocabulario_id, controle)?;
        tx.commit()?;
        Ok(resultado)
    }

    pub fn paginar_descoberta(
        &self,
        consulta_id: i64,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<ResultadoDescoberta> {
        self.paginar_descoberta_com_controle(
            consulta_id,
            deslocamento,
            limite,
            &ControleDescoberta::novo(),
        )
    }

    pub fn paginar_descoberta_com_controle(
        &self,
        consulta_id: i64,
        deslocamento: i64,
        limite: i64,
        controle: &Arc<ControleDescoberta>,
    ) -> Resultado<ResultadoDescoberta> {
        controle.verificar()?;
        pesquisa_local::paginar(&self.conn, consulta_id, deslocamento, limite, controle)
    }

    pub fn listar_consultas_descoberta(&self) -> Resultado<Vec<ConsultaDescobertaSalva>> {
        pesquisa_local::listar_consultas(&self.conn)
    }

    /// Mensagens de um trecho de conversa (do início ao fim, inclusive).
    pub fn trecho(&self, inicio_id: i64, fim_id: i64) -> Resultado<Trecho> {
        conversas::trecho(&self.conn, inicio_id, fim_id)
    }

    // ------------------------------------------------------------ evidências

    pub fn evidencias(&self) -> Resultado<Vec<Evidencia>> {
        evidencias::listar(&self.conn)
    }

    pub fn marcacoes(&self) -> Resultado<Marcacoes> {
        evidencias::marcacoes(&self.conn)
    }

    pub fn adicionar_evidencia(&mut self, nova: NovaEvidencia) -> Resultado<Evidencia> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let ev = evidencias::adicionar(&tx, &self.sessao, nova)?;
        tx.commit()?;
        tracing::info!(caso = %self.caso.id, evidencia = ev.id, numero = ev.numero, "evidência adicionada");
        Ok(ev)
    }

    pub fn alterar_evidencia(&mut self, id: i64, textos: TextosEvidencia) -> Resultado<Evidencia> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let ev = evidencias::alterar_textos(&tx, &self.sessao, id, textos)?;
        tx.commit()?;
        Ok(ev)
    }

    pub fn mover_evidencia(&mut self, id: i64, numero: i64) -> Resultado<Vec<Evidencia>> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let lista = evidencias::mover(&tx, &self.sessao, id, numero)?;
        tx.commit()?;
        Ok(lista)
    }

    pub fn remover_evidencia(&mut self, id: i64, motivo: Option<String>) -> Resultado<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        evidencias::remover(&tx, &self.sessao, id, motivo)?;
        tx.commit()?;
        tracing::info!(caso = %self.caso.id, evidencia = id, "evidência removida");
        Ok(())
    }

    // ------------------------------------------------------------ relatórios

    pub fn sessao(&self) -> &Sessao {
        &self.sessao
    }

    pub fn relatorios(&self) -> Resultado<Vec<RelatorioRegistrado>> {
        relatorios::listar(&self.conn)
    }

    /// Confere os PDFs registrados e lista arquivos sem registro na pasta de relatórios
    /// (ver [`relatorios::conferir_pasta`]).
    pub fn conferir_pasta_relatorios(&self) -> Resultado<relatorios::ConferenciaPastaRelatorios> {
        relatorios::conferir_pasta(&self.pasta, &self.relatorios()?)
    }

    /// Registra um PDF já gravado em `relatorios/` (com auditoria).
    pub fn registrar_relatorio(&mut self, novo: NovoRelatorio) -> Resultado<RelatorioRegistrado> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let r = relatorios::registrar(&tx, &self.sessao, novo)?;
        tx.commit()?;
        tracing::info!(caso = %self.caso.id, relatorio = r.id, sha256 = %r.sha256, "relatório gerado");
        Ok(r)
    }

    pub fn resumo(&self) -> Resultado<ResumoCaso> {
        resumo::resumo(&self.conn)
    }

    pub fn identificadores(
        &self,
        texto: Option<&str>,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<Identificador>> {
        resumo::identificadores(&self.conn, texto, deslocamento, limite)
    }

    // ------------------------------------------------------------ agenda e chamadas

    pub fn contatos(
        &self,
        filtro: &FiltroContatos,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<ContatoAgenda>> {
        agenda::contatos(&self.conn, filtro, deslocamento, limite)
    }

    pub fn apps_contatos(&self) -> Resultado<Vec<TotalApp>> {
        agenda::apps_contatos(&self.conn)
    }

    pub fn chamadas(
        &self,
        filtro: &FiltroChamadas,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<Chamada>> {
        agenda::chamadas(&self.conn, filtro, deslocamento, limite)
    }

    pub fn resumo_chamadas(&self) -> Resultado<ResumoChamadas> {
        agenda::resumo_chamadas(&self.conn)
    }

    /// Registros com data de todas as extrações, em ordem cronológica.
    pub fn linha_do_tempo(
        &self,
        filtro: &FiltroLinhaTempo,
        deslocamento: i64,
        limite: i64,
    ) -> Resultado<Pagina<AchadoArtefato>> {
        linha_tempo::linha_do_tempo(&self.conn, filtro, deslocamento, limite)
    }

    pub fn resumo_linha_do_tempo(&self, filtro: &FiltroLinhaTempo) -> Resultado<ResumoLinhaTempo> {
        linha_tempo::resumo_linha_do_tempo(&self.conn, filtro)
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
    } else if reservado_no_windows(&nome) {
        format!("caso-{nome}")
    } else {
        nome
    }
}

/// Nomes que o Windows reserva para dispositivos (a pasta não poderia ser criada).
fn reservado_no_windows(nome: &str) -> bool {
    let n = nome.to_uppercase();
    matches!(n.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|p| {
            n.strip_prefix(p).is_some_and(|resto| {
                matches!(
                    resto,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        })
}
