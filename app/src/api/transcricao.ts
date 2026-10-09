/** DTOs da E02; não são contratos ts-rs gerados. A resposta vem do backend e
 * é tratada como derivada, sempre vinculada ao arquivo e ao hash de origem. */
export type EstadoTranscricao = "processando" | "pronta" | "sem_suporte" | "cancelada" | "erro";
export type TranscricaoLocal = {
  id: number;
  caso_id: string;
  extracao_id: string;
  arquivo_id: number;
  caminho: string;
  sha256_origem: string;
  estado: EstadoTranscricao;
  idioma: string | null;
  motor_id: string;
  motor_versao: string;
  iniciado_em: string;
  concluido_em: string | null;
  erro: string | null;
  hash_derivado: string;
  total_segmentos: number;
};
export type SegmentoTranscricao = {
  id: number;
  transcricao_id: number;
  ordem: number;
  inicio_ms: number;
  fim_ms: number;
  texto: string;
  confianca: number | null;
  falante: string | null;
  hash_segmento: string;
};
export type PaginaTranscricao = { total: number; itens: SegmentoTranscricao[] };
