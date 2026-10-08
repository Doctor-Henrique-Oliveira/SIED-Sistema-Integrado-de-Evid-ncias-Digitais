// Anotações autorizadas e avaliação de ocorrências, não treinamento de modelos.
import { useEffect, useRef, useState, type FormEvent } from "react";
import { api, type AmostraSupervisionada, type AvaliacaoSupervisionada, type Caso, type DatasetSupervisionado, type DocumentoIndexado, type EntidadeDocumento, type EstadoRevisaoEntidade, type Pagina } from "../api/cliente";
import { dataHora } from "../util/formatar";
import { Botao } from "./Botao";
import { AreaTexto, Campo, Selecao } from "./Campo";
import { Aviso, Carregando, Hash, Paginacao, Selo, tabela, Vazio } from "./Estrutura";

const ESTADOS: [EstadoRevisaoEntidade, string][] = [["nao_revisado", "Não revisado"], ["confirmado", "Rótulo conferido"], ["inconclusivo", "Inconclusivo"], ["descartado", "Descartado"]];
function mensagem(erro: unknown) { return erro instanceof Error ? erro.message : "A operação de avaliação local não pôde ser concluída."; }
export function trechoUtf8(texto: string, inicio: number, fim: number): string | null {
  if (!Number.isSafeInteger(inicio) || !Number.isSafeInteger(fim) || inicio < 0 || fim <= inicio || fim - inicio > 4096) return null;
  const bytes = new TextEncoder().encode(texto);
  if (fim > bytes.length) return null;
  try { return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes.subarray(inicio, fim)); }
  catch { return null; }
}
function metrica(valor: number | null) {
  return valor === null ? "Não calculável" : valor.toLocaleString("pt-BR", { style: "percent", maximumFractionDigits: 1 });
}

export function DatasetsLocais({ caso }: { caso: Caso }) { return <SessaoDatasets key={caso.id} caso={caso} />; }
function SessaoDatasets({ caso }: { caso: Caso }) {
  const [datasets, setDatasets] = useState<Pagina<DatasetSupervisionado> | null>(null);
  const [deslocamento, setDeslocamento] = useState(0); const [recarga, setRecarga] = useState(0);
  const [selecionado, setSelecionado] = useState<DatasetSupervisionado | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [nome, setNome] = useState(""); const [autorizacao, setAutorizacao] = useState(""); const [retencao, setRetencao] = useState("");
  const [autorizado, setAutorizado] = useState(false); const [criando, setCriando] = useState(false);
  const montado = useRef(true);
  useEffect(() => { montado.current = true; return () => { montado.current = false; }; }, []);
  useEffect(() => {
    let atual = true; setDatasets(null);
    void api.listarDatasets(caso.id, deslocamento, 20).then((pagina) => {
      if (!atual || !montado.current) return;
      if (pagina.itens.length > 20 || pagina.itens.some((dataset) => dataset.caso_id !== caso.id || !Number.isSafeInteger(dataset.id) || dataset.id <= 0)) throw new Error("A lista de datasets recebida não corresponde ao caso atual.");
      setDatasets(pagina);
      setSelecionado((antigo) => antigo ? pagina.itens.find((dataset) => dataset.id === antigo.id) ?? antigo : null);
    }).catch((falha) => { if (atual && montado.current) setErro(mensagem(falha)); });
    return () => { atual = false; };
  }, [caso.id, deslocamento, recarga]);
  async function criar(evento: FormEvent) {
    evento.preventDefault();
    if (!autorizado || criando || !nome.trim() || !autorizacao.trim() || !retencao.trim()) return;
    setCriando(true); setErro(null);
    try {
      const dataset = await api.criarDataset(caso.id, nome, autorizacao, retencao);
      if (!montado.current) return;
      if (dataset.caso_id !== caso.id || !Number.isSafeInteger(dataset.id) || dataset.id <= 0) throw new Error("O dataset criado não corresponde ao caso atual.");
      setSelecionado(dataset); setDeslocamento(0); setRecarga((atual) => atual + 1);
      setNome(""); setAutorizacao(""); setRetencao(""); setAutorizado(false);
    } catch (falha) { if (montado.current) setErro(mensagem(falha)); }
    finally { if (montado.current) setCriando(false); }
  }
  return <section aria-label="Datasets e avaliação local" className="space-y-4">
    <div className="painel p-4">
      <div className="flex flex-wrap items-center justify-between gap-2"><h2 className="text-sm font-semibold text-texto">Revisão supervisionada · avaliação local</h2><Selo tom="neutro">SEM TREINAMENTO / SEM TRANSFERÊNCIA</Selo></div>
      <p className="mt-2 text-xs leading-relaxed text-texto-2">Datasets são conjuntos de anotações explicitamente autorizadas, restritos a este caso. Não treinam modelos, não copiam a extração para um catálogo global e não enviam dados. Rótulos se referem a ocorrências candidatas, não à identidade ou veracidade do conteúdo.</p>
      <details className="mt-3"><summary className="cursor-pointer text-xs text-ouro-claro">Criar dataset autorizado</summary>
        <form aria-label="Criar dataset local" onSubmit={(evento) => void criar(evento)} className="mt-3 space-y-3">
          <Campo rotulo="Nome do dataset" value={nome} maxLength={50} required disabled={criando} onChange={(evento) => setNome(evento.target.value)} />
          <AreaTexto rotulo="Autorização declarada" value={autorizacao} maxLength={1000} required disabled={criando} onChange={(evento) => setAutorizacao(evento.target.value)} ajuda="Registre a autorização para esta finalidade e para este caso. O SIED não concede autorização jurídica." />
          <AreaTexto rotulo="Retenção declarada" value={retencao} maxLength={500} required disabled={criando} onChange={(evento) => setRetencao(evento.target.value)} ajuda="Política registrada como texto; não configura exclusão automática de dados." />
          <label className="flex items-start gap-2 text-xs text-texto-2"><input type="checkbox" checked={autorizado} disabled={criando} onChange={(evento) => setAutorizado(evento.target.checked)} />Confirmo a autorização para usar amostras deste caso nesta avaliação local.</label>
          <Botao type="submit" tamanho="p" variante="secundario" carregando={criando} disabled={!autorizado || !nome.trim() || !autorizacao.trim() || !retencao.trim()}>Criar dataset</Botao>
        </form>
      </details>
    </div>
    {erro && <Aviso>{erro}</Aviso>}
    <div className="painel p-4"><h3 className="text-sm font-semibold text-texto">Datasets deste caso</h3>
      {datasets === null ? <Carregando texto="Carregando datasets locais…" /> : datasets.itens.length === 0 ? <div className="mt-3"><Vazio titulo="Nenhum dataset autorizado">Crie somente após declarar autorização e retenção. Nenhuma amostra é coletada automaticamente.</Vazio></div> : <ul className="mt-3 divide-y divide-borda" aria-label="Datasets locais">{datasets.itens.map((dataset) => <li key={dataset.id} className="flex flex-wrap items-center justify-between gap-2 py-2"><div><p className="text-sm text-texto">{dataset.nome} · v{dataset.versao}</p><p className="mt-1 text-xs text-texto-3">{dataset.total_amostras} intervalo(s) · {dataset.total_versoes} versão(ões) de anotação · {dataset.autor}</p></div><Botao tamanho="p" variante="secundario" onClick={() => { setSelecionado(dataset); setErro(null); }}>Abrir dataset #{dataset.id}</Botao></li>)}</ul>}
      {datasets && <Paginacao total={datasets.total} deslocamento={deslocamento} limite={20} mudar={setDeslocamento} />}
    </div>
    {selecionado && <DatasetAberto key={selecionado.id} caso={caso} dataset={selecionado} atualizado={() => setRecarga((atual) => atual + 1)} />}
  </section>;
}

function DatasetAberto({ caso, dataset, atualizado }: { caso: Caso; dataset: DatasetSupervisionado; atualizado: () => void }) {
  const [documentoId, setDocumentoId] = useState(""); const [documento, setDocumento] = useState<DocumentoIndexado | null>(null);
  const [candidatos, setCandidatos] = useState<EntidadeDocumento[]>([]); const [abrindo, setAbrindo] = useState(false);
  const [inicio, setInicio] = useState(""); const [fim, setFim] = useState(""); const [esperado, setEsperado] = useState("positivo");
  const [estado, setEstado] = useState<EstadoRevisaoEntidade>("nao_revisado"); const [nota, setNota] = useState("");
  const [autorizaAmostra, setAutorizaAmostra] = useState(false); const [salvando, setSalvando] = useState(false);
  const [amostras, setAmostras] = useState<Pagina<AmostraSupervisionada> | null>(null); const [deslocamento, setDeslocamento] = useState(0); const [recarga, setRecarga] = useState(0);
  const [avaliacao, setAvaliacao] = useState<AvaliacaoSupervisionada | null>(null); const [avaliando, setAvaliando] = useState(false);
  const [erro, setErro] = useState<string | null>(null); const [sucesso, setSucesso] = useState<string | null>(null);
  const montado = useRef(true); const leitura = useRef(0); const operacao = useRef<string | null>(null);
  useEffect(() => {
    montado.current = true;
    return () => { montado.current = false; leitura.current += 1; const antiga = operacao.current; operacao.current = null; if (antiga) void api.cancelarDescoberta(antiga).catch(() => undefined); };
  }, []);
  useEffect(() => {
    let atual = true; setAmostras(null);
    void api.amostrasDataset(caso.id, dataset.id, deslocamento, 20).then((pagina) => {
      if (!atual || !montado.current) return;
      if (pagina.itens.length > 20 || pagina.itens.some((amostra) => amostra.dataset_id !== dataset.id)) throw new Error("As amostras recebidas não correspondem ao dataset selecionado.");
      setAmostras(pagina);
    }).catch((falha) => { if (atual && montado.current) setErro(mensagem(falha)); });
    return () => { atual = false; };
  }, [caso.id, dataset.id, deslocamento, recarga]);
  const inicioNumero = Number(inicio); const fimNumero = Number(fim);
  const trecho = documento && inicio !== "" && fim !== "" ? trechoUtf8(documento.texto, inicioNumero, fimNumero) : null;
  const previstos = candidatos.filter((candidato) => candidato.inicio_utf8 === inicioNumero && candidato.fim_utf8 === fimNumero);
  const previsto = trecho !== null && previstos.length > 0;
  async function abrirDocumento(evento: FormEvent) {
    evento.preventDefault(); const id = Number(documentoId);
    if (!Number.isSafeInteger(id) || id <= 0 || abrindo || salvando) return;
    const atual = ++leitura.current; setAbrindo(true); setDocumento(null); setCandidatos([]); setInicio(""); setFim(""); setErro(null); setAutorizaAmostra(false);
    try {
      const [doc, entidades] = await Promise.all([api.obterDocumento(caso.id, id), api.entidadesDocumento(caso.id, id)]);
      if (!montado.current || leitura.current !== atual) return;
      if (doc.id !== id || doc.caso_id !== caso.id || entidades.some((entidade) => entidade.documento_id !== id || trechoUtf8(doc.texto, entidade.inicio_utf8, entidade.fim_utf8) !== entidade.literal)) throw new Error("O documento/candidatos recebidos não correspondem à origem selecionada.");
      setDocumento(doc); setCandidatos(entidades); setSucesso(null);
    } catch (falha) { if (montado.current && leitura.current === atual) setErro(mensagem(falha)); }
    finally { if (montado.current && leitura.current === atual) setAbrindo(false); }
  }
  async function adicionar(evento: FormEvent) {
    evento.preventDefault(); if (!documento || trecho === null || !autorizaAmostra || salvando || avaliando) return;
    setSalvando(true); setErro(null); setSucesso(null);
    try {
      const amostra = await api.adicionarAmostra(caso.id, dataset.id, { documento_id: documento.id, inicio_utf8: inicioNumero, fim_utf8: fimNumero, esperado: esperado === "positivo", previsto, estado_rotulo: estado, nota: nota === "" ? null : nota });
      if (!montado.current) return;
      if (amostra.dataset_id !== dataset.id || amostra.documento_id !== documento.id || amostra.inicio_utf8 !== inicioNumero || amostra.fim_utf8 !== fimNumero || amostra.trecho !== trecho || amostra.previsto !== previsto || amostra.sha256_texto_documento !== documento.sha256_texto || amostra.sha256_origem !== documento.sha256_origem) throw new Error("A amostra retornada não corresponde ao intervalo selecionado.");
      setSucesso(`Amostra #${amostra.id} · versão ${amostra.versao} registrada. Texto/origem não foram alterados.`);
      setAutorizaAmostra(false); setAvaliacao(null); setDeslocamento(0); setRecarga((atual) => atual + 1); atualizado();
    } catch (falha) { if (montado.current) setErro(mensagem(falha)); }
    finally { if (montado.current) setSalvando(false); }
  }
  async function avaliar() {
    if (operacao.current || salvando) return;
    const id = crypto.randomUUID(); operacao.current = id; setAvaliando(true); setAvaliacao(null); setErro(null);
    try {
      const resultado = await api.avaliarDataset(caso.id, dataset.id, id);
      if (!montado.current || operacao.current !== id) return;
      if (resultado.dataset_id !== dataset.id) throw new Error("A avaliação não corresponde ao dataset selecionado.");
      setAvaliacao(resultado);
    } catch (falha) { if (montado.current && operacao.current === id) setErro(mensagem(falha)); }
    finally { if (montado.current && operacao.current === id) { operacao.current = null; setAvaliando(false); } }
  }
  return <section aria-label="Dataset autorizado aberto" className="painel space-y-4 p-4">
    <div><h3 className="text-sm font-semibold text-texto">{dataset.nome} · dataset #{dataset.id} · v{dataset.versao}</h3><p className="mt-1 text-xs text-texto-3">Criado por {dataset.autor} · {dataHora(dataset.criado_em, caso.fuso_horario)} · escopo: {dataset.caso_id}</p>
      <details className="mt-2 text-xs text-texto-2"><summary className="cursor-pointer">Autorização e retenção registradas</summary><p className="mt-2 whitespace-pre-wrap break-words">{dataset.autorizacao}</p><p className="mt-2 whitespace-pre-wrap break-words">{dataset.retencao}</p></details>
    </div>
    <form aria-label="Abrir documento para anotação" onSubmit={(evento) => void abrirDocumento(evento)} className="flex flex-wrap items-end gap-2"><div className="min-w-52 flex-1"><Campo rotulo="ID do documento indexado" type="number" min={1} step={1} value={documentoId} disabled={abrindo || salvando} onChange={(evento) => setDocumentoId(evento.target.value)} ajuda="ID do derivado disponível na proveniência em Arquivos/Busca. Não recebe caminho arbitrário." /></div><Botao type="submit" tamanho="p" variante="secundario" carregando={abrindo} disabled={!documentoId}>Abrir texto para anotação</Botao></form>
    {documento && <div className="space-y-3 border-t border-borda pt-3">
      <p className="break-all font-mono text-xs text-texto-2">{documento.caminho} · {documento.pagina === null ? "texto nativo" : `página ${documento.pagina} de ${documento.total_paginas}`} · {documento.motor_id} v{documento.motor_versao}</p>
      <p className="text-xs text-texto-3">Snapshot armazenado; a origem atual não foi relida. Motor de candidatos: {documento.entidades_motor_versao}.</p>
      <pre role="region" aria-label="Texto de referência da anotação" tabIndex={0} className="max-h-64 overflow-auto rounded-md border border-borda bg-fundo p-3 font-mono text-sm text-texto">{documento.texto}</pre>
      <form aria-label="Adicionar amostra autorizada" onSubmit={(evento) => void adicionar(evento)} className="space-y-3">
        <Selecao rotulo="Ocorrência detectada para seleção" value="" disabled={salvando} onChange={(evento) => { const candidato = candidatos.find((entidade) => entidade.id === Number(evento.target.value)); if (candidato) { setInicio(String(candidato.inicio_utf8)); setFim(String(candidato.fim_utf8)); setAutorizaAmostra(false); } }}><option value="">Selecione candidato real ou informe intervalo manual</option>{candidatos.map((candidato) => <option key={candidato.id} value={candidato.id}>{candidato.tipo} · {candidato.literal} · [{candidato.inicio_utf8}, {candidato.fim_utf8})</option>)}</Selecao>
        <div className="grid gap-3 sm:grid-cols-2"><Campo rotulo="Início do intervalo UTF-8" type="number" min={0} step={1} value={inicio} disabled={salvando} onChange={(evento) => { setInicio(evento.target.value); setAutorizaAmostra(false); }} /><Campo rotulo="Fim do intervalo UTF-8" type="number" min={1} step={1} value={fim} disabled={salvando} onChange={(evento) => { setFim(evento.target.value); setAutorizaAmostra(false); }} /></div>
        {trecho === null ? <p className="text-xs text-texto-3">Selecione intervalo completo de até 4.096 bytes UTF-8; fronteiras no meio de um caractere são recusadas.</p> : <div className="rounded-md border border-borda bg-fundo p-3"><p className="sobrelinha">Prévia exata da amostra</p><pre aria-label="Trecho selecionado para amostra" className="mt-2 whitespace-pre-wrap break-all font-mono text-sm text-texto">{trecho}</pre><p className="mt-2 text-xs text-texto-2">Previsão armazenada: {previsto ? `ocorrência exata detectada (${previstos.map((candidato) => candidato.tipo).join(", ")})` : "nenhuma ocorrência exata detectada neste intervalo"}. O backend confere esta informação novamente.</p></div>}
        <Selecao rotulo="Resultado esperado pelo analista" value={esperado} disabled={salvando} onChange={(evento) => setEsperado(evento.target.value)}><option value="positivo">Positivo: deveria ser identificado como ocorrência candidata</option><option value="negativo">Negativo: não deveria ser identificado como ocorrência candidata</option></Selecao>
        <Selecao rotulo="Estado do rótulo humano" value={estado} disabled={salvando} onChange={(evento) => setEstado(evento.target.value as EstadoRevisaoEntidade)}>{ESTADOS.map(([valor, rotulo]) => <option key={valor} value={valor}>{rotulo}</option>)}</Selecao>
        <AreaTexto rotulo="Nota da anotação" maxLength={1000} value={nota} disabled={salvando} onChange={(evento) => setNota(evento.target.value)} />
        <label className="flex items-start gap-2 text-xs text-texto-2"><input type="checkbox" checked={autorizaAmostra} disabled={salvando || trecho === null} onChange={(evento) => setAutorizaAmostra(evento.target.checked)} />Autorizo registrar este intervalo e este rótulo no dataset local selecionado.</label>
        <Botao type="submit" tamanho="p" variante="secundario" carregando={salvando} disabled={trecho === null || !autorizaAmostra || avaliando}>Registrar amostra</Botao>
        <p className="text-xs text-texto-3">Somente rótulos explicitamente conferidos entram nas métricas. Alterar um rótulo cria nova versão de anotação; não modifica texto/evidências e não retreina o motor.</p>
      </form>
    </div>}
    {erro && <Aviso>{erro}</Aviso>}{sucesso && <p role="status" className="text-xs text-ok-claro">{sucesso}</p>}
    <div className="border-t border-borda pt-3"><h4 className="text-sm font-semibold text-texto">Amostras registradas · versões</h4>{amostras === null ? <Carregando texto="Carregando amostras…" /> : amostras.itens.length === 0 ? <p className="mt-2 text-xs text-texto-3">Nenhuma amostra registrada.</p> : <ul className="mt-2 divide-y divide-borda" aria-label="Amostras supervisionadas">{amostras.itens.map((amostra) => <li key={amostra.id} className="space-y-1 py-2 text-xs text-texto-2"><p className="font-mono">Amostra #{amostra.id} · v{amostra.versao} · derivado #{amostra.documento_id} · [{amostra.inicio_utf8}, {amostra.fim_utf8})</p><pre className="whitespace-pre-wrap break-all font-mono text-sm text-texto">{amostra.trecho}</pre><p>Esperado: {amostra.esperado ? "positivo" : "negativo"} · previsão: {amostra.previsto ? "ocorrência candidata" : "sem ocorrência"} · {ESTADOS.find(([valor]) => valor === amostra.estado_rotulo)?.[1] ?? amostra.estado_rotulo}</p><p>Motor de candidatos: {amostra.entidades_motor_versao} · {amostra.revisor} · {dataHora(amostra.criado_em, caso.fuso_horario)}</p><details><summary className="cursor-pointer">Hashes da amostra e origem</summary><dl className="mt-2 space-y-1"><div><dt>Origem registrada</dt><dd><Hash valor={amostra.sha256_origem} /></dd></div><div><dt>Texto de referência</dt><dd><Hash valor={amostra.sha256_texto_documento} /></dd></div><div><dt>Amostra versionada</dt><dd><Hash valor={amostra.sha256_amostra} /></dd></div></dl></details>{amostra.nota !== null && <p className="whitespace-pre-wrap break-words">{amostra.nota}</p>}</li>)}</ul>}{amostras && <Paginacao total={amostras.total} deslocamento={deslocamento} limite={20} mudar={setDeslocamento} />}</div>
    <div className="border-t border-borda pt-3"><Botao tamanho="p" variante="secundario" carregando={avaliando} disabled={salvando} onClick={() => void avaliar()}>Avaliar rótulos conferidos</Botao><p className="mt-2 text-xs text-texto-3">Avaliação explícita das anotações existentes, sem treinamento. Recall refere-se somente aos intervalos manualmente anotados, não ao universo do caso.</p></div>
    {avaliacao && <section aria-label="Métricas da avaliação local" className="space-y-3">
      <dl className="grid grid-cols-2 gap-3 text-xs sm:grid-cols-5">{[["Precisão", avaliacao.precision], ["Recall anotado", avaliacao.recall], ["F1", avaliacao.f1], ["Acurácia", avaliacao.acuracia], ["Cobertura de revisão", avaliacao.cobertura]].map(([nomeMetrica, valor]) => <div key={String(nomeMetrica)}><dt className="sobrelinha">{nomeMetrica}</dt><dd className="mt-1 text-sm text-texto">{metrica(valor as number | null)}</dd></div>)}</dl>
      <p className="text-xs text-texto-2">{avaliacao.elegiveis} intervalo(s) elegível(is) de {avaliacao.total_amostras} · {avaliacao.total_versoes} versão(ões) · {avaliacao.nao_revisadas} não revisado(s), {avaliacao.inconclusivas} inconclusivo(s), {avaliacao.descartadas} descartado(s).</p>
      <table className={tabela.base} aria-label="Matriz de confusão anotada"><thead className={tabela.cabeca}><tr><th className={tabela.th}>Rótulo esperado</th><th className={tabela.th}>Ocorrência prevista</th><th className={tabela.th}>Sem ocorrência prevista</th></tr></thead><tbody className={tabela.corpo}><tr><th className={tabela.td}>Positivo</th><td className={tabela.td}>TP: {avaliacao.tp}</td><td className={tabela.td}>FN: {avaliacao["fn"]}</td></tr><tr><th className={tabela.td}>Negativo</th><td className={tabela.td}>FP: {avaliacao.fp}</td><td className={tabela.td}>TN: {avaliacao.tn}</td></tr></tbody></table>
      <p className="break-all font-mono text-xs text-texto-3">Motores avaliados: {avaliacao.motores_avaliados.join(" · ") || "Nenhum motor com amostra elegível"}</p>
      <ul className="list-inside list-disc space-y-1 text-xs text-texto-2" aria-label="Limitações da avaliação">{avaliacao.limites.map((limite, indice) => <li key={indice}>{limite}</li>)}</ul>
    </section>}
  </section>;
}
