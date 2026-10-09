import type { Caso, Dispositivo, Extracao } from "../api/cliente";
import { useDispositivos, useExtracoes } from "../api/consultas";
import { Aviso, Carregando } from "../componentes/Estrutura";
import { Botao } from "../componentes/Botao";
import { Icone } from "../componentes/Icone";
import { SecaoDispositivos, SecaoExtracoes } from "../componentes/SecoesCaso";
import { identificacaoCaso } from "../util/formatar";

type ConsultaDispositivos = ReturnType<typeof useDispositivos>;
type ConsultaExtracoes = ReturnType<typeof useExtracoes>;

export function fontesProntas(dispositivos: Dispositivo[] | undefined, extracoes: Extracao[] | undefined) {
  return !!dispositivos?.length && !!extracoes?.some((item) => item.status === "concluida");
}

/** A estação só libera o workspace após dispositivo e extração concluída reais. */
export function PreparacaoCaso({
  caso,
  pasta,
  dispositivos,
  extracoes,
}: {
  caso: Caso;
  pasta: string;
  dispositivos: ConsultaDispositivos;
  extracoes: ConsultaExtracoes;
}) {
  const possuiDispositivo = (dispositivos.data?.length ?? 0) > 0;
  const possuiExtracao = extracoes.data?.some((item) => item.status === "concluida") ?? false;
  const carregando = dispositivos.isPending || extracoes.isPending;
  const etapaAtual = possuiExtracao ? 3 : possuiDispositivo ? 2 : 1;

  return (
    <div className="preparacao-sied mx-auto max-w-[1180px] px-7 pb-10 pt-7">
      <div className="preparacao-sied__topo mb-6 flex items-center justify-between gap-4 border-b border-borda pb-5">
        <div className="flex min-w-0 items-center gap-3.5">
          <span className="preparacao-sied__marca grid size-10 shrink-0 place-items-center rounded-md border border-ouro/35 bg-ouro/8 text-ouro">
            <Icone nome="escudo" className="size-5" />
          </span>
          <div className="min-w-0">
            <p className="sobrelinha text-ouro!">Inicialização do workspace</p>
            <h1 className="mt-0.5 truncate text-2xl font-semibold tracking-[-0.025em] text-texto">Preparar fontes do caso</h1>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <span className="hidden text-right sm:block">
            <span className="block text-2xs font-medium text-texto-2">Etapa {Math.min(etapaAtual, 2)} de 2</span>
            <span className="block text-3xs text-texto-3">Dispositivo + extração verificada</span>
          </span>
          <span className="inline-flex items-center gap-1.5 rounded-sm border border-borda-2 px-2.5 py-1 font-mono text-2xs uppercase tracking-wide text-texto-2">
            <span className="size-1.5 rounded-full bg-ok" aria-hidden /> Local · offline
          </span>
        </div>
      </div>

      <section aria-label="Identificação do caso" className="preparacao-sied__caso mb-6 border-l-2 border-ouro px-5 py-4">
        <div className="flex flex-wrap items-baseline justify-between gap-x-5 gap-y-1">
          <div>
            <p className="sobrelinha">{caso.procedimento ?? "Procedimento não informado"}</p>
            <h2 className="mt-1 text-xl font-semibold tracking-tight text-texto">{identificacaoCaso(caso)}</h2>
          </div>
          <p className="max-w-sm truncate text-xs text-texto-2" title={caso.unidade}>{caso.unidade}</p>
        </div>
        <p className="mt-2 truncate font-mono text-2xs text-texto-3" title={pasta}>{pasta}</p>
      </section>

      {carregando ? <Carregando texto="Conferindo fontes do caso…" /> : null}
      {dispositivos.isError ? <div className="mb-4"><Aviso>{dispositivos.error.message}<Botao variante="secundario" tamanho="p" className="mt-2" onClick={() => void dispositivos.refetch()}>Tentar novamente</Botao></Aviso></div> : null}
      {extracoes.isError ? <div className="mb-4"><Aviso>{extracoes.error.message}<Botao variante="secundario" tamanho="p" className="mt-2" onClick={() => void extracoes.refetch()}>Tentar novamente</Botao></Aviso></div> : null}

      <div className="preparacao-sied__fluxo grid gap-6 lg:grid-cols-[13.5rem_minmax(0,1fr)]">
        <aside className="preparacao-sied__trilho" aria-label="Progresso da preparação">
          <p className="sobrelinha mb-4">Sequência de liberação</p>
          <ol className="space-y-0">
            <li className={`preparacao-sied__marco ${possuiDispositivo ? "is-completo" : "is-ativo"}`}>
              <span className="preparacao-sied__numero">01</span>
              <div><p className="text-sm font-medium text-texto">Dispositivo</p><p className="mt-0.5 text-xs text-texto-3">Identificação do aparelho examinado</p></div>
            </li>
            <li className={`preparacao-sied__marco ${possuiExtracao ? "is-completo" : possuiDispositivo ? "is-ativo" : ""}`}>
              <span className="preparacao-sied__numero">02</span>
              <div><p className="text-sm font-medium text-texto">Extração</p><p className="mt-0.5 text-xs text-texto-3">Importação concluída e inventariada</p></div>
            </li>
            <li className={`preparacao-sied__marco ${possuiExtracao ? "is-ativo" : ""}`}>
              <span className="preparacao-sied__numero"><Icone nome="painel" className="size-3.5" /></span>
              <div><p className="text-sm font-medium text-texto">Workspace</p><p className="mt-0.5 text-xs text-texto-3">Análise liberada após as fontes</p></div>
            </li>
          </ol>
          <div className="mt-5 border-t border-borda pt-4">
            <p className="flex items-start gap-2 text-xs text-texto-3"><Icone nome="info" className="mt-0.5 size-3.5 shrink-0 text-ouro" />A evidência original permanece intocada durante toda a preparação.</p>
          </div>
        </aside>

        <div className="min-w-0 space-y-6">
        <section aria-label="Etapa 1: dispositivo" className={`preparacao-sied__etapa ${possuiDispositivo ? "is-completa" : "is-ativa"}`}>
          <div className="mb-2.5 flex items-center gap-3">
            <span className={`preparacao-sied__rotulo-etapa font-mono text-xs ${possuiDispositivo ? "text-ok" : "text-ouro"}`}>01 · Fonte física</span>
            <div className="h-px flex-1 bg-borda-2" />
            <span className={`flex items-center gap-1.5 text-xs ${possuiDispositivo ? "text-ok" : "text-texto-3"}`}>
              {possuiDispositivo ? <><Icone nome="check" className="size-3.5" /> Cadastrado</> : "Aguardando dispositivo"}
            </span>
          </div>
          <SecaoDispositivos caso={caso} compacto />
        </section>

        <section aria-label="Etapa 2: extração" className={`preparacao-sied__etapa ${possuiExtracao ? "is-completa" : possuiDispositivo ? "is-ativa" : "is-bloqueada"}`}>
          <div className="mb-2.5 flex items-center gap-3">
            <span className={`preparacao-sied__rotulo-etapa font-mono text-xs ${possuiExtracao ? "text-ok" : possuiDispositivo ? "text-ouro" : "text-texto-3"}`}>02 · Fonte lógica</span>
            <div className="h-px flex-1 bg-borda-2" />
            <span className={`flex items-center gap-1.5 text-xs ${possuiExtracao ? "text-ok" : "text-texto-3"}`}>
              {possuiExtracao ? <><Icone nome="check" className="size-3.5" /> Concluída</> : "Aguardando extração concluída"}
            </span>
          </div>
          <SecaoExtracoes caso={caso} compacto />
        </section>
        </div>
      </div>
      <div className="preparacao-sied__rodape mt-7 flex flex-wrap items-center justify-between gap-3 border-t border-borda pt-4 text-xs text-texto-3">
        <p>O workspace será aberto automaticamente quando ambas as etapas estiverem concluídas.</p>
        <span className={`inline-flex items-center gap-1.5 font-mono text-2xs uppercase tracking-wide ${possuiExtracao ? "text-ok" : "text-texto-3"}`}>
          <span className={`size-1.5 rounded-full ${possuiExtracao ? "bg-ok" : "bg-texto-4"}`} aria-hidden />
          {possuiExtracao ? "Workspace pronto" : "Workspace protegido"}
        </span>
      </div>
    </div>
  );
}
