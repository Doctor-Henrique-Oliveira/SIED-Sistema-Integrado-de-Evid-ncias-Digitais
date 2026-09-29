// Leitura do progresso da importação para a tela. Tudo vem dos números que o
// backend informa; nada é simulado. Quando o backend não tem como medir a etapa,
// a fração é nula e a barra fica indeterminada.

import { useEffect, useState } from "react";
import type { Progresso } from "../api/tipos/Progresso";

export const PASSOS = [
  { id: "contagem", rotulo: "Contagem" },
  { id: "inventario", rotulo: "Inventário e SHA-256" },
  { id: "bancos", rotulo: "Bancos conhecidos" },
  { id: "interpretacao", rotulo: "Cópia verificada e leitura" },
] as const;

/** Passo (1 a 4) da etapa atual; null nas etapas terminais. */
export function passoDaEtapa(etapa: Progresso["etapa"]): number | null {
  switch (etapa) {
    case "preparando":
    case "contando":
      return 1;
    case "inventariando":
      return 2;
    case "planejando":
      return 3;
    case "processando":
      return 4;
    default:
      return null;
  }
}

/** Fração concluída da etapa atual, ou null quando não há medida. */
export function fracaoDaEtapa(p: Progresso): number | null {
  switch (p.etapa) {
    case "inventariando":
      if (p.bytes_total > 0) return p.bytes_feitos / p.bytes_total;
      return p.arquivos_total > 0 ? p.arquivos_feitos / p.arquivos_total : null;
    case "processando":
      return p.tarefas_total > 0 ? p.tarefas_feitas / p.tarefas_total : null;
    default:
      return null;
  }
}

/** Fração da cópia de trabalho em curso (cópia + conferência do SHA-256). */
export function fracaoCopia(p: Progresso): number | null {
  if (p.fase_tarefa !== "copiando_verificando" || !p.copia_bytes_total) return null;
  return (p.copia_bytes_feitos ?? 0) / p.copia_bytes_total;
}

/** "2 min 13 s" a partir de milissegundos. */
export function duracaoMs(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min ${s % 60} s`;
  return `${Math.floor(m / 60)} h ${m % 60} min`;
}

/**
 * Taxa e tempo restante do inventário, medidos desde o início da etapa. Só aparece
 * depois de alguns segundos de medida, para não mostrar números instáveis.
 */
export function ritmoInventario(p: Progresso, agora: number): { porSegundo: number; restanteMs: number | null } | null {
  if (p.etapa !== "inventariando" || p.etapa_inicio_ms == null || p.bytes_total <= 0) return null;
  const decorrido = agora - p.etapa_inicio_ms;
  if (decorrido < 3000 || p.bytes_feitos <= 0) return null;
  const porSegundo = (p.bytes_feitos / decorrido) * 1000;
  const faltam = p.bytes_total - p.bytes_feitos;
  return { porSegundo, restanteMs: porSegundo > 0 ? (faltam / porSegundo) * 1000 : null };
}

/** Relógio que avança a cada segundo enquanto `ativo`. */
export function useAgora(ativo: boolean): number {
  const [agora, setAgora] = useState(() => Date.now());
  useEffect(() => {
    if (!ativo) return;
    const t = setInterval(() => setAgora(Date.now()), 1000);
    return () => clearInterval(t);
  }, [ativo]);
  return agora;
}
