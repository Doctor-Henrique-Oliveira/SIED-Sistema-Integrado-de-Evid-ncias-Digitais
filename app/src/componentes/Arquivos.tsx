// Inventário de arquivos de uma extração: tabela (Arquivos, aba da extração) e
// grade (Mídias), com painel de detalhes. Tudo vem do inventário gravado na
// importação; nenhuma miniatura é gerada ainda (nada é lido da evidência aqui).

import { useState, type ReactNode } from "react";
import type { ArquivoExtracao, Caso, Extracao } from "../api/cliente";
import { useArquivos, useDispositivos, useExecucoes, useExtracoes, useTotaisPorCategoria } from "../api/consultas";
import { dataHora, extensao, nomeArquivo, numero, tamanho } from "../util/formatar";
import { nomeCategoria } from "../util/rotulos";
import { Aviso, Cartao, Carregando, entrada, Filtros, Hash, Paginacao, Selo, tabela, Vazio } from "./Estrutura";
import { Icone, type NomeIcone } from "./Icone";

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
  const [escolhida, setEscolhida] = useState<string | null>(null);
  const id = escolhida ?? extracoes.data?.[0]?.id ?? null;
  return { extracoes, id, setEscolhida };
}

function DetalheArquivo({ caso, f, parser, fechar }: {
  caso: Caso;
  f: ArquivoExtracao;
  parser: string | undefined;
  fechar: () => void;
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
    </Cartao>
  );
}

const LIMITE = 100;

/**
 * Inventário de uma extração. `midias` = grade com os filtros da galeria;
 * senão, tabela com todas as categorias.
 */
export function InventarioArquivos({ caso, extracaoId, midias }: { caso: Caso; extracaoId: string; midias?: boolean }) {
  const [categoria, setCategoria] = useState<string | null>(midias ? "midia" : null);
  const [texto, setTexto] = useState("");
  const [busca, setBusca] = useState<string | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const [selecionado, setSelecionado] = useState<ArquivoExtracao | null>(null);
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
    <div className={`grid gap-4 ${selecionado ? "xl:grid-cols-[minmax(0,1fr)_24rem]" : ""}`}>
      <Cartao denso>
        <div className="flex flex-wrap items-center gap-3 border-b border-borda px-3 py-2.5">
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
        {arquivos.isPending ? (
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
        ) : midias ? (
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(9.5rem,1fr))] gap-3 p-3" aria-label="Mídias">
            {arquivos.data.itens.map((f) => (
              <li key={f.id}>
                <button
                  type="button"
                  aria-pressed={selecionado?.id === f.id}
                  onClick={() => setSelecionado(f)}
                  title={f.caminho}
                  className={`flex w-full flex-col overflow-hidden rounded-lg border text-left transition-colors ${
                    selecionado?.id === f.id ? "border-ouro/70 bg-ouro/5" : "border-borda bg-sup-2/60 hover:border-borda-2"
                  }`}
                >
                  <span className="grid aspect-[4/3] w-full place-items-center bg-fundo text-texto-3">
                    <Icone nome={iconeCategoria[f.categoria] ?? "arquivos"} className="size-8" />
                  </span>
                  <span className="block w-full px-2 py-1.5">
                    <span className="block truncate text-xs text-texto">{nomeArquivo(f.caminho)}</span>
                    <span className="tabular block text-[11px] text-texto-3">
                      {nomeCategoria[f.categoria] ?? f.categoria} · {tamanho(f.tamanho)}
                    </span>
                  </span>
                </button>
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
                    className={`cursor-pointer ${selecionado?.id === f.id ? "bg-ouro/8" : tabela.linha}`}
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
        {arquivos.data && arquivos.data.total > LIMITE && (
          <div className="border-t border-borda px-3">
            <Paginacao total={arquivos.data.total} deslocamento={deslocamento} limite={LIMITE} mudar={setDeslocamento} />
          </div>
        )}
      </Cartao>
      {selecionado && (
        <DetalheArquivo caso={caso} f={selecionado} parser={parserDe(selecionado.caminho)} fechar={() => setSelecionado(null)} />
      )}
    </div>
  );
}
