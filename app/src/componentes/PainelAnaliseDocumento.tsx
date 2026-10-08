// Derivados persistidos são distintos da evidência original. Revisões humanas
// anotam candidatos literais, sem converter uma ocorrência em identidade/fato.
import { useEffect, useRef, useState, type FormEvent } from "react";
import { api, type Caso, type DocumentoIndexado, type EntidadeDocumento, type EstadoRevisaoEntidade, type Pagina, type RevisaoEntidade } from "../api/cliente";
import { dataHora } from "../util/formatar";
import { Botao } from "./Botao";
import { AreaTexto, Selecao } from "./Campo";
import { Aviso, Carregando, Hash, Paginacao, Selo, Vazio } from "./Estrutura";

const ESTADOS: EstadoRevisaoEntidade[] = ["nao_revisado", "confirmado", "descartado", "inconclusivo"];
const ROTULOS: Record<EstadoRevisaoEntidade, string> = {
  nao_revisado: "Não revisado",
  confirmado: "Literal confirmado",
  descartado: "Candidato descartado",
  inconclusivo: "Inconclusivo",
};
const TIPOS: Record<string, string> = { email: "E-mail", telefone: "Telefone", url: "URL", dominio: "Domínio", ip: "IP" };

function mensagem(erro: unknown) {
  return erro instanceof Error ? erro.message : "Não foi possível concluir a operação documental.";
}

/** Verifica limites e fronteiras UTF-8, não índices UTF-16 do JavaScript. */
function candidatoCompativel(entidade: EntidadeDocumento, documento: DocumentoIndexado, bytes: Uint8Array) {
  if (entidade.documento_id !== documento.id || !Number.isSafeInteger(entidade.id) || entidade.id <= 0 ||
    !Number.isSafeInteger(entidade.inicio_utf8) || !Number.isSafeInteger(entidade.fim_utf8) ||
    entidade.inicio_utf8 < 0 || entidade.fim_utf8 <= entidade.inicio_utf8 || entidade.fim_utf8 > bytes.length ||
    !ESTADOS.includes(entidade.estado)) return false;
  try {
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes.subarray(entidade.inicio_utf8, entidade.fim_utf8)) === entidade.literal;
  } catch {
    return false;
  }
}

export function PainelAnaliseDocumento({ caso, documento, exibirTexto = true }: { caso: Caso; documento: DocumentoIndexado; exibirTexto?: boolean }) {
  if (documento.caso_id !== caso.id) {
    return <Aviso>O documento armazenado não pertence ao caso atual. O conteúdo não foi exibido.</Aviso>;
  }
  return <SessaoAnalise key={`${caso.id}:${documento.id}:${documento.sha256_texto}:${documento.executado_em}`} caso={caso} documento={documento} exibirTexto={exibirTexto} />;
}

function SessaoAnalise({ caso, documento, exibirTexto }: { caso: Caso; documento: DocumentoIndexado; exibirTexto: boolean }) {
  const [candidatos, setCandidatos] = useState<EntidadeDocumento[] | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [deslocamentoCandidatos, setDeslocamentoCandidatos] = useState(0);
  const ativa = useRef(true);
  useEffect(() => {
    ativa.current = true;
    let atual = true;
    void api.entidadesDocumento(caso.id, documento.id).then((entidades) => {
      if (!atual || !ativa.current) return;
      const bytes = new TextEncoder().encode(documento.texto);
      if (!entidades.every((entidade) => candidatoCompativel(entidade, documento, bytes))) {
        setErro("Os candidatos recebidos não correspondem ao texto armazenado. Nenhuma revisão foi disponibilizada.");
        return;
      }
      setCandidatos(entidades);
    }).catch((falha) => {
      if (atual && ativa.current) setErro(mensagem(falha));
    });
    return () => { atual = false; ativa.current = false; };
  }, [caso.id, documento]);

  return (
    <section aria-label="Análise documental armazenada" className="mt-4 space-y-3 border-t border-borda pt-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-sm font-semibold text-texto">Documento indexado · análise local</h3>
        <Selo tom="neutro">DERIVADO ARMAZENADO</Selo>
      </div>
      <p className="text-xs leading-relaxed text-texto-2">
        Este texto foi armazenado após conferência na indexação. Abrir ou pesquisar o derivado não relê a origem atual.
        A extração original permanece separada e inalterada.
      </p>
      <details className="text-xs">
        <summary className="cursor-pointer text-texto-2">Proveniência do documento indexado</summary>
        <dl className="mt-3 space-y-2 text-texto-2" aria-label="Proveniência da indexação">
          <div><dt className="sobrelinha">Origem registrada</dt><dd className="mt-1 break-all font-mono">{documento.caminho}</dd></div>
          <div><dt className="sobrelinha">Caso / extração / arquivo / derivado</dt><dd className="mt-1 break-all font-mono">{documento.caso_id} / {documento.extracao_id} / {documento.arquivo_id} / {documento.id}</dd></div>
          <div><dt className="sobrelinha">SHA-256 da origem na indexação</dt><dd className="mt-1"><Hash valor={documento.sha256_origem} /></dd></div>
          <div><dt className="sobrelinha">SHA-256 do texto armazenado</dt><dd className="mt-1"><Hash valor={documento.sha256_texto} /></dd></div>
          <div><dt className="sobrelinha">Motor documental</dt><dd className="mt-1 font-mono">{documento.motor_id} v{documento.motor_versao}</dd></div>
          <div><dt className="sobrelinha">Indexação realizada em</dt><dd className="mt-1">{dataHora(documento.executado_em, caso.fuso_horario)}</dd></div>
          <div><dt className="sobrelinha">Página</dt><dd className="mt-1">{documento.pagina === null ? "Não se aplica ao texto nativo" : `${documento.pagina} de ${documento.total_paginas}`}</dd></div>
          <div><dt className="sobrelinha">Motor de candidatos</dt><dd className="mt-1 font-mono">{documento.entidades_motor_versao}</dd></div>
        </dl>
      </details>
      {exibirTexto && <div className="overflow-hidden rounded-md border border-borda bg-fundo">
        {documento.texto === "" ? <p className="p-3 text-sm text-texto-2">O derivado armazenado não contém texto.</p> : (
          <pre role="region" aria-label="Texto documental armazenado" tabIndex={0} className="max-h-[28rem] overflow-auto p-3 font-mono text-sm leading-relaxed text-texto">{documento.texto}</pre>
        )}
      </div>}
      <div className="border-t border-borda pt-3">
        <h4 className="text-sm font-semibold text-texto">Candidatos literais para revisão</h4>
        <p className="mt-1 text-xs leading-relaxed text-texto-2">
          Reconhecimento por regras locais, sem IA generativa. Confirmar significa conferir a ocorrência literal no texto,
          não identificar uma pessoa, validar um número ou afirmar a veracidade do conteúdo. Nada é adicionado automaticamente a evidências ou relatórios.
        </p>
        {documento.entidades_limitadas && <div className="mt-2"><Aviso tipo="atencao">O limite de candidatos foi atingido. Esta lista não é exaustiva.</Aviso></div>}
        {erro ? <div className="mt-3"><Aviso>{erro}</Aviso></div> : candidatos === null ? (
          <Carregando texto="Carregando candidatos documentais…" />
        ) : candidatos.length === 0 ? (
          <div className="mt-3"><Vazio titulo="Nenhum candidato literal detectado">A ausência de candidatos não comprova ausência de informação no documento.</Vazio></div>
        ) : (
          <div className="mt-3 divide-y divide-borda" aria-label="Candidatos documentais">
            {candidatos.slice(deslocamentoCandidatos, deslocamentoCandidatos + 50).map((entidade) => (
              <RevisaoCandidato key={entidade.id} entidade={entidade} caso={caso} documento={documento} salvar={(revisada) => {
                if (!ativa.current) return;
                setCandidatos((atuais) => atuais?.map((atual) => atual.id === revisada.id ? revisada : atual) ?? null);
              }} />
            ))}
          </div>
        )}
        {candidatos !== null && <Paginacao total={candidatos.length} deslocamento={deslocamentoCandidatos} limite={50} mudar={setDeslocamentoCandidatos} />}
      </div>
    </section>
  );
}

function RevisaoCandidato({ entidade, caso, documento, salvar }: {
  entidade: EntidadeDocumento; caso: Caso; documento: DocumentoIndexado; salvar: (entidade: EntidadeDocumento) => void;
}) {
  const [estado, setEstado] = useState(entidade.estado);
  const [nota, setNota] = useState(entidade.nota ?? "");
  const [salvando, setSalvando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const [sucesso, setSucesso] = useState(false);
  const [historicoAberto, setHistoricoAberto] = useState(false);
  const ativa = useRef(true);
  useEffect(() => {
    ativa.current = true;
    return () => { ativa.current = false; };
  }, []);

  async function revisar(evento: FormEvent) {
    evento.preventDefault();
    if (salvando) return;
    setSalvando(true); setErro(null); setSucesso(false);
    try {
      const revisada = await api.revisarEntidade(caso.id, entidade.id, estado, nota === "" ? null : nota);
      if (!ativa.current) return;
      const bytes = new TextEncoder().encode(documento.texto);
      if (!candidatoCompativel(revisada, documento, bytes) || revisada.id !== entidade.id ||
          revisada.tipo !== entidade.tipo || revisada.literal !== entidade.literal ||
          revisada.inicio_utf8 !== entidade.inicio_utf8 || revisada.fim_utf8 !== entidade.fim_utf8 ||
          revisada.regra_versao !== entidade.regra_versao || revisada.estado !== estado) {
        setErro("A revisão recebida não corresponde ao candidato selecionado. O retorno não foi aplicado.");
        return;
      }
      setNota(revisada.nota ?? ""); salvar(revisada); setSucesso(true);
    } catch (falha) {
      if (ativa.current) setErro(mensagem(falha));
    } finally {
      if (ativa.current) setSalvando(false);
    }
  }

  return (
    <article aria-label={`Candidato ${entidade.id}`} className="py-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h5 className="text-xs font-medium text-texto-2">{TIPOS[entidade.tipo] ?? entidade.tipo}</h5>
        <Selo tom={entidade.estado === "confirmado" ? "info" : "neutro"}>{ROTULOS[entidade.estado]}</Selo>
      </div>
      <p className="mt-1 break-all font-mono text-sm text-texto">{entidade.literal}</p>
      <p className="mt-1 text-2xs text-texto-3">Posição em bytes UTF-8: [{entidade.inicio_utf8}, {entidade.fim_utf8}) · regra {entidade.regra_versao}</p>
      <form onSubmit={(evento) => void revisar(evento)} className="mt-3 space-y-2">
        <Selecao rotulo="Decisão da revisão" value={estado} disabled={salvando} onChange={(evento) => setEstado(evento.target.value as EstadoRevisaoEntidade)}>
          {ESTADOS.map((opcao) => <option value={opcao} key={opcao}>{ROTULOS[opcao]}</option>)}
        </Selecao>
        <AreaTexto rotulo="Nota do analista" value={nota} maxLength={1000} disabled={salvando} onChange={(evento) => setNota(evento.target.value)} ajuda="Anotação separada do texto original. Até 1.000 caracteres." />
        <Botao type="submit" tamanho="p" variante="secundario" carregando={salvando}>Salvar revisão</Botao>
      </form>
      {erro && <div className="mt-2"><Aviso>{erro}</Aviso></div>}
      {sucesso && <p role="status" className="mt-2 text-xs text-ok-claro">Revisão registrada. O texto original não foi alterado.</p>}
      {entidade.revista_em && <p className="mt-2 text-2xs text-texto-3">Revisão: {dataHora(entidade.revista_em, caso.fuso_horario)}{entidade.revisor ? ` · ${entidade.revisor}` : ""}</p>}
      <div className="mt-2">
        <Botao tamanho="p" variante="fantasma" aria-expanded={historicoAberto} onClick={() => setHistoricoAberto((aberto) => !aberto)}>{historicoAberto ? "Ocultar histórico" : "Ver histórico de revisões"}</Botao>
        {historicoAberto && <HistoricoRevisoes key={`${entidade.id}:${entidade.revista_em ?? "original"}`} caso={caso} entidadeId={entidade.id} />}
      </div>
    </article>
  );
}

function HistoricoRevisoes({ caso, entidadeId }: { caso: Caso; entidadeId: number }) {
  const [deslocamento, setDeslocamento] = useState(0);
  const [historico, setHistorico] = useState<Pagina<RevisaoEntidade> | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  useEffect(() => {
    let atual = true;
    setHistorico(null); setErro(null);
    void api.revisoesEntidade(caso.id, entidadeId, deslocamento, 20).then((pagina) => {
      if (!atual) return;
      if (!Number.isSafeInteger(pagina.total) || pagina.total < 0 || pagina.itens.length > 20 ||
          pagina.itens.some((revisao) => revisao.entidade_id !== entidadeId || !ESTADOS.includes(revisao.estado))) {
        setErro("O histórico recebido não corresponde ao candidato selecionado.");
        return;
      }
      setHistorico(pagina);
    }).catch((falha) => { if (atual) setErro(mensagem(falha)); });
    return () => { atual = false; };
  }, [caso.id, entidadeId, deslocamento]);
  return (
    <section aria-label="Histórico de decisões humanas" className="mt-2 border-t border-borda pt-2">
      <h6 className="sobrelinha">Histórico de decisões humanas</h6>
      {erro ? <div className="mt-2"><Aviso>{erro}</Aviso></div> : historico === null ? <Carregando texto="Carregando histórico de revisões…" /> : (
        <>
          {historico.itens.length === 0 ? <p className="mt-2 text-xs text-texto-3">Nenhuma revisão registrada para este candidato.</p> : (
            <ol className="mt-2 space-y-2" aria-label="Revisões registradas">
              {historico.itens.map((revisao) => (
                <li key={revisao.id} className="border-l border-borda-2 pl-3 text-xs text-texto-2">
                  <p>{ROTULOS[revisao.estado]} · {dataHora(revisao.revista_em, caso.fuso_horario)}</p>
                  <p className="mt-1 break-all font-mono text-2xs text-texto-3">{revisao.revisor} · {revisao.maquina}</p>
                  {revisao.nota !== null && <p className="mt-1 whitespace-pre-wrap break-words">{revisao.nota}</p>}
                </li>
              ))}
            </ol>
          )}
          <Paginacao total={historico.total} deslocamento={deslocamento} limite={20} mudar={setDeslocamento} />
        </>
      )}
    </section>
  );
}
