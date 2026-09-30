// Marcação de evidências nas telas de análise: mensagens (registro ou trecho) e
// arquivos do inventário. O hash do alvo é gravado pelo backend no momento da
// marcação; aqui só se escolhe o que marcar e se mostra o número "E-n".

import { createContext, useContext, useMemo } from "react";
import { Link } from "react-router-dom";
import type { ArtefatoResumo, Caso, CasoId } from "../api/cliente";
import { useAdicionarEvidencia, useMarcacoes } from "../api/consultas";
import { dataHoraMs } from "../util/formatar";
import { Botao } from "./Botao";
import { Selo } from "./Estrutura";
import { Icone } from "./Icone";

/** Números de evidência por registro e por arquivo (um registro pode estar em mais de uma). */
export type Marcas = { registros: Map<number, number[]>; arquivos: Map<number, number[]> };

const vazio: Marcas = { registros: new Map(), arquivos: new Map() };

function agrupar(pares: [number, number][]): Map<number, number[]> {
  const m = new Map<number, number[]>();
  for (const [id, n] of pares) m.set(id, [...(m.get(id) ?? []), n]);
  return m;
}

export function useMarcas(casoId: CasoId): Marcas {
  const q = useMarcacoes(casoId);
  return useMemo(
    () => (q.data ? { registros: agrupar(q.data.registros), arquivos: agrupar(q.data.arquivos) } : vazio),
    [q.data],
  );
}

/** Marcas disponíveis para as bolhas do chat sem passar por todas as camadas. */
export const MarcasContexto = createContext<Marcas>(vazio);
export const useMarcasContexto = () => useContext(MarcasContexto);

export function SeloEvidencia({ numeros }: { numeros: number[] | undefined }) {
  if (!numeros || numeros.length === 0) return null;
  return (
    <Selo tom="ouro" titulo="Marcada como evidência (número no relatório)">
      <Icone nome="relatorios" className="size-3" />
      {numeros.map((n) => `E-${n}`).join(", ")}
    </Selo>
  );
}

function LinkRelatorios({ caso }: { caso: Caso }) {
  return (
    <Link to={`/casos/${caso.id}/relatorios`} className="inline-flex items-center gap-1 text-xs text-ouro-claro hover:underline">
      <Icone nome="relatorios" className="size-3.5" />
      Ver em Relatórios
    </Link>
  );
}

/**
 * Barra do chat: marca a mensagem selecionada ou um trecho (início e fim). O trecho
 * vale na ordem da conversa, qualquer que seja a ordem em que as pontas são escolhidas.
 */
export function BarraEvidencia({ caso, selecionada, inicio, setInicio }: {
  caso: Caso;
  selecionada: ArtefatoResumo | null;
  inicio: ArtefatoResumo | null;
  setInicio: (m: ArtefatoResumo | null) => void;
}) {
  const marcas = useMarcas(caso.id);
  const adicionar = useAdicionarEvidencia(caso.id);
  if (!selecionada && !inicio) return null;
  const numeros = selecionada ? marcas.registros.get(selecionada.id) : undefined;
  const quando = (m: ArtefatoResumo) => dataHoraMs(m.data_hora_ms, caso.fuso_horario);
  return (
    <div
      role="region"
      aria-label="Evidência"
      className="flex flex-wrap items-center gap-x-3 gap-y-1.5 border-b border-borda bg-ouro/5 px-3.5 py-1.5 text-xs text-texto-2"
    >
      {inicio ? (
        <>
          <span className="min-w-0">
            <span className="font-medium text-ouro-claro">Trecho:</span> início no registro nº {inicio.id} ({quando(inicio)}).{" "}
            {selecionada && selecionada.id !== inicio.id ? `Fim no registro nº ${selecionada.id} (${quando(selecionada)}).` : "Selecione a última mensagem."}
          </span>
          <span className="ml-auto flex items-center gap-2">
            <Botao
              variante="primario"
              className="!py-1 !text-xs"
              icone="mais"
              disabled={!selecionada || selecionada.id === inicio.id}
              carregando={adicionar.isPending}
              onClick={() =>
                selecionada &&
                adicionar.mutate(
                  { artefato_id: inicio.id, trecho_fim_id: selecionada.id },
                  { onSuccess: () => setInicio(null) },
                )
              }
            >
              Adicionar trecho às evidências
            </Botao>
            <Botao variante="fantasma" className="!py-1 !text-xs" onClick={() => setInicio(null)}>
              Cancelar trecho
            </Botao>
          </span>
        </>
      ) : (
        selecionada && (
          <>
            <span className="min-w-0 truncate">Mensagem selecionada: registro nº {selecionada.id}</span>
            {numeros ? <SeloEvidencia numeros={numeros} /> : null}
            <span className="ml-auto flex items-center gap-2">
              {numeros ? (
                <LinkRelatorios caso={caso} />
              ) : (
                <Botao
                  variante="secundario"
                  className="!py-1 !text-xs"
                  icone="mais"
                  carregando={adicionar.isPending}
                  onClick={() => adicionar.mutate({ artefato_id: selecionada.id })}
                >
                  Adicionar às evidências
                </Botao>
              )}
              <Botao variante="fantasma" className="!py-1 !text-xs" onClick={() => setInicio(selecionada)}>
                Marcar início do trecho
              </Botao>
            </span>
          </>
        )
      )}
      {adicionar.isError && (
        <p role="alert" className="basis-full text-erro">
          {adicionar.error.message}
        </p>
      )}
      {adicionar.isSuccess && !adicionar.isPending && (
        <p role="status" className="basis-full text-ok">
          Adicionada como evidência nº {adicionar.data.numero}
          {adicionar.data.itens > 1 ? ` (${adicionar.data.itens} mensagens)` : ""}. O hash foi registrado agora.
        </p>
      )}
    </div>
  );
}

/** Detalhe de um arquivo do inventário: marcar como evidência. */
export function EvidenciaArquivo({ caso, arquivoId, sha256 }: { caso: Caso; arquivoId: number; sha256: string | null }) {
  const marcas = useMarcas(caso.id);
  const adicionar = useAdicionarEvidencia(caso.id);
  const numeros = marcas.arquivos.get(arquivoId);
  return (
    <div className="mt-3 border-t border-borda pt-3" aria-label="Evidência do arquivo">
      {numeros ? (
        <div className="flex flex-wrap items-center gap-2">
          <SeloEvidencia numeros={numeros} />
          <LinkRelatorios caso={caso} />
        </div>
      ) : sha256 ? (
        <Botao
          variante="secundario"
          icone="mais"
          carregando={adicionar.isPending}
          onClick={() => adicionar.mutate({ arquivo_id: arquivoId })}
        >
          Adicionar às evidências
        </Botao>
      ) : (
        <p className="text-xs text-texto-3">Sem SHA-256 no inventário: o arquivo não pode ser marcado como evidência.</p>
      )}
      {adicionar.isError && (
        <p role="alert" className="mt-1.5 text-xs text-erro">
          {adicionar.error.message}
        </p>
      )}
    </div>
  );
}

/** Detalhe de um registro solto (contato, chamada): marcar como evidência. */
export function EvidenciaRegistro({ caso, artefatoId }: { caso: Caso; artefatoId: number }) {
  const marcas = useMarcas(caso.id);
  const adicionar = useAdicionarEvidencia(caso.id);
  const numeros = marcas.registros.get(artefatoId);
  return (
    <div className="mt-3 border-t border-borda pt-3" aria-label="Evidência do registro">
      {numeros ? (
        <div className="flex flex-wrap items-center gap-2">
          <SeloEvidencia numeros={numeros} />
          <LinkRelatorios caso={caso} />
        </div>
      ) : (
        <Botao
          variante="secundario"
          icone="mais"
          carregando={adicionar.isPending}
          onClick={() => adicionar.mutate({ artefato_id: artefatoId })}
        >
          Adicionar às evidências
        </Botao>
      )}
      {adicionar.isError && (
        <p role="alert" className="mt-1.5 text-xs text-erro">
          {adicionar.error.message}
        </p>
      )}
      {adicionar.isSuccess && !adicionar.isPending && (
        <p role="status" className="mt-1.5 text-xs text-ok">
          Adicionado como evidência nº {adicionar.data.numero}. O hash foi registrado agora.
        </p>
      )}
    </div>
  );
}
