import { useEffect, useRef, useState } from "react";
import { api, type Caso } from "../api/cliente";
import type { ItemBuscaHibrida, ResultadoBuscaHibrida } from "../api/embeddings";
import { Botao } from "./Botao";
import { Aviso, Carregando, Cartao, Paginacao, Selo, Vazio } from "./Estrutura";

const LIMITE = 25;

export function BuscaHibridaLocal({ caso }: { caso: Caso }) {
  const [texto, setTexto] = useState("");
  const [peso, setPeso] = useState(0.6);
  const [resultado, setResultado] = useState<ResultadoBuscaHibrida | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const [estado, setEstado] = useState<string | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const operacao = useRef<string | null>(null);
  const montado = useRef(true);

  useEffect(() => () => {
    montado.current = false;
    if (operacao.current) void api.cancelarDescoberta(operacao.current).catch(() => undefined);
  }, []);

  async function indexar() {
    const id = crypto.randomUUID();
    operacao.current = id; setErro(null); setEstado("Preparando índice local…");
    try {
      const r = await api.indexarEmbeddings(caso.id, id);
      if (montado.current) setEstado(`${r.criados} vetores novos · ${r.reutilizados} reutilizados · ${r.motor_id}/${r.motor_versao}`);
    } catch (e) { if (montado.current) { setEstado(null); setErro(e instanceof Error ? e.message : String(e)); } }
    finally { operacao.current = null; }
  }

  async function buscar(pagina = 0) {
    if (!texto.trim()) return;
    const id = crypto.randomUUID();
    operacao.current = id; setErro(null); setEstado("Combinando busca textual e similaridade local…"); setDeslocamento(pagina);
    try {
      const r = await api.buscarHibrida(caso.id, id, texto.trim(), pagina, LIMITE, peso);
      if (montado.current) setResultado(r);
    } catch (e) { if (montado.current) setErro(e instanceof Error ? e.message : String(e)); }
    finally { if (montado.current) setEstado(null); operacao.current = null; }
  }

  function cancelar() {
    if (operacao.current) void api.cancelarDescoberta(operacao.current);
  }

  return <section className="space-y-4" aria-label="Busca híbrida local">
    <Cartao titulo="Pesquisa híbrida local" icone="busca">
      <div className="space-y-3">
        <p className="text-sm text-texto-2">Combina correspondência textual com um embedding local determinístico. O índice é derivado e permanece no banco do caso.</p>
        <form className="flex flex-wrap gap-2" onSubmit={(e) => { e.preventDefault(); void buscar(0); }}>
          <input className="entrada min-w-[18rem] flex-1" value={texto} maxLength={1024} onChange={(e) => setTexto(e.target.value)} placeholder="Tema, nome, telefone ou frase" aria-label="Consulta híbrida" />
          <Botao type="submit" disabled={!texto.trim() || !!operacao.current} icone="busca">Buscar</Botao>
        </form>
        <div className="flex flex-wrap items-center gap-3 text-xs text-texto-2">
          <label className="flex items-center gap-2">Peso semântico <input aria-label="Peso semântico" type="range" min="0" max="1" step="0.1" value={peso} onChange={(e) => setPeso(Number(e.target.value))} /> <span className="font-mono">{Math.round(peso * 100)}%</span></label>
          <Botao tamanho="p" variante="secundario" disabled={!!operacao.current} onClick={() => void indexar()}>Indexar conteúdo local</Botao>
          {operacao.current && <Botao tamanho="p" variante="fantasma" onClick={cancelar}>Cancelar</Botao>}
        </div>
        {estado && <Carregando texto={estado} />}
        {erro && <Aviso>{erro}</Aviso>}
      </div>
    </Cartao>
    {resultado ? <Cartao titulo={`Candidatos (${resultado.itens.total})`} denso>
      <div className="border-b border-borda px-4 py-3 text-xs text-texto-2"><span className="font-mono">{resultado.motor_id}/{resultado.motor_versao}</span> · peso semântico {Math.round(resultado.peso_semantico * 100)}% · scores são ranking, não confiança.</div>
      {resultado.avisos.map((aviso) => <p key={aviso} className="px-4 pt-2 text-xs text-atencao-claro">{aviso}</p>)}
      {resultado.itens.itens.length === 0 ? <div className="p-4"><Vazio titulo="Nenhum candidato encontrado">Indexe novamente após uma importação ou ajuste a consulta.</Vazio></div> : <ul className="divide-y divide-borda" aria-label="Candidatos híbridos">
        {resultado.itens.itens.map((item) => <Resultado key={`${item.origem_tipo}:${item.origem_id}`} item={item} />)}
      </ul>}
      <div className="px-4 py-3"><Paginacao total={resultado.itens.total} deslocamento={deslocamento} limite={LIMITE} mudar={(p) => void buscar(p)} /></div>
    </Cartao> : <Vazio titulo="Execute uma pesquisa híbrida" icone="busca">Indexe os textos do caso uma vez e veja, em cada item, quais sinais textual e semântico contribuíram para a posição.</Vazio>}
  </section>;
}

function Resultado({ item }: { item: ItemBuscaHibrida }) {
  const rotulo = item.origem_tipo === "artefato" ? "Registro" : item.origem_tipo === "documento" ? "Documento derivado" : "Transcrição";
  return <li className="px-4 py-3" data-teste="candidato-hibrido">
    <div className="flex flex-wrap items-center gap-2"><Selo tom="info">{rotulo}</Selo><span className="font-semibold text-texto">{item.titulo}</span><span className="ml-auto font-mono text-2xs text-texto-3">ranking {item.score_hibrido.toFixed(4)}</span></div>
    <p className="mt-1 whitespace-pre-wrap break-words text-sm text-texto-2">{item.trecho}</p>
    <details className="mt-2 text-xs text-texto-2"><summary className="cursor-pointer text-ouro-claro">Por que este resultado apareceu?</summary><ul className="mt-2 space-y-2">{item.sinais.map((s) => <li key={`${s.mecanismo}:${s.posicao ?? "sem-posicao"}`} className="border-l border-borda pl-3"><p><strong>{s.mecanismo === "textual" ? "Correspondência textual" : "Similaridade semântica local"}</strong> · contribuição {s.contribuicao.toFixed(5)}</p><p>{s.motivo}</p><p className="font-mono text-2xs text-texto-3">{s.motor_id}/{s.motor_versao}{s.posicao == null ? "" : ` · posição ${s.posicao}`}</p></li>)}</ul><p className="mt-2 text-texto-3">Origem preservada: {item.origem_tipo} #{item.origem_id} · extração {item.extracao_id}. A relação é um candidato derivado para revisão humana.</p></details>
  </li>;
}
