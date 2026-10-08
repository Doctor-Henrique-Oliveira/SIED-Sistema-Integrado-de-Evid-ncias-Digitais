// Cabeçalho fixo: contexto do caso, busca global no caso (Ctrl+K), importação em
// curso e estação/usuário. Fino de propósito, para não roubar espaço da área útil.

import { useEffect, useRef, useState, type FormEvent } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import type { Caso, InfoApp } from "../api/cliente";
import { useEstadoImportacao, useFecharCaso } from "../api/consultas";
import { identificacaoCaso } from "../util/formatar";
import { fracaoDaEtapa, PASSOS, passoDaEtapa } from "../util/progresso";
import { nomeEtapa } from "../util/rotulos";
import { configuracoes, grupos } from "./BarraLateral";
import { Icone } from "./Icone";

/** Nome da seção atual para o trilho de navegação (Caso › Módulo). */
function secaoAtual(caminho: string): string | null {
  if (caminho === "/") return "Casos";
  if (caminho === configuracoes.para) return configuracoes.rotulo;
  if (caminho === "/casos/novo") return "Novo caso";
  const m = caminho.match(/^\/casos\/[^/]+(?:\/([^/]+))?/);
  if (!m) return null;
  if (!m[1]) return "Dashboard";
  if (m[1] === "busca") return "Busca";
  for (const g of grupos) for (const mod of g.modulos) if (mod.para.endsWith(`/${m[1]}`)) return mod.rotulo;
  return null;
}

function ImportacaoEmCurso({ caso }: { caso: Caso }) {
  const estado = useEstadoImportacao(caso.id).data;
  if (!estado || estado.terminada) return null;
  const p = estado.progresso;
  const passo = passoDaEtapa(p.etapa);
  const fracao = fracaoDaEtapa(p);
  const pct = fracao == null ? null : Math.round(fracao * 100);
  return (
    <Link
      to={`/casos/${caso.id}/extracoes`}
      title={nomeEtapa[p.etapa] ?? p.etapa}
      className="flex h-7 items-center gap-2 rounded-md border border-ouro/35 bg-ouro/8 px-2.5 text-xs text-ouro-claro transition-colors hover:bg-ouro/14"
    >
      <span className="size-2.5 animate-spin rounded-full border-2 border-ouro/40 border-r-ouro" aria-hidden />
      Importando
      {passo != null && (
        <span className="tabular text-ouro-claro/80">
          {" "}· etapa {passo}/{PASSOS.length}
          {pct != null ? ` · ${pct}%` : ""}
        </span>
      )}
    </Link>
  );
}

function BuscaGlobal({ caso }: { caso: Caso }) {
  const [texto, setTexto] = useState("");
  const campo = useRef<HTMLInputElement>(null);
  const navegar = useNavigate();

  // Ctrl+K (ou Cmd+K) leva o foco para a busca de qualquer tela.
  useEffect(() => {
    function atalho(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        campo.current?.focus();
        campo.current?.select();
      }
    }
    window.addEventListener("keydown", atalho);
    return () => window.removeEventListener("keydown", atalho);
  }, []);

  function enviar(e: FormEvent) {
    e.preventDefault();
    const q = texto.trim();
    if (!q) return;
    navegar(`/casos/${caso.id}/busca?q=${encodeURIComponent(q)}`);
    campo.current?.blur();
  }

  return (
    <form onSubmit={enviar} role="search" aria-label="Busca global no caso" className="relative w-full max-w-lg">
      <Icone nome="busca" className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-texto-3" />
      <input
        ref={campo}
        type="search"
        value={texto}
        onChange={(e) => setTexto(e.target.value)}
        placeholder="Buscar no caso"
        title="Mensagens, telefones e nomes de arquivo do caso aberto"
        aria-label="Busca global no caso"
        className="entrada h-8 min-h-0 w-full border-borda bg-sup/60 py-0 pl-8 pr-16"
      />
      <kbd className="tecla pointer-events-none absolute right-2 top-1/2 -translate-y-1/2">Ctrl K</kbd>
    </form>
  );
}

export function BarraSuperior({ caso, info, preparacao = false }: { caso: Caso | undefined; info: InfoApp | undefined; preparacao?: boolean }) {
  const fechar = useFecharCaso();
  const navegar = useNavigate();
  const secao = secaoAtual(useLocation().pathname);
  const acaoDiscreta = "sied-topbar__acao rounded-sm px-2 py-1 text-xs text-texto-2 transition-colors hover:bg-sup-2 hover:text-texto focus-visible:outline focus-visible:outline-ouro";

  return (
    <header className="sied-topbar flex h-[58px] shrink-0 items-center gap-5 border-b border-borda bg-lateral px-6">
      <nav aria-label="Você está em" className="flex min-w-0 items-center gap-1.5 text-sm">
        {caso ? (
          <>
            {preparacao && <span className="mr-3 flex shrink-0 items-center gap-2.5 border-r border-borda-2 pr-4 text-texto" aria-label="SIED">
              <Icone nome="escudo" className="size-4 text-ouro" />
              <span className="text-xs font-semibold tracking-[0.18em]">SIED</span>
            </span>}
            <span className="sied-topbar__caso flex min-w-0 items-center gap-2">
              <span className="hidden font-mono text-3xs uppercase tracking-[0.12em] text-texto-4 xl:inline">Caso</span>
              <Link to={`/casos/${caso.id}`} className="truncate font-semibold tracking-[-0.01em] text-texto hover:text-ouro-claro" title="Visão geral do caso">
                {identificacaoCaso(caso)}
              </Link>
            </span>
            <span className="hidden truncate text-texto-3 2xl:inline">· {caso.unidade}</span>
            {preparacao ? (
              <><Icone nome="direita" className="size-3.5 shrink-0 text-texto-4" /><span className="shrink-0 text-texto-2">Preparação</span></>
            ) : secao && secao !== "Dashboard" && (
              <>
                <Icone nome="direita" className="size-3.5 shrink-0 text-texto-4" />
                <span className="truncate text-texto-2" aria-current="page">{secao}</span>
              </>
            )}
            <span className="mx-1 h-4 w-px shrink-0 bg-borda-2" aria-hidden />
            <Link to="/" className={acaoDiscreta} title="Trocar de caso">
              Trocar
            </Link>
            <button type="button" onClick={() => fechar.mutate(undefined, { onSuccess: () => navegar("/") })} className={acaoDiscreta}>
              Fechar caso
            </button>
          </>
        ) : (
          <span className="text-texto-2" aria-current="page">
            {secao ?? "Nenhum caso aberto"}
          </span>
        )}
      </nav>

      <div className="flex flex-1 justify-center">{caso && !preparacao && <BuscaGlobal caso={caso} />}</div>

      <div className="flex shrink-0 items-center gap-3 text-xs text-texto-2">
        {caso && !preparacao && <ImportacaoEmCurso caso={caso} />}
        <span
          className="sied-topbar__local hidden h-6 items-center gap-1.5 rounded-sm px-2 text-2xs font-medium uppercase tracking-[0.08em] text-texto-3 ring-1 ring-inset ring-borda-2 lg:flex"
          title="O SIED não usa internet: tudo fica nesta estação"
        >
          <Icone nome="cadeado" className="size-3 text-ok" />
          Local
        </span>
        {info && (
          <span className="flex items-center gap-1.5" title={`Usuário ${info.usuario} na estação ${info.maquina}`}>
            <Icone nome="usuario" className="size-3.5 text-texto-3" />
            <span className="max-w-44 truncate">
              {info.usuario}
              <span className="text-texto-3"> @ {info.maquina}</span>
            </span>
          </span>
        )}
      </div>
    </header>
  );
}
