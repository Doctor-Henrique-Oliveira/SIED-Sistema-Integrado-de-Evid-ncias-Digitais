import { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { api, type Caso, type EstadoRevisaoEntidade, type Pagina, type CandidatoRevisao, type FiltroFilaRevisao } from "../api/cliente";
import { Botao } from "./Botao";
import { AreaTexto, Selecao } from "./Campo";
import { Aviso, Carregando, Hash, Paginacao, Selo, Vazio } from "./Estrutura";

const ESTADOS: [EstadoRevisaoEntidade, string][] = [
  ["nao_revisado", "Não revisado"],
  ["confirmado", "Confirmado"],
  ["inconclusivo", "Inconclusivo"],
  ["descartado", "Descartado"],
];

function mensagem(erro: unknown) {
  return erro instanceof Error ? erro.message : "A fila de revisão não pôde ser carregada.";
}

function tom(estado: EstadoRevisaoEntidade): "ouro" | "ok" | "erro" | "atencao" | "neutro" {
  if (estado === "confirmado") return "ok";
  if (estado === "descartado") return "erro";
  if (estado === "inconclusivo") return "atencao";
  return "neutro";
}

function validar(pagina: Pagina<CandidatoRevisao>, casoId: string): Pagina<CandidatoRevisao> {
  if (!Number.isSafeInteger(pagina.total) || pagina.total < 0 || pagina.itens.length > 50) throw new Error("A fila recebida excede os limites esperados.");
  for (const item of pagina.itens) {
    if (item.caso_id !== casoId || !Number.isSafeInteger(item.id) || item.id <= 0 || item.documento_id <= 0 || item.arquivo_id <= 0) throw new Error("A fila recebeu um candidato de outro caso.");
    if (!Number.isSafeInteger(item.inicio_utf8) || !Number.isSafeInteger(item.fim_utf8) || item.inicio_utf8 < 0 || item.fim_utf8 <= item.inicio_utf8 || item.fim_utf8 - item.inicio_utf8 > 4096) throw new Error("A fila recebeu offsets documentais inválidos.");
    if (!ESTADOS.some(([estado]) => estado === item.estado) || item.literal.length === 0 || item.literal.length > 4096 || item.motor_versao.length > 400) throw new Error("A fila recebeu um candidato sem proveniência válida.");
  }
  return pagina;
}

export function FilaRevisao({ caso }: { caso: Caso }) {
  const [aberto, setAberto] = useState(false);
  const [filtroEstado, setFiltroEstado] = useState<EstadoRevisaoEntidade | "todos">("nao_revisado");
  const [motor, setMotor] = useState("");
  const [pagina, setPagina] = useState<Pagina<CandidatoRevisao> | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const [recarga, setRecarga] = useState(0);
  const [carregando, setCarregando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const montado = useRef(true);
  useEffect(() => () => { montado.current = false; }, []);
  useEffect(() => {
    if (!aberto) return;
    let atual = true;
    setCarregando(true); setErro(null); setPagina(null);
    const filtro: FiltroFilaRevisao = { estado: filtroEstado === "todos" ? null : filtroEstado, motor_versao: motor.trim() || null };
    void api.filaRevisao(caso.id, filtro, deslocamento, 50).then((resultado) => {
      if (!atual || !montado.current) return;
      setPagina(validar(resultado, caso.id));
    }).catch((falha) => { if (atual && montado.current) setErro(mensagem(falha)); })
      .finally(() => { if (atual && montado.current) setCarregando(false); });
    return () => { atual = false; };
  }, [aberto, caso.id, filtroEstado, motor, deslocamento, recarga]);

  function alterarFiltroEstado(valor: string) {
    setFiltroEstado(valor as EstadoRevisaoEntidade | "todos"); setDeslocamento(0);
  }

  return <section aria-label="Fila de revisão humana" className="painel p-4">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <div><h3 className="text-sm font-semibold text-texto">Fila de revisão humana</h3><p className="mt-1 text-xs text-texto-2">Revise candidatos de entidades sem substituir a inferência automática, o motor ou a origem.</p></div>
      <Botao tamanho="p" variante={aberto ? "fantasma" : "secundario"} onClick={() => { setAberto((valor) => !valor); setErro(null); }}>{aberto ? "Ocultar fila" : "Abrir fila de revisão"}</Botao>
    </div>
    {aberto && <div className="mt-4 space-y-4">
      <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] sm:items-end">
        <Selecao rotulo="Estado humano" value={filtroEstado} onChange={(evento) => alterarFiltroEstado(evento.target.value)}><option value="nao_revisado">Não revisado</option><option value="confirmado">Confirmado</option><option value="inconclusivo">Inconclusivo</option><option value="descartado">Descartado</option><option value="todos">Todos</option></Selecao>
        <label className="campo"><span className="campo__rotulo">Versão do motor</span><input className="campo__controle" value={motor} maxLength={400} placeholder="ex.: entidades-v2" onChange={(evento) => { setMotor(evento.target.value); setDeslocamento(0); }} /></label>
        <Botao tamanho="p" variante="fantasma" onClick={() => setRecarga((valor) => valor + 1)}>Atualizar</Botao>
      </div>
      <p className="text-xs text-texto-3">Cada decisão é uma nova revisão auditável. Confirmado significa apenas ocorrência literal conferida; não é conclusão sobre identidade, autoria ou veracidade.</p>
      {erro && <Aviso>{erro}</Aviso>}
      {carregando ? <Carregando texto="Consultando candidatos conferidos…" /> : pagina === null ? null : pagina.itens.length === 0 ? <Vazio titulo="Nenhum candidato neste filtro">A fila só apresenta entidades derivadas já armazenadas no caso.</Vazio> : <ul className="divide-y divide-borda" aria-label="Candidatos para revisão">{pagina.itens.map((item) => <Candidato key={item.id} caso={caso} item={item} revisado={() => setRecarga((valor) => valor + 1)} />)}</ul>}
      {pagina && <Paginacao total={pagina.total} deslocamento={deslocamento} limite={50} mudar={setDeslocamento} />}
    </div>}
  </section>;
}

function Candidato({ caso, item, revisado }: { caso: Caso; item: CandidatoRevisao; revisado: () => void }) {
  const [estado, setEstado] = useState<EstadoRevisaoEntidade>(item.estado);
  const [nota, setNota] = useState(item.nota ?? "");
  const [salvando, setSalvando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  async function salvar() {
    if (salvando) return;
    setSalvando(true); setErro(null);
    try {
      const resultado = await api.revisarEntidade(caso.id, item.id, estado, nota.trim() || null);
      if (resultado.id !== item.id || resultado.documento_id !== item.documento_id || resultado.estado !== estado) throw new Error("A decisão retornada não corresponde ao candidato selecionado.");
      revisado();
    } catch (falha) { setErro(mensagem(falha)); }
    finally { setSalvando(false); }
  }
  return <li className="space-y-3 py-4 first:pt-1">
    <div className="flex flex-wrap items-start justify-between gap-2"><div><p className="text-sm font-medium text-texto"><span className="font-mono text-texto-3">#{item.id}</span> · {item.tipo} · <span className="font-mono">{item.literal}</span></p><p className="mt-1 break-all font-mono text-xs text-texto-3">{item.caminho} · offsets [{item.inicio_utf8}, {item.fim_utf8})</p></div><Selo tom={tom(item.estado)}>{ESTADOS.find(([estado]) => estado === item.estado)?.[1] ?? item.estado}</Selo></div>
    <div className="grid gap-3 text-xs text-texto-2 sm:grid-cols-2"><div><p><span className="sobrelinha">Motor / versão</span> {item.motor_versao}</p><p className="mt-1"><span className="sobrelinha">Documento</span> #{item.documento_id} · arquivo #{item.arquivo_id}</p></div><div><p><span className="sobrelinha">Origem</span> <Hash valor={item.sha256_origem} /></p><p className="mt-1"><span className="sobrelinha">Texto</span> <Hash valor={item.sha256_texto} /></p></div></div>
    <div className="flex flex-wrap items-end gap-2"><Link className="botao botao-p botao-fantasma" to={`/casos/${encodeURIComponent(caso.id)}/arquivos?extracao=${encodeURIComponent(item.extracao_id)}&arquivo=${item.arquivo_id}`}>Abrir arquivo original</Link><Selecao rotulo="Decisão humana" value={estado} onChange={(evento) => setEstado(evento.target.value as EstadoRevisaoEntidade)}>{ESTADOS.map(([valor, rotulo]) => <option key={valor} value={valor}>{rotulo}</option>)}</Selecao><Botao tamanho="p" variante="secundario" carregando={salvando} onClick={() => void salvar()}>Registrar decisão</Botao></div>
    <AreaTexto rotulo="Nota da revisão (opcional)" maxLength={4000} value={nota} disabled={salvando} onChange={(evento) => setNota(evento.target.value)} />
    {item.revisor && <p className="text-xs text-texto-3">Última decisão por {item.revisor}{item.revista_em ? ` · ${item.revista_em}` : ""}.</p>}
    {erro && <Aviso>{erro}</Aviso>}
  </li>;
}
