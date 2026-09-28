import type { ReactNode } from "react";

export function Cabecalho({ titulo, subtitulo, acoes }: { titulo: string; subtitulo?: ReactNode; acoes?: ReactNode }) {
  return (
    <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
      <div className="min-w-0">
        <h1 className="truncate text-2xl font-semibold tracking-tight text-slate-900">{titulo}</h1>
        {subtitulo && <div className="mt-1 text-sm text-slate-500">{subtitulo}</div>}
      </div>
      {acoes && <div className="flex gap-2">{acoes}</div>}
    </header>
  );
}

export function Cartao({ titulo, acoes, children, className = "" }: {
  titulo?: string;
  acoes?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`rounded-xl bg-white shadow-sm ring-1 ring-slate-200 ${className}`}>
      {(titulo || acoes) && (
        <div className="flex items-center justify-between border-b border-slate-100 px-5 py-3">
          {titulo && <h2 className="text-sm font-semibold text-slate-900">{titulo}</h2>}
          {acoes}
        </div>
      )}
      <div className="p-5">{children}</div>
    </section>
  );
}

/** Lista de pares rótulo/valor; valores ausentes aparecem como "—". */
export function Definicoes({ itens }: { itens: [string, ReactNode][] }) {
  return (
    <dl className="grid grid-cols-1 gap-x-6 gap-y-3 sm:grid-cols-2 lg:grid-cols-3">
      {itens.map(([rotulo, valor]) => (
        <div key={rotulo} className="min-w-0">
          <dt className="text-xs font-medium uppercase tracking-wide text-slate-500">{rotulo}</dt>
          <dd className="mt-0.5 break-words text-sm text-slate-900">{valor ?? "—"}</dd>
        </div>
      ))}
    </dl>
  );
}

export function Aviso({ tipo = "erro", children }: { tipo?: "erro" | "info" | "sucesso"; children: ReactNode }) {
  const cores = {
    erro: "bg-rose-50 text-rose-800 ring-rose-200",
    info: "bg-sky-50 text-sky-800 ring-sky-200",
    sucesso: "bg-emerald-50 text-emerald-800 ring-emerald-200",
  }[tipo];
  return (
    <div role={tipo === "erro" ? "alert" : "status"} className={`rounded-lg px-4 py-3 text-sm ring-1 ${cores}`}>
      {children}
    </div>
  );
}

export function Carregando({ texto = "Carregando…" }: { texto?: string }) {
  return (
    <div className="flex items-center gap-2 py-6 text-sm text-slate-500" role="status">
      <span className="size-4 animate-spin rounded-full border-2 border-slate-300 border-r-transparent" aria-hidden />
      {texto}
    </div>
  );
}

export function Vazio({ titulo, children }: { titulo: string; children?: ReactNode }) {
  return (
    <div className="rounded-xl border-2 border-dashed border-slate-200 px-6 py-10 text-center">
      <p className="font-medium text-slate-700">{titulo}</p>
      {children && <div className="mt-2 text-sm text-slate-500">{children}</div>}
    </div>
  );
}

export function Selo({ tom = "neutro", children }: { tom?: "ok" | "atencao" | "erro" | "neutro"; children: ReactNode }) {
  const cores = {
    ok: "bg-emerald-50 text-emerald-700 ring-emerald-200",
    atencao: "bg-amber-50 text-amber-800 ring-amber-200",
    erro: "bg-rose-50 text-rose-700 ring-rose-200",
    neutro: "bg-slate-100 text-slate-700 ring-slate-200",
  }[tom];
  return (
    <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ring-1 ring-inset ${cores}`}>
      {children}
    </span>
  );
}

/** Hash completo em fonte monoespaçada, quebrando linha quando precisar. */
export function Hash({ valor }: { valor: string | null | undefined }) {
  if (!valor) return <>—</>;
  return <span className="break-all font-mono text-xs">{valor}</span>;
}

export function BarraProgresso({ fracao, rotulo }: { fracao: number | null; rotulo: string }) {
  const pct = fracao == null ? null : Math.max(0, Math.min(100, fracao * 100));
  return (
    <div
      role="progressbar"
      aria-label={rotulo}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct == null ? undefined : Math.round(pct)}
      className="h-2 w-full overflow-hidden rounded-full bg-slate-200"
    >
      <div
        className={`h-full rounded-full bg-marca-600 transition-[width] duration-300 ${pct == null ? "w-1/3 animate-pulse" : ""}`}
        style={pct == null ? undefined : { width: `${pct}%` }}
      />
    </div>
  );
}

export function Paginacao({ total, deslocamento, limite, mudar }: {
  total: number;
  deslocamento: number;
  limite: number;
  mudar: (deslocamento: number) => void;
}) {
  if (total <= limite) return null;
  const fim = Math.min(deslocamento + limite, total);
  return (
    <div className="mt-3 flex items-center justify-between text-sm text-slate-600">
      <span>
        {(deslocamento + 1).toLocaleString("pt-BR")}–{fim.toLocaleString("pt-BR")} de {total.toLocaleString("pt-BR")}
      </span>
      <div className="flex gap-2">
        <button
          type="button"
          className="rounded-md px-3 py-1 ring-1 ring-slate-300 hover:bg-slate-50 disabled:opacity-40"
          disabled={deslocamento === 0}
          onClick={() => mudar(Math.max(0, deslocamento - limite))}
        >
          Anterior
        </button>
        <button
          type="button"
          className="rounded-md px-3 py-1 ring-1 ring-slate-300 hover:bg-slate-50 disabled:opacity-40"
          disabled={fim >= total}
          onClick={() => mudar(deslocamento + limite)}
        >
          Próxima
        </button>
      </div>
    </div>
  );
}
