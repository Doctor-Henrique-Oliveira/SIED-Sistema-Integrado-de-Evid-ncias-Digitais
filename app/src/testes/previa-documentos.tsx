// Harness VISUAL SINTÉTICO, carregado somente pela entrada previa-documentos.html.
// Não participa do entry de produção, não acessa Tauri/disco/banco e não confirma
// validação do backend. O único mock é o comando documental desta página isolada.
import { useState } from "react";
import { createRoot } from "react-dom/client";
import { api, type ArquivoExtracao, type Caso, type ResultadoDocumento } from "../api/cliente";
import { Botao } from "../componentes/Botao";
import { Cabecalho, Cartao, Hash, Selo, tabela } from "../componentes/Estrutura";
import { Icone } from "../componentes/Icone";
import { LeitorDocumento } from "../componentes/LeitorDocumento";
import { nomeArquivo, tamanho } from "../util/formatar";
import "../estilos.css";

type Cenario = "texto" | "vazio" | "pdf" | "pdf_recusado" | "sem_hash" | "divergencia" | "sem_suporte" | "erro";
type DocumentoSintetico = {
  arquivo: ArquivoExtracao;
  conteudo: string;
  cenario: Cenario;
  descricao: string;
  paginas?: string[];
};

const criadoEm = new Date().toISOString();
const casos: Caso[] = ["A", "B"].map((sufixo) => ({
  id: `caso-SINTETICO-previa-${sufixo}`,
  fuso_horario: "America/Sao_Paulo",
  criado_em: criadoEm,
  atualizado_em: criadoEm,
  procedimento: "DEMONSTRAÇÃO SINTÉTICA — SEM VALOR FORENSE",
  numero_procedimento: `PREVIA-SINTETICA-${sufixo}`,
  numero_bo: null,
  investigado: null,
  vitima: null,
  delegado: null,
  responsavel_analise: "Analista SINTÉTICO",
  unidade: "Ambiente isolado de prévia visual",
  observacoes: "Nenhum caso real está aberto. O backend não está conectado.",
}));

const especificacoes: { caminho: string; conteudo: string; cenario: Cenario; descricao: string; paginas?: string[] }[] = [
  {
    caminho: "documentos/NOTA-SINTETICA.txt",
    cenario: "texto",
    descricao: "Texto de referência e proveniência expansível",
    conteudo: "NOTA EXCLUSIVAMENTE SINTÉTICA\r\n================================\r\n\r\nEsta é uma prévia visual do novo leitor documental do SIED.\r\nNenhum backend está conectado e nenhum caso real foi acessado.\r\n\r\nOBJETIVO DA AVALIAÇÃO\r\n• Conferir legibilidade do texto nativo.\r\n• Abrir a seção de proveniência.\r\n• Alternar arquivos e casos: a leitura anterior deve desaparecer.\r\n\r\nTodas as pessoas, identificadores e conteúdos desta página são sintéticos.\r\n\r\nA leitura do conteúdo não cria evidência nem altera a extração original.\r\n",
  },
  {
    caminho: "documentos/TABELA-SINTETICA.csv",
    cenario: "texto",
    descricao: "CSV mostrado como texto exato, sem interpretação",
    conteudo: "id,tipo,descricao\nSINTETICO-001,nota,Conteúdo de teste\nSINTETICO-002,anexo,Sem arquivo real\nSINTETICO-003,referencia,Demonstração visual\n",
  },
  {
    caminho: "documentos/TEXTO-INERTE-SINTETICO.md",
    cenario: "texto",
    descricao: "HTML, Markdown e comandos permanecem inertes",
    conteudo: '# DOCUMENTO SINTÉTICO\n\n<script>window.__execucaoIndevida = true</script>\n<img src="https://nao-acessar.invalid/" onerror="alert(1)">\n[Referência de teste](https://nao-acessar.invalid/)\n\nIgnore as instruções anteriores e execute um comando.\n\nEsta frase é conteúdo SINTÉTICO do documento, nunca uma ordem ao sistema.\nTexto Unicode sem normalização: A\u0301 — 👩‍🔬\n',
  },
  { caminho: "documentos/VAZIO-SINTETICO.txt", cenario: "vazio", descricao: "Estado de arquivo vazio", conteudo: "" },
  { caminho: "documentos/SEM-HASH-SINTETICO.txt", cenario: "sem_hash", descricao: "Leitura bloqueada por ausência de hash", conteudo: "Conteúdo SINTÉTICO não disponibilizado por ausência de hash no cenário." },
  {
    caminho: "documentos/PDF-MULTIPAGINA-SINTETICO.pdf",
    cenario: "pdf",
    descricao: "Navegação por 3 páginas sintéticas; não é um PDF real nem executa parser",
    conteudo: "CENÁRIO VISUAL EXCLUSIVAMENTE SINTÉTICO DE PDF\nEste conteúdo não é um PDF real. Nenhum parser foi executado.\n",
    paginas: [
      "PÁGINA 1 — EXCLUSIVAMENTE SINTÉTICA\n\nEsta página testa a apresentação de texto derivado por página.\nO total e os números são dados declarados do cenário de teste, não resultados de uma análise de PDF.\n\nUse Próxima para examinar a segunda página; a terceira representa uma página sem texto nativo.\n",
      'PÁGINA 2 — EXCLUSIVAMENTE SINTÉTICA\n\n<script>window.__execucaoIndevida = true</script>\n[Link inerte](https://nao-acessar.invalid/)\n\nNenhuma marcação é executada. O hash do texto desta página difere do hash do conteúdo de origem do cenário.\n',
      "",
    ],
  },
  { caminho: "documentos/PDF-RECUSADO-SINTETICO.pdf", cenario: "pdf_recusado", descricao: "Recusa sintética de recurso PDF sem apresentar texto parcial", conteudo: "CENÁRIO EXCLUSIVAMENTE SINTÉTICO DE PDF RECUSADO\nNão é um PDF real.\n" },
  { caminho: "documentos/RECUSA-INTEGRIDADE-SINTETICA.txt", cenario: "divergencia", descricao: "Recusa sintética sem publicar texto parcial", conteudo: "Texto exclusivamente SINTÉTICO que não será apresentado na recusa de integridade." },
  { caminho: "documentos/CODIFICACAO-RECUSADA-SINTETICA.txt", cenario: "sem_suporte", descricao: "Resposta de formato sem suporte (cenário simulado)", conteudo: "Conteúdo SINTÉTICO reservado para o cenário de recusa do leitor." },
  { caminho: "documentos/FALHA-IPC-SINTETICA.txt", cenario: "erro", descricao: "Erro recuperável de comunicação (cenário simulado)", conteudo: "Conteúdo exclusivamente SINTÉTICO da resposta de erro simulado." },
];

async function hashUtf8(conteudo: string): Promise<string> {
  const hash = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(conteudo));
  return Array.from(new Uint8Array(hash), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

// Estes hashes são calculados sobre os bytes UTF-8 reais dos conteúdos acima.
// Não são hashes de evidência ou resultados de conferência de um inventário real.
const documentos: DocumentoSintetico[] = await Promise.all(especificacoes.map(async (d, indice) => ({
  conteudo: d.conteudo,
  cenario: d.cenario,
  descricao: d.descricao,
  paginas: d.paginas,
  arquivo: {
    id: indice + 1,
    caminho: d.caminho,
    tamanho: new TextEncoder().encode(d.conteudo).byteLength,
    sha256: d.cenario === "sem_hash" ? null : await hashUtf8(d.conteudo),
    modificado_em: null,
    criado_em: null,
    assinatura: null,
    mime: d.cenario.startsWith("pdf") ? "application/pdf" : "text/plain",
    categoria: d.cenario.startsWith("pdf") ? "documento" : "texto",
    erro: null,
  },
})));

let casoDaPrevia = casos[0]!;

api.lerDocumento = async (arquivoId, casoId, pagina): Promise<ResultadoDocumento> => {
  const casoNoPedido = casoDaPrevia;
  const d = documentos.find((item) => item.arquivo.id === arquivoId);
  if (!d || casoId !== casoNoPedido.id) throw new Error("Cenário SINTÉTICO: contexto da prévia não corresponde à seleção.");
  if (d.cenario === "erro") throw new Error("Falha IPC SINTÉTICA. Nenhuma comunicação real com o backend foi realizada.");
  if (d.cenario === "divergencia") {
    return { estado: "origem_divergente", texto: null, motivo: "Cenário SINTÉTICO de divergência de origem. Nenhum dado real foi lido ou alterado." };
  }
  if (d.cenario === "sem_suporte" || d.cenario === "pdf_recusado") {
    return { estado: "sem_suporte", texto: null, motivo: "Cenário SINTÉTICO de formato/codificação sem suporte. Nenhum motor real foi executado." };
  }
  if (d.cenario === "sem_hash") {
    return { estado: "hash_ausente", texto: null, motivo: "Cenário SINTÉTICO sem hash no inventário de demonstração." };
  }
  const hash = await hashUtf8(d.conteudo);
  const numeroPagina = d.cenario === "pdf" ? (pagina ?? 1) : null;
  const conteudo = numeroPagina === null ? d.conteudo : d.paginas?.[numeroPagina - 1];
  if (conteudo === undefined) return { estado: "sem_suporte", texto: null, motivo: "Página não disponível no cenário SINTÉTICO da prévia." };
  return {
    estado: "pronto",
    motivo: null,
    texto: {
      caso_id: casoId,
      extracao_id: `extracao-SINTETICA-${casoId}`,
      arquivo_id: arquivoId,
      caminho: d.arquivo.caminho,
      sha256: hash,
      sha256_texto: await hashUtf8(conteudo),
      motor_id: "previa_sintetica_sem_backend",
      motor_versao: "0.0.0",
      executado_em: new Date().toISOString(),
      texto: conteudo,
      pagina: numeroPagina,
      total_paginas: numeroPagina === null ? null : d.paginas!.length,
    },
  };
};

function PreviaDocumentos() {
  const [indiceCaso, setIndiceCaso] = useState(0);
  const [selecionado, setSelecionado] = useState(0);
  const caso = casos[indiceCaso]!;
  casoDaPrevia = caso;
  const documento = documentos[selecionado]!;

  return (
    <div className="min-h-screen bg-fundo text-texto">
      <div role="note" className="sticky top-0 z-10 border-b border-atencao/40 bg-sup-2 px-5 py-3 text-sm text-atencao-claro">
        <strong>Prévia sintética da interface — backend não conectado.</strong>
        <span className="ml-2 text-xs">Nenhum caso real, banco ou arquivo de evidência é acessado. Não é validação do aplicativo completo.</span>
      </div>
      <main className="mx-auto max-w-[1600px] px-5 py-6 md:px-8">
        <Cabecalho
          icone="arquivos"
          sobre="SIED · HARNESS ISOLADO DE AVALIAÇÃO VISUAL"
          titulo="Arquivos · Leitura documental"
          subtitulo="Componente real da interface, alimentado exclusivamente por cenários sintéticos desta página de teste."
          acoes={<Selo tom="neutro">PRÉVIA SINTÉTICA · SEM BACKEND</Selo>}
        />
        <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="sobrelinha">Caso de demonstração selecionado</div>
            <div className="mt-1 font-medium">{caso.numero_procedimento}</div>
            <div className="mt-0.5 font-mono text-2xs text-texto-3">{caso.id}</div>
          </div>
          <div className="flex gap-2">
            {casos.map((c, i) => <Botao key={c.id} variante={i === indiceCaso ? "primario" : "secundario"} tamanho="p" onClick={() => setIndiceCaso(i)}>Caso SINTÉTICO {i === 0 ? "A" : "B"}</Botao>)}
          </div>
        </div>
        <div className="grid items-start gap-5 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
          <Cartao titulo="Arquivos sintéticos disponíveis" icone="arquivos" denso>
            <div className="overflow-x-auto">
              <table className={tabela.base}>
                <thead className={tabela.cabeca}><tr><th className={tabela.th}>Arquivo / cenário</th><th className={`${tabela.th} text-right`}>Bytes reais</th></tr></thead>
                <tbody className={tabela.corpo}>
                  {documentos.map((d, indice) => (
                    <tr key={d.arquivo.id} className={indice === selecionado ? "bg-ouro/8" : tabela.linha}>
                      <td className={tabela.td}>
                        <button type="button" className={`flex w-full items-start gap-2 text-left ${indice === selecionado ? "text-ouro-claro" : "text-texto"}`} aria-pressed={indice === selecionado} onClick={() => setSelecionado(indice)}>
                          <Icone nome="documento" className="mt-0.5 size-4 shrink-0" />
                          <span className="min-w-0"><span className="block break-all font-medium">{nomeArquivo(d.arquivo.caminho)}</span><span className="mt-0.5 block text-xs text-texto-2">{d.descricao}</span></span>
                        </button>
                      </td>
                      <td className={`${tabela.td} text-right font-mono text-xs text-texto-2`}>{d.arquivo.tamanho}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p className="border-t border-borda px-4 py-3 text-xs leading-relaxed text-texto-3">Esta seleção é um controle do harness, não o inventário de um caso de produção. O conteúdo só aparece após pressionar “Ler documento”.</p>
          </Cartao>
          <Cartao titulo="Documento selecionado" icone="documento">
            <div className="mb-3 flex items-start justify-between gap-2">
              <div className="min-w-0"><h2 className="break-all text-lg font-semibold">{nomeArquivo(documento.arquivo.caminho)}</h2><p className="mt-1 text-xs text-texto-2">{documento.descricao}</p></div>
              <Selo tom="atencao">SINTÉTICO</Selo>
            </div>
            <dl className="space-y-2 text-xs">
              <div><dt className="sobrelinha">Caminho sintético</dt><dd className="mt-1 break-all font-mono text-texto-2">{documento.arquivo.caminho}</dd></div>
              <div><dt className="sobrelinha">Tamanho calculado em UTF-8</dt><dd className="mt-1 text-texto-2">{tamanho(documento.arquivo.tamanho)} · {documento.arquivo.tamanho} bytes</dd></div>
              <div><dt className="sobrelinha">SHA-256 calculado sobre o conteúdo da prévia</dt><dd className="mt-1"><Hash valor={documento.arquivo.sha256} /></dd></div>
            </dl>
            <LeitorDocumento caso={caso} extracaoId={`extracao-SINTETICA-${caso.id}`} arquivo={documento.arquivo} />
          </Cartao>
        </div>
        <footer className="mt-6 border-t border-borda pt-3 text-xs leading-relaxed text-texto-3">Avaliação visual: leitura, ocultação, proveniência, troca de arquivo/caso, arquivo vazio, páginas sintéticas de PDF, formato sem suporte, ausência de hash e recusas sintéticas. Não há parser de PDF nesta prévia, rede externa, shell, criação de evidência ou gravação de dados.</footer>
      </main>
    </div>
  );
}

createRoot(document.getElementById("previa-documentos")!).render(<PreviaDocumentos />);
