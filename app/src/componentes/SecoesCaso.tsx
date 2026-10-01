// Seções do caso usadas no Dashboard e nas páginas próprias (Dispositivos, Extrações).
// `compacto` = cartões para a visão geral; sem ele, tabela completa.

import { useState } from "react";
import { Link } from "react-router-dom";
import { api, normalizarErro, type Caso, type ConferenciaDerivados, type Dispositivo, type VerificacaoAuditoria } from "../api/cliente";
import { useAuditoria, useCadastrarDispositivo, useDispositivos, useEstadoImportacao, useExtracoes, useReconstruirDerivados } from "../api/consultas";
import { bytes, dataHora, duracao, numero, tamanho } from "../util/formatar";
import { nomeStatusExtracao, tomStatus } from "../util/rotulos";
import { Botao } from "./Botao";
import { Aviso, Cartao, Carregando, HashCurto, Selo, tabela, Vazio } from "./Estrutura";
import { FormDispositivo } from "./FormDispositivo";
import { Icone } from "./Icone";
import { AssistenteImportacao, PainelImportacao } from "./Importacao";

export const nomeSistema = { android: "Android", ios: "iOS", outro: "Outro", desconhecido: "Desconhecido" } as const;

export const nomeAcao: Record<string, string> = {
  caso_criado: "Caso criado",
  caso_aberto: "Caso aberto",
  caso_fechado: "Caso fechado",
  dispositivo_cadastrado: "Dispositivo cadastrado",
  importacao_iniciada: "Importação iniciada",
  importacao_concluida: "Importação concluída",
  importacao_cancelada: "Importação cancelada",
  importacao_falhou: "Importação falhou",
  importacao_interrompida: "Importação interrompida",
  derivados_reconstruidos: "Conversas e índices reconstruídos",
};

function aparelho(d: Dispositivo) {
  return [d.fabricante, d.nome_comercial ?? d.modelo].filter(Boolean).join(" ") || "—";
}

// ---------------------------------------------------------------- Dispositivos

export function SecaoDispositivos({ caso, compacto }: { caso: Caso; compacto?: boolean }) {
  const dispositivos = useDispositivos(caso.id);
  const cadastrar = useCadastrarDispositivo(caso.id);
  const [formAberto, setFormAberto] = useState(false);
  const fuso = caso.fuso_horario;

  return (
    <Cartao
      titulo={`Dispositivos${dispositivos.data?.length ? ` (${dispositivos.data.length})` : ""}`}
      icone="dispositivo"
      denso
      acoes={
        !formAberto && (
          <Botao variante="secundario" icone="mais" onClick={() => setFormAberto(true)}>
            Adicionar dispositivo
          </Botao>
        )
      }
    >
      {formAberto && (
        <div className="border-b border-borda bg-sup-2/50 p-4">
          <FormDispositivo
            salvar={(d) => cadastrar.mutateAsync(d).catch((e: unknown) => Promise.reject(normalizarErro(e)))}
            aoConcluir={() => setFormAberto(false)}
            aoCancelar={() => setFormAberto(false)}
          />
        </div>
      )}
      <div className={compacto ? "p-3" : ""}>
        {dispositivos.isPending ? (
          <div className="px-4">
            <Carregando />
          </div>
        ) : dispositivos.isError ? (
          <div className="p-4">
            <Aviso>{dispositivos.error.message}</Aviso>
          </div>
        ) : dispositivos.data.length === 0 ? (
          !formAberto && (
            <div className={compacto ? "" : "p-4"}>
              <Vazio titulo="Nenhum dispositivo cadastrado" icone="dispositivo">
                Cadastre o aparelho examinado antes de importar extrações.
              </Vazio>
            </div>
          )
        ) : compacto ? (
          <ul className="grid gap-3 lg:grid-cols-2 2xl:grid-cols-3" aria-label="Dispositivos do caso">
            {dispositivos.data.map((d) => (
              <li key={d.id} className="rounded-lg border border-borda bg-sup-2/50 p-3">
                <div className="flex items-start gap-3">
                  <Icone nome="dispositivo" className="mt-0.5 size-4 shrink-0 text-texto-3" />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center justify-between gap-2">
                      <p className="truncate font-medium text-texto">{d.apelido}</p>
                      <Selo tom="neutro">
                        {nomeSistema[d.sistema]} {d.versao_sistema}
                      </Selo>
                    </div>
                    <p className="truncate text-xs text-texto-2">
                      {aparelho(d)}
                      {d.nome_comercial && d.modelo && <span className="text-texto-3"> ({d.modelo})</span>}
                    </p>
                  </div>
                </div>
                <dl className="mt-2.5 grid grid-cols-2 gap-x-3 gap-y-1.5 text-xs">
                  <div className="col-span-2 min-w-0">
                    <dt className="text-texto-3">IMEI</dt>
                    <dd className="truncate font-mono text-texto">{[d.imei1, d.imei2].filter(Boolean).join(" / ") || "—"}</dd>
                  </div>
                  <div>
                    <dt className="text-texto-3">Capacidade</dt>
                    <dd className="text-texto">{bytes(d.capacidade_bytes)}</dd>
                  </div>
                  <div>
                    <dt className="text-texto-3">Linha</dt>
                    <dd className="truncate text-texto">{[d.numero, d.operadora].filter(Boolean).join(" · ") || "—"}</dd>
                  </div>
                </dl>
              </li>
            ))}
          </ul>
        ) : (
          <div className="overflow-x-auto">
            <table className={tabela.base} aria-label="Dispositivos do caso">
              <thead className={tabela.cabeca}>
                <tr>
                  <th className={tabela.th}>Identificação</th>
                  <th className={tabela.th}>Aparelho</th>
                  <th className={tabela.th}>Sistema</th>
                  <th className={tabela.th}>IMEI</th>
                  <th className={tabela.th}>Série</th>
                  <th className={tabela.th}>Linha / operadora</th>
                  <th className={tabela.th}>Capacidade</th>
                  <th className={tabela.th}>Cadastrado em</th>
                </tr>
              </thead>
              <tbody className={tabela.corpo}>
                {dispositivos.data.map((d) => (
                  <tr key={d.id} className={tabela.linha}>
                    <td className={`${tabela.td} font-medium text-texto`}>
                      {d.apelido}
                      {d.observacoes && <div className="mt-0.5 max-w-xs text-xs font-normal text-texto-3">{d.observacoes}</div>}
                    </td>
                    <td className={tabela.td}>
                      {aparelho(d)}
                      {d.nome_comercial && d.modelo && <span className="text-texto-3"> ({d.modelo})</span>}
                    </td>
                    <td className={tabela.td}>
                      {nomeSistema[d.sistema]} {d.versao_sistema}
                    </td>
                    <td className={`${tabela.td} font-mono text-xs`}>{[d.imei1, d.imei2].filter(Boolean).join(" / ") || "—"}</td>
                    <td className={`${tabela.td} font-mono text-xs`}>{d.serial ?? "—"}</td>
                    <td className={tabela.td}>{[d.numero, d.operadora].filter(Boolean).join(" · ") || "—"}</td>
                    <td className={`${tabela.td} tabular`}>{bytes(d.capacidade_bytes)}</td>
                    <td className={`${tabela.td} tabular text-texto-2`}>{dataHora(d.criado_em, fuso)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </Cartao>
  );
}

// ---------------------------------------------------------------- Extrações

export function SecaoExtracoes({ caso, compacto }: { caso: Caso; compacto?: boolean }) {
  const extracoes = useExtracoes(caso.id);
  const dispositivos = useDispositivos(caso.id);
  const importacao = useEstadoImportacao(caso.id);
  const [assistente, setAssistente] = useState(false);
  const [dispensada, setDispensada] = useState<string | null>(null);
  const estado = importacao.data;
  const emCurso = !!estado && !estado.terminada;
  const listaDisp: Dispositivo[] = dispositivos.data ?? [];
  const apelido = (id: string) => listaDisp.find((d) => d.id === id)?.apelido ?? "—";
  const fuso = caso.fuso_horario;

  return (
    <Cartao
      titulo={`Extrações${extracoes.data?.length ? ` (${extracoes.data.length})` : ""}`}
      icone="extracao"
      denso
      acoes={
        !assistente && (
          <Botao
            variante="secundario"
            icone="extracao"
            onClick={() => setAssistente(true)}
            disabled={emCurso || listaDisp.length === 0}
            title={listaDisp.length === 0 ? "Cadastre um dispositivo primeiro" : emCurso ? "Há uma importação em andamento" : undefined}
          >
            Importar extração
          </Botao>
        )
      }
    >
      <div className="space-y-3 p-3">
        {assistente && <AssistenteImportacao caso={caso} dispositivos={listaDisp} aoFechar={() => setAssistente(false)} />}
        {estado && dispensada !== estado.extracao_id && (
          <PainelImportacao caso={caso} estado={estado} aoDispensar={() => setDispensada(estado.extracao_id)} />
        )}
        {extracoes.isPending ? (
          <Carregando />
        ) : extracoes.isError ? (
          <Aviso>{extracoes.error.message}</Aviso>
        ) : extracoes.data.length === 0 ? (
          !assistente && (
            <Vazio titulo="Nenhuma extração importada" icone="extracao">
              Importe a pasta gerada pela ferramenta de extração. Os originais nunca são alterados.
            </Vazio>
          )
        ) : compacto ? (
          <ul className="grid gap-3 lg:grid-cols-2 2xl:grid-cols-3" aria-label="Extrações do caso">
            {extracoes.data.map((x) => (
              <li key={x.id}>
                <Link
                  to={`/casos/${caso.id}/extracoes/${x.id}`}
                  className="block rounded-lg border border-borda bg-sup-2/60 p-3 transition-colors hover:border-borda-2 hover:bg-sup-2"
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <p className="truncate font-medium text-texto">{apelido(x.dispositivo_id)}</p>
                      <p className="truncate text-xs text-texto-2">
                        {x.importador_nome} · {dataHora(x.iniciada_em, fuso)}
                      </p>
                    </div>
                    <Selo tom={tomStatus(x.status)}>{nomeStatusExtracao[x.status] ?? x.status}</Selo>
                  </div>
                  <div className="mt-2.5 grid grid-cols-3 gap-2 text-xs">
                    <div>
                      <p className="text-texto-3">Arquivos</p>
                      <p className="tabular text-texto">{numero(x.total_arquivos)}</p>
                    </div>
                    <div>
                      <p className="text-texto-3">Tamanho</p>
                      <p className="tabular text-texto">{tamanho(x.total_bytes)}</p>
                    </div>
                    <div>
                      <p className="text-texto-3">Registros</p>
                      <p className="tabular text-texto">{numero(x.total_artefatos)}</p>
                    </div>
                  </div>
                  <p className="mt-2 truncate font-mono text-2xs text-texto-3" title={x.origem_caminho}>
                    {x.origem_caminho}
                  </p>
                </Link>
              </li>
            ))}
          </ul>
        ) : (
          <div className="-mx-3 overflow-x-auto">
            <table className={tabela.base} aria-label="Extrações do caso">
              <thead className={tabela.cabeca}>
                <tr>
                  <th className={tabela.th}>Dispositivo</th>
                  <th className={tabela.th}>Ferramenta</th>
                  <th className={tabela.th}>Situação</th>
                  <th className={`${tabela.th} text-right`}>Arquivos</th>
                  <th className={`${tabela.th} text-right`}>Registros</th>
                  <th className={`${tabela.th} text-right`}>Avisos</th>
                  <th className={tabela.th}>Manifesto</th>
                  <th className={tabela.th}>Importada em</th>
                  <th className={tabela.th}>Duração</th>
                </tr>
              </thead>
              <tbody className={tabela.corpo}>
                {extracoes.data.map((x) => (
                  <tr key={x.id} className={tabela.linha}>
                    <td className={tabela.td}>
                      <Link to={`/casos/${caso.id}/extracoes/${x.id}`} className="font-medium text-ouro-claro hover:underline">
                        {apelido(x.dispositivo_id)}
                      </Link>
                      <div className="max-w-xs truncate font-mono text-2xs text-texto-3" title={x.origem_caminho}>
                        {x.origem_caminho}
                      </div>
                    </td>
                    <td className={tabela.td}>
                      {x.importador_nome} <span className="text-xs text-texto-3">v{x.importador_versao}</span>
                    </td>
                    <td className={tabela.td}>
                      <Selo tom={tomStatus(x.status)}>{nomeStatusExtracao[x.status] ?? x.status}</Selo>
                    </td>
                    <td className={`${tabela.td} tabular text-right`}>
                      {numero(x.total_arquivos)} <span className="text-texto-3">({tamanho(x.total_bytes)})</span>
                    </td>
                    <td className={`${tabela.td} tabular text-right`}>{numero(x.total_artefatos)}</td>
                    <td className={`${tabela.td} tabular text-right`}>{numero(x.total_avisos)}</td>
                    <td className={tabela.td}>
                      <HashCurto valor={x.hash_manifesto} />
                    </td>
                    <td className={`${tabela.td} tabular whitespace-nowrap text-texto-2`}>{dataHora(x.iniciada_em, fuso)}</td>
                    <td className={`${tabela.td} tabular whitespace-nowrap text-texto-2`}>{duracao(x.iniciada_em, x.concluida_em)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </Cartao>
  );
}

// ---------------------------------------------------------------- Auditoria

export function SecaoAtividades({ caso }: { caso: Caso }) {
  const auditoria = useAuditoria(caso.id);
  const [verificacao, setVerificacao] = useState<VerificacaoAuditoria | null>(null);
  const [erroVerificacao, setErroVerificacao] = useState<string | null>(null);
  const [verificando, setVerificando] = useState(false);

  async function verificar() {
    setVerificando(true);
    setErroVerificacao(null);
    try {
      setVerificacao(await api.verificarAuditoria());
    } catch (e) {
      setVerificacao(null);
      setErroVerificacao(normalizarErro(e).message);
    } finally {
      setVerificando(false);
    }
  }

  return (
    <Cartao
      titulo="Auditoria"
      icone="integridade"
      acoes={
        <Botao variante="secundario" icone="integridade" onClick={verificar} carregando={verificando}>
          Verificar integridade
        </Botao>
      }
    >
      <div className="mb-3">
        {verificacao ? (
          <Aviso tipo="sucesso">
            Trilha íntegra: {verificacao.registros} registros. Hash final{" "}
            <span className="font-mono text-xs" title={verificacao.hash_final}>
              {verificacao.hash_final.slice(0, 16)}…
            </span>
          </Aviso>
        ) : erroVerificacao ? (
          <Aviso>{erroVerificacao}</Aviso>
        ) : (
          <p className="text-xs text-texto-3">
            Cada atividade é encadeada por SHA-256 à anterior. A verificação recalcula toda a cadeia.
          </p>
        )}
      </div>
      <DadosDerivados />
      {auditoria.isPending ? (
        <Carregando />
      ) : auditoria.isError ? (
        <Aviso>{auditoria.error.message}</Aviso>
      ) : (
        <ol className="relative space-y-2.5 border-l border-borda-2 pl-4 text-sm" aria-label="Atividades recentes">
          {auditoria.data.map((r) => (
            <li key={r.seq} className="relative">
              <span className="absolute -left-[21px] top-1.5 size-2 rounded-full bg-borda-2 ring-4 ring-sup" aria-hidden />
              <p className="text-texto">{nomeAcao[r.acao] ?? r.acao}</p>
              <p className="tabular text-2xs text-texto-3">
                {dataHora(r.ts, caso.fuso_horario)} · {r.usuario} · {r.maquina}
              </p>
            </li>
          ))}
        </ol>
      )}
    </Cartao>
  );
}

const nomeDerivado: Record<string, string> = {
  conversa: "Conversas",
  conversa_participante: "Participantes das conversas",
  busca_artefato: "Índice de busca dos registros",
  busca_arquivo: "Índice de busca dos arquivos",
};

/** Conversas, participantes e índices de busca são calculados a partir dos registros
 * importados. Aqui o analista confere se batem e, se não, manda recalcular. */
function DadosDerivados() {
  const [conferencia, setConferencia] = useState<ConferenciaDerivados | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [conferindo, setConferindo] = useState(false);
  const reconstruir = useReconstruirDerivados();

  async function conferir() {
    setConferindo(true);
    setErro(null);
    reconstruir.reset();
    try {
      setConferencia(await api.conferirDerivados());
    } catch (e) {
      setConferencia(null);
      setErro(normalizarErro(e).message);
    } finally {
      setConferindo(false);
    }
  }

  const divergentes = conferencia?.tabelas.filter((t) => t.faltando > 0 || t.sobrando > 0 || !t.indice_integro) ?? [];

  return (
    <section className="mb-4 rounded-md border border-borda bg-sup-2 px-3.5 py-3" aria-label="Conversas e índices">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="text-sm text-texto">Conversas e índices de busca</p>
          <p className="text-xs text-texto-3">Calculados a partir dos registros importados, que nunca são alterados.</p>
        </div>
        <Botao variante="secundario" tamanho="p" icone="banco" onClick={conferir} carregando={conferindo} disabled={reconstruir.isPending}>
          Conferir
        </Botao>
      </div>
      {erro && (
        <div className="mt-2.5">
          <Aviso>{erro}</Aviso>
        </div>
      )}
      {reconstruir.isSuccess ? (
        <div className="mt-2.5">
          <Aviso tipo="sucesso">Reconstruído: conversas e índices conferem com os registros. A ação ficou na auditoria.</Aviso>
        </div>
      ) : reconstruir.isError ? (
        <div className="mt-2.5">
          <Aviso>{normalizarErro(reconstruir.error).message}</Aviso>
        </div>
      ) : conferencia?.consistente ? (
        <div className="mt-2.5">
          <Aviso tipo="sucesso">Conferem com os registros importados.</Aviso>
        </div>
      ) : conferencia ? (
        <div className="mt-2.5">
          <Aviso tipo="atencao">
            <p>Diferenças encontradas:</p>
            <ul className="mt-1 space-y-0.5 text-xs" aria-label="Diferenças nos dados derivados">
              {divergentes.map((t) => (
                <li key={t.tabela}>
                  {nomeDerivado[t.tabela] ?? t.tabela}:{" "}
                  {[
                    t.faltando > 0 && `${numero(t.faltando)} faltando`,
                    t.sobrando > 0 && `${numero(t.sobrando)} a mais ou alteradas`,
                    !t.indice_integro && "índice danificado",
                  ]
                    .filter(Boolean)
                    .join(", ")}
                </li>
              ))}
            </ul>
            <p className="mt-1.5 text-xs">Reconstruir recalcula tudo a partir dos registros importados e fica registrado na auditoria.</p>
            <Botao className="mt-2" tamanho="p" icone="integridade" onClick={() => reconstruir.mutate()} carregando={reconstruir.isPending}>
              Reconstruir
            </Botao>
          </Aviso>
        </div>
      ) : null}
    </section>
  );
}
