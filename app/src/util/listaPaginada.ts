// Itens de uma lista longa carregados por página conforme a rolagem. A primeira
// página vem de quem chama (é ela que informa o total); as demais só são pedidas
// quando algum índice visível cai nelas.

import type { Pagina } from "../api/cliente";
import { usePaginasExtras } from "../api/consultas";

export function useListaPaginada<T>({ chave, primeira, tamanho, buscar, indices }: {
  chave: readonly unknown[];
  primeira: Pagina<T> | undefined;
  tamanho: number;
  buscar: (deslocamento: number, limite: number) => Promise<Pagina<T>>;
  /** Índices perto da área visível. */
  indices: number[];
}) {
  const paginas = [...new Set(indices.map((i) => Math.floor(i / tamanho)))].filter((p) => p > 0).sort((a, b) => a - b);
  const extras = usePaginasExtras(chave, paginas, (p) => buscar(p * tamanho, tamanho));
  const porPagina = new Map<number, T[]>();
  if (primeira) porPagina.set(0, primeira.itens);
  paginas.forEach((p, k) => {
    const d = extras[k]?.data;
    if (d) porPagina.set(p, d.itens);
  });
  const item = (i: number): T | undefined => porPagina.get(Math.floor(i / tamanho))?.[i % tamanho];
  return { item, carregados: [...porPagina.values()].flat() };
}
