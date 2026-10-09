import { useEffect, useRef, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import type { Caso, FiltroBusca, ItemMultimodal, MecanismoMultimodal, ResultadoMultimodal } from "../api/cliente";
import { api, normalizarErro } from "../api/cliente";
import { Botao } from "./Botao";
import { ContextoRelacional } from "./ContextoRelacional";
import { Aviso, Carregando, Hash, Selo, Vazio } from "./Estrutura";

const LIMITE = 20;
const FONTES: Array<[MecanismoMultimodal, string]> = [
  ["mensagens", "Mensagens"], ["ocr", "OCR"], ["documentos", "Documentos"],
  ["transcricoes", "Transcrições"], ["embeddings", "Semântica"], ["visao", "Imagens"],
  ["metadados", "Metadados"], ["entidades", "Entidades"],
];
const STOPWORDS = new Set("a ao aos as com como da das de do dos e em este esta estes estas eu encontre encontrar existem existe mostrar mostre para por quais qual que relacionado relacionadas relacionados sobre tema um uma os no nos na nas deste quero são sao haja há ha me isso veja mencionam menciona contendo ligados ligadas eventos próximos proximos horário horario timeline documentos mensagens conversas imagens imagem arquivos audio áudio vídeo video".split(" "));

export type ConsultaAssistente = { termo: string; mecanismos: MecanismoMultimodal[]; interpretacao: string };

/** Interpretação determinística; não usa LLM, rede ou inferência de identidade. */
export function interpretarConsultaAssistente(pergunta: string): ConsultaAssistente {
  const original = pergunta.trim();
  if (!original) throw new Error("Escreva uma pergunta sobre os dados do caso.");
  const normalizada = original.toLocaleLowerCase("pt-BR");
  const mecanismos = new Set<MecanismoMultimodal>();
  if (/mensag|convers|telefone|chamada|contato/.test(normalizada)) mecanismos.add("mensagens");
  if (/ocr|texto\s+(?:dentro|em)|imagem.*texto/.test(normalizada)) mecanismos.add("ocr");
  if (/document|pdf|arquivo/.test(normalizada)) mecanismos.add("documentos");
  if (/transcri|áudio|audio|vídeo|video/.test(normalizada)) mecanismos.add("transcricoes");
  if (/semânt|semantic|tema|relacionad|associad|similar/.test(normalizada)) mecanismos.add("embeddings");
  if (/imagem|foto|visual|objeto/.test(normalizada)) mecanismos.add("visao");
  if (/metadado|caminho|localiza|domínio|dominio|ip\b/.test(normalizada)) mecanismos.add("metadados");
  if (/entidad|imei|e-mail|email|telefone|domínio|dominio|ip\b/.test(normalizada)) mecanismos.add("entidades");
  if (mecanismos.size === 0) FONTES.forEach(([id]) => mecanismos.add(id));
  const citado = original.match(/["“]([^"”]+)["”]/)?.[1]?.trim();
  if (citado && /^(?:\+?\d[\d\s().-]{5,})$/.test(citado)) mecanismos.add("entidades");
  const tokens = normalizada.normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/[^\p{L}\p{N}@._:+-]+/gu, " ").split(/\s+/).filter((token) => token.length >= 3 && !STOPWORDS.has(token));
  const termo = citado || tokens.slice(0, 8).join(" ");
  if (!termo) throw new Error("Não consegui extrair um termo pesquisável. Use aspas para indicar o texto exato.");
  const nomes = FONTES.filter(([id]) => mecanismos.has(id)).map(([, nome]) => nome).join(", ");
  return { termo, mecanismos: [...mecanismos], interpretacao: `Regra local identificou “${termo}” e selecionou: ${nomes}. Confira as fontes antes de executar.` };
}

function destino(casoId: string, item: ItemMultimodal) {
  if (item.arquivo_id !== null) return `/casos/${casoId}/arquivos?extracao=${encodeURIComponent(item.extracao_id)}&arquivo=${item.arquivo_id}`;
  return `/casos/${casoId}/extracoes/${encodeURIComponent(item.extracao_id)}`;
}

function mensagemErro(erro: unknown) { return normalizarErro(erro).erro.mensagem; }

export function AssistenteLocal({ caso, filtroInicial }: { caso: Caso; filtroInicial?: FiltroBusca }) {
  const [pergunta, setPergunta] = useState(filtroInicial?.texto ?? "");
  const [interpretacao, setInterpretacao] = useState<ConsultaAssistente | null>(null);
  const [resultado, setResultado] = useState<ResultadoMultimodal | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [operacao, setOperacao] = useState<string | null>(null);
  const atual = useRef<string | null>(null);
  const montado = useRef(true);
  useEffect(() => () => { montado.current = false; const id = atual.current; atual.current = null; if (id) void api.cancelarDescoberta(id).catch(() => undefined); }, []);

  async function perguntar(evento: FormEvent) {
    evento.preventDefault();
    let consulta: ConsultaAssistente;
    try { consulta = interpretarConsultaAssistente(pergunta); } catch (falha) { setErro(mensagemErro(falha)); return; }
    if (atual.current) return;
    const id = crypto.randomUUID(); atual.current = id; setOperacao(id); setErro(null); setResultado(null); setInterpretacao(consulta);
    const filtro: FiltroBusca = { texto: consulta.termo };
    if (filtroInicial?.extracao_id) filtro.extracao_id = filtroInicial.extracao_id;
    try {
      const recebido = await api.pesquisarMultimodal(caso.id, id, { filtro, mecanismos: consulta.mecanismos }, 0, LIMITE);
      if (!montado.current || atual.current !== id) return;
      if (recebido.caso_id !== caso.id || recebido.pedido.filtro.texto !== consulta.termo || recebido.itens.itens.length > LIMITE) throw new Error("A resposta não corresponde ao caso ou à consulta interpretada.");
      setResultado(recebido);
    } catch (falha) { if (montado.current && atual.current === id) setErro(mensagemErro(falha)); }
    finally { if (montado.current && atual.current === id) { atual.current = null; setOperacao(null); } }
  }
  const ocupado = operacao !== null;
  return <section aria-label="Assistente local explicável" className="space-y-4">
    <div className="painel p-4"><div className="flex flex-wrap items-center justify-between gap-2"><div><h2 className="text-sm font-semibold text-texto">Assistente local explicável</h2><p className="mt-1 text-xs text-texto-2">Faça uma pergunta sobre este caso. A consulta é interpretada por regras locais e executa somente índices já existentes.</p></div><Selo tom="info">SEM REDE</Selo></div>
      <form onSubmit={perguntar} className="mt-3 space-y-3"><div className="flex gap-2"><input aria-label="Pergunta ao assistente local" className="entrada min-w-0 flex-1" value={pergunta} disabled={ocupado} onChange={(e) => setPergunta(e.target.value)} placeholder="Ex.: quais documentos mencionam este telefone?" /><Botao type="submit" carregando={ocupado}>Consultar caso</Botao></div><p className="text-2xs text-texto-3">Use aspas para um termo exato. A interpretação é exibida antes da resposta; o assistente não executa ações, não acessa outros casos e não envia dados.</p></form>
      {interpretacao && <div role="status" className="mt-3 rounded-md border border-borda bg-fundo p-3 text-xs text-texto-2"><p className="font-semibold text-texto">Consulta interpretada</p><p className="mt-1">{interpretacao.interpretacao}</p><p className="mt-1 font-mono text-2xs text-texto-3">Termo recuperado: {interpretacao.termo}</p></div>}
    </div>
    {ocupado && <div className="flex items-center gap-2"><Carregando texto="Recuperando suporte local…" /><Botao tamanho="p" variante="secundario" onClick={() => { const id = atual.current; if (id) void api.cancelarDescoberta(id); }}>Cancelar</Botao></div>}
    {erro && <Aviso>{erro}</Aviso>}
    {resultado && <div className="painel p-4"><div className="flex flex-wrap items-center justify-between gap-2"><h3 className="text-sm font-semibold text-texto">Resposta verificável sobre o caso</h3><Selo tom={resultado.itens.total > 0 ? "info" : "atencao"}>{resultado.itens.total > 0 ? "SUPORTE ENCONTRADO" : "SEM SUPORTE SUFICIENTE"}</Selo></div><p className="mt-2 text-sm text-texto-2">{resultado.itens.total > 0 ? `Foram encontrados ${resultado.itens.total} candidato(s) nos índices locais selecionados.` : "Não encontrei suporte suficiente nos índices selecionados para responder esta pergunta."}</p><details className="mt-3 border-t border-borda pt-3"><summary className="cursor-pointer text-xs text-ouro-claro">Por que esta resposta apareceu?</summary><p className="mt-2 text-xs text-texto-2">A resposta é uma recuperação de candidatos, não uma conclusão pericial. Foram usados: {resultado.pedido.mecanismos.join(", ")}. O ranking preserva os sinais individuais de cada mecanismo.</p></details>{resultado.itens.total === 0 ? <Vazio titulo="Nenhum artefato de suporte">Tente um termo literal entre aspas ou selecione uma pesquisa multimodal mais específica.</Vazio> : <ul aria-label="Fontes da resposta do assistente" className="mt-3 divide-y divide-borda">{resultado.itens.itens.map((item) => <li key={item.chave} className="py-3"><div className="flex flex-wrap items-start justify-between gap-2"><Link className="text-sm font-semibold text-texto hover:text-ouro-claro" to={destino(caso.id, item)}>{item.titulo}</Link><span className="font-mono text-2xs text-texto-3">RRF {item.score_ranking.toFixed(4)} · não é confiança</span></div><p className="mt-1 whitespace-pre-wrap text-sm text-texto-2">{item.trecho || "Sem trecho; abra a origem."}</p><details className="mt-2 text-xs text-texto-2"><summary className="cursor-pointer text-ouro-claro">Ver fontes e sinais</summary><p className="mt-2">SHA-256 da origem: <Hash valor={item.hash_origem} /></p><ul className="mt-2 space-y-2">{item.sinais.map((sinal, indice) => <li key={`${sinal.mecanismo}-${indice}`} className="border-l border-borda pl-3"><p><Selo tom="info">{sinal.mecanismo}</Selo> · {sinal.motivo}</p><p className="mt-1 font-mono text-2xs text-texto-3">Motor {sinal.motor} · versão {sinal.motor_versao} · contribuição {sinal.contribuicao_rrf.toFixed(6)}</p></li>)}</ul></details>{item.origem_tipo === "mensagem" && <details className="mt-2"><summary className="cursor-pointer text-xs text-ouro-claro">Explorar timeline e relações deste registro</summary><ContextoRelacional caso={caso} artefatoId={item.origem_id} /></details>}</li>)}</ul>}</div>}
  </section>;
}
