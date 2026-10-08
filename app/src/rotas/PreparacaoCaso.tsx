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

  return (
    <div className="mx-auto max-w-[1040px] px-7 pb-10 pt-8">
      <div className="mb-7 flex items-center justify-between gap-4 border-b border-borda pb-5">
        <div className="flex min-w-0 items-center gap-3.5">
          <span className="grid size-10 shrink-0 place-items-center rounded-md border border-ouro/35 bg-ouro/8 text-ouro">
            <Icone nome="escudo" className="size-5" />
          </span>
          <div className="min-w-0">
            <p className="sobrelinha text-ouro!">Estação forense · preparação</p>
            <h1 className="mt-0.5 truncate text-2xl font-semibold tracking-tight text-texto">Preparar o caso</h1>
          </div>
        </div>
        <span className="rounded-sm border border-borda-2 px-2.5 py-1 font-mono text-2xs uppercase tracking-wide text-texto-2">Local · offline</span>
      </div>

      <section aria-label="Identificação do caso" className="mb-6 border-l-2 border-ouro bg-sup/75 px-5 py-4">
        <div className="flex flex-wrap items-baseline justify-between gap-x-5 gap-y-1">
          <div>
            <p className="sobrelinha">{caso.procedimento ?? "Procedimento não informado"}</p>
            <h2 className="mt-1 text-xl font-semibold tracking-tight text-texto">{identificacaoCaso(caso)}</h2>
          </div>
          <p className="max-w-sm truncate text-xs text-texto-2" title={caso.unidade}>{caso.unidade}</p>
        </div>
        <p className="mt-2 truncate font-mono text-2xs text-texto-3" title={pasta}>{pasta}</p>
      </section>

      <div className="mb-6 flex items-start gap-4 text-sm text-texto-2">
        <span className="mt-0.5 shrink-0 text-ouro"><Icone nome="info" className="size-4" /></span>
        <p>Cadastre o dispositivo e importe uma extração concluída para liberar o workspace. Até lá, somente a preparação do caso fica visível.</p>
      </div>

      {carregando ? <Carregando texto="Conferindo fontes do caso…" /> : null}
      {dispositivos.isError ? <div className="mb-4"><Aviso>{dispositivos.error.message}<Botao variante="secundario" tamanho="p" className="mt-2" onClick={() => void dispositivos.refetch()}>Tentar novamente</Botao></Aviso></div> : null}
      {extracoes.isError ? <div className="mb-4"><Aviso>{extracoes.error.message}<Botao variante="secundario" tamanho="p" className="mt-2" onClick={() => void extracoes.refetch()}>Tentar novamente</Botao></Aviso></div> : null}

      <div className="space-y-6">
        <section aria-label="Etapa 1: dispositivo">
          <div className="mb-2.5 flex items-center gap-3">
            <span className={`font-mono text-xs ${possuiDispositivo ? "text-ok" : "text-ouro"}`}>01</span>
            <div className="h-px flex-1 bg-borda-2" />
            <span className={`flex items-center gap-1.5 text-xs ${possuiDispositivo ? "text-ok" : "text-texto-3"}`}>
              {possuiDispositivo ? <><Icone nome="check" className="size-3.5" /> Cadastrado</> : "Aguardando dispositivo"}
            </span>
          </div>
          <SecaoDispositivos caso={caso} compacto />
        </section>

        <section aria-label="Etapa 2: extração">
          <div className="mb-2.5 flex items-center gap-3">
            <span className={`font-mono text-xs ${possuiExtracao ? "text-ok" : possuiDispositivo ? "text-ouro" : "text-texto-3"}`}>02</span>
            <div className="h-px flex-1 bg-borda-2" />
            <span className={`flex items-center gap-1.5 text-xs ${possuiExtracao ? "text-ok" : "text-texto-3"}`}>
              {possuiExtracao ? <><Icone nome="check" className="size-3.5" /> Concluída</> : "Aguardando extração concluída"}
            </span>
          </div>
          <SecaoExtracoes caso={caso} compacto />
        </section>
      </div>
      <p className="mt-7 border-t border-borda pt-4 text-xs text-texto-3">A preparação não altera a extração original. O workspace será aberto automaticamente quando ambas as etapas estiverem concluídas.</p>
    </div>
  );
}
