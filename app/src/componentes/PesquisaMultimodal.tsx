import { useEffect, useRef, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import type { Caso, FiltroBusca, ItemMultimodal, MecanismoMultimodal, PedidoMultimodal, ResultadoMultimodal } from "../api/cliente";
import { api, FalhaSied, normalizarErro } from "../api/cliente";
import { useExtracoes } from "../api/consultas";
import { Botao } from "./Botao";
import { Aviso, Carregando, Hash, Paginacao, Selo, Vazio } from "./Estrutura";

const LIMITE = 30;
const MECANISMOS: Array<[MecanismoMultimodal, string]> = [
  ["mensagens", "Mensagens"],
  ["ocr", "OCR"],
  ["documentos", "Documentos"],
  ["transcricoes", "Transcrições"],
  ["embeddings", "Embeddings"],
  ["visao", "Visão"],
  ["metadados", "Metadados"],
  ["entidades", "Entidades"],
];

function destino(casoId: string, item: ItemMultimodal) {
  if (item.arquivo_id !== null) {
    return `/casos/${casoId}/arquivos?extracao=${encodeURIComponent(item.extracao_id)}&arquivo=${item.arquivo_id}`;
  }
  return `/casos/${casoId}/extracoes/${encodeURIComponent(item.extracao_id)}`;
}

function erroTexto(erro: unknown) {
  const falha = normalizarErro(erro);
  return falha instanceof FalhaSied ? falha.erro.mensagem : String(erro);
}

function dataHora(valor: string, fuso: string) {
  const data = new Date(valor);
  return Number.isNaN(data.getTime()) ? valor : new Intl.DateTimeFormat("pt-BR", { dateStyle: "short", timeStyle: "medium", timeZone: fuso }).format(data);
}

export function PesquisaMultimodal({ caso, filtro }: { caso: Caso; filtro?: FiltroBusca }) {
  const extracoes = useExtracoes(caso.id);
  const [texto, setTexto] = useState(filtro?.texto ?? "");
  const [extracao, setExtracao] = useState(filtro?.extracao_id ?? "");
  const [mecanismos, setMecanismos] = useState<MecanismoMultimodal[]>(MECANISMOS.map(([id]) => id));
  const [resultado, setResultado] = useState<ResultadoMultimodal | null>(null);
  const [pedido, setPedido] = useState<PedidoMultimodal | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const [erro, setErro] = useState<string | null>(null);
  const [operacao, setOperacao] = useState<string | null>(null);
  const [filtroSinal, setFiltroSinal] = useState<MecanismoMultimodal | "todos">("todos");
  const atual = useRef<string | null>(null);
  const montado = useRef(true);

  useEffect(() => () => {
    montado.current = false;
    const id = atual.current;
    atual.current = null;
    if (id) void api.cancelarDescoberta(id).catch(() => undefined);
  }, []);

  async function executar(novo: PedidoMultimodal, proximo = 0) {
    if (atual.current) return;
    const id = crypto.randomUUID();
    atual.current = id;
    setOperacao(id);
    setErro(null);
    if (proximo === 0) setResultado(null);
    try {
      const recebido = await api.pesquisarMultimodal(caso.id, id, novo, proximo, LIMITE);
      if (!montado.current || atual.current !== id) return;
      const mecanismosRecebidos = recebido.pedido.mecanismos;
      if (recebido.caso_id !== caso.id || recebido.pedido.filtro.texto !== novo.filtro.texto ||
          mecanismosRecebidos.length !== novo.mecanismos.length || mecanismosRecebidos.some((x) => !novo.mecanismos.includes(x)) ||
          recebido.itens.itens.length > LIMITE || recebido.itens.itens.some((x) => !Number.isFinite(x.score_ranking) || x.score_ranking < 0 || x.score_ranking > 1)) {
        throw new Error("A resposta multimodal não corresponde ao caso, filtros ou mecanismos solicitados.");
      }
      setPedido(novo);
      setResultado(recebido);
      setDeslocamento(proximo);
      setFiltroSinal("todos");
    } catch (falha) {
      if (montado.current && atual.current === id) setErro(erroTexto(falha));
    } finally {
      if (montado.current && atual.current === id) {
        atual.current = null;
        setOperacao(null);
      }
    }
  }

  function buscar(evento: FormEvent) {
    evento.preventDefault();
    const termo = texto.trim();
    if (!termo || new TextEncoder().encode(termo).length > 1024) {
      setErro("Informe um termo com até 1.024 bytes UTF-8.");
      return;
    }
    if (mecanismos.length === 0) {
      setErro("Selecione pelo menos um mecanismo.");
      return;
    }
    const proximo: FiltroBusca = { texto: termo };
    if (extracao) proximo.extracao_id = extracao;
    void executar({ filtro: proximo, mecanismos });
  }

  const visiveis = resultado?.itens.itens.filter((item) => filtroSinal === "todos" || item.sinais.some((sinal) => sinal.mecanismo === filtroSinal)) ?? [];
  const ocupado = operacao !== null;

  return (
    <section aria-label="Pesquisa multimodal local" className="space-y-4">
      <div className="painel p-4">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-sm font-semibold text-texto">Pesquisa profunda multimodal</h2>
          <Selo tom="info">LOCAL / OFFLINE</Selo>
        </div>
        <p className="mt-1 text-xs leading-relaxed text-texto-2">Combina índices existentes sem alterar a evidência original. Cada resultado conserva os sinais e o motor que contribuíram para o ranking.</p>
        <form role="search" aria-label="Pesquisa multimodal" onSubmit={buscar} className="mt-3 space-y-3">
          <div className="flex gap-2"><input aria-label="Termo da pesquisa multimodal" type="search" className="entrada min-w-0 flex-1" value={texto} disabled={ocupado} onChange={(e) => setTexto(e.target.value)} /><Botao type="submit" carregando={ocupado}>Pesquisar</Botao></div>
          <fieldset disabled={ocupado} className="space-y-2"><legend className="sobrelinha mb-2">Mecanismos que serão executados</legend><div className="flex flex-wrap gap-x-5 gap-y-2">{MECANISMOS.map(([id, rotulo]) => <label key={id} className="flex items-center gap-2 text-xs text-texto-2"><input type="checkbox" checked={mecanismos.includes(id)} onChange={(e) => setMecanismos((atuais) => e.target.checked ? [...atuais, id] : atuais.filter((x) => x !== id))} />{rotulo}</label>)}</div></fieldset>
          <select aria-label="Extração multimodal" className="entrada" value={extracao} disabled={ocupado} onChange={(e) => setExtracao(e.target.value)}><option value="">Todas as extrações</option>{extracoes.data?.map((x) => <option key={x.id} value={x.id}>{x.importador_nome} · {x.iniciada_em.slice(0, 10)}</option>)}</select>
        </form>
        <p className="mt-3 text-2xs text-texto-3">O ranking RRF organiza candidatos; não é confiança, autoria, causalidade ou conclusão pericial.</p>
      </div>
      {ocupado && <div className="flex flex-wrap items-center gap-2"><Carregando texto="Combinando sinais locais…" /><Botao tamanho="p" variante="secundario" onClick={() => { const id = atual.current; if (id) void api.cancelarDescoberta(id); }}>Cancelar</Botao></div>}
      {erro && <Aviso>{erro}</Aviso>}
      {!resultado && !ocupado && !erro && <Vazio titulo="Escolha os mecanismos e pesquise">Nada é executado ao digitar ou marcar fontes.</Vazio>}
      {resultado && <div className="painel p-4">
        <div className="flex flex-wrap items-center justify-between gap-2"><h3 className="text-sm font-semibold text-texto">Resultados correlacionados</h3><Selo tom={resultado.janela_limitada ? "atencao" : "neutro"}>{resultado.janela_limitada ? "JANELA LIMITADA" : "RESULTADO LOCAL"}</Selo></div>
        <p className="mt-1 text-xs text-texto-2">{resultado.recuperados_na_janela} candidato(s) na união · {dataHora(resultado.executada_em, caso.fuso_horario)}</p>
        {resultado.avisos.map((aviso) => <p key={aviso} className="mt-2 text-xs text-texto-3">{aviso}</p>)}
        <label className="mt-3 flex flex-wrap items-center gap-2 text-xs text-texto-2">Filtrar por sinal<select aria-label="Filtrar mecanismo multimodal" className="entrada" value={filtroSinal} onChange={(e) => setFiltroSinal(e.target.value as MecanismoMultimodal | "todos")}><option value="todos">Todos</option>{MECANISMOS.map(([id, rotulo]) => <option key={id} value={id}>{rotulo}</option>)}</select><span>{visiveis.length} exibido(s)</span></label>
        {visiveis.length === 0 ? <div className="mt-3"><Vazio titulo="Nenhum resultado nesta página">A ausência de candidato não comprova ausência de informação no caso.</Vazio></div> : <ul aria-label="Resultados multimodais" className="mt-3 divide-y divide-borda">{visiveis.map((item) => <li key={item.chave} className="py-3">
          <div className="flex flex-wrap items-start justify-between gap-2"><Link className="min-w-0 flex-1 break-all text-sm font-semibold text-texto hover:text-ouro-claro" to={destino(caso.id, item)}>{item.titulo}</Link><span className="font-mono text-2xs text-texto-3">RRF {item.score_ranking.toFixed(4)} · não é confiança</span></div>
          <p className="mt-1 whitespace-pre-wrap break-words text-sm text-texto-2">{item.trecho || "Sem trecho textual; consulte a origem."}</p>
          <p className="mt-1 font-mono text-2xs text-texto-3">{item.origem_tipo} · extração {item.extracao_id}{item.inicio_ms === null ? "" : ` · ${item.inicio_ms}–${item.fim_ms ?? item.inicio_ms} ms`}</p>
          <details className="mt-2 text-xs text-texto-2"><summary className="cursor-pointer text-ouro-claro">Por que este resultado apareceu?</summary><p className="mt-2">Origem conferida: <Hash valor={item.hash_origem} /></p><ol className="mt-2 space-y-2">{item.sinais.map((sinal, indice) => <li key={`${sinal.mecanismo}-${indice}`} className="border-l border-borda pl-3"><p><Selo tom="info">{MECANISMOS.find(([id]) => id === sinal.mecanismo)?.[1] ?? sinal.mecanismo}</Selo> · posição {sinal.posicao} · contribuição {sinal.contribuicao_rrf.toFixed(6)}</p><p className="mt-1">{sinal.motivo}</p><p className="mt-1 font-mono text-2xs text-texto-3">Motor: {sinal.motor} · versão {sinal.motor_versao}</p>{sinal.hash_derivado && <p className="mt-1">Hash do derivado: <Hash valor={sinal.hash_derivado} /></p>}</li>)}</ol><p className="mt-2 text-texto-3">Sinais derivados permanecem separados da evidência original e são apenas candidatos para revisão humana.</p></details>
        </li>)}</ul>}
        {pedido && !ocupado && <Paginacao total={resultado.recuperados_na_janela} deslocamento={deslocamento} limite={LIMITE} mudar={(proximo) => void executar(pedido, proximo)} />}
      </div>}
    </section>
  );
}
