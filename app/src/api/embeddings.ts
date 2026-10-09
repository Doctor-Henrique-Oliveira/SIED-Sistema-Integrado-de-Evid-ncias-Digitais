// DTO local provisório para a etapa E03: os contratos equivalentes marcados com
// #[ts(export)] no Rust serão regenerados pelo cargo test quando o linker MSVC
// estiver disponível. Nenhum arquivo em ./tipos é alterado manualmente.
import type { CasoId } from "./tipos/CasoId";
import type { ExtracaoId } from "./tipos/ExtracaoId";

export type ResumoIndexacaoEmbeddings = {
  examinados: number;
  criados: number;
  reutilizados: number;
  ignorados: number;
  motor_id: string;
  motor_versao: string;
};

export type SinalBuscaHibrida = {
  mecanismo: string;
  motivo: string;
  contribuicao: number;
  posicao: number | null;
  motor_id: string;
  motor_versao: string;
};

export type ItemBuscaHibrida = {
  origem_tipo: "artefato" | "documento" | "transcricao";
  origem_id: number;
  extracao_id: ExtracaoId;
  arquivo_id: number | null;
  titulo: string;
  trecho: string;
  score_hibrido: number;
  score_semantico: number | null;
  score_textual: number | null;
  inicio_ms: number | null;
  fim_ms: number | null;
  sinais: SinalBuscaHibrida[];
};

export type ResultadoBuscaHibrida = {
  consulta: string;
  itens: { total: number; itens: ItemBuscaHibrida[] };
  motor_id: string;
  motor_versao: string;
  peso_semantico: number;
  avisos: string[];
};

export type IdCasoEmbeddings = CasoId;
