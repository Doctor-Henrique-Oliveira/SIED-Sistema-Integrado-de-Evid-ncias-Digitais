import { useEffect, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  api, type Caso, type ConsultaDescobertaSalva, type ExecucaoDescoberta, type FiltroBusca,
  type PedidoDescoberta, type ResultadoDescoberta, type VocabularioPesquisa, type VersaoVocabularioPesquisa,
} from "../api/cliente";
import {
  chaves, useConsultasDescoberta, useExecutarDescoberta, useExtracoes,
  usePaginarDescoberta, useSalvarVocabularioPesquisa, useVocabulariosPesquisa,
} from "../api/consultas";
import { dataHora, dataHoraMs, limiteDia, numero, tamanho } from "../util/formatar";
import { nomeApp, nomeCategoria, nomeTipoArtefato, rotuloApp } from "../util/rotulos";
import { importarVocabularioPesquisa, MAX_RAMOS, MAX_TERMOS, termosHabilitados } from "../util/vocabularioPesquisa";
import { TrechoDestacado } from "../rotas/PaginaBusca";
import { Botao } from "./Botao";
import { Aviso, Carregando, Cartao, Definicoes, Hash, Paginacao, Selo, tabela, Vazio } from "./Estrutura";
import { destinoRegistro } from "./LinhaTempo";

export type ModoDescobertaLocal = PedidoDescoberta["modo"];
const LIMITE = 50;
const vazio: VocabularioPesquisa = { nome: "", origem: "", ramos: [] };
const nomes: Record<ModoDescobertaLocal, string> = {
  expansao: "Expansão explícita", literal: "Correspondência literal", indicadores: "Indicadores locais",
};
type TipoIndicador = NonNullable<PedidoDescoberta["indicador"]>;
const indicadores: [TipoIndicador, string][] = [
  ["ip", "IP"], ["email", "E-mail"], ["url", "URL"], ["dominio", "Domínio"],
  ["hash", "Hash"], ["telefone", "Telefone"], ["coordenada", "Coordenada"],
];

/** O formulário não dispara inferência ao digitar/trocar modo: somente submissão explícita. */
export function DescobertaLocal({ caso, modo }: { caso: Caso; modo: ModoDescobertaLocal }) {
  const extracoes = useExtracoes(caso.id);
  const vocabularios = useVocabulariosPesquisa(caso.id);
  const consultas = useConsultasDescoberta(caso.id);
  const executar = useExecutarDescoberta(caso.id);
  const paginar = usePaginarDescoberta();
  const salvar = useSalvarVocabularioPesquisa(caso.id);
  const qc = useQueryClient();
  const exportar = useMutation({
    mutationFn: api.exportarVocabularioPesquisa,
    onSuccess: () => void qc.invalidateQueries({ queryKey: chaves.auditoria(caso.id) }),
  });
  const [texto, setTexto] = useState("");
  const [app, setApp] = useState("");
  const [tipo, setTipo] = useState("");
  const [extracao, setExtracao] = useState("");
  const [de, setDe] = useState("");
  const [ate, setAte] = useState("");
  const [indicador, setIndicador] = useState<TipoIndicador>("ip");
  const [dados, setDados] = useState<VocabularioPesquisa>(vazio);
  const [vocabularioId, setVocabularioId] = useState<number | null>(null);
  const [json, setJson] = useState("");
  const [erroLocal, setErroLocal] = useState<string | null>(null);
  const [execucao, setExecucao] = useState<ExecucaoDescoberta | null>(null);
  const [resultado, setResultado] = useState<ResultadoDescoberta | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const [erroResultado, setErroResultado] = useState<string | null>(null);
  useEffect(() => { setErroLocal(null); }, [modo]);
  const ocupado = executar.isPending || paginar.isPending;
  const vocabularioOcupado = salvar.isPending || exportar.isPending;
  const ativos = termosHabilitados(dados);
  const totalTermos = dados.ramos.reduce((n, ramo) => n + ramo.termos.filter((t) => t.trim()).length, 0);
  const versao = vocabularios.data?.find((v) => v.id === vocabularioId);

  function editar(novos: VocabularioPesquisa) {
    setDados(novos); setVocabularioId(null);
    // Status/JSON antigos não descrevem esta edição ainda não salva.
    salvar.reset(); exportar.reset();
  }
  function carregarVersao(v: VersaoVocabularioPesquisa | undefined) {
    setErroLocal(null);
    setVocabularioId(v?.id ?? null);
    setDados(v ? structuredClone(v.dados) : structuredClone(vazio));
    salvar.reset(); exportar.reset();
  }
  async function salvarVersao() {
    setErroLocal(null);
    try {
      const validos = importarVocabularioPesquisa(JSON.stringify({ ...dados, ramos: dados.ramos.map((r) => ({ ...r, termos: r.termos.map((t) => t.trim()).filter(Boolean) })) }));
      const v = await salvar.mutateAsync(validos);
      setDados(v.dados); setVocabularioId(v.id);
    } catch (e) { setErroLocal(e instanceof Error ? e.message : String(e)); }
  }
  function importar() {
    setErroLocal(null);
    try { editar(importarVocabularioPesquisa(json)); }
    catch (e) { setErroLocal(e instanceof Error ? e.message : String(e)); }
  }
  async function iniciar(pedido: PedidoDescoberta, id: number | null) {
    if (ocupado || vocabularioOcupado) return;
    setErroLocal(null); setErroResultado(null);
    // Remove resultados anteriores durante uma execução; nunca apresenta a página
    // antiga como se fosse a resposta da consulta que ainda está processando.
    setResultado(null); setExecucao(null);
    try {
      const e = await executar.mutateAsync({ pedido, vocabularioId: id });
      setExecucao(e); setResultado(e.resultado); setDeslocamento(0);
    } catch (e) { setErroResultado(e instanceof Error ? e.message : String(e)); }
  }
  function buscar(e: FormEvent) {
    e.preventDefault();
    if (ocupado || vocabularioOcupado) return;
    if (modo !== "indicadores" && !texto.trim()) { setErroLocal("Informe a consulta original."); return; }
    if (de && ate && de > ate) { setErroLocal("O início do período deve ser anterior ao fim."); return; }
    if (modo === "expansao" && dados.ramos.length && vocabularioId === null) {
      setErroLocal("Salve uma versão da árvore antes de usar seus termos na pesquisa."); return;
    }
    const filtro: FiltroBusca = { texto: modo === "literal" ? texto : texto.trim() };
    if (app) filtro.app = app;
    if (tipo) filtro.tipo = tipo;
    if (extracao) filtro.extracao_id = extracao;
    if (de) { const ms = limiteDia(de, caso.fuso_horario); if (ms != null) filtro.de_ms = ms; }
    if (ate) { const ms = limiteDia(ate, caso.fuso_horario, true); if (ms != null) filtro.ate_ms = ms; }
    void iniciar({ filtro, modo, termos: modo === "expansao" ? ativos : [], ...(modo === "indicadores" ? { indicador } : {}) }, modo === "expansao" ? vocabularioId : null);
  }
  async function mudarPagina(proximo: number) {
    if (!execucao || ocupado || vocabularioOcupado) return;
    setErroResultado(null);
    try {
      const r = await paginar.mutateAsync({ consultaId: execucao.consulta.id, deslocamento: proximo, limite: LIMITE });
      setResultado(r); setDeslocamento(proximo);
    } catch (e) { setResultado(null); setErroResultado(e instanceof Error ? e.message : String(e)); }
  }

  return (
    <div className="space-y-4">
      <section className="painel p-4" aria-label="Descoberta local">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-sm font-semibold text-texto">{nomes[modo]}</h2>
          <div className="flex gap-2"><Selo tom="info">AI-0 · local</Selo><Selo>Caso atual</Selo></div>
        </div>
        <p className="mb-3 text-sm text-texto-2">
          {modo === "expansao" ? "Termos escolhidos por você, combinados por OU. A árvore não atribui significado a palavras ou emojis." :
            modo === "literal" ? "Sequência exata, incluindo maiúsculas, acentos e combinações Unicode de emojis. Espaços nas bordas são desconsiderados. Sem expansão ou interpretação de significado." :
              "Ocorrências candidatas em campos de registros importados. Validação técnica não comprova titularidade, identidade, vínculo ou relevância investigativa."}
        </p>
        <form onSubmit={buscar} role="search" className="space-y-3">
          <div className="flex flex-wrap gap-2">
            {modo === "indicadores" && <select className="entrada" aria-label="Tipo de indicador" value={indicador} onChange={(e) => setIndicador(e.target.value as TipoIndicador)}>
              {indicadores.map(([valor, nome]) => <option key={valor} value={valor}>{nome}</option>)}
            </select>}
            <input type="search" aria-label="Consulta original" value={texto} onChange={(e) => setTexto(e.target.value)} maxLength={200}
              placeholder={modo === "indicadores" ? "Opcional: limitar ao texto informado" : modo === "literal" ? "Texto ou sequência exata de emojis" : "Consulta original"}
              className="entrada h-10 min-w-48 flex-1 text-base" />
            <Botao type="submit" carregando={executar.isPending} disabled={ocupado || vocabularioOcupado}>Pesquisar no caso</Botao>
          </div>
          <div className="flex flex-wrap items-center gap-3 text-xs text-texto-2">
            <select aria-label="Extração da descoberta" className="entrada" value={extracao} onChange={(e) => setExtracao(e.target.value)}>
              <option value="">Todas as extrações</option>
              {extracoes.data?.map((x) => <option key={x.id} value={x.id}>{x.importador_nome} · {x.iniciada_em.slice(0, 10)}</option>)}
            </select>
            <select aria-label="App da descoberta" className="entrada" value={app} onChange={(e) => setApp(e.target.value)}>
              <option value="">Todos os apps</option>{Object.entries(nomeApp).map(([id, nome]) => <option key={id} value={id}>{nome}</option>)}
            </select>
            <select aria-label="Tipo de registro" className="entrada" value={tipo} onChange={(e) => setTipo(e.target.value)}>
              <option value="">Todos os tipos</option>{Object.entries(nomeTipoArtefato).map(([id, nome]) => <option key={id} value={id}>{nome}</option>)}
            </select>
            <label className="flex items-center gap-2">De <input aria-label="Data inicial da descoberta" type="date" className="entrada" value={de} onChange={(e) => setDe(e.target.value)} /></label>
            <label className="flex items-center gap-2">Até <input aria-label="Data final da descoberta" type="date" className="entrada" value={ate} onChange={(e) => setAte(e.target.value)} /></label>
          </div>
          <p className="text-xs text-texto-3">Datas no fuso do caso ({caso.fuso_horario}). Sem rede, OCR, semântica ou leitura do conteúdo de PDFs/arquivos nesta etapa.</p>
          {modo !== "indicadores" && (app || tipo || de || ate) && <p className="text-xs text-atencao-claro">Filtros de app, tipo e período se aplicam a registros. Com esses filtros, nomes de arquivo ficam fora da busca.</p>}
          {modo === "expansao" && <fieldset aria-label="Configuração do vocabulário" disabled={ocupado || vocabularioOcupado} className="min-w-0 border-t border-borda pt-3">
            <label className="mb-2 flex flex-wrap items-center gap-2 text-xs text-texto-2">Versão do vocabulário
              <select aria-label="Versão do vocabulário" className="entrada min-w-52" disabled={vocabularios.isPending || ocupado}
                value={vocabularioId ?? ""} onChange={(e) => carregarVersao(vocabularios.data?.find((v) => v.id === Number(e.target.value)))}>
                <option value="">Árvore vazia / edição não salva</option>
                {vocabularios.data?.map((v) => <option key={v.id} value={v.id}>{v.dados.nome} · v{v.versao}</option>)}
              </select>
            </label>
            {vocabularios.isPending && <Carregando texto="Lendo versões do caso…" />}
            {vocabularios.isError && <Aviso>{vocabularios.error.message}</Aviso>}
            {versao && <div className="mb-3 text-xs text-texto-3">v{versao.versao} · {versao.autor} · {dataHora(versao.criado_em, caso.fuso_horario)} · Origem: {versao.dados.origem}</div>}
            <div className="space-y-2" aria-label="Árvore de pesquisa">
              {dados.ramos.length === 0 && <p className="py-1 text-sm text-texto-3">Nenhum ramo definido. Não há associações ou dicionários pré-carregados.</p>}
              {dados.ramos.map((ramo, i) => <div key={i} className="flex flex-wrap items-start gap-3 border-l-2 border-borda-2 pl-3">
                <label className="flex min-w-40 items-center gap-2 text-sm text-texto">
                  <input type="checkbox" checked={ramo.habilitado} aria-label={`Usar ramo ${ramo.nome || i + 1}`} disabled={ocupado}
                    onChange={(e) => editar({ ...dados, ramos: dados.ramos.map((r, j) => j === i ? { ...r, habilitado: e.target.checked } : r) })} />
                  {ramo.nome || `Ramo ${i + 1}`}
                </label>
                <div className="flex flex-1 flex-wrap gap-1.5">{ramo.termos.map((t, j) => <span key={j} className={`rounded-sm bg-sup-2 px-2 py-0.5 text-sm ${ramo.habilitado ? "text-texto" : "text-texto-3 line-through"}`}>{t}</span>)}</div>
              </div>)}
            </div>
            {dados.ramos.length > 0 && vocabularioId === null && <p className="mt-2 text-xs text-atencao-claro">Árvore alterada. Salve uma nova versão antes de pesquisar; a versão anterior não será modificada.</p>}
            <div className="mt-3 grid gap-2 border-t border-borda pt-3 text-xs text-texto-2" aria-label="Termos planejados">
              <p>Consulta original: <span className="break-all font-medium text-texto">{texto || "não informada"}</span></p>
              <p>Expansões habilitadas ({ativos.length}): <span className="break-all text-texto">{ativos.join(" · ") || "nenhuma"}</span></p>
            </div>
            <details className="mt-3 border-t border-borda pt-3">
              <summary className="cursor-pointer text-sm text-ouro-claro">Editar / importar / exportar vocabulário do caso</summary>
              <div className="mt-3 space-y-3">
                <div className="grid gap-3 md:grid-cols-2">
                  <label className="space-y-1 text-xs text-texto-2">Nome do vocabulário<input className="entrada w-full" aria-label="Nome do vocabulário" maxLength={100} value={dados.nome} onChange={(e) => editar({ ...dados, nome: e.target.value })} /></label>
                  <label className="space-y-1 text-xs text-texto-2">Origem / justificativa<input className="entrada w-full" aria-label="Origem do vocabulário" maxLength={300} value={dados.origem} onChange={(e) => editar({ ...dados, origem: e.target.value })} /></label>
                </div>
                {dados.ramos.map((ramo, i) => <div key={i} className="grid items-start gap-2 border-t border-borda pt-3 md:grid-cols-[12rem_minmax(0,1fr)_auto]">
                  <label className="space-y-1 text-xs text-texto-2">Nome do ramo {i + 1}<input aria-label={`Nome do ramo ${i + 1}`} className="entrada w-full" maxLength={100} value={ramo.nome}
                    onChange={(e) => editar({ ...dados, ramos: dados.ramos.map((r, j) => i === j ? { ...r, nome: e.target.value } : r) })} /></label>
                  <label className="space-y-1 text-xs text-texto-2">Termos · um por linha<textarea aria-label={`Termos do ramo ${i + 1}`} className="entrada min-h-20 w-full" rows={3} maxLength={MAX_TERMOS * 401} value={ramo.termos.join("\n")}
                    onChange={(e) => editar({ ...dados, ramos: dados.ramos.map((r, j) => i === j ? { ...r, termos: e.target.value.split("\n") } : r) })} /></label>
                  <Botao variante="fantasma" tamanho="p" onClick={() => editar({ ...dados, ramos: dados.ramos.filter((_, j) => j !== i) })}>Remover ramo {i + 1}</Botao>
                </div>)}
                <div className="flex flex-wrap items-center gap-2">
                  <Botao variante="secundario" tamanho="p" disabled={dados.ramos.length >= MAX_RAMOS || ocupado} onClick={() => editar({ ...dados, ramos: [...dados.ramos, { nome: "", habilitado: true, termos: [] }] })}>Adicionar ramo</Botao>
                  <Botao variante="secundario" tamanho="p" carregando={salvar.isPending} disabled={ocupado || totalTermos > MAX_TERMOS} onClick={() => void salvarVersao()}>Salvar nova versão</Botao>
                  <Botao variante="fantasma" tamanho="p" carregando={exportar.isPending} disabled={vocabularioId === null || ocupado} onClick={() => { if (vocabularioId !== null) exportar.mutate(vocabularioId); }}>Preparar JSON de exportação</Botao>
                  <span className="text-xs text-texto-3">{dados.ramos.length}/{MAX_RAMOS} ramos · {totalTermos}/{MAX_TERMOS} termos · até 200 caracteres por termo</span>
                </div>
                {salvar.isSuccess && <p className="text-xs text-ok-claro">Versão salva no caso, com autor, data e hash. As versões anteriores permanecem disponíveis.</p>}
                {exportar.isError && <Aviso>{exportar.error.message}</Aviso>}
                {exportar.data && <label className="block space-y-1 text-xs text-texto-2">JSON exportado · copie manualmente<textarea aria-label="JSON exportado" readOnly value={exportar.data} className="entrada min-h-24 w-full font-mono text-xs" /></label>}
                <label className="block space-y-1 text-xs text-texto-2">Importar JSON · cole o conteúdo<textarea aria-label="JSON para importar" className="entrada min-h-20 w-full font-mono text-xs" value={json} onChange={(e) => setJson(e.target.value)} maxLength={200 * 1024} /></label>
                <Botao variante="secundario" tamanho="p" disabled={!json.trim() || ocupado} onClick={importar}>Validar e carregar JSON</Botao>
                <p className="text-xs text-texto-3">Importar ou alternar ramos só altera esta edição; salve uma nova versão local para pesquisar. O snapshot referencia os ramos e termos da versão imutável. Escopo: este caso. Perfis institucionais não estão disponíveis.</p>
              </div>
            </details>
          </fieldset>}
        </form>
        {erroLocal && <div className="mt-3"><Aviso>{erroLocal}</Aviso></div>}
      </section>

      {extracoes.isError && <Aviso>{extracoes.error.message}</Aviso>}
      <Aviso tipo="info">Resultados são candidatos para revisão humana. Nenhum achado é selecionado como evidência automaticamente. OCR, modelos de IA, correlações e consultas externas não fazem parte da AI-0.</Aviso>
      {ocupado && <Carregando texto={executar.isPending ? "Executando consulta local…" : "Lendo página da consulta…"} />}
      {erroResultado && <Aviso>{erroResultado} Execute a consulta novamente se o conjunto de dados do caso mudou.</Aviso>}
      {!resultado && !ocupado && !erroResultado && <Vazio titulo="Escolha um mecanismo e pesquise" icone="busca">A execução é explícita; digitar, alternar modos ou abrir o histórico não inicia uma pesquisa.</Vazio>}
      {resultado && execucao && <Resultados caso={caso} resultado={resultado} consulta={execucao.consulta} deslocamento={deslocamento} mudar={(p) => void mudarPagina(p)} />}
      <section className="painel p-4" aria-label="Histórico de descoberta">
        <h2 className="mb-2 text-sm font-semibold text-texto">Consultas salvas · últimas execuções do caso</h2>
        <p className="mb-3 text-xs text-texto-3">O snapshot conserva configuração, termos, versão e universo de IDs; não é uma cópia congelada dos resultados. Reexecutar cria uma nova consulta auditada, sem substituir a anterior.</p>
        {consultas.isPending ? <Carregando texto="Lendo histórico…" /> : consultas.isError ? <Aviso>{consultas.error.message}</Aviso> :
          consultas.data?.length ? <ul className="divide-y divide-borda">{consultas.data.map((c) => <li key={c.id}>
            <details className="py-2">
              <summary className="cursor-pointer text-sm text-texto"><span className="mr-2 font-mono text-xs text-texto-3">#{c.id}</span>{c.pedido.filtro.texto || indicadores.find(([id]) => id === c.pedido.indicador)?.[1] || "Indicadores"}<span className="ml-2 text-xs text-texto-3">{nomes[c.pedido.modo]} · {dataHora(c.criada_em, caso.fuso_horario)}</span></summary>
              <div className="mt-3 space-y-3"><Snapshot caso={caso} consulta={c} /><Botao variante="secundario" tamanho="p" disabled={ocupado || vocabularioOcupado} onClick={() => void iniciar(c.pedido, c.vocabulario_id)}>Reexecutar consulta #{c.id}</Botao></div>
            </details>
          </li>)}</ul> : <p className="text-sm text-texto-3">Nenhuma consulta de descoberta executada neste caso.</p>}
      </section>
    </div>
  );
}

function Snapshot({ caso, consulta: c }: { caso: Caso; consulta: ConsultaDescobertaSalva }) {
  const filtro = c.pedido.filtro;
  return <div className="space-y-2 text-xs text-texto-2">
    <Definicoes colunas={3} itens={[
      ["Consulta original", filtro.texto || "sem restrição textual"],
      ["Mecanismo", nomes[c.pedido.modo]],
      ["Termos efetivos", c.termos_efetivos.join(" · ") || "tipo de indicador"],
      ["Filtros", [filtro.extracao_id ? `Extração: ${filtro.extracao_id}` : "Todas as extrações", filtro.app ? `App: ${rotuloApp(filtro.app)}` : "Todos os apps", filtro.tipo ? `Tipo: ${nomeTipoArtefato[filtro.tipo] ?? filtro.tipo}` : "Todos os tipos"].join(" · ")],
      ["Período (fuso do caso)", `${filtro.de_ms != null ? dataHoraMs(filtro.de_ms, caso.fuso_horario) : "sem início"} → ${filtro.ate_ms != null ? dataHoraMs(filtro.ate_ms, caso.fuso_horario) : "sem fim"}`],
      ["Vocabulário", c.vocabulario_id != null ? `Versão local #${c.vocabulario_id}` : "não utilizado"],
      ["Algoritmo / índice", `${c.algoritmo} · índice v${c.versao_indice}`],
      ["Universo de origem", `Artefatos até ID ${c.ultimo_artefato_id}; arquivos até ID ${c.ultimo_arquivo_id}`],
      ["Execução salva", `${dataHora(c.criada_em, caso.fuso_horario)} · ${numero(c.total_artefatos)} registros · ${numero(c.total_arquivos)} arquivos`],
    ]} />
    <p className="break-all">SHA-256 do snapshot: <Hash valor={c.sha256} /></p>
  </div>;
}

function Resultados({ caso, resultado: r, consulta, deslocamento, mudar }: { caso: Caso; resultado: ResultadoDescoberta; consulta: ConsultaDescobertaSalva; deslocamento: number; mudar: (p: number) => void }) {
  const filtro = consulta.pedido.filtro;
  const arquivosIncluidos = r.modo !== "indicadores" && !filtro.app && !filtro.tipo && filtro.de_ms == null && filtro.ate_ms == null;
  return <section className="space-y-4" aria-label="Resultados da descoberta">
    <Cartao titulo={`Execução #${consulta.id} · ${nomes[r.modo]}`}><Snapshot caso={caso} consulta={consulta} /></Cartao>
    <div className="flex flex-wrap items-center gap-2 text-xs text-texto-2" role="status">
      <Selo tom="info">Candidatos não revisados</Selo><span>{numero(r.artefatos.total)} registro(s) · {numero(r.arquivos.total)} arquivo(s)</span>
      {!arquivosIncluidos && <span className="text-texto-3">Nomes de arquivo fora do escopo desta consulta.</span>}
    </div>
    <Cartao titulo={`Registros (${numero(r.artefatos.total)})`} icone="busca" denso>
      {r.artefatos.total === 0 ? <div className="p-4"><Vazio titulo="Nenhum candidato encontrado" /></div> :
        <ul className="divide-y divide-borda" aria-label="Registros da descoberta">{r.artefatos.itens.map((item) => {
          const a = item.achado; const reg = a.artefato; const destino = destinoRegistro(caso.id, a);
          return <li key={reg.id} className="px-4 py-3">
            <Link to={destino.para} className="group block hover:text-ouro-claro">
              <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-texto-3"><span>{dataHoraMs(reg.data_hora_ms, caso.fuso_horario)}</span><span>{rotuloApp(reg.app)}</span><Selo>{nomeTipoArtefato[reg.tipo] ?? reg.tipo}</Selo><span className="font-mono">{reg.remetente ?? reg.id_externo}</span><span className="ml-auto text-ouro-claro">{destino.acao}</span></div>
              <p className="mt-1 break-words text-sm text-texto">{a.trecho ? <TrechoDestacado partes={a.trecho} /> : reg.conteudo ?? reg.nome_arquivo ?? "Conteúdo textual não informado"}</p>
            </Link>
            <details className="mt-2 text-xs text-texto-2"><summary className="cursor-pointer text-ouro-claro">Por que este resultado apareceu?</summary>
              <div className="mt-2 space-y-1.5"><p>Origem: extração <span className="font-mono">{a.extracao_id}</span> · registro <span className="font-mono">{reg.id}</span></p>
                <ul className="list-inside list-disc">{item.motivos.map((m, i) => <li key={i}>{m}</li>)}</ul>
                {item.indicadores.map((ind, i) => <p key={i} className="break-all"><Selo>{ind.tipo}</Selo> <span className="font-mono text-texto">{ind.valor}</span> · campo: {ind.campo}</p>)}
                <p>Mecanismo: {r.algoritmo}. Abra o registro para contexto e origem técnica. Correspondência não é conclusão forense.</p>
              </div>
            </details>
          </li>;
        })}</ul>}
    </Cartao>
    {arquivosIncluidos && <Cartao titulo={`Nomes de arquivo (${numero(r.arquivos.total)})`} icone="arquivos" denso>
      {r.arquivos.total === 0 ? <div className="p-4"><Vazio titulo="Nenhum nome de arquivo encontrado" /></div> : <div className="overflow-x-auto"><table className={tabela.base} aria-label="Arquivos da descoberta"><tbody className={tabela.corpo}>{r.arquivos.itens.map((f) => <tr key={f.id} className={tabela.linha}>
        <td className={`${tabela.td} break-all font-mono text-xs`}><Link className="text-texto hover:text-ouro-claro" to={`/casos/${caso.id}/extracoes/${f.extracao_id}`}>{f.caminho}</Link><p className="mt-1 font-sans text-texto-3">Correspondência no caminho · extração {f.extracao_id} · inventário #{f.id}</p></td>
        <td className={`${tabela.td} text-xs`}>{nomeCategoria[f.categoria] ?? f.categoria}</td><td className={`${tabela.td} whitespace-nowrap text-xs`}>{tamanho(f.tamanho)}</td><td className={`${tabela.td} text-xs`}><Hash valor={f.sha256} /></td>
      </tr>)}</tbody></table></div>}
    </Cartao>}
    <Paginacao total={Math.max(r.artefatos.total, r.arquivos.total)} deslocamento={deslocamento} limite={LIMITE} mudar={mudar} />
  </section>;
}
