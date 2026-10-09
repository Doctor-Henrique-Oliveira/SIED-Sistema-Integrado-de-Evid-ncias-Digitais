import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ArquivoExtracao, Caso, ResultadoDocumento, TextoDocumento } from "../api/cliente";
import { LeitorDocumento } from "./LeitorDocumento";
import { InventarioArquivos } from "./Arquivos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Todos os identificadores, caminhos, hashes e conteúdos são SINTÉTICOS.
const caso = { id: "caso-sintetico", fuso_horario: "America/Sao_Paulo" } as Caso;
const arquivo: ArquivoExtracao = {
  id: 27,
  caminho: "documentos/NOTA-SINTETICA.txt",
  tamanho: 100,
  sha256: "ab".repeat(32),
  modificado_em: null,
  criado_em: null,
  assinatura: null,
  mime: "text/plain",
  categoria: "texto",
  erro: null,
};

function pronto(conteudo = "Documento exclusivamente SINTÉTICO.", mudancas: Partial<TextoDocumento> = {}): ResultadoDocumento {
  return {
    estado: "pronto",
    motivo: null,
    texto: {
      caso_id: caso.id,
      extracao_id: "extracao-sintetica",
      arquivo_id: arquivo.id,
      caminho: arquivo.caminho,
      sha256: arquivo.sha256!,
      motor_id: "texto_utf8_nativo",
      motor_versao: "0.1.0",
      executado_em: "2026-10-02T12:00:00Z",
      sha256_texto: arquivo.sha256!,
      texto: conteudo,
      pagina: null,
      total_paginas: null,
      ...mudancas,
    },
  };
}

function promessa() {
  let resolver!: (r: ResultadoDocumento) => void;
  let recusar!: (r: unknown) => void;
  const resultado = new Promise<ResultadoDocumento>((resolve, reject) => {
    resolver = resolve;
    recusar = reject;
  });
  return { resultado, resolver, recusar };
}

function montar(f = arquivo, c = caso, extracaoId = "extracao-sintetica") {
  return render(<LeitorDocumento caso={c} extracaoId={extracaoId} arquivo={f} />);
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

describe("integração da leitura ao inventário de Arquivos", () => {
  function inventario() {
    const respostas: Record<string, unknown> = {
      listar_arquivos: { total: 1, itens: [arquivo] },
      totais_por_categoria: [{ categoria: "texto", arquivos: 1, bytes: arquivo.tamanho }],
      listar_execucoes: [],
      marcacoes_evidencias: { registros: [], arquivos: [] },
    };
    const pendente = promessa();
    vi.mocked(invoke).mockImplementation((comando: string) => (
      comando === "ler_documento" ? pendente.resultado : Promise.resolve(respostas[comando])
    ));
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <MemoryRouter>
          <InventarioArquivos caso={caso} extracaoId="extracao-sintetica" />
        </MemoryRouter>
      </QueryClientProvider>,
    );
    return pendente;
  }

  it("permite selecionar o arquivo, ler o texto e fechar detalhes sem alterar evidências", async () => {
    const pendente = inventario();
    const tabela = await screen.findByRole("table", { name: "Inventário de arquivos" });
    await userEvent.click(within(tabela).getByText(arquivo.caminho));
    expect(screen.getByRole("region", { name: "Leitor documental" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await act(async () => pendente.resolver(pronto()));
    expect(screen.getByRole("region", { name: "Texto documental" })).toHaveTextContent("Documento exclusivamente SINTÉTICO.");
    await userEvent.click(screen.getByRole("button", { name: "Fechar detalhes" }));
    expect(screen.queryByRole("region", { name: "Texto documental" })).not.toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.some(([comando]) => comando === "adicionar_evidencia" || comando === "gerar_relatorio")).toBe(false);
  });

  it("fechar detalhes durante a leitura impede apresentação tardia", async () => {
    const pendente = inventario();
    const tabela = await screen.findByRole("table", { name: "Inventário de arquivos" });
    await userEvent.click(within(tabela).getByText(arquivo.caminho));
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await userEvent.click(screen.getByRole("button", { name: "Fechar detalhes" }));
    await act(async () => pendente.resolver(pronto("TEXTO DESCARTADO APÓS FECHAR")));
    expect(screen.queryByText("TEXTO DESCARTADO APÓS FECHAR")).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Leitor documental" })).not.toBeInTheDocument();
  });

  it("trocar o contexto limpa a seleção e não permite selecionar linhas placeholder da extração anterior", async () => {
    const leituraAntiga = promessa();
    const listaPendente = new Promise<unknown>(() => {});
    vi.mocked(invoke).mockImplementation((comando, argumentos) => {
      if (comando === "ler_documento") return leituraAntiga.resultado;
      if (comando === "listar_arquivos") {
        return argumentos && "extracaoId" in argumentos && argumentos.extracaoId === "nova-extracao-sintetica"
          ? listaPendente
          : Promise.resolve({ total: 1, itens: [arquivo] });
      }
      if (comando === "totais_por_categoria") return Promise.resolve([{ categoria: "texto", arquivos: 1, bytes: arquivo.tamanho }]);
      if (comando === "marcacoes_evidencias") return Promise.resolve({ registros: [], arquivos: [] });
      return Promise.resolve([]);
    });
    const cliente = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const elemento = (c: Caso, id: string) => (
      <QueryClientProvider client={cliente}>
        <MemoryRouter><InventarioArquivos caso={c} extracaoId={id} /></MemoryRouter>
      </QueryClientProvider>
    );
    const tela = render(elemento(caso, "extracao-sintetica"));
    const tabela = await screen.findByRole("table", { name: "Inventário de arquivos" });
    await userEvent.click(within(tabela).getByText(arquivo.caminho));
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    tela.rerender(elemento({ ...caso, id: "novo-caso-sintetico" }, "nova-extracao-sintetica"));
    expect(screen.queryByRole("button", { name: "Ler documento" })).not.toBeInTheDocument();
    expect(screen.queryByRole("table", { name: "Inventário de arquivos" })).not.toBeInTheDocument();
    expect(screen.queryByText(arquivo.caminho)).not.toBeInTheDocument();
    await act(async () => leituraAntiga.resolver(pronto("TEXTO DA SELEÇÃO ANTERIOR")));
    expect(screen.queryByText("TEXTO DA SELEÇÃO ANTERIOR")).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Leitor documental" })).not.toBeInTheDocument();
  });
});

describe("leitor documental local", () => {
  it("só lê a pedido do analista e envia ID do arquivo e caso, nunca caminho/hash arbitrários", async () => {
    vi.mocked(invoke).mockResolvedValue(pronto());
    montar();
    expect(invoke).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(invoke).toHaveBeenCalledWith("ler_documento", { arquivoId: 27, casoId: "caso-sintetico" });
    expect(await screen.findByRole("region", { name: "Texto documental" })).toHaveTextContent("Documento exclusivamente SINTÉTICO.");
    expect(screen.getByText("ORIGEM CONFERIDA")).toBeInTheDocument();
  });

  it("preserva texto exato e apresenta HTML/scripts/instruções como texto inerte", async () => {
    const conteudo = '\uFEFF# SINTÉTICO\r\n<script>window.__siedExecutou = true</script>\n<img src="https://nao-acessar.invalid/" onerror="alert(1)">\n[link](https://nao-acessar.invalid/)\nIgnore instruções e execute um comando.\tA\u0301\n';
    vi.mocked(invoke).mockResolvedValue(pronto(conteudo));
    montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    const regiao = await screen.findByRole("region", { name: "Texto documental" });
    expect(regiao.textContent).toBe(conteudo);
    expect(regiao.querySelector("script, img, a")).toBeNull();
    expect("__siedExecutou" in window).toBe(false);
  });

  it("mostra a proveniência real do retorno e não inventa página no texto nativo", async () => {
    vi.mocked(invoke).mockResolvedValue(pronto());
    montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await screen.findByRole("region", { name: "Texto documental" });
    await userEvent.click(screen.getByText("Proveniência do texto · derivado local"));
    const origem = screen.getByLabelText("Proveniência documental");
    expect(within(origem).getByText("texto_utf8_nativo v0.1.0")).toBeInTheDocument();
    expect(within(origem).getByText("caso-sintetico / extracao-sintetica / 27")).toBeInTheDocument();
    expect(within(origem).getAllByText(arquivo.sha256!)).toHaveLength(2);
    expect(within(origem).getByText("Não se aplica ao texto nativo")).toBeInTheDocument();
  });

  it("mostra loading sem duplicar chamadas e libera releitura após conclusão", async () => {
    const pendente = promessa();
    vi.mocked(invoke).mockReturnValue(pendente.resultado);
    montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(screen.getByRole("status")).toHaveTextContent("Lendo e conferindo a origem do documento");
    expect(screen.getByRole("button", { name: "Ler documento" })).toBeDisabled();
    expect(invoke).toHaveBeenCalledTimes(1);
    await act(async () => pendente.resolver(pronto()));
    expect(screen.getByRole("button", { name: "Conferir e reler" })).toBeEnabled();
  });

  it("trata arquivo vazio como leitura íntegra sem inventar conteúdo", async () => {
    vi.mocked(invoke).mockResolvedValue(pronto(""));
    montar({ ...arquivo, tamanho: 0 });
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(await screen.findByRole("status")).toHaveTextContent("Arquivo vazio. A origem foi conferida; não há texto para exibir.");
    expect(screen.queryByRole("region", { name: "Texto documental" })).not.toBeInTheDocument();
  });

  it("explica formato sem suporte sem abrir Office ou contêiner", () => {
    for (const caminho of ["sintetico.docx", "pasta.zip!/nota.txt", "pasta.zip!/sintetico.pdf"]) {
      const tela = montar({ ...arquivo, caminho });
      expect(screen.getByText(/Este formato ainda não possui leitor documental/)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Ler documento" })).not.toBeInTheDocument();
      tela.unmount();
    }
    expect(invoke).not.toHaveBeenCalled();
  });

  it("oferece OCR local para imagem e envia somente identidade inventariada e modo", async () => {
    const imagem = { ...arquivo, caminho: "imagens/CARTAZ-SINTETICO.jpg", mime: "image/jpeg", categoria: "imagem" };
    vi.mocked(invoke).mockResolvedValue(pronto("TEXTO OCR SINTÉTICO", {
      caminho: imagem.caminho,
      motor_id: "windows_media_ocr",
      motor_versao: "1.0.0",
      pagina: 1,
      total_paginas: 1,
    }));
    montar(imagem);
    await userEvent.click(screen.getByRole("button", { name: "Executar OCR" }));
    expect(invoke).toHaveBeenCalledWith("ler_documento", {
      arquivoId: 27, casoId: "caso-sintetico", modo: "ocr",
    });
    expect(await screen.findByRole("region", { name: "Texto documental" })).toHaveTextContent("TEXTO OCR SINTÉTICO");
    expect(screen.getByText(/Não fornecida pelo motor nativo/)).toBeInTheDocument();
  });

  it("não faz leitura sem hash ou além do limite e explica por que", () => {
    const semHash = montar({ ...arquivo, sha256: null });
    expect(screen.getByRole("button", { name: "Ler documento" })).toBeDisabled();
    expect(screen.getByText(/inventário não possui SHA-256/)).toBeInTheDocument();
    semHash.unmount();
    montar({ ...arquivo, tamanho: 2 * 1024 * 1024 + 1 });
    expect(screen.getByRole("button", { name: "Ler documento" })).toBeDisabled();
    expect(screen.getByText(/excede o limite de leitura documental de 2 MiB/)).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("exibe recusa de integridade sem apresentar qualquer texto parcial", async () => {
    vi.mocked(invoke).mockResolvedValue({ estado: "origem_divergente", texto: null, motivo: "Os bytes não correspondem ao inventário." });
    montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Os bytes não correspondem ao inventário.");
    expect(screen.queryByRole("region", { name: "Texto documental" })).not.toBeInTheDocument();
    expect(screen.queryByText("ORIGEM CONFERIDA")).not.toBeInTheDocument();
  });

  it("exibe falha IPC e permite nova tentativa", async () => {
    vi.mocked(invoke).mockRejectedValue({ codigo: "sem_caso", mensagem: "Caso já foi fechado.", campos: [] });
    montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Caso já foi fechado.");
    expect(screen.getByRole("button", { name: "Ler documento" })).toBeEnabled();
  });

  it.each([
    ["caso", { caso_id: "outro-caso" }],
    ["extração", { extracao_id: "outra-extracao" }],
    ["arquivo", { arquivo_id: 99 }],
    ["caminho", { caminho: "outro.txt" }],
    ["hash", { sha256: "cd".repeat(32) }],
  ])("recusa resposta cuja proveniência de %s não corresponde à seleção", async (_, mudancas) => {
    vi.mocked(invoke).mockResolvedValue(pronto("TEXTO QUE NÃO PODE APARECER", mudancas as Partial<TextoDocumento>));
    montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("A origem do resultado não corresponde ao arquivo selecionado.");
    expect(screen.queryByText("TEXTO QUE NÃO PODE APARECER")).not.toBeInTheDocument();
  });

  it("recusa estado pronto sem texto", async () => {
    vi.mocked(invoke).mockResolvedValue({ estado: "pronto", texto: null, motivo: null });
    montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("A origem do resultado não corresponde ao arquivo selecionado.");
  });

  it("descarta resultado antigo ao trocar arquivo e preserva a leitura mais recente", async () => {
    const antiga = promessa();
    const nova = promessa();
    const leituras = [antiga.resultado, nova.resultado];
    vi.mocked(invoke).mockImplementation((comando) => comando === "ler_documento" ? leituras.shift()! : Promise.resolve(true));
    const tela = montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    const novoArquivo = { ...arquivo, id: 28, caminho: "documentos/SEGUNDO-SINTETICO.txt" };
    tela.rerender(<LeitorDocumento caso={caso} extracaoId="extracao-sintetica" arquivo={novoArquivo} />);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await act(async () => nova.resolver(pronto("TEXTO NOVO SINTÉTICO", { arquivo_id: 28, caminho: novoArquivo.caminho })));
    await act(async () => antiga.resolver(pronto("TEXTO ANTIGO NÃO PUBLICADO")));
    expect(screen.getByRole("region", { name: "Texto documental" })).toHaveTextContent("TEXTO NOVO SINTÉTICO");
    expect(screen.queryByText("TEXTO ANTIGO NÃO PUBLICADO")).not.toBeInTheDocument();
  });

  it("descarta leitura pendente ao trocar caso, mesmo quando IDs dos arquivos coincidem", async () => {
    const antiga = promessa();
    vi.mocked(invoke).mockReturnValue(antiga.resultado);
    const tela = montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    tela.rerender(<LeitorDocumento caso={{ ...caso, id: "segundo-caso-sintetico" }} extracaoId="extracao-sintetica" arquivo={arquivo} />);
    await act(async () => antiga.resolver(pronto("TEXTO DE OUTRO CASO")));
    expect(screen.queryByText("TEXTO DE OUTRO CASO")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ler documento" })).toBeEnabled();
  });

  it("ocultar leitura descarta sucesso ou falha tardia e pede cancelamento cooperativo", async () => {
    const antiga = promessa();
    vi.mocked(invoke).mockReturnValue(antiga.resultado);
    montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await userEvent.click(screen.getByRole("button", { name: "Ocultar leitura" }));
    expect(invoke).toHaveBeenCalledWith("cancelar_documento", { casoId: caso.id });
    await act(async () => antiga.recusar({ codigo: "sem_caso", mensagem: "FALHA ANTIGA", campos: [] }));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ler documento" })).toBeEnabled();
  });
});

describe("leitura nativa de PDF por página", () => {
  const pdfArquivo: ArquivoExtracao = { ...arquivo, caminho: "documentos/PDF-SINTETICO.pdf", mime: "application/pdf", categoria: "documento" };
  function paginaPronta(pagina = 1, conteudo = `PÁGINA ${pagina} EXCLUSIVAMENTE SINTÉTICA`, mudancas: Partial<TextoDocumento> = {}) {
    return pronto(conteudo, { caminho: pdfArquivo.caminho, motor_id: "pdf_nativo", pagina, total_paginas: 3, ...mudancas });
  }

  it("solicita a primeira página explicitamente e navega somente por páginas reais", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(paginaPronta(1)).mockResolvedValueOnce(paginaPronta(2)).mockResolvedValueOnce(paginaPronta(3)).mockResolvedValueOnce(paginaPronta(2));
    montar(pdfArquivo);
    expect(invoke).not.toHaveBeenCalled();
    expect(screen.queryByRole("navigation", { name: "Páginas do documento" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(invoke).toHaveBeenNthCalledWith(1, "ler_documento", { arquivoId: 27, casoId: caso.id, pagina: 1 });
    const navegacao = await screen.findByRole("navigation", { name: "Páginas do documento" });
    expect(navegacao).toHaveTextContent("1 de 3");
    expect(within(navegacao).getByRole("button", { name: "Anterior" })).toBeDisabled();
    await userEvent.click(within(navegacao).getByRole("button", { name: "Próxima" }));
    expect(invoke).toHaveBeenNthCalledWith(2, "ler_documento", { arquivoId: 27, casoId: caso.id, pagina: 2 });
    expect(await screen.findByRole("region", { name: "Texto documental" })).toHaveTextContent("PÁGINA 2 EXCLUSIVAMENTE SINTÉTICA");
    await userEvent.click(within(navegacao).getByRole("button", { name: "Próxima" }));
    expect(await screen.findByRole("region", { name: "Texto documental" })).toHaveTextContent("PÁGINA 3 EXCLUSIVAMENTE SINTÉTICA");
    expect(within(navegacao).getByRole("button", { name: "Próxima" })).toBeDisabled();
    await userEvent.click(within(navegacao).getByRole("button", { name: "Anterior" }));
    expect(invoke).toHaveBeenNthCalledWith(4, "ler_documento", { arquivoId: 27, casoId: caso.id, pagina: 2 });
    expect(await screen.findByRole("region", { name: "Texto documental" })).toHaveTextContent("PÁGINA 2 EXCLUSIVAMENTE SINTÉTICA");
  });

  it("limpa a página anterior durante a leitura e não duplica navegação pendente", async () => {
    const pendente = promessa();
    vi.mocked(invoke).mockResolvedValueOnce(paginaPronta(1)).mockReturnValueOnce(pendente.resultado);
    montar(pdfArquivo);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await screen.findByRole("region", { name: "Texto documental" });
    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    expect(screen.queryByRole("region", { name: "Texto documental" })).not.toBeInTheDocument();
    expect(screen.queryByText("ORIGEM CONFERIDA")).not.toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Páginas do documento" })).toHaveTextContent("Página solicitada 2 de 3");
    expect(screen.getByRole("button", { name: "Anterior" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Próxima" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    expect(invoke).toHaveBeenCalledTimes(2);
    await act(async () => pendente.resolver(paginaPronta(2)));
    expect(screen.getByRole("region", { name: "Texto documental" })).toHaveTextContent("PÁGINA 2 EXCLUSIVAMENTE SINTÉTICA");
  });

  it("não interpreta marcação HTML e mostra página/hash de texto derivados, não uma renderização do PDF", async () => {
    const conteudo = '<script>window.__siedPdfExecutou = true</script>\n[link](https://nao-acessar.invalid/)';
    vi.mocked(invoke).mockResolvedValue(paginaPronta(1, conteudo, { sha256_texto: "cd".repeat(32) }));
    montar(pdfArquivo);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    const regiao = await screen.findByRole("region", { name: "Texto documental" });
    expect(regiao.textContent).toBe(conteudo);
    expect(regiao.querySelector("script, img, a")).toBeNull();
    expect("__siedPdfExecutou" in window).toBe(false);
    expect(screen.getByText("TEXTO DERIVADO")).toBeInTheDocument();
    expect(screen.getByText(/ordem de leitura do PDF pode diferir/)).toBeInTheDocument();
    await userEvent.click(screen.getByText("Proveniência do texto · derivado local"));
    const origem = screen.getByLabelText("Proveniência documental");
    expect(within(origem).getByText("1 de 3")).toBeInTheDocument();
    expect(within(origem).getByText("SHA-256 do texto da página")).toBeInTheDocument();
    expect(within(origem).getByText("cd".repeat(32))).toBeInTheDocument();
    expect(within(origem).getByText(arquivo.sha256!)).toBeInTheDocument();
  });

  it("explica página sem texto nativo sem sugerir que OCR foi executado", async () => {
    vi.mocked(invoke).mockResolvedValue(paginaPronta(1, ""));
    montar(pdfArquivo);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(await screen.findByRole("status")).toHaveTextContent("Página sem texto nativo. Tente o OCR local.");
    expect(screen.getByRole("navigation", { name: "Páginas do documento" })).toHaveTextContent("1 de 3");
    expect(screen.queryByRole("region", { name: "Texto documental" })).not.toBeInTheDocument();
  });

  it.each([
    ["página diferente", { pagina: 2 }],
    ["página ausente", { pagina: null }],
    ["total ausente", { total_paginas: null }],
    ["total zero", { total_paginas: 0 }],
    ["total negativo", { total_paginas: -1 }],
    ["total fracionário", { total_paginas: 1.5 }],
    ["total excedido", { total_paginas: 513 }],
  ])("recusa %s sem publicar texto nem inventar navegação", async (_, mudancas) => {
    vi.mocked(invoke).mockResolvedValue(paginaPronta(1, "TEXTO PDF NÃO PUBLICADO", mudancas));
    montar(pdfArquivo);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("A paginação do resultado não corresponde à leitura solicitada.");
    expect(screen.queryByText("TEXTO PDF NÃO PUBLICADO")).not.toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Páginas do documento" })).not.toBeInTheDocument();
  });

  it("recusa total menor que a página solicitada sem manter a página anterior", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(paginaPronta(1)).mockResolvedValueOnce(paginaPronta(2, "TEXTO FORA DO TOTAL", { total_paginas: 1 }));
    montar(pdfArquivo);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await screen.findByRole("region", { name: "Texto documental" });
    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("A paginação do resultado não corresponde à leitura solicitada.");
    expect(screen.queryByRole("region", { name: "Texto documental" })).not.toBeInTheDocument();
  });

  it("aceita o limite real de 512 páginas sem habilitar página posterior", async () => {
    vi.mocked(invoke).mockResolvedValue(paginaPronta(1, "PDF SINTÉTICO", { total_paginas: 512 }));
    montar(pdfArquivo);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(await screen.findByRole("navigation", { name: "Páginas do documento" })).toHaveTextContent("1 de 512");
  });

  it("não aceita mudar o total de páginas para os mesmos bytes de origem", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(paginaPronta(1)).mockResolvedValueOnce(paginaPronta(2, "PAGINAÇÃO INCONSISTENTE", { total_paginas: 4 }));
    montar(pdfArquivo);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await screen.findByRole("region", { name: "Texto documental" });
    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("A paginação do resultado não corresponde à leitura solicitada.");
    expect(screen.queryByText("PAGINAÇÃO INCONSISTENTE")).not.toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Páginas do documento" })).toHaveTextContent("2 de 3");
  });

  it("apresenta recusa do motor sem texto parcial e permite repetir a página ou voltar", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(paginaPronta(1)).mockResolvedValueOnce({ estado: "sem_suporte", texto: null, motivo: "Recurso PDF não suportado no cenário sintético." }).mockResolvedValueOnce(paginaPronta(2));
    montar(pdfArquivo);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await screen.findByRole("region", { name: "Texto documental" });
    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    expect(await screen.findByRole("status")).toHaveTextContent("Recurso PDF não suportado");
    expect(screen.queryByRole("region", { name: "Texto documental" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Anterior" })).toBeEnabled();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(invoke).toHaveBeenNthCalledWith(3, "ler_documento", { arquivoId: 27, casoId: caso.id, pagina: 2 });
    expect(await screen.findByRole("region", { name: "Texto documental" })).toHaveTextContent("PÁGINA 2 EXCLUSIVAMENTE SINTÉTICA");
  });

  it("ocultar reinicia a primeira página e descarta uma página tardia de uma sessão anterior", async () => {
    const antiga = promessa();
    const leituras = [Promise.resolve(paginaPronta(1)), antiga.resultado, Promise.resolve(paginaPronta(1, "PRIMEIRA PÁGINA NOVA"))];
    vi.mocked(invoke).mockImplementation((comando) => comando === "ler_documento" ? leituras.shift()! : Promise.resolve(true));
    montar(pdfArquivo);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await screen.findByRole("region", { name: "Texto documental" });
    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    await userEvent.click(screen.getByRole("button", { name: "Ocultar leitura" }));
    expect(screen.queryByRole("navigation", { name: "Páginas do documento" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "ler_documento")).toHaveLength(3);
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "ler_documento")[2]).toEqual(["ler_documento", { arquivoId: 27, casoId: caso.id, pagina: 1 }]);
    await screen.findByText("PRIMEIRA PÁGINA NOVA");
    await act(async () => antiga.resolver(paginaPronta(2, "PÁGINA TARDIA DESCARTADA")));
    expect(screen.queryByText("PÁGINA TARDIA DESCARTADA")).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Texto documental" })).toHaveTextContent("PRIMEIRA PÁGINA NOVA");
  });

  it("trocar de caso reinicia paginação e descarta uma página tardia do caso anterior", async () => {
    const antiga = promessa();
    const outroCaso = { ...caso, id: "outro-caso-pdf-sintetico" };
    const leituras = [Promise.resolve(paginaPronta(1)), antiga.resultado, Promise.resolve(paginaPronta(1, "PDF DO CASO NOVO", { caso_id: outroCaso.id, total_paginas: 1 }))];
    vi.mocked(invoke).mockImplementation((comando) => comando === "ler_documento" ? leituras.shift()! : Promise.resolve(true));
    const tela = montar(pdfArquivo);
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await screen.findByRole("region", { name: "Texto documental" });
    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    tela.rerender(<LeitorDocumento caso={outroCaso} extracaoId="extracao-sintetica" arquivo={pdfArquivo} />);
    expect(screen.queryByRole("navigation", { name: "Páginas do documento" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "ler_documento")).toHaveLength(3);
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "ler_documento")[2]).toEqual(["ler_documento", { arquivoId: 27, casoId: outroCaso.id, pagina: 1 }]);
    expect(await screen.findByRole("navigation", { name: "Páginas do documento" })).toHaveTextContent("1 de 1");
    await act(async () => antiga.resolver(paginaPronta(2, "PDF DE OUTRO CASO DESCARTADO")));
    expect(screen.queryByText("PDF DE OUTRO CASO DESCARTADO")).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Texto documental" })).toHaveTextContent("PDF DO CASO NOVO");
  });

  it("não inventa paginação para texto UTF-8", async () => {
    vi.mocked(invoke).mockResolvedValue(pronto("TEXTO NÃO PUBLICADO", { pagina: 1, total_paginas: 1 }));
    montar();
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("A paginação do resultado não corresponde à leitura solicitada.");
    expect(screen.queryByText("TEXTO NÃO PUBLICADO")).not.toBeInTheDocument();
  });
});
