import { useState, type FormEvent } from "react";
import { Link, useSearchParams } from "react-router-dom";
import type { Caso, FiltroBusca } from "../api/cliente";
import { useBusca, useExtracoes } from "../api/consultas";
import { Botao } from "../componentes/Botao";
import { Aviso, Cabecalho, Cartao, Carregando, Paginacao, Vazio } from "../componentes/Estrutura";
import { dataHoraMs, limiteDia, numero, tamanho } from "../util/formatar";
import { nomeCategoria, rotuloApp } from "../util/rotulos";
import { ComCasoAberto } from "./PaginaCaso";

const LIMITE = 50;
const INICIO = "\u0001";
const FIM = "\u0002";

export function PaginaBusca() {
  return <ComCasoAberto>{(caso) => <Busca caso={caso} />}</ComCasoAberto>;
}

/** Trecho devolvido pelo backend com o termo entre \u0001 e \u0002. */
export function TrechoDestacado({ texto }: { texto: string }) {
  const partes: { texto: string; marcado: boolean }[] = [];
  let resto = texto;
  while (resto) {
    const i = resto.indexOf(INICIO);
    if (i < 0) {
      partes.push({ texto: resto, marcado: false });
      break;
    }
    if (i > 0) partes.push({ texto: resto.slice(0, i), marcado: false });
    const f = resto.indexOf(FIM, i);
    const fim = f < 0 ? resto.length : f;
    partes.push({ texto: resto.slice(i + 1, fim), marcado: true });
    resto = resto.slice(fim + 1);
  }
  return (
    <>
      {partes.map((p, i) =>
        p.marcado ? (
          <mark key={i} className="rounded bg-amber-200 px-0.5 text-slate-900">
            {p.texto}
          </mark>
        ) : (
          <span key={i}>{p.texto}</span>
        ),
      )}
    </>
  );
}

function Busca({ caso }: { caso: Caso }) {
  const [params, setParams] = useSearchParams();
  const extracoes = useExtracoes(caso.id);
  const [texto, setTexto] = useState(params.get("q") ?? "");
  const [extracaoId, setExtracaoId] = useState("");
  const [de, setDe] = useState("");
  const [ate, setAte] = useState("");
  const [filtro, setFiltro] = useState<FiltroBusca | null>(
    params.get("q") ? { texto: params.get("q")! } : null,
  );
  const [deslocamento, setDeslocamento] = useState(0);
  const resultado = useBusca(caso.id, filtro, deslocamento, LIMITE);

  function buscar(e: FormEvent) {
    e.preventDefault();
    const f: FiltroBusca = { texto: texto.trim() };
    if (extracaoId) f.extracao_id = extracaoId;
    const deMs = de ? limiteDia(de, caso.fuso_horario) : null;
    const ateMs = ate ? limiteDia(ate, caso.fuso_horario, true) : null;
    if (deMs != null) f.de_ms = deMs;
    if (ateMs != null) f.ate_ms = ateMs;
    setFiltro(f);
    setDeslocamento(0);
    setParams(f.texto ? { q: f.texto } : {}, { replace: true });
  }

  return (
    <>
      <Cabecalho
        titulo="Busca"
        subtitulo={
          <Link to={`/casos/${caso.id}`} className="text-marca-700 hover:underline">
            ← Voltar ao caso
          </Link>
        }
      />
      <div className="grid gap-6">
        <Cartao>
          <form onSubmit={buscar} className="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]" role="search">
            <input
              id="texto-busca"
              type="search"
              value={texto}
              onChange={(e) => setTexto(e.target.value)}
              placeholder="Palavras, telefone ou nome de arquivo"
              aria-label="Texto da busca"
              className="rounded-lg border border-slate-300 px-3 py-2 text-sm outline-none focus:border-marca-500"
            />
            <Botao type="submit">Buscar</Botao>
            <div className="flex flex-wrap items-center gap-3 text-sm md:col-span-2">
              {(extracoes.data?.length ?? 0) > 1 && (
                <select
                  aria-label="Extração"
                  value={extracaoId}
                  onChange={(e) => setExtracaoId(e.target.value)}
                  className="rounded-lg border border-slate-300 px-2 py-1.5"
                >
                  <option value="">Todas as extrações</option>
                  {extracoes.data?.map((x) => (
                    <option key={x.id} value={x.id}>
                      {x.importador_nome} · {x.iniciada_em.slice(0, 10)}
                    </option>
                  ))}
                </select>
              )}
              <label className="flex items-center gap-2 text-slate-600">
                De
                <input type="date" value={de} onChange={(e) => setDe(e.target.value)} className="rounded-lg border border-slate-300 px-2 py-1" />
              </label>
              <label className="flex items-center gap-2 text-slate-600">
                Até
                <input type="date" value={ate} onChange={(e) => setAte(e.target.value)} className="rounded-lg border border-slate-300 px-2 py-1" />
              </label>
              <span className="text-xs text-slate-500">
                Sem diferenciar acentos e maiúsculas. Palavras parciais funcionam (“encontr” acha “encontro”). Datas no
                fuso do caso ({caso.fuso_horario}).
              </span>
            </div>
          </form>
        </Cartao>

        {filtro == null || !filtro.texto ? null : resultado.isPending ? (
          <Carregando texto="Buscando…" />
        ) : resultado.isError ? (
          <Aviso>{resultado.error.message}</Aviso>
        ) : (
          <>
            <Cartao
              titulo={`Registros (${numero(resultado.data.artefatos.total)})${
                resultado.data.modo === "telefone" ? " · busca por telefone" : ""
              }`}
            >
              {resultado.data.artefatos.total === 0 ? (
                <Vazio titulo="Nenhum registro encontrado" />
              ) : (
                <ul className="divide-y divide-slate-100" aria-label="Registros encontrados">
                  {resultado.data.artefatos.itens.map((a) => {
                    const r = a.artefato;
                    const destino =
                      a.conversa_id != null
                        ? `/casos/${caso.id}/conversas/${a.conversa_id}?registro=${r.id}`
                        : `/casos/${caso.id}/extracoes/${a.extracao_id}`;
                    return (
                      <li key={r.id} className="py-2">
                        <Link to={destino} className="block rounded-lg px-2 py-1 hover:bg-slate-50" data-teste="achado">
                          <div className="flex flex-wrap items-baseline gap-x-3 text-xs text-slate-500">
                            <span>{dataHoraMs(r.data_hora_ms, caso.fuso_horario)}</span>
                            <span className="uppercase">{rotuloApp(r.app)}</span>
                            <span>{r.de_mim == null ? "" : r.de_mim ? "Enviada" : "Recebida"}</span>
                            <span className="font-mono">{r.remetente ?? r.destinatarios.join(", ")}</span>
                          </div>
                          <p className="mt-0.5 text-sm text-slate-900">
                            {a.trecho ? <TrechoDestacado texto={a.trecho} /> : (r.conteudo ?? "—")}
                          </p>
                        </Link>
                      </li>
                    );
                  })}
                </ul>
              )}
            </Cartao>
            {resultado.data.modo === "texto" && (
              <Cartao titulo={`Arquivos (${numero(resultado.data.arquivos.total)})`}>
                {resultado.data.arquivos.total === 0 ? (
                  <Vazio titulo="Nenhum arquivo com esse nome" />
                ) : (
                  <table className="w-full text-left text-sm" aria-label="Arquivos encontrados">
                    <tbody className="divide-y divide-slate-100">
                      {resultado.data.arquivos.itens.map((f) => (
                        <tr key={f.id} className="align-top">
                          <td className="break-all py-2 pr-4 font-mono text-xs">
                            <Link to={`/casos/${caso.id}/extracoes/${f.extracao_id}`} className="hover:underline">
                              {f.caminho}
                            </Link>
                          </td>
                          <td className="whitespace-nowrap py-2 pr-4 text-xs">{nomeCategoria[f.categoria] ?? f.categoria}</td>
                          <td className="whitespace-nowrap py-2 pr-4 text-right text-xs">{tamanho(f.tamanho)}</td>
                          <td className="py-2 font-mono text-xs" title={f.sha256 ?? undefined}>
                            {f.sha256 ? `${f.sha256.slice(0, 16)}…` : "—"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </Cartao>
            )}
            <Paginacao
              total={Math.max(resultado.data.artefatos.total, resultado.data.arquivos.total)}
              deslocamento={deslocamento}
              limite={LIMITE}
              mudar={setDeslocamento}
            />
          </>
        )}
      </div>
    </>
  );
}
