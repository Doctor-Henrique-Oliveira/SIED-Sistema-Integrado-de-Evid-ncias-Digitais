import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { Link } from "react-router-dom";
import { api, normalizarErro, type Caso, type Deteccao, type Dispositivo, type EstadoImportacao } from "../api/cliente";
import { useCancelarImportacao, useIniciarImportacao } from "../api/consultas";
import { numero, tamanho } from "../util/formatar";
import { duracaoMs, fracaoCopia, fracaoDaEtapa, PASSOS, passoDaEtapa, ritmoInventario, useAgora } from "../util/progresso";
import { nomeEtapa, nomeStatusExtracao, nomeSuporte, tomStatus } from "../util/rotulos";
import { Botao } from "./Botao";
import { Selecao } from "./Campo";
import { Aviso, BarraProgresso, Selo } from "./Estrutura";
import { Icone } from "./Icone";

/** Escolha do dispositivo, da pasta e do importador. */
export function AssistenteImportacao({ caso, dispositivos, aoFechar }: {
  caso: Caso;
  dispositivos: Dispositivo[];
  aoFechar: () => void;
}) {
  const [dispositivoId, setDispositivoId] = useState(dispositivos.length === 1 ? (dispositivos[0]?.id ?? "") : "");
  const [pasta, setPasta] = useState<string | null>(null);
  const [digitada, setDigitada] = useState("");
  const [deteccoes, setDeteccoes] = useState<Deteccao[] | null>(null);
  const [importador, setImportador] = useState<string | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [analisando, setAnalisando] = useState(false);
  const iniciar = useIniciarImportacao(caso.id);

  async function escolherPasta() {
    const p = await open({ directory: true, title: "Selecione a pasta da extração (somente leitura)" });
    if (typeof p !== "string") return;
    await analisar(p);
  }

  async function analisar(p: string) {
    setPasta(p);
    setDigitada(p);
    setErro(null);
    setDeteccoes(null);
    setAnalisando(true);
    try {
      const d = await api.detectarFonte(p);
      setDeteccoes(d);
      setImportador(d.find((x) => x.suporte === "completo" || x.suporte === "parcial")?.importador.id ?? null);
    } catch (e) {
      setErro(normalizarErro(e).message);
    } finally {
      setAnalisando(false);
    }
  }

  async function confirmar() {
    if (!pasta || !importador || !dispositivoId) return;
    setErro(null);
    try {
      await iniciar.mutateAsync({ dispositivoId, pasta, importador });
      aoFechar();
    } catch (e) {
      setErro(normalizarErro(e).message);
    }
  }

  return (
    <div className="space-y-4 rounded-lg border border-borda-2 bg-sup-2/50 p-4" aria-label="Assistente de importação">
      <Aviso tipo="info">
        A pasta da extração é apenas lida: nada é gravado nela. Todo arquivo recebe SHA-256, e o manifesto, as
        cópias de trabalho e os índices ficam na pasta do caso.
      </Aviso>
      <div className="grid gap-4 sm:grid-cols-2">
        <Selecao
          rotulo="Dispositivo"
          obrigatorio
          value={dispositivoId}
          onChange={(e) => setDispositivoId(e.target.value)}
        >
          <option value="">Selecione…</option>
          {dispositivos.map((d) => (
            <option key={d.id} value={d.id}>
              {d.apelido}
            </option>
          ))}
        </Selecao>
        <div className="flex flex-col gap-1.5">
          <label htmlFor="pasta-extracao" className="text-xs font-medium text-texto-2">
            Pasta da extração
          </label>
          <form
            className="flex items-center gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              if (digitada.trim()) void analisar(digitada.trim());
            }}
          >
            <input
              id="pasta-extracao"
              value={digitada}
              onChange={(e) => setDigitada(e.target.value)}
              placeholder="Cole o caminho ou escolha a pasta"
              className="min-w-0 flex-1 rounded-lg border border-borda-2 bg-fundo px-3 py-2 font-mono text-xs text-texto outline-none placeholder:text-texto-3 focus:border-ouro/70 focus:ring-2 focus:ring-ouro/20"
            />
            {digitada.trim() && digitada.trim() !== pasta && (
              <Botao variante="secundario" type="submit" carregando={analisando}>
                Analisar
              </Botao>
            )}
            <Botao variante="secundario" onClick={escolherPasta} carregando={analisando}>
              Escolher…
            </Botao>
          </form>
        </div>
      </div>

      {deteccoes && (
        <fieldset>
          <legend className="mb-2 text-xs font-medium text-texto-2">Ferramenta de extração</legend>
          <div className="space-y-2">
            {deteccoes.map((d) => {
              const disponivel = d.suporte === "completo" || d.suporte === "parcial";
              return (
                <label
                  key={d.importador.id}
                  className={`flex gap-3 rounded-lg border bg-sup p-3 ${
                    importador === d.importador.id ? "border-ouro/60 bg-ouro/5" : "border-borda"
                  } ${disponivel ? "cursor-pointer" : "opacity-60"}`}
                >
                  <input
                    type="radio"
                    name="importador"
                    className="mt-1 accent-[var(--color-ouro)]"
                    disabled={!disponivel}
                    checked={importador === d.importador.id}
                    onChange={() => setImportador(d.importador.id)}
                  />
                  <span className="min-w-0">
                    <span className="flex flex-wrap items-center gap-2 text-[13px] font-medium text-texto">
                      {d.importador.nome}
                      <span className="font-normal text-texto-3">v{d.importador.versao}</span>
                      <Selo tom={disponivel ? "ok" : "neutro"}>{nomeSuporte[d.suporte] ?? d.suporte}</Selo>
                    </span>
                    <ul className="mt-1 list-disc pl-4 text-xs text-texto-2">
                      {d.motivos.map((m) => (
                        <li key={m}>{m}</li>
                      ))}
                    </ul>
                  </span>
                </label>
              );
            })}
          </div>
        </fieldset>
      )}

      {erro && <Aviso>{erro}</Aviso>}
      <div className="flex justify-end gap-2">
        <Botao variante="fantasma" onClick={aoFechar}>
          Cancelar
        </Botao>
        <Botao onClick={confirmar} disabled={!pasta || !importador || !dispositivoId} carregando={iniciar.isPending}>
          Iniciar importação
        </Botao>
      </div>
    </div>
  );
}

/** Progresso da importação em curso ou resumo da última. */
export function PainelImportacao({ caso, estado, aoDispensar }: {
  caso: Caso;
  estado: EstadoImportacao;
  aoDispensar: () => void;
}) {
  const cancelar = useCancelarImportacao();
  const p = estado.progresso;
  const x = estado.extracao;
  const agora = useAgora(!estado.terminada);
  const fracao = fracaoDaEtapa(p);
  const passo = passoDaEtapa(p.etapa);
  const copia = fracaoCopia(p);
  const ritmo = ritmoInventario(p, agora);

  if (estado.terminada) {
    return (
      <div className="rounded-lg border border-borda-2 bg-sup-2/60 p-3.5" aria-label="Resultado da importação">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <p className="flex items-center gap-2 text-[13px] font-medium text-texto">
            Importação {x ? <Selo tom={tomStatus(x.status)}>{nomeStatusExtracao[x.status] ?? x.status}</Selo> : "terminada"}
          </p>
          <div className="flex gap-2">
            <Link
              to={`/casos/${caso.id}/extracoes/${estado.extracao_id}`}
              className="rounded-lg bg-ouro px-3 py-1.5 text-[13px] font-medium text-[#1a1305] hover:bg-ouro-claro"
            >
              Abrir extração
            </Link>
            <Botao variante="fantasma" onClick={aoDispensar}>
              Dispensar
            </Botao>
          </div>
        </div>
        {x && (
          <p className="mt-2 text-[13px] text-texto-2">
            {numero(x.total_arquivos)} arquivos ({tamanho(x.total_bytes)}) inventariados,{" "}
            {numero(x.total_artefatos)} registros interpretados
            {x.total_avisos ? `, ${numero(x.total_avisos)} avisos` : ""}.
          </p>
        )}
        {x?.erro && x.status !== "cancelada" && <p className="mt-2 text-[13px] text-erro">{x.erro}</p>}
        {estado.erro && <p className="mt-2 text-[13px] text-erro">{estado.erro}</p>}
      </div>
    );
  }

  return (
    <div className="space-y-3 rounded-lg border border-ouro/40 bg-ouro/5 p-3.5" aria-label="Importação em andamento">
      <div className="flex items-center justify-between gap-3">
        <p className="flex items-center gap-2 text-[13px] font-medium text-texto" data-teste="etapa-importacao">
          <span className="size-3 animate-spin rounded-full border-2 border-ouro/40 border-r-ouro" aria-hidden />
          {nomeEtapa[p.etapa] ?? p.etapa}…
        </p>
        <Botao
          variante="secundario"
          onClick={() => cancelar.mutate()}
          carregando={cancelar.isPending}
          disabled={p.etapa === "cancelada"}
        >
          Cancelar importação
        </Botao>
      </div>
      <Passos atual={passo} />
      <BarraProgresso fracao={fracao} rotulo="Progresso da importação" />
      <div className="tabular flex flex-wrap gap-x-6 gap-y-1 text-xs text-texto-2">
        <span>
          Arquivos: {numero(p.arquivos_feitos)} de {numero(p.arquivos_total)}
        </span>
        <span>
          Lidos: {tamanho(p.bytes_feitos)} de {tamanho(p.bytes_total)}
        </span>
        {p.tarefas_total > 0 && (
          <span>
            Bancos interpretados: {numero(p.tarefas_feitas)} de {numero(p.tarefas_total)}
          </span>
        )}
        {p.artefatos > 0 && <span>Registros: {numero(p.artefatos)}</span>}
        {p.avisos > 0 && <span className="text-atencao">Avisos: {numero(p.avisos)}</span>}
        {p.inicio_ms != null && <span>Decorrido: {duracaoMs(agora - p.inicio_ms)}</span>}
        {ritmo && (
          <span title="Medido desde o início desta etapa">
            {tamanho(ritmo.porSegundo)}/s
            {ritmo.restanteMs != null && ` · restam cerca de ${duracaoMs(ritmo.restanteMs)}`}
          </span>
        )}
      </div>
      {p.etapa === "processando" && p.fase_tarefa && (
        <p className="tabular text-xs text-texto-2" data-teste="fase-tarefa">
          {p.fase_tarefa === "copiando_verificando"
            ? `Copiando para a pasta do caso e conferindo o SHA-256${
                p.copia_bytes_total ? `: ${tamanho(p.copia_bytes_feitos ?? 0)} de ${tamanho(p.copia_bytes_total)}` : ""
              }${copia != null ? ` (${Math.round(copia * 100)}%)` : ""}`
            : "Lendo a cópia verificada e gravando os registros"}
        </p>
      )}
      {p.item_atual && (
        <p className="truncate font-mono text-xs text-texto-3" title={p.item_atual}>
          {p.item_atual}
        </p>
      )}
      <ConsoleImportacao progresso={p} />
    </div>
  );
}

/** As quatro etapas da importação, com a atual em destaque. */
function Passos({ atual }: { atual: number | null }) {
  return (
    <ol className="grid grid-cols-2 gap-2 sm:grid-cols-4" aria-label="Etapas da importação">
      {PASSOS.map((passo, i) => {
        const n = i + 1;
        const estado = atual == null ? "pendente" : n < atual ? "feito" : n === atual ? "atual" : "pendente";
        return (
          <li
            key={passo.id}
            aria-current={estado === "atual" ? "step" : undefined}
            className={`flex items-center gap-2 rounded-md border px-2 py-1.5 text-xs ${
              estado === "atual"
                ? "border-ouro/60 bg-ouro/10 text-texto"
                : estado === "feito"
                  ? "border-borda text-texto-2"
                  : "border-borda text-texto-3"
            }`}
          >
            <span
              className={`grid size-4.5 shrink-0 place-items-center rounded-full text-[10px] font-semibold ${
                estado === "feito" ? "bg-ok/20 text-ok" : estado === "atual" ? "bg-ouro text-[#1a1305]" : "bg-sup-3 text-texto-3"
              }`}
              aria-hidden
            >
              {estado === "feito" ? "✓" : n}
            </span>
            <span className="truncate">{passo.rotulo}</span>
            <span className="sr-only">{estado === "feito" ? " (concluída)" : estado === "atual" ? " (em andamento)" : ""}</span>
          </li>
        );
      })}
    </ol>
  );
}

/**
 * Espaço do console técnico da importação (roadmap, CONTINUIDADE.md §14). Hoje mostra só
 * o estado real que o backend já informa a cada consulta; o registro contínuo de eventos
 * (hash, cópia verificada, parsers, gravação, indexação) virá do backend, sem simulação.
 */
export function ConsoleImportacao({ progresso: p }: { progresso: EstadoImportacao["progresso"] }) {
  const linhas: [string, string][] = [
    ["etapa", p.etapa],
    ["arquivos", `${numero(p.arquivos_feitos)}/${numero(p.arquivos_total)}`],
    ["bytes", `${tamanho(p.bytes_feitos)}/${tamanho(p.bytes_total)}`],
    ["bancos", `${numero(p.tarefas_feitas)}/${numero(p.tarefas_total)}`],
    ["registros", numero(p.artefatos)],
    ["avisos", numero(p.avisos)],
  ];
  if (p.fase_tarefa) linhas.push(["tarefa", p.fase_tarefa]);
  if (p.copia_bytes_total != null) linhas.push(["copia", `${tamanho(p.copia_bytes_feitos ?? 0)}/${tamanho(p.copia_bytes_total)}`]);
  if (p.item_atual) linhas.push(["item", p.item_atual]);
  if (p.mensagem) linhas.push(["mensagem", p.mensagem]);
  return (
    <details className="group rounded-md border border-borda bg-fundo">
      <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-1.5 text-xs text-texto-2 hover:text-texto">
        <Icone nome="direita" className="size-3.5 transition-transform group-open:rotate-90" />
        <Icone nome="terminal" className="size-3.5" />
        Console técnico
        <span className="text-texto-3">· estado atual informado pelo backend</span>
      </summary>
      <pre className="overflow-x-auto border-t border-borda px-3 py-2 font-mono text-[11px] leading-relaxed text-texto-2" aria-label="Estado técnico da importação">
        {linhas.map(([k, v]) => `${k.padEnd(10)}${v}`).join("\n")}
      </pre>
    </details>
  );
}
