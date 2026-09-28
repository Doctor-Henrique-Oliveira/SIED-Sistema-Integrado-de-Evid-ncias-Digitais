import { useState, type ReactNode } from "react";
import { useParams } from "react-router-dom";
import type { Caso, Dispositivo, Extracao } from "../api/cliente";
import { useArtefatos, useDispositivos, useExecucoes, useExtracao } from "../api/consultas";
import { InventarioArquivos } from "../componentes/Arquivos";
import { Abas, Aviso, Cabecalho, Cartao, Carregando, Hash, Paginacao, Selo, tabela, Vazio } from "../componentes/Estrutura";
import { Icone, type NomeIcone } from "../componentes/Icone";
import { PainelTecnico } from "../componentes/PainelTecnico";
import { nomeSistema } from "../componentes/SecoesCaso";
import { dataHora, dataHoraMs, duracao, numero, tamanho } from "../util/formatar";
import { nomeEstadoRegistro, nomeStatusExecucao, nomeStatusExtracao, rotuloApp, tomStatus } from "../util/rotulos";
import { ComCasoAberto, VoltarAoCaso } from "./PaginaCaso";

const LIMITE = 100;

export function PaginaExtracao() {
  const { extracaoId = "" } = useParams();
  return <ComCasoAberto>{(caso) => <DetalheExtracao caso={caso} extracaoId={extracaoId} />}</ComCasoAberto>;
}

type Aba = "artefatos" | "arquivos" | "processamento";

function DetalheExtracao({ caso, extracaoId }: { caso: Caso; extracaoId: string }) {
  const extracao = useExtracao(extracaoId);
  const dispositivos = useDispositivos(caso.id);
  const execucoes = useExecucoes(extracaoId);
  const [aba, setAba] = useState<Aba>("artefatos");

  if (extracao.isPending) return <Carregando />;
  if (extracao.isError) return <Aviso>{extracao.error.message}</Aviso>;
  const x = extracao.data;
  const disp = dispositivos.data?.find((d) => d.id === x.dispositivo_id);

  return (
    <>
      <Cabecalho
        icone="extracao"
        sobre={<VoltarAoCaso caso={caso} />}
        titulo={`Extração — ${disp?.apelido ?? "dispositivo"}`}
        subtitulo={`${x.importador_nome} · importada em ${dataHora(x.iniciada_em, caso.fuso_horario)}`}
        acoes={<Selo tom={tomStatus(x.status)}>{nomeStatusExtracao[x.status] ?? x.status}</Selo>}
      />
      <div className="space-y-4">
        <BlocosTecnicos caso={caso} x={x} disp={disp} parsers={execucoes.data?.length} />
        <div>
          <Abas
            rotulo="Conteúdo da extração"
            atual={aba}
            mudar={setAba}
            abas={[
              ["artefatos", `Registros interpretados (${numero(x.total_artefatos ?? 0)})`],
              ["arquivos", `Arquivos (${numero(x.total_arquivos ?? 0)})`],
              ["processamento", `Processamento (${numero(execucoes.data?.length ?? 0)})`],
            ]}
          />
          <div className="pt-3">
            {aba === "artefatos" && <AbaArtefatos caso={caso} x={x} />}
            {aba === "arquivos" && <InventarioArquivos caso={caso} extracaoId={x.id} />}
            {aba === "processamento" && <AbaProcessamento caso={caso} x={x} />}
          </div>
        </div>
      </div>
    </>
  );
}

function Bloco({ titulo, icone, children, className = "" }: { titulo: string; icone: NomeIcone; children: ReactNode; className?: string }) {
  return (
    <section aria-label={titulo} className={`min-w-0 rounded-xl border border-borda bg-sup p-3.5 ${className}`}>
      <h2 className="mb-2 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wider text-texto-3">
        <Icone nome={icone} className="size-3.5" />
        {titulo}
      </h2>
      <dl className="space-y-1.5">{children}</dl>
    </section>
  );
}

function Par({ rotulo, children, mono }: { rotulo: string; children: ReactNode; mono?: boolean }) {
  return (
    <div className="flex items-baseline justify-between gap-3 text-[13px]">
      <dt className="shrink-0 text-texto-2">{rotulo}</dt>
      <dd className={`min-w-0 break-words text-right text-texto ${mono ? "break-all font-mono text-xs" : "tabular"}`}>{children ?? "—"}</dd>
    </div>
  );
}

function BlocosTecnicos({ caso, x, disp, parsers }: { caso: Caso; x: Extracao; disp: Dispositivo | undefined; parsers: number | undefined }) {
  const fuso = caso.fuso_horario;
  return (
    <div className="space-y-3">
      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <Bloco titulo="Dispositivo" icone="dispositivo">
          <Par rotulo="Identificação">{disp?.apelido}</Par>
          <Par rotulo="Aparelho">{disp ? [disp.fabricante, disp.nome_comercial ?? disp.modelo].filter(Boolean).join(" ") || "—" : "—"}</Par>
          <Par rotulo="Sistema">{disp ? `${nomeSistema[disp.sistema]} ${disp.versao_sistema ?? ""}` : "—"}</Par>
          <Par rotulo="IMEI" mono>
            {disp?.imei1 ?? "—"}
          </Par>
        </Bloco>
        <Bloco titulo="Ferramenta e importador" icone="terminal">
          <Par rotulo="Importador">{`${x.importador_nome} (v${x.importador_versao})`}</Par>
          <Par rotulo="Ferramenta">{x.ferramenta_versao ?? "não identificada"}</Par>
          <Par rotulo="Tipo de extração">{x.tipo_extracao ?? "não informado"}</Par>
          <Par rotulo="Parsers executados">{parsers == null ? "—" : numero(parsers)}</Par>
        </Bloco>
        <Bloco titulo="Contagens" icone="lista">
          <Par rotulo="Arquivos inventariados">{`${numero(x.total_arquivos)} (${tamanho(x.total_bytes)})`}</Par>
          <Par rotulo="Registros interpretados">{numero(x.total_artefatos)}</Par>
          <Par rotulo="Itens ignorados">{numero(x.itens_ignorados)}</Par>
          <Par rotulo="Avisos">
            <span className={x.total_avisos ? "text-atencao" : ""}>{numero(x.total_avisos)}</span>
          </Par>
        </Bloco>
        <Bloco titulo="Tempo e responsável" icone="relogio">
          <Par rotulo="Início">{dataHora(x.iniciada_em, fuso)}</Par>
          <Par rotulo="Fim">{dataHora(x.concluida_em, fuso)}</Par>
          <Par rotulo="Duração">{duracao(x.iniciada_em, x.concluida_em)}</Par>
          <Par rotulo="Analista / estação">{`${x.usuario} · ${x.maquina}`}</Par>
        </Bloco>
      </div>
      <div className="grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <Bloco titulo="Origem (somente leitura)" icone="cadeado">
          <div className="text-[13px]">
            <dt className="text-texto-2">Pasta de origem</dt>
            <dd className="mt-0.5 break-all font-mono text-xs text-texto">{x.origem_caminho}</dd>
          </div>
          <Par rotulo="Versão do SIED">{x.sied_versao}</Par>
        </Bloco>
        <Bloco titulo="Integridade" icone="integridade">
          <div className="text-[13px]">
            <dt className="text-texto-2">SHA-256 do manifesto</dt>
            <dd className="mt-0.5" data-teste="hash-manifesto">
              <Hash valor={x.hash_manifesto} />
            </dd>
          </div>
          <p className="text-xs text-texto-3">
            O manifesto (formato sha256sum) está em importacoes\{x.id}\manifesto.sha256 na pasta do caso.
          </p>
          <Par rotulo="Arquivos ilegíveis">
            <span className={x.arquivos_ilegiveis ? "text-atencao" : ""}>{numero(x.arquivos_ilegiveis)}</span>
          </Par>
        </Bloco>
      </div>
      {x.erro && x.status !== "concluida" && <Aviso tipo={x.status === "falhou" ? "erro" : "atencao"}>{x.erro}</Aviso>}
      {x.observacoes.length > 0 && (
        <details className="group rounded-xl border border-borda bg-sup text-[13px]">
          <summary className="flex cursor-pointer list-none items-center gap-2 px-3.5 py-2.5 text-texto-2 hover:text-texto">
            <Icone nome="direita" className="size-3.5 transition-transform group-open:rotate-90" />
            Observações da importação ({x.observacoes.length})
          </summary>
          <ul className="list-disc space-y-1 border-t border-borda py-2.5 pl-9 pr-4 text-xs text-texto-2">
            {x.observacoes.map((o, i) => (
              <li key={i} className="break-all">
                {o}
              </li>
            ))}
          </ul>
        </details>
      )}
    </div>
  );
}

function AbaArtefatos({ caso, x }: { caso: Caso; x: Extracao }) {
  const [deslocamento, setDeslocamento] = useState(0);
  const [selecionado, setSelecionado] = useState<number | null>(null);
  const artefatos = useArtefatos(x.id, deslocamento, LIMITE);

  if (artefatos.isPending) return <Carregando />;
  if (artefatos.isError) return <Aviso>{artefatos.error.message}</Aviso>;
  if (artefatos.data.total === 0)
    return (
      <Vazio titulo="Nenhum registro interpretado" icone="banco">
        Os parsers disponíveis não encontraram bancos conhecidos nesta extração. Todos os arquivos estão na aba Arquivos.
      </Vazio>
    );

  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_26rem]">
      <Cartao denso>
        <div className="overflow-x-auto">
          <table className={tabela.base} aria-label="Registros interpretados">
            <thead className={tabela.cabeca}>
              <tr>
                <th className={tabela.th}>Data e hora</th>
                <th className={tabela.th}>App</th>
                <th className={tabela.th}>Direção</th>
                <th className={tabela.th}>Contato</th>
                <th className={tabela.th}>Conteúdo</th>
                <th className={tabela.th}>Estado</th>
              </tr>
            </thead>
            <tbody className={tabela.corpo}>
              {artefatos.data.itens.map((a) => (
                <tr
                  key={a.id}
                  onClick={() => setSelecionado(a.id)}
                  onKeyDown={(e) => e.key === "Enter" && setSelecionado(a.id)}
                  tabIndex={0}
                  aria-selected={selecionado === a.id}
                  className={`cursor-pointer ${selecionado === a.id ? "bg-ouro/8" : tabela.linha}`}
                >
                  <td className={`${tabela.td} tabular whitespace-nowrap text-texto-2`}>{dataHoraMs(a.data_hora_ms, caso.fuso_horario)}</td>
                  <td className={`${tabela.td} whitespace-nowrap text-texto-2`}>{rotuloApp(a.app ?? a.tipo)}</td>
                  <td className={tabela.td}>
                    {a.de_mim == null ? "—" : a.de_mim ? <span className="text-ouro-claro">Enviada</span> : <span className="text-info">Recebida</span>}
                  </td>
                  <td className={`${tabela.td} whitespace-nowrap font-mono text-xs`}>{a.remetente ?? (a.destinatarios.join(", ") || "—")}</td>
                  <td className={`${tabela.td} max-w-md truncate text-texto`} title={a.conteudo ?? undefined}>
                    {a.conteudo ?? "—"}
                  </td>
                  <td className={tabela.td}>
                    {a.estado_registro !== "ativo" ? (
                      <Selo tom="atencao">{nomeEstadoRegistro[a.estado_registro] ?? a.estado_registro}</Selo>
                    ) : (
                      <span className="text-xs text-texto-3">{nomeEstadoRegistro.ativo}</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {artefatos.data.total > LIMITE && (
          <div className="border-t border-borda px-3">
            <Paginacao total={artefatos.data.total} deslocamento={deslocamento} limite={LIMITE} mudar={setDeslocamento} />
          </div>
        )}
      </Cartao>
      <PainelTecnico caso={caso} artefatoId={selecionado} className="xl:sticky xl:top-0 xl:self-start" />
    </div>
  );
}

function AbaProcessamento({ caso, x }: { caso: Caso; x: Extracao }) {
  const execucoes = useExecucoes(x.id);
  if (execucoes.isPending) return <Carregando />;
  if (execucoes.isError) return <Aviso>{execucoes.error.message}</Aviso>;
  if (execucoes.data.length === 0) return <Vazio titulo="Nenhum parser foi executado nesta extração" icone="terminal" />;
  return (
    <Cartao denso>
      <div className="overflow-x-auto">
        <table className={tabela.base} aria-label="Execuções de parser">
          <thead className={tabela.cabeca}>
            <tr>
              <th className={tabela.th}>Parser</th>
              <th className={tabela.th}>Arquivo lido (cópia verificada)</th>
              <th className={tabela.th}>Situação</th>
              <th className={`${tabela.th} text-right`}>Registros</th>
              <th className={`${tabela.th} text-right`}>Avisos</th>
              <th className={tabela.th}>Início</th>
              <th className={tabela.th}>Duração</th>
            </tr>
          </thead>
          <tbody className={tabela.corpo}>
            {execucoes.data.map((e) => (
              <tr key={e.id} className={tabela.linha}>
                <td className={tabela.td}>
                  {e.parser_nome} <span className="text-xs text-texto-3">v{e.parser_versao}</span>
                </td>
                <td className={tabela.td}>
                  <div className="break-all font-mono text-xs text-texto">{e.arquivo_caminho ?? "—"}</div>
                  {e.copia_sha256 && (
                    <div className="text-xs text-texto-3" title="SHA-256 da cópia de trabalho (igual ao original)">
                      cópia verificada <span className="font-mono">{e.copia_sha256.slice(0, 16)}…</span>
                    </div>
                  )}
                  {e.erro && <div className="text-xs text-erro">{e.erro}</div>}
                </td>
                <td className={tabela.td}>
                  <Selo tom={tomStatus(e.status)}>{nomeStatusExecucao[e.status] ?? e.status}</Selo>
                </td>
                <td className={`${tabela.td} tabular text-right`}>{numero(e.artefatos)}</td>
                <td className={`${tabela.td} tabular text-right ${e.avisos ? "text-atencao" : ""}`}>{numero(e.avisos)}</td>
                <td className={`${tabela.td} tabular whitespace-nowrap text-xs text-texto-2`}>{dataHora(e.iniciada_em, caso.fuso_horario)}</td>
                <td className={`${tabela.td} tabular whitespace-nowrap text-xs text-texto-2`}>{duracao(e.iniciada_em, e.concluida_em)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Cartao>
  );
}
