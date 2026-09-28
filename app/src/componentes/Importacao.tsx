import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { Link } from "react-router-dom";
import { api, normalizarErro, type Caso, type Deteccao, type Dispositivo, type EstadoImportacao } from "../api/cliente";
import { useCancelarImportacao, useIniciarImportacao } from "../api/consultas";
import { numero, tamanho } from "../util/formatar";
import { nomeEtapa, nomeStatusExtracao, nomeSuporte, tomStatus } from "../util/rotulos";
import { Botao } from "./Botao";
import { Selecao } from "./Campo";
import { Aviso, BarraProgresso, Selo } from "./Estrutura";

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
    <div className="space-y-5 rounded-lg bg-slate-50 p-4 ring-1 ring-slate-200">
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
          <label htmlFor="pasta-extracao" className="text-sm font-medium text-slate-700">
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
              className="min-w-0 flex-1 rounded-lg border border-slate-300 bg-white px-3 py-2 font-mono text-xs outline-none focus:border-marca-500"
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
          <legend className="mb-2 text-sm font-medium text-slate-700">Ferramenta de extração</legend>
          <div className="space-y-2">
            {deteccoes.map((d) => {
              const disponivel = d.suporte === "completo" || d.suporte === "parcial";
              return (
                <label
                  key={d.importador.id}
                  className={`flex gap-3 rounded-lg bg-white p-3 ring-1 ${
                    importador === d.importador.id ? "ring-marca-500" : "ring-slate-200"
                  } ${disponivel ? "cursor-pointer" : "opacity-60"}`}
                >
                  <input
                    type="radio"
                    name="importador"
                    className="mt-1"
                    disabled={!disponivel}
                    checked={importador === d.importador.id}
                    onChange={() => setImportador(d.importador.id)}
                  />
                  <span className="min-w-0">
                    <span className="flex items-center gap-2 text-sm font-medium text-slate-900">
                      {d.importador.nome}
                      <span className="font-normal text-slate-500">v{d.importador.versao}</span>
                      <Selo tom={disponivel ? "ok" : "neutro"}>{nomeSuporte[d.suporte] ?? d.suporte}</Selo>
                    </span>
                    <ul className="mt-1 list-disc pl-4 text-xs text-slate-600">
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
  const fracao = p.bytes_total > 0 ? p.bytes_feitos / p.bytes_total : p.etapa === "contando" ? null : 0;

  if (estado.terminada) {
    return (
      <div className="rounded-lg bg-white p-4 ring-1 ring-slate-200" aria-label="Resultado da importação">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <p className="flex items-center gap-2 text-sm font-medium text-slate-900">
            Importação {x ? <Selo tom={tomStatus(x.status)}>{nomeStatusExtracao[x.status] ?? x.status}</Selo> : "terminada"}
          </p>
          <div className="flex gap-2">
            <Link
              to={`/casos/${caso.id}/extracoes/${estado.extracao_id}`}
              className="rounded-lg px-3 py-1.5 text-sm font-medium text-marca-700 hover:bg-marca-50"
            >
              Abrir extração
            </Link>
            <Botao variante="fantasma" onClick={aoDispensar}>
              Dispensar
            </Botao>
          </div>
        </div>
        {x && (
          <p className="mt-2 text-sm text-slate-600">
            {numero(x.total_arquivos)} arquivos ({tamanho(x.total_bytes)}) inventariados,{" "}
            {numero(x.total_artefatos)} registros interpretados
            {x.total_avisos ? `, ${numero(x.total_avisos)} avisos` : ""}.
          </p>
        )}
        {x?.erro && x.status !== "cancelada" && <p className="mt-2 text-sm text-rose-700">{x.erro}</p>}
        {estado.erro && <p className="mt-2 text-sm text-rose-700">{estado.erro}</p>}
      </div>
    );
  }

  return (
    <div className="space-y-3 rounded-lg bg-white p-4 ring-1 ring-marca-500" aria-label="Importação em andamento">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-medium text-slate-900" data-teste="etapa-importacao">
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
      <BarraProgresso fracao={fracao} rotulo="Progresso da importação" />
      <div className="flex flex-wrap gap-x-6 gap-y-1 text-xs text-slate-600">
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
      </div>
      {p.item_atual && (
        <p className="truncate font-mono text-xs text-slate-500" title={p.item_atual}>
          {p.item_atual}
        </p>
      )}
    </div>
  );
}
