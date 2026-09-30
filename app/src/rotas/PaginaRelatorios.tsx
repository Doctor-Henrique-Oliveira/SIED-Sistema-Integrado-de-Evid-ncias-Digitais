// Relatórios: a lista de evidências selecionadas (ordem, textos, remoção com motivo)
// e a geração do PDF. O backend confere o hash de cada evidência, relê os arquivos na
// pasta da extração (somente leitura) e grava o PDF e o .sha256 em <caso>/relatorios.

import { useState } from "react";
import { Link } from "react-router-dom";
import type { Caso, Evidencia, SecoesRelatorio } from "../api/cliente";
import {
  useAlterarEvidencia,
  useEvidencias,
  useGerarRelatorio,
  useMoverEvidencia,
  useRelatorios,
  useRemoverEvidencia,
} from "../api/consultas";
import { Botao } from "../componentes/Botao";
import { AreaTexto, Campo } from "../componentes/Campo";
import { Aviso, Cabecalho, Cartao, Carregando, Hash, HashCurto, Selo, Vazio } from "../componentes/Estrutura";
import { Icone } from "../componentes/Icone";
import { dataHora, dataHoraMs, identificacaoCaso, nomeArquivo, numero, tamanho } from "../util/formatar";
import { nomeTipoArtefato, rotuloApp, rotuloSubtipo } from "../util/rotulos";
import { ComCasoAberto } from "./PaginaCaso";

export function PaginaRelatorios() {
  return <ComCasoAberto>{(caso, pasta) => <Relatorios caso={caso} pasta={pasta} />}</ComCasoAberto>;
}

function Relatorios({ caso, pasta }: { caso: Caso; pasta: string }) {
  return (
    <>
      <Cabecalho
        icone="relatorios"
        sobre={identificacaoCaso(caso)}
        titulo="Relatórios"
        subtitulo="Evidências selecionadas e relatório em PDF, com hashes, origem técnica e cadeia de custódia."
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_26rem]">
        <ListaEvidencias caso={caso} />
        <div className="flex min-w-0 flex-col gap-4">
          <GerarRelatorio caso={caso} pasta={pasta} />
          <RelatoriosGerados caso={caso} pasta={pasta} />
        </div>
      </div>
    </>
  );
}

// ---------------------------------------------------------------- Evidências

function tipoEvidencia(e: Evidencia): string {
  if (e.tipo === "trecho") return `Trecho · ${numero(e.itens)} mensagens`;
  if (e.tipo === "arquivo") return "Arquivo";
  return "Registro";
}

/** Descrição do alvo a partir dos próprios dados (sem inventar nada). */
function descricao(e: Evidencia, fuso: string): string {
  if (e.arquivo) return e.arquivo.caminho;
  const r = e.registro;
  if (!r) return "—";
  if (e.tipo === "trecho" && e.registro_fim)
    return `${rotuloApp(r.app)} · registros nº ${r.id} a ${e.registro_fim.id} · ${dataHoraMs(r.data_hora_ms, fuso)} a ${dataHoraMs(e.registro_fim.data_hora_ms, fuso)}`;
  const quem = r.de_mim ? "Enviada" : r.remetente ? `De ${r.remetente}` : null;
  const tipo = r.subtipo && r.subtipo !== "texto" ? rotuloSubtipo(r.subtipo) : (nomeTipoArtefato[r.tipo] ?? r.tipo);
  return [rotuloApp(r.app ?? r.tipo), tipo, quem, dataHoraMs(r.data_hora_ms, fuso), `registro nº ${r.id}`].filter(Boolean).join(" · ");
}

function tituloPadrao(e: Evidencia): string {
  if (e.arquivo) return nomeArquivo(e.arquivo.caminho);
  if (e.tipo === "trecho") return "Trecho de conversa";
  const r = e.registro;
  return r?.conteudo ?? (r ? (nomeTipoArtefato[r.tipo] ?? r.tipo) : "Registro");
}

function ListaEvidencias({ caso }: { caso: Caso }) {
  const lista = useEvidencias(caso.id);
  return (
    <Cartao
      titulo="Evidências selecionadas"
      icone="lista"
      rotulo="Evidências selecionadas"
      denso
      acoes={lista.data && <Selo tom={lista.data.length ? "ouro" : "neutro"}>{numero(lista.data.length)}</Selo>}
    >
      {lista.isPending ? (
        <div className="px-4">
          <Carregando />
        </div>
      ) : lista.isError ? (
        <div className="p-4">
          <Aviso>{lista.error.message}</Aviso>
        </div>
      ) : lista.data.length === 0 ? (
        <div className="p-4">
          <Vazio titulo="Nenhuma evidência selecionada" icone="relatorios">
            Em Conversas, selecione uma mensagem e use “Adicionar às evidências” (ou marque o início e o fim de um trecho). Em Mídias e
            Arquivos, abra os detalhes do arquivo. O hash de cada item é registrado no momento da marcação.
          </Vazio>
        </div>
      ) : (
        <ol className="divide-y divide-borda">
          {lista.data.map((e) => (
            <ItemEvidencia key={e.id} caso={caso} e={e} total={lista.data.length} />
          ))}
        </ol>
      )}
    </Cartao>
  );
}

function ItemEvidencia({ caso, e, total }: { caso: Caso; e: Evidencia; total: number }) {
  const [modo, setModo] = useState<"ver" | "editar" | "remover">("ver");
  const mover = useMoverEvidencia(caso.id);
  const ref = `E-${e.numero}`;
  const origem =
    e.conversa_id != null && e.registro
      ? `/casos/${caso.id}/conversas/${e.conversa_id}?registro=${e.registro.id}`
      : e.arquivo
        ? `/casos/${caso.id}/extracoes/${e.arquivo.extracao_id}`
        : null;
  return (
    <li className="px-4 py-3" aria-label={`Evidência ${ref}`}>
      <div className="flex items-start gap-3">
        <span className="tabular mt-0.5 w-11 shrink-0 font-mono text-sm font-semibold text-ouro-claro">{ref}</span>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-1.5">
            <p className="min-w-0 truncate font-medium text-texto">{e.titulo ?? tituloPadrao(e)}</p>
            <Selo tom="info">{tipoEvidencia(e)}</Selo>
            {e.categoria && <Selo>{e.categoria}</Selo>}
          </div>
          <p className="mt-0.5 break-words text-xs text-texto-2">{descricao(e, caso.fuso_horario)}</p>
          {e.descricao_contextual && <p className="mt-1 text-xs text-texto">{e.descricao_contextual}</p>}
          {e.observacao && (
            <p className="mt-1 text-xs text-texto-2">
              <span className="text-texto-3">Observação: </span>
              {e.observacao}
            </p>
          )}
          <p className="mt-1 text-2xs text-texto-3">
            Marcada por {e.criada_por} em {dataHora(e.criada_em, caso.fuso_horario)} · hash na seleção <HashCurto valor={e.hash_no_momento} />
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-0.5">
          <button
            type="button"
            aria-label={`Subir ${ref}`}
            disabled={e.numero === 1 || mover.isPending}
            onClick={() => mover.mutate({ id: e.id, numero: e.numero - 1 })}
            className="rounded p-1 text-texto-2 hover:bg-sup-2 hover:text-texto disabled:opacity-30"
          >
            <Icone nome="abaixo" className="size-4 rotate-180" />
          </button>
          <button
            type="button"
            aria-label={`Descer ${ref}`}
            disabled={e.numero === total || mover.isPending}
            onClick={() => mover.mutate({ id: e.id, numero: e.numero + 1 })}
            className="rounded p-1 text-texto-2 hover:bg-sup-2 hover:text-texto disabled:opacity-30"
          >
            <Icone nome="abaixo" className="size-4" />
          </button>
        </div>
      </div>
      {modo === "ver" ? (
        <div className="mt-2 flex flex-wrap items-center gap-3 pl-14 text-xs">
          <button type="button" onClick={() => setModo("editar")} className="text-ouro-claro hover:underline">
            Editar textos
          </button>
          {origem && (
            <Link to={origem} className="text-ouro-claro hover:underline">
              Ver na origem
            </Link>
          )}
          <button type="button" onClick={() => setModo("remover")} className="text-texto-2 hover:text-erro hover:underline">
            Remover
          </button>
        </div>
      ) : modo === "editar" ? (
        <EditarEvidencia caso={caso} e={e} fechar={() => setModo("ver")} />
      ) : (
        <RemoverEvidencia caso={caso} e={e} fechar={() => setModo("ver")} />
      )}
      {mover.isError && <p role="alert" className="mt-1 pl-14 text-xs text-erro">{mover.error.message}</p>}
    </li>
  );
}

function EditarEvidencia({ caso, e, fechar }: { caso: Caso; e: Evidencia; fechar: () => void }) {
  const alterar = useAlterarEvidencia(caso.id);
  const [textos, setTextos] = useState({
    categoria: e.categoria ?? "",
    titulo: e.titulo ?? "",
    observacao: e.observacao ?? "",
    descricao_contextual: e.descricao_contextual ?? "",
  });
  const campo = (k: keyof typeof textos) => ({
    value: textos[k],
    onChange: (ev: { target: { value: string } }) => setTextos({ ...textos, [k]: ev.target.value }),
  });
  return (
    <form
      className="mt-3 grid gap-3 pl-14 sm:grid-cols-2"
      aria-label={`Editar E-${e.numero}`}
      onSubmit={(ev) => {
        ev.preventDefault();
        alterar.mutate({ id: e.id, textos }, { onSuccess: fechar });
      }}
    >
      <Campo rotulo="Título" placeholder={tituloPadrao(e)} {...campo("titulo")} />
      <Campo rotulo="Categoria" placeholder="Ex.: Ameaças, Localização" {...campo("categoria")} />
      <div className="sm:col-span-2">
        <AreaTexto rotulo="Descrição contextual" {...campo("descricao_contextual")} />
      </div>
      <div className="sm:col-span-2">
        <AreaTexto rotulo="Observação do analista" {...campo("observacao")} />
      </div>
      {alterar.isError && (
        <div className="sm:col-span-2">
          <Aviso>{alterar.error.message}</Aviso>
        </div>
      )}
      <div className="flex gap-2 sm:col-span-2">
        <Botao type="submit" carregando={alterar.isPending}>
          Salvar
        </Botao>
        <Botao variante="fantasma" onClick={fechar}>
          Cancelar
        </Botao>
        <p className="self-center text-2xs text-texto-3">O alvo e o hash não mudam; a alteração vai para a trilha de auditoria.</p>
      </div>
    </form>
  );
}

function RemoverEvidencia({ caso, e, fechar }: { caso: Caso; e: Evidencia; fechar: () => void }) {
  const remover = useRemoverEvidencia(caso.id);
  const [motivo, setMotivo] = useState("");
  return (
    <form
      className="mt-3 space-y-2 pl-14"
      aria-label={`Remover E-${e.numero}`}
      onSubmit={(ev) => {
        ev.preventDefault();
        remover.mutate({ id: e.id, motivo: motivo.trim() || null });
      }}
    >
      <AreaTexto
        rotulo="Motivo da remoção"
        ajuda="Fica registrado com a data e o analista. A evidência sai da lista, mas não é apagada do banco do caso."
        value={motivo}
        onChange={(ev) => setMotivo(ev.target.value)}
      />
      {remover.isError && <Aviso>{remover.error.message}</Aviso>}
      <div className="flex gap-2">
        <Botao type="submit" variante="perigo" carregando={remover.isPending}>
          Remover E-{e.numero}
        </Botao>
        <Botao variante="fantasma" onClick={fechar}>
          Cancelar
        </Botao>
      </div>
    </form>
  );
}

// ---------------------------------------------------------------- Geração

const SECOES: [keyof SecoesRelatorio, string][] = [
  ["capa", "Capa"],
  ["procedimento", "Identificação do procedimento"],
  ["dispositivos", "Dispositivos"],
  ["aquisicao", "Extrações e hashes"],
  ["metodologia", "Metodologia"],
  ["evidencias", "Evidências selecionadas"],
  ["observacoes", "Observações"],
  ["conclusao", "Conclusão do analista"],
  ["apendice_tecnico", "Apêndice técnico (origem)"],
  ["integridade", "Integridade e cadeia de custódia"],
];

const todas = Object.fromEntries(SECOES.map(([k]) => [k, true])) as SecoesRelatorio;

function caminhoCompleto(pasta: string, relativo: string): string {
  const sep = pasta.includes("\\") ? "\\" : "/";
  return `${pasta.replace(/[\\/]+$/, "")}${sep}${relativo.split("/").join(sep)}`;
}

type Conferencia = { numero: number; situacao: string };

function conferencias(v: unknown): Conferencia[] {
  return Array.isArray(v) ? (v as Conferencia[]) : [];
}

function ResumoConferencia({ v }: { v: unknown }) {
  const c = conferencias(v);
  if (c.length === 0) return <>nenhuma evidência</>;
  const div = c.filter((x) => x.situacao === "divergente");
  const ind = c.filter((x) => x.situacao === "indisponivel");
  const ok = c.length - div.length - ind.length;
  return (
    <>
      {numero(ok)} de {numero(c.length)} conferidas
      {div.length > 0 && <span className="text-erro"> · divergentes: {div.map((x) => `E-${x.numero}`).join(", ")}</span>}
      {ind.length > 0 && <span className="text-atencao"> · sem conferência: {ind.map((x) => `E-${x.numero}`).join(", ")}</span>}
    </>
  );
}

function GerarRelatorio({ caso, pasta }: { caso: Caso; pasta: string }) {
  const gerar = useGerarRelatorio(caso.id);
  const evidencias = useEvidencias(caso.id);
  const [titulo, setTitulo] = useState("");
  const [observacoes, setObservacoes] = useState("");
  const [conclusao, setConclusao] = useState("");
  const [secoes, setSecoes] = useState<SecoesRelatorio>(todas);
  const r = gerar.data;
  return (
    <Cartao titulo="Gerar relatório em PDF" icone="relatorios" rotulo="Gerar relatório">
      <form
        className="space-y-3"
        onSubmit={(ev) => {
          ev.preventDefault();
          gerar.mutate({
            titulo: titulo.trim() || undefined,
            observacoes: observacoes.trim() || undefined,
            conclusao: conclusao.trim() || undefined,
            secoes,
          });
        }}
      >
        <Campo rotulo="Título" placeholder="Relatório de análise de evidências digitais" value={titulo} onChange={(e) => setTitulo(e.target.value)} />
        <fieldset>
          <legend className="text-xs font-medium text-texto-2">Seções</legend>
          <div className="mt-1.5 grid grid-cols-2 gap-x-3 gap-y-1">
            {SECOES.map(([k, rotulo]) => (
              <label key={k} className="flex items-center gap-2 text-xs text-texto">
                <input
                  type="checkbox"
                  checked={secoes[k]}
                  onChange={(e) => setSecoes({ ...secoes, [k]: e.target.checked })}
                  className="accent-[var(--color-ouro)]"
                />
                {rotulo}
              </label>
            ))}
          </div>
        </fieldset>
        <AreaTexto rotulo="Texto das observações" value={observacoes} onChange={(e) => setObservacoes(e.target.value)} />
        <AreaTexto rotulo="Texto da conclusão" value={conclusao} onChange={(e) => setConclusao(e.target.value)} />
        <div className="flex flex-wrap items-center gap-3">
          <Botao type="submit" icone="relatorios" carregando={gerar.isPending}>
            Gerar PDF
          </Botao>
          <span className="text-2xs text-texto-3">
            {gerar.isPending
              ? "Conferindo hashes, relendo arquivos na extração e diagramando…"
              : `${numero(evidencias.data?.length ?? 0)} evidências na ordem da lista.`}
          </span>
        </div>
      </form>
      {gerar.isError && (
        <div className="mt-3">
          <Aviso>{gerar.error.message}</Aviso>
        </div>
      )}
      {r && !gerar.isPending && (
        <div className="mt-3">
          <Aviso tipo={conferencias(r.evidencias).some((c) => c.situacao !== "conferida") ? "atencao" : "sucesso"}>
            <p className="font-medium">Relatório gerado</p>
            <p className="mt-1 break-all font-mono text-xs">{caminhoCompleto(pasta, r.caminho)}</p>
            <p className="mt-1 text-xs">
              SHA-256 <Hash valor={r.sha256} />
            </p>
            <p className="mt-1 text-xs">
              {tamanho(r.tamanho)} · <ResumoConferencia v={r.evidencias} />
            </p>
          </Aviso>
        </div>
      )}
      <p className="mt-3 text-2xs leading-snug text-texto-3">
        O PDF e o arquivo .sha256 ficam na pasta do caso, em “relatorios”. A geração relê os arquivos marcados na pasta da extração, somente
        leitura, e fica registrada na trilha de auditoria. Se a trilha não conferir, o relatório não é gerado.
      </p>
    </Cartao>
  );
}

function RelatoriosGerados({ caso, pasta }: { caso: Caso; pasta: string }) {
  const lista = useRelatorios(caso.id);
  return (
    <Cartao titulo="Relatórios gerados" icone="integridade" rotulo="Relatórios gerados" denso>
      {lista.isPending ? (
        <div className="px-4">
          <Carregando />
        </div>
      ) : lista.isError ? (
        <div className="p-4">
          <Aviso>{lista.error.message}</Aviso>
        </div>
      ) : lista.data.length === 0 ? (
        <p className="px-4 py-3 text-sm text-texto-2">Nenhum relatório gerado neste caso.</p>
      ) : (
        <ul className="divide-y divide-borda">
          {lista.data.map((r) => (
            <li key={r.id} className="px-4 py-2.5 text-xs">
              <p className="break-all font-mono text-texto" title={caminhoCompleto(pasta, r.caminho)}>
                {nomeArquivo(r.caminho)}
              </p>
              <p className="mt-0.5 text-texto-2">
                {dataHora(r.gerado_em, caso.fuso_horario)} · {r.gerado_por} · {tamanho(r.tamanho)} · <ResumoConferencia v={r.evidencias} />
              </p>
              <p className="mt-0.5 text-texto-3">
                SHA-256 <HashCurto valor={r.sha256} tamanho={24} /> · trilha até o registro {numero(r.auditoria_ate_seq)}
              </p>
            </li>
          ))}
        </ul>
      )}
    </Cartao>
  );
}

