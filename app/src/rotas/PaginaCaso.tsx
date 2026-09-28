import { useEffect, useState, type ReactNode } from "react";
import { Link, useParams } from "react-router-dom";
import { api, normalizarErro, type Caso, type Dispositivo, type VerificacaoAuditoria } from "../api/cliente";
import {
  useAbrirCaso,
  useAuditoria,
  useCadastrarDispositivo,
  useCasoAtual,
  useDispositivos,
  useEstadoImportacao,
  useExtracoes,
} from "../api/consultas";
import { Botao } from "../componentes/Botao";
import { Aviso, Cabecalho, Cartao, Carregando, Definicoes, Selo, Vazio } from "../componentes/Estrutura";
import { FormDispositivo } from "../componentes/FormDispositivo";
import { AssistenteImportacao, PainelImportacao } from "../componentes/Importacao";
import { bytes, dataHora, identificacaoCaso, numero, tamanho } from "../util/formatar";
import { nomeStatusExtracao, tomStatus } from "../util/rotulos";

const nomeSistema = { android: "Android", ios: "iOS", outro: "Outro", desconhecido: "Desconhecido" } as const;

const nomeAcao: Record<string, string> = {
  caso_criado: "Caso criado",
  caso_aberto: "Caso aberto",
  caso_fechado: "Caso fechado",
  dispositivo_cadastrado: "Dispositivo cadastrado",
  importacao_iniciada: "Importação iniciada",
  importacao_concluida: "Importação concluída",
  importacao_cancelada: "Importação cancelada",
  importacao_falhou: "Importação falhou",
  importacao_interrompida: "Importação interrompida",
};

export function PaginaCaso() {
  return <ComCasoAberto>{(caso, pasta) => <DetalheCaso caso={caso} pasta={pasta} />}</ComCasoAberto>;
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

function DetalheCaso({ caso, pasta }: { caso: Caso; pasta: string }) {
  const fuso = caso.fuso_horario;
  return (
    <>
      <Cabecalho
        titulo={identificacaoCaso(caso)}
        subtitulo={
          <span className="font-mono text-xs" title="Pasta do caso">
            {pasta}
          </span>
        }
      />
      <div className="grid gap-6">
        <Cartao titulo="Procedimento">
          <Definicoes
            itens={[
              ["Tipo", caso.procedimento],
              ["Número do procedimento", caso.numero_procedimento],
              ["Número do BO", caso.numero_bo],
              ["Investigado", caso.investigado],
              ["Vítima", caso.vitima],
              ["Delegado(a)", caso.delegado],
              ["Responsável pela análise", caso.responsavel_analise],
              ["Unidade", caso.unidade],
              ["Criado em", dataHora(caso.criado_em, fuso)],
            ]}
          />
          {caso.observacoes && <p className="mt-4 whitespace-pre-wrap text-sm text-slate-700">{caso.observacoes}</p>}
        </Cartao>
        <SecaoDispositivos caso={caso} />
        <SecaoExtracoes caso={caso} />
        <SecaoAtividades caso={caso} />
      </div>
    </>
  );
}

function SecaoDispositivos({ caso }: { caso: Caso }) {
  const dispositivos = useDispositivos(caso.id);
  const cadastrar = useCadastrarDispositivo(caso.id);
  const [formAberto, setFormAberto] = useState(false);

  return (
    <Cartao
      titulo="Dispositivos"
      acoes={
        !formAberto && (
          <Botao variante="secundario" onClick={() => setFormAberto(true)}>
            Adicionar dispositivo
          </Botao>
        )
      }
    >
      {formAberto && (
        <div className="mb-6 rounded-lg bg-slate-50 p-4 ring-1 ring-slate-200">
          <FormDispositivo
            salvar={(d) => cadastrar.mutateAsync(d).catch((e: unknown) => Promise.reject(normalizarErro(e)))}
            aoConcluir={() => setFormAberto(false)}
            aoCancelar={() => setFormAberto(false)}
          />
        </div>
      )}
      {dispositivos.isPending ? (
        <Carregando />
      ) : dispositivos.isError ? (
        <Aviso>{dispositivos.error.message}</Aviso>
      ) : dispositivos.data.length === 0 ? (
        !formAberto && <Vazio titulo="Nenhum dispositivo cadastrado">Cadastre o aparelho examinado antes de importar extrações.</Vazio>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm" aria-label="Dispositivos do caso">
            <thead className="text-xs uppercase tracking-wide text-slate-500">
              <tr>
                <th className="py-2 pr-4 font-medium">Identificação</th>
                <th className="py-2 pr-4 font-medium">Aparelho</th>
                <th className="py-2 pr-4 font-medium">Sistema</th>
                <th className="py-2 pr-4 font-medium">IMEI</th>
                <th className="py-2 pr-4 font-medium">Capacidade</th>
                <th className="py-2 font-medium">Cadastrado em</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {dispositivos.data.map((d) => (
                <tr key={d.id}>
                  <td className="py-2.5 pr-4 font-medium text-slate-900">{d.apelido}</td>
                  <td className="py-2.5 pr-4">
                    {[d.fabricante, d.nome_comercial ?? d.modelo].filter(Boolean).join(" ") || "—"}
                    {d.nome_comercial && d.modelo && <span className="text-slate-500"> ({d.modelo})</span>}
                  </td>
                  <td className="py-2.5 pr-4">
                    {nomeSistema[d.sistema]} {d.versao_sistema}
                  </td>
                  <td className="py-2.5 pr-4 font-mono text-xs">
                    {[d.imei1, d.imei2].filter(Boolean).join(" / ") || "—"}
                  </td>
                  <td className="py-2.5 pr-4">{bytes(d.capacidade_bytes)}</td>
                  <td className="py-2.5 text-slate-500">{dataHora(d.criado_em, caso.fuso_horario)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Cartao>
  );
}

function SecaoExtracoes({ caso }: { caso: Caso }) {
  const extracoes = useExtracoes(caso.id);
  const dispositivos = useDispositivos(caso.id);
  const importacao = useEstadoImportacao(caso.id);
  const [assistente, setAssistente] = useState(false);
  const [dispensada, setDispensada] = useState<string | null>(null);
  const estado = importacao.data;
  const emCurso = !!estado && !estado.terminada;
  const listaDisp: Dispositivo[] = dispositivos.data ?? [];
  const apelido = (id: string) => listaDisp.find((d) => d.id === id)?.apelido ?? "—";

  return (
    <Cartao
      titulo="Extrações"
      acoes={
        !assistente && (
          <Botao
            variante="secundario"
            onClick={() => setAssistente(true)}
            disabled={emCurso || listaDisp.length === 0}
            title={listaDisp.length === 0 ? "Cadastre um dispositivo primeiro" : undefined}
          >
            Importar extração
          </Botao>
        )
      }
    >
      <div className="space-y-4">
        {assistente && (
          <AssistenteImportacao caso={caso} dispositivos={listaDisp} aoFechar={() => setAssistente(false)} />
        )}
        {estado && dispensada !== estado.extracao_id && (
          <PainelImportacao caso={caso} estado={estado} aoDispensar={() => setDispensada(estado.extracao_id)} />
        )}
        {extracoes.isPending ? (
          <Carregando />
        ) : extracoes.isError ? (
          <Aviso>{extracoes.error.message}</Aviso>
        ) : extracoes.data.length === 0 ? (
          !assistente && (
            <Vazio titulo="Nenhuma extração importada">
              Importe a pasta gerada pela ferramenta de extração. Os originais nunca são alterados.
            </Vazio>
          )
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm" aria-label="Extrações do caso">
              <thead className="text-xs uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="py-2 pr-4 font-medium">Dispositivo</th>
                  <th className="py-2 pr-4 font-medium">Ferramenta</th>
                  <th className="py-2 pr-4 font-medium">Situação</th>
                  <th className="py-2 pr-4 font-medium">Arquivos</th>
                  <th className="py-2 pr-4 font-medium">Registros</th>
                  <th className="py-2 font-medium">Importada em</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100">
                {extracoes.data.map((x) => (
                  <tr key={x.id}>
                    <td className="py-2.5 pr-4">
                      <Link
                        to={`/casos/${caso.id}/extracoes/${x.id}`}
                        className="font-medium text-marca-700 hover:underline"
                      >
                        {apelido(x.dispositivo_id)}
                      </Link>
                      <div className="max-w-xs truncate font-mono text-xs text-slate-500" title={x.origem_caminho}>
                        {x.origem_caminho}
                      </div>
                    </td>
                    <td className="py-2.5 pr-4">{x.importador_nome}</td>
                    <td className="py-2.5 pr-4">
                      <Selo tom={tomStatus(x.status)}>{nomeStatusExtracao[x.status] ?? x.status}</Selo>
                    </td>
                    <td className="py-2.5 pr-4">
                      {numero(x.total_arquivos)} <span className="text-slate-500">({tamanho(x.total_bytes)})</span>
                    </td>
                    <td className="py-2.5 pr-4">{numero(x.total_artefatos)}</td>
                    <td className="py-2.5 text-slate-500">{dataHora(x.iniciada_em, caso.fuso_horario)}</td>
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

function SecaoAtividades({ caso }: { caso: Caso }) {
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
      titulo="Registro de atividades"
      acoes={
        <Botao variante="secundario" onClick={verificar} carregando={verificando}>
          Verificar integridade
        </Botao>
      }
    >
      {verificacao && (
        <div className="mb-4">
          <Aviso tipo="sucesso">
            Trilha íntegra: {verificacao.registros} registros. Hash final{" "}
            <span className="font-mono text-xs">{verificacao.hash_final.slice(0, 16)}…</span>
          </Aviso>
        </div>
      )}
      {erroVerificacao && (
        <div className="mb-4">
          <Aviso>{erroVerificacao}</Aviso>
        </div>
      )}
      {auditoria.isPending ? (
        <Carregando />
      ) : auditoria.isError ? (
        <Aviso>{auditoria.error.message}</Aviso>
      ) : (
        <ol className="space-y-2 text-sm" aria-label="Atividades recentes">
          {auditoria.data.map((r) => (
            <li key={r.seq} className="flex items-baseline gap-3">
              <span className="w-40 shrink-0 text-xs text-slate-500">{dataHora(r.ts, caso.fuso_horario)}</span>
              <span className="text-slate-900">{nomeAcao[r.acao] ?? r.acao}</span>
              <span className="truncate text-xs text-slate-500">
                {r.usuario} · {r.maquina}
              </span>
            </li>
          ))}
        </ol>
      )}
    </Cartao>
  );
}
