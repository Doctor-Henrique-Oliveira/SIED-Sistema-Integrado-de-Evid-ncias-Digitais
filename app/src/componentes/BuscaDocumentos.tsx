import { useEffect, useRef, useState, type FormEvent } from "react";
import { api, type AchadoDocumento, type Caso, type DocumentoIndexado, type Pagina } from "../api/cliente";
import { dataHora, nomeArquivo } from "../util/formatar";
import { Botao } from "./Botao";
import { Aviso, Carregando, HashCurto, Paginacao, Selo, Vazio } from "./Estrutura";
import { PainelAnaliseDocumento } from "./PainelAnaliseDocumento";

const LIMITE = 20;
const MAXIMO_TERMO = 256;
type FaixaUtf8 = { inicio: number; fim: number };

function mensagem(erro: unknown) {
  return erro instanceof Error ? erro.message : "Não foi possível consultar os documentos indexados.";
}

function faixaLiteralUtf8(conteudo: string, termo: string): FaixaUtf8 | null {
  const procurado = termo.trim();
  if (!procurado) return null;
  let indice = conteudo.indexOf(procurado);
  if (indice < 0) {
    const normalizado = procurado.toLocaleLowerCase("pt-BR");
    for (let i = 0; i <= conteudo.length - procurado.length; i += 1) {
      if (conteudo.slice(i, i + procurado.length).toLocaleLowerCase("pt-BR") === normalizado) {
        indice = i; break;
      }
    }
  }
  if (indice < 0) return null;
  const codificador = new TextEncoder();
  const inicio = codificador.encode(conteudo.slice(0, indice)).length;
  const fim = inicio + codificador.encode(conteudo.slice(indice, indice + procurado.length)).length;
  return { inicio, fim };
}

export function BuscaDocumentos({ caso, termo = "" }: { caso: Caso; termo?: string }) {
  return <SessaoBusca key={caso.id} caso={caso} termo={termo} />;
}

function SessaoBusca({ caso, termo }: { caso: Caso; termo: string }) {
  const [texto, setTexto] = useState(termo);
  const [pedido, setPedido] = useState<{ termo: string; deslocamento: number } | null>(null);
  const [resultado, setResultado] = useState<Pagina<AchadoDocumento> | null>(null);
  const [buscando, setBuscando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const [documento, setDocumento] = useState<DocumentoIndexado | null>(null);
  const [destaque, setDestaque] = useState<FaixaUtf8 | null>(null);
  const [abrindo, setAbrindo] = useState(false);
  const [erroAbertura, setErroAbertura] = useState<string | null>(null);
  const ativa = useRef(true);
  const abertura = useRef(0);
  useEffect(() => {
    ativa.current = true;
    return () => { ativa.current = false; abertura.current += 1; };
  }, []);
  useEffect(() => {
    setTexto(termo);
    setPedido(termo.trim() ? { termo: termo.trim(), deslocamento: 0 } : null);
    setDocumento(null); setDestaque(null); setErroAbertura(null); setAbrindo(false); abertura.current += 1;
  }, [termo]);

  useEffect(() => {
    let atual = true;
    setResultado(null); setErro(null);
    if (!pedido) { setBuscando(false); return; }
    if (pedido.termo.length > MAXIMO_TERMO) {
      setErro("A busca documental admite até 256 caracteres."); setBuscando(false); return;
    }
    setBuscando(true);
    void api.buscarDocumentos(caso.id, pedido.termo, pedido.deslocamento, LIMITE).then((pagina) => {
      if (!atual || !ativa.current) return;
      if (!Number.isSafeInteger(pagina.total) || pagina.total < 0 || pagina.itens.length > LIMITE ||
          pagina.itens.some((achado) => achado.caso_id !== caso.id || !Number.isSafeInteger(achado.id) || achado.id <= 0)) {
        setErro("Os resultados documentais não correspondem ao caso ou à paginação solicitada. Nenhum conteúdo foi exibido.");
        return;
      }
      setResultado(pagina);
    }).catch((falha) => {
      if (atual && ativa.current) setErro(mensagem(falha));
    }).finally(() => {
      if (atual && ativa.current) setBuscando(false);
    });
    return () => { atual = false; };
  }, [caso.id, pedido]);

  function buscar(evento: FormEvent) {
    evento.preventDefault();
    const normalizado = texto.trim();
    setPedido(normalizado ? { termo: normalizado, deslocamento: 0 } : null);
    setDocumento(null); setDestaque(null); setErroAbertura(null); setAbrindo(false); abertura.current += 1;
  }

  async function abrir(achado: AchadoDocumento) {
    const atual = ++abertura.current;
    setDocumento(null); setDestaque(null); setErroAbertura(null); setAbrindo(true);
    try {
      const completo = await api.obterDocumento(caso.id, achado.id);
      if (!ativa.current || abertura.current !== atual) return;
      if (completo.caso_id !== caso.id || completo.id !== achado.id ||
          completo.extracao_id !== achado.extracao_id || completo.arquivo_id !== achado.arquivo_id ||
          completo.caminho !== achado.caminho || completo.pagina !== achado.pagina ||
          completo.total_paginas !== achado.total_paginas || completo.sha256_origem !== achado.sha256_origem ||
          completo.sha256_texto !== achado.sha256_texto) {
        setErroAbertura("O documento recebido não corresponde ao resultado selecionado. O conteúdo não foi exibido.");
        return;
      }
      setDestaque(faixaLiteralUtf8(completo.texto, pedido?.termo ?? ""));
      setDocumento(completo);
    } catch (falha) {
      if (ativa.current && abertura.current === atual) setErroAbertura(mensagem(falha));
    } finally {
      if (ativa.current && abertura.current === atual) setAbrindo(false);
    }
  }

  function mudarPagina(deslocamento: number) {
    if (!pedido || buscando) return;
    setPedido({ ...pedido, deslocamento }); setDocumento(null); setDestaque(null); setErroAbertura(null);
    setAbrindo(false); abertura.current += 1;
  }

  return (
    <section aria-label="Busca em textos documentais" className="painel p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-sm font-semibold text-texto">Textos documentais indexados</h2>
        <Selo tom="neutro">DERIVADOS LOCAIS</Selo>
      </div>
      <p className="mt-1 text-xs leading-relaxed text-texto-2">
        Pesquisa somente em documentos/páginas indexados explicitamente em Arquivos. Não é uma busca em toda a extração
        nem conferência da origem atual. Os filtros de app, extração e data da busca de registros não se aplicam a esta seção.
      </p>
      <form role="search" aria-label="Pesquisar documentos indexados" onSubmit={buscar} className="mt-3 flex items-center gap-2">
        <input type="search" aria-label="Termo documental" className="entrada min-w-0 flex-1" maxLength={MAXIMO_TERMO} value={texto} onChange={(evento) => setTexto(evento.target.value)} placeholder="Palavra ou expressão no texto indexado" />
        <Botao type="submit" tamanho="p" variante="secundario" carregando={buscando}>Buscar documentos</Botao>
      </form>
      {buscando ? <Carregando texto="Buscando textos documentais indexados…" /> : erro ? (
        <div className="mt-3"><Aviso>{erro}</Aviso></div>
      ) : resultado ? (
        <div className="mt-3">
          <p className="text-xs text-texto-2" role="status">{resultado.total.toLocaleString("pt-BR")} documento(s)/página(s) indexado(s) encontrado(s)</p>
          {resultado.itens.length === 0 ? <div className="mt-3"><Vazio titulo="Nenhum texto documental encontrado">Indexe documentos/páginas em Arquivos antes de pesquisar. A ausência de resultados não indica ausência de informação na extração.</Vazio></div> : (
            <ul aria-label="Resultados documentais" className="mt-2 divide-y divide-borda">
              {resultado.itens.map((achado) => (
                <li key={achado.id} className="py-3">
                  <div className="flex flex-wrap items-start justify-between gap-2">
                    <div className="min-w-0 flex-1">
                      <p className="break-all text-sm font-medium text-texto">{nomeArquivo(achado.caminho)}</p>
                      <p className="mt-0.5 break-all font-mono text-2xs text-texto-3">{achado.caminho}</p>
                    </div>
                    <Botao tamanho="p" variante="fantasma" onClick={() => void abrir(achado)}>Abrir texto indexado</Botao>
                  </div>
                  <p className="mt-2 whitespace-pre-wrap break-words text-sm text-texto-2">{achado.trecho}</p>
                  <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-2xs text-texto-3">
                    <span>{achado.pagina === null ? "Texto nativo" : `Página ${achado.pagina} de ${achado.total_paginas}`}</span>
                    <span>Indexado: {dataHora(achado.executado_em, caso.fuso_horario)}</span>
                    <span className="font-mono">{achado.motor_id} v{achado.motor_versao}</span>
                    <span>Origem registrada: <HashCurto valor={achado.sha256_origem} /></span>
                  </div>
                </li>
              ))}
            </ul>
          )}
          <Paginacao total={resultado.total} deslocamento={pedido?.deslocamento ?? 0} limite={LIMITE} mudar={mudarPagina} />
        </div>
      ) : <p className="mt-3 text-xs text-texto-3">Informe um termo para pesquisar os derivados documentais do caso.</p>}
      {abrindo && <Carregando texto="Abrindo derivado documental armazenado…" />}
      {erroAbertura && <div className="mt-3"><Aviso>{erroAbertura}</Aviso></div>}
      {documento && (
        <>
          <div className="mt-3 flex justify-end"><Botao tamanho="p" variante="fantasma" onClick={() => { abertura.current += 1; setDocumento(null); setDestaque(null); }}>Fechar documento indexado</Botao></div>
          <PainelAnaliseDocumento caso={caso} documento={documento} destaqueUtf8={destaque} />
        </>
      )}
    </section>
  );
}
