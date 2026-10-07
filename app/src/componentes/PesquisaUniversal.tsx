import { useEffect, useRef, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { api, FalhaSied, type Caso, type DocumentoIndexado, type FiltroBusca, type ItemUniversal, type MecanismoUniversal, type PedidoUniversal, type ResultadoUniversal } from "../api/cliente";
import { useExtracoes } from "../api/consultas";
import { dataHora, dataHoraMs, limiteDia } from "../util/formatar";
import { nomeApp, nomeTipoArtefato } from "../util/rotulos";
import { Botao } from "./Botao";
import { Aviso, Carregando, Hash, Paginacao, Selo, Vazio } from "./Estrutura";
import { PainelAnaliseDocumento } from "./PainelAnaliseDocumento";
import { ContextoRelacional } from "./ContextoRelacional";

const LIMITE = 20;
const FONTES: [MecanismoUniversal, string][] = [["registros", "Registros / mensagens"], ["arquivos", "Nomes de arquivo"], ["documentos", "Textos documentais indexados"], ["entidades", "Candidatos literais documentais"]];
const ROTULOS: Record<MecanismoUniversal, string> = { registros: "Registros", arquivos: "Nomes de arquivo", documentos: "Textos documentais", entidades: "Candidatos literais" };

/** Gramática explícita e pequena, não um modelo ou interpretação de intenção. */
export function interpretarConsultaLocal(frase: string): { termo: string; mecanismos: MecanismoUniversal[] } {
  const resultado = /^\s*(?:buscar\s+)?(mensagens|registros|arquivos|documentos|entidades)\s+(?:contendo|com)\s+"([^"\r\n]{1,256})"\s*$/iu.exec(frase);
  if (!resultado || !resultado[2]?.trim()) {
    throw new Error('Formato não reconhecido. Use, por exemplo: documentos contendo "termo". Datas, pessoas e significado não são interpretados.');
  }
  const fonte = resultado[1]!.toLocaleLowerCase("pt-BR");
  return { termo: resultado[2]!, mecanismos: [fonte === "mensagens" ? "registros" : fonte as MecanismoUniversal] };
}

function erroTexto(erro: unknown) {
  return erro instanceof Error ? erro.message : "A consulta integrada não pôde ser concluída.";
}
function dataInicial(ms: number | undefined, fuso: string) {
  if (ms === undefined) return "";
  const partes = new Intl.DateTimeFormat("en-CA", { timeZone: fuso, year: "numeric", month: "2-digit", day: "2-digit" }).formatToParts(new Date(ms));
  return ["year", "month", "day"].map((tipo) => partes.find((parte) => parte.type === tipo)?.value ?? "").join("-");
}
function destinoItem(casoId: string, item: ItemUniversal) {
  if (item.artefato_id !== null) {
    if (item.conversa_id !== null) return `/casos/${casoId}/conversas/${item.conversa_id}?registro=${item.artefato_id}`;
    if (item.tipo === "contato" || item.tipo === "chamada") return `/casos/${casoId}/contatos?aba=${item.tipo === "chamada" ? "chamadas" : "agenda"}&registro=${item.artefato_id}`;
    return `/casos/${casoId}/extracoes/${item.extracao_id}`;
  }
  return `/casos/${casoId}/arquivos?extracao=${encodeURIComponent(item.extracao_id)}${item.arquivo_id === null ? "" : `&arquivo=${item.arquivo_id}`}`;
}

export function PesquisaUniversal({ caso, filtro }: { caso: Caso; filtro?: FiltroBusca }) {
  return <SessaoPesquisa key={caso.id} caso={caso} filtroInicial={filtro} />;
}

function SessaoPesquisa({ caso, filtroInicial }: { caso: Caso; filtroInicial?: FiltroBusca }) {
  const extracoes = useExtracoes(caso.id);
  const [texto, setTexto] = useState(filtroInicial?.texto ?? "");
  const [mecanismos, setMecanismos] = useState<MecanismoUniversal[]>(FONTES.map(([fonte]) => fonte));
  const [extracao, setExtracao] = useState(filtroInicial?.extracao_id ?? "");
  const [app, setApp] = useState(filtroInicial?.app ?? "");
  const [tipo, setTipo] = useState(filtroInicial?.tipo ?? "");
  const [de, setDe] = useState(dataInicial(filtroInicial?.de_ms, caso.fuso_horario));
  const [ate, setAte] = useState(dataInicial(filtroInicial?.ate_ms, caso.fuso_horario));
  const [deEditado, setDeEditado] = useState(false); const [ateEditado, setAteEditado] = useState(false);
  const [pedido, setPedido] = useState<PedidoUniversal | null>(null);
  const [resultado, setResultado] = useState<ResultadoUniversal | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const [erro, setErro] = useState<string | null>(null);
  const [operacao, setOperacao] = useState<string | null>(null);
  const [avisoCancelamento, setAvisoCancelamento] = useState<string | null>(null);
  const [cancelamentoSolicitado, setCancelamentoSolicitado] = useState(false);
  const [filtroSinal, setFiltroSinal] = useState<MecanismoUniversal | "todos">("todos");
  const [frase, setFrase] = useState(""); const [preparada, setPreparada] = useState<string | null>(null);
  const [documento, setDocumento] = useState<DocumentoIndexado | null>(null);
  const [abrindo, setAbrindo] = useState(false); const [erroDocumento, setErroDocumento] = useState<string | null>(null);
  const emCurso = useRef<string | null>(null); const montado = useRef(true); const abertura = useRef(0);
  useEffect(() => {
    montado.current = true;
    return () => {
      montado.current = false; abertura.current += 1;
      const antiga = emCurso.current; emCurso.current = null;
      if (antiga) void api.cancelarDescoberta(antiga).catch(() => undefined);
    };
  }, []);

  async function executar(novo: PedidoUniversal, proximo = 0) {
    if (emCurso.current) return;
    const id = crypto.randomUUID(); emCurso.current = id;
    setOperacao(id); setCancelamentoSolicitado(false); setAvisoCancelamento(null); setErro(null);
    setDocumento(null); setErroDocumento(null); setAbrindo(false); abertura.current += 1;
    // Não apresentar uma página antiga como resposta de uma nova configuração.
    if (proximo === 0) setResultado(null);
    try {
      const resposta = await api.pesquisarUniversal(caso.id, id, novo, proximo, LIMITE);
      if (!montado.current || emCurso.current !== id) return;
      const recebida = resposta.pedido.filtro;
      const esperado = novo.filtro;
      if (resposta.caso_id !== caso.id || recebida.texto !== esperado.texto ||
          (recebida.extracao_id ?? null) !== (esperado.extracao_id ?? null) ||
          (recebida.app ?? null) !== (esperado.app ?? null) || (recebida.tipo ?? null) !== (esperado.tipo ?? null) ||
          (recebida.de_ms ?? null) !== (esperado.de_ms ?? null) || (recebida.ate_ms ?? null) !== (esperado.ate_ms ?? null) ||
          resposta.pedido.mecanismos.length !== novo.mecanismos.length ||
          resposta.pedido.mecanismos.some((fonte) => !novo.mecanismos.includes(fonte)) ||
          !Number.isSafeInteger(resposta.recuperados_na_janela) || resposta.recuperados_na_janela < 0 ||
          resposta.recuperados_na_janela > 400 || resposta.itens.length > LIMITE ||
          resposta.itens.some((item) => !Number.isFinite(item.score_ranking) || item.score_ranking < 0 || item.score_ranking > 1 ||
            item.sinais.some((sinal) => !novo.mecanismos.includes(sinal.mecanismo) || !Number.isFinite(sinal.contribuicao_rrf) || sinal.contribuicao_rrf <= 0)) ||
          new Set(resposta.itens.map((item) => item.chave)).size !== resposta.itens.length) {
        throw new Error("O resultado integrado não corresponde ao caso, filtros ou janela solicitados.");
      }
      setPedido(novo); setResultado(resposta); setDeslocamento(proximo); setFiltroSinal("todos");
    } catch (falha) {
      if (!montado.current || emCurso.current !== id) return;
      const codigo: string | null = falha instanceof FalhaSied ? falha.erro.codigo : null;
      setErro(codigo === "descoberta_cancelada" ? "Consulta integrada cancelada pelo backend. Nenhum resultado parcial novo foi apresentado."
        : codigo === "descoberta_limite" ? "A consulta atingiu o limite cooperativo. Restrinja as fontes/filtros e execute novamente."
        : erroTexto(falha));
    } finally {
      if (montado.current && emCurso.current === id) {
        emCurso.current = null; setOperacao(null); setCancelamentoSolicitado(false); setAvisoCancelamento(null);
      }
    }
  }
  function buscar(evento: FormEvent) {
    evento.preventDefault();
    if (!texto.trim() || mecanismos.length === 0 || new TextEncoder().encode(texto.trim()).length > 1024) {
      setErro("Informe um termo e selecione pelo menos um mecanismo disponível. O termo admite até 1.024 bytes UTF-8."); return;
    }
    if (de && ate && de > ate) { setErro("A data inicial deve ser anterior ou igual à final."); return; }
    const filtro: FiltroBusca = { texto: texto.trim() };
    if (extracao) filtro.extracao_id = extracao;
    if (app) filtro.app = app;
    if (tipo) filtro.tipo = tipo;
    if (!deEditado && filtroInicial?.de_ms !== undefined) filtro.de_ms = filtroInicial.de_ms;
    else if (de) { const ms = limiteDia(de, caso.fuso_horario); if (ms !== null) filtro.de_ms = ms; }
    if (!ateEditado && filtroInicial?.ate_ms !== undefined) filtro.ate_ms = filtroInicial.ate_ms;
    else if (ate) { const ms = limiteDia(ate, caso.fuso_horario, true); if (ms !== null) filtro.ate_ms = ms; }
    void executar({ filtro, mecanismos });
  }
  async function cancelar() {
    const id = emCurso.current;
    if (!id || cancelamentoSolicitado) return;
    setCancelamentoSolicitado(true);
    try {
      const aceito = await api.cancelarDescoberta(id);
      if (!montado.current || emCurso.current !== id) return;
      setAvisoCancelamento(aceito ? "Sinal de cancelamento solicitado. Aguardando a resposta final da consulta."
        : "Sinal não aceito: a consulta pode estar terminando ou ainda não estar registrada. Aguardando retorno.");
      if (!aceito) setCancelamentoSolicitado(false);
    } catch (falha) {
      if (montado.current && emCurso.current === id) { setAvisoCancelamento(erroTexto(falha)); setCancelamentoSolicitado(false); }
    }
  }
  function preparar() {
    try {
      const sugestao = interpretarConsultaLocal(frase);
      setTexto(sugestao.termo); setMecanismos(sugestao.mecanismos); setErro(null);
      setPreparada(`Termo: ${sugestao.termo} · Fonte: ${ROTULOS[sugestao.mecanismos[0]!]}. Os demais filtros do formulário continuam aplicados. Confira e execute explicitamente.`);
    } catch (falha) { setErro(erroTexto(falha)); setPreparada(null); }
  }
  async function abrirDocumento(item: ItemUniversal, documentoId: number, hash: string | null, pagina: number | null) {
    const atual = ++abertura.current;
    setDocumento(null); setErroDocumento(null); setAbrindo(true);
    try {
      const recebido = await api.obterDocumento(caso.id, documentoId);
      if (!montado.current || abertura.current !== atual) return;
      if (recebido.caso_id !== caso.id || recebido.id !== documentoId || recebido.arquivo_id !== item.arquivo_id ||
          recebido.extracao_id !== item.extracao_id || recebido.pagina !== pagina || recebido.sha256_texto !== hash ||
          recebido.sha256_origem !== item.hash_origem) {
        throw new Error("O derivado recebido não corresponde ao resultado integrado selecionado.");
      }
      setDocumento(recebido);
    } catch (falha) { if (montado.current && abertura.current === atual) setErroDocumento(erroTexto(falha)); }
    finally { if (montado.current && abertura.current === atual) setAbrindo(false); }
  }
  const ocupado = operacao !== null;
  const visiveis = resultado?.itens.filter((item) => filtroSinal === "todos" || item.sinais.some((sinal) => sinal.mecanismo === filtroSinal)) ?? [];

  return (
    <section aria-label="Pesquisa integrada local" className="space-y-4">
      <div className="painel p-4">
        <div className="flex flex-wrap items-center justify-between gap-2"><h2 className="text-sm font-semibold text-texto">Recuperação integrada · índices locais reais</h2><Selo tom="info">LOCAL / OFFLINE</Selo></div>
        <p className="mt-1 text-xs leading-relaxed text-texto-2">Combina posições da busca de registros, nomes de arquivo, textos indexados e candidatos literais. O ranking não expressa confiança, relevância pericial ou veracidade. Não utiliza modelos de IA.</p>
        <form role="search" aria-label="Consulta integrada" onSubmit={buscar} className="mt-3 space-y-3">
          <div className="flex gap-2"><input aria-label="Termo da busca integrada" type="search" className="entrada min-w-0 flex-1" value={texto} maxLength={256} disabled={ocupado} onChange={(evento) => setTexto(evento.target.value)} /><Botao type="submit" carregando={ocupado}>Pesquisar índices locais</Botao></div>
          <fieldset disabled={ocupado} className="space-y-2"><legend className="sobrelinha mb-2">Mecanismos efetivamente disponíveis</legend>
            <div className="flex flex-wrap gap-x-5 gap-y-2">{FONTES.map(([fonte, rotulo]) => <label key={fonte} className="flex items-center gap-2 text-xs text-texto-2"><input type="checkbox" checked={mecanismos.includes(fonte)} onChange={(evento) => setMecanismos((atuais) => evento.target.checked ? [...atuais, fonte] : atuais.filter((atual) => atual !== fonte))} />{rotulo}</label>)}</div>
            <div className="flex flex-wrap gap-x-5 gap-y-2 text-xs text-texto-3">{["OCR", "Transcrição", "Busca semântica", "Classificação de imagens"].map((fonte) => <label key={fonte} className="flex items-center gap-2"><input type="checkbox" checked={false} disabled readOnly />{fonte} · motor indisponível</label>)}</div>
          </fieldset>
          <fieldset disabled={ocupado} className="flex flex-wrap items-center gap-3 text-xs text-texto-2"><legend className="sr-only">Filtros da busca integrada</legend>
            <select aria-label="Extração integrada" className="entrada" value={extracao} onChange={(evento) => setExtracao(evento.target.value)}><option value="">Todas as extrações</option>{extracoes.data?.map((x) => <option key={x.id} value={x.id}>{x.importador_nome} · {x.iniciada_em.slice(0, 10)}</option>)}</select>
            <select aria-label="App integrado" className="entrada" value={app} onChange={(evento) => setApp(evento.target.value)}><option value="">Todos os apps</option>{Object.entries(nomeApp).map(([id, rotulo]) => <option key={id} value={id}>{rotulo}</option>)}</select>
            <select aria-label="Tipo de registro integrado" className="entrada" value={tipo} onChange={(evento) => setTipo(evento.target.value)}><option value="">Todos os tipos</option>{Object.entries(nomeTipoArtefato).map(([id, rotulo]) => <option key={id} value={id}>{rotulo}</option>)}</select>
            <label className="flex items-center gap-2">De<input aria-label="Data inicial integrada" type="date" className="entrada" value={de} onChange={(evento) => { setDeEditado(true); setDe(evento.target.value); }} /></label>
            <label className="flex items-center gap-2">Até<input aria-label="Data final integrada" type="date" className="entrada" value={ate} onChange={(evento) => { setAteEditado(true); setAte(evento.target.value); }} /></label>
          </fieldset>
          <p className="text-xs text-texto-3">Datas no fuso do caso ({caso.fuso_horario}); datas de arquivos vêm do inventário, não do conteúdo. App/tipo de registro podem excluir documentos e entidades sem atributo correspondente.</p>
          {!deEditado && filtroInicial?.de_ms !== undefined && <p className="text-xs text-texto-3">Início exato preservado: {dataHoraMs(filtroInicial.de_ms, caso.fuso_horario)}. Editar o campo de data passa a usar o início do dia.</p>}
          {!ateEditado && filtroInicial?.ate_ms !== undefined && <p className="text-xs text-texto-3">Fim exato preservado: {dataHoraMs(filtroInicial.ate_ms, caso.fuso_horario)}. Editar o campo de data passa a usar o fim do dia.</p>}
        </form>
        <details className="mt-3 border-t border-borda pt-3"><summary className="cursor-pointer text-xs text-ouro-claro">Preparar consulta por regra local · sem modelo</summary>
          <p className="mt-2 text-xs text-texto-2">Formato: documentos contendo "termo". Também aceita mensagens, registros, arquivos ou entidades. Somente a fonte e a expressão entre aspas são reconhecidas; não interpreta significado, pessoas ou datas.</p>
          <div className="mt-2 flex gap-2"><input aria-label="Consulta por regra local" className="entrada min-w-0 flex-1" value={frase} maxLength={512} disabled={ocupado} onChange={(evento) => setFrase(evento.target.value)} /><Botao tamanho="p" variante="secundario" disabled={ocupado} onClick={preparar}>Preparar consulta</Botao></div>
          {preparada && <p role="status" className="mt-2 text-xs text-texto-2">{preparada}</p>}
        </details>
      </div>
      {extracoes.isError && <Aviso>{extracoes.error.message}</Aviso>}
      {ocupado && <div className="flex flex-wrap items-center gap-2"><Carregando texto="Consultando e combinando índices locais…" /><Botao tamanho="p" variante="secundario" disabled={cancelamentoSolicitado} onClick={() => void cancelar()}>{cancelamentoSolicitado ? "Cancelamento solicitado" : "Cancelar consulta integrada"}</Botao></div>}
      {avisoCancelamento && <Aviso tipo="info">{avisoCancelamento}</Aviso>}
      {erro && <Aviso>{erro}</Aviso>}
      {!resultado && !ocupado && !erro && <Vazio titulo="Selecione as fontes e pesquise">Nenhum mecanismo é executado ao digitar, alterar filtros ou preparar uma consulta.</Vazio>}
      {resultado && <div className="painel p-4">
        <div className="flex flex-wrap items-center justify-between gap-2"><h3 className="text-sm font-semibold text-texto">Resultados da janela recuperada</h3><Selo tom={resultado.janela_limitada ? "atencao" : "neutro"}>{resultado.janela_limitada ? "JANELA LIMITADA" : "JANELA RECUPERADA"}</Selo></div>
        <p className="mt-1 text-xs text-texto-2">{resultado.recuperados_na_janela} item(s) na união recuperada · até 100 candidatos por mecanismo. Não é uma contagem global do caso.</p>
        <p className="mt-1 font-mono text-2xs text-texto-3">{resultado.algoritmo} · {dataHora(resultado.executada_em, caso.fuso_horario)}</p>
        {resultado.avisos.length > 0 && <ul className="mt-2 list-inside list-disc space-y-1 text-xs text-texto-2" aria-label="Limitações da recuperação">{resultado.avisos.map((aviso, indice) => <li key={indice}>{aviso}</li>)}</ul>}
        <label className="mt-3 flex flex-wrap items-center gap-2 text-xs text-texto-2">Filtrar sinais nesta página<select aria-label="Mecanismo dos resultados desta página" className="entrada" value={filtroSinal} onChange={(evento) => setFiltroSinal(evento.target.value as MecanismoUniversal | "todos")}><option value="todos">Todos os mecanismos</option>{FONTES.map(([fonte, rotulo]) => <option value={fonte} key={fonte}>{rotulo}</option>)}</select><span>{visiveis.length} item(s) exibido(s) nesta página</span></label>
        {visiveis.length === 0 ? <div className="mt-3"><Vazio titulo="Nenhum item nesta página">A ausência de resultados não comprova ausência de informação na extração. Textos e entidades exigem indexação explícita.</Vazio></div> : (
          <ul aria-label="Resultados integrados" className="mt-3 divide-y divide-borda">{visiveis.map((item) => <li key={item.chave} className="py-3">
            <div className="flex flex-wrap items-start justify-between gap-2"><Link className="min-w-0 flex-1 break-all text-sm font-semibold text-texto hover:text-ouro-claro" to={destinoItem(caso.id, item)}>{item.titulo}</Link><span className="font-mono text-2xs text-texto-3">Ranking {item.score_ranking.toFixed(4)} · não é confiança</span></div>
            <p className="mt-1 whitespace-pre-wrap break-words text-sm text-texto-2">{item.trecho}</p>
            <p className="mt-1 font-mono text-2xs text-texto-3">Extração {item.extracao_id} · {item.data_hora_ms === null ? "Data não informada" : dataHoraMs(item.data_hora_ms, caso.fuso_horario)}</p>
            <details className="mt-2 text-xs text-texto-2"><summary className="cursor-pointer text-ouro-claro">Por que este item apareceu?</summary>
              <p className="mt-2">{item.artefato_id === null ? "SHA-256 registrado da origem" : "Hash histórico registrado do artefato"}: <Hash valor={item.hash_origem} /></p>
              <ol className="mt-2 space-y-2">{item.sinais.map((sinal, indice) => <li key={`${sinal.mecanismo}:${indice}`} className="border-l border-borda pl-3">
                <p><Selo tom="info">{ROTULOS[sinal.mecanismo]}</Selo> · posição {sinal.posicao} · contribuição RRF {sinal.contribuicao_rrf.toFixed(6)}</p>
                <p className="mt-1 whitespace-pre-wrap break-words">{sinal.motivo}</p><p className="mt-1 font-mono text-2xs text-texto-3">Motor: {sinal.motor}{sinal.pagina === null ? "" : ` · página ${sinal.pagina}`}</p>
                {sinal.hash_derivado && <p className="mt-1">Hash do derivado: <Hash valor={sinal.hash_derivado} /></p>}
                {sinal.documento_id !== null && <Botao tamanho="p" variante="fantasma" disabled={ocupado} onClick={() => void abrirDocumento(item, sinal.documento_id!, sinal.hash_derivado, sinal.pagina)}>Abrir derivado #{sinal.documento_id}{sinal.pagina === null ? "" : ` · página ${sinal.pagina}`}</Botao>}
              </li>)}</ol>
              <p className="mt-2 text-texto-3">RRF combina posições com k=60 e pesos iguais. Correspondência, ranking e confirmação literal não constituem conclusão forense. Derivados armazenados não conferem a origem atual.</p>
            </details>
            {item.artefato_id !== null && <details className="mt-2"><summary className="cursor-pointer text-xs text-ouro-claro">Explorar timeline e relações deste registro</summary><ContextoRelacional caso={caso} artefatoId={item.artefato_id} /></details>}
          </li>)}</ul>
        )}
        {!ocupado && pedido && <Paginacao total={resultado.recuperados_na_janela} deslocamento={deslocamento} limite={LIMITE} mudar={(proximo) => void executar(pedido, proximo)} />}
      </div>}
      {abrindo && <Carregando texto="Abrindo derivado da recuperação integrada…" />}
      {erroDocumento && <Aviso>{erroDocumento}</Aviso>}
      {documento && <><div className="flex justify-end"><Botao tamanho="p" variante="fantasma" onClick={() => { abertura.current += 1; setDocumento(null); }}>Fechar derivado integrado</Botao></div><PainelAnaliseDocumento caso={caso} documento={documento} /></>}
    </section>
  );
}
