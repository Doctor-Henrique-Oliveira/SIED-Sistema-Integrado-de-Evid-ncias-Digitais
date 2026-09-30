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

export type {
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
  obterConversa: (conversaId: number) => chamar<Conversa>("obter_conversa", { conversaId }),
  mensagensConversa: (conversaId: number, deslocamento: number, limite: number) =>
    chamar<Pagina<ArtefatoResumo>>("mensagens_conversa", { conversaId, deslocamento, limite }),
  conversaDoArtefato: (artefatoId: number) => chamar<number | null>("conversa_do_artefato", { artefatoId }),
  buscar: (filtro: FiltroBusca, deslocamento: number, limite: number) =>
    chamar<ResultadoBusca>("buscar", { filtro, deslocamento, limite }),

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
  /** Confere os hashes, relê os arquivos na extração e grava o PDF na pasta do caso. */
  gerarRelatorio: (pedido: PedidoRelatorio) => chamar<RelatorioRegistrado>("gerar_relatorio", { pedido }),
};
