import type { ReactNode } from "react";
import type { Caso } from "../api/cliente";
import { useDetalheArtefato } from "../api/consultas";
import { dataHoraMs, tamanho } from "../util/formatar";
import { nomeConfianca, nomeEstadoRegistro, nomeTipoArtefato, nomeVinculoAnexo, rotuloApp, rotuloSubtipo } from "../util/rotulos";
import { Aviso, Cartao, Carregando, Hash, SecaoRecolhivel, Selo } from "./Estrutura";

function Linha({ rotulo, children, mono }: { rotulo: string; children: ReactNode; mono?: boolean }) {
  return (
    <div className="grid grid-cols-[7.5rem_minmax(0,1fr)] gap-2 py-1 text-xs">
      <dt className="text-texto-3">{rotulo}</dt>
      <dd className={`break-all text-texto ${mono ? "font-mono" : ""}`}>{children ?? "—"}</dd>
    </div>
  );
}

const tomConfianca = (c: string) => (c === "alta" ? "ok" : c === "media" ? "atencao" : c === "baixa" ? "erro" : "neutro");

/** Visão técnica: como o registro foi lido e de onde veio, até a linha da tabela. */
export function PainelTecnico({ caso, artefatoId, className = "" }: { caso: Caso; artefatoId: number | null; className?: string }) {
  const d = useDetalheArtefato(artefatoId);
  if (artefatoId == null)
    return (
      <Cartao titulo="Detalhes técnicos" icone="banco" className={className}>
        <p className="text-sm text-texto-2">Selecione um registro para ver a origem exata do dado.</p>
      </Cartao>
    );
  if (d.isPending)
    return (
      <Cartao titulo="Detalhes técnicos" icone="banco" className={className}>
        <Carregando />
      </Cartao>
    );
  if (d.isError) return <Aviso>{d.error.message}</Aviso>;
  const a = d.data;
  const metadados = Object.entries(a.metadados ?? {});
  return (
    <Cartao
      titulo="Detalhes técnicos"
      icone="banco"
      denso
      className={className}
      acoes={
        <Selo tom={a.resumo.estado_registro === "ativo" ? "ok" : "atencao"}>
          {nomeEstadoRegistro[a.resumo.estado_registro] ?? a.resumo.estado_registro}
        </Selo>
      }
    >
      <div aria-label="Detalhes técnicos do registro">
        <div className="border-b border-borda px-4 py-3">
          <p className="tabular text-sm text-texto">{dataHoraMs(a.resumo.data_hora_ms, caso.fuso_horario)}</p>
          <p className="text-xs text-texto-3">
            {rotuloApp(a.resumo.app ?? a.resumo.tipo)} · {nomeTipoArtefato[a.resumo.tipo] ?? a.resumo.tipo}
            {a.resumo.subtipo ? ` · ${rotuloSubtipo(a.resumo.subtipo)}` : ""} · registro nº {a.resumo.id}
          </p>
        </div>

        {a.referencias.map((r, i) => (
          <SecaoRecolhivel key={i} titulo={r.principal ? "Origem" : `Origem adicional (${r.papel_arquivo})`} icone="arquivos">
            <dl>
              <Linha rotulo="Arquivo" mono>
                <span data-teste="origem-arquivo">{r.arquivo_caminho}</span>
              </Linha>
              <Linha rotulo="SHA-256">
                <Hash valor={r.arquivo_sha256} />
              </Linha>
              <Linha rotulo="Tabela" mono>
                {r.tabela ?? "—"}
              </Linha>
              <Linha rotulo="Registro" mono>
                {r.id_registro ?? "—"}
              </Linha>
              {r.deslocamento != null && (
                <Linha rotulo="Deslocamento" mono>
                  {r.deslocamento}
                </Linha>
              )}
              <Linha rotulo="Consulta" mono>
                {r.consulta ?? "—"}
              </Linha>
            </dl>
            <p className="sobrelinha mb-1 mt-2">Valores brutos</p>
            <div className="overflow-x-auto rounded-md border border-borda bg-fundo">
              <table className="w-full text-xs" aria-label="Valores brutos">
                <tbody className="divide-y divide-borda">
                  {Object.entries(r.valores_brutos as Record<string, unknown>).map(([k, v]) => (
                    <tr key={k} className="align-top">
                      <td className="whitespace-nowrap px-2 py-1 font-mono text-texto-3">{k}</td>
                      <td className="break-all px-2 py-1 font-mono text-texto">
                        {v === null ? <span className="text-texto-3">NULL</span> : JSON.stringify(v)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </SecaoRecolhivel>
        ))}

        {(a.anexo_caminho || a.resumo.anexo_vinculo) && (
          <SecaoRecolhivel titulo="Anexo" icone="midias">
            <dl>
              <Linha rotulo="Caminho no app" mono>
                {a.anexo_caminho ?? "—"}
              </Linha>
              <Linha rotulo="Nome e tamanho">
                {[a.resumo.nome_arquivo, a.resumo.mime, a.resumo.tamanho != null ? tamanho(a.resumo.tamanho) : null].filter(Boolean).join(" · ") || "—"}
              </Linha>
              <Linha rotulo="Na extração">
                {a.resumo.anexo_vinculo ? (nomeVinculoAnexo[a.resumo.anexo_vinculo] ?? a.resumo.anexo_vinculo) : "—"}
                {a.resumo.anexo_vinculo && <span className="ml-1 font-mono text-texto-3">({a.resumo.anexo_vinculo})</span>}
              </Linha>
              {a.anexo_arquivo && (
                <>
                  <Linha rotulo="Arquivo" mono>
                    <span data-teste="anexo-arquivo">{a.anexo_arquivo.caminho}</span>
                  </Linha>
                  <Linha rotulo="SHA-256">
                    <Hash valor={a.anexo_arquivo.sha256} />
                  </Linha>
                  <Linha rotulo="Tamanho">{tamanho(a.anexo_arquivo.tamanho)}</Linha>
                </>
              )}
            </dl>
          </SecaoRecolhivel>
        )}

        {a.resumo.latitude != null && a.resumo.longitude != null && (
          <SecaoRecolhivel titulo="Localização" icone="localizacoes">
            <dl>
              <Linha rotulo="Latitude" mono>
                {a.resumo.latitude}
              </Linha>
              <Linha rotulo="Longitude" mono>
                {a.resumo.longitude}
              </Linha>
            </dl>
          </SecaoRecolhivel>
        )}

        <SecaoRecolhivel titulo="Data e hora" icone="relogio">
          <dl>
            <Linha rotulo="Fuso do caso">{dataHoraMs(a.resumo.data_hora_ms, caso.fuso_horario)}</Linha>
            <Linha rotulo="Timestamp original" mono>
              {a.data_hora_original ?? "—"}
            </Linha>
            <Linha rotulo="Formato original" mono>
              {a.formato_data ?? "—"}
            </Linha>
          </dl>
        </SecaoRecolhivel>

        <SecaoRecolhivel titulo="Processamento" icone="terminal">
          <dl>
            <Linha rotulo="Parser">{`${a.parser} v${a.parser_versao}`}</Linha>
            <Linha rotulo="Importador">{`${a.importador} v${a.importador_versao}`}</Linha>
            <Linha rotulo="Estado">
              {nomeEstadoRegistro[a.resumo.estado_registro] ?? a.resumo.estado_registro}{" "}
              <span className="font-mono text-texto-3">({a.resumo.estado_registro})</span>
            </Linha>
            <Linha rotulo="Confiança">
              <Selo tom={tomConfianca(a.confianca)}>{nomeConfianca[a.confianca] ?? a.confianca}</Selo>
            </Linha>
            <Linha rotulo="Pasta de origem" mono>
              {a.origem_caminho}
            </Linha>
            <Linha rotulo="Manifesto">
              <Hash valor={a.hash_manifesto} />
            </Linha>
            <Linha rotulo="Hash do registro">
              <Hash valor={a.hash_registro} />
            </Linha>
          </dl>
        </SecaoRecolhivel>

        {metadados.length > 0 && (
          <SecaoRecolhivel titulo={`Metadados (${metadados.length})`} icone="lista" aberta={false}>
            <dl>
              {metadados.map(([k, v]) => (
                <Linha key={k} rotulo={k} mono>
                  {v === null ? "NULL" : typeof v === "string" ? v : JSON.stringify(v)}
                </Linha>
              ))}
            </dl>
          </SecaoRecolhivel>
        )}
      </div>
    </Cartao>
  );
}
