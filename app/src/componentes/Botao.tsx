import type { ButtonHTMLAttributes } from "react";
import { Icone, type NomeIcone } from "./Icone";

type Variante = "primario" | "secundario" | "fantasma" | "perigo";

const estilos: Record<Variante, string> = {
  // Dourado só para a ação principal da tela.
  primario: "bg-ouro text-[#1a1305] hover:bg-ouro-claro disabled:bg-sup-3 disabled:text-texto-3",
  secundario: "border border-borda-2 bg-sup-2 text-texto hover:bg-sup-3 disabled:text-texto-3 disabled:hover:bg-sup-2",
  fantasma: "text-texto-2 hover:bg-sup-2 hover:text-texto disabled:text-texto-3 disabled:hover:bg-transparent",
  perigo: "border border-erro/40 bg-erro/10 text-erro hover:bg-erro/20 disabled:opacity-50",
};

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variante?: Variante;
  carregando?: boolean;
  icone?: NomeIcone;
};

export function Botao({ variante = "primario", carregando, disabled, icone, className = "", children, ...resto }: Props) {
  return (
    <button
      type="button"
      disabled={disabled || carregando}
      className={`inline-flex items-center justify-center gap-2 rounded-lg px-3 py-1.5 text-[13px] font-medium transition-colors disabled:cursor-not-allowed ${estilos[variante]} ${className}`}
      {...resto}
    >
      {carregando ? (
        <span className="size-3.5 animate-spin rounded-full border-2 border-current border-r-transparent" aria-hidden />
      ) : (
        icone && <Icone nome={icone} className="size-4" />
      )}
      {children}
    </button>
  );
}
