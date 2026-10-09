// Cliente tipado dos comandos do backend. Os tipos em ./tipos são gerados a partir
// do Rust (ts-rs, `cargo test`); não edite aqueles arquivos à mão.

import { invoke } from "@tauri-apps/api/core";
import type { Caso } from "./tipos/Caso";
import type { CasoAberto } from "./tipos/CasoAberto";
import type { CasoCatalogado } from "./tipos/CasoCatalogado";
import type { CasoId } from "./tipos/CasoId";
import type { Dispositivo } from "./tipos/Dispositivo";
import type { ErroApp } from "./tipos/ErroApp";
import type { InfoApp } from "./tipos/InfoApp";
import type { NovoCaso } from "./tipos/NovoCaso";
import type { NovoDispositivo } from "./tipos/NovoDispositivo";
import type { RegistroAuditoria } from "./tipos/RegistroAuditoria";
import type { VerificacaoAuditoria } from "./tipos/VerificacaoAuditoria";
import type { ArquivoExtracao } from "./tipos/ArquivoExtracao";
import type { ArtefatoDetalhe } from "./tipos/ArtefatoDetalhe";
import type { ArtefatoResumo } from "./tipos/ArtefatoResumo";
import type { Deteccao } from "./tipos/Deteccao";
import type { DispositivoId } from "./tipos/DispositivoId";
import type { EstadoImportacao } from "./tipos/EstadoImportacao";
import type { ExecucaoParser } from "./tipos/ExecucaoParser";
import type { Extracao } from "./tipos/Extracao";
import type { ExtracaoId } from "./tipos/ExtracaoId";
import type { Pagina } from "./tipos/Pagina";
import type { TotalCategoria } from "./tipos/TotalCategoria";
import type { Conversa } from "./tipos/Conversa";
import type { FiltroBusca } from "./tipos/FiltroBusca";
import type { ResultadoBusca } from "./tipos/ResultadoBusca";
import type { AchadoArtefato } from "./tipos/AchadoArtefato";
import type { PedacoTrecho } from "./tipos/PedacoTrecho";
import type { AchadoArquivo } from "./tipos/AchadoArquivo";
import type { Identificador } from "./tipos/Identificador";
import type { ResumoCaso } from "./tipos/ResumoCaso";
import type { TotalTipo } from "./tipos/TotalTipo";
import type { AppConversas } from "./tipos/AppConversas";
import type { EstadoMiniatura } from "./tipos/EstadoMiniatura";
import type { FiltroConversas } from "./tipos/FiltroConversas";
import type { Miniatura } from "./tipos/Miniatura";
import type { PosicaoMensagem } from "./tipos/PosicaoMensagem";
import type { Evidencia } from "./tipos/Evidencia";
import type { Marcacoes } from "./tipos/Marcacoes";
import type { NovaEvidencia } from "./tipos/NovaEvidencia";
import type { PedidoRelatorio } from "./tipos/PedidoRelatorio";
import type { RelatorioRegistrado } from "./tipos/RelatorioRegistrado";
import type { ConferenciaPastaRelatorios } from "./tipos/ConferenciaPastaRelatorios";
import type { ConferenciaDerivados } from "./tipos/ConferenciaDerivados";
import type { DiferencaDerivado } from "./tipos/DiferencaDerivado";
import type { ReconstrucaoDerivados } from "./tipos/ReconstrucaoDerivados";
import type { ArquivoAvulso } from "./tipos/ArquivoAvulso";
import type { SecoesRelatorio } from "./tipos/SecoesRelatorio";
import type { TextosEvidencia } from "./tipos/TextosEvidencia";
import type { TipoEvidencia } from "./tipos/TipoEvidencia";
import type { Chamada } from "./tipos/Chamada";
import type { ContatoAgenda } from "./tipos/ContatoAgenda";
import type { ContatoVinculado } from "./tipos/ContatoVinculado";
import type { FiltroChamadas } from "./tipos/FiltroChamadas";
import type { FiltroContatos } from "./tipos/FiltroContatos";
import type { EventoImportacao } from "./tipos/EventoImportacao";
import type { FiltroLinhaTempo } from "./tipos/FiltroLinhaTempo";
import type { NivelEvento } from "./tipos/NivelEvento";
import type { ResumoLinhaTempo } from "./tipos/ResumoLinhaTempo";
import type { ResumoChamadas } from "./tipos/ResumoChamadas";
import type { TelefoneContato } from "./tipos/TelefoneContato";
import type { TotalApp } from "./tipos/TotalApp";
import type { EstadoDocumento } from "./tipos/EstadoDocumento";
import type { TextoDocumento } from "./tipos/TextoDocumento";
import type { ResultadoDocumento } from "./tipos/ResultadoDocumento";
import type { DocumentoIndexado } from "./tipos/DocumentoIndexado";
import type { AchadoDocumento } from "./tipos/AchadoDocumento";
import type { EntidadeDocumento } from "./tipos/EntidadeDocumento";
import type { EstadoRevisaoEntidade } from "./tipos/EstadoRevisaoEntidade";
import type { RevisaoEntidade } from "./tipos/RevisaoEntidade";
import type { PedidoDescoberta } from "./tipos/PedidoDescoberta";
import type { ResultadoDescoberta } from "./tipos/ResultadoDescoberta";
import type { AchadoDescoberta } from "./tipos/AchadoDescoberta";
import type { VocabularioPesquisa } from "./tipos/VocabularioPesquisa";
import type { RamoVocabulario } from "./tipos/RamoVocabulario";
import type { VersaoVocabularioPesquisa } from "./tipos/VersaoVocabularioPesquisa";
import type { ConsultaDescobertaSalva } from "./tipos/ConsultaDescobertaSalva";
import type { ExecucaoDescoberta } from "./tipos/ExecucaoDescoberta";
import type { MecanismoUniversal } from "./tipos/MecanismoUniversal";
import type { PedidoUniversal } from "./tipos/PedidoUniversal";
import type { SinalUniversal } from "./tipos/SinalUniversal";
import type { ItemUniversal } from "./tipos/ItemUniversal";
import type { ResultadoUniversal } from "./tipos/ResultadoUniversal";
import type { ContextoRelacoes } from "./tipos/ContextoRelacoes";
import type { NoRelacao } from "./tipos/NoRelacao";
import type { ArestaRelacao } from "./tipos/ArestaRelacao";
import type { VisualIndexado } from "./tipos/VisualIndexado";
import type { SimilarVisual } from "./tipos/SimilarVisual";
import type { ResultadoSimilarVisual } from "./tipos/ResultadoSimilarVisual";
import type { DatasetSupervisionado } from "./tipos/DatasetSupervisionado";
import type { PedidoAmostraSupervisionada } from "./tipos/PedidoAmostraSupervisionada";
import type { AmostraSupervisionada } from "./tipos/AmostraSupervisionada";
import type { AvaliacaoSupervisionada } from "./tipos/AvaliacaoSupervisionada";
import type { PaginaTranscricao, SegmentoTranscricao, TranscricaoLocal } from "./transcricao";
import type { ResumoIndexacaoEmbeddings, ResultadoBuscaHibrida } from "./embeddings";
import type { ResultadoVisaoLocal } from "./visual";
import type { ItemMultimodal, MecanismoMultimodal, PedidoMultimodal, ResultadoMultimodal } from "./multimodal";
import type { CandidatoRevisao, FiltroFilaRevisao } from "./revisao";
import type { NovoProvedorEstacao, ProvedorEstacao } from "./provedores";

export type {
  MecanismoUniversal,
  PedidoUniversal,
  SinalUniversal,
  ItemUniversal,
  ResultadoUniversal,
  ContextoRelacoes,
  NoRelacao,
  ArestaRelacao,
  VisualIndexado,
  SimilarVisual,
  ResultadoSimilarVisual,
  DatasetSupervisionado,
  PedidoAmostraSupervisionada,
  AmostraSupervisionada,
  AvaliacaoSupervisionada,
  TranscricaoLocal,
  SegmentoTranscricao,
  PaginaTranscricao,
  ResumoIndexacaoEmbeddings,
  ResultadoBuscaHibrida,
  ResultadoVisaoLocal,
  PedidoMultimodal,
  ResultadoMultimodal,
  ItemMultimodal,
  MecanismoMultimodal,
  CandidatoRevisao,
  FiltroFilaRevisao,
  ProvedorEstacao,
  NovoProvedorEstacao,
  PedidoDescoberta,
  ResultadoDescoberta,
  AchadoDescoberta,
  VocabularioPesquisa,
  RamoVocabulario,
  VersaoVocabularioPesquisa,
  ConsultaDescobertaSalva,
  ExecucaoDescoberta,
  DocumentoIndexado,
  AchadoDocumento,
  EntidadeDocumento,
  EstadoRevisaoEntidade,
  RevisaoEntidade,
  EstadoDocumento,
  TextoDocumento,
  ResultadoDocumento,
  Chamada,
  EventoImportacao,
  NivelEvento,
  FiltroLinhaTempo,
  ResumoLinhaTempo,
  ContatoAgenda,
  ContatoVinculado,
  FiltroChamadas,
  FiltroContatos,
  ResumoChamadas,
  TelefoneContato,
  TotalApp,
  Evidencia,
  Marcacoes,
  NovaEvidencia,
  PedidoRelatorio,
  RelatorioRegistrado,
  ConferenciaPastaRelatorios,
  ConferenciaDerivados,
  DiferencaDerivado,
  ReconstrucaoDerivados,
  ArquivoAvulso,
  SecoesRelatorio,
  TextosEvidencia,
  TipoEvidencia,
  AppConversas,
  EstadoMiniatura,
  FiltroConversas,
  Miniatura,
  PosicaoMensagem,
  Identificador,
  ResumoCaso,
  TotalTipo,
  AchadoArquivo,
  AchadoArtefato,
  PedacoTrecho,
  Conversa,
  FiltroBusca,
  ResultadoBusca,
  ArquivoExtracao,
  ArtefatoDetalhe,
  ArtefatoResumo,
  Deteccao,
  DispositivoId,
  EstadoImportacao,
  ExecucaoParser,
  Extracao,
  ExtracaoId,
  Pagina,
  TotalCategoria,
  Caso,
  CasoAberto,
  CasoCatalogado,
  CasoId,
  Dispositivo,
  ErroApp,
  InfoApp,
  NovoCaso,
  NovoDispositivo,
  RegistroAuditoria,
  VerificacaoAuditoria,
};

/** Erro lançado por qualquer chamada ao backend. */
export class FalhaSied extends Error {
  readonly erro: ErroApp;
  constructor(erro: ErroApp) {
    super(erro.mensagem);
    this.name = "FalhaSied";
    this.erro = erro;
  }

  /** Mensagem do campo, quando o backend recusou por validação. */
  doCampo(campo: string): string | undefined {
    return this.erro.campos.find((c) => c.campo === campo)?.mensagem;
  }
}

function eErroApp(x: unknown): x is ErroApp {
  return typeof x === "object" && x !== null && "codigo" in x && "mensagem" in x;
}

/** Converte qualquer falha de `invoke` em `FalhaSied`. */
export function normalizarErro(e: unknown): FalhaSied {
  if (e instanceof FalhaSied) return e;
  if (eErroApp(e)) return new FalhaSied({ ...e, campos: e.campos ?? [] });
  const mensagem = e instanceof Error ? e.message : String(e);
  return new FalhaSied({ codigo: "interno", mensagem, campos: [] });
}

async function chamar<T>(comando: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(comando, args);
  } catch (e) {
    throw normalizarErro(e);
  }
}

export const api = {
  infoApp: () => chamar<InfoApp>("info_app"),
  listarCasos: () => chamar<CasoCatalogado[]>("listar_casos"),
  listarProvedoresEstacao: () => chamar<ProvedorEstacao[]>("listar_provedores_estacao"),
  salvarProvedorEstacao: (dados: NovoProvedorEstacao) =>
    chamar<ProvedorEstacao>("salvar_provedor_estacao", { dados }),
  removerCasoCatalogo: (casoId: CasoId) => chamar<boolean>("remover_caso_catalogo", { casoId }),
  criarCaso: (dados: NovoCaso, pastaBase: string | null) =>
    chamar<CasoAberto>("criar_caso", { dados, pastaBase }),
  abrirCaso: (casoId: CasoId) => chamar<CasoAberto>("abrir_caso", { casoId }),
  abrirPastaCaso: (pasta: string) => chamar<CasoAberto>("abrir_pasta_caso", { pasta }),
  fecharCaso: () => chamar<null>("fechar_caso"),
  casoAtual: () => chamar<CasoAberto | null>("caso_atual"),
  cadastrarDispositivo: (dados: NovoDispositivo) =>
    chamar<Dispositivo>("cadastrar_dispositivo", { dados }),
  listarDispositivos: () => chamar<Dispositivo[]>("listar_dispositivos"),
  listarAuditoria: (limite: number) => chamar<RegistroAuditoria[]>("listar_auditoria", { limite }),
  verificarAuditoria: () => chamar<VerificacaoAuditoria>("verificar_auditoria"),
  /** Pasta num compartilhamento de rede, inclusive unidade mapeada (Z:), que o caminho não revela. */
  pastaEmRede: (caminho: string) => chamar<boolean>("pasta_em_rede", { caminho }),
  /** Confere conversas e índices de busca contra os registros importados. Só lê. */
  conferirDerivados: () => chamar<ConferenciaDerivados>("conferir_derivados"),
  /** Recalcula conversas e índices de busca a partir dos registros importados. */
  reconstruirDerivados: () => chamar<ReconstrucaoDerivados>("reconstruir_derivados"),

  detectarFonte: (pasta: string) => chamar<Deteccao[]>("detectar_fonte", { pasta }),
  iniciarImportacao: (dispositivoId: DispositivoId, pasta: string, importador: string) =>
    chamar<EstadoImportacao>("iniciar_importacao", { dispositivoId, pasta, importador }),
  estadoImportacao: () => chamar<EstadoImportacao | null>("estado_importacao"),
  cancelarImportacao: () => chamar<null>("cancelar_importacao"),
  eventosImportacao: (desde: number) => chamar<EventoImportacao[]>("eventos_importacao", { desde }),
  eventosDaExtracao: (id: ExtracaoId) => chamar<EventoImportacao[]>("eventos_da_extracao", { id }),
  listarExtracoes: () => chamar<Extracao[]>("listar_extracoes"),
  obterExtracao: (extracaoId: ExtracaoId) => chamar<Extracao>("obter_extracao", { extracaoId }),
  totaisPorCategoria: (extracaoId: ExtracaoId) =>
    chamar<TotalCategoria[]>("totais_por_categoria", { extracaoId }),
  listarArquivos: (
    extracaoId: ExtracaoId,
    filtro: { categoria: string | null; texto: string | null },
    deslocamento: number,
    limite: number,
  ) => chamar<Pagina<ArquivoExtracao>>("listar_arquivos", { extracaoId, ...filtro, deslocamento, limite }),
  listarExecucoes: (extracaoId: ExtracaoId) => chamar<ExecucaoParser[]>("listar_execucoes", { extracaoId }),
  listarArtefatos: (extracaoId: ExtracaoId, deslocamento: number, limite: number) =>
    chamar<Pagina<ArtefatoResumo>>("listar_artefatos", { extracaoId, deslocamento, limite }),
  detalheArtefato: (artefatoId: number) => chamar<ArtefatoDetalhe>("detalhe_artefato", { artefatoId }),

  /** Filtros aplicados no banco, sobre o caso inteiro (não só sobre a página). */
  listarConversas: (filtro: FiltroConversas, deslocamento: number, limite: number) =>
    chamar<Pagina<Conversa>>("listar_conversas", {
      extracaoId: filtro.extracao_id ?? null,
      texto: filtro.texto ?? null,
      app: filtro.app ?? null,
      tipo: filtro.tipo ?? null,
      comAnexo: filtro.com_anexo ?? null,
      deslocamento,
      limite,
    }),
  appsDasConversas: async (extracaoId: ExtracaoId | null) =>
    (await chamar<AppConversas[] | undefined>("apps_das_conversas", { extracaoId })) ?? [],
  posicaoMensagem: async (conversaId: number, artefatoId: number) =>
    (await chamar<number | null | undefined>("posicao_mensagem", { conversaId, artefatoId })) ?? null,
  localizarNaConversa: (conversaId: number, termo: string, limite: number) =>
    chamar<Pagina<PosicaoMensagem>>("localizar_na_conversa", { conversaId, termo, limite }),
  /** Miniatura de uma imagem do inventário (lida da extração e conferida pelo SHA-256). */
  miniatura: async (arquivoId: number) => (await chamar<Miniatura | undefined>("miniatura", { arquivoId })) ?? null,
  /** Texto conferido contra o inventário; PDF admite página real. Não recebe caminho ou hash do frontend. */
    lerDocumento: (arquivoId: number, casoId: CasoId, pagina?: number, modo?: "automatico" | "ocr") =>
      chamar<ResultadoDocumento>("ler_documento", { arquivoId, casoId, ...(pagina === undefined ? {} : { pagina }), ...(modo === undefined ? {} : { modo }) }),
  /** Relê/confronta a origem no backend antes de armazenar o derivado. Nunca recebe texto/hash do frontend. */
    indexarDocumento: (arquivoId: number, casoId: CasoId, pagina?: number, modo?: "automatico" | "ocr") =>
      chamar<DocumentoIndexado>("indexar_documento", { arquivoId, casoId, ...(pagina === undefined ? {} : { pagina }), ...(modo === undefined ? {} : { modo }) }),
  /** Busca somente em textos derivados armazenados; não relê nem confere a origem atual. */
  buscarDocumentos: (casoId: CasoId, termo: string, deslocamento: number, limite: number) =>
    chamar<Pagina<AchadoDocumento>>("buscar_documentos", { casoId, termo, deslocamento, limite }),
  obterDocumento: (casoId: CasoId, documentoId: number) =>
    chamar<DocumentoIndexado>("obter_documento", { casoId, documentoId }),
  /** Resposta validada na tela até os contratos ts-rs poderem ser regenerados pelo Rust. */
  regioesDocumento: (casoId: CasoId, documentoId: number, deslocamento: number, limite: number) =>
    chamar<unknown>("regioes_documento", { casoId, documentoId, deslocamento, limite }),
  localizarRegiaoDocumento: (casoId: CasoId, documentoId: number, inicioUtf8: number, fimUtf8: number) =>
    chamar<number | null>("localizar_regiao_documento", { casoId, documentoId, inicioUtf8, fimUtf8 }),
  entidadesDocumento: (casoId: CasoId, documentoId: number) =>
    chamar<EntidadeDocumento[]>("entidades_documento", { casoId, documentoId }),
  /** Confirmação é da ocorrência literal, não da identidade ou veracidade do conteúdo. */
  revisarEntidade: (casoId: CasoId, entidadeId: number, estado: EstadoRevisaoEntidade, nota: string | null) =>
    chamar<EntidadeDocumento>("revisar_entidade", { casoId, entidadeId, estado, nota }),
  revisoesEntidade: (casoId: CasoId, entidadeId: number, deslocamento = 0, limite = 20) =>
    chamar<Pagina<RevisaoEntidade>>("revisoes_entidade", { casoId, entidadeId, deslocamento, limite }),
  /** Sinal cooperativo; true não significa que a operação já terminou ou foi revertida. */
  cancelarDocumento: (casoId: CasoId) => chamar<boolean>("cancelar_documento", { casoId }),
  transcreverLocal: (casoId: CasoId, arquivoId: number) =>
    chamar<TranscricaoLocal>("transcrever_local", { casoId, arquivoId }),
  cancelarTranscricao: (casoId: CasoId) => chamar<boolean>("cancelar_transcricao", { casoId }),
  listarTranscricoes: (casoId: CasoId, arquivoId: number) =>
    chamar<TranscricaoLocal[]>("listar_transcricoes", { casoId, arquivoId }),
  segmentosTranscricao: (casoId: CasoId, transcricaoId: number, deslocamento: number, limite: number, termo?: string) =>
    chamar<PaginaTranscricao>("segmentos_transcricao", { casoId, transcricaoId, deslocamento, limite, termo: termo?.trim() || null }),
  caminhoMidiaTranscricao: (casoId: CasoId, arquivoId: number) =>
    chamar<string>("caminho_midia_transcricao", { casoId, arquivoId }),
  /** Descritor estrutural local com origem conferida; não detecção/classificação de objetos. */
  indexarVisual: (casoId: CasoId, arquivoId: number) =>
    chamar<VisualIndexado>("indexar_visual", { casoId, arquivoId }),
  analisarVisualLocal: (casoId: CasoId, arquivoId: number) =>
    chamar<ResultadoVisaoLocal>("analisar_visual_local", { casoId, arquivoId }),
  semelhantesVisuais: (casoId: CasoId, operacaoId: string, visualId: number, maxBits: number, limite: number) =>
    chamar<ResultadoSimilarVisual>("semelhantes_visuais", { casoId, operacaoId, visualId, maxBits, limite }),
  obterArquivo: (casoId: CasoId, extracaoId: ExtracaoId, arquivoId: number) =>
    chamar<ArquivoExtracao>("obter_arquivo", { casoId, extracaoId, arquivoId }),
  /** Avaliação local autorizada; nenhum treinamento/exportação automática. */
  criarDataset: (casoId: CasoId, nome: string, autorizacao: string, retencao: string) =>
    chamar<DatasetSupervisionado>("criar_dataset", { casoId, nome, autorizacao, retencao }),
  listarDatasets: (casoId: CasoId, deslocamento = 0, limite = 20) =>
    chamar<Pagina<DatasetSupervisionado>>("listar_datasets", { casoId, deslocamento, limite }),
  adicionarAmostra: (casoId: CasoId, datasetId: number, pedido: PedidoAmostraSupervisionada) =>
    chamar<AmostraSupervisionada>("adicionar_amostra", { casoId, datasetId, pedido }),
  amostrasDataset: (casoId: CasoId, datasetId: number, deslocamento = 0, limite = 20) =>
    chamar<Pagina<AmostraSupervisionada>>("amostras_dataset", { casoId, datasetId, deslocamento, limite }),
  avaliarDataset: (casoId: CasoId, datasetId: number, operacaoId: string) =>
    chamar<AvaliacaoSupervisionada>("avaliar_dataset", { casoId, datasetId, operacaoId }),
  filaRevisao: (casoId: CasoId, filtro: FiltroFilaRevisao, deslocamento = 0, limite = 50) =>
    chamar<Pagina<CandidatoRevisao>>("fila_revisao", { casoId, filtro, deslocamento, limite }),
  obterConversa: (conversaId: number) => chamar<Conversa>("obter_conversa", { conversaId }),
  mensagensConversa: (conversaId: number, deslocamento: number, limite: number) =>
    chamar<Pagina<ArtefatoResumo>>("mensagens_conversa", { conversaId, deslocamento, limite }),
  conversaDoArtefato: (artefatoId: number) => chamar<number | null>("conversa_do_artefato", { artefatoId }),
  buscar: (filtro: FiltroBusca, deslocamento: number, limite: number) =>
    chamar<ResultadoBusca>("buscar", { filtro, deslocamento, limite }),
  /** Pesquisa local explícita: guarda configuração e audita a execução no caso. */
  executarDescoberta: (pedido: PedidoDescoberta, vocabularioId: number | null, operacaoId: string, casoId: CasoId) =>
    chamar<ExecucaoDescoberta>("executar_descoberta", { pedido, vocabularioId, operacaoId, casoId }),
  paginarDescoberta: (consultaId: number, deslocamento: number, limite: number, operacaoId: string, casoId: CasoId) =>
    chamar<ResultadoDescoberta>("paginar_descoberta", { consultaId, deslocamento, limite, operacaoId, casoId }),
  /** Sinal aceito não significa operação encerrada: aguarde a resposta original. */
  cancelarDescoberta: (operacaoId: string) => chamar<boolean>("cancelar_descoberta", { operacaoId }),
  pesquisarUniversal: (casoId: CasoId, operacaoId: string, pedido: PedidoUniversal, deslocamento: number, limite: number) =>
    chamar<ResultadoUniversal>("pesquisar_universal", { casoId, operacaoId, pedido, deslocamento, limite }),
  pesquisarMultimodal: (casoId: CasoId, operacaoId: string, pedido: PedidoMultimodal, deslocamento: number, limite: number) =>
    chamar<ResultadoMultimodal>("pesquisar_multimodal", { casoId, operacaoId, pedido, deslocamento, limite }),
  indexarEmbeddings: (casoId: CasoId, operacaoId: string) =>
    chamar<ResumoIndexacaoEmbeddings>("indexar_embeddings", { casoId, operacaoId }),
  buscarHibrida: (casoId: CasoId, operacaoId: string, texto: string, deslocamento: number, limite: number, pesoSemantico: number) =>
    chamar<ResultadoBuscaHibrida>("buscar_hibrida", { casoId, operacaoId, texto, deslocamento, limite, pesoSemantico }),
  contextoRelacoes: (casoId: CasoId, operacaoId: string, artefatoId: number, janelaMinutos: number, limite: number) =>
    chamar<ContextoRelacoes>("contexto_relacoes", { casoId, operacaoId, artefatoId, janelaMinutos, limite }),
  listarConsultasDescoberta: (casoId: CasoId) => chamar<ConsultaDescobertaSalva[]>("listar_consultas_descoberta", { casoId }),
  listarVocabulariosPesquisa: (casoId: CasoId) => chamar<VersaoVocabularioPesquisa[]>("listar_vocabularios_pesquisa", { casoId }),
  salvarVocabularioPesquisa: (casoId: CasoId, dados: VocabularioPesquisa) =>
    chamar<VersaoVocabularioPesquisa>("salvar_vocabulario_pesquisa", { casoId, dados }),
  /** JSON versionado, preparado pelo backend; não abre rede nem grava na extração. */
  exportarVocabularioPesquisa: (casoId: CasoId, id: number) => chamar<string>("exportar_vocabulario_pesquisa", { casoId, id }),

  resumoCaso: () => chamar<ResumoCaso>("resumo_caso"),
  listarIdentificadores: (texto: string | null, deslocamento: number, limite: number) =>
    chamar<Pagina<Identificador>>("listar_identificadores", { texto, deslocamento, limite }),
  listarContatos: (filtro: FiltroContatos, deslocamento: number, limite: number) =>
    chamar<Pagina<ContatoAgenda>>("listar_contatos", { filtro, deslocamento, limite }),
  appsDosContatos: () => chamar<TotalApp[]>("apps_dos_contatos"),
  listarChamadas: (filtro: FiltroChamadas, deslocamento: number, limite: number) =>
    chamar<Pagina<Chamada>>("listar_chamadas", { filtro, deslocamento, limite }),
  resumoChamadas: () => chamar<ResumoChamadas>("resumo_chamadas"),
  linhaDoTempo: (filtro: FiltroLinhaTempo, deslocamento: number, limite: number) =>
    chamar<Pagina<AchadoArtefato>>("linha_do_tempo", { filtro, deslocamento, limite }),
  resumoLinhaDoTempo: (filtro: FiltroLinhaTempo) => chamar<ResumoLinhaTempo>("resumo_linha_do_tempo", { filtro }),

  listarEvidencias: () => chamar<Evidencia[]>("listar_evidencias"),
  marcacoesEvidencias: () => chamar<Marcacoes>("marcacoes_evidencias"),
  adicionarEvidencia: (nova: NovaEvidencia) => chamar<Evidencia>("adicionar_evidencia", { nova }),
  alterarEvidencia: (id: number, textos: TextosEvidencia) => chamar<Evidencia>("alterar_evidencia", { id, textos }),
  moverEvidencia: (id: number, numero: number) => chamar<Evidencia[]>("mover_evidencia", { id, numero }),
  removerEvidencia: (id: number, motivo: string | null) => chamar<null>("remover_evidencia", { id, motivo }),
  listarRelatorios: () => chamar<RelatorioRegistrado[]>("listar_relatorios"),
  /** Relê os PDFs registrados (SHA-256) e lista arquivos sem registro na pasta. Só lê. */
  conferirPastaRelatorios: () => chamar<ConferenciaPastaRelatorios>("conferir_pasta_relatorios"),
  /** Confere os hashes, relê os arquivos na extração e grava o PDF na pasta do caso. */
  gerarRelatorio: (pedido: PedidoRelatorio) => chamar<RelatorioRegistrado>("gerar_relatorio", { pedido }),
};
