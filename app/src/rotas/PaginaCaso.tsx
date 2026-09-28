import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { api, normalizarErro, type Caso, type VerificacaoAuditoria } from "../api/cliente";
import { useAbrirCaso, useAuditoria, useCadastrarDispositivo, useCasoAtual, useDispositivos } from "../api/consultas";
import { Botao } from "../componentes/Botao";
import { Aviso, Cabecalho, Cartao, Carregando, Definicoes, Vazio } from "../componentes/Estrutura";
import { FormDispositivo } from "../componentes/FormDispositivo";
import { bytes, dataHora, identificacaoCaso } from "../util/formatar";

const nomeSistema = { android: "Android", ios: "iOS", outro: "Outro", desconhecido: "Desconhecido" } as const;

const nomeAcao: Record<string, string> = {
  caso_criado: "Caso criado",
  caso_aberto: "Caso aberto",
  caso_fechado: "Caso fechado",
  dispositivo_cadastrado: "Dispositivo cadastrado",
};

export function PaginaCaso() {
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
  return <DetalheCaso caso={atual.data.caso} pasta={atual.data.pasta} />;
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
