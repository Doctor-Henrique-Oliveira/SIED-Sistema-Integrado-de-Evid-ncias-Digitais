// Casca do aplicativo: barra lateral fixa, cabeçalho e a área útil rolável.

import { useEffect, useRef } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { useCasoAtual, useInfoApp } from "../api/consultas";
import { BarraLateral } from "../componentes/BarraLateral";
import { BarraSuperior } from "../componentes/BarraSuperior";
import { Aviso } from "../componentes/Estrutura";

export function Layout() {
  const info = useInfoApp();
  const casoAtual = useCasoAtual();
  const caso = casoAtual.data?.caso;
  const { pathname } = useLocation();
  const principal = useRef<HTMLElement>(null);

  // Cada tela começa do topo (a área útil é um único elemento rolável).
  useEffect(() => {
    principal.current?.scrollTo?.({ top: 0 });
  }, [pathname]);

  return (
    <div className="flex h-full">
      {/* Botão (não âncora): com o hash router, "#conteudo" trocaria a rota. */}
      <button
        type="button"
        onClick={() => document.getElementById("conteudo")?.focus()}
        className="sr-only focus:not-sr-only focus:absolute focus:left-2 focus:top-2 focus:z-50 focus:rounded-md focus:bg-ouro focus:px-3 focus:py-1.5 focus:text-ouro-tinta"
      >
        Pular para o conteúdo
      </button>
      <BarraLateral caso={caso} info={info.data} />
      <div className="flex min-w-0 flex-1 flex-col">
        <BarraSuperior caso={caso} info={info.data} />
        <main ref={principal} id="conteudo" tabIndex={-1} className="min-h-0 outline-none flex-1 overflow-y-auto">
          <div className="mx-auto max-w-[1680px] px-6 py-5">
            {info.isError ? (
              <Aviso>
                <p className="font-medium">O SIED não conseguiu iniciar.</p>
                <p className="mt-1">{info.error.message}</p>
              </Aviso>
            ) : (
              <Outlet />
            )}
          </div>
        </main>
      </div>
    </div>
  );
}
