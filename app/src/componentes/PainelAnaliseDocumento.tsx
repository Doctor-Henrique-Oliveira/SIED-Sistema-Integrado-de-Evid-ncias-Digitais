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

type RegiaoOcr = { id: number; documento_id: number; ordem: number; inicio_utf8: number; fim_utf8: number; texto: string; x: number; y: number; largura: number; altura: number; confianca: number | null };
type PaginaRegioes = { total: number; itens: RegiaoOcr[] };
type DocumentoComOcr = DocumentoIndexado & { largura_px?: number | null; altura_px?: number | null; total_regioes?: number };

function metadadosOcr(documento: DocumentoIndexado) {
  const candidato = documento as DocumentoComOcr;
  const largura = Number.isSafeInteger(candidato.largura_px) && candidato.largura_px! > 0 ? candidato.largura_px! : null;
  const altura = Number.isSafeInteger(candidato.altura_px) && candidato.altura_px! > 0 ? candidato.altura_px! : null;
  const total = Number.isSafeInteger(candidato.total_regioes) && candidato.total_regioes! >= 0 && candidato.total_regioes! <= 50_000 ? candidato.total_regioes! : 0;
  return { largura, altura, total };
}

function paginaRegioesCompativel(valor: unknown, documento: DocumentoIndexado, largura: number, altura: number, total: number, deslocamento: number): PaginaRegioes | null {
  if (typeof valor !== "object" || valor === null) return null;
  const candidata = valor as { total?: unknown; itens?: unknown };
  if (candidata.total !== total || !Array.isArray(candidata.itens) || candidata.itens.length > 100) return null;
  const bytes = new TextEncoder().encode(documento.texto);
  const ids = new Set<number>();
  const regioes: RegiaoOcr[] = [];
  for (const desconhecida of candidata.itens) {
    if (typeof desconhecida !== "object" || desconhecida === null) return null;
    const r = desconhecida as RegiaoOcr;
    if (!Number.isSafeInteger(r.id) || r.id <= 0 || ids.has(r.id) || r.documento_id !== documento.id ||
      !Number.isSafeInteger(r.ordem) || r.ordem < deslocamento || r.ordem >= deslocamento + 100 || !Number.isSafeInteger(r.inicio_utf8) ||
      !Number.isSafeInteger(r.fim_utf8) || r.inicio_utf8 < 0 || r.fim_utf8 <= r.inicio_utf8 || r.fim_utf8 > bytes.length ||
      typeof r.texto !== "string" || ![r.x, r.y].every((n) => Number.isSafeInteger(n) && n >= 0) ||
      ![r.largura, r.altura].every((n) => Number.isSafeInteger(n) && n > 0) ||
      r.x + r.largura > largura || r.y + r.altura > altura ||
      !(r.confianca === null || (Number.isInteger(r.confianca) && r.confianca >= 0 && r.confianca <= 100))) return null;
    try {
      if (new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes.subarray(r.inicio_utf8, r.fim_utf8)) !== r.texto) return null;
    } catch { return null; }
    ids.add(r.id);
    regioes.push(r);
  }
  return { total, itens: regioes };
}

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

type FaixaUtf8 = { inicio: number; fim: number };

export function PainelAnaliseDocumento({ caso, documento, exibirTexto = true, destaqueUtf8 = null }: { caso: Caso; documento: DocumentoIndexado; exibirTexto?: boolean; destaqueUtf8?: FaixaUtf8 | null }) {
  if (documento.caso_id !== caso.id) {
    return <Aviso>O documento armazenado não pertence ao caso atual. O conteúdo não foi exibido.</Aviso>;
  }
  return <SessaoAnalise key={`${caso.id}:${documento.id}:${documento.sha256_texto}:${documento.executado_em}`} caso={caso} documento={documento} exibirTexto={exibirTexto} destaqueUtf8={destaqueUtf8} />;
}

function SessaoAnalise({ caso, documento, exibirTexto, destaqueUtf8 }: { caso: Caso; documento: DocumentoIndexado; exibirTexto: boolean; destaqueUtf8: FaixaUtf8 | null }) {
  const [candidatos, setCandidatos] = useState<EntidadeDocumento[] | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [deslocamentoCandidatos, setDeslocamentoCandidatos] = useState(0);
  const [deslocamentoRegioes, setDeslocamentoRegioes] = useState(0);
  const [regiaoSelecionada, setRegiaoSelecionada] = useState<number | null>(null);
  const [paginaRegioes, setPaginaRegioes] = useState<PaginaRegioes | null>(null);
  const [carregandoRegioes, setCarregandoRegioes] = useState(false);
  const [erroRegioes, setErroRegioes] = useState<string | null>(null);
  const [erroLocalizacao, setErroLocalizacao] = useState<string | null>(null);
  const [localizandoRegiao, setLocalizandoRegiao] = useState(destaqueUtf8 !== null);
  const [ordemDestaque, setOrdemDestaque] = useState<number | null>(null);
  const ocr = metadadosOcr(documento);
  const regioesVisiveis = paginaRegioes?.itens ?? [];
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
  useEffect(() => {
    let atual = true;
    setErroLocalizacao(null); setOrdemDestaque(null);
    if (destaqueUtf8 === null || ocr.total === 0) { setLocalizandoRegiao(false); return; }
    setLocalizandoRegiao(true);
    void api.localizarRegiaoDocumento(caso.id, documento.id, destaqueUtf8.inicio, destaqueUtf8.fim).then((ordem) => {
      if (!atual || !ativa.current) return;
      if (ordem !== null && (!Number.isSafeInteger(ordem) || ordem < 0 || ordem >= ocr.total)) {
        setErroLocalizacao("A localização OCR recebida não corresponde ao documento aberto.");
        return;
      }
      setOrdemDestaque(ordem);
      if (ordem !== null) setDeslocamentoRegioes(Math.floor(ordem / 100) * 100);
    }).catch((falha) => {
      if (atual && ativa.current) setErroLocalizacao(mensagem(falha));
    }).finally(() => {
      if (atual && ativa.current) setLocalizandoRegiao(false);
    });
    return () => { atual = false; };
  }, [caso.id, destaqueUtf8, documento.id, ocr.total]);
  useEffect(() => {
    let atual = true;
    setPaginaRegioes(null); setErroRegioes(null); setRegiaoSelecionada(null);
    if (ocr.total === 0 || localizandoRegiao) { setCarregandoRegioes(false); return; }
    if (ocr.largura === null || ocr.altura === null) {
      setCarregandoRegioes(false);
      setErroRegioes("O snapshot OCR informa regiões sem dimensões válidas da página.");
      return;
    }
    setCarregandoRegioes(true);
    void api.regioesDocumento(caso.id, documento.id, deslocamentoRegioes, 100).then((retorno) => {
      if (!atual || !ativa.current) return;
      const pagina = paginaRegioesCompativel(retorno, documento, ocr.largura!, ocr.altura!, ocr.total, deslocamentoRegioes);
      if (pagina === null) {
        setErroRegioes("As regiões recebidas não correspondem ao documento ou à paginação solicitada.");
        return;
      }
      const alvo = ordemDestaque === null ? null : pagina.itens.find((regiao) => regiao.ordem === ordemDestaque);
      if (ordemDestaque !== null && !alvo) {
        setErroRegioes("A região correspondente ao trecho não está na página retornada.");
        return;
      }
      setPaginaRegioes(pagina);
      setRegiaoSelecionada(alvo?.id ?? null);
    }).catch((falha) => {
      if (atual && ativa.current) setErroRegioes(mensagem(falha));
    }).finally(() => {
      if (atual && ativa.current) setCarregandoRegioes(false);
    });
    return () => { atual = false; };
  }, [caso.id, documento, deslocamentoRegioes, localizandoRegiao, ocr.altura, ocr.largura, ocr.total, ordemDestaque]);

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
          <div><dt className="sobrelinha">Navegação</dt><dd className="mt-1"><a
            className="text-info-claro hover:text-texto hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ouro"
            href={`/casos/${encodeURIComponent(caso.id)}/arquivos?extracao=${encodeURIComponent(documento.extracao_id)}&arquivo=${documento.arquivo_id}${documento.pagina === null ? "" : `&pagina=${documento.pagina}`}`}
          >Abrir arquivo original{documento.pagina === null ? "" : ` · página ${documento.pagina}`}</a></dd></div>
          <div><dt className="sobrelinha">SHA-256 da origem na indexação</dt><dd className="mt-1"><Hash valor={documento.sha256_origem} /></dd></div>
          <div><dt className="sobrelinha">SHA-256 do texto armazenado</dt><dd className="mt-1"><Hash valor={documento.sha256_texto} /></dd></div>
          <div><dt className="sobrelinha">Motor documental</dt><dd className="mt-1 font-mono">{documento.motor_id} v{documento.motor_versao}</dd></div>
          <div><dt className="sobrelinha">Indexação realizada em</dt><dd className="mt-1">{dataHora(documento.executado_em, caso.fuso_horario)}</dd></div>
          <div><dt className="sobrelinha">Página</dt><dd className="mt-1">{documento.pagina === null ? "Não se aplica ao texto nativo" : `${documento.pagina} de ${documento.total_paginas}`}</dd></div>
          {ocr.largura !== null && ocr.altura !== null && <div><dt className="sobrelinha">Imagem processada</dt><dd className="mt-1 font-mono">{ocr.largura} × {ocr.altura} px · {ocr.total.toLocaleString("pt-BR")} região(ões)</dd></div>}
          <div><dt className="sobrelinha">Motor de candidatos</dt><dd className="mt-1 font-mono">{documento.entidades_motor_versao}</dd></div>
        </dl>
      </details>
      {exibirTexto && <div className="overflow-hidden rounded-md border border-borda bg-fundo">
        {documento.texto === "" ? <p className="p-3 text-sm text-texto-2">O derivado armazenado não contém texto.</p> : (
          <pre role="region" aria-label="Texto documental armazenado" tabIndex={0} className="max-h-[28rem] overflow-auto p-3 font-mono text-sm leading-relaxed text-texto">{documento.texto}</pre>
        )}
      </div>}
      {ocr.total > 0 && <section aria-label="Regiões OCR" className="border-t border-borda pt-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>
            <h4 className="text-sm font-semibold text-texto">Mapa de regiões OCR</h4>
            <p className="mt-1 text-xs text-texto-2">Coordenadas na página/imagem renderizada; cada palavra permanece vinculada ao texto e à origem acima.</p>
          </div>
          <Selo tom="neutro">{ocr.total.toLocaleString("pt-BR")} REGIÕES</Selo>
        </div>
        {(localizandoRegiao || carregandoRegioes) && <Carregando texto={localizandoRegiao ? "Localizando ocorrência no mapa OCR…" : "Carregando regiões OCR desta página…"} />}
        {erroLocalizacao && <div className="mt-3"><Aviso>{erroLocalizacao}</Aviso></div>}
        {erroRegioes && <div className="mt-3"><Aviso>{erroRegioes}</Aviso></div>}
        {paginaRegioes && ocr.largura !== null && ocr.altura !== null && <div className="mt-3 rounded-md border border-borda bg-fundo p-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-xs font-medium text-texto">Mapa espacial da página</p>
            <p className="text-2xs text-texto-3">Geometria derivada · não substitui a imagem original</p>
          </div>
          <svg
            role="img"
            aria-label="Mapa espacial das regiões OCR"
            viewBox={`0 0 ${ocr.largura} ${ocr.altura}`}
            className="mt-3 max-h-80 w-full rounded-sm border border-borda bg-sup-2/40"
            preserveAspectRatio="xMidYMid meet"
          >
            {regioesVisiveis.map((regiao) => <rect
              key={regiao.id}
              x={regiao.x} y={regiao.y} width={regiao.largura} height={regiao.altura}
              tabIndex={0}
              role="button"
              aria-label={`Região ${regiao.ordem + 1}: ${regiao.texto}`}
              className={regiaoSelecionada === regiao.id ? "fill-ouro/25 stroke-ouro" : "fill-info/10 stroke-info/70"}
              vectorEffect="non-scaling-stroke"
              strokeWidth={regiaoSelecionada === regiao.id ? 2 : 1}
              onClick={() => setRegiaoSelecionada(regiao.id)}
              onKeyDown={(evento) => {
                if (evento.key === "Enter" || evento.key === " ") {
                  evento.preventDefault(); setRegiaoSelecionada(regiao.id);
                }
              }}
            />)}
          </svg>
          {regiaoSelecionada !== null && (() => {
            const selecionada = regioesVisiveis.find((regiao) => regiao.id === regiaoSelecionada);
            return selecionada ? <p role="status" className="mt-2 break-words text-xs text-texto-2">
              Região #{selecionada.ordem + 1} selecionada: <span className="font-medium text-texto">{selecionada.texto}</span>
              <span className="ml-2 font-mono text-texto-3">x {selecionada.x} · y {selecionada.y} · {selecionada.largura} × {selecionada.altura}px</span>
            </p> : null;
          })()}
        </div>}
        {paginaRegioes && <ol className="mt-3 grid gap-2 md:grid-cols-2" aria-label="Palavras e coordenadas OCR">
          {regioesVisiveis.map((regiao) => <li key={regiao.id} className={`rounded-sm border px-3 py-2 ${regiaoSelecionada === regiao.id ? "border-ouro/60 bg-ouro/5" : "border-borda bg-sup-2/40"}`}>
            <div className="flex items-start justify-between gap-2"><span className="break-words text-sm text-texto">{regiao.texto}</span><span className="font-mono text-2xs text-texto-3">#{regiao.ordem + 1}</span></div>
            <p className="mt-1 font-mono text-2xs text-texto-3">x {regiao.x} · y {regiao.y} · {regiao.largura} × {regiao.altura}px · bytes [{regiao.inicio_utf8}, {regiao.fim_utf8})</p>
            <p className="mt-1 text-2xs text-texto-3">Confiança: {regiao.confianca === null ? "não fornecida pelo motor" : `${regiao.confianca}%`}</p>
            <button type="button" className="mt-2 text-2xs font-medium text-info-claro hover:text-texto focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ouro" onClick={() => setRegiaoSelecionada(regiao.id)}>Localizar no mapa</button>
          </li>)}
        </ol>}
        <Paginacao total={ocr.total} deslocamento={deslocamentoRegioes} limite={100} mudar={(novo) => { setDeslocamentoRegioes(novo); setRegiaoSelecionada(null); }} />
      </section>}
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
