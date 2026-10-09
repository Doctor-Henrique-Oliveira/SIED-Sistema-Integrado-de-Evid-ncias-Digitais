/** Candidatos derivados de um motor visual local opt-in. */
export type RegiaoVisualLocal = {
  rotulo: string;
  confianca: number;
  x: number;
  y: number;
  largura: number;
  altura: number;
  mecanismo: string;
  versao: string;
};

export type ResultadoVisaoLocal = {
  motor_id: string;
  motor_versao: string;
  regioes: RegiaoVisualLocal[];
  observacao: string;
};
