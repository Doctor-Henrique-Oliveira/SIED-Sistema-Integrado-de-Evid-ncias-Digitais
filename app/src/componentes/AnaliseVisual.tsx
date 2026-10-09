import { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { api, type ArquivoExtracao, type Caso, type VisualIndexado, type ResultadoSimilarVisual } from "../api/cliente";
import type { ResultadoVisaoLocal } from "../api/visual";
import { Botao } from "./Botao";
import { Aviso, Carregando, Hash, Selo } from "./Estrutura";

export function AnaliseVisual({ caso, extracaoId, arquivo }: {caso:Caso;extracaoId:string;arquivo:ArquivoExtracao}) {
  return <Sessao key={`${caso.id}:${extracaoId}:${arquivo.id}:${arquivo.sha256}`} caso={caso} extracaoId={extracaoId} arquivo={arquivo} />;
}
function Sessao({caso,extracaoId,arquivo}:{caso:Caso;extracaoId:string;arquivo:ArquivoExtracao}) {
  const [visual,setVisual]=useState<VisualIndexado|null>(null);
  const [resultado,setResultado]=useState<ResultadoSimilarVisual|null>(null);
  const [visao,setVisao]=useState<ResultadoVisaoLocal|null>(null);
  const [ocupada,setOcupada]=useState(false);
  const [erro,setErro]=useState<string|null>(null);
  const [distancia,setDistancia]=useState(7);
  const ativa=useRef(true), operacao=useRef<string|null>(null);
  useEffect(()=>{ativa.current=true;return()=>{ativa.current=false;if(operacao.current)void api.cancelarDescoberta(operacao.current).catch(()=>{});};},[]);
  async function indexar(){
    if(ocupada)return;setOcupada(true);setErro(null);setResultado(null);
    try{
      const v=await api.indexarVisual(caso.id,arquivo.id);
      if(!ativa.current)return;
      if(v.caso_id!==caso.id||v.arquivo_id!==arquivo.id||v.extracao_id!==extracaoId||v.sha256_origem!==arquivo.sha256||v.caminho!==arquivo.caminho){throw Error("Descritor não corresponde à imagem selecionada.");}
      setVisual(v);
    }catch(e){if(ativa.current)setErro(e instanceof Error?e.message:"Não foi possível analisar a imagem.");}
    finally{if(ativa.current)setOcupada(false);}
  }
  async function pesquisar(){
    if(!visual||ocupada)return;setOcupada(true);setErro(null);setResultado(null);
    const id=crypto.randomUUID();operacao.current=id;
    try{
      const r=await api.semelhantesVisuais(caso.id,id,visual.id,distancia,30);
      if(!ativa.current)return;
      if(r.referencia.id!==visual.id||r.referencia.caso_id!==caso.id||r.referencia.hash_derivado!==visual.hash_derivado)throw Error("Resultado visual não corresponde à referência solicitada.");
      setResultado(r);
    }catch(e){if(ativa.current)setErro(e instanceof Error?e.message:"Não foi possível pesquisar imagens.");}
    finally{if(ativa.current){operacao.current=null;setOcupada(false);}}
  }
  async function classificar(){
    if(ocupada)return;setOcupada(true);setErro(null);setVisao(null);
    try{const r=await api.analisarVisualLocal(caso.id,arquivo.id);if(ativa.current)setVisao(r);}
    catch(e){if(ativa.current)setErro(e instanceof Error?e.message:"Não foi possível executar o motor visual local.");}
    finally{if(ativa.current)setOcupada(false);}
  }
  return <section aria-label="Similaridade visual local" className="mt-4 space-y-3 border-t border-borda pt-4">
    <div className="flex items-center justify-between gap-2"><h3 className="text-sm font-semibold">Pesquisa por imagem</h3><Selo tom="neutro">LOCAL · DERIVADO</Selo></div>
    <p className="text-xs text-texto-2">SHA-256 identifica arquivos de mesmos bytes. dHash compara estrutura visual grosseira, não identidade ou objetos. Classificação neural está indisponível.</p>
    <div className="flex flex-wrap gap-2"><Botao tamanho="p" variante="secundario" onClick={()=>void indexar()} disabled={!arquivo.sha256||arquivo.tamanho>2*1024*1024||ocupada}>Analisar imagem</Botao><Botao tamanho="p" variante="fantasma" onClick={()=>void classificar()} disabled={!arquivo.sha256||ocupada}>Classificar/detectar local</Botao></div>
    {(!arquivo.sha256 || arquivo.tamanho > 2*1024*1024) && <p className="text-xs text-texto-3">{!arquivo.sha256 ? "A imagem não tem SHA-256 conferível no inventário." : "A leitura visual local aceita arquivos de até 2 MiB nesta versão."} Análise indisponível para este arquivo.</p>}
    {ocupada&&<Carregando texto="Processando consulta visual local…"/>}
    {operacao.current&&<Botao tamanho="p" variante="fantasma" onClick={()=>void api.cancelarDescoberta(operacao.current!).catch(e=>setErro(String(e)))}>Solicitar cancelamento</Botao>}
    {erro&&<Aviso>{erro}</Aviso>}
    {visual&&<>
      <p className="text-xs text-texto-2">{visual.largura} × {visual.altura} · luminância média {visual.luminancia_media.toFixed(1)} / 255</p>
      <details className="text-xs"><summary className="cursor-pointer">Descritor e proveniência</summary><p className="mt-2 break-all font-mono">{visual.motor_id} · {visual.motor_versao}</p><p className="mt-2">dHash (não criptográfico): {visual.dhash}</p><Hash valor={visual.hash_derivado}/><p className="mt-2 text-texto-3">Cache versionado conferido contra seu hash. Reutilizá-lo não relê a imagem atual. Sem rotação EXIF/ICC; imagens uniformes diferentes podem coincidir.</p></details>
      <label className="flex items-center gap-2 text-xs">Máxima distância em bits<select aria-label="Distância visual" className="entrada" value={distancia} disabled={ocupada} onChange={e=>setDistancia(Number(e.target.value))}>{[0,3,7,12,16].map(n=><option key={n} value={n}>{n} de 64</option>)}</select></label>
      <Botao tamanho="p" onClick={()=>void pesquisar()} disabled={ocupada}>Buscar semelhantes no caso</Botao>
    </>}
    {resultado&&<>
      <p role="status" className="text-xs">{resultado.descritores_examinados} descritores examinados · {resultado.itens.length} candidatos exibidos{resultado.varredura_limitada?" · varredura limitada":""}</p>
      {resultado.limites.map((l,i)=><p key={i} className="text-xs text-texto-3">{l}</p>)}
      {resultado.itens.length===0?<p className="text-xs">Nenhum candidato nesta janela. A ausência não comprova ausência de imagens relacionadas.</p>:<ul className="divide-y divide-borda" aria-label="Imagens semelhantes">{resultado.itens.map(a=><li key={a.arquivo_id} className="py-2 text-xs"><Link className="break-all text-ouro-claro" to={`/casos/${caso.id}/arquivos?extracao=${a.extracao_id}&arquivo=${a.arquivo_id}`}>{a.caminho}</Link><p className="mt-1 text-texto-2">{a.mecanismo==="sha256_exato"?"Mesmos bytes por SHA-256":"Candidato dHash"}{a.distancia_bits!==null?` · distância ${a.distancia_bits}/64`:""}{a.score_tecnico!==null?` · score técnico ${a.score_tecnico.toFixed(4)}`:""}</p><details><summary className="cursor-pointer">Por que esta imagem apareceu?</summary><p>{a.mecanismo==="sha256_exato"?"SHA-256 do inventário igual ao da referência. Não estabelece autoria ou interpretação do conteúdo.":"Distância Hamming do descritor dHash dentro do limite escolhido; não é classificação de objetos nem identidade."}</p><Hash valor={a.sha256}/></details></li>)}</ul>}
    </>}
    {visao&&<section className="rounded border border-info/30 bg-info/5 p-3" aria-label="Candidatos visuais locais"><div className="flex items-center justify-between gap-2"><h4 className="text-xs font-semibold text-texto">Candidatos de visão local</h4><Selo tom="info">CANDIDATO</Selo></div><p className="mt-1 text-2xs text-texto-3">{visao.motor_id} · {visao.motor_versao} · {visao.regioes.length} região(ões)</p><p className="mt-2 text-xs text-texto-2">{visao.observacao}</p>{visao.regioes.length===0?<p className="mt-2 text-xs text-texto-3">Nenhuma região foi retornada pelo motor configurado.</p>:<ul className="mt-2 space-y-1.5" aria-label="Regiões candidatas">{visao.regioes.map((r,i)=><li key={`${r.rotulo}-${i}`} className="rounded border border-borda bg-sup-2/40 p-2 text-xs"><div className="flex items-center justify-between gap-2"><span className="font-medium text-texto">{r.rotulo}</span><span className="font-mono text-2xs text-ouro">{(r.confianca*100).toFixed(1)}%</span></div><p className="mt-1 font-mono text-2xs text-texto-3">x {r.x.toFixed(3)} · y {r.y.toFixed(3)} · w {r.largura.toFixed(3)} · h {r.altura.toFixed(3)}</p><p className="mt-1 text-2xs text-texto-3">Mecanismo: {r.mecanismo} · versão {r.versao}</p></li>)}</ul>}</section>}
  </section>;
}
