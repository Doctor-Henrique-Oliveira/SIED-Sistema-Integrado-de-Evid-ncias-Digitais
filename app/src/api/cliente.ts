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

export type {
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
};
