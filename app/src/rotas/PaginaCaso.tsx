import { useEffect, type ReactNode } from "react";
import { Link, useParams } from "react-router-dom";
import type { Caso, Extracao, ResumoCaso } from "../api/cliente";
import { useAbrirCaso, useCasoAtual, useExtracoes, useResumoCaso } from "../api/consultas";
import { Aviso, Cartao, Carregando, Indicador, Selo } from "../componentes/Estrutura";
import { Icone, type NomeIcone } from "../componentes/Icone";
import { SecaoAtividades, SecaoDispositivos, SecaoExtracoes } from "../componentes/SecoesCaso";
import { dataHora, identificacaoCaso, numero, tamanho } from "../util/formatar";

export function PaginaCaso() {
  return <ComCasoAberto>{(caso, pasta) => <PainelCaso caso={caso} pasta={pasta} />}</ComCasoAberto>;
}

/** Garante que o caso da rota esteja aberto no backend antes de mostrar o conteúdo. */
export function ComCasoAberto({ children }: { children: (caso: Caso, pasta: string) => ReactNode }) {
  const { casoId = "" } = useParams();
  const atual = useCasoAtual();
  const abrir = useAbrirCaso();
  const abertoAqui = atual.data?.caso.id === casoId;

  // Ao navegar direto para um caso (ex.: pela lista), abre-o no backend.
  useEffect(() => {
    if (atual.isSuccess && !abertoAqui && !abrir.isPending && !abrir.isError) abrir.mutate(casoId);
  }, [atual.isSuccess, abertoAqui, casoId, abrir]);

  if (abrir.isError) return <Aviso>{abrir.error.message}</Aviso>;
  if (!abertoAqui || !atual.data) return <Carregando texto="Abrindo caso…" />;
  return <>{children(atual.data.caso, atual.data.pasta)}</>;
}

/** Link "← Voltar ao caso" das páginas internas. */
export function VoltarAoCaso({ caso }: { caso: Caso }) {
  return (
    <Link to={`/casos/${caso.id}`} className="normal-case tracking-normal text-texto-2 hover:text-ouro-claro">
      ← Voltar ao caso
    </Link>
  );
}

// ---------------------------------------------------------------- Dashboard do caso

function PainelCaso({ caso, pasta }: { caso: Caso; pasta: string }) {
  const resumo = useResumoCaso(caso.id);
  return (
    <div className="space-y-4">
      <CabecalhoCaso caso={caso} pasta={pasta} />
      <Atalhos caso={caso} />
      <Indicadores caso={caso} r={resumo.data} />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_22rem]">
        <div className="min-w-0 space-y-4">
          <SecaoDispositivos caso={caso} compacto />
          <SecaoExtracoes caso={caso} compacto />
        </div>
        <div className="min-w-0 space-y-4">
          <Alertas caso={caso} r={resumo.data} />
          <SecaoAtividades caso={caso} />
        </div>
      </div>
    </div>
  );
}

function CabecalhoCaso({ caso, pasta }: { caso: Caso; pasta: string }) {
  const fuso = caso.fuso_horario;
  const campos: [string, ReactNode][] = [
    ["Nº do procedimento", caso.numero_procedimento],
    ["Nº do BO", caso.numero_bo],
    ["Investigado", caso.investigado],
    ["Vítima", caso.vitima],
    ["Delegado(a)", caso.delegado],
    ["Responsável pela análise", caso.responsavel_analise],
    ["Unidade", caso.unidade],
    ["Criado em", dataHora(caso.criado_em, fuso)],
  ];
  return (
    <section
      aria-label="Identificação do caso"
      className="overflow-hidden rounded-xl border border-borda bg-gradient-to-br from-sup-2 via-sup to-sup"
    >
      <div className="flex flex-wrap items-start justify-between gap-4 border-b border-borda px-5 pb-3.5 pt-4">
        <div className="flex min-w-0 items-start gap-3.5">
          <span className="grid size-11 shrink-0 place-items-center rounded-xl bg-ouro/12 text-ouro ring-1 ring-ouro/35">
            <Icone nome="casos" className="size-6" />
          </span>
          <div className="min-w-0">
            <p className="text-[11px] font-semibold uppercase tracking-wider text-ouro">
              {caso.procedimento ?? "Procedimento não informado"}
            </p>
            <h1 className="truncate text-2xl font-semibold tracking-tight text-texto">{identificacaoCaso(caso)}</h1>
            <p className="mt-0.5 truncate font-mono text-[11px] text-texto-3" title="Pasta do caso">
              {pasta}
            </p>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Selo tom="ok" titulo="O banco deste caso está aberto nesta estação">
            <Icone nome="check" className="size-3" /> Aberto nesta estação
          </Selo>
          <Selo tom="neutro" titulo="Fuso usado para exibir todas as datas do caso">
            <Icone nome="relogio" className="size-3" /> {caso.fuso_horario}
          </Selo>
        </div>
      </div>
      <dl className="grid grid-cols-2 gap-x-6 gap-y-3 px-5 py-3.5 md:grid-cols-4 2xl:grid-cols-8">
        {campos.map(([rotulo, valor]) => (
          <div key={rotulo} className="min-w-0">
            <dt className="text-[11px] font-medium uppercase tracking-wider text-texto-3">{rotulo}</dt>
            <dd className="mt-0.5 truncate text-[13px] text-texto" title={typeof valor === "string" ? valor : undefined}>
              {valor ?? "—"}
            </dd>
          </div>
        ))}
      </dl>
      {caso.observacoes && (
        <p className="whitespace-pre-wrap border-t border-borda px-5 py-3 text-[13px] text-texto-2">{caso.observacoes}</p>
      )}
    </section>
  );
}

function Atalhos({ caso }: { caso: Caso }) {
  const itens: [string, NomeIcone, string][] = [
    ["Conversas", "conversas", "conversas"],
    ["Buscar", "busca", "busca"],
    ["Mídias", "midias", "midias"],
    ["Arquivos", "arquivos", "arquivos"],
    ["Localizações", "localizacoes", "localizacoes"],
    ["Relatórios", "relatorios", "relatorios"],
  ];
  return (
    <nav aria-label="Atalhos do caso" className="flex flex-wrap gap-2">
      {itens.map(([rotulo, icone, rota]) => (
        <Link
          key={rota}
          to={`/casos/${caso.id}/${rota}`}
          className={`inline-flex items-center gap-2 rounded-lg border px-3 py-1.5 text-[13px] font-medium transition-colors ${
            rota === "busca"
              ? "border-ouro/50 bg-ouro text-[#1a1305] hover:bg-ouro-claro"
              : "border-borda-2 bg-sup text-texto hover:bg-sup-2"
          }`}
        >
          <Icone nome={icone} className="size-4" />
          {rotulo}
        </Link>
      ))}
    </nav>
  );
}

const CATEGORIAS_MIDIA = ["imagem", "video", "audio", "documento"];

function Indicadores({ caso, r }: { caso: Caso; r: ResumoCaso | undefined }) {
  const base = `/casos/${caso.id}`;
  const midias = r?.arquivos_por_categoria.filter((c) => CATEGORIAS_MIDIA.includes(c.categoria)).reduce((s, c) => s + c.arquivos, 0);
  const principais = r?.registros_por_tipo
    .slice(0, 2)
    .map((t) => `${t.tipo}: ${numero(t.total)}`)
    .join(" · ");
  const n = (v: number | undefined) => (v == null ? null : numero(v));
  return (
    <section aria-label="Indicadores do caso" className="grid grid-cols-2 gap-3 md:grid-cols-4 2xl:grid-cols-8">
      <Indicador rotulo="Dispositivos" icone="dispositivo" valor={n(r?.dispositivos)} para={`${base}/dispositivos`} />
      <Indicador
        rotulo="Extrações"
        icone="extracao"
        valor={n(r?.extracoes)}
        detalhe={r && `${numero(r.extracoes_concluidas)} concluídas`}
        para={`${base}/extracoes`}
      />
      <Indicador rotulo="Arquivos" icone="arquivos" valor={n(r?.arquivos)} detalhe={r && tamanho(r.bytes)} para={`${base}/arquivos`} />
      <Indicador rotulo="Mídias" icone="midias" valor={n(midias)} detalhe="imagens, vídeos, áudios, docs" para={`${base}/midias`} />
      <Indicador rotulo="Registros" icone="banco" tom="ouro" valor={n(r?.registros)} detalhe={principais || (r && "interpretados")} />
      <Indicador rotulo="Conversas" icone="conversas" valor={n(r?.conversas)} para={`${base}/conversas`} />
      <Indicador
        rotulo="Identificadores"
        icone="contatos"
        valor={n(r?.identificadores)}
        detalhe="nas conversas"
        para={`${base}/contatos`}
      />
      <Indicador
        rotulo="Localizações"
        icone="localizacoes"
        valor={n(r?.localizacoes)}
        detalhe="registros com coordenadas"
        para={`${base}/localizacoes`}
      />
    </section>
  );
}

type Alerta = { tom: "erro" | "atencao" | "info"; texto: string; para?: string };

function Alertas({ caso, r }: { caso: Caso; r: ResumoCaso | undefined }) {
  const extracoes = useExtracoes(caso.id);
  const alertas: Alerta[] = [];
  for (const x of extracoes.data ?? ([] as Extracao[])) {
    if (x.status === "falhou" || x.status === "interrompida" || x.status === "cancelada") {
      alertas.push({
        tom: x.status === "falhou" ? "erro" : "atencao",
        texto: `Extração ${x.status === "falhou" ? "falhou" : x.status === "cancelada" ? "cancelada" : "interrompida"}: ${x.importador_nome}, ${dataHora(x.iniciada_em, caso.fuso_horario)}`,
        para: `/casos/${caso.id}/extracoes/${x.id}`,
      });
    }
    if (x.arquivos_ilegiveis) {
      alertas.push({
        tom: "atencao",
        texto: `${numero(x.arquivos_ilegiveis)} arquivo(s) ilegível(is) na extração ${x.importador_nome}`,
        para: `/casos/${caso.id}/extracoes/${x.id}`,
      });
    }
  }
  if (r?.execucoes_com_problema) {
    alertas.push({ tom: "atencao", texto: `${numero(r.execucoes_com_problema)} execução(ões) de parser com problema` });
  }
  if (r?.avisos_parser) {
    alertas.push({ tom: "info", texto: `${numero(r.avisos_parser)} aviso(s) registrados pelos parsers` });
  }
  const cor = { erro: "text-erro", atencao: "text-atencao", info: "text-info" };

  return (
    <Cartao titulo="Alertas e pendências" icone="alerta">
      {extracoes.isPending ? (
        <Carregando />
      ) : alertas.length === 0 ? (
        <p className="flex items-center gap-2 text-[13px] text-texto-2">
          <Icone nome="check" className="size-4 text-ok" />
          Nenhum alerta nas importações deste caso.
        </p>
      ) : (
        <ul className="space-y-2 text-[13px]" aria-label="Alertas do caso">
          {alertas.map((a, i) => (
            <li key={i} className="flex items-start gap-2">
              <Icone nome={a.tom === "info" ? "info" : "alerta"} className={`mt-0.5 size-4 ${cor[a.tom]}`} />
              {a.para ? (
                <Link to={a.para} className="text-texto hover:text-ouro-claro hover:underline">
                  {a.texto}
                </Link>
              ) : (
                <span className="text-texto">{a.texto}</span>
              )}
            </li>
          ))}
        </ul>
      )}
    </Cartao>
  );
}
