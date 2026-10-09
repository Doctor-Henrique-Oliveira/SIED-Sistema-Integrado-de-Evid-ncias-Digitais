import { invoke } from "@tauri-apps/api/core";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AchadoDocumento, ArquivoExtracao, Caso, DocumentoIndexado, EntidadeDocumento, Pagina, ResultadoDocumento } from "../api/cliente";
import { BuscaDocumentos } from "./BuscaDocumentos";
import { LeitorDocumento } from "./LeitorDocumento";
import { PainelAnaliseDocumento } from "./PainelAnaliseDocumento";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Conteúdo, caminhos, identificadores, endereços e hashes exclusivamente SINTÉTICOS.
const caso = { id: "caso-sintetico", fuso_horario: "America/Sao_Paulo" } as Caso;
const texto = "SINTÉTICO 🚀 contato-sintetico@example.invalid";
const documento: DocumentoIndexado = {
  id: 41, caso_id: caso.id, extracao_id: "extracao-sintetica", arquivo_id: 27,
  caminho: "documentos/NOTA-SINTETICA.txt", sha256_origem: "ab".repeat(32), sha256_texto: "cd".repeat(32), texto,
  pagina: null, total_paginas: null, motor_id: "texto_utf8_nativo", motor_versao: "0.1.0",
  executado_em: "2026-10-02T12:00:00Z", entidades_limitadas: false, entidades_motor_versao: "regras_sinteticas-1",
};
const literal = "contato-sintetico@example.invalid";
const inicio = new TextEncoder().encode(texto.slice(0, texto.indexOf(literal))).length;
const entidade: EntidadeDocumento = {
  id: 73, documento_id: documento.id, tipo: "email", inicio_utf8: inicio,
  fim_utf8: inicio + new TextEncoder().encode(literal).length, literal, regra_versao: "email_sintetico-1",
  estado: "nao_revisado", nota: null, revista_em: null, revisor: null,
};
const arquivo: ArquivoExtracao = {
  id: documento.arquivo_id, caminho: documento.caminho, tamanho: 100, sha256: documento.sha256_origem,
  modificado_em: null, criado_em: null, assinatura: null, mime: "text/plain", categoria: "texto", erro: null,
};
const leitura: ResultadoDocumento = {
  estado: "pronto", motivo: null,
  texto: {
    caso_id: caso.id, extracao_id: documento.extracao_id, arquivo_id: arquivo.id, caminho: arquivo.caminho,
    sha256: arquivo.sha256!, sha256_texto: documento.sha256_texto, texto: documento.texto,
    pagina: null, total_paginas: null, motor_id: documento.motor_id, motor_versao: documento.motor_versao,
    executado_em: documento.executado_em,
  },
};
function achado(d = documento): AchadoDocumento {
  const { texto: conteudo, ...dados } = d;
  return { ...dados, trecho: conteudo.slice(0, 400) };
}
function pendente<T>() {
  let resolver!: (valor: T) => void;
  const promessa = new Promise<T>((resolve) => { resolver = resolve; });
  return { promessa, resolver };
}
beforeEach(() => { vi.mocked(invoke).mockReset(); });

describe("indexação explícita no leitor", () => {
  function respostas(indexacao: Promise<DocumentoIndexado> = Promise.resolve(documento)) {
    vi.mocked(invoke).mockImplementation((comando) => {
      if (comando === "ler_documento") return Promise.resolve(leitura);
      if (comando === "indexar_documento") return indexacao;
      if (comando === "entidades_documento") return Promise.resolve([entidade]);
      if (comando === "cancelar_documento") return Promise.resolve(true);
      return Promise.reject(new Error(`Comando sintético inesperado: ${comando}`));
    });
  }
  async function ler() {
    await userEvent.click(screen.getByRole("button", { name: "Ler documento" }));
    await screen.findByRole("region", { name: "Texto documental" });
  }
  it("só indexa após comando explícito; envia IDs, não texto/hash/caminho do frontend", async () => {
    respostas();
    render(<LeitorDocumento caso={caso} extracaoId={documento.extracao_id} arquivo={arquivo} />);
    expect(invoke).not.toHaveBeenCalled();
    await ler();
    expect(vi.mocked(invoke).mock.calls.map(([comando]) => comando)).toEqual(["ler_documento"]);
    await userEvent.click(screen.getByRole("button", { name: "Indexar documento" }));
    expect(invoke).toHaveBeenCalledWith("indexar_documento", { arquivoId: arquivo.id, casoId: caso.id });
    expect(await screen.findByRole("region", { name: "Análise documental armazenada" })).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Texto documental armazenado" })).not.toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.some(([comando]) => comando === "adicionar_evidencia" || comando === "gerar_relatorio")).toBe(false);
  });
  it("não duplica indexação e descreve cancelamento solicitado sem alegar reversão concluída", async () => {
    const trabalho = pendente<DocumentoIndexado>(); respostas(trabalho.promessa);
    render(<LeitorDocumento caso={caso} extracaoId={documento.extracao_id} arquivo={arquivo} />);
    await ler(); await userEvent.click(screen.getByRole("button", { name: "Indexar documento" }));
    expect(screen.getByRole("button", { name: "Indexar documento" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Conferir e reler" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Solicitar cancelamento" }));
    expect(invoke).toHaveBeenCalledWith("cancelar_documento", { casoId: caso.id });
    expect(await screen.findByRole("status")).toHaveTextContent("Cancelamento solicitado. Aguardando o encerramento cooperativo");
    expect(screen.queryByText("Indexação cancelada")).not.toBeInTheDocument();
    await act(async () => trabalho.resolver(documento));
    expect(screen.getByRole("button", { name: "Indexar documento" })).toBeEnabled();
  });
  it("recusa derivado recebido para outra origem/página", async () => {
    respostas(Promise.resolve({ ...documento, arquivo_id: 999 }));
    render(<LeitorDocumento caso={caso} extracaoId={documento.extracao_id} arquivo={arquivo} />);
    await ler(); await userEvent.click(screen.getByRole("button", { name: "Indexar documento" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("não corresponde à leitura selecionada");
    expect(screen.queryByRole("region", { name: "Análise documental armazenada" })).not.toBeInTheDocument();
  });
  it("ignora conclusão tardia da indexação após trocar de caso", async () => {
    const trabalho = pendente<DocumentoIndexado>(); respostas(trabalho.promessa);
    const tela = render(<LeitorDocumento caso={caso} extracaoId={documento.extracao_id} arquivo={arquivo} />);
    await ler(); await userEvent.click(screen.getByRole("button", { name: "Indexar documento" }));
    tela.rerender(<LeitorDocumento caso={{ ...caso, id: "outro-caso-sintetico" }} extracaoId="outra-extracao" arquivo={arquivo} />);
    await act(async () => trabalho.resolver(documento));
    expect(screen.queryByRole("region", { name: "Análise documental armazenada" })).not.toBeInTheDocument();
  });
  it("não mostra cancelamento pendente depois de a indexação ter terminado", async () => {
    const trabalho = pendente<DocumentoIndexado>(); const cancelamento = pendente<boolean>();
    respostas(trabalho.promessa);
    vi.mocked(invoke).mockImplementation((comando) => {
      if (comando === "ler_documento") return Promise.resolve(leitura);
      if (comando === "indexar_documento") return trabalho.promessa;
      if (comando === "cancelar_documento") return cancelamento.promessa;
      return Promise.resolve([]);
    });
    render(<LeitorDocumento caso={caso} extracaoId={documento.extracao_id} arquivo={arquivo} />);
    await ler(); await userEvent.click(screen.getByRole("button", { name: "Indexar documento" }));
    await userEvent.click(screen.getByRole("button", { name: "Solicitar cancelamento" }));
    await act(async () => trabalho.resolver(documento));
    await act(async () => cancelamento.resolver(true));
    expect(screen.queryByText(/Cancelamento solicitado\. Aguardando/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Indexar documento" })).toBeEnabled();
  });
  it("fila PDF só avança após cada retorno real e permite pausar/continuar sem reiniciar páginas concluídas", async () => {
    const pdf = { ...arquivo, caminho: "documentos/PDF-SINTETICO.pdf", mime: "application/pdf" };
    const pagina1 = { ...documento, caminho: pdf.caminho, pagina: 1, total_paginas: 2, motor_id: "pdf_nativo_lopdf" };
    const pagina2 = { ...pagina1, id: 42, pagina: 2, texto: "Segunda página SINTÉTICA.", sha256_texto: "ef".repeat(32) };
    const primeira = pendente<DocumentoIndexado>();
    vi.mocked(invoke).mockImplementation((comando, argumentos) => {
      if (comando === "ler_documento") return Promise.resolve({ ...leitura, texto: { ...leitura.texto!, caminho: pdf.caminho, pagina: 1, total_paginas: 2, motor_id: "pdf_nativo_lopdf" } });
      if (comando === "indexar_documento") return argumentos && "pagina" in argumentos && argumentos.pagina === 1 ? primeira.promessa : Promise.resolve(pagina2);
      return Promise.resolve([]);
    });
    render(<LeitorDocumento caso={caso} extracaoId={documento.extracao_id} arquivo={pdf} />);
    await ler(); await userEvent.click(screen.getByRole("button", { name: "Indexar todas as páginas" }));
    expect(screen.getByText(/0 de 2 página\(s\) confirmada\(s\)/)).toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "indexar_documento")).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "Pausar após esta página" }));
    await act(async () => primeira.resolver(pagina1));
    expect(screen.getByText(/1 de 2 página\(s\) confirmada\(s\).*Fila pausada/)).toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "indexar_documento")).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "Continuar fila" }));
    expect(await screen.findByText(/2 de 2 página\(s\) confirmada\(s\).*Fila concluída/)).toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "indexar_documento").map(([, args]) => args && "pagina" in args ? args.pagina : null)).toEqual([1, 2]);
    expect(screen.getByRole("region", { name: "Texto documental armazenado" }).textContent).toBe(pagina2.texto);
  });
  it("sair da tela durante fila PDF impede início automático da página seguinte", async () => {
    const primeira = pendente<DocumentoIndexado>();
    const pdf = { ...arquivo, caminho: "PDF-SINTETICO.pdf" };
    vi.mocked(invoke).mockImplementation((comando) => comando === "ler_documento"
      ? Promise.resolve({ ...leitura, texto: { ...leitura.texto!, caminho: pdf.caminho, pagina: 1, total_paginas: 2 } })
      : primeira.promessa);
    const tela = render(<LeitorDocumento caso={caso} extracaoId={documento.extracao_id} arquivo={pdf} />);
    await ler(); await userEvent.click(screen.getByRole("button", { name: "Indexar todas as páginas" }));
    tela.unmount();
    await act(async () => primeira.resolver({ ...documento, caminho: pdf.caminho, pagina: 1, total_paginas: 2 }));
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "indexar_documento")).toHaveLength(1);
  });
});

describe("derivados e candidatos separados da evidência", () => {
  it("exibe regiões OCR paginadas com coordenadas e ausência honesta de confiança", async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const ocr = {
      ...documento, pagina: 1, total_paginas: 1, motor_id: "windows_media_ocr",
      motor_versao: "1.0.0+pt-BR", largura_px: 1200, altura_px: 800,
      regioes: [{ id: 91, documento_id: documento.id, ordem: 0, inicio_utf8: 0,
        fim_utf8: 10, texto: "SINTÉTICO", x: 100, y: 80, largura: 240, altura: 52, confianca: null }],
    } as DocumentoIndexado;
    render(<PainelAnaliseDocumento caso={caso} documento={ocr} />);
    const regioes = screen.getByRole("region", { name: "Regiões OCR" });
    expect(within(regioes).getByText("SINTÉTICO")).toBeInTheDocument();
    expect(within(regioes).getByText(/x 100 · y 80 · 240 × 52px/)).toBeInTheDocument();
    expect(within(regioes).getByText(/Confiança: não fornecida pelo motor/)).toBeInTheDocument();
    expect(screen.getByText(/1200 × 800 px · 1 região/)).toBeInTheDocument();
  });
  it("exibe texto inerte, proveniência persistida e nunca selo de conferência da origem atual", async () => {
    const conteudo = '<script>window.__siedCacheExecutou=true</script><img src="https://nao-acessar.invalid/">';
    vi.mocked(invoke).mockResolvedValue([]);
    render(<PainelAnaliseDocumento caso={caso} documento={{ ...documento, texto: conteudo }} />);
    const regiao = screen.getByRole("region", { name: "Texto documental armazenado" });
    expect(regiao.textContent).toBe(conteudo); expect(regiao.querySelector("script,img,a")).toBeNull();
    expect(screen.getByText("DERIVADO ARMAZENADO")).toBeInTheDocument();
    expect(screen.queryByText("ORIGEM CONFERIDA")).not.toBeInTheDocument();
    expect(screen.getByText(/não relê a origem atual/)).toBeInTheDocument();
    expect(await screen.findByText("Nenhum candidato literal detectado")).toBeInTheDocument();
  });
  it("usa offsets UTF-8 reais após acentos e emoji; confirmação é da ocorrência literal", async () => {
    vi.mocked(invoke).mockResolvedValue([entidade]);
    render(<PainelAnaliseDocumento caso={caso} documento={documento} />);
    const candidato = await screen.findByRole("article", { name: "Candidato 73" });
    expect(within(candidato).getByText(literal)).toBeInTheDocument();
    expect(within(candidato).getByText(/Posição em bytes UTF-8/)).toHaveTextContent(`[${inicio}, ${entidade.fim_utf8})`);
    expect(screen.getByText(/não identificar uma pessoa/)).toBeInTheDocument();
  });
  it("recusa candidatos cujo literal não corresponde à faixa UTF-8 da origem armazenada", async () => {
    vi.mocked(invoke).mockResolvedValue([{ ...entidade, inicio_utf8: inicio - 1 }]);
    render(<PainelAnaliseDocumento caso={caso} documento={documento} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("não correspondem ao texto armazenado");
    expect(screen.queryByRole("button", { name: "Salvar revisão" })).not.toBeInTheDocument();
  });
  it("recusa documento de outro caso antes de qualquer consulta", () => {
    render(<PainelAnaliseDocumento caso={{ ...caso, id: "outro-caso-sintetico" }} documento={documento} />);
    expect(screen.getByRole("alert")).toHaveTextContent("não pertence ao caso atual");
    expect(invoke).not.toHaveBeenCalled();
  });
  it("revisão explícita envia somente candidato/decisão/nota e preserva texto/literal", async () => {
    vi.mocked(invoke).mockImplementation((comando) => Promise.resolve(comando === "entidades_documento" ? [entidade] : {
      ...entidade, estado: "confirmado", nota: "SINTÉTICO: conferi somente o literal.", revista_em: "2026-10-02T12:05:00Z", revisor: "Analista SINTÉTICO",
    }));
    render(<PainelAnaliseDocumento caso={caso} documento={documento} />);
    const candidato = await screen.findByRole("article", { name: "Candidato 73" });
    await userEvent.selectOptions(within(candidato).getByRole("combobox", { name: "Decisão da revisão" }), "confirmado");
    await userEvent.type(within(candidato).getByRole("textbox", { name: "Nota do analista" }), "SINTÉTICO: conferi somente o literal.");
    await userEvent.click(within(candidato).getByRole("button", { name: "Salvar revisão" }));
    expect(invoke).toHaveBeenCalledWith("revisar_entidade", { casoId: caso.id, entidadeId: entidade.id, estado: "confirmado", nota: "SINTÉTICO: conferi somente o literal." });
    expect(await within(candidato).findByRole("status")).toHaveTextContent("texto original não foi alterado");
    expect(screen.getByRole("region", { name: "Texto documental armazenado" }).textContent).toBe(texto);
    expect(vi.mocked(invoke).mock.calls.some(([comando]) => comando === "adicionar_evidencia" || comando === "gerar_relatorio")).toBe(false);
  });
  it("não aplica revisão cujo retorno muda os offsets/regra/literal", async () => {
    vi.mocked(invoke).mockImplementation((comando) => Promise.resolve(comando === "entidades_documento" ? [entidade] : { ...entidade, estado: "confirmado", regra_versao: "outra-regra" }));
    render(<PainelAnaliseDocumento caso={caso} documento={documento} />);
    const candidato = await screen.findByRole("article", { name: "Candidato 73" });
    await userEvent.selectOptions(within(candidato).getByRole("combobox"), "confirmado");
    await userEvent.click(within(candidato).getByRole("button", { name: "Salvar revisão" }));
    expect(await within(candidato).findByRole("alert")).toHaveTextContent("não corresponde ao candidato selecionado");
    expect(within(candidato).getByText("Não revisado", { selector: "span" })).toBeInTheDocument();
  });
  it("lista limitada tem aviso e não se apresenta como pesquisa exaustiva", async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    render(<PainelAnaliseDocumento caso={caso} documento={{ ...documento, entidades_limitadas: true }} />);
    expect(screen.getByText(/Esta lista não é exaustiva/)).toBeInTheDocument();
    await screen.findByText("Nenhum candidato literal detectado");
  });
  it("histórico é carregado explicitamente e usa paginação limitada, mantendo notas como texto", async () => {
    const historico = { total: 21, itens: [{ id: 81, entidade_id: entidade.id, estado: "inconclusivo", nota: "<script>SINTÉTICO</script>", revista_em: "2026-10-06T12:00:00Z", revisor: "Analista SINTÉTICO", maquina: "ESTAÇÃO-SINTÉTICA" }] };
    vi.mocked(invoke).mockImplementation((comando) => Promise.resolve(comando === "entidades_documento" ? [entidade] : historico));
    render(<PainelAnaliseDocumento caso={caso} documento={documento} />);
    const candidato = await screen.findByRole("article", { name: "Candidato 73" });
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "revisoes_entidade")).toHaveLength(0);
    await userEvent.click(within(candidato).getByRole("button", { name: "Ver histórico de revisões" }));
    expect(invoke).toHaveBeenCalledWith("revisoes_entidade", { casoId: caso.id, entidadeId: entidade.id, deslocamento: 0, limite: 20 });
    const revisoes = await screen.findByRole("list", { name: "Revisões registradas" });
    expect(revisoes).toHaveTextContent("<script>SINTÉTICO</script>"); expect(revisoes.querySelector("script")).toBeNull();
    await userEvent.click(within(candidato).getByRole("button", { name: "Próxima" }));
    expect(invoke).toHaveBeenCalledWith("revisoes_entidade", { casoId: caso.id, entidadeId: entidade.id, deslocamento: 20, limite: 20 });
  });
});

describe("busca documental paginada", () => {
  function responderBusca(pagina: Pagina<AchadoDocumento> = { total: 1, itens: [achado()] }) {
    vi.mocked(invoke).mockImplementation((comando) => {
      if (comando === "buscar_documentos") return Promise.resolve(pagina);
      if (comando === "obter_documento") return Promise.resolve(documento);
      if (comando === "entidades_documento") return Promise.resolve([entidade]);
      return Promise.reject(new Error(`Comando sintético inesperado: ${comando}`));
    });
  }
  it("não busca termo vazio; usa IDs do caso e paginação limitada na busca explícita", async () => {
    responderBusca(); render(<BuscaDocumentos caso={caso} />);
    expect(invoke).not.toHaveBeenCalled();
    await userEvent.type(screen.getByRole("searchbox", { name: "Termo documental" }), "SINTÉTICO");
    await userEvent.click(screen.getByRole("button", { name: "Buscar documentos" }));
    expect(invoke).toHaveBeenCalledWith("buscar_documentos", { casoId: caso.id, termo: "SINTÉTICO", deslocamento: 0, limite: 20 });
    expect(await screen.findByRole("list", { name: "Resultados documentais" })).toHaveTextContent("NOTA-SINTETICA.txt");
    expect(screen.queryByRole("region", { name: "Texto documental armazenado" })).not.toBeInTheDocument();
  });
  it("abre somente derivado selecionado com fonte/hash/candidatos e aviso de origem não relida", async () => {
    responderBusca(); render(<BuscaDocumentos caso={caso} termo="SINTÉTICO" />);
    await userEvent.click(await screen.findByRole("button", { name: "Abrir texto indexado" }));
    expect(invoke).toHaveBeenCalledWith("obter_documento", { casoId: caso.id, documentoId: documento.id });
    expect(await screen.findByRole("region", { name: "Texto documental armazenado" })).toHaveTextContent(texto);
    expect(screen.getByText("DERIVADO ARMAZENADO")).toBeInTheDocument();
    expect(screen.queryByText("ORIGEM CONFERIDA")).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Fechar documento indexado" }));
    expect(screen.queryByRole("region", { name: "Texto documental armazenado" })).not.toBeInTheDocument();
  });
  it("paginação consulta páginas reais de 20 e descarta o documento aberto", async () => {
    responderBusca({ total: 21, itens: [achado()] }); render(<BuscaDocumentos caso={caso} termo="SINTÉTICO" />);
    await screen.findByRole("list", { name: "Resultados documentais" });
    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    expect(invoke).toHaveBeenCalledWith("buscar_documentos", { casoId: caso.id, termo: "SINTÉTICO", deslocamento: 20, limite: 20 });
  });
  it("recusa resultados de outro caso e não oferece abertura", async () => {
    responderBusca({ total: 1, itens: [{ ...achado(), caso_id: "outro-caso-sintetico" }] });
    render(<BuscaDocumentos caso={caso} termo="SINTÉTICO" />);
    expect(await screen.findByRole("alert")).toHaveTextContent("não correspondem ao caso");
    expect(screen.queryByRole("button", { name: "Abrir texto indexado" })).not.toBeInTheDocument();
  });
  it("ignora respostas tardias de busca e abertura após troca de caso", async () => {
    const busca = pendente<Pagina<AchadoDocumento>>();
    vi.mocked(invoke).mockReturnValue(busca.promessa);
    const tela = render(<BuscaDocumentos caso={caso} termo="SINTÉTICO" />);
    tela.rerender(<BuscaDocumentos caso={{ ...caso, id: "outro-caso-sintetico" }} />);
    await act(async () => busca.resolver({ total: 1, itens: [achado()] }));
    expect(screen.queryByRole("list", { name: "Resultados documentais" })).not.toBeInTheDocument();
    expect(screen.queryByText("NOTA-SINTETICA.txt")).not.toBeInTheDocument();
  });
  it("recusa texto completo que não corresponde ao achado selecionado", async () => {
    vi.mocked(invoke).mockImplementation((comando) => Promise.resolve(comando === "buscar_documentos" ? { total: 1, itens: [achado()] } : { ...documento, sha256_texto: "ef".repeat(32) }));
    render(<BuscaDocumentos caso={caso} termo="SINTÉTICO" />);
    await userEvent.click(await screen.findByRole("button", { name: "Abrir texto indexado" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("não corresponde ao resultado selecionado");
    expect(screen.queryByRole("region", { name: "Texto documental armazenado" })).not.toBeInTheDocument();
  });
});
