// Inventário de arquivos de uma extração: tabela (Arquivos, aba da extração) e
// grade (Mídias), com painel de detalhes. Os dados vêm do inventário gravado na
// importação. As miniaturas das imagens são geradas pelo backend a partir do arquivo
// da extração, só depois de conferir o SHA-256 com o inventário.

import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { useSearchParams } from "react-router-dom";
import { api, type ArquivoExtracao, type Caso, type Extracao } from "../api/cliente";
import { chaves, useArquivos, useDispositivos, useExecucoes, useExtracoes, useTotaisPorCategoria } from "../api/consultas";
import { useListaPaginada } from "../util/listaPaginada";
import { dataHora, extensao, nomeArquivo, numero, tamanho } from "../util/formatar";
import { nomeCategoria } from "../util/rotulos";
import { Aviso, Cartao, Carregando, entrada, Filtros, Hash, Paginacao, Selo, tabela, Vazio } from "./Estrutura";
import { EvidenciaArquivo } from "./Evidencias";
import { Icone, type NomeIcone } from "./Icone";
import { MiniaturaArquivo, SobreMiniatura } from "./Miniatura";
import { LeitorDocumento } from "./LeitorDocumento";
import { AnaliseVisual } from "./AnaliseVisual";

export const iconeCategoria: Record<string, NomeIcone> = {
  banco_sqlite: "banco",
  imagem: "midias",
  video: "video",
  audio: "audio",
  documento: "documento",
  compactado: "compactado",
  texto: "arquivos",
  vazio: "arquivos",
  outro: "arquivos",
};

/** Escolha da extração (Mídias e Arquivos trabalham uma extração por vez). */
export function SeletorExtracao({ caso, valor, mudar }: {
  caso: Caso;
  valor: string | null;
  mudar: (id: string) => void;
}) {
  const extracoes = useExtracoes(caso.id);
  const dispositivos = useDispositivos(caso.id);
  const apelido = (x: Extracao) => dispositivos.data?.find((d) => d.id === x.dispositivo_id)?.apelido ?? "dispositivo";
  if (!extracoes.data || extracoes.data.length === 0) return null;
  return (
    <label className="flex items-center gap-2 text-xs text-texto-2">
      Extração
      <select aria-label="Extração" value={valor ?? ""} onChange={(e) => mudar(e.target.value)} className={`${entrada} max-w-sm`}>
        {extracoes.data.map((x) => (
          <option key={x.id} value={x.id}>
            {apelido(x)} · {x.importador_nome} · {dataHora(x.iniciada_em, caso.fuso_horario)}
          </option>
        ))}
      </select>
    </label>
  );
}

/** Extração escolhida (a primeira da lista, se nenhuma foi escolhida) e o estado das extrações. */
export function useExtracaoEscolhida(caso: Caso) {
  const extracoes = useExtracoes(caso.id);
  const [params] = useSearchParams();
  const [escolhida, setEscolhida] = useState<string | null>(null);
  const solicitada=params.get("extracao");
  const id = escolhida ?? (extracoes.data?.some(e=>e.id===solicitada)?solicitada:null) ?? extracoes.data?.[0]?.id ?? null;
  return { extracoes, id, setEscolhida };
}

function DetalheArquivo({ caso, extracaoId, f, parser, fechar, documental, paginaSolicitada }: {
  caso: Caso;
  extracaoId: string;
  f: ArquivoExtracao;
  parser: string | undefined;
  fechar: () => void;
  documental: boolean;
  paginaSolicitada?: number;
}) {
  const fuso = caso.fuso_horario;
  const linha = (rotulo: string, valor: ReactNode, mono = false) => (
    <div className="grid grid-cols-[6.5rem_minmax(0,1fr)] gap-2 py-1 text-xs">
      <dt className="text-texto-3">{rotulo}</dt>
      <dd className={`break-all text-texto ${mono ? "font-mono" : ""}`}>{valor ?? "—"}</dd>
    </div>
  );
  return (
    <Cartao
      titulo="Detalhes do arquivo"
      icone={iconeCategoria[f.categoria] ?? "arquivos"}
      acoes={
        <button type="button" onClick={fechar} aria-label="Fechar detalhes" className="rounded p-1 text-texto-3 hover:bg-sup-2 hover:text-texto">
          <Icone nome="fechar" className="size-4" />
        </button>
      }
    >
      {(f.categoria === "imagem" || f.categoria === "video" || f.categoria === "audio") && (
        <div className="mb-3">
          <MiniaturaArquivo
            caso={caso}
            arquivoId={f.id}
            categoria={f.categoria}
            icone={iconeCategoria[f.categoria] ?? "arquivos"}
            ajuste="conter"
            className="aspect-[4/3] w-full rounded-md border border-borda"
          />
          <p className="mt-1.5 text-2xs leading-snug text-texto-3">
            <SobreMiniatura caso={caso} arquivoId={f.id} categoria={f.categoria} />
          </p>
        </div>
      )}
      <p className="mb-2 break-all font-medium text-texto">{nomeArquivo(f.caminho)}</p>
      <dl aria-label="Metadados do arquivo">
        {linha("Caminho", f.caminho, true)}
        {linha("Categoria", nomeCategoria[f.categoria] ?? f.categoria)}
        {linha("Extensão", extensao(f.caminho) || "—", true)}
        {linha("Assinatura", f.assinatura, true)}
        {linha("MIME", f.mime, true)}
        {linha("Tamanho", `${tamanho(f.tamanho)} (${numero(f.tamanho)} bytes)`)}
        {linha("Modificado", dataHora(f.modificado_em, fuso))}
        {linha("Criado", dataHora(f.criado_em, fuso))}
        {linha("SHA-256", <Hash valor={f.sha256} />)}
        {linha("Interpretado por", parser ?? "nenhum parser")}
        {f.erro && linha("Erro de leitura", <span className="text-erro">{f.erro}</span>)}
      </dl>
      <EvidenciaArquivo caso={caso} arquivoId={f.id} sha256={f.sha256} />
      {documental && <LeitorDocumento caso={caso} extracaoId={extracaoId} arquivo={f}
        paginaInicial={paginaSolicitada !== undefined && Number.isSafeInteger(paginaSolicitada) && paginaSolicitada >= 1 && paginaSolicitada <= 512 ? paginaSolicitada : undefined} />}
      {f.categoria === "imagem" && <AnaliseVisual caso={caso} extracaoId={extracaoId} arquivo={f} />}
    </Cartao>
  );
}

const LIMITE = 100;

/**
 * Inventário de uma extração. `midias` = grade com os filtros da galeria;
 * senão, tabela com todas as categorias.
 */
export function InventarioArquivos({ caso, extracaoId, midias }: { caso: Caso; extracaoId: string; midias?: boolean }) {
  const [params]=useSearchParams();
  const arquivoSolicitado=Number(params.get("arquivo"));
  const paginaSolicitada=Number(params.get("pagina"));
  const [erroOrigem,setErroOrigem]=useState<string|null>(null);
  const [categoria, setCategoria] = useState<string | null>(midias ? "midia" : null);
  const [texto, setTexto] = useState("");
  const [busca, setBusca] = useState<string | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const [selecao, setSelecao] = useState<{ casoId: string; extracaoId: string; arquivo: ArquivoExtracao } | null>(null);
  // Oculta a seleção anterior já no primeiro render de outro contexto, antes do
  // efeito de limpeza: um ID numérico não identifica o arquivo entre casos.
  const selecionado = selecao?.casoId === caso.id && selecao.extracaoId === extracaoId ? selecao.arquivo : null;
  const setSelecionado = (arquivo: ArquivoExtracao | null) => {
    setSelecao(arquivo ? { casoId: caso.id, extracaoId, arquivo } : null);
  };
  useEffect(() => setSelecao(null), [caso.id, extracaoId]);
  useEffect(()=>{
    let atual=true;setErroOrigem(null);
    if(params.get("extracao")!==extracaoId||!Number.isSafeInteger(arquivoSolicitado)||arquivoSolicitado<=0)return;
    void api.obterArquivo(caso.id,extracaoId,arquivoSolicitado).then(a=>{
      if(atual){if(a.id!==arquivoSolicitado)throw Error("O arquivo recebido não corresponde ao solicitado.");setSelecao({casoId:caso.id,extracaoId,arquivo:a});}
    }).catch(e=>{if(atual)setErroOrigem(e instanceof Error?e.message:"Não foi possível abrir o arquivo original.");});
    return()=>{atual=false;};
  },[caso.id,extracaoId,arquivoSolicitado,params]);
  const totais = useTotaisPorCategoria(extracaoId);
  const execucoes = useExecucoes(extracaoId);
  const arquivos = useArquivos(extracaoId, { categoria, texto: busca }, deslocamento, LIMITE);

  // Arquivo → parser que o leu (só bancos que algum parser interpretou).
  const parserDe = (caminho: string) => {
    const e = execucoes.data?.find((x) => x.arquivo_caminho === caminho);
    return e ? `${e.parser_nome} v${e.parser_versao}` : undefined;
  };

  function filtrar(c: string | null) {
    setCategoria(c);
    setDeslocamento(0);
    setSelecionado(null);
  }

  const qtd = (c: string) => totais.data?.find((t) => t.categoria === c)?.arquivos ?? 0;
  const opcoes: [string | null, ReactNode][] = midias
    ? [
        ["midia", `Todas (${numero(qtd("imagem") + qtd("video") + qtd("audio") + qtd("documento"))})`],
        ["imagem", `Imagens (${numero(qtd("imagem"))})`],
        ["video", `Vídeos (${numero(qtd("video"))})`],
        ["audio", `Áudios (${numero(qtd("audio"))})`],
        ["documento", `Documentos (${numero(qtd("documento"))})`],
        ["outro", `Outros (${numero(qtd("outro"))})`],
      ]
    : [
        [null, `Todos (${numero(totais.data?.reduce((s, t) => s + t.arquivos, 0) ?? 0)})`],
        ...(totais.data ?? []).map((t): [string, string] => [t.categoria, `${nomeCategoria[t.categoria] ?? t.categoria} (${numero(t.arquivos)})`]),
      ];

  return (
    <div className={`workbench-forense grid gap-4 ${selecionado ? "xl:grid-cols-[minmax(0,1fr)_24rem]" : ""}`}>
      {erroOrigem&&<Aviso>{erroOrigem}</Aviso>}
      <Cartao denso>
        <div className="workbench-forense__ferramentas flex flex-wrap items-center gap-3 border-b border-borda px-3 py-2.5">
          <Filtros rotulo="Filtrar por tipo" opcoes={opcoes} atual={categoria} mudar={filtrar} />
          <form
            className="ml-auto"
            onSubmit={(e) => {
              e.preventDefault();
              setBusca(texto.trim() || null);
              setDeslocamento(0);
            }}
          >
            <input
              type="search"
              value={texto}
              onChange={(e) => setTexto(e.target.value)}
              placeholder="Caminho, extensão (.db) ou SHA-256"
              aria-label="Buscar arquivos"
              className={`${entrada} w-72`}
            />
          </form>
        </div>
        {arquivos.isPending || arquivos.isPlaceholderData ? (
          <div className="px-4">
            <Carregando />
          </div>
        ) : arquivos.isError ? (
          <div className="p-4">
            <Aviso>{arquivos.error.message}</Aviso>
          </div>
        ) : arquivos.data.total === 0 ? (
          <div className="p-4">
            <Vazio titulo={midias ? "Nenhuma mídia nesta seleção" : "Nenhum arquivo encontrado"} icone={midias ? "midias" : "arquivos"} />
          </div>
        ) : midias && arquivos.data.total > LIMITE ? (
          <GradeVirtual
            caso={caso}
            extracaoId={extracaoId}
            filtro={{ categoria, texto: busca }}
            primeira={arquivos.data}
            selecionado={selecionado}
            selecionar={setSelecionado}
          />
        ) : midias ? (
          <ul className="galeria-forense grid grid-cols-[repeat(auto-fill,minmax(9.5rem,1fr))] gap-3 p-3" aria-label="Mídias">
            {arquivos.data.itens.map((f) => (
              <li key={f.id}>
                <CartaoMidia caso={caso} f={f} ativo={selecionado?.id === f.id} selecionar={setSelecionado} />
              </li>
            ))}
          </ul>
        ) : (
          <div className="overflow-x-auto">
            <table className={tabela.base} aria-label="Inventário de arquivos">
              <thead className={tabela.cabeca}>
                <tr>
                  <th className={tabela.th}>Caminho</th>
                  <th className={tabela.th}>Ext.</th>
                  <th className={tabela.th}>Tipo</th>
                  <th className={`${tabela.th} text-right`}>Tamanho</th>
                  <th className={tabela.th}>Modificado</th>
                  <th className={tabela.th}>SHA-256</th>
                  <th className={tabela.th}>Parser</th>
                </tr>
              </thead>
              <tbody className={tabela.corpo}>
                {arquivos.data.itens.map((f) => (
                  <tr
                    key={f.id}
                    tabIndex={0}
                    aria-selected={selecionado?.id === f.id}
                    onClick={() => setSelecionado(f)}
                    onKeyDown={(e) => e.key === "Enter" && setSelecionado(f)}
                    className={`${tabela.linha} cursor-pointer ${selecionado?.id === f.id ? "bg-ouro/8" : ""}`}
                  >
                    <td className={`${tabela.td} max-w-md break-all font-mono text-xs text-texto`}>{f.caminho}</td>
                    <td className={`${tabela.td} font-mono text-xs text-texto-2`}>{extensao(f.caminho) || "—"}</td>
                    <td className={`${tabela.td} whitespace-nowrap text-xs`}>
                      {nomeCategoria[f.categoria] ?? f.categoria}
                      {f.assinatura && <span className="text-texto-3"> · {f.assinatura}</span>}
                    </td>
                    <td className={`${tabela.td} tabular whitespace-nowrap text-right text-xs`}>{tamanho(f.tamanho)}</td>
                    <td className={`${tabela.td} tabular whitespace-nowrap text-xs text-texto-2`}>{dataHora(f.modificado_em, caso.fuso_horario)}</td>
                    <td className={`${tabela.td} font-mono text-xs`} title={f.sha256 ?? f.erro ?? undefined}>
                      {f.sha256 ? <span className="text-texto-2">{f.sha256.slice(0, 16)}…</span> : <Selo tom="erro">ilegível</Selo>}
                    </td>
                    <td className={`${tabela.td} whitespace-nowrap text-xs text-texto-2`}>{parserDe(f.caminho) ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {!midias && arquivos.data && arquivos.data.total > LIMITE && (
          <div className="border-t border-borda px-3">
            <Paginacao total={arquivos.data.total} deslocamento={deslocamento} limite={LIMITE} mudar={setDeslocamento} />
          </div>
        )}
      </Cartao>
      {selecionado && (
        <DetalheArquivo caso={caso} extracaoId={extracaoId} f={selecionado} parser={parserDe(selecionado.caminho)} fechar={() => setSelecionado(null)} documental={!midias} paginaSolicitada={paginaSolicitada} />
      )}
    </div>
  );
}

/** Um item da grade de mídias: miniatura (ou ícone do tipo), nome e tamanho. */
function CartaoMidia({ caso, f, ativo, selecionar }: {
  caso: Caso;
  f: ArquivoExtracao;
  ativo: boolean;
  selecionar: (f: ArquivoExtracao) => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={ativo}
      onClick={() => selecionar(f)}
      title={f.caminho}
      className={`midia-forense flex w-full flex-col overflow-hidden rounded-lg border text-left transition-colors ${
        ativo ? "border-ouro/70 bg-ouro/5" : "border-borda bg-sup-2/60 hover:border-borda-2"
      }`}
    >
      <MiniaturaArquivo
        caso={caso}
        arquivoId={f.id}
        categoria={f.categoria}
        icone={iconeCategoria[f.categoria] ?? "arquivos"}
        className="aspect-[4/3] w-full"
      />
      <span className="block w-full px-2 py-1.5">
        <span className="block truncate text-xs text-texto">{nomeArquivo(f.caminho)}</span>
        <span className="tabular block text-2xs text-texto-3">
          {nomeCategoria[f.categoria] ?? f.categoria} · {tamanho(f.tamanho)}
        </span>
      </span>
    </button>
  );
}

/** Largura mínima de um item da grade e espaço entre eles (px), como na grade simples. */
const LARGURA_ITEM = 152;
const ESPACO = 12;
const LEGENDA = 44;

/**
 * Grade de mídias longa: só as linhas perto da área visível existem na tela, e as
 * páginas do inventário são pedidas ao banco conforme a rolagem.
 */
function GradeVirtual({ caso, extracaoId, filtro, primeira, selecionado, selecionar }: {
  caso: Caso;
  extracaoId: string;
  filtro: { categoria: string | null; texto: string | null };
  primeira: { total: number; itens: ArquivoExtracao[] };
  selecionado: ArquivoExtracao | null;
  selecionar: (f: ArquivoExtracao) => void;
}) {
  const rolagem = useRef<HTMLDivElement>(null);
  const [largura, setLargura] = useState(0);
  useEffect(() => {
    const el = rolagem.current;
    if (!el) return;
    setLargura(el.clientWidth || el.offsetWidth);
    if (typeof ResizeObserver === "undefined") return;
    const o = new ResizeObserver(() => setLargura(el.clientWidth || el.offsetWidth));
    o.observe(el);
    return () => o.disconnect();
  }, []);
  const util = Math.max(0, largura - 2 * ESPACO);
  const colunas = Math.max(1, Math.floor((util + ESPACO) / (LARGURA_ITEM + ESPACO)));
  const larguraItem = colunas > 0 ? (util - ESPACO * (colunas - 1)) / colunas : LARGURA_ITEM;
  const alturaLinha = Math.round(larguraItem * 0.75 + LEGENDA + ESPACO);
  const linhas = Math.ceil(primeira.total / colunas);

  const virtual = useVirtualizer({
    count: linhas,
    getScrollElement: () => rolagem.current,
    estimateSize: () => alturaLinha,
    overscan: 3,
  });
  useEffect(() => virtual.measure(), [alturaLinha, virtual]);

  const linhasVisiveis = virtual.getVirtualItems();
  const indices = linhasVisiveis.flatMap((l) => Array.from({ length: colunas }, (_, k) => l.index * colunas + k));
  const { item } = useListaPaginada({
    chave: [...chaves.extracao(extracaoId), "arquivos", filtro],
    primeira,
    tamanho: LIMITE,
    buscar: (d, l) => api.listarArquivos(extracaoId, filtro, d, l),
    indices,
  });

  return (
    <div ref={rolagem} className="h-[calc(100vh-17rem)] min-h-80 overflow-y-auto" aria-label="Mídias (lista longa)">
      <ul aria-label="Mídias" className="relative w-full" style={{ height: virtual.getTotalSize() + ESPACO }}>
        {linhasVisiveis.map((l) => (
          <li
            key={l.key}
            className="absolute inset-x-0 grid gap-3 px-3"
            style={{ top: l.start + ESPACO, height: alturaLinha - ESPACO, gridTemplateColumns: `repeat(${colunas}, minmax(0, 1fr))` }}
          >
            {Array.from({ length: colunas }, (_, k) => {
              const i = l.index * colunas + k;
              if (i >= primeira.total) return null;
              const f = item(i);
              return f ? (
                <CartaoMidia key={f.id} caso={caso} f={f} ativo={selecionado?.id === f.id} selecionar={selecionar} />
              ) : (
                <span key={`v${i}`} className="midia-forense animate-pulse rounded-lg border border-borda bg-sup-2/40" aria-hidden />
              );
            })}
          </li>
        ))}
      </ul>
      <p className="tabular px-3 pb-2 text-center text-2xs text-texto-3" role="status">
        {numero(primeira.total)} itens · carregados conforme a rolagem
      </p>
    </div>
  );
}
