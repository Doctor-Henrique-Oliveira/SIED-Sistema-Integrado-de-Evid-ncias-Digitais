import type { ExtracaoId } from "./tipos/ExtracaoId";

/** Adaptador temporário até o ambiente Rust voltar a gerar contratos ts-rs. */
export type FiltroFilaRevisao = {
  estado?: EstadoFilaRevisao | null;
  motor_versao?: string | null;
};

export type EstadoFilaRevisao = "nao_revisado" | "confirmado" | "descartado" | "inconclusivo";

export type CandidatoRevisao = {
  id: number;
  documento_id: number;
  caso_id: string;
  extracao_id: ExtracaoId;
  arquivo_id: number;
  caminho: string;
  tipo: string;
  literal: string;
  inicio_utf8: number;
  fim_utf8: number;
  estado: EstadoFilaRevisao;
  nota: string | null;
  motor_versao: string;
  sha256_origem: string;
  sha256_texto: string;
  revista_em: string | null;
  revisor: string | null;
};
