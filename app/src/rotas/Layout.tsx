// Casca do aplicativo: barra lateral fixa, cabeçalho e a área útil rolável.

import { useCallback, useEffect, useRef, useState } from "react";
import { Outlet, useLocation, useNavigate } from "react-router-dom";
import { useAbrirCaso, useCasoAtual, useInfoApp } from "../api/consultas";
import { BarraLateral } from "../componentes/BarraLateral";
import { BarraSuperior } from "../componentes/BarraSuperior";
import { ErroExperiencia, ForensicBootConsole, SplashScreen, type FaseExperiencia } from "../componentes/Experiencia";
import { Aviso } from "../componentes/Estrutura";
import { prepararJanelaEntrada, prepararJanelaWorkspace } from "../util/janela";

export function Layout() {
  const info = useInfoApp();
  const casoAtual = useCasoAtual();
  const caso = casoAtual.data?.caso;
  const aberto = casoAtual.data;
  const { pathname } = useLocation();
  const navegar = useNavigate();
  const abrir = useAbrirCaso();
  const [fase, setFase] = useState<FaseExperiencia>("splash");
  const [splashConcluida, setSplashConcluida] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const principal = useRef<HTMLElement>(null);

  const rotaCaso = pathname.match(/^\/casos\/([^/]+)/)?.[1];
  const irAposSplash = useCallback(() => {
    setSplashConcluida(true);
  }, []);

  useEffect(() => {
    if (!splashConcluida || fase !== "splash" || casoAtual.isPending || info.isPending) return;
    if (aberto) setFase("booting-case");
    else setFase(pathname === "/casos/novo" ? "creating-case" : "welcome");
  }, [aberto, casoAtual.isPending, fase, info.isPending, pathname, splashConcluida]);

  useEffect(() => { void prepararJanelaEntrada(); }, []);
  useEffect(() => {
    if (fase === "splash") return;
    if (aberto && fase !== "booting-case" && fase !== "workspace" && fase !== "error") setFase("booting-case");
    if (!aberto && fase === "workspace") setFase(pathname === "/casos/novo" ? "creating-case" : "welcome");
    if (!aberto && (fase === "welcome" || fase === "creating-case")) setFase(pathname === "/casos/novo" ? "creating-case" : "welcome");
  }, [aberto, fase, pathname]);

  const erroNoBoot = useCallback((mensagem: string) => { setErro(mensagem); setFase("error"); }, []);
  const workspacePronto = useCallback(() => {
    if (!aberto) return;
    void prepararJanelaWorkspace().finally(() => {
      setFase("workspace");
      if (pathname === "/") navegar(`/casos/${aberto.caso.id}`, { replace: true });
    });
  }, [aberto, navegar, pathname]);

  useEffect(() => {
    if (fase === "splash" || !rotaCaso || rotaCaso === "novo" || aberto?.caso.id === rotaCaso || abrir.isPending || abrir.isError) return;
    setFase("opening-case");
    abrir.mutate(rotaCaso, { onError: (e) => { setErro(e.message); setFase("error"); } });
  }, [aberto?.caso.id, abrir, fase, rotaCaso]);

  // Cada tela começa do topo (a área útil é um único elemento rolável).
  useEffect(() => {
    principal.current?.scrollTo?.({ top: 0 });
  }, [pathname]);

  if (fase === "splash") return <SplashScreen aoTerminar={irAposSplash} />;
  if ((fase === "opening-case" || fase === "booting-case") && aberto) {
    return <ForensicBootConsole aberto={aberto} info={info.data} aoErro={erroNoBoot} aoPronto={workspacePronto} />;
  }
  if (fase === "opening-case" || fase === "booting-case") return <main className="boot-forense"><p className="font-mono text-sm text-texto-2">Abrindo caso…</p></main>;
  if (fase === "error") return <ErroExperiencia mensagem={erro ?? abrir.error?.message ?? "Falha não identificada."} aoVoltar={() => { setErro(null); setFase("welcome"); navegar("/"); }} aoTentar={() => { setErro(null); abrir.reset(); setFase("welcome"); navegar("/"); }} />;

  if (!aberto) {
    return (
      <div className="h-full overflow-y-auto bg-fundo">
        <main className="mx-auto min-h-full max-w-[1120px] px-6 py-6" id="conteudo" tabIndex={-1}>
          {info.isError ? <Aviso>{info.error.message}</Aviso> : <Outlet />}
        </main>
      </div>
    );
  }

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
