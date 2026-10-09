import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState, type RefObject } from "react";
import { Link } from "react-router-dom";
import { api, type Caso } from "../api/cliente";
import type { SegmentoTranscricao, TranscricaoLocal } from "../api/transcricao";
import type { ArquivoExtracao } from "../api/tipos/ArquivoExtracao";
import { Aviso, Carregando, entrada, Paginacao, Selo, Vazio } from "./Estrutura";
import { Icone } from "./Icone";

function tempo(ms: number) {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return h ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
}

/** Pesquisa e revisão de segmentos derivados; não apresenta inferência quando o provedor local não existe. */
export function TranscricaoLocal({ caso, arquivo }: { caso: Caso; arquivo: ArquivoExtracao }) {
  const [itens, setItens] = useState<TranscricaoLocal[]>([]);
  const [selecionada, setSelecionada] = useState<TranscricaoLocal | null>(null);
  const [segmentos, setSegmentos] = useState<SegmentoTranscricao[]>([]);
  const [total, setTotal] = useState(0);
  const [termo, setTermo] = useState("");
  const [busca, setBusca] = useState("");
  const [erro, setErro] = useState<string | null>(null);
  const [processando, setProcessando] = useState(false);
  const [cancelando, setCancelando] = useState(false);
  const [offset, setOffset] = useState(0);
  const [mediaUrl, setMediaUrl] = useState<string | null>(null);
  const [mediaErro, setMediaErro] = useState<string | null>(null);
  const player = useRef<HTMLAudioElement | HTMLVideoElement>(null);
  const limite = 50;

  const carregar = async () => {
    try { setErro(null); const r = await api.listarTranscricoes(caso.id, arquivo.id); setItens(r); const pronta = r.find(x => x.estado === "pronta"); setSelecionada(pronta ?? r[0] ?? null); } catch (e) { setErro(e instanceof Error ? e.message : "Não foi possível carregar as transcrições."); }
  };
  useEffect(() => { void carregar(); }, [caso.id, arquivo.id]);
  useEffect(() => {
    let ativo = true;
    if (!selecionada || selecionada.estado !== "pronta") { setSegmentos([]); setTotal(0); return; }
    void api.segmentosTranscricao(caso.id, selecionada.id, offset, limite, busca).then(r => { if (ativo) { setSegmentos(r.itens); setTotal(r.total); } }).catch(e => { if (ativo) setErro(e instanceof Error ? e.message : "Falha ao buscar segmentos."); });
    return () => { ativo = false; };
  }, [caso.id, selecionada?.id, offset, busca]);
  useEffect(() => {
    let ativo = true;
    setMediaUrl(null); setMediaErro(null);
    if (!selecionada || selecionada.estado !== "pronta") return () => { ativo = false; };
    void api.caminhoMidiaTranscricao(caso.id, arquivo.id).then(caminho => {
      if (ativo) setMediaUrl(convertFileSrc(caminho));
    }).catch(e => { if (ativo) setMediaErro(e instanceof Error ? e.message : "Não foi possível preparar a reprodução local."); });
    return () => { ativo = false; };
  }, [caso.id, arquivo.id, selecionada?.id, selecionada?.estado]);

  async function iniciar() {
    setProcessando(true); setErro(null);
    try { const r = await api.transcreverLocal(caso.id, arquivo.id); setItens(v => [r, ...v.filter(x => x.id !== r.id)]); setSelecionada(r); setOffset(0); }
    catch (e) { setErro(e instanceof Error ? e.message : "Não foi possível executar a transcrição local."); }
    finally { setProcessando(false); }
  }
  async function cancelar() {
    setCancelando(true); try { await api.cancelarTranscricao(caso.id); } finally { setCancelando(false); }
  }

  return <section className="mt-4 border-t border-borda pt-4" aria-label="Transcrição local">
    <div className="mb-2 flex items-center justify-between gap-2">
      <div><h3 className="flex items-center gap-2 text-sm font-semibold text-texto"><Icone nome="audio" className="size-4 text-ouro" /> Transcrição local</h3><p className="mt-1 text-2xs text-texto-3">Derivada somente do arquivo conferido; nenhum conteúdo é enviado para a rede.</p></div>
      {processando ? <button type="button" className="rounded border border-erro/60 px-2 py-1 text-xs text-erro hover:bg-erro/10" onClick={() => void cancelar()} disabled={cancelando}>{cancelando ? "Cancelando…" : "Cancelar"}</button> : <button type="button" className="rounded border border-borda-2 px-2 py-1 text-xs text-texto hover:border-ouro/70" onClick={() => void iniciar()}><Icone nome="audio" className="mr-1 inline size-3" /> Executar local</button>}
    </div>
    {erro && <Aviso>{erro}</Aviso>}
    {processando && <Carregando />}
    {!processando && itens.length === 0 && <Vazio titulo="Nenhuma transcrição executada" icone="audio" />}
      {!processando && itens.length > 0 && <>
      <div className="mb-2 flex flex-wrap gap-1.5">{itens.map(x => <button key={x.id} type="button" onClick={() => { setSelecionada(x); setOffset(0); }} className={`rounded border px-2 py-1 text-2xs ${selecionada?.id === x.id ? "border-ouro/70 bg-ouro/10 text-ouro" : "border-borda text-texto-2"}`}><Selo tom={x.estado === "pronta" ? "ok" : x.estado === "sem_suporte" ? "atencao" : "erro"}>{x.estado === "sem_suporte" ? "sem suporte" : x.estado}</Selo> · {x.motor_id}</button>)}</div>
      {selecionada?.estado === "sem_suporte" && <div className="rounded border border-aviso/40 bg-aviso/5 p-3 text-xs text-texto-2">{selecionada.erro ?? "Nenhum provedor local disponível."}<div className="mt-1 font-mono text-2xs text-texto-3">Origem: {selecionada.caminho} · SHA-256: {selecionada.sha256_origem.slice(0, 16)}…</div></div>}
      {selecionada?.estado === "pronta" && <>
        <div className="mb-3 rounded border border-borda bg-sup-2/40 p-2">
          <div className="mb-1 flex items-center justify-between gap-2 text-2xs text-texto-3"><span>Reprodução local · os timestamps abaixo navegam neste arquivo</span><Link className="text-info hover:underline" to={`/casos/${caso.id}/consultas`}>Abrir linha do tempo →</Link></div>
          {mediaErro ? <p className="text-xs text-texto-3">{mediaErro}</p> : mediaUrl ? (arquivo.categoria === "video" ? <video ref={player as RefObject<HTMLVideoElement>} className="max-h-48 w-full rounded bg-black" controls preload="metadata" src={mediaUrl} /> : <audio ref={player as RefObject<HTMLAudioElement>} className="w-full" controls preload="metadata" src={mediaUrl} />) : <p className="text-xs text-texto-3">Preparando o arquivo local…</p>}
        </div>
        <form className="mb-2 flex gap-2" onSubmit={e => { e.preventDefault(); setOffset(0); setBusca(termo); }}><input className={`${entrada} min-w-0 flex-1`} value={termo} onChange={e => setTermo(e.target.value)} placeholder="Buscar na transcrição" aria-label="Buscar na transcrição" /><button className="rounded border border-borda-2 px-2 text-xs text-texto" type="submit">Buscar</button></form>
        {segmentos.length === 0 ? <Vazio titulo={busca ? "Nenhum trecho corresponde à busca" : "Transcrição sem segmentos"} icone="audio" /> : <ol className="space-y-1.5" aria-label="Segmentos da transcrição">{segmentos.map(s => <li key={s.id} className="rounded border border-borda bg-sup-2/40 p-2 text-xs"><button type="button" className="mr-2 rounded bg-ouro/10 px-1.5 py-0.5 font-mono text-2xs text-ouro hover:bg-ouro/20" title="Posição no áudio/vídeo" onClick={() => { if (player.current) { player.current.currentTime = s.inicio_ms / 1000; void player.current.play().catch(() => undefined); } }}>{tempo(s.inicio_ms)} → {tempo(s.fim_ms)}</button><span className="text-texto">{s.texto}</span>{s.falante && <span className="ml-2 text-2xs text-texto-3">[{s.falante}]</span>}<Link className="ml-2 text-2xs text-info hover:underline" to={`/casos/${caso.id}/consultas`}>linha do tempo</Link></li>)}</ol>}
        {total > limite && <Paginacao total={total} deslocamento={offset} limite={limite} mudar={setOffset} />}
      </>}
    </>}
  </section>;
}
