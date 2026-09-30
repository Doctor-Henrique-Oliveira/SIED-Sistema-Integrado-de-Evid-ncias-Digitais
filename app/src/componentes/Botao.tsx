import type { ButtonHTMLAttributes } from "react";
import { Icone, type NomeIcone } from "./Icone";

export type Variante = "primario" | "secundario" | "fantasma" | "perigo";
export type Tamanho = "m" | "p";

/** Classes de um botão do design system (receitas em estilos.css). Também usadas por LinkBotao. */
export function classeBotao(variante: Variante = "primario", tamanho: Tamanho = "m") {
  // Dourado só para a ação principal da tela.
  const v = {
    primario: "botao-primario",
    secundario: "botao-secundario",
    fantasma: "botao-fantasma",
    perigo: "botao-perigo",
  }[variante];
  return `botao ${tamanho === "p" ? "botao-p" : ""} ${v}`;
}

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variante?: Variante;
  tamanho?: Tamanho;
  carregando?: boolean;
  icone?: NomeIcone;
};

export function Botao({ variante = "primario", tamanho = "m", carregando, disabled, icone, className = "", children, ...resto }: Props) {
  const tamIcone = tamanho === "p" ? "size-3.5" : "size-4";
  return (
    <button type="button" disabled={disabled || carregando} className={`${classeBotao(variante, tamanho)} ${className}`} {...resto}>
      {carregando ? (
        <span className={`${tamIcone} animate-spin rounded-full border-2 border-current border-r-transparent`} aria-hidden />
      ) : (
        icone && <Icone nome={icone} className={tamIcone} />
      )}
      {children}
    </button>
  );
}
