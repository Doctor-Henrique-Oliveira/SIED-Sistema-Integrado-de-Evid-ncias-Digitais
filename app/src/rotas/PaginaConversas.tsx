import { Fragment, useEffect, useRef, useState } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router-dom";
import type { ArtefatoResumo, Caso, Conversa } from "../api/cliente";
import { useConversa, useConversas, useExtracoes, useMensagensConversa } from "../api/consultas";
import { Aviso, Cabecalho, Cartao, Carregando, Paginacao, Selo, Vazio } from "../componentes/Estrutura";
import { PainelTecnico } from "../componentes/PainelTecnico";
import { dataHoraMs, dia, hora, numero } from "../util/formatar";
import { rotuloApp } from "../util/rotulos";
import { ComCasoAberto } from "./PaginaCaso";

const LIMITE_LISTA = 100;
const LIMITE_MENSAGENS = 500;

export function PaginaConversas() {
  return <ComCasoAberto>{(caso) => <Conversas caso={caso} />}</ComCasoAberto>;
}

/** Nome exibido da conversa: participantes como emitidos pelo parser. */
export function tituloConversa(c: Conversa): string {
  if (c.participantes.length === 0) return `Conversa ${c.id_externo}`;
  const nomes = c.participantes.map((p) => p.identificador).join(", ");
  const resto = c.total_participantes - c.participantes.length;
  return resto > 0 ? `${nomes} e mais ${resto}` : nomes;
}

function Conversas({ caso }: { caso: Caso }) {
  const { conversaId } = useParams();
  const [params] = useSearchParams();
  const selecionada = conversaId ? Number(conversaId) : null;
  // Registro a destacar (vindo da busca).
  const destaque = params.get("registro") ? Number(params.get("registro")) : null;
  // Com a visão técnica aberta, a lista de conversas sai de cena em telas menores
  // para o chat e a origem caberem lado a lado.
  const [tecnica, setTecnica] = useState(destaque != null);

  return (
    <>
      <Cabecalho
        titulo="Conversas"
        subtitulo={
          <Link to={`/casos/${caso.id}`} className="text-marca-700 hover:underline">
            ← Voltar ao caso
          </Link>
        }
        acoes={
          <Link
            to={`/casos/${caso.id}/busca`}
            className="rounded-lg px-4 py-2 text-sm font-medium text-slate-700 ring-1 ring-inset ring-slate-300 hover:bg-slate-50"
          >
            Buscar em tudo
          </Link>
        }
      />
      <div className={`grid gap-4 ${tecnica ? "2xl:grid-cols-[20rem_minmax(0,1fr)]" : "lg:grid-cols-[20rem_minmax(0,1fr)]"}`}>
        <div className={`self-start ${tecnica ? "hidden 2xl:block" : ""}`}>
          <ListaConversas caso={caso} selecionada={selecionada} />
        </div>
        {selecionada == null ? (
          <Cartao>
            <p className="text-sm text-slate-500">Selecione uma conversa para ver as mensagens.</p>
          </Cartao>
        ) : (
          <Chat
            key={selecionada}
            caso={caso}
            conversaId={selecionada}
            destaque={destaque}
            tecnica={tecnica}
            setTecnica={setTecnica}
          />
        )}
      </div>
    </>
  );
}

function ListaConversas({ caso, selecionada }: { caso: Caso; selecionada: number | null }) {
  const [extracaoId, setExtracaoId] = useState<string | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const extracoes = useExtracoes(caso.id);
  const lista = useConversas(caso.id, extracaoId, deslocamento, LIMITE_LISTA);

  return (
    <Cartao>
      {(extracoes.data?.length ?? 0) > 1 && (
        <select
          aria-label="Extração"
          value={extracaoId ?? ""}
          onChange={(e) => {
            setExtracaoId(e.target.value || null);
            setDeslocamento(0);
          }}
          className="mb-3 w-full rounded-lg border border-slate-300 px-2 py-1.5 text-sm"
        >
          <option value="">Todas as extrações</option>
          {extracoes.data?.map((x) => (
            <option key={x.id} value={x.id}>
              {x.importador_nome} · {x.iniciada_em.slice(0, 10)}
            </option>
          ))}
        </select>
      )}
      {lista.isPending ? (
        <Carregando />
      ) : lista.isError ? (
        <Aviso>{lista.error.message}</Aviso>
      ) : lista.data.total === 0 ? (
        <Vazio titulo="Nenhuma conversa">Importe uma extração com mensagens para ver as conversas aqui.</Vazio>
      ) : (
        <>
          <ul className="-mx-2 divide-y divide-slate-100" aria-label="Lista de conversas">
            {lista.data.itens.map((c) => (
              <li key={c.id}>
                <Link
                  to={`/casos/${caso.id}/conversas/${c.id}`}
                  aria-current={selecionada === c.id ? "true" : undefined}
                  className={`block rounded-lg px-2 py-2 ${selecionada === c.id ? "bg-marca-50" : "hover:bg-slate-50"}`}
                >
                  <div className="flex items-baseline justify-between gap-2">
                    <span className="truncate text-sm font-medium text-slate-900">{tituloConversa(c)}</span>
                    <span className="shrink-0 text-xs text-slate-500">{rotuloApp(c.app)}</span>
                  </div>
                  <p className="truncate text-xs text-slate-600">{c.ultima_mensagem ?? "—"}</p>
                  <p className="text-xs text-slate-400">
                    {numero(c.total_mensagens)} mensagens · {dataHoraMs(c.ultima_ms, caso.fuso_horario)}
                  </p>
                </Link>
              </li>
            ))}
          </ul>
          <Paginacao total={lista.data.total} deslocamento={deslocamento} limite={LIMITE_LISTA} mudar={setDeslocamento} />
        </>
      )}
    </Cartao>
  );
}

function Chat({
  caso,
  conversaId,
  destaque,
  tecnica,
  setTecnica,
}: {
  caso: Caso;
  conversaId: number;
  destaque: number | null;
  tecnica: boolean;
  setTecnica: (v: boolean) => void;
}) {
  const conversa = useConversa(caso.id, conversaId);
  const [deslocamento, setDeslocamento] = useState(0);
  const mensagens = useMensagensConversa(caso.id, conversaId, deslocamento, LIMITE_MENSAGENS);
  const [selecionado, setSelecionado] = useState<number | null>(destaque);
  const navegar = useNavigate();

  if (conversa.isPending) return <Carregando />;
  if (conversa.isError) return <Aviso>{conversa.error.message}</Aviso>;
  const c = conversa.data;

  const cabecalho = (
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 pb-3">
      <div className="min-w-0">
        <h2 className="truncate font-semibold text-slate-900">{tituloConversa(c)}</h2>
        <p className="text-xs text-slate-500">
          {rotuloApp(c.app)} · {numero(c.total_mensagens)} mensagens ({numero(c.enviadas)} enviadas) ·{" "}
          {dataHoraMs(c.primeira_ms, caso.fuso_horario)} a {dataHoraMs(c.ultima_ms, caso.fuso_horario)}
        </p>
      </div>
      <div role="group" aria-label="Modo de visualização" className="flex rounded-lg ring-1 ring-slate-300">
        {(
          [
            [false, "Visão normal"],
            [true, "Detalhes técnicos"],
          ] as [boolean, string][]
        ).map(([valor, rotulo]) => (
          <button
            key={rotulo}
            type="button"
            aria-pressed={tecnica === valor}
            onClick={() => setTecnica(valor)}
            className={`px-3 py-1.5 text-xs font-medium first:rounded-l-lg last:rounded-r-lg ${
              tecnica === valor ? "bg-marca-600 text-white" : "text-slate-700 hover:bg-slate-50"
            }`}
          >
            {rotulo}
          </button>
        ))}
      </div>
    </div>
  );

  return (
    <div className={`grid gap-4 ${tecnica ? "lg:grid-cols-[minmax(0,1fr)_26rem]" : ""}`}>
      <Cartao>
        {cabecalho}
        {mensagens.isPending ? (
          <Carregando />
        ) : mensagens.isError ? (
          <Aviso>{mensagens.error.message}</Aviso>
        ) : (
          <>
            <Paginacao
              total={mensagens.data.total}
              deslocamento={deslocamento}
              limite={LIMITE_MENSAGENS}
              mudar={setDeslocamento}
            />
            <Mensagens
              caso={caso}
              itens={mensagens.data.itens}
              selecionado={selecionado}
              selecionar={(id) => {
                setSelecionado(id);
                setTecnica(true);
                if (destaque != null) navegar(`/casos/${caso.id}/conversas/${conversaId}`, { replace: true });
              }}
              rolarPara={destaque}
            />
          </>
        )}
      </Cartao>
      {tecnica && <PainelTecnico caso={caso} artefatoId={selecionado} />}
    </div>
  );
}

function Mensagens({
  caso,
  itens,
  selecionado,
  selecionar,
  rolarPara,
}: {
  caso: Caso;
  itens: ArtefatoResumo[];
  selecionado: number | null;
  selecionar: (id: number) => void;
  rolarPara: number | null;
}) {
  const alvo = useRef<HTMLDivElement>(null);
  useEffect(() => {
    alvo.current?.scrollIntoView({ block: "center" });
  }, [rolarPara, itens]);

  let diaAnterior: string | null = null;
  return (
    <div className="mt-3 space-y-1.5" aria-label="Mensagens da conversa">
      {itens.map((m) => {
        const d = m.data_hora_ms == null ? "Sem data" : dia(m.data_hora_ms, caso.fuso_horario);
        const separador = d !== diaAnterior;
        diaAnterior = d;
        const enviada = m.de_mim === true;
        const ativo = selecionado === m.id;
        return (
          <Fragment key={m.id}>
            {separador && (
              <div className="py-2 text-center">
                <span className="rounded-full bg-slate-100 px-3 py-0.5 text-xs text-slate-600">{d}</span>
              </div>
            )}
            <div
              ref={m.id === rolarPara ? alvo : undefined}
              className={`flex ${enviada ? "justify-end" : "justify-start"}`}
            >
              <button
                type="button"
                onClick={() => selecionar(m.id)}
                aria-pressed={ativo}
                data-teste="mensagem"
                className={`max-w-[75%] rounded-2xl px-3 py-2 text-left text-sm shadow-sm ring-1 ${
                  enviada ? "rounded-br-sm bg-marca-50 ring-marca-200" : "rounded-bl-sm bg-white ring-slate-200"
                } ${ativo ? "outline-2 outline-offset-2 outline-marca-600" : ""}`}
              >
                {!enviada && m.remetente && <p className="mb-0.5 font-mono text-xs text-slate-500">{m.remetente}</p>}
                <p className="whitespace-pre-wrap break-words text-slate-900">
                  {m.conteudo ?? <span className="italic text-slate-400">(sem texto)</span>}
                </p>
                <p className="mt-1 flex items-center justify-end gap-2 text-[11px] text-slate-500">
                  {m.estado_registro !== "ativo" && <Selo tom="atencao">{m.estado_registro}</Selo>}
                  {m.data_hora_ms == null ? "—" : hora(m.data_hora_ms, caso.fuso_horario)}
                </p>
              </button>
            </div>
          </Fragment>
        );
      })}
    </div>
  );
}
