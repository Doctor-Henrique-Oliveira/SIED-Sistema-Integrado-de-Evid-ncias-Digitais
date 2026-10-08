import { useEffect, type ReactNode } from "react";
import { Link, useParams } from "react-router-dom";
import type { Caso, Extracao, ResumoCaso } from "../api/cliente";
import { useAbrirCaso, useCasoAtual, useExtracoes, usePastaEmRede, useResumoCaso } from "../api/consultas";
import { classeBotao } from "../componentes/Botao";
import { Aviso, Cartao, Carregando, FaixaIndicadores, Indicador, Selo } from "../componentes/Estrutura";
import { Icone, type NomeIcone } from "../componentes/Icone";
import { SecaoDispositivos, SecaoExtracoes } from "../componentes/SecoesCaso";
import { dataHora, identificacaoCaso, localDaPasta, numero, tamanho } from "../util/formatar";
import { nomeTipoArtefato } from "../util/rotulos";

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
    <div className="space-y-5">
      <CabecalhoCaso caso={caso} pasta={pasta} />
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="sobrelinha text-ouro!">Centro de análise</p>
          <h2 className="mt-0.5 text-lg font-semibold tracking-tight text-texto">Panorama operacional</h2>
        </div>
        <Atalhos caso={caso} />
      </div>
      <Indicadores caso={caso} r={resumo.data} />
      <LeituraCaso r={resumo.data} />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <div className="min-w-0">
          <SecaoDispositivos caso={caso} compacto />
        </div>
        <div className="min-w-0">
          <SecaoExtracoes caso={caso} compacto />
        </div>
      </div>
      <Alertas caso={caso} pasta={pasta} r={resumo.data} />
    </div>
  );
}

function CabecalhoCaso({ caso, pasta }: { caso: Caso; pasta: string }) {
  const fuso = caso.fuso_horario;
  const principais: [string, ReactNode][] = [
    ["Unidade", caso.unidade],
    ["Responsável pela análise", caso.responsavel_analise],
    ["Delegado(a)", caso.delegado],
    ["Criado em", dataHora(caso.criado_em, fuso)],
  ];
  const vinculados: [string, ReactNode][] = [
    ["Nº do procedimento", caso.numero_procedimento],
    ["Nº do BO", caso.numero_bo],
    ["Investigado", caso.investigado],
    ["Vítima", caso.vitima],
  ];
  return (
    <section aria-label="Identificação do caso" className="relative overflow-hidden rounded-md border border-borda bg-sup px-6 py-5">
      <span className="absolute inset-y-0 left-0 w-0.5 bg-ouro" aria-hidden />
      <div className="flex flex-wrap items-start justify-between gap-x-8 gap-y-4">
        <div className="min-w-0">
          <p className="sobrelinha flex items-center gap-1.5 text-ouro!">
            <Icone nome="casos" className="size-3.5" />
            {caso.procedimento ?? "Procedimento não informado"}
          </p>
          <h1 className="mt-1 truncate text-2xl font-semibold tracking-tight text-texto">{identificacaoCaso(caso)}</h1>
          <p className="mt-0.5 truncate font-mono text-2xs text-texto-3" title="Pasta do caso">
            {pasta}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Selo tom="ok" titulo="O banco deste caso está aberto nesta estação">
            <Icone nome="check" className="size-3" /> Aberto nesta estação
          </Selo>
          <Selo tom="neutro" titulo="Fuso registrado neste caso; usado para exibir as datas sem reinterpretá-las">
            <Icone nome="relogio" className="size-3" /> Fuso do caso: {caso.fuso_horario}
          </Selo>
        </div>
      </div>
      <dl className="mt-5 grid grid-cols-2 gap-x-6 gap-y-3 xl:grid-cols-4">
        {principais.map(([rotulo, valor]) => (
          <div key={rotulo} className="min-w-0 border-l border-borda-2 pl-3">
            <dt className="sobrelinha truncate">{rotulo}</dt>
            <dd className="mt-0.5 truncate text-sm text-texto" title={typeof valor === "string" ? valor : undefined}>
              {valor ?? "—"}
            </dd>
          </div>
        ))}
      </dl>
      <dl className="mt-4 grid grid-cols-2 gap-x-6 gap-y-3 border-t border-borda pt-4 lg:grid-cols-4">
        {vinculados.map(([rotulo, valor]) => (
          <div key={rotulo} className="min-w-0">
            <dt className="sobrelinha truncate">{rotulo}</dt>
            <dd className="mt-0.5 truncate text-sm text-texto-2" title={typeof valor === "string" ? valor : undefined}>{valor ?? "—"}</dd>
          </div>
        ))}
      </dl>
      {caso.observacoes && <p className="mt-3 whitespace-pre-wrap text-sm text-texto-2">{caso.observacoes}</p>}
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
    <nav aria-label="Atalhos do caso" className="flex flex-wrap gap-1.5">
      {itens.map(([rotulo, icone, rota]) => (
        <Link
          key={rota}
          to={`/casos/${caso.id}/${rota}`}
          className={classeBotao(rota === "busca" ? "primario" : "secundario")}
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
    .slice(0, 3)
    .map((t) => `${(nomeTipoArtefato[t.tipo] ?? t.tipo).toLowerCase()}: ${numero(t.total)}`)
    .join(" · ");
  const n = (v: number | undefined) => (v == null ? null : numero(v));
  return (
    <FaixaIndicadores rotulo="Indicadores do caso" className="grid-cols-2 md:grid-cols-4 2xl:grid-cols-8">
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
    </FaixaIndicadores>
  );
}

function LeituraCaso({ r }: { r: ResumoCaso | undefined }) {
  const processamento = r
    ? r.execucoes_com_problema > 0
      ? { tom: "atencao" as const, titulo: "Revisão necessária", detalhe: `${numero(r.execucoes_com_problema)} execução(ões) com problema` }
      : { tom: "ok" as const, titulo: "Processamento estável", detalhe: `${numero(r.execucoes_parser)} execução(ões) registradas` }
    : { tom: "neutro" as const, titulo: "Resumo em carregamento", detalhe: "Aguardando leitura do caso" };
  const cobertura = r ? `${numero(r.registros)} registros interpretados` : "—";
  const fontes = r ? `${numero(r.dispositivos)} dispositivo(s) · ${numero(r.extracoes)} extração(ões)` : "—";
  return (
    <section aria-label="Leitura rápida do caso" className="grid gap-4 rounded-xl border border-borda bg-sup/55 px-4 py-3 md:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)_minmax(0,1fr)]">
      <div className="flex min-w-0 items-start gap-3">
        <span className="mt-1 flex size-7 shrink-0 items-center justify-center rounded-md bg-ouro/10 text-ouro"><Icone nome="painel" className="size-4" /></span>
        <div className="min-w-0"><p className="sobrelinha">Leitura rápida</p><p className="mt-1 truncate text-sm font-medium text-texto">Visão operacional do caso</p><p className="mt-0.5 truncate text-xs text-texto-3">{cobertura}</p></div>
      </div>
      <div className="border-l border-borda pl-3"><p className="sobrelinha">Fontes</p><p className="mt-1 truncate font-mono text-xs text-texto-2">{fontes}</p><p className="mt-0.5 text-xs text-texto-3">{r ? `${numero(r.arquivos)} arquivos inventariados` : "Resumo ainda não disponível"}</p></div>
      <div className="border-l border-borda pl-3"><p className="sobrelinha">Processamento</p><div className="mt-1 flex items-center gap-2"><Selo tom={processamento.tom}>{processamento.titulo}</Selo></div><p className="mt-1 truncate text-xs text-texto-3">{processamento.detalhe}</p></div>
    </section>
  );
}

type Alerta = { tom: "erro" | "atencao" | "info"; texto: string; para?: string };

function Alertas({ caso, pasta, r }: { caso: Caso; pasta: string; r: ResumoCaso | undefined }) {
  const extracoes = useExtracoes(caso.id);
  const emRede = usePastaEmRede(pasta);
  const local = localDaPasta(pasta, emRede.data);
  const alertas: Alerta[] = [];
  if (local) {
    alertas.push({
      tom: "atencao",
      texto: `A pasta do caso está ${local === "compartilhamento de rede" ? "num compartilhamento de rede" : `no ${local}`}: a rede ou a sincronização podem danificar o banco do caso aberto. Feche o caso e copie a pasta para um disco local.`,
    });
  }
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
        <p className="flex items-center gap-2 text-sm text-texto-2">
          <Icone nome="check" className="size-4 text-ok" />
          Nenhum alerta nas importações deste caso.
        </p>
      ) : (
        <ul className="space-y-2 text-sm" aria-label="Alertas do caso">
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
