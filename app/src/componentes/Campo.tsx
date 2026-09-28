import { useId, type InputHTMLAttributes, type ReactNode, type SelectHTMLAttributes, type TextareaHTMLAttributes } from "react";

const base =
  "w-full rounded-lg border bg-white px-3 py-2 text-sm text-slate-900 shadow-xs outline-none transition placeholder:text-slate-400 focus:border-marca-500 focus:ring-3 focus:ring-marca-100";

function Rotulo({ id, rotulo, obrigatorio, erro, ajuda, children }: {
  id: string;
  rotulo: string;
  obrigatorio?: boolean;
  erro?: string;
  ajuda?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium text-slate-700">
        {rotulo}
        {obrigatorio && <span className="ml-0.5 text-rose-600" aria-hidden>*</span>}
      </label>
      {children}
      {erro ? (
        <p id={`${id}-erro`} role="alert" className="text-xs text-rose-600">{erro}</p>
      ) : ajuda ? (
        <p className="text-xs text-slate-500">{ajuda}</p>
      ) : null}
    </div>
  );
}

type Comum = { rotulo: string; erro?: string; ajuda?: string; obrigatorio?: boolean };

export function Campo({ rotulo, erro, ajuda, obrigatorio, ...resto }: Comum & InputHTMLAttributes<HTMLInputElement>) {
  const id = useId();
  return (
    <Rotulo id={id} rotulo={rotulo} obrigatorio={obrigatorio} erro={erro} ajuda={ajuda}>
      <input
        id={id}
        aria-invalid={!!erro}
        aria-describedby={erro ? `${id}-erro` : undefined}
        className={`${base} ${erro ? "border-rose-400" : "border-slate-300"}`}
        {...resto}
      />
    </Rotulo>
  );
}

export function AreaTexto({ rotulo, erro, ajuda, obrigatorio, ...resto }: Comum & TextareaHTMLAttributes<HTMLTextAreaElement>) {
  const id = useId();
  return (
    <Rotulo id={id} rotulo={rotulo} obrigatorio={obrigatorio} erro={erro} ajuda={ajuda}>
      <textarea
        id={id}
        rows={3}
        aria-invalid={!!erro}
        className={`${base} ${erro ? "border-rose-400" : "border-slate-300"}`}
        {...resto}
      />
    </Rotulo>
  );
}

export function Selecao({ rotulo, erro, ajuda, obrigatorio, children, ...resto }: Comum & SelectHTMLAttributes<HTMLSelectElement>) {
  const id = useId();
  return (
    <Rotulo id={id} rotulo={rotulo} obrigatorio={obrigatorio} erro={erro} ajuda={ajuda}>
      <select id={id} aria-invalid={!!erro} className={`${base} ${erro ? "border-rose-400" : "border-slate-300"}`} {...resto}>
        {children}
      </select>
    </Rotulo>
  );
}
