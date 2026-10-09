import { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { api, type ArestaRelacao, type Caso, type ContextoRelacoes, type NoRelacao } from "../api/cliente";
import { dataHoraMs } from "../util/formatar";
import { Botao } from "./Botao";
import { Aviso, Carregando, Hash, Selo, Vazio } from "./Estrutura";

const JANELAS = [5, 30, 60, 1440] as const;

function destino(casoId: string, no: NoRelacao) {
  if (no.artefato_id !== null && no.conversa_id !== null) return `/casos/${casoId}/conversas/${no.conversa_id}?registro=${no.artefato_id}`;
  if (no.artefato_id !== null) return `/casos/${casoId}/extracoes/${no.extracao_id}?registro=${no.artefato_id}`;
  if (no.conversa_id !== null) return `/casos/${casoId}/conversas/${no.conversa_id}`;
  if (no.arquivo_id !== null) return `/casos/${casoId}/arquivos?extracao=${encodeURIComponent(no.extracao_id)}&arquivo=${no.arquivo_id}`;
  return `/casos/${casoId}/extracoes/${no.extracao_id}`;
}

function GrafoVisual({ nos, arestas, onSelect }: { nos: NoRelacao[]; arestas: ArestaRelacao[]; onSelect: (aresta: ArestaRelacao) => void }) {
  const posicoes = new Map(nos.map((no, indice) => [no.chave, { x: 50 + 38 * Math.cos((indice / Math.max(nos.length, 1)) * Math.PI * 2), y: 50 + 38 * Math.sin((indice / Math.max(nos.length, 1)) * Math.PI * 2) }]));
  return <div className="overflow-hidden rounded-md border border-borda bg-fundo p-2"><svg viewBox="0 0 100 100" role="img" aria-label="Grafo investigativo local" className="h-64 w-full">
    {arestas.map((aresta, indice) => { const de = posicoes.get(aresta.de); const para = posicoes.get(aresta.para); if (!de || !para) return null; return <line key={`${aresta.de}:${aresta.para}:${indice}`} x1={de.x} y1={de.y} x2={para.x} y2={para.y} stroke={aresta.classe === "direta" ? "#6ea8fe" : "#d6a94f"} strokeWidth="1.4" strokeDasharray={aresta.classe === "sugerida" ? "3 2" : undefined} tabIndex={0} role="button" aria-label={`Selecionar relação ${aresta.classe}`} onClick={() => onSelect(aresta)} onKeyDown={(evento) => { if (evento.key === "Enter" || evento.key === " ") onSelect(aresta); }} />; })}
    {nos.map((no) => { const p = posicoes.get(no.chave)!; return <g key={no.chave}><circle cx={p.x} cy={p.y} r="4" fill={no.artefato_id !== null ? "#d6a94f" : "#6ea8fe"} stroke="#0b1220" strokeWidth="1.2" /><text x={p.x} y={p.y + 7} textAnchor="middle" fill="currentColor" fontSize="3.1">{no.rotulo.slice(0, 24)}</text><title>{no.rotulo}</title></g>; })}
  </svg><p className="px-1 text-2xs text-texto-3">Linhas azuis = relação direta registrada · linhas douradas tracejadas = sugestão temporal. Selecione uma linha para explicar a origem.</p></div>;
}

/** Grafo limitado de relações verificáveis e contexto temporal do registro selecionado. */
export function ContextoRelacional({ caso, artefatoId }: { caso: Caso; artefatoId: number }) {
  const [janela, setJanela] = useState<(typeof JANELAS)[number]>(30);
  const [resultado, setResultado] = useState<ContextoRelacoes | null>(null);
  const [aresta, setAresta] = useState<ArestaRelacao | null>(null);
  const [operacao, setOperacao] = useState<string | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const ativa = useRef(true);
  const atual = useRef<string | null>(null);
  useEffect(() => {
    ativa.current = true;
    return () => {
      ativa.current = false;
      const id = atual.current; atual.current = null;
      if (id) void api.cancelarDescoberta(id).catch(() => undefined);
    };
  }, []);

  async function consultar() {
    if (atual.current || !Number.isSafeInteger(artefatoId) || artefatoId <= 0) return;
    const id = crypto.randomUUID();
    atual.current = id; setOperacao(id); setResultado(null); setAresta(null); setErro(null);
    try {
      const recebido = await api.contextoRelacoes(caso.id, id, artefatoId, janela, 30);
      if (!ativa.current || atual.current !== id) return;
      if (recebido.caso_id !== caso.id || recebido.origem.artefato.id !== artefatoId ||
          recebido.janela_minutos !== janela || recebido.eventos.itens.length > 30 ||
          recebido.nos.some((no) => no.extracao_id === "")) {
        throw new Error("O contexto recebido não corresponde ao registro e à janela solicitados.");
      }
      setResultado(recebido);
    } catch (falha) {
      if (ativa.current && atual.current === id) setErro(falha instanceof Error ? falha.message : "Não foi possível obter o contexto relacional.");
    } finally {
      if (ativa.current && atual.current === id) { atual.current = null; setOperacao(null); }
    }
  }

  const nos = new Map(resultado?.nos.map((no) => [no.chave, no]) ?? []);
  const origem = resultado?.nos.find((no) => no.artefato_id === artefatoId);
  return <section aria-label="Contexto relacional experimental" className="mt-3 space-y-3 rounded-md border border-borda bg-fundo p-3">
    <div className="flex flex-wrap items-center justify-between gap-2"><h4 className="text-sm font-semibold text-texto">Contexto relacional do registro #{artefatoId}</h4><Selo tom="atencao">EXPERIMENTAL</Selo></div>
    <p className="text-xs text-texto-2">Vínculos de origem e conversa são diretos; proximidade de horário é somente sugestão contextual. Cada aresta mostra sua regra.</p>
    <div className="flex flex-wrap items-end gap-2"><label className="text-xs text-texto-2">Janela temporal
      <select aria-label="Janela temporal relacional" className="entrada ml-2" value={janela} disabled={operacao !== null} onChange={(evento) => setJanela(Number(evento.target.value) as (typeof JANELAS)[number])}>
        {JANELAS.map((minutos) => <option key={minutos} value={minutos}>{minutos === 1440 ? "±24 horas" : `±${minutos} minutos`}</option>)}
      </select></label>
      <Botao tamanho="p" variante="secundario" carregando={operacao !== null} onClick={() => void consultar()}>Explorar até 30 eventos</Botao>
      {operacao && <Botao tamanho="p" variante="fantasma" onClick={() => void api.cancelarDescoberta(operacao)}>Cancelar</Botao>}
    </div>
    {operacao && <Carregando texto="Consultando vínculos e eventos do caso…" />}
    {erro && <Aviso>{erro}</Aviso>}
    {resultado && <>
      <p className="text-xs text-texto-2">{resultado.eventos.total} evento(s) com data na janela · {resultado.nos.length} nó(s) · {resultado.arestas.length} ligação(ões) nesta página.</p>
      {resultado.limitado && <Selo tom="atencao">PÁGINA LIMITADA · NÃO REPRESENTA TODO O CASO</Selo>}
      {resultado.avisos.length > 0 && <ul className="list-inside list-disc space-y-1 text-xs text-texto-2">{resultado.avisos.map((aviso, indice) => <li key={indice}>{aviso}</li>)}</ul>}
      <div className="border-t border-borda pt-3"><p className="sobrelinha">Grafo local · selecione a ligação para conferir sua origem</p>
        {origem && <p className="mt-2 text-sm text-texto">Origem: <Link className="text-ouro-claro hover:underline" to={destino(caso.id, origem)}>{origem.rotulo}</Link></p>}
        {resultado.nos.length > 0 && <div className="mt-3"><GrafoVisual nos={resultado.nos} arestas={resultado.arestas} onSelect={setAresta} /></div>}
        {resultado.arestas.length === 0 ? <Vazio titulo="Nenhum vínculo nesta página">A ausência aqui não prova que não existam relações fora da janela ou dos limites.</Vazio> :
          <ul aria-label="Ligações do grafo" className="mt-2 grid gap-2 lg:grid-cols-2">{resultado.arestas.map((ligacao, indice) => {
            const de = nos.get(ligacao.de); const para = nos.get(ligacao.para);
            return <li key={`${ligacao.de}:${ligacao.para}:${indice}`}>
              <button type="button" className="w-full rounded-md border border-borda p-2 text-left text-xs text-texto-2 hover:border-ouro-claro focus-visible:outline focus-visible:outline-ouro-claro" onClick={() => setAresta(ligacao)}>
                <Selo tom={ligacao.classe === "direta" ? "info" : "atencao"}>{ligacao.classe === "direta" ? "DIRETA" : "SUGERIDA"}</Selo>
                <span className="ml-2 break-all">{de?.rotulo ?? ligacao.de} → {para?.rotulo ?? ligacao.para}</span>
              </button>
            </li>;
          })}</ul>}
        {aresta && <div role="region" aria-label="Explicação da ligação selecionada" className="mt-3 border-l-2 border-ouro-claro pl-3 text-xs text-texto-2">
          <p className="font-semibold text-texto">Por que estes itens estão relacionados?</p>
          <p className="mt-1">{aresta.motivo}</p><p className="mt-1 font-mono">Mecanismo: {aresta.mecanismo}</p>
          {aresta.score_contexto !== null && <p>Proximidade temporal normalizada: {aresta.score_contexto.toFixed(3)} · não é probabilidade de vínculo.</p>}
          <div className="mt-2 flex flex-wrap gap-3">{[aresta.de, aresta.para].map((chave) => {
            const no = nos.get(chave);
            return no ? <Link key={chave} to={destino(caso.id, no)} className="text-ouro-claro hover:underline">Abrir {no.rotulo}</Link> : null;
          })}</div>
          {[aresta.de, aresta.para].map((chave) => {
            const no = nos.get(chave);
            return no?.hash_origem ? <p key={chave} className="mt-1">{no.rotulo} · SHA-256 histórico do inventário: <Hash valor={no.hash_origem} /></p> : null;
          })}
        </div>}
      </div>
      <div className="border-t border-borda pt-3"><h5 className="text-sm font-semibold text-texto">Timeline da janela</h5>
        {resultado.eventos.itens.length === 0 ? <p className="mt-2 text-xs text-texto-3">Sem eventos datados nesta janela ou data da origem indisponível.</p> : <ol aria-label="Eventos na janela" className="mt-2 divide-y divide-borda">{resultado.eventos.itens.map((evento) => <li key={evento.artefato.id} className="flex flex-wrap items-center justify-between gap-2 py-2 text-xs"><span className="font-mono text-texto-3">{evento.artefato.data_hora_ms === null ? "Data não informada" : dataHoraMs(evento.artefato.data_hora_ms, caso.fuso_horario)}</span><span>{evento.artefato.tipo} · registro #{evento.artefato.id}</span><Link className="text-ouro-claro hover:underline" to={destino(caso.id, { chave: "", tipo: "artefato", rotulo: "", artefato_id: evento.artefato.id, arquivo_id: null, conversa_id: evento.conversa_id, extracao_id: evento.extracao_id, hash_origem: null })}>Abrir origem</Link></li>)}</ol>}
      </div>
    </>}
  </section>;
}
