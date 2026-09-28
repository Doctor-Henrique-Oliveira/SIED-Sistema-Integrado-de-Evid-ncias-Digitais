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
