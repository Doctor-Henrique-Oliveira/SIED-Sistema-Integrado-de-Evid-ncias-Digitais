/** Contrato temporário da E05 até a exportação ts-rs Rust voltar a executar no Windows. */
export type MecanismoMultimodal = "mensagens" | "ocr" | "documentos" | "transcricoes" | "embeddings" | "visao" | "metadados" | "entidades";
export type PedidoMultimodal = { filtro: import("./tipos/FiltroBusca").FiltroBusca; mecanismos: MecanismoMultimodal[] };
export type SinalMultimodal = { mecanismo: MecanismoMultimodal; motivo: string; posicao: number; contribuicao_rrf: number; motor: string; motor_versao: string; hash_derivado: string | null };
export type ItemMultimodal = { chave: string; origem_tipo: string; origem_id: number; extracao_id: string; arquivo_id: number | null; titulo: string; trecho: string; inicio_ms: number | null; fim_ms: number | null; hash_origem: string | null; sinais: SinalMultimodal[]; score_ranking: number };
export type ResultadoMultimodal = { caso_id: string; pedido: PedidoMultimodal; algoritmo: string; executada_em: string; itens: { total: number; itens: ItemMultimodal[] }; recuperados_na_janela: number; janela_limitada: boolean; avisos: string[] };
