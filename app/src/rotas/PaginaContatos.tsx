// Módulo Contatos: agenda do aparelho (e contatos dos apps), registro de chamadas e
// identificadores das conversas. Tudo como os parsers leram: o SIED não junta
// contatos repetidos nem deduz quem usava cada número. O vínculo "Na agenda" é só a
// igualdade exata de dígitos dentro da mesma extração, calculado na consulta.

import { useEffect, useState, type ReactNode } from "react";
import { Link, useSearchParams } from "react-router-dom";
import type { Caso, Chamada, ContatoAgenda, ContatoVinculado, FiltroChamadas, FiltroContatos } from "../api/cliente";
import { useAppsDosContatos, useChamadas, useContatos, useIdentificadores, useResumoChamadas } from "../api/consultas";
import { EvidenciaRegistro, SeloEvidencia, useMarcas } from "../componentes/Evidencias";
import {
  Abas,
  Aviso,
  Cabecalho,
  Cartao,
  Carregando,
  entrada,
  FaixaIndicadores,
  Filtros,
  Indicador,
  Paginacao,
  Selo,
  tabela,
  Vazio,
  type Tom,
} from "../componentes/Estrutura";
import { Icone } from "../componentes/Icone";
import { PainelTecnico } from "../componentes/PainelTecnico";
import { dataHoraMs, duracaoSegundos, identificacaoCaso, numero } from "../util/formatar";
import {
  nomeApresentacao,
  nomeEstadoRegistro,
  nomeNumeroEspecial,
  nomeRecurso,
  nomeTipoTelefone,
  rotuloApp,
  rotuloSubtipo,
} from "../util/rotulos";
import { ComCasoAberto } from "./PaginaCaso";

const LIMITE = 100;

type Aba = "agenda" | "chamadas" | "identificadores";
const ABAS: Aba[] = ["agenda", "chamadas", "identificadores"];

export function PaginaContatos() {
  return <ComCasoAberto>{(caso) => <Contatos caso={caso} />}</ComCasoAberto>;
}

/** Estado da tela na URL (?aba=, ?q=, ?registro=), para links da busca e entre abas. */
function useEstadoUrl() {
  const [params, setParams] = useSearchParams();
  const bruta = params.get("aba");
  const aba: Aba = ABAS.includes(bruta as Aba) ? (bruta as Aba) : "agenda";
  const q = params.get("q");
  const registro = Number(params.get("registro")) || null;
  const ir = (novo: { aba?: Aba; q?: string | null; registro?: number | null }) => {
    const p = new URLSearchParams();
    p.set("aba", novo.aba ?? aba);
    const nq = "q" in novo ? novo.q : q;
    if (nq) p.set("q", nq);
    const nr = "registro" in novo ? novo.registro : registro;
    if (nr != null) p.set("registro", String(nr));
    setParams(p, { replace: true });
  };
  return { aba, q, registro, ir };
}

function Contatos({ caso }: { caso: Caso }) {
  const estado = useEstadoUrl();
  return (
    <>
      <Cabecalho
        icone="contatos"
        sobre={identificacaoCaso(caso)}
        titulo="Contatos"
        subtitulo="Agenda do aparelho, registro de chamadas e identificadores das conversas, exatamente como os parsers leram."
      />
      <div className="space-y-3">
        <Abas<Aba>
          rotulo="Seções de contatos"
          atual={estado.aba}
          mudar={(a) => estado.ir({ aba: a, q: null, registro: null })}
          abas={[
            ["agenda", "Agenda e contatos"],
            ["chamadas", "Chamadas"],
            ["identificadores", "Identificadores das conversas"],
          ]}
        />
        {estado.aba === "agenda" ? (
          <Agenda key="agenda" caso={caso} estado={estado} />
        ) : estado.aba === "chamadas" ? (
          <Chamadas key="chamadas" caso={caso} estado={estado} />
        ) : (
          <Identificadores key="identificadores" caso={caso} estado={estado} />
        )}
      </div>
    </>
  );
}

type Estado = ReturnType<typeof useEstadoUrl>;

/** Campo de filtro por texto que aplica ao enviar (Enter) e acompanha ?q=. */
function FiltroTexto({ valor, aplicar, rotulo, dica }: {
  valor: string | null;
  aplicar: (t: string | null) => void;
  rotulo: string;
  dica: string;
}) {
  const [texto, setTexto] = useState(valor ?? "");
  useEffect(() => setTexto(valor ?? ""), [valor]);
  return (
    <form
      role="search"
      onSubmit={(e) => {
        e.preventDefault();
        aplicar(texto.trim() || null);
      }}
    >
      <input
        type="search"
        value={texto}
        onChange={(e) => setTexto(e.target.value)}
        placeholder={dica}
        aria-label={rotulo}
        className={`${entrada} w-72`}
      />
    </form>
  );
}

/** Contatos da agenda com o mesmo número (nome e selo quando o contato está apagado). */
function NaAgenda({ agenda }: { agenda: ContatoVinculado[] }) {
  if (agenda.length === 0) return <span className="text-texto-3">—</span>;
  return (
    <span className="flex flex-wrap items-center gap-1">
      {agenda.map((c) => (
        <span key={c.id} className="inline-flex items-center gap-1">
          <span className="text-texto">{c.nome ?? <em className="text-texto-3">sem nome</em>}</span>
          {c.estado_registro !== "ativo" && (
            <Selo tom="atencao">{nomeEstadoRegistro[c.estado_registro] ?? c.estado_registro}</Selo>
          )}
        </span>
      ))}
    </span>
  );
}

function SeloEstado({ estado }: { estado: string }) {
  if (estado === "ativo") return null;
  return <Selo tom="atencao">{nomeEstadoRegistro[estado] ?? estado}</Selo>;
}

/** Duas colunas: lista à esquerda, registro selecionado à direita. */
function ListaComDetalhe({ lista, detalhe }: { lista: ReactNode; detalhe: ReactNode }) {
  return (
    <div className="grid items-start gap-3 xl:grid-cols-[minmax(0,1fr)_340px] 2xl:grid-cols-[minmax(0,1fr)_400px]">
      <div className="min-w-0">{lista}</div>
      <div className="min-w-0 space-y-3 xl:sticky xl:top-0">{detalhe}</div>
    </div>
  );
}

function Linha({ rotulo, children }: { rotulo: string; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[7.5rem_minmax(0,1fr)] gap-2 py-1 text-xs">
      <dt className="text-texto-3">{rotulo}</dt>
      <dd className="break-words text-texto">{children ?? "—"}</dd>
    </div>
  );
}

// ------------------------------------------------------------------- Agenda

function Agenda({ caso, estado }: { caso: Caso; estado: Estado }) {
  const [app, setApp] = useState<string | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const filtro: FiltroContatos = {};
  if (estado.q) filtro.texto = estado.q;
  if (app) filtro.app = app;
  const apps = useAppsDosContatos(caso.id);
  const lista = useContatos(caso.id, filtro, deslocamento, LIMITE);
  const marcas = useMarcas(caso.id);
  const selecionado = lista.data?.itens.find((c) => c.id === estado.registro) ?? null;
  const totalApps = apps.data?.reduce((s, a) => s + a.total, 0) ?? 0;

  const tabelaContatos = lista.isPending ? (
    <Carregando />
  ) : lista.isError ? (
    <Aviso>{lista.error.message}</Aviso>
  ) : lista.data.total === 0 ? (
    <Vazio titulo={estado.q || app ? "Nenhum contato com esse filtro" : "Nenhum contato importado"} icone="contatos">
      {estado.q || app
        ? undefined
        : "Importe uma extração com a agenda do Android (contacts2.db) ou com bancos de contatos de apps (ex.: wa.db do WhatsApp)."}
    </Vazio>
  ) : (
    <Cartao denso titulo={`Contatos (${numero(lista.data.total)})`} icone="contatos">
      <div className="overflow-x-auto">
        <table className={tabela.base} aria-label="Contatos">
          <thead className={tabela.cabeca}>
            <tr>
              <th className={tabela.th}>Nome</th>
              <th className={tabela.th}>Telefones</th>
              <th className={tabela.th}>Origem</th>
              <th className={tabela.th}>Situação</th>
            </tr>
          </thead>
          <tbody className={tabela.corpo}>
            {lista.data.itens.map((c) => (
              <tr
                key={c.id}
                aria-selected={c.id === estado.registro}
                className={`${tabela.linha} cursor-pointer ${c.id === estado.registro ? "bg-ouro/8" : ""}`}
                onClick={() => estado.ir({ registro: c.id })}
              >
                <td className={tabela.td}>
                  <button
                    type="button"
                    className="text-left font-medium text-texto hover:text-ouro-claro"
                    onClick={(e) => {
                      e.stopPropagation();
                      estado.ir({ registro: c.id });
                    }}
                  >
                    {c.nome ?? <em className="font-normal text-texto-3">Sem nome no registro</em>}
                  </button>
                  {c.empresa && <p className="text-xs text-texto-3">{c.empresa}</p>}
                </td>
                <td className={`${tabela.td} font-mono text-xs`}>
                  {c.telefones.length > 0
                    ? c.telefones.map((t, i) => <div key={i}>{t.numero}</div>)
                    : c.app !== "contatos_android" && c.id_externo
                      ? <span className="text-texto-2">{c.id_externo}</span>
                      : <span className="text-texto-3">—</span>}
                </td>
                <td className={`${tabela.td} text-xs`}>
                  <div className="text-texto-2">{rotuloApp(c.app)}</div>
                  {c.conta && <div className="text-texto-3">{c.conta}</div>}
                </td>
                <td className={tabela.td}>
                  <span className="flex flex-wrap gap-1">
                    <SeloEstado estado={c.estado_registro} />
                    {c.favorito && <Selo tom="info">Favorito</Selo>}
                    <SeloEvidencia numeros={marcas.registros.get(c.id)} />
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {lista.data.total > LIMITE && (
        <div className="border-t border-borda px-3">
          <Paginacao total={lista.data.total} deslocamento={deslocamento} limite={LIMITE} mudar={setDeslocamento} />
        </div>
      )}
    </Cartao>
  );

  return (
    <>
      <Aviso tipo="info">
        Cada linha é um registro de contato como o aparelho gravou: na agenda do Android, um contato por conta (Google,
        WhatsApp, aparelho). Contatos repetidos em contas diferentes aparecem separados, e o SIED não os junta. Apagados
        continuam listados com o selo da situação.
      </Aviso>
      <div className="flex flex-wrap items-center justify-between gap-3">
        {apps.data && apps.data.length > 1 ? (
          <Filtros<string | null>
            rotulo="Origem dos contatos"
            atual={app}
            mudar={(a) => {
              setApp(a);
              setDeslocamento(0);
            }}
            opcoes={[
              [null, `Todos (${numero(totalApps)})`],
              ...apps.data.map((a): [string, string] => [a.app, `${rotuloApp(a.app)} (${numero(a.total)})`]),
            ]}
          />
        ) : (
          <span />
        )}
        <FiltroTexto
          valor={estado.q}
          rotulo="Filtrar contatos"
          dica="Filtrar por nome ou telefone"
          aplicar={(t) => {
            setDeslocamento(0);
            estado.ir({ q: t, registro: null });
          }}
        />
      </div>
      <ListaComDetalhe
        lista={tabelaContatos}
        detalhe={
          estado.registro == null ? (
            <PainelTecnico caso={caso} artefatoId={null} />
          ) : (
            <>
              {selecionado && <DetalheContato caso={caso} c={selecionado} estado={estado} />}
              {!selecionado && (
                <Cartao titulo="Contato" icone="contatos">
                  <p className="text-sm text-texto-2">Registro nº {estado.registro}.</p>
                  <EvidenciaRegistro caso={caso} artefatoId={estado.registro} />
                </Cartao>
              )}
              <PainelTecnico caso={caso} artefatoId={estado.registro} />
            </>
          )
        }
      />
    </>
  );
}

function DetalheContato({ caso, c, estado }: { caso: Caso; c: ContatoAgenda; estado: Estado }) {
  return (
    <Cartao titulo={c.nome ?? "Sem nome no registro"} icone="contatos" rotulo="Contato selecionado">
      <dl>
        <Linha rotulo="Origem">{rotuloApp(c.app)}</Linha>
        {c.conta && (
          <Linha rotulo="Conta">
            {c.conta}
            {c.tipo_conta && <span className="ml-1 font-mono text-texto-3">({c.tipo_conta})</span>}
          </Linha>
        )}
        {c.empresa && <Linha rotulo="Empresa">{c.empresa}</Linha>}
        {c.emails.length > 0 && <Linha rotulo="E-mails">{c.emails.join(", ")}</Linha>}
        {c.id_externo && (
          <Linha rotulo="Id no app">
            <span className="font-mono">{c.id_externo}</span>
          </Linha>
        )}
      </dl>
      {c.telefones.length > 0 && (
        <>
          <p className="sobrelinha mb-1 mt-3">Telefones</p>
          <ul className="divide-y divide-borda rounded-md border border-borda" aria-label="Telefones do contato">
            {c.telefones.map((t, i) => (
              <li key={i} className="flex items-center justify-between gap-2 px-2.5 py-1.5 text-xs">
                <span className="min-w-0">
                  <span className="whitespace-nowrap font-mono text-texto">{t.numero}</span>
                  <span className="ml-2 text-texto-3">
                    {[t.tipo ? (nomeTipoTelefone[t.tipo] ?? t.tipo) : null, t.rotulo].filter(Boolean).join(" · ")}
                  </span>
                </span>
                <button
                  type="button"
                  className="shrink-0 text-ouro-claro hover:underline"
                  onClick={() => estado.ir({ aba: "chamadas", q: t.numero, registro: null })}
                >
                  Ver chamadas
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      <EvidenciaRegistro caso={caso} artefatoId={c.id} />
    </Cartao>
  );
}

// ----------------------------------------------------------------- Chamadas

const tomSubtipo = (s: string | null): Tom =>
  s === "recebida" ? "info" : s === "perdida" || s === "rejeitada" || s === "bloqueada" ? "atencao" : "neutro";

function SeloChamada({ c }: { c: Pick<Chamada, "subtipo" | "de_mim"> }) {
  const seta = c.de_mim === true ? "↗" : c.de_mim === false ? "↙" : "";
  return (
    <Selo tom={tomSubtipo(c.subtipo)}>
      {seta && <span aria-hidden>{seta}</span>}
      {rotuloSubtipo(c.subtipo ?? "desconhecido")}
    </Selo>
  );
}

/** Número da chamada; sem número, diz o que o aparelho gravou no lugar dele. */
function NumeroChamada({ c }: { c: Chamada }) {
  if (c.numero) return <span className="font-mono text-xs text-texto">{c.numero}</span>;
  const motivo = c.numero_especial
    ? (nomeNumeroEspecial[c.numero_especial] ?? `Código ${c.numero_especial}`)
    : c.apresentacao && c.apresentacao !== "permitido"
      ? (nomeApresentacao[c.apresentacao] ?? c.apresentacao)
      : "Sem número gravado";
  return <span className="text-xs italic text-texto-3">{motivo}</span>;
}

function Chamadas({ caso, estado }: { caso: Caso; estado: Estado }) {
  const [subtipo, setSubtipo] = useState<string | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const filtro: FiltroChamadas = {};
  if (estado.q) filtro.texto = estado.q;
  if (subtipo) filtro.subtipo = subtipo;
  const resumo = useResumoChamadas(caso.id);
  const lista = useChamadas(caso.id, filtro, deslocamento, LIMITE);
  const marcas = useMarcas(caso.id);
  const selecionada = lista.data?.itens.find((c) => c.id === estado.registro) ?? null;
  const r = resumo.data;
  const porSubtipo = (s: string) => r?.por_subtipo.find((x) => x.subtipo === s)?.total ?? 0;

  if (r && r.total === 0)
    return (
      <Vazio titulo="Nenhuma chamada importada" icone="telefone">
        Importe uma extração com o registro de chamadas do Android (calllog.db, ou contacts2.db em versões antigas).
      </Vazio>
    );

  const tabelaChamadas = lista.isPending ? (
    <Carregando />
  ) : lista.isError ? (
    <Aviso>{lista.error.message}</Aviso>
  ) : lista.data.total === 0 ? (
    <Vazio titulo="Nenhuma chamada com esse filtro" icone="telefone" />
  ) : (
    <Cartao denso titulo={`Chamadas (${numero(lista.data.total)})`} icone="telefone">
      <div className="overflow-x-auto">
        <table className={tabela.base} aria-label="Chamadas">
          <thead className={tabela.cabeca}>
            <tr>
              <th className={tabela.th}>Data e hora</th>
              <th className={tabela.th}>Tipo</th>
              <th className={tabela.th}>Número</th>
              <th className={tabela.th}>Nome no registro · agenda</th>
              <th className={`${tabela.th} text-right`}>Duração</th>
            </tr>
          </thead>
          <tbody className={tabela.corpo}>
            {lista.data.itens.map((c) => (
              <tr
                key={c.id}
                aria-selected={c.id === estado.registro}
                className={`${tabela.linha} cursor-pointer ${c.id === estado.registro ? "bg-ouro/8" : ""}`}
                onClick={() => estado.ir({ registro: c.id })}
              >
                <td className={`${tabela.td} whitespace-nowrap`}>
                  <button
                    type="button"
                    className="tabular text-left text-xs text-texto hover:text-ouro-claro"
                    onClick={(e) => {
                      e.stopPropagation();
                      estado.ir({ registro: c.id });
                    }}
                  >
                    {dataHoraMs(c.data_hora_ms, caso.fuso_horario)}
                  </button>
                </td>
                <td className={tabela.td}>
                  <span className="flex flex-wrap gap-1">
                    <SeloChamada c={c} />
                    <SeloEstado estado={c.estado_registro} />
                    <SeloEvidencia numeros={marcas.registros.get(c.id)} />
                  </span>
                </td>
                <td className={tabela.td}>
                  <NumeroChamada c={c} />
                </td>
                <td className={`${tabela.td} text-xs`}>
                  <div>{c.nome_no_registro ?? <span className="text-texto-3">—</span>}</div>
                  {c.agenda.length > 0 && (
                    <div className="mt-0.5 flex items-start gap-1 text-texto-2" title="Contato da agenda com o mesmo número">
                      <Icone nome="contatos" className="mt-px size-3 text-texto-3" />
                      <span className="sr-only">Na agenda:</span>
                      <NaAgenda agenda={c.agenda} />
                    </div>
                  )}
                </td>
                <td className={`${tabela.td} tabular whitespace-nowrap text-right text-xs`}>{duracaoSegundos(c.duracao_s)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {lista.data.total > LIMITE && (
        <div className="border-t border-borda px-3">
          <Paginacao total={lista.data.total} deslocamento={deslocamento} limite={LIMITE} mudar={setDeslocamento} />
        </div>
      )}
    </Cartao>
  );

  return (
    <>
      <FaixaIndicadores rotulo="Resumo das chamadas" className="grid-cols-2 md:grid-cols-3 xl:grid-cols-6">
        <Indicador rotulo="Chamadas" icone="telefone" tom="ouro" valor={r ? numero(r.total) : null} />
        <Indicador rotulo="Recebidas" icone="telefone" tom="info" valor={r ? numero(porSubtipo("recebida")) : null} />
        <Indicador rotulo="Efetuadas" icone="telefone" valor={r ? numero(porSubtipo("efetuada")) : null} />
        <Indicador rotulo="Perdidas" icone="telefone" tom="atencao" valor={r ? numero(porSubtipo("perdida")) : null} />
        <Indicador rotulo="Duração somada" icone="relogio" valor={r ? duracaoSegundos(r.duracao_total_s) : null} />
        <Indicador rotulo="Números distintos" icone="hash" valor={r ? numero(r.numeros_distintos) : null} />
      </FaixaIndicadores>
      <Aviso tipo="info">
        “Nome no registro” é o nome que o aparelho gravou junto da chamada. Abaixo dele, com o ícone de contato, aparecem os
        contatos da agenda da mesma extração com exatamente o mesmo número (ou o número normalizado pelo aparelho): é uma coincidência de número, não uma identificação
        de quem usava a linha.
      </Aviso>
      <div className="flex flex-wrap items-center justify-between gap-3">
        {r && r.por_subtipo.length > 1 ? (
          <Filtros<string | null>
            rotulo="Tipo de chamada"
            atual={subtipo}
            mudar={(s) => {
              setSubtipo(s);
              setDeslocamento(0);
            }}
            opcoes={[
              [null, `Todas (${numero(r.total)})`],
              ...r.por_subtipo.map((s): [string, string] => [s.subtipo, `${rotuloSubtipo(s.subtipo)} (${numero(s.total)})`]),
            ]}
          />
        ) : (
          <span />
        )}
        <FiltroTexto
          valor={estado.q}
          rotulo="Filtrar chamadas"
          dica="Filtrar por número ou nome"
          aplicar={(t) => {
            setDeslocamento(0);
            estado.ir({ q: t, registro: null });
          }}
        />
      </div>
      <ListaComDetalhe
        lista={tabelaChamadas}
        detalhe={
          estado.registro == null ? (
            <PainelTecnico caso={caso} artefatoId={null} />
          ) : (
            <>
              {selecionada ? (
                <DetalheChamada caso={caso} c={selecionada} />
              ) : (
                <Cartao titulo="Chamada" icone="telefone">
                  <p className="text-sm text-texto-2">Registro nº {estado.registro}.</p>
                  <EvidenciaRegistro caso={caso} artefatoId={estado.registro} />
                </Cartao>
              )}
              <PainelTecnico caso={caso} artefatoId={estado.registro} />
            </>
          )
        }
      />
    </>
  );
}

function DetalheChamada({ caso, c }: { caso: Caso; c: Chamada }) {
  return (
    <Cartao titulo="Chamada selecionada" icone="telefone" rotulo="Chamada selecionada" acoes={<SeloChamada c={c} />}>
      <dl>
        <Linha rotulo="Data e hora">{dataHoraMs(c.data_hora_ms, caso.fuso_horario)}</Linha>
        <Linha rotulo={c.de_mim ? "Para" : "De"}>
          <NumeroChamada c={c} />
        </Linha>
        <Linha rotulo="Duração">{duracaoSegundos(c.duracao_s)}</Linha>
        {c.nome_no_registro && <Linha rotulo="Nome no registro">{c.nome_no_registro}</Linha>}
        <Linha rotulo="Na agenda">
          <NaAgenda agenda={c.agenda} />
        </Linha>
        {c.apresentacao && <Linha rotulo="Apresentação">{nomeApresentacao[c.apresentacao] ?? c.apresentacao}</Linha>}
        {c.recursos.length > 0 && <Linha rotulo="Recursos">{c.recursos.map((x) => nomeRecurso[x] ?? x).join(", ")}</Linha>}
        {c.conta_telefonica && (
          <Linha rotulo="Conta/chip">
            <span className="font-mono">{c.conta_telefonica}</span>
          </Linha>
        )}
      </dl>
      {c.numero && (
        <Link
          to={`/casos/${caso.id}/busca?q=${encodeURIComponent(c.numero)}`}
          className="mt-2 inline-flex items-center gap-1 text-xs text-ouro-claro hover:underline"
        >
          <Icone nome="busca" className="size-3.5" />
          Buscar o número no caso
        </Link>
      )}
      <EvidenciaRegistro caso={caso} artefatoId={c.id} />
    </Cartao>
  );
}

// ---------------------------------------------------------- Identificadores

function Identificadores({ caso, estado }: { caso: Caso; estado: Estado }) {
  const [deslocamento, setDeslocamento] = useState(0);
  const lista = useIdentificadores(caso.id, estado.q, deslocamento, LIMITE);

  return (
    <>
      <Aviso tipo="info">
        Os identificadores aparecem exatamente como o parser leu. O SIED não junta números parecidos nem deduz a quem
        pertencem. O nome no app, quando existe, é o gravado nos contatos do próprio app na mesma extração (ex.: wa.db do
        WhatsApp); “Na agenda” mostra contatos do Android da mesma extração com exatamente o mesmo número.
      </Aviso>
      <Cartao
        denso
        titulo={lista.data ? `Identificadores (${numero(lista.data.total)})` : "Identificadores"}
        icone="contatos"
        acoes={
          <FiltroTexto
            valor={estado.q}
            rotulo="Filtrar identificadores"
            dica="Filtrar por identificador ou nome"
            aplicar={(t) => {
              setDeslocamento(0);
              estado.ir({ q: t });
            }}
          />
        }
      >
        {lista.isPending ? (
          <div className="px-4">
            <Carregando />
          </div>
        ) : lista.isError ? (
          <div className="p-4">
            <Aviso>{lista.error.message}</Aviso>
          </div>
        ) : lista.data.total === 0 ? (
          <div className="p-4">
            <Vazio
              titulo={estado.q ? "Nenhum identificador ou nome com esse trecho" : "Nenhum identificador nas conversas"}
              icone="contatos"
            >
              {estado.q ? undefined : "Importe uma extração com mensagens para ver os identificadores aqui."}
            </Vazio>
          </div>
        ) : (
          <>
            <div className="overflow-x-auto">
              <table className={tabela.base} aria-label="Identificadores das conversas">
                <thead className={tabela.cabeca}>
                  <tr>
                    <th className={tabela.th}>Identificador</th>
                    <th className={tabela.th}>Nome no app</th>
                    <th className={tabela.th}>Na agenda</th>
                    <th className={tabela.th}>App</th>
                    <th className={`${tabela.th} text-right`}>Conversas</th>
                    <th className={`${tabela.th} text-right`}>Mensagens</th>
                    <th className={tabela.th}>Primeira</th>
                    <th className={tabela.th}>Última</th>
                    <th className={tabela.th}>
                      <span className="sr-only">Ações</span>
                    </th>
                  </tr>
                </thead>
                <tbody className={tabela.corpo}>
                  {lista.data.itens.map((i) => (
                    <tr key={`${i.app}:${i.identificador}`} className={tabela.linha}>
                      <td className={`${tabela.td} font-mono text-xs text-texto`}>{i.identificador}</td>
                      <td className={`${tabela.td} text-texto`}>{i.nome ?? <span className="text-texto-3">—</span>}</td>
                      <td className={`${tabela.td} text-xs`}>
                        <NaAgenda agenda={i.agenda ?? []} />
                      </td>
                      <td className={tabela.td}>{rotuloApp(i.app)}</td>
                      <td className={`${tabela.td} tabular text-right`}>{numero(i.conversas)}</td>
                      <td className={`${tabela.td} tabular text-right`}>{numero(i.mensagens)}</td>
                      <td className={`${tabela.td} tabular whitespace-nowrap text-xs text-texto-2`}>
                        {dataHoraMs(i.primeira_ms, caso.fuso_horario)}
                      </td>
                      <td className={`${tabela.td} tabular whitespace-nowrap text-xs text-texto-2`}>
                        {dataHoraMs(i.ultima_ms, caso.fuso_horario)}
                      </td>
                      <td className={`${tabela.td} text-right`}>
                        <Link
                          to={`/casos/${caso.id}/busca?q=${encodeURIComponent(i.identificador)}`}
                          className="whitespace-nowrap text-xs text-ouro-claro hover:underline"
                        >
                          Buscar ocorrências
                        </Link>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {lista.data.total > LIMITE && (
              <div className="border-t border-borda px-3">
                <Paginacao total={lista.data.total} deslocamento={deslocamento} limite={LIMITE} mudar={setDeslocamento} />
              </div>
            )}
          </>
        )}
      </Cartao>
    </>
  );
}
