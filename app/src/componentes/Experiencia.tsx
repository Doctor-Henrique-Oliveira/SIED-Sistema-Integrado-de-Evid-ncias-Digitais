import { useEffect, useMemo, useState } from "react";
import type { CasoAberto, InfoApp, VerificacaoAuditoria } from "../api/cliente";
import { api } from "../api/cliente";
import { identificacaoCaso, numero } from "../util/formatar";
import { Icone } from "./Icone";

export type FaseExperiencia = "splash" | "welcome" | "creating-case" | "opening-case" | "booting-case" | "workspace" | "error";

export function SplashScreen({ aoTerminar, duracao = 1900 }: { aoTerminar: () => void; duracao?: number }) {
  useEffect(() => {
    const reduzir = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    const id = window.setTimeout(aoTerminar, reduzir ? 180 : duracao);
    return () => window.clearTimeout(id);
  }, [aoTerminar, duracao]);

  return (
    <main className="splash-sied" aria-label="Inicialização do SIED">
      <div className="splash-sied__marca" aria-hidden>
        <span className="splash-sied__anel splash-sied__anel--externo" />
        <span className="splash-sied__anel splash-sied__anel--interno" />
        <span className="splash-sied__nucleo"><Icone nome="escudo" className="size-11" /></span>
      </div>
      <div className="splash-sied__assinatura">
        <p className="splash-sied__sigla">SIED</p>
        <p className="splash-sied__nome">Sistema Integrado de Evidências Digitais</p>
        <span className="splash-sied__linha" aria-hidden />
        <p className="splash-sied__estado">Inicializando estação forense local</p>
      </div>
    </main>
  );
}

type EventoBoot = { id: string; nivel: "step" | "ok" | "warn" | "error" | "metric"; rotulo: string; detalhe?: string };

export function ForensicBootConsole({ aberto, info, aoPronto, aoErro }: {
  aberto: CasoAberto;
  info?: InfoApp;
  aoPronto: () => void;
  aoErro: (mensagem: string) => void;
}) {
  const [auditoria, setAuditoria] = useState<VerificacaoAuditoria | null>(null);
  const [pronto, setPronto] = useState(false);

  useEffect(() => {
    let ativo = true;
    api.verificarAuditoria()
      .then((resultado) => { if (ativo) setAuditoria(resultado); })
      .catch((erro: unknown) => { if (ativo) aoErro(erro instanceof Error ? erro.message : String(erro)); });
    return () => { ativo = false; };
  }, [aberto.caso.id, aoErro]);

  useEffect(() => {
    if (!auditoria) return;
    const id = window.setTimeout(() => setPronto(true), 420);
    return () => window.clearTimeout(id);
  }, [auditoria]);
  useEffect(() => {
    if (!pronto) return;
    const id = window.setTimeout(aoPronto, 520);
    return () => window.clearTimeout(id);
  }, [pronto, aoPronto]);

  const eventos = useMemo<EventoBoot[]>(() => [
    { id: "pasta", nivel: "ok", rotulo: "Pasta de caso reconhecida", detalhe: aberto.pasta },
    { id: "banco", nivel: "ok", rotulo: "Banco do caso aberto" },
    { id: "caso", nivel: "ok", rotulo: "Caso identificado", detalhe: identificacaoCaso(aberto.caso) },
    ...(auditoria ? [{ id: "auditoria", nivel: "ok" as const, rotulo: "Trilha de auditoria conferida", detalhe: `${numero(auditoria.registros)} registros` }] :
      [{ id: "auditoria", nivel: "step" as const, rotulo: "Conferindo trilha de auditoria" }]),
    ...(info ? [{ id: "sessao", nivel: "ok" as const, rotulo: "Sessão local preparada", detalhe: `${info.usuario} @ ${info.maquina}` }] : []),
    ...(pronto ? [{ id: "workspace", nivel: "ok" as const, rotulo: "Workspace pronto" }] : []),
  ], [aberto, auditoria, info, pronto]);

  return (
    <main className="boot-forense" aria-label="Preparando workspace do caso">
      <section className="boot-forense__terminal">
        <header className="boot-forense__cabecalho">
          <span className="flex items-center gap-2"><Icone nome="terminal" className="size-4 text-ouro" /> SIED FORENSIC SESSION</span>
          <span className="text-2xs text-texto-3">LOCAL · OFFLINE</span>
        </header>
        <div className="boot-forense__contexto">
          <p className="sobrelinha text-ouro!">Inicialização operacional</p>
          <h1>{identificacaoCaso(aberto.caso)}</h1>
        </div>
        <ol className="boot-forense__eventos" role="log" aria-live="polite">
          {eventos.map((evento, indice) => (
            <li key={evento.id} className={`boot-forense__evento boot-forense__evento--${evento.nivel}`} style={{ animationDelay: `${indice * 70}ms` }}>
              <span className="boot-forense__indice">{String(indice + 1).padStart(2, "0")}</span>
              <span className="boot-forense__nivel">[{evento.nivel === "step" ? "..." : evento.nivel.toUpperCase()}]</span>
              <span className="min-w-0"><strong>{evento.rotulo}</strong>{evento.detalhe && <small title={evento.detalhe}>{evento.detalhe}</small>}</span>
            </li>
          ))}
        </ol>
        <footer className={`boot-forense__rodape ${pronto ? "is-ready" : ""}`}>
          <span className="boot-forense__cursor" aria-hidden />
          {pronto ? "CASO PRONTO · liberando modo operacional" : "Validando ambiente do caso"}
        </footer>
      </section>
    </main>
  );
}

export function ErroExperiencia({ mensagem, aoVoltar, aoTentar }: { mensagem: string; aoVoltar: () => void; aoTentar: () => void }) {
  return (
    <main className="boot-forense">
      <section className="boot-forense__terminal" role="alert">
        <header className="boot-forense__cabecalho text-erro"><span>[ERRO] Não foi possível preparar o caso</span></header>
        <div className="p-8">
          <p className="sobrelinha">Motivo</p><p className="mt-2 text-sm text-erro-claro">{mensagem}</p>
          <p className="mt-6 text-sm text-texto-2">O caso permanece protegido. Tente novamente ou volte à tela inicial.</p>
          <div className="mt-6 flex gap-2">
            <button type="button" className="botao botao-primario" onClick={aoTentar}>Tentar novamente</button>
            <button type="button" className="botao botao-secundario" onClick={aoVoltar}>Voltar</button>
          </div>
        </div>
      </section>
    </main>
  );
}
