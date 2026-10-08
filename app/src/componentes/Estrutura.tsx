// Blocos visuais reutilizados por todas as telas do SIED (design system).
// Cores, tipografia e cantos vêm dos tokens e receitas de estilos.css: nada de cor
// solta nas páginas. Hierarquia por tipografia e linhas finas, não por cartões.

import { useId, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { classeBotao } from "./Botao";
import { Icone, type NomeIcone } from "./Icone";

/** Classes das tabelas densas (cabeçalho fixo em caixa alta, linhas com divisória). */
export const tabela = {
  base: "tabela-forense w-full text-left text-sm",
  cabeca: "tabela-forense__cabeca border-b border-borda bg-lateral/40 text-3xs uppercase tracking-[0.05em] text-texto-3",
  th: "px-3 py-2 font-semibold",
  corpo: "divide-y divide-borda",
  td: "px-3 py-[7px] align-top",
  linha: "tabela-forense__linha transition-colors hover:bg-sup-2/70",
};

/** Cabeçalho da página: sobrelinha opcional, título, subtítulo e ações à direita. */
export function Cabecalho({ titulo, subtitulo, acoes, sobre, icone }: {
  titulo: string;
  subtitulo?: ReactNode;
  acoes?: ReactNode;
  sobre?: ReactNode;
  icone?: NomeIcone;
}) {
  return (
    <header className="cabecalho-modulo mb-5 flex flex-wrap items-end justify-between gap-x-6 gap-y-3 border-b border-borda pb-4">
      <div className="flex min-w-0 items-start gap-3">
        {icone && (
          <span className="cabecalho-modulo__icone" aria-hidden>
            <Icone nome={icone} className="size-[18px] text-ouro" />
          </span>
        )}
        <div className="min-w-0">
          {sobre && <div className="sobrelinha cabecalho-modulo__sobre mb-1">{sobre}</div>}
          <h1 className="min-w-0 truncate text-xl font-semibold tracking-[-0.018em] text-texto">{titulo}</h1>
          {subtitulo && <div className="cabecalho-modulo__subtitulo mt-1 max-w-4xl text-sm text-texto-2">{subtitulo}</div>}
        </div>
      </div>
      {acoes && <div className="cabecalho-modulo__acoes flex flex-wrap items-center gap-2">{acoes}</div>}
    </header>
  );
}

/** Painel com título opcional. `denso` tira o preenchimento (para tabelas de borda a borda). */
export function Cartao({ titulo, acoes, children, className = "", denso, icone, rotulo }: {
  titulo?: ReactNode;
  acoes?: ReactNode;
  children: ReactNode;
  className?: string;
  denso?: boolean;
  icone?: NomeIcone;
  rotulo?: string;
}) {
  return (
    <section aria-label={rotulo} className={`painel cartao-sied min-w-0 ${className}`}>
      {(titulo || acoes) && (
        <div className="flex min-h-10 items-center justify-between gap-3 border-b border-borda px-4 py-1.5">
          {titulo && (
            <h2 className="flex min-w-0 items-center gap-2 text-sm font-semibold text-texto">
              {icone && <Icone nome={icone} className="size-4 text-texto-3" />}
              <span className="truncate">{titulo}</span>
            </h2>
          )}
          {acoes && <div className="flex shrink-0 items-center gap-2">{acoes}</div>}
        </div>
      )}
      <div className={denso ? "" : "p-4"}>{children}</div>
    </section>
  );
}

/** Lista de pares rótulo/valor; valores ausentes aparecem como "—". */
export function Definicoes({ itens, colunas = 3 }: { itens: [string, ReactNode][]; colunas?: 2 | 3 | 4 }) {
  const grade = { 2: "sm:grid-cols-2", 3: "sm:grid-cols-2 lg:grid-cols-3", 4: "sm:grid-cols-2 xl:grid-cols-4" }[colunas];
  return (
    <dl className={`grid grid-cols-1 gap-x-6 gap-y-3 ${grade}`}>
      {itens.map(([rotulo, valor]) => (
        <div key={rotulo} className="min-w-0">
          <dt className="sobrelinha">{rotulo}</dt>
          <dd className="mt-0.5 break-words text-sm text-texto">{valor ?? "—"}</dd>
        </div>
      ))}
    </dl>
  );
}

type TipoAviso = "erro" | "info" | "sucesso" | "atencao";

const estiloAviso: Record<TipoAviso, { cor: string; icone: NomeIcone }> = {
  erro: { cor: "border-erro/35 border-l-erro bg-erro/8 text-erro-claro", icone: "alerta" },
  info: { cor: "border-info/30 border-l-info bg-info/7 text-info-claro", icone: "info" },
  sucesso: { cor: "border-ok/30 border-l-ok bg-ok/7 text-ok-claro", icone: "check" },
  atencao: { cor: "border-atencao/35 border-l-atencao bg-atencao/8 text-atencao-claro", icone: "alerta" },
};

export function Aviso({ tipo = "erro", children }: { tipo?: TipoAviso; children: ReactNode }) {
  const e = estiloAviso[tipo];
  return (
    <div role={tipo === "erro" ? "alert" : "status"} className={`flex gap-2.5 rounded-md border border-l-2 px-3.5 py-2.5 text-sm ${e.cor}`}>
      <Icone nome={e.icone} className="mt-0.5 size-4" />
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}

export function Carregando({ texto = "Carregando…" }: { texto?: string }) {
  return (
    <div className="flex items-center gap-2 py-6 text-sm text-texto-2" role="status">
      <span className="size-3.5 animate-spin rounded-full border-2 border-borda-2 border-r-ouro" aria-hidden />
      {texto}
    </div>
  );
}

/** Estado vazio: sem dados ainda (não é erro). */
export function Vazio({ titulo, children, icone, acao }: {
  titulo: string;
  children?: ReactNode;
  icone?: NomeIcone;
  acao?: ReactNode;
}) {
  return (
    <div className="estado-vazio flex flex-col items-center rounded-xl border border-dashed border-borda-2 px-6 py-8 text-center">
      {icone && (
        <span className="estado-vazio__icone" aria-hidden>
          <Icone nome={icone} className="size-5 text-texto-2" />
        </span>
      )}
      <p className="font-medium text-texto">{titulo}</p>
      {children && <div className="mt-1.5 max-w-prose text-sm text-texto-2">{children}</div>}
      {acao && <div className="mt-4">{acao}</div>}
    </div>
  );
}

/** Módulo ainda não disponível: explica o que falta, sem mostrar dados inventados. */
export function Indisponivel({ titulo, icone, children, previsto }: {
  titulo: string;
  icone: NomeIcone;
  children: ReactNode;
  previsto?: string[];
}) {
  return (
    <div className="painel modulo-planejado p-5" role="status">
      <div className="flex items-start gap-3.5">
        <Icone nome={icone} className="mt-0.5 size-5 shrink-0 text-texto-3" />
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <p className="font-semibold text-texto">{titulo}</p>
            <Selo tom="neutro">Ainda não disponível</Selo>
          </div>
          <div className="mt-1.5 max-w-3xl text-sm leading-relaxed text-texto-2">{children}</div>
          {previsto && previsto.length > 0 && (
            <div className="mt-4">
              <p className="sobrelinha">Estrutura preparada</p>
              <ul className="mt-2 grid gap-x-6 gap-y-1 text-sm text-texto-2 sm:grid-cols-2">
                {previsto.map((p) => (
                  <li key={p} className="flex items-center gap-2">
                    <span className="size-1 rounded-full bg-texto-3" aria-hidden />
                    {p}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export type Tom = "ok" | "atencao" | "erro" | "neutro" | "info" | "ouro";

const corSelo: Record<Tom, string> = {
  ok: "bg-ok/10 text-ok ring-ok/30",
  atencao: "bg-atencao/10 text-atencao ring-atencao/30",
  erro: "bg-erro/10 text-erro ring-erro/35",
  neutro: "bg-sup-2 text-texto-2 ring-borda-2",
  info: "bg-info/10 text-info ring-info/30",
  ouro: "bg-ouro/10 text-ouro-claro ring-ouro/35",
};

/** Selo de situação (StatusBadge). */
export function Selo({ tom = "neutro", children, titulo }: { tom?: Tom; children: ReactNode; titulo?: string }) {
  return (
    <span
      title={titulo}
      className={`inline-flex h-5 items-center gap-1 whitespace-nowrap rounded-sm px-1.5 text-2xs font-medium ring-1 ring-inset ${corSelo[tom]}`}
    >
      {children}
    </span>
  );
}

/** Hash completo em fonte monoespaçada, quebrando linha quando precisar. */
export function Hash({ valor }: { valor: string | null | undefined }) {
  if (!valor) return <>—</>;
  return <span className="break-all font-mono text-xs text-texto">{valor}</span>;
}

/** Hash abreviado, com o valor completo no título (passe o mouse). */
export function HashCurto({ valor, tamanho = 16 }: { valor: string | null | undefined; tamanho?: number }) {
  if (!valor) return <>—</>;
  return (
    <span className="font-mono text-xs text-texto-2" title={valor}>
      {valor.slice(0, tamanho)}…
    </span>
  );
}

export function BarraProgresso({ fracao, rotulo }: { fracao: number | null; rotulo: string }) {
  const pct = fracao == null ? null : Math.max(0, Math.min(100, fracao * 100));
  return (
    <div
      role="progressbar"
      aria-label={rotulo}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct == null ? undefined : Math.round(pct)}
      className="h-1 w-full overflow-hidden rounded-full bg-sup-3"
    >
      <div
        className={`h-full rounded-full bg-ouro transition-[width] duration-300 ${pct == null ? "w-1/3 animate-pulse" : ""}`}
        style={pct == null ? undefined : { width: `${pct}%` }}
      />
    </div>
  );
}

export function Paginacao({ total, deslocamento, limite, mudar }: {
  total: number;
  deslocamento: number;
  limite: number;
  mudar: (deslocamento: number) => void;
}) {
  if (total <= limite) return null;
  const fim = Math.min(deslocamento + limite, total);
  const botao = classeBotao("secundario", "p");
  return (
    <div className="flex items-center justify-between gap-3 px-1 py-2 text-xs text-texto-2">
      <span className="tabular">
        {(deslocamento + 1).toLocaleString("pt-BR")}–{fim.toLocaleString("pt-BR")} de {total.toLocaleString("pt-BR")}
      </span>
      <div className="flex gap-1.5">
        <button type="button" className={botao} disabled={deslocamento === 0} onClick={() => mudar(Math.max(0, deslocamento - limite))}>
          Anterior
        </button>
        <button type="button" className={botao} disabled={fim >= total} onClick={() => mudar(deslocamento + limite)}>
          Próxima
        </button>
      </div>
    </div>
  );
}

/** Faixa de indicadores: células separadas por linhas finas, sem um cartão por número. */
export function FaixaIndicadores({ rotulo, children, className = "" }: { rotulo: string; children: ReactNode; className?: string }) {
  return (
    <section aria-label={rotulo} className={`grid gap-px overflow-hidden rounded-xl border border-borda bg-borda ${className}`}>
      {children}
    </section>
  );
}

/** Indicador numérico (StatCard). `valor` nulo = indisponível, nunca um número inventado. */
export function Indicador({ rotulo, valor, detalhe, icone, tom = "neutro", para }: {
  rotulo: string;
  valor: ReactNode | null;
  detalhe?: ReactNode;
  icone: NomeIcone;
  tom?: "neutro" | "ouro" | "ok" | "atencao" | "erro" | "info";
  para?: string;
}) {
  const corIcone = {
    neutro: "text-texto-3",
    ouro: "text-ouro",
    ok: "text-ok",
    atencao: "text-atencao",
    erro: "text-erro",
    info: "text-info",
  }[tom];
  const conteudo = (
    <>
      <p className="sobrelinha flex items-center gap-1.5">
        <Icone nome={icone} className={`size-3.5 ${corIcone}`} />
        <span className="truncate">{rotulo}</span>
      </p>
      <p className={`tabular mt-1.5 text-2xl font-semibold tracking-[-0.02em] ${valor == null ? "text-texto-3" : "text-texto"}`}>
        {valor ?? "—"}
      </p>
      {detalhe && <div className="mt-0.5 truncate text-xs text-texto-3">{detalhe}</div>}
    </>
  );
  const classe = "block min-w-0 bg-sup px-4 py-3";
  return para ? (
    <Link to={para} className={`${classe} group transition-colors hover:bg-sup-2`}>
      {conteudo}
    </Link>
  ) : (
    <div className={classe}>{conteudo}</div>
  );
}

/** Seção recolhível (blocos técnicos). Acessível: botão com aria-expanded controla a região. */
export function SecaoRecolhivel({ titulo, children, aberta = true, extra, icone }: {
  titulo: string;
  children: ReactNode;
  aberta?: boolean;
  extra?: ReactNode;
  icone?: NomeIcone;
}) {
  const [aberto, setAberto] = useState(aberta);
  const id = useId();
  return (
    <div className="border-b border-borda last:border-b-0">
      <button
        type="button"
        aria-expanded={aberto}
        aria-controls={id}
        onClick={() => setAberto((a) => !a)}
        className="sobrelinha flex w-full items-center gap-2 px-4 py-2.5 text-left text-texto-2! transition-colors hover:bg-sup-2/60 hover:text-texto!"
      >
        <Icone nome="direita" className={`size-3.5 transition-transform ${aberto ? "rotate-90" : ""}`} />
        {icone && <Icone nome={icone} className="size-3.5 text-texto-3" />}
        <span className="flex-1">{titulo}</span>
        {extra}
      </button>
      <div id={id} hidden={!aberto} className="px-4 pb-3.5">
        {children}
      </div>
    </div>
  );
}

/** Link com aparência de botão (ações que navegam). */
export function LinkBotao({ para, children, variante = "secundario", icone, className = "" }: {
  para: string;
  children: ReactNode;
  variante?: "primario" | "secundario" | "fantasma";
  icone?: NomeIcone;
  className?: string;
}) {
  return (
    <Link to={para} className={`${classeBotao(variante)} ${className}`}>
      {icone && <Icone nome={icone} className="size-4" />}
      {children}
    </Link>
  );
}

/** Abas acessíveis (role=tablist/tab). */
export function Abas<T extends string>({ abas, atual, mudar, rotulo }: {
  abas: [T, ReactNode][];
  atual: T;
  mudar: (a: T) => void;
  rotulo: string;
}) {
  return (
    <div role="tablist" aria-label={rotulo} className="abas-forenses flex gap-1 overflow-x-auto border-b border-borda">
      {abas.map(([id, texto]) => (
        <button
          key={id}
          type="button"
          role="tab"
          aria-selected={atual === id}
          onClick={() => mudar(id)}
          className={`-mb-px whitespace-nowrap border-b-2 px-3 py-2 text-sm font-medium transition-colors ${
            atual === id ? "border-ouro text-texto" : "border-transparent text-texto-3 hover:border-borda-2 hover:text-texto-2"
          }`}
        >
          {texto}
        </button>
      ))}
    </div>
  );
}

/** Pílulas de filtro (categoria, app...). */
export function Filtros<T extends string | null>({ opcoes, atual, mudar, rotulo }: {
  opcoes: [T, ReactNode][];
  atual: T;
  mudar: (v: T) => void;
  rotulo: string;
}) {
  return (
    <div role="group" aria-label={rotulo} className="flex flex-wrap gap-1.5">
      {opcoes.map(([valor, texto]) => (
        <button
          key={String(valor)}
          type="button"
          aria-pressed={atual === valor}
          onClick={() => mudar(valor)}
          className={`h-6 rounded-md px-2 text-xs font-medium ring-1 ring-inset transition-colors ${
            atual === valor ? "bg-ouro/12 text-ouro-claro ring-ouro/45" : "text-texto-2 ring-borda-2 hover:bg-sup-2 hover:text-texto"
          }`}
        >
          {texto}
        </button>
      ))}
    </div>
  );
}

/** Classe dos campos de entrada soltos (busca, filtros) no tema escuro. */
export const entrada = "entrada";
