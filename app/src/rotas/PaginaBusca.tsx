import { useEffect, useState, type FormEvent } from "react";
import { Link, useSearchParams } from "react-router-dom";
import type { Caso, FiltroBusca } from "../api/cliente";
import { useBusca, useExtracoes } from "../api/consultas";
import { Botao } from "../componentes/Botao";
import { Aviso, Cabecalho, Cartao, Carregando, entrada, Paginacao, Selo, tabela, Vazio } from "../componentes/Estrutura";
import { Icone } from "../componentes/Icone";
import { dataHoraMs, limiteDia, numero, tamanho } from "../util/formatar";
import { nomeApp, nomeCategoria, nomeTipoArtefato, rotuloApp, rotuloSubtipo } from "../util/rotulos";
import { ComCasoAberto, VoltarAoCaso } from "./PaginaCaso";

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
          <mark key={i} className="rounded-sm bg-ouro/35 px-0.5 text-texto">
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
  const [app, setApp] = useState("");
  const [de, setDe] = useState("");
  const [ate, setAte] = useState("");
  const [filtro, setFiltro] = useState<FiltroBusca | null>(params.get("q") ? { texto: params.get("q")! } : null);
  const [deslocamento, setDeslocamento] = useState(0);
  const resultado = useBusca(caso.id, filtro, deslocamento, LIMITE);

  // Nova busca vinda do cabeçalho (Ctrl+K) enquanto esta tela já está aberta.
  const q = params.get("q");
  useEffect(() => {
    if (q && q !== filtro?.texto) {
      setTexto(q);
      setFiltro({ texto: q });
      setDeslocamento(0);
    }
    // Só reage à URL; o próprio formulário atualiza `filtro`.
  }, [q]);

  function buscar(e: FormEvent) {
    e.preventDefault();
    const f: FiltroBusca = { texto: texto.trim() };
    if (extracaoId) f.extracao_id = extracaoId;
    if (app) f.app = app;
    const deMs = de ? limiteDia(de, caso.fuso_horario) : null;
    const ateMs = ate ? limiteDia(ate, caso.fuso_horario, true) : null;
    if (deMs != null) f.de_ms = deMs;
    if (ateMs != null) f.ate_ms = ateMs;
    setFiltro(f);
    setDeslocamento(0);
    setParams(f.texto ? { q: f.texto } : {}, { replace: true });
  }

  const r = resultado.data;

  return (
    <>
      <Cabecalho
        icone="busca"
        sobre={<VoltarAoCaso caso={caso} />}
        titulo="Busca"
        subtitulo="Mensagens, telefones e nomes de arquivo de todas as extrações do caso."
      />
      <div className="space-y-4">
        <section aria-label="Formulário de busca" className="rounded-xl border border-borda bg-sup p-4">
          <form onSubmit={buscar} className="space-y-3" role="search">
            <div className="flex gap-2">
              <div className="relative flex-1">
                <Icone nome="busca" className="pointer-events-none absolute left-3.5 top-1/2 size-5 -translate-y-1/2 text-texto-3" />
                <input
                  id="texto-busca"
                  type="search"
                  value={texto}
                  onChange={(e) => setTexto(e.target.value)}
                  placeholder="Palavras, telefone ou nome de arquivo"
                  aria-label="Texto da busca"
                  className="w-full rounded-lg border border-borda-2 bg-fundo py-2.5 pl-11 pr-3 text-[15px] text-texto outline-none placeholder:text-texto-3 focus:border-ouro/70 focus:ring-2 focus:ring-ouro/20"
                />
              </div>
              <Botao type="submit" className="px-5">
                Buscar
              </Botao>
            </div>
            <div className="flex flex-wrap items-center gap-3 text-xs text-texto-2">
              {(extracoes.data?.length ?? 0) > 1 && (
                <select aria-label="Extração" value={extracaoId} onChange={(e) => setExtracaoId(e.target.value)} className={entrada}>
                  <option value="">Todas as extrações</option>
                  {extracoes.data?.map((x) => (
                    <option key={x.id} value={x.id}>
                      {x.importador_nome} · {x.iniciada_em.slice(0, 10)}
                    </option>
                  ))}
                </select>
              )}
              <select aria-label="App" value={app} onChange={(e) => setApp(e.target.value)} className={entrada}>
                <option value="">Todos os apps</option>
                {Object.entries(nomeApp).map(([id, nome]) => (
                  <option key={id} value={id}>
                    {nome}
                  </option>
                ))}
              </select>
              <label className="flex items-center gap-2">
                De
                <input type="date" value={de} onChange={(e) => setDe(e.target.value)} className={entrada} />
              </label>
              <label className="flex items-center gap-2">
                Até
                <input type="date" value={ate} onChange={(e) => setAte(e.target.value)} className={entrada} />
              </label>
              <span className="text-texto-3">
                Sem diferenciar acentos e maiúsculas. Palavras parciais funcionam (“encontr” acha “encontro”). Datas no fuso do caso (
                {caso.fuso_horario}).
              </span>
            </div>
          </form>
        </section>

        {filtro == null || !filtro.texto ? (
          <Vazio titulo="Digite o que procura" icone="busca">
            Um telefone em qualquer formato (ex.: (21) 90000-0002) busca pelos dígitos; palavras buscam no texto das mensagens e no
            caminho dos arquivos.
          </Vazio>
        ) : resultado.isPending ? (
          <Carregando texto="Buscando…" />
        ) : resultado.isError ? (
          <Aviso>{resultado.error.message}</Aviso>
        ) : r ? (
          <>
            <div className="flex flex-wrap items-center gap-2 text-xs text-texto-2" role="status">
              <span>Resultados para</span>
              <span className="rounded-md bg-sup-2 px-2 py-0.5 font-medium text-texto">{filtro.texto}</span>
              {r.modo === "telefone" && <Selo tom="info">interpretado como telefone</Selo>}
              <span className="tabular">
                · {numero(r.artefatos.total)} registro(s)
                {r.modo === "texto" ? ` · ${numero(r.arquivos.total)} arquivo(s)` : ""}
              </span>
            </div>
            <Cartao
              titulo={`Registros (${numero(r.artefatos.total)})${r.modo === "telefone" ? " · busca por telefone" : ""}`}
              icone="conversas"
              denso
            >
              {r.artefatos.total === 0 ? (
                <div className="p-4">
                  <Vazio titulo="Nenhum registro encontrado" />
                </div>
              ) : (
                <ul className="divide-y divide-borda" aria-label="Registros encontrados">
                  {r.artefatos.itens.map((a) => {
                    const reg = a.artefato;
                    const destino =
                      a.conversa_id != null
                        ? `/casos/${caso.id}/conversas/${a.conversa_id}?registro=${reg.id}`
                        : `/casos/${caso.id}/extracoes/${a.extracao_id}`;
                    return (
                      <li key={reg.id}>
                        <Link to={destino} className="group block px-4 py-2.5 transition-colors hover:bg-sup-2" data-teste="achado">
                          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-texto-3">
                            <span className="tabular">{dataHoraMs(reg.data_hora_ms, caso.fuso_horario)}</span>
                            <span className="font-medium uppercase text-texto-2">{rotuloApp(reg.app)}</span>
                            {reg.tipo !== "mensagem" && <Selo tom="info">{nomeTipoArtefato[reg.tipo] ?? reg.tipo}</Selo>}
                            <span>{reg.de_mim == null ? "" : reg.de_mim ? "Enviada" : "Recebida"}</span>
                            <span className="font-mono">{reg.remetente ?? (reg.destinatarios.join(", ") || (reg.tipo !== "mensagem" ? reg.id_externo : ""))}</span>
                            <span className="ml-auto text-ouro-claro opacity-0 transition-opacity group-hover:opacity-100">
                              {a.conversa_id != null ? "abrir na conversa →" : "abrir na extração →"}
                            </span>
                          </div>
                          <p className="mt-0.5 text-[13px] text-texto">
                            {a.trecho ? (
                              <TrechoDestacado texto={a.trecho} />
                            ) : (
                              (reg.conteudo ?? reg.nome_arquivo ?? (reg.subtipo && reg.subtipo !== reg.app ? rotuloSubtipo(reg.subtipo) : "—"))
                            )}
                          </p>
                        </Link>
                      </li>
                    );
                  })}
                </ul>
              )}
            </Cartao>
            {r.modo === "texto" && (
              <Cartao titulo={`Arquivos (${numero(r.arquivos.total)})`} icone="arquivos" denso>
                {r.arquivos.total === 0 ? (
                  <div className="p-4">
                    <Vazio titulo="Nenhum arquivo com esse nome" />
                  </div>
                ) : (
                  <table className={tabela.base} aria-label="Arquivos encontrados">
                    <tbody className={tabela.corpo}>
                      {r.arquivos.itens.map((f) => (
                        <tr key={f.id} className={tabela.linha}>
                          <td className={`${tabela.td} break-all font-mono text-xs`}>
                            <Link to={`/casos/${caso.id}/extracoes/${f.extracao_id}`} className="text-texto hover:text-ouro-claro hover:underline">
                              {f.caminho}
                            </Link>
                          </td>
                          <td className={`${tabela.td} whitespace-nowrap text-xs`}>{nomeCategoria[f.categoria] ?? f.categoria}</td>
                          <td className={`${tabela.td} tabular whitespace-nowrap text-right text-xs`}>{tamanho(f.tamanho)}</td>
                          <td className={`${tabela.td} font-mono text-xs text-texto-2`} title={f.sha256 ?? undefined}>
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
              total={Math.max(r.artefatos.total, r.arquivos.total)}
              deslocamento={deslocamento}
              limite={LIMITE}
              mudar={setDeslocamento}
            />
          </>
        ) : null}
      </div>
    </>
  );
}
