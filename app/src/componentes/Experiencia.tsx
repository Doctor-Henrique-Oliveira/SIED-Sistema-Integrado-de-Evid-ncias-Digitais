import { useEffect, useMemo, useState, type CSSProperties } from "react";
import type { CasoAberto, InfoApp, VerificacaoAuditoria } from "../api/cliente";
import { api } from "../api/cliente";
import { identificacaoCaso, numero } from "../util/formatar";
import { Icone } from "./Icone";
import "./ForensicBoot.css";

export type FaseExperiencia = "splash" | "welcome" | "creating-case" | "opening-case" | "booting-case" | "workspace" | "error";

export function SplashScreen({ aoTerminar, duracao = 3800 }: { aoTerminar: () => void; duracao?: number }) {
  useEffect(() => {
    const reduzir = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    const id = window.setTimeout(aoTerminar, reduzir ? 180 : duracao);
    return () => window.clearTimeout(id);
  }, [aoTerminar, duracao]);

  return (
    <main
      className="splash-sied"
      aria-label="Inicialização do SIED"
      style={{ "--sied-splash-duration": `${duracao}ms` } as CSSProperties}
    >
      <span className="splash-sied__halo" aria-hidden />
      <div className="splash-sied__marca" aria-hidden>
        <span className="splash-sied__anel splash-sied__anel--orbital" />
        <span className="splash-sied__anel splash-sied__anel--externo" />
        <span className="splash-sied__anel splash-sied__anel--interno" />
        <span className="splash-sied__eixos" />
        <span className="splash-sied__nucleo">
          <span className="splash-sied__varredura" />
          <Icone nome="escudo" className="size-12" />
        </span>
      </div>
      <div className="splash-sied__assinatura">
        <p className="splash-sied__sigla">SIED</p>
        <p className="splash-sied__nome">Sistema Integrado de Evidências Digitais</p>
        <div className="splash-sied__inicializacao">
          <span className="splash-sied__linha" aria-hidden><span /></span>
          <p className="splash-sied__estado"><span aria-hidden />Inicializando estação forense local</p>
        </div>
      </div>
    </main>
  );
}

type EstadoBoot = "aguardando" | "processando" | "ok";
type EventoBoot = {
  id: string;
  codigo: string;
  rotulo: string;
  detalhe?: string;
  estado: EstadoBoot;
};

export function ForensicBootConsole({ aberto, info, aoPronto, aoErro }: {
  aberto: CasoAberto;
  info?: InfoApp;
  aoPronto: () => void;
  aoErro: (mensagem: string) => void;
}) {
  const [auditoria, setAuditoria] = useState<VerificacaoAuditoria | null>(null);
  const [etapaAtiva, setEtapaAtiva] = useState(0);
  const [pronto, setPronto] = useState(false);

  useEffect(() => {
    let ativo = true;
    api.verificarAuditoria()
      .then((resultado) => { if (ativo) setAuditoria(resultado); })
      .catch((erro: unknown) => { if (ativo) aoErro(erro instanceof Error ? erro.message : String(erro)); });
    return () => { ativo = false; };
  }, [aberto.caso.id, aoErro]);

  useEffect(() => {
    if (etapaAtiva >= 3 && !auditoria) return;
    if (etapaAtiva >= 5) {
      if (!auditoria) return;
      const id = window.setTimeout(() => setPronto(true), 720);
      return () => window.clearTimeout(id);
    }
    const id = window.setTimeout(() => setEtapaAtiva((atual) => atual + 1), etapaAtiva === 0 ? 680 : 600);
    return () => window.clearTimeout(id);
  }, [auditoria, etapaAtiva]);
  useEffect(() => {
    if (!pronto) return;
    const id = window.setTimeout(aoPronto, 520);
    return () => window.clearTimeout(id);
  }, [pronto, aoPronto]);

  const eventos = useMemo<EventoBoot[]>(() => {
    const definirEstado = (indice: number, concluido = indice < etapaAtiva): EstadoBoot => {
      if (indice < etapaAtiva && concluido) return "ok";
      if (indice === etapaAtiva) return concluido ? "ok" : "processando";
      return "aguardando";
    };
    return [
      { id: "pasta", codigo: "DIRECTORY", rotulo: "Pasta de caso reconhecida", detalhe: aberto.pasta, estado: definirEstado(0) },
      { id: "banco", codigo: "CASE DATABASE", rotulo: "Banco do caso aberto", detalhe: "caso.sied.db", estado: definirEstado(1) },
      { id: "caso", codigo: "CASE IDENTITY", rotulo: "Caso identificado", detalhe: identificacaoCaso(aberto.caso), estado: definirEstado(2) },
      { id: "auditoria", codigo: "AUDIT CHAIN", rotulo: auditoria ? "Trilha de auditoria conferida" : "Conferindo trilha de auditoria", detalhe: auditoria ? `${numero(auditoria.registros)} registros conferidos` : "Validação em andamento", estado: definirEstado(3, Boolean(auditoria)) },
      { id: "sessao", codigo: "LOCAL SESSION", rotulo: "Sessão local preparada", detalhe: info ? `${info.usuario} @ ${info.maquina}` : "Estação local", estado: definirEstado(4, Boolean(info)) },
      { id: "workspace", codigo: "WORKSPACE", rotulo: pronto ? "Workspace pronto" : "Preparação do workspace", detalhe: pronto ? "Ambiente operacional liberado" : "Preparando ambiente operacional", estado: definirEstado(5, pronto) },
    ];
  }, [aberto, auditoria, etapaAtiva, info, pronto]);

  return (
    <main className={`boot-forense ${pronto ? "is-ready" : etapaAtiva >= 5 ? "is-workspace-ready" : "is-processing"}`} aria-label="Preparando workspace do caso">
      <div className="boot-forense__stream" aria-hidden="true">
        {["SQLITE / FTS5 / LOCAL", "RECORD / MEDIA / INDEX", "CONVERSATION / CASE_DB", "LOCAL_SESSION / AUDIT"].map((texto, i) => (
          <div key={texto} style={{ "--stream-i": i } as CSSProperties}>
            <span>VISUAL · {texto} / ◇ ─ ◇ / {texto} / ◇ ─ ◇ / {texto}</span>
          </div>
        ))}
      </div>
      <header className="boot-forense__topo" data-tauri-drag-region>
        <span className="boot-forense__marca"><Icone nome="escudo" /> SIED <span>FORENSIC SESSION</span></span>
        <span className="boot-forense__modo"><i /> LOCAL / OFFLINE</span>
      </header>
      <div className="boot-forense__cena">
        <section className="boot-forense__identidade" aria-label="Sessão do caso">
          <div className="boot-forense__emblema" aria-hidden="true">
            <svg viewBox="0 0 200 200" className="boot-forense__orbita">
              <circle cx="100" cy="100" r="89" />
              <circle cx="100" cy="100" r="72" />
              <path d="M100 5v13M100 182v13M5 100h13M182 100h13" />
            </svg>
            <span><Icone nome="escudo" /></span>
          </div>
          <p className="boot-forense__sobrelinha">ESTAÇÃO DE ANÁLISE DIGITAL</p>
          <h1>{pronto ? "Sessão preparada." : "Preparando sua sessão."}</h1>
          <p className="boot-forense__caso" title={identificacaoCaso(aberto.caso)}>{identificacaoCaso(aberto.caso)}</p>
          <p className="boot-forense__pasta" title={aberto.pasta}>{aberto.pasta}</p>
          <div className="boot-forense__marcos" aria-label="Etapas da sessão">
            {eventos.map((evento, i) => <span key={evento.id} data-estado={evento.estado} title={`${i + 1}: ${evento.codigo}`} />)}
          </div>
          <p className="boot-forense__nota">{pronto ? "Ambiente operacional liberado" : "Preparação local do ambiente de investigação"}</p>
        </section>
        <section className="boot-forense__sequencia" aria-label="Sequência operacional">
          <div className="boot-forense__legenda"><span>INICIALIZAÇÃO DO CASO</span><span>01 — 06</span></div>
          <ol className="boot-forense__eventos" role="log" aria-live="polite" aria-label="Etapas de inicialização">
            {eventos.map((evento, indice) => (
              <li key={evento.id} className={`boot-forense__evento boot-forense__evento--${evento.estado}`}>
                <span className="boot-forense__indice">{String(indice + 1).padStart(2, "0")}</span>
                <div className="boot-forense__leitura">
                  <span className="boot-forense__codigo">{evento.codigo}</span>
                  <strong>{evento.rotulo}</strong>
                  <small title={evento.detalhe}>{evento.estado === "aguardando" ? "Na sequência" : evento.detalhe}</small>
                </div>
                <span className="boot-forense__estado" aria-label={evento.estado}>
                  {evento.estado === "ok" ? <Icone nome="check" /> : evento.estado === "processando" ? <i /> : <span>—</span>}
                </span>
              </li>
            ))}
          </ol>
        </section>
      </div>
      <footer className="boot-forense__rodape">
        <div className="boot-forense__estacao-info"><span>CASE <b title={aberto.caso.id}>{aberto.caso.id.slice(0, 8).toUpperCase()}</b></span><span>STATION <b title={info?.maquina}>{info?.maquina ?? "—"}</b></span></div>
        <div className="boot-forense__conclusao" role="status">{pronto ? <><Icone nome="check" /> CASE READY</> : <><span className="boot-forense__cursor" /> PREPARANDO SESSÃO</>}</div>
      </footer>
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
