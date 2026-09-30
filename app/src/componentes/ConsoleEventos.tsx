// Console técnico da importação (CONTINUIDADE.md §14.1): cada linha é um evento real do
// backend (inventário, hash, cópia conferida, parser, gravação, aviso, erro), com o horário
// no fuso do caso. Nada é animado ou simulado.

import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { EventoImportacao, NivelEvento } from "../api/cliente";
import { hora, numero } from "../util/formatar";
import { Icone } from "./Icone";

const COR: Record<NivelEvento, string> = {
  info: "text-texto-2",
  ok: "text-ok",
  aviso: "text-atencao",
  erro: "text-erro",
};
const MARCA: Record<NivelEvento, string> = { info: " ", ok: "✓", aviso: "!", erro: "✗" };
const NOME: Record<NivelEvento, string> = { info: "informação", ok: "concluído", aviso: "aviso", erro: "erro" };

/** Decorrido desde o primeiro evento (+mm:ss ou +h:mm:ss). */
function decorrido(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h > 0 ? `+${h}:${String(m).padStart(2, "0")}:${ss}` : `+${String(m).padStart(2, "0")}:${ss}`;
}

export function ConsoleEventos({ eventos, fuso, aoVivo = false, aberto: abertoInicial = true, carregando = false }: {
  eventos: EventoImportacao[];
  fuso: string;
  /** Importação em curso: acompanha o fim da lista enquanto o usuário não rolar para cima. */
  aoVivo?: boolean;
  aberto?: boolean;
  carregando?: boolean;
}) {
  const [aberto, setAberto] = useState(abertoInicial);
  const id = useId();
  const caixa = useRef<HTMLOListElement>(null);
  const noFim = useRef(true);
  const avisos = eventos.filter((e) => e.nivel === "aviso").length;
  const erros = eventos.filter((e) => e.nivel === "erro").length;
  const inicio = eventos[0]?.ts_ms;

  // Mantém a última linha à vista se o usuário estava no fim.
  useLayoutEffect(() => {
    const c = caixa.current;
    if (c && aberto && noFim.current) c.scrollTop = c.scrollHeight;
  }, [eventos.length, aberto]);
  useEffect(() => {
    noFim.current = true;
  }, [aberto]);

  return (
    <section className="overflow-hidden rounded-md border border-borda bg-fundo" aria-label="Console da importação">
      <button
        type="button"
        aria-expanded={aberto}
        aria-controls={id}
        onClick={() => setAberto((a) => !a)}
        className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-texto-2 transition-colors hover:text-texto"
      >
        <Icone nome="direita" className={`size-3.5 transition-transform ${aberto ? "rotate-90" : ""}`} />
        <Icone nome="terminal" className="size-3.5" />
        <span className="font-medium">Console da importação</span>
        <span className="tabular text-texto-3">
          · {numero(eventos.length)} evento(s)
          {avisos > 0 && <span className="text-atencao"> · {numero(avisos)} linha(s) de aviso</span>}
          {erros > 0 && <span className="text-erro"> · {numero(erros)} linha(s) de erro</span>}
        </span>
        {aoVivo && (
          <span className="ml-auto flex items-center gap-1.5 text-2xs uppercase tracking-wide text-ouro-claro">
            <span className="size-1.5 animate-pulse rounded-full bg-ouro" aria-hidden />
            ao vivo
          </span>
        )}
      </button>
      <div id={id} hidden={!aberto} className="border-t border-borda">
        {eventos.length === 0 ? (
          <p className="px-3 py-2 font-mono text-2xs text-texto-3">
            {carregando ? "Lendo o log da importação…" : aoVivo ? "Aguardando o primeiro evento do backend…" : "Nenhum evento registrado para esta importação."}
          </p>
        ) : (
          <ol
            ref={caixa}
            role="log"
            aria-live={aoVivo ? "polite" : "off"}
            aria-label="Eventos da importação"
            onScroll={(e) => {
              const c = e.currentTarget;
              noFim.current = c.scrollHeight - c.scrollTop - c.clientHeight < 24;
            }}
            className="max-h-72 overflow-y-auto px-3 py-2 font-mono text-2xs leading-relaxed"
          >
            {eventos.map((e) => (
              <li key={e.seq} className={`grid grid-cols-[auto_auto_auto_minmax(0,1fr)] gap-x-2 ${COR[e.nivel]}`} data-nivel={e.nivel}>
                <span className="tabular text-texto-3">[{hora(e.ts_ms, fuso)}]</span>
                <span className="tabular text-texto-3/70">{inicio != null ? decorrido(e.ts_ms - inicio) : ""}</span>
                <span aria-hidden className="w-2 text-center">
                  {MARCA[e.nivel]}
                </span>
                <span className="break-words">
                  {e.nivel !== "info" && <span className="sr-only">{NOME[e.nivel]}: </span>}
                  {e.mensagem}
                </span>
              </li>
            ))}
          </ol>
        )}
      </div>
    </section>
  );
}
