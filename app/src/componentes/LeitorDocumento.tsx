// Leitura documental local. O backend resolve o arquivo no inventário do caso e
// confere seus bytes; a interface recebe apenas texto e proveniência verificados.
// O conteúdo nunca é interpretado como HTML, Markdown, link ou comando.

import { useEffect, useRef, useState } from "react";
import { api, type ArquivoExtracao, type Caso, type DocumentoIndexado, type ResultadoDocumento } from "../api/cliente";
import { dataHora, extensao, nomeArquivo } from "../util/formatar";
import { Botao } from "./Botao";
import { Aviso, Carregando, Hash, Selo } from "./Estrutura";
import { Icone } from "./Icone";
import { PainelAnaliseDocumento } from "./PainelAnaliseDocumento";

const EXTENSOES_TEXTO = new Set(["txt", "csv", "tsv", "json", "xml", "md", "log"]);
const LIMITE_BYTES = 2 * 1024 * 1024;
const LIMITE_PAGINAS = 512;

type Props = { caso: Caso; extracaoId: string; arquivo: ArquivoExtracao };
type Leitura =
  | { estado: "inicial" }
  | { estado: "carregando" }
  | { estado: "resultado"; resultado: ResultadoDocumento }
  | { estado: "erro"; mensagem: string };
type FilaPdf = { estado: "ativa" | "pausando" | "pausada" | "concluida" | "erro"; total: number; concluidas: number; proxima: number };

/** A chave também desmonta a sessão em mudanças de caso/extração/hash/seleção. */
export function LeitorDocumento(props: Props) {
  const { caso, extracaoId, arquivo } = props;
  const chave = JSON.stringify([caso.id, extracaoId, arquivo.id, arquivo.caminho, arquivo.sha256, arquivo.tamanho]);
  return <SessaoLeitura key={chave} {...props} />;
}

function SessaoLeitura({ caso, extracaoId, arquivo }: Props) {
  const [leitura, setLeitura] = useState<Leitura>({ estado: "inicial" });
  const [paginaSelecionada, setPaginaSelecionada] = useState(1);
  const [totalPaginas, setTotalPaginas] = useState<number | null>(null);
  const [indexando, setIndexando] = useState(false);
  const [indexado, setIndexado] = useState<DocumentoIndexado | null>(null);
  const [erroIndexacao, setErroIndexacao] = useState<string | null>(null);
  const [cancelando, setCancelando] = useState(false);
  const [avisoCancelamento, setAvisoCancelamento] = useState<string | null>(null);
  const [fila, setFila] = useState<FilaPdf | null>(null);
  const pausaSolicitada = useRef(false);
  const operacaoAtiva = useRef(false);
  const ativa = useRef(true);
  const tentativa = useRef(0);
  useEffect(() => {
    ativa.current = true;
    return () => {
      ativa.current = false;
      pausaSolicitada.current = true;
      operacaoAtiva.current = false;
      tentativa.current += 1;
    };
  }, []);

  const pdf = extensao(arquivo.caminho) === "pdf";
  const suportado = (pdf || EXTENSOES_TEXTO.has(extensao(arquivo.caminho))) && !arquivo.caminho.includes("!/");
  const disponivel = suportado && !!arquivo.sha256 && arquivo.tamanho <= LIMITE_BYTES;

  async function ler(pagina = paginaSelecionada) {
    if (!disponivel || leitura.estado === "carregando" || indexando) return;
    if (pdf && (!Number.isInteger(pagina) || pagina < 1 || pagina > LIMITE_PAGINAS || (totalPaginas !== null && pagina > totalPaginas))) return;
    const atual = ++tentativa.current;
    setPaginaSelecionada(pagina);
    setIndexado(null); setErroIndexacao(null); setAvisoCancelamento(null);
    setFila(null);
    setLeitura({ estado: "carregando" });
    try {
      const resultado = pdf
        ? await api.lerDocumento(arquivo.id, caso.id, pagina)
        : await api.lerDocumento(arquivo.id, caso.id);
      if (!ativa.current || tentativa.current !== atual) return;
      const texto = resultado.texto;
      // Não publique conteúdo de outra seleção/caso, mesmo se uma resposta IPC
      // chegar depois de uma navegação ou trouxer proveniência incompatível.
      if (resultado.estado === "pronto" && (
        !texto || texto.caso_id !== caso.id || texto.extracao_id !== extracaoId ||
        texto.arquivo_id !== arquivo.id || texto.caminho !== arquivo.caminho || texto.sha256 !== arquivo.sha256
      )) {
        setLeitura({ estado: "erro", mensagem: "A origem do resultado não corresponde ao arquivo selecionado. O conteúdo não foi exibido." });
        return;
      }
      if (resultado.estado === "pronto" && texto) {
        const paginacaoCorreta = pdf
          ? texto.pagina === pagina && Number.isInteger(texto.total_paginas) && texto.total_paginas !== null &&
            texto.total_paginas >= pagina && texto.total_paginas <= LIMITE_PAGINAS &&
            (totalPaginas === null || texto.total_paginas === totalPaginas)
          : texto.pagina === null && texto.total_paginas === null;
        if (!paginacaoCorreta) {
          setLeitura({ estado: "erro", mensagem: "A paginação do resultado não corresponde à leitura solicitada. O conteúdo não foi exibido." });
          return;
        }
        setTotalPaginas(texto.total_paginas);
      }
      setLeitura({ estado: "resultado", resultado });
    } catch (erro) {
      if (!ativa.current || tentativa.current !== atual) return;
      setLeitura({ estado: "erro", mensagem: erro instanceof Error ? erro.message : "Não foi possível ler o documento. Tente novamente." });
    }
  }

  function ocultar() {
    // Apenas fecha a apresentação: não afirma cancelar uma operação do backend.
    tentativa.current += 1;
    setPaginaSelecionada(1);
    setTotalPaginas(null);
    setIndexado(null); setErroIndexacao(null); setIndexando(false); setAvisoCancelamento(null);
    setFila(null);
    setLeitura({ estado: "inicial" });
  }

  const resultado = leitura.estado === "resultado" ? leitura.resultado : null;
  const texto = resultado?.estado === "pronto" ? resultado.texto : null;
  const aberta = leitura.estado !== "inicial";

  async function indexar() {
    if (!texto || indexando) return;
    const atual = ++tentativa.current;
    operacaoAtiva.current = true;
    setFila(null);
    setIndexando(true); setIndexado(null); setErroIndexacao(null); setAvisoCancelamento(null);
    try {
      const documento = pdf
        ? await api.indexarDocumento(arquivo.id, caso.id, texto.pagina!)
        : await api.indexarDocumento(arquivo.id, caso.id);
      if (!ativa.current || tentativa.current !== atual) return;
      if (documento.caso_id !== caso.id || documento.extracao_id !== extracaoId ||
          documento.arquivo_id !== arquivo.id || documento.caminho !== arquivo.caminho ||
          documento.sha256_origem !== arquivo.sha256 || documento.sha256_texto !== texto.sha256_texto ||
          documento.texto !== texto.texto || documento.pagina !== texto.pagina || documento.total_paginas !== texto.total_paginas) {
        setErroIndexacao("O derivado retornado não corresponde à leitura selecionada. O conteúdo não foi exibido.");
        return;
      }
      setIndexado(documento); setAvisoCancelamento(null);
    } catch (erro) {
      if (ativa.current && tentativa.current === atual) setErroIndexacao(erro instanceof Error ? erro.message : "Não foi possível indexar este documento.");
    } finally {
      if (ativa.current && tentativa.current === atual) {
        operacaoAtiva.current = false; setIndexando(false); setCancelando(false);
      }
    }
  }

  async function indexarTodas(proxima = 1, concluidas = 0) {
    if (!pdf || !texto || totalPaginas === null || indexando) return;
    const total = totalPaginas;
    const atual = ++tentativa.current;
    pausaSolicitada.current = false;
    operacaoAtiva.current = true;
    setIndexando(true); setIndexado(null); setErroIndexacao(null); setAvisoCancelamento(null);
    setFila({ estado: "ativa", total, concluidas, proxima });
    let paginaAtual = proxima;
    let confirmadas = concluidas;
    try {
      for (; paginaAtual <= total; paginaAtual += 1) {
        if (!ativa.current || tentativa.current !== atual) return;
        if (pausaSolicitada.current) {
          setFila({ estado: "pausada", total, concluidas: confirmadas, proxima: paginaAtual });
          return;
        }
        const documento = await api.indexarDocumento(arquivo.id, caso.id, paginaAtual);
        if (!ativa.current || tentativa.current !== atual) return;
        if (!Number.isSafeInteger(documento.id) || documento.id <= 0 || documento.caso_id !== caso.id ||
            documento.extracao_id !== extracaoId || documento.arquivo_id !== arquivo.id ||
            documento.caminho !== arquivo.caminho || documento.sha256_origem !== arquivo.sha256 ||
            documento.pagina !== paginaAtual || documento.total_paginas !== total ||
            (paginaAtual === texto.pagina && (documento.sha256_texto !== texto.sha256_texto || documento.texto !== texto.texto))) {
          throw new Error("O derivado retornado não corresponde à página solicitada. A fila foi interrompida.");
        }
        confirmadas += 1;
        setIndexado(documento);
        setFila({ estado: pausaSolicitada.current ? "pausando" : "ativa", total, concluidas: confirmadas, proxima: paginaAtual + 1 });
      }
      setFila({ estado: "concluida", total, concluidas: confirmadas, proxima: total + 1 });
      setAvisoCancelamento(null);
    } catch (erro) {
      if (!ativa.current || tentativa.current !== atual) return;
      setErroIndexacao(erro instanceof Error ? erro.message : "Não foi possível indexar a página. A fila foi interrompida.");
      setFila({ estado: pausaSolicitada.current ? "pausada" : "erro", total, concluidas: confirmadas, proxima: paginaAtual });
    } finally {
      if (ativa.current && tentativa.current === atual) {
        operacaoAtiva.current = false; setIndexando(false); setCancelando(false);
      }
    }
  }

  function pausarFila() {
    pausaSolicitada.current = true;
    setFila((atual) => atual && atual.estado === "ativa" ? { ...atual, estado: "pausando" } : atual);
  }

  async function cancelarIndexacao() {
    if (!indexando || cancelando) return;
    const atual = tentativa.current;
    if (fila) pausarFila();
    setCancelando(true);
    try {
      const solicitado = await api.cancelarDocumento(caso.id);
      if (!ativa.current || tentativa.current !== atual || !operacaoAtiva.current) return;
      setAvisoCancelamento(solicitado
        ? "Cancelamento solicitado. Aguardando o encerramento cooperativo da operação."
        : "Não há operação documental ativa para cancelar. Aguardando o retorno da indexação.");
    } catch (erro) {
      if (ativa.current && tentativa.current === atual && operacaoAtiva.current) setAvisoCancelamento(erro instanceof Error ? erro.message : "Não foi possível solicitar o cancelamento.");
    } finally {
      if (ativa.current && tentativa.current === atual) setCancelando(false);
    }
  }

  return (
    <section aria-label="Leitor documental" className="mt-4 border-t border-borda pt-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="flex items-center gap-2 text-sm font-semibold text-texto">
          <Icone nome="documento" className="size-4 text-ouro" />
          Leitura documental
        </h3>
        <Selo tom="neutro">LOCAL · SOMENTE LEITURA</Selo>
      </div>
      <p className="mt-1 text-xs leading-relaxed text-texto-2">
        {pdf ? "Texto nativo do PDF, por página, com origem conferida pelo SHA-256." : "Texto UTF-8 com origem conferida pelo SHA-256."} Sem alterar a extração original.
      </p>
      {pdf && <p className="mt-1 text-xs leading-relaxed text-texto-3">Texto derivado. A ordem de leitura do PDF pode diferir da composição visual. PDFs sem texto nativo ou com recursos não suportados podem ser recusados; OCR não está disponível.</p>}
      {suportado ? (
        <>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <Botao icone="documento" tamanho="p" onClick={() => void ler()} disabled={!disponivel || indexando} carregando={leitura.estado === "carregando"}>
              {texto ? "Conferir e reler" : "Ler documento"}
            </Botao>
            {aberta && <Botao variante="fantasma" tamanho="p" onClick={ocultar} disabled={indexando}>Ocultar leitura</Botao>}
            <span className="text-2xs text-texto-3">Até 2 MiB{pdf ? " · até 512 páginas" : ""} · sem OCR</span>
          </div>
          {!arquivo.sha256 && <p className="mt-2 text-xs text-atencao-claro">Leitura indisponível: o inventário não possui SHA-256 deste arquivo.</p>}
          {arquivo.tamanho > LIMITE_BYTES && <p className="mt-2 text-xs text-atencao-claro">Este arquivo excede o limite de leitura documental de 2 MiB.</p>}
        </>
      ) : (
        <p className="mt-3 text-xs leading-relaxed text-texto-3">
          Este formato ainda não possui leitor documental. Imagens/OCR, Office e arquivos em contêineres não são processados nesta versão.
        </p>
      )}

      {pdf && totalPaginas !== null && (
        <nav aria-label="Páginas do documento" className="mt-3 flex flex-wrap items-center justify-between gap-2 border-y border-borda py-2">
          <span aria-live="polite" className="text-xs text-texto-2">{leitura.estado === "carregando" || !texto ? "Página solicitada" : "Página"} <span className="font-mono text-texto">{paginaSelecionada} de {totalPaginas}</span></span>
          <div className="flex items-center gap-1">
            <Botao variante="secundario" tamanho="p" onClick={() => void ler(paginaSelecionada - 1)} disabled={paginaSelecionada <= 1 || leitura.estado === "carregando" || indexando}>Anterior</Botao>
            <Botao variante="secundario" tamanho="p" onClick={() => void ler(paginaSelecionada + 1)} disabled={paginaSelecionada >= totalPaginas || leitura.estado === "carregando" || indexando}>Próxima</Botao>
          </div>
        </nav>
      )}

      {leitura.estado === "carregando" && <Carregando texto="Lendo e conferindo a origem do documento…" />}
      {leitura.estado === "erro" && <div className="mt-3"><Aviso>{leitura.mensagem}</Aviso></div>}
      {resultado && resultado.estado !== "pronto" && (
        <div className="mt-3">
          <Aviso tipo={resultado.estado === "origem_divergente" || resultado.estado === "falhou" ? "erro" : "atencao"}>
            <p className="font-medium">Documento não exibido</p>
            <p className="mt-1">{resultado.motivo ?? "O backend recusou a leitura deste documento."}</p>
            <p className="mt-1 font-mono text-xs">Estado: {resultado.estado}</p>
          </Aviso>
        </div>
      )}
      {texto && (
        <div className="mt-4 overflow-hidden rounded-md border border-borda-2 bg-fundo">
          <div className="flex flex-wrap items-center gap-2 border-b border-borda bg-sup-2/50 px-3 py-2">
            <Icone nome="check" className="size-3.5 text-ok" />
            <span className="min-w-0 flex-1 truncate text-xs font-medium text-texto" title={texto.caminho}>{nomeArquivo(texto.caminho)}</span>
            <Selo tom="neutro">TEXTO DERIVADO</Selo>
            <Selo tom="ok">ORIGEM CONFERIDA</Selo>
          </div>
          {texto.texto === "" ? (
            <p className="px-3 py-8 text-sm text-texto-2" role="status">{pdf ? "Página sem texto nativo; OCR não disponível." : "Arquivo vazio. A origem foi conferida; não há texto para exibir."}</p>
          ) : (
            <pre
              role="region"
              aria-label="Texto documental"
              tabIndex={0}
              className="max-h-[32rem] min-h-48 overflow-auto p-3 font-mono text-sm leading-relaxed text-texto"
            >{texto.texto}</pre>
          )}
          <details className="border-t border-borda px-3 py-2 text-xs">
            <summary className="cursor-pointer text-texto-2">Proveniência do texto · derivado local</summary>
            <dl aria-label="Proveniência documental" className="mt-3 space-y-2 text-texto-2">
              <div><dt className="sobrelinha">Origem</dt><dd className="mt-1 break-all font-mono">{texto.caminho}</dd></div>
              <div><dt className="sobrelinha">Caso / extração / arquivo</dt><dd className="mt-1 break-all font-mono">{texto.caso_id} / {texto.extracao_id} / {texto.arquivo_id}</dd></div>
              <div><dt className="sobrelinha">SHA-256 da origem conferida</dt><dd className="mt-1"><Hash valor={texto.sha256} /></dd></div>
              <div><dt className="sobrelinha">SHA-256 do texto{pdf ? " da página" : ""}</dt><dd className="mt-1"><Hash valor={texto.sha256_texto} /></dd></div>
              <div><dt className="sobrelinha">Motor</dt><dd className="mt-1 font-mono">{texto.motor_id} v{texto.motor_versao}</dd></div>
              <div><dt className="sobrelinha">Leitura realizada em</dt><dd className="mt-1">{dataHora(texto.executado_em, caso.fuso_horario)}</dd></div>
              <div><dt className="sobrelinha">Página</dt><dd className="mt-1">{texto.pagina === null ? "Não se aplica ao texto nativo" : `${texto.pagina} de ${texto.total_paginas}`}</dd></div>
            </dl>
            <p className="mt-3 leading-relaxed text-texto-3">Conteúdo apresentado como texto, sem executar instruções. Esta leitura não cria evidência nem altera relatórios.</p>
          </details>
          <div className="border-t border-borda px-3 py-3">
            <div className="flex flex-wrap items-center gap-2">
              <Botao tamanho="p" variante="secundario" onClick={() => void indexar()} carregando={indexando}>{pdf ? "Indexar esta página" : "Indexar documento"}</Botao>
              {pdf && totalPaginas !== null && totalPaginas > 1 && <Botao tamanho="p" variante="secundario" disabled={indexando} onClick={() => void indexarTodas()}>Indexar todas as páginas</Botao>}
              {fila && (fila.estado === "pausada" || fila.estado === "erro") && <Botao tamanho="p" variante="secundario" disabled={indexando} onClick={() => void indexarTodas(fila.proxima, fila.concluidas)}>Continuar fila</Botao>}
              {fila?.estado === "ativa" && <Botao tamanho="p" variante="fantasma" onClick={pausarFila}>Pausar após esta página</Botao>}
              {indexando && <Botao tamanho="p" variante="fantasma" onClick={() => void cancelarIndexacao()} carregando={cancelando}>Solicitar cancelamento</Botao>}
            </div>
            <p className="mt-1 text-xs leading-relaxed text-texto-3">Ação explícita: o backend relê e confere a origem antes de armazenar texto e candidatos para busca/revisão. Não cria evidência nem altera relatórios.</p>
            {avisoCancelamento && <p role="status" className="mt-2 text-xs text-texto-2">{avisoCancelamento}</p>}
            {fila && <div className="mt-2 text-xs text-texto-2">
              <p role="status">{fila.concluidas} de {fila.total} página(s) confirmada(s) no índice · {fila.estado === "concluida" ? "Fila concluída" : fila.estado === "pausada" ? "Fila pausada" : fila.estado === "pausando" ? "Pausa solicitada; a página atual pode concluir" : fila.estado === "erro" ? `Interrompida na página ${fila.proxima}` : `Indexando página ${fila.proxima}`}</p>
              <p className="mt-1 text-texto-3">Fila temporária desta tela. Ao sair, não são iniciadas novas páginas; resultados já armazenados permanecem no caso. Recomeçar pode reaproveitar snapshots existentes sem descartar revisões humanas.</p>
            </div>}
            {erroIndexacao && <div className="mt-2"><Aviso>{erroIndexacao}</Aviso></div>}
            {indexado && !indexando && <PainelAnaliseDocumento caso={caso} documento={indexado} exibirTexto={indexado.pagina !== texto.pagina} />}
          </div>
        </div>
      )}
    </section>
  );
}
