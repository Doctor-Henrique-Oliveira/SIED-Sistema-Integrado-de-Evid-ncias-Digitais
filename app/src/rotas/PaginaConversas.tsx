// Conversas em três colunas: lista (esquerda), chat (centro) e contexto ou origem
// técnica (direita). As conversas são montadas a partir das mensagens gravadas;
// nada aqui altera registros. Título, tipo e nomes vêm dos registros que o próprio
// app gravou na mesma extração (ex.: assunto do grupo e agenda do WhatsApp).

import { Fragment, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router-dom";
import type { ArtefatoResumo, Caso, Conversa } from "../api/cliente";
import { useConversa, useConversas, useExtracoes, useMensagensConversa } from "../api/consultas";
import { Aviso, Cabecalho, Carregando, entrada, Filtros, LinkBotao, Paginacao, SecaoRecolhivel, Selo, Vazio } from "../componentes/Estrutura";
import { Icone } from "../componentes/Icone";
import { PainelTecnico } from "../componentes/PainelTecnico";
import { dataHoraMs, dia, hora, numero, tamanho } from "../util/formatar";
import { nomeEstadoRegistro, nomeTipoConversa, nomeVinculoAnexo, rotuloApp, rotuloSubtipo } from "../util/rotulos";
import { ComCasoAberto, VoltarAoCaso } from "./PaginaCaso";

const LIMITE_LISTA = 100;
const LIMITE_MENSAGENS = 500;

export function PaginaConversas() {
  return <ComCasoAberto>{(caso) => <Conversas caso={caso} />}</ComCasoAberto>;
}

/** Nome exibido da conversa: o título gravado pelo app (assunto do grupo ou nome do
 * contato); sem ele, os participantes (nome do contato ou identificador). */
export function tituloConversa(c: Conversa): string {
  if (c.titulo) return c.titulo;
  if (c.participantes.length === 0) return `Conversa ${c.id_externo}`;
  const nomes = c.participantes.map((p) => p.nome ?? p.identificador).join(", ");
  const resto = c.total_participantes - c.participantes.length;
  return resto > 0 ? `${nomes} e mais ${resto}` : nomes;
}

/** Grupo pelo tipo que o app gravou; sem ele, pela quantidade de participantes. */
const eGrupo = (c: Conversa) => (c.tipo ? c.tipo === "grupo" : c.total_participantes > 1);

const rotuloTipoConversa = (c: Conversa) =>
  c.tipo ? (nomeTipoConversa[c.tipo] ?? c.tipo) : eGrupo(c) ? "Grupo" : "Individual";

/** Nome do contato por identificador, para o remetente de cada mensagem. */
const nomesDe = (c: Conversa) => new Map(c.participantes.flatMap((p) => (p.nome ? [[p.identificador, p.nome] as const] : [])));

/** Resumo curto de como o anexo foi procurado no inventário (a frase completa fica no contexto). */
function situacaoAnexo(m: ArtefatoResumo): { texto: string; tom: "ok" | "atencao" | "neutro" } {
  if (m.anexo_arquivo_id != null) return { texto: "Na extração", tom: "ok" };
  switch (m.anexo_vinculo) {
    case "ambiguo":
      return { texto: "Mais de um arquivo possível", tom: "atencao" };
    case "tamanho_diferente":
      return { texto: "Tamanho não confere", tom: "atencao" };
    case "nao_encontrado":
      return { texto: "Não encontrado na extração", tom: "atencao" };
    default:
      return { texto: "Caminho não informado pelo app", tom: "neutro" };
  }
}

const iconeAnexo = (subtipo: string | null) =>
  subtipo === "imagem" || subtipo === "gif" || subtipo === "figurinha"
    ? "midias"
    : subtipo === "audio"
      ? "audio"
      : subtipo === "video"
        ? "video"
        : "documento";

const temAnexo = (m: ArtefatoResumo) => m.nome_arquivo != null || m.anexo_vinculo != null;
const temLocal = (m: ArtefatoResumo) => m.latitude != null && m.longitude != null;
const coordenadas = (m: ArtefatoResumo) => `${m.latitude!.toFixed(6)}, ${m.longitude!.toFixed(6)}`;

function Avatar({ c, grande }: { c: Conversa; grande?: boolean }) {
  return (
    <span
      className={`grid shrink-0 place-items-center rounded-full bg-sup-3 text-texto-2 ring-1 ring-borda-2 ${grande ? "size-10" : "size-9"}`}
      aria-hidden
    >
      <Icone nome={eGrupo(c) ? "grupo" : "usuario"} className={grande ? "size-5" : "size-4"} />
    </span>
  );
}

function Conversas({ caso }: { caso: Caso }) {
  const { conversaId } = useParams();
  const [params] = useSearchParams();
  const selecionada = conversaId ? Number(conversaId) : null;
  // Registro a destacar (vindo da busca).
  const destaque = params.get("registro") ? Number(params.get("registro")) : null;
  // Visão técnica: a coluna da direita mostra a origem da mensagem selecionada. Em
  // telas menores a lista sai de cena para o chat e a origem caberem lado a lado.
  const [tecnica, setTecnica] = useState(destaque != null);
  const [contexto, setContexto] = useState(true);

  const colunas =
    selecionada == null
      ? "grid-cols-[18rem_minmax(0,1fr)]"
      : tecnica
        ? "grid-cols-[minmax(0,1fr)_24rem] 2xl:grid-cols-[19rem_minmax(0,1fr)_26rem]"
        : contexto
          ? "grid-cols-[18rem_minmax(0,1fr)] xl:grid-cols-[18rem_minmax(0,1fr)_18rem] 2xl:grid-cols-[19rem_minmax(0,1fr)_20rem]"
          : "grid-cols-[18rem_minmax(0,1fr)] 2xl:grid-cols-[19rem_minmax(0,1fr)]";

  return (
    <>
      <Cabecalho
        icone="conversas"
        sobre={<VoltarAoCaso caso={caso} />}
        titulo="Conversas"
        acoes={
          <LinkBotao para={`/casos/${caso.id}/busca`} icone="busca">
            Buscar em tudo
          </LinkBotao>
        }
      />
      <div className={`grid h-[calc(100vh-10.25rem)] min-h-[28rem] gap-3 ${colunas}`}>
        <div className={`min-h-0 ${selecionada != null && tecnica ? "hidden 2xl:flex" : "flex"}`}>
          <ListaConversas caso={caso} selecionada={selecionada} />
        </div>
        {selecionada == null ? (
          <div className="grid place-items-center rounded-xl border border-borda bg-sup">
            <div className="max-w-sm text-center">
              <Icone nome="conversas" className="mx-auto size-8 text-texto-3" />
              <p className="mt-2 text-[13px] text-texto-2">Selecione uma conversa para ver as mensagens.</p>
            </div>
          </div>
        ) : (
          <Chat
            key={selecionada}
            caso={caso}
            conversaId={selecionada}
            destaque={destaque}
            tecnica={tecnica}
            setTecnica={setTecnica}
            contexto={contexto}
            setContexto={setContexto}
          />
        )}
      </div>
    </>
  );
}

// ---------------------------------------------------------------- Lista

function ListaConversas({ caso, selecionada }: { caso: Caso; selecionada: number | null }) {
  const [extracaoId, setExtracaoId] = useState<string | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const [filtro, setFiltro] = useState("");
  const [app, setApp] = useState<string | null>(null);
  const extracoes = useExtracoes(caso.id);
  const lista = useConversas(caso.id, extracaoId, deslocamento, LIMITE_LISTA);

  const itens = lista.data?.itens ?? [];
  const apps = [...new Set(itens.map((c) => c.app))];
  const termo = filtro.trim().toLowerCase();
  const visiveis = itens.filter(
    (c) =>
      (app == null || c.app === app) &&
      (!termo ||
        tituloConversa(c).toLowerCase().includes(termo) ||
        c.participantes.some((p) => p.identificador.toLowerCase().includes(termo)) ||
        (c.ultima_mensagem ?? "").toLowerCase().includes(termo)),
  );

  return (
    <section aria-label="Conversas do caso" className="flex min-h-0 w-full flex-col rounded-xl border border-borda bg-sup">
      <div className="space-y-2 border-b border-borda p-2.5">
        <div className="relative">
          <Icone nome="busca" className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-texto-3" />
          <input
            type="search"
            value={filtro}
            onChange={(e) => setFiltro(e.target.value)}
            placeholder="Filtrar por participante ou texto"
            aria-label="Filtrar conversas"
            className={`${entrada} w-full pl-8`}
          />
        </div>
        {(extracoes.data?.length ?? 0) > 1 && (
          <select
            aria-label="Extração"
            value={extracaoId ?? ""}
            onChange={(e) => {
              setExtracaoId(e.target.value || null);
              setDeslocamento(0);
            }}
            className={`${entrada} w-full`}
          >
            <option value="">Todas as extrações</option>
            {extracoes.data?.map((x) => (
              <option key={x.id} value={x.id}>
                {x.importador_nome} · {x.iniciada_em.slice(0, 10)}
              </option>
            ))}
          </select>
        )}
        {apps.length > 1 && (
          <Filtros
            rotulo="Filtrar por app"
            atual={app}
            mudar={setApp}
            opcoes={[[null, "Todos"] as [string | null, ReactNode], ...apps.map((a): [string | null, ReactNode] => [a, rotuloApp(a)])]}
          />
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {lista.isPending ? (
          <div className="px-3">
            <Carregando />
          </div>
        ) : lista.isError ? (
          <div className="p-3">
            <Aviso>{lista.error.message}</Aviso>
          </div>
        ) : lista.data.total === 0 ? (
          <div className="p-3">
            <Vazio titulo="Nenhuma conversa" icone="conversas">
              Importe uma extração com mensagens para ver as conversas aqui.
            </Vazio>
          </div>
        ) : (
          <ul className="divide-y divide-borda/70" aria-label="Lista de conversas">
            {visiveis.map((c) => {
              const ativa = selecionada === c.id;
              return (
                <li key={c.id}>
                  <Link
                    to={`/casos/${caso.id}/conversas/${c.id}`}
                    aria-current={ativa ? "true" : undefined}
                    className={`flex gap-2.5 px-3 py-2.5 transition-colors ${
                      ativa ? "bg-ouro/10 shadow-[inset_2px_0_0_var(--color-ouro)]" : "hover:bg-sup-2"
                    }`}
                  >
                    <Avatar c={c} />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-baseline justify-between gap-2">
                        <span className={`truncate text-[13px] font-medium ${ativa ? "text-ouro-claro" : "text-texto"}`}>{tituloConversa(c)}</span>
                        <span className="tabular shrink-0 text-[11px] text-texto-3">
                          {c.ultima_ms == null ? "—" : dataHoraMs(c.ultima_ms, caso.fuso_horario).slice(0, 10)}
                        </span>
                      </div>
                      <p className="truncate text-xs text-texto-2">{c.ultima_mensagem ?? "—"}</p>
                      <div className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[11px] text-texto-3">
                        <span className="tabular whitespace-nowrap">{numero(c.total_mensagens)} mensagens · {rotuloApp(c.app)}</span>
                        {eGrupo(c) && <Selo tom="info">Grupo</Selo>}
                        {c.com_anexo > 0 && (
                          <span className="flex items-center gap-0.5" title={`${c.com_anexo} com anexo`}>
                            <Icone nome="midias" className="size-3" />
                            {numero(c.com_anexo)}
                          </span>
                        )}
                      </div>
                    </div>
                  </Link>
                </li>
              );
            })}
            {visiveis.length === 0 && <li className="px-3 py-6 text-center text-xs text-texto-2">Nenhuma conversa com esse filtro.</li>}
          </ul>
        )}
      </div>
      {lista.data && lista.data.total > LIMITE_LISTA && (
        <div className="border-t border-borda px-2">
          <Paginacao total={lista.data.total} deslocamento={deslocamento} limite={LIMITE_LISTA} mudar={setDeslocamento} />
        </div>
      )}
    </section>
  );
}

// ---------------------------------------------------------------- Chat

function Chat({
  caso,
  conversaId,
  destaque,
  tecnica,
  setTecnica,
  contexto,
  setContexto,
}: {
  caso: Caso;
  conversaId: number;
  destaque: number | null;
  tecnica: boolean;
  setTecnica: (v: boolean) => void;
  contexto: boolean;
  setContexto: (v: boolean) => void;
}) {
  const conversa = useConversa(caso.id, conversaId);
  const [deslocamento, setDeslocamento] = useState(0);
  const mensagens = useMensagensConversa(caso.id, conversaId, deslocamento, LIMITE_MENSAGENS);
  const [selecionado, setSelecionado] = useState<number | null>(destaque);
  const [localizar, setLocalizar] = useState("");
  const [achado, setAchado] = useState(0);
  const navegar = useNavigate();

  const termo = localizar.trim().toLowerCase();
  const achados = useMemo(
    () => (termo ? (mensagens.data?.itens ?? []).filter((m) => (m.conteudo ?? "").toLowerCase().includes(termo)).map((m) => m.id) : []),
    [termo, mensagens.data],
  );
  const atual = achados.length ? achados[Math.min(achado, achados.length - 1)]! : null;

  if (conversa.isPending)
    return (
      <div className="rounded-xl border border-borda bg-sup px-4">
        <Carregando />
      </div>
    );
  if (conversa.isError) return <Aviso>{conversa.error.message}</Aviso>;
  const c = conversa.data;

  const botaoModo = (valor: boolean, rotulo: string) => (
    <button
      key={rotulo}
      type="button"
      aria-pressed={tecnica === valor}
      onClick={() => setTecnica(valor)}
      className={`px-2.5 py-1 text-xs font-medium first:rounded-l-md last:rounded-r-md ${
        tecnica === valor ? "bg-ouro/15 text-ouro-claro" : "text-texto-2 hover:bg-sup-2 hover:text-texto"
      }`}
    >
      {rotulo}
    </button>
  );

  return (
    <>
      <section aria-label="Chat" className="flex min-h-0 min-w-0 flex-col rounded-xl border border-borda bg-sup">
        <div className="flex flex-wrap items-center gap-3 border-b border-borda px-3.5 py-2.5">
          <Avatar c={c} grande />
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-[14px] font-semibold text-texto">{tituloConversa(c)}</h2>
            <p className="tabular truncate text-xs text-texto-2">
              {rotuloApp(c.app)} · {numero(c.total_mensagens)} mensagens ({numero(c.enviadas)} enviadas) ·{" "}
              {dataHoraMs(c.primeira_ms, caso.fuso_horario)} a {dataHoraMs(c.ultima_ms, caso.fuso_horario)}
            </p>
          </div>
          <div role="group" aria-label="Modo de visualização" className="flex rounded-md ring-1 ring-borda-2">
            {botaoModo(false, "Visão normal")}
            {botaoModo(true, "Detalhes técnicos")}
          </div>
          {!tecnica && (
            <button
              type="button"
              onClick={() => setContexto(!contexto)}
              aria-pressed={contexto}
              title={contexto ? "Ocultar o painel de contexto" : "Mostrar o painel de contexto"}
              className="hidden rounded-md p-1.5 text-texto-2 ring-1 ring-borda-2 hover:bg-sup-2 hover:text-texto xl:block"
            >
              <Icone nome="info" className="size-4" />
              <span className="sr-only">Painel de contexto</span>
            </button>
          )}
        </div>
        <div className="flex items-center gap-2 border-b border-borda px-3.5 py-1.5">
          <Icone nome="busca" className="size-3.5 text-texto-3" />
          <input
            type="search"
            value={localizar}
            onChange={(e) => {
              setLocalizar(e.target.value);
              setAchado(0);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && achados.length) setAchado((achado + (e.shiftKey ? achados.length - 1 : 1)) % achados.length);
            }}
            placeholder="Localizar nesta página da conversa (Enter: próximo)"
            aria-label="Localizar na conversa"
            className="min-w-0 flex-1 bg-transparent text-xs text-texto outline-none placeholder:text-texto-3"
          />
          {termo && (
            <span className="tabular text-[11px] text-texto-3" role="status">
              {achados.length ? `${Math.min(achado, achados.length - 1) + 1} de ${achados.length}` : "nenhuma"}
            </span>
          )}
        </div>
        {mensagens.isPending ? (
          <div className="px-4">
            <Carregando />
          </div>
        ) : mensagens.isError ? (
          <div className="p-4">
            <Aviso>{mensagens.error.message}</Aviso>
          </div>
        ) : (
          <>
            {mensagens.data.total > LIMITE_MENSAGENS && (
              <div className="border-b border-borda px-3">
                <Paginacao total={mensagens.data.total} deslocamento={deslocamento} limite={LIMITE_MENSAGENS} mudar={setDeslocamento} />
              </div>
            )}
            <Mensagens
              caso={caso}
              grupo={eGrupo(c)}
              nomes={nomesDe(c)}
              itens={mensagens.data.itens}
              selecionado={selecionado}
              destaque={destaque}
              termo={termo}
              achadoAtual={atual}
              selecionar={(id) => {
                setSelecionado(id);
                setTecnica(true);
                if (destaque != null) navegar(`/casos/${caso.id}/conversas/${conversaId}`, { replace: true });
              }}
            />
          </>
        )}
      </section>
      {tecnica ? (
        <PainelTecnico caso={caso} artefatoId={selecionado} className="min-h-0 overflow-y-auto" />
      ) : (
        contexto && (
          <PainelContexto
            caso={caso}
            c={c}
            selecionada={mensagens.data?.itens.find((m) => m.id === selecionado) ?? null}
            abrirOrigem={() => setTecnica(true)}
            abrirRegistro={(id) => {
              setSelecionado(id);
              setTecnica(true);
            }}
          />
        )
      )}
    </>
  );
}

/** Texto com as ocorrências de `termo` marcadas (localizar na conversa). */
function ComDestaque({ texto, termo }: { texto: string; termo: string }) {
  if (!termo) return <>{texto}</>;
  const partes: ReactNode[] = [];
  const baixo = texto.toLowerCase();
  let i = 0;
  let k = 0;
  while (i < texto.length) {
    const j = baixo.indexOf(termo, i);
    if (j < 0) {
      partes.push(texto.slice(i));
      break;
    }
    if (j > i) partes.push(texto.slice(i, j));
    partes.push(
      <mark key={k++} className="rounded-sm bg-ouro/35 px-0.5 text-texto">
        {texto.slice(j, j + termo.length)}
      </mark>,
    );
    i = j + termo.length;
  }
  return <>{partes}</>;
}

function Mensagens({
  caso,
  grupo,
  nomes,
  itens,
  selecionado,
  destaque,
  termo,
  achadoAtual,
  selecionar,
}: {
  caso: Caso;
  grupo: boolean;
  nomes: Map<string, string>;
  itens: ArtefatoResumo[];
  selecionado: number | null;
  destaque: number | null;
  termo: string;
  achadoAtual: number | null;
  selecionar: (id: number) => void;
}) {
  const alvo = useRef<HTMLDivElement>(null);
  const rolarPara = achadoAtual ?? destaque;
  useEffect(() => {
    alvo.current?.scrollIntoView?.({ block: "center" });
  }, [rolarPara, itens]);

  const porIdExterno = new Map(itens.flatMap((m) => (m.id_externo ? [[m.id_externo, m] as const] : [])));
  let diaAnterior: string | null = null;
  return (
    <div className="min-h-0 flex-1 space-y-1.5 overflow-y-auto px-4 py-3" aria-label="Mensagens da conversa">
      {itens.length === 0 && <p className="py-6 text-center text-[13px] text-texto-2">Nenhuma mensagem nesta página.</p>}
      {itens.map((m) => {
        const d = m.data_hora_ms == null ? "Sem data" : dia(m.data_hora_ms, caso.fuso_horario);
        const separador = d !== diaAnterior;
        diaAnterior = d;
        const enviada = m.de_mim === true;
        const ativo = selecionado === m.id;
        const daBusca = destaque === m.id;
        const achado = termo !== "" && (m.conteudo ?? "").toLowerCase().includes(termo);
        return (
          <Fragment key={m.id}>
            {separador && (
              <div className="sticky top-0 z-10 py-2 text-center">
                <span className="rounded-full border border-borda bg-sup-2 px-3 py-0.5 text-[11px] text-texto-2 shadow-sm">{d}</span>
              </div>
            )}
            {m.subtipo === "sistema" ? (
              <div ref={m.id === rolarPara ? alvo : undefined} className="render-preguicoso flex justify-center">
                <button
                  type="button"
                  onClick={() => selecionar(m.id)}
                  aria-pressed={ativo}
                  data-teste="mensagem"
                  className={`max-w-[min(85%,40rem)] rounded-full border border-borda bg-fundo/60 px-3 py-1 text-center text-[12px] text-texto-2 hover:bg-sup-2 ${
                    ativo ? "outline-2 outline-offset-2 outline-ouro" : ""
                  } ${daBusca || (achado && m.id === achadoAtual) ? "ring-2 ring-ouro/70" : ""}`}
                >
                  {m.conteudo != null ? <ComDestaque texto={m.conteudo} termo={termo} /> : rotuloSubtipo("sistema")}
                  <span className="tabular ml-2 text-[11px] text-texto-3">{m.data_hora_ms == null ? "—" : hora(m.data_hora_ms, caso.fuso_horario)}</span>
                </button>
              </div>
            ) : (
              <div
                ref={m.id === rolarPara ? alvo : undefined}
                className={`render-preguicoso flex ${enviada ? "justify-end" : "justify-start"}`}
              >
                <button
                  type="button"
                  onClick={() => selecionar(m.id)}
                  aria-pressed={ativo}
                  data-teste="mensagem"
                  className={`max-w-[min(75%,40rem)] rounded-2xl border px-3 py-2 text-left text-[13px] transition-colors ${
                    enviada ? "rounded-br-sm border-info/25 bg-info/12 hover:bg-info/18" : "rounded-bl-sm border-borda-2 bg-sup-2 hover:bg-sup-3"
                  } ${ativo ? "outline-2 outline-offset-2 outline-ouro" : ""} ${daBusca || (achado && m.id === achadoAtual) ? "ring-2 ring-ouro/70" : achado ? "ring-1 ring-ouro/35" : ""}`}
                >
                  {!enviada && m.remetente && (
                    <p className="mb-0.5 flex min-w-0 items-baseline gap-1.5 text-[11px]">
                      {nomes.has(m.remetente) && (
                        <span className={`truncate font-medium ${grupo ? "text-ouro-claro" : "text-texto-2"}`}>{nomes.get(m.remetente)}</span>
                      )}
                      <span className={`truncate font-mono ${grupo && !nomes.has(m.remetente) ? "text-ouro-claro" : "text-texto-3"}`}>{m.remetente}</span>
                    </p>
                  )}
                  {m.subtipo && m.subtipo !== "texto" && m.subtipo !== m.app && (
                    <p className="mb-1">
                      <Selo tom={m.subtipo === "apagada" || m.subtipo === "desconhecido" ? "atencao" : "info"}>{rotuloSubtipo(m.subtipo)}</Selo>
                    </p>
                  )}
                  {m.resposta_a_id_externo && <Citacao alvo={porIdExterno.get(m.resposta_a_id_externo)} nomes={nomes} />}
                  {temAnexo(m) && <Anexo m={m} />}
                  {temLocal(m) && (
                    <p className="mb-1 flex items-center gap-1 font-mono text-xs text-texto-2" title="Latitude, longitude">
                      <Icone nome="localizacoes" className="size-3.5 shrink-0 text-texto-3" />
                      {coordenadas(m)}
                    </p>
                  )}
                  {(m.conteudo != null || !(temAnexo(m) || temLocal(m))) && (
                    <p className="whitespace-pre-wrap break-words text-texto">
                      {m.conteudo != null ? (
                        <ComDestaque texto={m.conteudo} termo={termo} />
                      ) : (
                        <span className="italic text-texto-3">{m.subtipo === "apagada" ? "(mensagem apagada: o texto não está no banco)" : "(sem texto)"}</span>
                      )}
                    </p>
                  )}
                  <p className="mt-1 flex items-center justify-end gap-2 text-[11px] text-texto-3">
                    {daBusca && <span className="text-ouro-claro">resultado da busca</span>}
                    {m.estado_registro !== "ativo" && <Selo tom="atencao">{nomeEstadoRegistro[m.estado_registro] ?? m.estado_registro}</Selo>}
                    <span className="tabular">{m.data_hora_ms == null ? "—" : hora(m.data_hora_ms, caso.fuso_horario)}</span>
                  </p>
                </button>
              </div>
            )}
          </Fragment>
        );
      })}
    </div>
  );
}

/** Trecho da mensagem respondida, quando ela está nesta página da conversa. */
function Citacao({ alvo, nomes }: { alvo: ArtefatoResumo | undefined; nomes: Map<string, string> }) {
  return (
    <div className="mb-1.5 rounded-md border-l-2 border-ouro/60 bg-fundo/50 px-2 py-1 text-xs" aria-label="Mensagem respondida">
      {alvo ? (
        <>
          <p className="font-medium text-texto-2">
            {alvo.de_mim ? "Mensagem enviada" : alvo.remetente ? (nomes.get(alvo.remetente) ?? alvo.remetente) : "Mensagem recebida"}
          </p>
          <p className="line-clamp-2 text-texto-3">{alvo.conteudo ?? (alvo.nome_arquivo ? alvo.nome_arquivo : alvo.subtipo ? rotuloSubtipo(alvo.subtipo) : "—")}</p>
        </>
      ) : (
        <p className="text-texto-3">Resposta a uma mensagem que não está nesta página.</p>
      )}
    </div>
  );
}

/** Arquivo anexado, com o que se sabe dele no app e se foi achado na extração. */
function Anexo({ m }: { m: ArtefatoResumo }) {
  const s = situacaoAnexo(m);
  return (
    <div className="mb-1.5 min-w-[14rem] rounded-md border border-borda bg-fundo/50 px-2 py-1.5" aria-label="Anexo">
      <div className="flex items-center gap-2">
        <Icone nome={iconeAnexo(m.subtipo)} className="size-4 shrink-0 text-texto-3" />
        <div className="min-w-0 flex-1">
          <p className="truncate text-xs text-texto" title={m.nome_arquivo ?? undefined}>
            {m.nome_arquivo ?? "Arquivo sem nome"}
          </p>
          <p className="tabular truncate text-[11px] text-texto-3">
            {[m.mime, m.tamanho != null ? tamanho(m.tamanho) : null].filter(Boolean).join(" · ") || "Tipo e tamanho não informados"}
          </p>
        </div>
      </div>
      <p className="mt-1">
        <Selo tom={s.tom}>{s.texto}</Selo>
      </p>
    </div>
  );
}

// ---------------------------------------------------------------- Contexto

function PainelContexto({ caso, c, selecionada, abrirOrigem, abrirRegistro }: {
  caso: Caso;
  c: Conversa;
  selecionada: ArtefatoResumo | null;
  abrirOrigem: () => void;
  abrirRegistro: (id: number) => void;
}) {
  const comNomes = c.participantes.some((p) => p.nome);
  const fuso = caso.fuso_horario;
  const par = (rotulo: string, valor: ReactNode, mono = false) => (
    <div className="flex items-baseline justify-between gap-3 py-0.5 text-xs">
      <dt className="shrink-0 text-texto-3">{rotulo}</dt>
      <dd className={`min-w-0 truncate text-right text-texto ${mono ? "font-mono" : "tabular"}`}>{valor}</dd>
    </div>
  );
  return (
    <aside aria-label="Contexto da conversa" className="hidden min-h-0 flex-col overflow-y-auto rounded-xl border border-borda bg-sup xl:flex">
      <p className="border-b border-borda px-4 py-2.5 text-[13px] font-semibold text-texto">Contexto da conversa</p>
      <SecaoRecolhivel titulo="Conversa" icone="conversas">
        <dl>
          {par("App", rotuloApp(c.app))}
          {c.titulo && par("Título", c.titulo)}
          {par("Tipo", rotuloTipoConversa(c))}
          {par("ID na origem", c.id_externo, true)}
          {par("Mensagens", numero(c.total_mensagens))}
          {par("Enviadas", numero(c.enviadas))}
          {par("Recebidas", numero(c.total_mensagens - c.enviadas))}
          {par("Com anexo", numero(c.com_anexo))}
          {par("Primeira", dataHoraMs(c.primeira_ms, fuso))}
          {par("Última", dataHoraMs(c.ultima_ms, fuso))}
        </dl>
        <Link to={`/casos/${caso.id}/extracoes/${c.extracao_id}`} className="mt-2 inline-flex items-center gap-1 text-xs text-ouro-claro hover:underline">
          <Icone nome="extracao" className="size-3.5" />
          Ver extração de origem
        </Link>
        {c.registro_id != null && (
          <button
            type="button"
            onClick={() => abrirRegistro(c.registro_id!)}
            className="mt-1 flex items-center gap-1 text-xs text-ouro-claro hover:underline"
          >
            <Icone nome="banco" className="size-3.5" />
            Abrir registro da conversa no app
          </button>
        )}
      </SecaoRecolhivel>
      <SecaoRecolhivel titulo={`Participantes (${c.total_participantes})`} icone="contatos">
        <ul className="space-y-1 text-xs">
          {c.participantes.map((p) => (
            <li key={p.identificador} className="flex items-baseline justify-between gap-2">
              <span className="min-w-0">
                {p.nome && <span className="block truncate text-texto">{p.nome}</span>}
                <span className={`block truncate font-mono ${p.nome ? "text-[11px] text-texto-3" : "text-texto"}`} title={p.identificador}>
                  {p.identificador}
                </span>
              </span>
              <span className="tabular shrink-0 text-texto-3">{numero(p.mensagens)} msg</span>
            </li>
          ))}
        </ul>
        <p className="mt-2 text-[11px] text-texto-3">
          {comNomes && "Nomes como gravados nos contatos do próprio app nesta extração. "}
          Identificadores exatamente como o parser leu; o SIED não deduz a quem pertencem.
        </p>
      </SecaoRecolhivel>
      <SecaoRecolhivel titulo="Mensagem selecionada" icone="info">
        {selecionada ? (
          <>
            <dl>
              {par("Registro nº", selecionada.id)}
              {par("Data", dataHoraMs(selecionada.data_hora_ms, fuso))}
              {par("Direção", selecionada.de_mim == null ? "—" : selecionada.de_mim ? "Enviada" : "Recebida")}
              {selecionada.subtipo && par("Tipo", rotuloSubtipo(selecionada.subtipo))}
              {par("Estado", nomeEstadoRegistro[selecionada.estado_registro] ?? selecionada.estado_registro)}
              {temLocal(selecionada) && par("Coordenadas", coordenadas(selecionada), true)}
            </dl>
            {temAnexo(selecionada) && (
              <p className="mt-2 text-xs text-texto-2">
                <span className="text-texto-3">Anexo: </span>
                {selecionada.anexo_arquivo_id != null
                  ? (nomeVinculoAnexo[selecionada.anexo_vinculo ?? "caminho_exato"] ?? "Arquivo na extração")
                  : selecionada.anexo_vinculo
                    ? (nomeVinculoAnexo[selecionada.anexo_vinculo] ?? selecionada.anexo_vinculo)
                    : "caminho não informado pelo app"}
              </p>
            )}
            <button type="button" onClick={abrirOrigem} className="mt-2 inline-flex items-center gap-1 text-xs text-ouro-claro hover:underline">
              <Icone nome="banco" className="size-3.5" />
              Abrir origem técnica
            </button>
          </>
        ) : (
          <p className="text-xs text-texto-2">Clique numa mensagem para ver de onde ela veio.</p>
        )}
      </SecaoRecolhivel>
    </aside>
  );
}
