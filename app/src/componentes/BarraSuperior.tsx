// Cabeçalho fixo: contexto do caso, busca global no caso (Ctrl+K), importação em
// curso e estação/usuário. Fino de propósito, para não roubar espaço da área útil.

import { useEffect, useRef, useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import type { Caso, InfoApp } from "../api/cliente";
import { useEstadoImportacao, useFecharCaso } from "../api/consultas";
import { identificacaoCaso } from "../util/formatar";
import { fracaoDaEtapa, PASSOS, passoDaEtapa } from "../util/progresso";
import { nomeEtapa } from "../util/rotulos";
import { Icone } from "./Icone";

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
      className="flex items-center gap-2 rounded-lg border border-ouro/35 bg-ouro/10 px-2.5 py-1 text-xs text-ouro-claro hover:bg-ouro/15"
    >
      <span className="size-3 animate-spin rounded-full border-2 border-ouro/40 border-r-ouro" aria-hidden />
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
    <form onSubmit={enviar} role="search" aria-label="Busca global no caso" className="relative w-full max-w-md">
      <Icone nome="busca" className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-texto-3" />
      <input
        ref={campo}
        type="search"
        value={texto}
        onChange={(e) => setTexto(e.target.value)}
        placeholder="Buscar no caso"
        title="Mensagens, telefones e nomes de arquivo do caso aberto"
        aria-label="Busca global no caso"
        className="w-full rounded-lg border border-borda bg-fundo py-1.5 pl-8 pr-14 text-[13px] text-texto outline-none placeholder:text-texto-3 focus:border-ouro/60 focus:ring-2 focus:ring-ouro/15"
      />
      <kbd className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 rounded border border-borda-2 px-1.5 font-mono text-[10px] text-texto-3">
        Ctrl K
      </kbd>
    </form>
  );
}

export function BarraSuperior({ caso, info }: { caso: Caso | undefined; info: InfoApp | undefined }) {
  const fechar = useFecharCaso();
  const navegar = useNavigate();

  return (
    <header className="flex h-13 shrink-0 items-center gap-4 border-b border-borda bg-lateral/80 px-5">
      <div className="flex min-w-0 items-center gap-2 text-[13px]">
        {caso ? (
          <>
            <Icone nome="casos" className="size-4 text-texto-3" />
            <Link to={`/casos/${caso.id}`} className="truncate font-medium text-texto hover:text-ouro-claro" title="Visão geral do caso">
              {identificacaoCaso(caso)}
            </Link>
            <span className="hidden truncate text-texto-3 xl:inline">· {caso.unidade}</span>
            <Link
              to="/"
              className="ml-1 rounded-md px-1.5 py-0.5 text-xs text-texto-3 hover:bg-sup-2 hover:text-texto"
              title="Trocar de caso"
            >
              Trocar
            </Link>
            <button
              type="button"
              onClick={() => fechar.mutate(undefined, { onSuccess: () => navegar("/") })}
              className="rounded-md px-1.5 py-0.5 text-xs text-texto-3 hover:bg-sup-2 hover:text-texto"
            >
              Fechar caso
            </button>
          </>
        ) : (
          <span className="text-texto-3">Nenhum caso aberto</span>
        )}
      </div>

      <div className="flex flex-1 justify-center">{caso && <BuscaGlobal caso={caso} />}</div>

      <div className="flex shrink-0 items-center gap-3 text-xs text-texto-2">
        {caso && <ImportacaoEmCurso caso={caso} />}
        <span className="hidden items-center gap-1.5 rounded-md border border-borda px-2 py-1 lg:flex" title="O SIED não usa internet: tudo fica nesta estação">
          <Icone nome="cadeado" className="size-3.5 text-ok" />
          Local
        </span>
        {info && (
          <span className="flex items-center gap-1.5" title={`Usuário ${info.usuario} na estação ${info.maquina}`}>
            <Icone nome="usuario" className="size-4 text-texto-3" />
            <span className="max-w-40 truncate">
              {info.usuario}
              <span className="text-texto-3"> @ {info.maquina}</span>
            </span>
          </span>
        )}
      </div>
    </header>
  );
}
