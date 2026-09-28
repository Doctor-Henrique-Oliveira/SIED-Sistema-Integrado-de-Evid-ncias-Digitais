import type { ButtonHTMLAttributes } from "react";

type Variante = "primario" | "secundario" | "fantasma";

const estilos: Record<Variante, string> = {
  primario: "bg-marca-600 text-white hover:bg-marca-700 disabled:bg-slate-300",
  secundario:
    "bg-white text-slate-700 ring-1 ring-inset ring-slate-300 hover:bg-slate-50 disabled:text-slate-400",
  fantasma: "text-slate-600 hover:bg-slate-200/60 disabled:text-slate-400",
};

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variante?: Variante;
  carregando?: boolean;
};

export function Botao({ variante = "primario", carregando, disabled, className = "", children, ...resto }: Props) {
  return (
    <button
      type="button"
      disabled={disabled || carregando}
      className={`inline-flex items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-marca-600 disabled:cursor-not-allowed ${estilos[variante]} ${className}`}
      {...resto}
    >
      {carregando && (
        <span className="size-3.5 animate-spin rounded-full border-2 border-current border-r-transparent" aria-hidden />
      )}
      {children}
    </button>
  );
}
