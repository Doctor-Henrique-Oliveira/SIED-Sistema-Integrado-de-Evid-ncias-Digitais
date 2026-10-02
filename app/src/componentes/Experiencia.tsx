import { useEffect, useMemo, useState, type CSSProperties } from "react";
import type { CasoAberto, InfoApp, VerificacaoAuditoria } from "../api/cliente";
import { api } from "../api/cliente";
import { identificacaoCaso, numero } from "../util/formatar";
import { Icone } from "./Icone";

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
  const fluxoEsquerdo = ["CASE_DB", "FTS5", "MEDIA", "AUDIT", "RECORD"];
  const fluxoDireito = ["SQLITE", "INDEX", "CONVERSATION", "LOCAL_SESSION", "DEVICE"];

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
      { id: "workspace", codigo: "WORKSPACE", rotulo: "Workspace pronto", detalhe: pronto ? "Ambiente operacional liberado" : "Preparando ambiente operacional", estado: definirEstado(5, pronto) },
    ];
  }, [aberto, auditoria, etapaAtiva, info, pronto]);

  return (
    <main className={`boot-forense ${pronto ? "is-ready" : etapaAtiva >= 5 ? "is-workspace-ready" : "is-processing"}`} aria-label="Preparando workspace do caso">
      <div className="boot-forense__fluxos" aria-hidden="true">
        <div className="boot-forense__fluxo boot-forense__fluxo--esquerdo">
          <span className="boot-forense__fluxo-legenda">AUXILIARY TELEMETRY</span>
          {fluxoEsquerdo.map((item, indice) => <span key={item} style={{ "--fluxo-atraso": `${indice * -1.15}s` } as CSSProperties}>{item}</span>)}
        </div>
        <div className="boot-forense__fluxo boot-forense__fluxo--direito">
          <span className="boot-forense__fluxo-legenda">VISUAL DATA LAYER</span>
          {fluxoDireito.map((item, indice) => <span key={item} style={{ "--fluxo-atraso": `${indice * -1.45}s` } as CSSProperties}>{item}</span>)}
        </div>
      </div>
      <section className="boot-forense__estacao">
        <header className="boot-forense__cabecalho" data-tauri-drag-region>
          <span className="boot-forense__marca"><Icone nome="terminal" className="size-4" /> SIED <b>/</b> FORENSIC BOOT</span>
          <span className="boot-forense__ambiente"><i /> LOCAL · OFFLINE</span>
        </header>
        <div className="boot-forense__corpo">
          <div className="boot-forense__introducao">
            <p className="sobrelinha text-ouro!">Estação de análise digital</p>
            <h1>Inicializando caso</h1>
            <p className="boot-forense__caso">{identificacaoCaso(aberto.caso)}</p>
            <p className="boot-forense__pasta" title={aberto.pasta}>{aberto.pasta}</p>
          </div>
          <div className="boot-forense__telemetria" aria-label="Estado da sessão">
            <div><span>MODE</span><strong>LOCAL / OFFLINE</strong></div>
            <div><span>CASE</span><strong>{aberto.caso.id.slice(0, 8).toUpperCase()}</strong></div>
            <div><span>STATION</span><strong>{info?.maquina ?? "LOCAL"}</strong></div>
            <div><span>RECORDS</span><strong>{auditoria ? numero(auditoria.registros) : "—"}</strong></div>
          </div>
          <ol className="boot-forense__eventos" role="log" aria-live="polite" aria-label="Etapas de inicialização">
            {eventos.map((evento, indice) => (
              <li key={evento.id} className={`boot-forense__evento boot-forense__evento--${evento.estado}`}>
                <span className="boot-forense__trilho" aria-hidden><i /></span>
                <span className="boot-forense__indice">{String(indice + 1).padStart(2, "0")}</span>
                <span className="boot-forense__leitura">
                  <span className="boot-forense__codigo">{evento.codigo}</span>
                  <strong>{evento.rotulo}</strong>
                  {evento.detalhe && <small title={evento.detalhe}>{evento.detalhe}</small>}
                </span>
                <span className="boot-forense__estado" aria-label={evento.estado}>{evento.estado === "ok" ? "OK" : evento.estado === "processando" ? "PROCESSING" : "—"}</span>
              </li>
            ))}
          </ol>
        </div>
        <footer className={`boot-forense__rodape ${pronto ? "is-ready" : ""}`}>
          <span className="boot-forense__cursor" aria-hidden />
          <span>{pronto ? "CASE READY" : "PROCESSING CASE ENVIRONMENT"}</span>
          <span className="boot-forense__integridade">{pronto ? "INTEGRITY: OK" : "AUDIT VALIDATION"}</span>
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
