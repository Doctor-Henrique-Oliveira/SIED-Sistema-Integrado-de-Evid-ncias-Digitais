import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import type { Caso, Extracao } from "../api/cliente";
import {
  useArquivos,
  useArtefatos,
  useDetalheArtefato,
  useDispositivos,
  useExecucoes,
  useExtracao,
  useTotaisPorCategoria,
} from "../api/consultas";
import { Aviso, Cabecalho, Cartao, Carregando, Definicoes, Hash, Paginacao, Selo, Vazio } from "../componentes/Estrutura";
import { dataHora, dataHoraMs, numero, tamanho } from "../util/formatar";
import { nomeCategoria, nomeStatusExecucao, nomeStatusExtracao, tomStatus } from "../util/rotulos";
import { ComCasoAberto } from "./PaginaCaso";

const LIMITE = 100;

export function PaginaExtracao() {
  const { extracaoId = "" } = useParams();
  return <ComCasoAberto>{(caso) => <DetalheExtracao caso={caso} extracaoId={extracaoId} />}</ComCasoAberto>;
}

type Aba = "artefatos" | "arquivos" | "processamento";

function DetalheExtracao({ caso, extracaoId }: { caso: Caso; extracaoId: string }) {
  const extracao = useExtracao(extracaoId);
  const dispositivos = useDispositivos(caso.id);
  const [aba, setAba] = useState<Aba>("artefatos");

  if (extracao.isPending) return <Carregando />;
  if (extracao.isError) return <Aviso>{extracao.error.message}</Aviso>;
  const x = extracao.data;
  const disp = dispositivos.data?.find((d) => d.id === x.dispositivo_id);

  return (
    <>
      <Cabecalho
        titulo={`Extração — ${disp?.apelido ?? "dispositivo"}`}
        subtitulo={
          <Link to={`/casos/${caso.id}`} className="text-marca-700 hover:underline">
            ← Voltar ao caso
          </Link>
        }
      />
      <div className="grid gap-6">
        <IdentificacaoExtracao caso={caso} x={x} />
        <div>
          <div role="tablist" aria-label="Conteúdo da extração" className="mb-3 flex gap-1 border-b border-slate-200">
            {(
              [
                ["artefatos", `Registros interpretados (${numero(x.total_artefatos ?? 0)})`],
                ["arquivos", `Arquivos (${numero(x.total_arquivos ?? 0)})`],
                ["processamento", "Processamento"],
              ] as [Aba, string][]
            ).map(([id, rotulo]) => (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={aba === id}
                onClick={() => setAba(id)}
                className={`-mb-px border-b-2 px-4 py-2 text-sm font-medium ${
                  aba === id ? "border-marca-600 text-marca-700" : "border-transparent text-slate-500 hover:text-slate-800"
                }`}
              >
                {rotulo}
              </button>
            ))}
          </div>
          {aba === "artefatos" && <AbaArtefatos caso={caso} x={x} />}
          {aba === "arquivos" && <AbaArquivos caso={caso} x={x} />}
          {aba === "processamento" && <AbaProcessamento caso={caso} x={x} />}
        </div>
      </div>
    </>
  );
}

function IdentificacaoExtracao({ caso, x }: { caso: Caso; x: Extracao }) {
  const fuso = caso.fuso_horario;
  return (
    <Cartao
      titulo="Identificação da importação"
      acoes={<Selo tom={tomStatus(x.status)}>{nomeStatusExtracao[x.status] ?? x.status}</Selo>}
    >
      <Definicoes
        itens={[
          ["Pasta de origem", <span className="break-all font-mono text-xs">{x.origem_caminho}</span>],
          ["Importador", `${x.importador_nome} (v${x.importador_versao})`],
          ["Ferramenta de extração", x.ferramenta_versao ?? "não identificada"],
          ["Arquivos inventariados", `${numero(x.total_arquivos)} (${tamanho(x.total_bytes)})`],
          ["Arquivos ilegíveis", numero(x.arquivos_ilegiveis)],
          ["Itens ignorados", numero(x.itens_ignorados)],
          ["Registros interpretados", numero(x.total_artefatos)],
          ["Avisos", numero(x.total_avisos)],
          ["Iniciada em", dataHora(x.iniciada_em, fuso)],
          ["Concluída em", dataHora(x.concluida_em, fuso)],
          ["Analista / estação", `${x.usuario} · ${x.maquina}`],
          ["Versão do SIED", x.sied_versao],
        ]}
      />
      <div className="mt-4">
        <p className="text-xs font-medium uppercase tracking-wide text-slate-500">SHA-256 do manifesto</p>
        <p className="mt-0.5" data-teste="hash-manifesto">
          <Hash valor={x.hash_manifesto} />
        </p>
        <p className="mt-1 text-xs text-slate-500">
          O manifesto (formato sha256sum) está em importacoes\{x.id}\manifesto.sha256 na pasta do caso.
        </p>
      </div>
      {x.erro && x.status !== "concluida" && (
        <div className="mt-4">
          <Aviso tipo={x.status === "falhou" ? "erro" : "info"}>{x.erro}</Aviso>
        </div>
      )}
      {x.observacoes.length > 0 && (
        <details className="mt-4 text-sm">
          <summary className="cursor-pointer text-slate-700">Observações da importação ({x.observacoes.length})</summary>
          <ul className="mt-2 list-disc space-y-1 pl-5 text-xs text-slate-600">
            {x.observacoes.map((o, i) => (
              <li key={i} className="break-all">
                {o}
              </li>
            ))}
          </ul>
        </details>
      )}
    </Cartao>
  );
}

function AbaArtefatos({ caso, x }: { caso: Caso; x: Extracao }) {
  const [deslocamento, setDeslocamento] = useState(0);
  const [selecionado, setSelecionado] = useState<number | null>(null);
  const artefatos = useArtefatos(x.id, deslocamento, LIMITE);

  if (artefatos.isPending) return <Carregando />;
  if (artefatos.isError) return <Aviso>{artefatos.error.message}</Aviso>;
  if (artefatos.data.total === 0)
    return (
      <Vazio titulo="Nenhum registro interpretado">
        Os parsers disponíveis não encontraram bancos conhecidos nesta extração. Todos os arquivos estão na aba Arquivos.
      </Vazio>
    );

  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,26rem)]">
      <Cartao>
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm" aria-label="Registros interpretados">
            <thead className="text-xs uppercase tracking-wide text-slate-500">
              <tr>
                <th className="py-2 pr-4 font-medium">Data e hora</th>
                <th className="py-2 pr-4 font-medium">App</th>
                <th className="py-2 pr-4 font-medium">Direção</th>
                <th className="py-2 pr-4 font-medium">Contato</th>
                <th className="py-2 font-medium">Conteúdo</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {artefatos.data.itens.map((a) => (
                <tr
                  key={a.id}
                  onClick={() => setSelecionado(a.id)}
                  onKeyDown={(e) => e.key === "Enter" && setSelecionado(a.id)}
                  tabIndex={0}
                  aria-selected={selecionado === a.id}
                  className={`cursor-pointer ${selecionado === a.id ? "bg-marca-50" : "hover:bg-slate-50"}`}
                >
                  <td className="whitespace-nowrap py-2 pr-4 text-slate-600">{dataHoraMs(a.data_hora_ms, caso.fuso_horario)}</td>
                  <td className="py-2 pr-4 uppercase text-slate-600">{a.app ?? a.tipo}</td>
                  <td className="py-2 pr-4">{a.de_mim == null ? "—" : a.de_mim ? "Enviada" : "Recebida"}</td>
                  <td className="whitespace-nowrap py-2 pr-4 font-mono text-xs">
                    {a.remetente ?? (a.destinatarios.join(", ") || "—")}
                  </td>
                  <td className="max-w-md truncate py-2" title={a.conteudo ?? undefined}>
                    {a.conteudo ?? "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <Paginacao total={artefatos.data.total} deslocamento={deslocamento} limite={LIMITE} mudar={setDeslocamento} />
      </Cartao>
      <PainelTecnico caso={caso} artefatoId={selecionado} />
    </div>
  );
}

/** Visão técnica: como o registro foi lido e de onde veio, até a linha da tabela. */
function PainelTecnico({ caso, artefatoId }: { caso: Caso; artefatoId: number | null }) {
  const d = useDetalheArtefato(artefatoId);
  if (artefatoId == null)
    return (
      <Cartao titulo="Detalhes técnicos">
        <p className="text-sm text-slate-500">Selecione um registro para ver a origem exata do dado.</p>
      </Cartao>
    );
  if (d.isPending) return <Cartao titulo="Detalhes técnicos"><Carregando /></Cartao>;
  if (d.isError) return <Aviso>{d.error.message}</Aviso>;
  const a = d.data;
  return (
    <Cartao titulo="Detalhes técnicos">
      <div className="space-y-4 text-sm" aria-label="Detalhes técnicos do registro">
        <Definicoes
          itens={[
            ["Data (fuso do caso)", dataHoraMs(a.resumo.data_hora_ms, caso.fuso_horario)],
            ["Valor original", <span className="font-mono text-xs">{a.data_hora_original ?? "—"}</span>],
            ["Formato", a.formato_data],
            ["Parser", `${a.parser} v${a.parser_versao}`],
            ["Importador", `${a.importador} v${a.importador_versao}`],
            ["Estado / confiança", `${a.resumo.estado_registro} · ${a.confianca}`],
          ]}
        />
        {a.referencias.map((r, i) => (
          <div key={i} className="space-y-2 rounded-lg bg-slate-50 p-3 ring-1 ring-slate-200">
            <p className="text-xs font-semibold uppercase tracking-wide text-slate-500">
              {r.principal ? "Origem" : `Origem adicional (${r.papel_arquivo})`}
            </p>
            <p className="break-all font-mono text-xs text-slate-800" data-teste="origem-arquivo">
              {r.arquivo_caminho}
            </p>
            <p>
              <span className="text-xs text-slate-500">SHA-256 </span>
              <Hash valor={r.arquivo_sha256} />
            </p>
            <p className="text-xs text-slate-700">
              Tabela <span className="font-mono">{r.tabela ?? "—"}</span> · registro{" "}
              <span className="font-mono">{r.id_registro ?? "—"}</span>
            </p>
            {r.consulta && <p className="break-all font-mono text-xs text-slate-600">{r.consulta}</p>}
            <table className="w-full text-xs" aria-label="Valores brutos">
              <tbody>
                {Object.entries(r.valores_brutos as Record<string, unknown>).map(([k, v]) => (
                  <tr key={k} className="align-top">
                    <td className="py-0.5 pr-3 font-mono text-slate-500">{k}</td>
                    <td className="break-all py-0.5 font-mono text-slate-900">{v === null ? "NULL" : JSON.stringify(v)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ))}
        <p>
          <span className="text-xs text-slate-500">Hash do registro normalizado </span>
          <Hash valor={a.hash_registro} />
        </p>
      </div>
    </Cartao>
  );
}

function AbaArquivos({ caso, x }: { caso: Caso; x: Extracao }) {
  const [categoria, setCategoria] = useState<string | null>(null);
  const [texto, setTexto] = useState("");
  const [busca, setBusca] = useState<string | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const totais = useTotaisPorCategoria(x.id);
  const arquivos = useArquivos(x.id, { categoria, texto: busca }, deslocamento, LIMITE);

  function filtrar(c: string | null) {
    setCategoria(c);
    setDeslocamento(0);
  }

  return (
    <Cartao>
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={() => filtrar(null)}
          className={`rounded-full px-3 py-1 text-xs ring-1 ${categoria == null ? "bg-marca-600 text-white ring-marca-600" : "ring-slate-300"}`}
        >
          Todos
        </button>
        {totais.data?.map((t) => (
          <button
            key={t.categoria}
            type="button"
            onClick={() => filtrar(t.categoria)}
            className={`rounded-full px-3 py-1 text-xs ring-1 ${
              categoria === t.categoria ? "bg-marca-600 text-white ring-marca-600" : "ring-slate-300"
            }`}
          >
            {nomeCategoria[t.categoria] ?? t.categoria} ({numero(t.arquivos)})
          </button>
        ))}
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
            placeholder="Buscar por caminho ou SHA-256"
            aria-label="Buscar arquivos"
            className="w-72 rounded-lg border border-slate-300 px-3 py-1.5 text-sm outline-none focus:border-marca-500"
          />
        </form>
      </div>
      {arquivos.isPending ? (
        <Carregando />
      ) : arquivos.isError ? (
        <Aviso>{arquivos.error.message}</Aviso>
      ) : arquivos.data.total === 0 ? (
        <Vazio titulo="Nenhum arquivo encontrado" />
      ) : (
        <>
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm" aria-label="Inventário de arquivos">
              <thead className="text-xs uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="py-2 pr-4 font-medium">Caminho</th>
                  <th className="py-2 pr-4 font-medium">Tipo</th>
                  <th className="py-2 pr-4 text-right font-medium">Tamanho</th>
                  <th className="py-2 pr-4 font-medium">Modificado</th>
                  <th className="py-2 font-medium">SHA-256</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100">
                {arquivos.data.itens.map((f) => (
                  <tr key={f.id} className="align-top">
                    <td className="max-w-md break-all py-2 pr-4 font-mono text-xs">{f.caminho}</td>
                    <td className="whitespace-nowrap py-2 pr-4 text-xs">
                      {nomeCategoria[f.categoria] ?? f.categoria}
                      {f.assinatura && <span className="text-slate-500"> · {f.assinatura}</span>}
                    </td>
                    <td className="whitespace-nowrap py-2 pr-4 text-right text-xs">{tamanho(f.tamanho)}</td>
                    <td className="whitespace-nowrap py-2 pr-4 text-xs text-slate-600">
                      {dataHora(f.modificado_em, caso.fuso_horario)}
                    </td>
                    <td className="py-2 font-mono text-xs" title={f.sha256 ?? f.erro ?? undefined}>
                      {f.sha256 ? `${f.sha256.slice(0, 16)}…` : <span className="text-rose-700">ilegível</span>}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <Paginacao total={arquivos.data.total} deslocamento={deslocamento} limite={LIMITE} mudar={setDeslocamento} />
        </>
      )}
    </Cartao>
  );
}

function AbaProcessamento({ caso, x }: { caso: Caso; x: Extracao }) {
  const execucoes = useExecucoes(x.id);
  if (execucoes.isPending) return <Carregando />;
  if (execucoes.isError) return <Aviso>{execucoes.error.message}</Aviso>;
  if (execucoes.data.length === 0) return <Vazio titulo="Nenhum parser foi executado nesta extração" />;
  return (
    <Cartao>
      <div className="overflow-x-auto">
        <table className="w-full text-left text-sm" aria-label="Execuções de parser">
          <thead className="text-xs uppercase tracking-wide text-slate-500">
            <tr>
              <th className="py-2 pr-4 font-medium">Parser</th>
              <th className="py-2 pr-4 font-medium">Arquivo lido</th>
              <th className="py-2 pr-4 font-medium">Situação</th>
              <th className="py-2 pr-4 font-medium">Registros</th>
              <th className="py-2 font-medium">Concluída em</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100">
            {execucoes.data.map((e) => (
              <tr key={e.id} className="align-top">
                <td className="py-2 pr-4">
                  {e.parser_nome} <span className="text-xs text-slate-500">v{e.parser_versao}</span>
                </td>
                <td className="py-2 pr-4">
                  <div className="break-all font-mono text-xs">{e.arquivo_caminho ?? "—"}</div>
                  {e.copia_sha256 && (
                    <div className="text-xs text-slate-500" title="SHA-256 da cópia de trabalho (igual ao original)">
                      cópia verificada {e.copia_sha256.slice(0, 16)}…
                    </div>
                  )}
                  {e.erro && <div className="text-xs text-rose-700">{e.erro}</div>}
                </td>
                <td className="py-2 pr-4">
                  <Selo tom={tomStatus(e.status)}>{nomeStatusExecucao[e.status] ?? e.status}</Selo>
                </td>
                <td className="py-2 pr-4 text-xs">
                  {numero(e.artefatos)}
                  {e.avisos ? <span className="text-amber-700"> · {numero(e.avisos)} avisos</span> : null}
                </td>
                <td className="whitespace-nowrap py-2 text-xs text-slate-600">{dataHora(e.concluida_em, caso.fuso_horario)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Cartao>
  );
}
