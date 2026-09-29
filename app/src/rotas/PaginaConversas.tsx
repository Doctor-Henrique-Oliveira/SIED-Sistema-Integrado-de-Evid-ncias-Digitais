// Conversas em três colunas: lista (esquerda), chat (centro) e contexto ou origem
// técnica (direita). As conversas são montadas a partir das mensagens gravadas;
// nada aqui altera registros. Título, tipo e nomes vêm dos registros que o próprio
// app gravou na mesma extração (ex.: assunto do grupo e agenda do WhatsApp).

import { useVirtualizer } from "@tanstack/react-virtual";
import { Fragment, useEffect, useMemo, useRef, useState, type ReactNode, type RefObject } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router-dom";
import { api, type ArtefatoResumo, type Caso, type Conversa, type FiltroConversas, type Pagina } from "../api/cliente";
import {
  chaves,
  useAppsDasConversas,
  useConversa,
  useConversas,
  useExtracoes,
  useLocalizarNaConversa,
  useMensagensConversa,
  usePosicaoMensagem,
} from "../api/consultas";
import { Aviso, Cabecalho, Carregando, entrada, Filtros, LinkBotao, SecaoRecolhivel, Selo, Vazio } from "../componentes/Estrutura";
import { Icone } from "../componentes/Icone";
import { MiniaturaArquivo } from "../componentes/Miniatura";
import { BarraEvidencia, MarcasContexto, SeloEvidencia, useMarcas, useMarcasContexto } from "../componentes/Evidencias";
import { PainelTecnico } from "../componentes/PainelTecnico";
import { useListaPaginada } from "../util/listaPaginada";
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

/** Valor que só muda depois de `ms` sem alteração (evita uma consulta por tecla). */
function useAdiado<T>(valor: T, ms: number): T {
  const [adiado, setAdiado] = useState(valor);
  useEffect(() => {
    const t = setTimeout(() => setAdiado(valor), ms);
    return () => clearTimeout(t);
  }, [valor, ms]);
  return adiado;
}

function ListaConversas({ caso, selecionada }: { caso: Caso; selecionada: number | null }) {
  const [extracaoId, setExtracaoId] = useState<string | null>(null);
  const [texto, setTexto] = useState("");
  const [app, setApp] = useState<string | null>(null);
  const [tipo, setTipo] = useState<string | null>(null);
  const [comAnexo, setComAnexo] = useState(false);
  const termo = useAdiado(texto.trim(), 250);
  const filtro: FiltroConversas = {
    extracao_id: extracaoId ?? undefined,
    texto: termo || undefined,
    app: app ?? undefined,
    tipo: tipo ?? undefined,
    com_anexo: comAnexo || undefined,
  };
  const filtrando = !!(termo || app || tipo || comAnexo);
  const extracoes = useExtracoes(caso.id);
  const apps = useAppsDasConversas(caso.id, extracaoId);
  const lista = useConversas(caso.id, filtro, 0, LIMITE_LISTA);
  const rolagem = useRef<HTMLDivElement>(null);

  function limpar() {
    setTexto("");
    setApp(null);
    setTipo(null);
    setComAnexo(false);
  }

  const totalApps = apps.data?.reduce((n, a) => n + a.conversas, 0) ?? 0;
  return (
    <section aria-label="Conversas do caso" className="flex min-h-0 w-full flex-col rounded-xl border border-borda bg-sup">
      <div className="space-y-2 border-b border-borda p-2.5">
        <div className="relative">
          <Icone nome="busca" className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-texto-3" />
          <input
            type="search"
            value={texto}
            onChange={(e) => setTexto(e.target.value)}
            placeholder="Nome, número ou título"
            title="Filtra no caso inteiro: título, participante, nome nos contatos do app ou telefone (com ou sem máscara)"
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
              setApp(null);
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
        {(apps.data?.length ?? 0) > 1 && (
          <Filtros
            rotulo="Filtrar por app"
            atual={app}
            mudar={setApp}
            opcoes={[
              [null, `Todos (${numero(totalApps)})`] as [string | null, ReactNode],
              ...apps.data!.map((a): [string | null, ReactNode] => [a.app, `${rotuloApp(a.app)} (${numero(a.conversas)})`]),
            ]}
          />
        )}
        <div className="flex flex-wrap items-center gap-1.5">
          <Filtros
            rotulo="Filtrar por tipo de conversa"
            atual={tipo}
            mudar={setTipo}
            opcoes={[
              [null, "Todas"],
              ["individual", "Individuais"],
              ["grupo", "Grupos"],
            ]}
          />
          <button
            type="button"
            aria-pressed={comAnexo}
            onClick={() => setComAnexo(!comAnexo)}
            className={`flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-medium ring-1 ring-inset transition-colors ${
              comAnexo ? "bg-ouro/15 text-ouro-claro ring-ouro/40" : "text-texto-2 ring-borda-2 hover:bg-sup-2 hover:text-texto"
            }`}
          >
            <Icone nome="midias" className="size-3" />
            Com anexo
          </button>
        </div>
        {lista.data && (
          <p className="tabular flex items-center justify-between text-[11px] text-texto-3" role="status">
            <span>
              {numero(lista.data.total)} {lista.data.total === 1 ? "conversa" : "conversas"}
              {filtrando ? " com os filtros" : ""}
              {lista.isFetching && lista.isPlaceholderData ? " · atualizando…" : ""}
            </span>
            {filtrando && (
              <button type="button" onClick={limpar} className="text-ouro-claro hover:underline">
                Limpar filtros
              </button>
            )}
          </p>
        )}
      </div>
      <div ref={rolagem} className="min-h-0 flex-1 overflow-y-auto">
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
            {filtrando ? (
              <p className="px-3 py-6 text-center text-xs text-texto-2">Nenhuma conversa com esses filtros.</p>
            ) : (
              <Vazio titulo="Nenhuma conversa" icone="conversas">
                Importe uma extração com mensagens para ver as conversas aqui.
              </Vazio>
            )}
          </div>
        ) : lista.data.total <= LIMITE_LISTA ? (
          <ul className="divide-y divide-borda/70" aria-label="Lista de conversas">
            {lista.data.itens.map((c) => (
              <li key={c.id}>
                <ItemConversa caso={caso} c={c} ativa={selecionada === c.id} />
              </li>
            ))}
          </ul>
        ) : (
          <ListaConversasLonga caso={caso} filtro={filtro} primeira={lista.data} selecionada={selecionada} rolagem={rolagem} />
        )}
      </div>
    </section>
  );
}

function ItemConversa({ caso, c, ativa }: { caso: Caso; c: Conversa; ativa: boolean }) {
  return (
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
  );
}

/** Mais conversas do que cabem numa página: só as linhas visíveis existem na tela e
 * as páginas seguintes vêm do banco conforme a rolagem. */
function ListaConversasLonga({ caso, filtro, primeira, selecionada, rolagem }: {
  caso: Caso;
  filtro: FiltroConversas;
  primeira: Pagina<Conversa>;
  selecionada: number | null;
  rolagem: RefObject<HTMLDivElement | null>;
}) {
  const virtual = useVirtualizer({
    count: primeira.total,
    getScrollElement: () => rolagem.current,
    estimateSize: () => 78,
    overscan: 8,
  });
  const itens = virtual.getVirtualItems();
  const { item } = useListaPaginada({
    chave: [...chaves.conversas(caso.id), "lista", filtro],
    primeira,
    tamanho: LIMITE_LISTA,
    buscar: (d, l) => api.listarConversas(filtro, d, l),
    indices: itens.map((v) => v.index),
  });
  return (
    <ul aria-label="Lista de conversas" className="relative w-full" style={{ height: virtual.getTotalSize() }}>
      {itens.map((v) => {
        const c = item(v.index);
        return (
          <li
            key={v.key}
            data-index={v.index}
            ref={virtual.measureElement}
            className="absolute inset-x-0 border-b border-borda/70"
            style={{ transform: `translateY(${v.start}px)` }}
          >
            {c ? (
              <ItemConversa caso={caso} c={c} ativa={selecionada === c.id} />
            ) : (
              <div className="flex h-[77px] animate-pulse items-center gap-2.5 px-3" aria-hidden>
                <span className="size-9 rounded-full bg-sup-3" />
                <span className="h-3 flex-1 rounded bg-sup-3" />
              </div>
            )}
          </li>
        );
      })}
    </ul>
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
  const mensagens = useMensagensConversa(caso.id, conversaId, 0, LIMITE_MENSAGENS);
  const [selecionado, setSelecionado] = useState<number | null>(destaque);
  const [localizar, setLocalizar] = useState("");
  const [achado, setAchado] = useState(0);
  const [carregados, setCarregados] = useState<ArtefatoResumo[]>([]);
  const [inicioTrecho, setInicioTrecho] = useState<ArtefatoResumo | null>(null);
  const marcas = useMarcas(caso.id);
  const navegar = useNavigate();

  // Conversa longa: as mensagens vêm por página conforme a rolagem, e localizar e
  // "resultado da busca" perguntam ao banco a posição de cada mensagem.
  const longa = (mensagens.data?.total ?? 0) > LIMITE_MENSAGENS;
  const termo = localizar.trim().toLowerCase();
  const termoAdiado = useAdiado(termo, 300);
  const noBanco = useLocalizarNaConversa(caso.id, conversaId, termoAdiado, longa);
  const posicaoDestaque = usePosicaoMensagem(caso.id, conversaId, destaque, longa);

  const achadosLocais = useMemo(
    () => (!longa && termo ? (mensagens.data?.itens ?? []).filter((m) => (m.conteudo ?? "").toLowerCase().includes(termo)).map((m) => m.id) : []),
    [longa, termo, mensagens.data],
  );
  const achadosBanco = longa && termo ? (noBanco.data?.itens ?? []) : [];
  const nAchados = longa ? achadosBanco.length : achadosLocais.length;
  const indiceAchado = nAchados ? Math.min(achado, nAchados - 1) : 0;
  const atual = longa ? (achadosBanco[indiceAchado]?.id ?? null) : (achadosLocais[indiceAchado] ?? null);
  const posicaoAtual = longa ? (achadosBanco[indiceAchado]?.posicao ?? null) : null;
  const totalAchados = longa ? (noBanco.data?.total ?? 0) : achadosLocais.length;

  if (conversa.isPending)
    return (
      <div className="rounded-xl border border-borda bg-sup px-4">
        <Carregando />
      </div>
    );
  if (conversa.isError) return <Aviso>{conversa.error.message}</Aviso>;
  const c = conversa.data;
  const lista = longa ? carregados : (mensagens.data?.itens ?? []);

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

  const selecionar = (id: number) => {
    setSelecionado(id);
    setTecnica(true);
    if (destaque != null) navegar(`/casos/${caso.id}/conversas/${conversaId}`, { replace: true });
  };

  const mensagemSelecionada = lista.find((m) => m.id === selecionado) ?? null;

  return (
    <MarcasContexto.Provider value={marcas}>
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
              if (e.key === "Enter" && nAchados) setAchado((indiceAchado + (e.shiftKey ? nAchados - 1 : 1)) % nAchados);
            }}
            placeholder="Localizar nesta conversa (Enter: próximo, Shift+Enter: anterior)"
            aria-label="Localizar na conversa"
            className="min-w-0 flex-1 bg-transparent text-xs text-texto outline-none placeholder:text-texto-3"
          />
          {termo && (
            <span className="tabular text-[11px] text-texto-3" role="status">
              {longa && (termo !== termoAdiado || noBanco.isFetching)
                ? "procurando…"
                : nAchados
                  ? `${indiceAchado + 1} de ${numero(totalAchados)}${totalAchados > nAchados ? ` (primeiras ${numero(nAchados)})` : ""}`
                  : "nenhuma"}
            </span>
          )}
        </div>
        <BarraEvidencia
          key={selecionado ?? 0}
          caso={caso}
          selecionada={mensagemSelecionada}
          inicio={inicioTrecho}
          setInicio={setInicioTrecho}
        />
        {mensagens.isPending ? (
          <div className="px-4">
            <Carregando />
          </div>
        ) : mensagens.isError ? (
          <div className="p-4">
            <Aviso>{mensagens.error.message}</Aviso>
          </div>
        ) : longa ? (
          <MensagensLongas
            caso={caso}
            conversaId={conversaId}
            grupo={eGrupo(c)}
            nomes={nomesDe(c)}
            primeira={mensagens.data}
            selecionado={selecionado}
            destaque={destaque}
            termo={termo}
            achadoAtual={atual}
            rolarPara={posicaoAtual ?? posicaoDestaque.data ?? null}
            aoCarregar={setCarregados}
            selecionar={selecionar}
          />
        ) : (
          <Mensagens
            caso={caso}
            grupo={eGrupo(c)}
            nomes={nomesDe(c)}
            itens={mensagens.data.itens}
            selecionado={selecionado}
            destaque={destaque}
            termo={termo}
            achadoAtual={atual}
            selecionar={selecionar}
          />
        )}
      </section>
      {tecnica ? (
        <PainelTecnico caso={caso} artefatoId={selecionado} className="min-h-0 overflow-y-auto" />
      ) : (
        contexto && (
          <PainelContexto
            caso={caso}
            c={c}
            selecionada={mensagemSelecionada}
            abrirOrigem={() => setTecnica(true)}
            abrirRegistro={(id) => {
              setSelecionado(id);
              setTecnica(true);
            }}
          />
        )
      )}
    </MarcasContexto.Provider>
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

const diaDe = (m: ArtefatoResumo, fuso: string) => (m.data_hora_ms == null ? "Sem data" : dia(m.data_hora_ms, fuso));

function SeparadorDia({ d, fixo }: { d: string; fixo?: boolean }) {
  return (
    <div className={`${fixo ? "sticky top-0 z-10" : ""} py-2 text-center`}>
      <span className="rounded-full border border-borda bg-sup-2 px-3 py-0.5 text-[11px] text-texto-2 shadow-sm">{d}</span>
    </div>
  );
}

/** Uma mensagem: bolha à esquerda (recebida) ou à direita (enviada), ou a linha
 * centralizada de uma mensagem de sistema. */
function Bolha({
  caso,
  m,
  grupo,
  nomes,
  respondida,
  semResposta,
  ativo,
  daBusca,
  termo,
  atual,
  alvo,
  preguicoso,
  selecionar,
}: {
  caso: Caso;
  m: ArtefatoResumo;
  grupo: boolean;
  nomes: Map<string, string>;
  respondida: ArtefatoResumo | undefined;
  semResposta: string;
  ativo: boolean;
  daBusca: boolean;
  termo: string;
  atual: boolean;
  alvo?: RefObject<HTMLDivElement | null>;
  preguicoso: boolean;
  selecionar: (id: number) => void;
}) {
  const enviada = m.de_mim === true;
  const evidencias = useMarcasContexto().registros.get(m.id);
  const achado = termo !== "" && (m.conteudo ?? "").toLowerCase().includes(termo);
  const lazy = preguicoso ? "render-preguicoso " : "";
  if (m.subtipo === "sistema")
    return (
      <div ref={alvo} className={`${lazy}flex justify-center`}>
        <button
          type="button"
          onClick={() => selecionar(m.id)}
          aria-pressed={ativo}
          data-teste="mensagem"
          className={`max-w-[min(85%,40rem)] rounded-full border border-borda bg-fundo/60 px-3 py-1 text-center text-[12px] text-texto-2 hover:bg-sup-2 ${
            ativo ? "outline-2 outline-offset-2 outline-ouro" : ""
          } ${daBusca || (achado && atual) ? "ring-2 ring-ouro/70" : ""}`}
        >
          {m.conteudo != null ? <ComDestaque texto={m.conteudo} termo={termo} /> : rotuloSubtipo("sistema")}
          <span className="tabular ml-2 text-[11px] text-texto-3">{m.data_hora_ms == null ? "—" : hora(m.data_hora_ms, caso.fuso_horario)}</span>
          {evidencias && (
            <span className="ml-2">
              <SeloEvidencia numeros={evidencias} />
            </span>
          )}
        </button>
      </div>
    );
  return (
    <div ref={alvo} className={`${lazy}flex ${enviada ? "justify-end" : "justify-start"}`}>
      <button
        type="button"
        onClick={() => selecionar(m.id)}
        aria-pressed={ativo}
        data-teste="mensagem"
        className={`max-w-[min(75%,40rem)] rounded-2xl border px-3 py-2 text-left text-[13px] transition-colors ${
          enviada ? "rounded-br-sm border-info/25 bg-info/12 hover:bg-info/18" : "rounded-bl-sm border-borda-2 bg-sup-2 hover:bg-sup-3"
        } ${ativo ? "outline-2 outline-offset-2 outline-ouro" : ""} ${daBusca || (achado && atual) ? "ring-2 ring-ouro/70" : achado ? "ring-1 ring-ouro/35" : ""}`}
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
        {m.resposta_a_id_externo && <Citacao alvo={respondida} nomes={nomes} ausente={semResposta} />}
        {temAnexo(m) && <Anexo caso={caso} m={m} />}
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
          <SeloEvidencia numeros={evidencias} />
          {m.estado_registro !== "ativo" && <Selo tom="atencao">{nomeEstadoRegistro[m.estado_registro] ?? m.estado_registro}</Selo>}
          <span className="tabular">{m.data_hora_ms == null ? "—" : hora(m.data_hora_ms, caso.fuso_horario)}</span>
        </p>
      </button>
    </div>
  );
}

/** Conversa que cabe numa página: todas as mensagens na tela. */
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
      {itens.length === 0 && <p className="py-6 text-center text-[13px] text-texto-2">Nenhuma mensagem nesta conversa.</p>}
      {itens.map((m) => {
        const d = diaDe(m, caso.fuso_horario);
        const separador = d !== diaAnterior;
        diaAnterior = d;
        return (
          <Fragment key={m.id}>
            {separador && <SeparadorDia d={d} fixo />}
            <Bolha
              caso={caso}
              m={m}
              grupo={grupo}
              nomes={nomes}
              respondida={m.resposta_a_id_externo ? porIdExterno.get(m.resposta_a_id_externo) : undefined}
              semResposta="A mensagem respondida não está entre os registros desta conversa."
              ativo={selecionado === m.id}
              daBusca={destaque === m.id}
              termo={termo}
              atual={m.id === achadoAtual}
              alvo={m.id === rolarPara ? alvo : undefined}
              preguicoso
              selecionar={selecionar}
            />
          </Fragment>
        );
      })}
    </div>
  );
}

/**
 * Conversa longa (milhares de mensagens): só as mensagens perto da área visível
 * existem na tela; as páginas são pedidas ao banco conforme a rolagem. A altura de
 * cada mensagem é medida quando ela aparece.
 */
function MensagensLongas({
  caso,
  conversaId,
  grupo,
  nomes,
  primeira,
  selecionado,
  destaque,
  termo,
  achadoAtual,
  rolarPara,
  aoCarregar,
  selecionar,
}: {
  caso: Caso;
  conversaId: number;
  grupo: boolean;
  nomes: Map<string, string>;
  primeira: Pagina<ArtefatoResumo>;
  selecionado: number | null;
  destaque: number | null;
  termo: string;
  achadoAtual: number | null;
  /** Posição (na ordem da conversa) a trazer para o centro. */
  rolarPara: number | null;
  aoCarregar: (itens: ArtefatoResumo[]) => void;
  selecionar: (id: number) => void;
}) {
  const rolagem = useRef<HTMLDivElement>(null);
  const virtual = useVirtualizer({
    count: primeira.total,
    getScrollElement: () => rolagem.current,
    estimateSize: () => 76,
    overscan: 12,
    gap: 6,
  });
  const visiveis = virtual.getVirtualItems();
  const { item, carregados } = useListaPaginada({
    chave: [...chaves.conversas(caso.id), "mensagens", conversaId],
    primeira,
    tamanho: LIMITE_MENSAGENS,
    buscar: (d, l) => api.mensagensConversa(conversaId, d, l),
    indices: visiveis.map((v) => v.index),
  });

  const nCarregados = carregados.length;
  useEffect(() => aoCarregar(carregados), [nCarregados]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (rolarPara != null) virtual.scrollToIndex(rolarPara, { align: "center" });
  }, [rolarPara, virtual]);

  const porIdExterno = new Map(carregados.flatMap((m) => (m.id_externo ? [[m.id_externo, m] as const] : [])));
  const primeiraVisivel = visiveis.find((v) => v.start + v.size > virtual.scrollOffset!) ?? visiveis[0];
  // No topo da conversa o próprio separador do primeiro dia já aparece.
  const diaTopo = primeiraVisivel && (virtual.scrollOffset ?? 0) > 8 ? item(primeiraVisivel.index) : undefined;

  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      {diaTopo && (
        <div className="pointer-events-none absolute inset-x-0 top-1 z-10 text-center">
          <span className="rounded-full border border-borda bg-sup-2 px-3 py-0.5 text-[11px] text-texto-2 shadow-sm">{diaDe(diaTopo, caso.fuso_horario)}</span>
        </div>
      )}
      <div ref={rolagem} className="min-h-0 flex-1 overflow-y-auto px-4 py-3" aria-label="Mensagens da conversa">
        <div className="relative w-full" style={{ height: virtual.getTotalSize() }}>
          {visiveis.map((v) => {
            const m = item(v.index);
            const anterior = v.index > 0 ? item(v.index - 1) : undefined;
            const d = m ? diaDe(m, caso.fuso_horario) : null;
            const separador = m && (v.index === 0 || (anterior && diaDe(anterior, caso.fuso_horario) !== d));
            return (
              <div
                key={v.key}
                data-index={v.index}
                ref={virtual.measureElement}
                className="absolute inset-x-0"
                style={{ transform: `translateY(${v.start}px)` }}
              >
                {m ? (
                  <>
                    {separador && <SeparadorDia d={d!} />}
                    <Bolha
                      caso={caso}
                      m={m}
                      grupo={grupo}
                      nomes={nomes}
                      respondida={m.resposta_a_id_externo ? porIdExterno.get(m.resposta_a_id_externo) : undefined}
                      semResposta="A mensagem respondida não está na parte da conversa já carregada."
                      ativo={selecionado === m.id}
                      daBusca={destaque === m.id}
                      termo={termo}
                      atual={m.id === achadoAtual}
                      preguicoso={false}
                      selecionar={selecionar}
                    />
                  </>
                ) : (
                  <div className="h-14 animate-pulse rounded-2xl bg-sup-2/40" aria-hidden />
                )}
              </div>
            );
          })}
        </div>
      </div>
      <div className="flex items-center justify-between gap-2 border-t border-borda px-3.5 py-1 text-[11px] text-texto-3">
        <span className="tabular" role="status">
          {numero(primeira.total)} mensagens · carregadas conforme a rolagem
        </span>
        <span className="flex gap-1">
          <button type="button" onClick={() => virtual.scrollToIndex(0, { align: "start" })} className="rounded px-1.5 py-0.5 hover:bg-sup-2 hover:text-texto">
            Primeira
          </button>
          <button
            type="button"
            onClick={() => virtual.scrollToIndex(primeira.total - 1, { align: "end" })}
            className="rounded px-1.5 py-0.5 hover:bg-sup-2 hover:text-texto"
          >
            Última
          </button>
        </span>
      </div>
    </div>
  );
}

/** Trecho da mensagem respondida, quando ela está entre as mensagens carregadas. */
function Citacao({ alvo, nomes, ausente }: { alvo: ArtefatoResumo | undefined; nomes: Map<string, string>; ausente: string }) {
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
        <p className="text-texto-3">{ausente}</p>
      )}
    </div>
  );
}

/** Arquivo anexado, com o que se sabe dele no app e se foi achado na extração. Foto
 * ligada ao inventário mostra a miniatura (gerada do arquivo conferido pelo hash). */
function Anexo({ caso, m }: { caso: Caso; m: ArtefatoResumo }) {
  const s = situacaoAnexo(m);
  const foto = m.anexo_arquivo_id != null && iconeAnexo(m.subtipo) === "midias";
  return (
    <div className="mb-1.5 min-w-[14rem] rounded-md border border-borda bg-fundo/50 px-2 py-1.5" aria-label="Anexo">
      {foto && (
        <MiniaturaArquivo
          caso={caso}
          arquivoId={m.anexo_arquivo_id}
          categoria="imagem"
          icone="midias"
          ajuste="conter"
          className="mb-1.5 h-40 w-full rounded"
        />
      )}
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
