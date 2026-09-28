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

export type {
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
};
