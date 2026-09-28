import type { Caso } from "../api/cliente";
import { useDetalheArtefato } from "../api/consultas";
import { dataHoraMs } from "../util/formatar";
import { Aviso, Cartao, Carregando, Definicoes, Hash } from "./Estrutura";

/** Visão técnica: como o registro foi lido e de onde veio, até a linha da tabela. */
export function PainelTecnico({ caso, artefatoId }: { caso: Caso; artefatoId: number | null }) {
  const d = useDetalheArtefato(artefatoId);
  if (artefatoId == null)
    return (
      <Cartao titulo="Detalhes técnicos">
        <p className="text-sm text-slate-500">Selecione um registro para ver a origem exata do dado.</p>
      </Cartao>
    );
  if (d.isPending) return <Cartao titulo="Detalhes técnicos"><Carregando /></Cartao>;
  if (d.isError) return <Aviso>{d.error.message}</Aviso>;
  const a = d.data;
  return (
    <Cartao titulo="Detalhes técnicos">
      <div className="space-y-4 text-sm" aria-label="Detalhes técnicos do registro">
        <Definicoes
          itens={[
            ["Data (fuso do caso)", dataHoraMs(a.resumo.data_hora_ms, caso.fuso_horario)],
            ["Valor original", <span className="font-mono text-xs">{a.data_hora_original ?? "—"}</span>],
            ["Formato", a.formato_data],
            ["Parser", `${a.parser} v${a.parser_versao}`],
            ["Importador", `${a.importador} v${a.importador_versao}`],
            ["Estado / confiança", `${a.resumo.estado_registro} · ${a.confianca}`],
          ]}
        />
        {a.referencias.map((r, i) => (
          <div key={i} className="space-y-2 rounded-lg bg-slate-50 p-3 ring-1 ring-slate-200">
            <p className="text-xs font-semibold uppercase tracking-wide text-slate-500">
              {r.principal ? "Origem" : `Origem adicional (${r.papel_arquivo})`}
            </p>
            <p className="break-all font-mono text-xs text-slate-800" data-teste="origem-arquivo">
              {r.arquivo_caminho}
            </p>
            <p>
              <span className="text-xs text-slate-500">SHA-256 </span>
              <Hash valor={r.arquivo_sha256} />
            </p>
            <p className="text-xs text-slate-700">
              Tabela <span className="font-mono">{r.tabela ?? "—"}</span> · registro{" "}
              <span className="font-mono">{r.id_registro ?? "—"}</span>
            </p>
            {r.consulta && <p className="break-all font-mono text-xs text-slate-600">{r.consulta}</p>}
            <table className="w-full text-xs" aria-label="Valores brutos">
              <tbody>
                {Object.entries(r.valores_brutos as Record<string, unknown>).map(([k, v]) => (
                  <tr key={k} className="align-top">
                    <td className="py-0.5 pr-3 font-mono text-slate-500">{k}</td>
                    <td className="break-all py-0.5 font-mono text-slate-900">{v === null ? "NULL" : JSON.stringify(v)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ))}
        <p>
          <span className="text-xs text-slate-500">Hash do registro normalizado </span>
          <Hash valor={a.hash_registro} />
        </p>
      </div>
    </Cartao>
  );
}

