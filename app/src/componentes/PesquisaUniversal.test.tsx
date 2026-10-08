import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Caso, DocumentoIndexado, FiltroBusca, ItemUniversal, PedidoUniversal, ResultadoUniversal } from "../api/cliente";
import { interpretarConsultaLocal, PesquisaUniversal } from "./PesquisaUniversal";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const caso = { id: "caso-sintetico", fuso_horario: "America/Sao_Paulo" } as Caso;
const pedido: PedidoUniversal = { filtro: { texto: "SINTÉTICO" }, mecanismos: ["registros", "arquivos", "documentos", "entidades"] };
const registro: ItemUniversal = {
  chave: "artefato:9", titulo: "Conversa SINTÉTICA", trecho: "Mensagem SINTÉTICA", tipo: "mensagem",
  extracao_id: "extracao-sintetica", artefato_id: 9, arquivo_id: null, conversa_id: 3, data_hora_ms: null,
  hash_origem: "ab".repeat(32), score_ranking: 0.25,
  sinais: [{ mecanismo: "registros", motivo: "Correspondência SINTÉTICA no conteúdo.", posicao: 1, contribuicao_rrf: 1 / 61, motor: "fts5-sintetico", documento_id: null, pagina: null, hash_derivado: null }],
};
const arquivo: ItemUniversal = {
  ...registro, chave: "arquivo:27", titulo: "documentos/NOTA-SINTETICA.txt", trecho: "Texto documental SINTÉTICO.", tipo: "documento",
  artefato_id: null, arquivo_id: 27, conversa_id: null,
  sinais: [{ mecanismo: "documentos", motivo: "Texto derivado armazenado SINTÉTICO; origem atual não relida.", posicao: 1, contribuicao_rrf: 1 / 61, motor: "texto_utf8_nativo@0.1.0", documento_id: 41, pagina: null, hash_derivado: "cd".repeat(32) }],
};
const documento: DocumentoIndexado = {
  id: 41, caso_id: caso.id, extracao_id: arquivo.extracao_id, arquivo_id: 27, caminho: arquivo.titulo,
  sha256_origem: arquivo.hash_origem!, sha256_texto: "cd".repeat(32), texto: arquivo.trecho, pagina: null, total_paginas: null,
  motor_id: "texto_utf8_nativo", motor_versao: "0.1.0", executado_em: "2026-10-06T12:00:00Z", entidades_limitadas: false, entidades_motor_versao: "sintetico-1",
};
function resposta(configuracao = pedido, mudancas: Partial<ResultadoUniversal> = {}): ResultadoUniversal {
  return { caso_id: caso.id, pedido: configuracao, algoritmo: "rrf-k60-v1-equal-mechanisms", executada_em: "2026-10-06T12:00:00Z", itens: [registro, arquivo], recuperados_na_janela: 2, janela_limitada: false, avisos: ["Scores não expressam confiança pericial. Dados exclusivamente SINTÉTICOS."], ...mudancas };
}
function pendente<T>() {
  let resolver!: (valor: T) => void; let rejeitar!: (erro: unknown) => void;
  const promessa = new Promise<T>((resolve, reject) => { resolver = resolve; rejeitar = reject; });
  return { promessa, resolver, rejeitar };
}
function elemento(c = caso, filtro?: FiltroBusca) {
  return <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><MemoryRouter><PesquisaUniversal caso={c} filtro={filtro} /></MemoryRouter></QueryClientProvider>;
}
beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((comando, argumentos) => {
    if (comando === "listar_extracoes") return Promise.resolve([]);
    if (comando === "pesquisar_universal") return Promise.resolve(resposta(argumentos && "pedido" in argumentos ? argumentos.pedido as PedidoUniversal : pedido));
    if (comando === "obter_documento") return Promise.resolve(documento);
    if (comando === "entidades_documento") return Promise.resolve([]);
    if (comando === "cancelar_descoberta") return Promise.resolve(true);
    return Promise.reject(new Error("Comando não previsto na fixture SINTÉTICA."));
  });
});
async function pesquisar() {
  await userEvent.type(screen.getByRole("searchbox", { name: "Termo da busca integrada" }), pedido.filtro.texto);
  await userEvent.click(screen.getByRole("button", { name: "Pesquisar índices locais" }));
}

describe("consulta por regra local sem modelo", () => {
  it("reconhece exclusivamente expressão entre aspas e fonte declarada sem interpretar semântica", () => {
    expect(interpretarConsultaLocal('documentos contendo "SINTÉTICO 👨‍👩‍👧‍👦"')).toEqual({ termo: "SINTÉTICO 👨‍👩‍👧‍👦", mecanismos: ["documentos"] });
    expect(interpretarConsultaLocal('buscar mensagens com "termo"')).toEqual({ termo: "termo", mecanismos: ["registros"] });
    expect(() => interpretarConsultaLocal("descubra quem cometeu um crime")).toThrow(/Formato não reconhecido/);
    expect(() => interpretarConsultaLocal('documentos suspeitos de ontem contendo "termo"')).toThrow();
  });
  it("prepara formulário e mostra prévia, mas não executa automaticamente", async () => {
    render(elemento());
    await userEvent.click(screen.getByText("Preparar consulta por regra local · sem modelo"));
    await userEvent.type(screen.getByRole("textbox", { name: "Consulta por regra local" }), 'documentos contendo "SINTÉTICO"');
    await userEvent.click(screen.getByRole("button", { name: "Preparar consulta" }));
    expect(screen.getByRole("searchbox", { name: "Termo da busca integrada" })).toHaveValue("SINTÉTICO");
    expect(screen.getByRole("checkbox", { name: "Textos documentais indexados" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Registros / mensagens" })).not.toBeChecked();
    expect(screen.getByText(/Confira e execute explicitamente/)).toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.some(([comando]) => comando === "pesquisar_universal")).toBe(false);
  });
});
describe("recuperação integrada por mecanismos reais", () => {
  it("não consulta ao digitar e não envia motores indisponíveis", async () => {
    render(elemento());
    await userEvent.type(screen.getByRole("searchbox", { name: "Termo da busca integrada" }), "SINTÉTICO");
    expect(vi.mocked(invoke).mock.calls.some(([comando]) => comando === "pesquisar_universal")).toBe(false);
    expect(screen.getByRole("checkbox", { name: "OCR · motor indisponível" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Pesquisar índices locais" }));
    expect(invoke).toHaveBeenCalledWith("pesquisar_universal", { casoId: caso.id, operacaoId: expect.any(String), pedido, deslocamento: 0, limite: 20 });
    expect(await screen.findByRole("list", { name: "Resultados integrados" })).toBeInTheDocument();
  });
  it("apresenta ranking como posição combinada, explicação e navegação à origem preservadas", async () => {
    render(elemento()); await pesquisar();
    const resultados = await screen.findByRole("list", { name: "Resultados integrados" });
    expect(within(resultados).getByRole("link", { name: "Conversa SINTÉTICA" })).toHaveAttribute("href", `/casos/${caso.id}/conversas/3?registro=9`);
    expect(within(resultados).getByRole("link", { name: arquivo.titulo })).toHaveAttribute("href", `/casos/${caso.id}/arquivos?extracao=extracao-sintetica&arquivo=27`);
    expect(within(resultados).getAllByText("Ranking 0.2500 · não é confiança")).toHaveLength(2);
    await userEvent.click(within(resultados).getAllByText("Por que este item apareceu?")[0]!);
    expect(within(resultados).getByText("Correspondência SINTÉTICA no conteúdo.")).toBeVisible();
    expect(screen.getByText(/Não é uma contagem global do caso/)).toBeInTheDocument();
  });
  it("filtra sinais somente na página recuperada, sem criar nova consulta ou contagem global falsa", async () => {
    render(elemento()); await pesquisar(); await screen.findByRole("list", { name: "Resultados integrados" });
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Mecanismo dos resultados desta página" }), "documentos");
    const resultados = screen.getByRole("list", { name: "Resultados integrados" });
    expect(within(resultados).queryByText(registro.titulo)).not.toBeInTheDocument();
    expect(within(resultados).getByText(arquivo.titulo)).toBeInTheDocument();
    expect(screen.getByText("1 item(s) exibido(s) nesta página")).toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "pesquisar_universal")).toHaveLength(1);
  });
  it("abre derivado pelo ID/hash do sinal, não relê origem nem cria evidência/relatório", async () => {
    render(elemento()); await pesquisar(); await screen.findByRole("list", { name: "Resultados integrados" });
    await userEvent.click(screen.getAllByText("Por que este item apareceu?")[1]!);
    await userEvent.click(screen.getByRole("button", { name: "Abrir derivado #41" }));
    expect(invoke).toHaveBeenCalledWith("obter_documento", { casoId: caso.id, documentoId: 41 });
    expect(await screen.findByRole("region", { name: "Texto documental armazenado" }).then((r) => r.textContent)).toBe(documento.texto);
    expect(screen.queryByText("ORIGEM CONFERIDA")).not.toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.some(([comando]) => comando === "ler_documento" || comando === "adicionar_evidencia" || comando === "gerar_relatorio")).toBe(false);
  });
  it("preserva app, extração e timestamps exatos recebidos no filtro inicial", async () => {
    const filtro = { texto: "SINTÉTICO", extracao_id: "extracao-sintetica", app: "whatsapp", de_ms: 1791289234567, ate_ms: 1791375634567 };
    render(elemento(caso, filtro)); await userEvent.click(screen.getByRole("button", { name: "Pesquisar índices locais" }));
    expect(invoke).toHaveBeenCalledWith("pesquisar_universal", { casoId: caso.id, operacaoId: expect.any(String), pedido: { ...pedido, filtro }, deslocamento: 0, limite: 20 });
  });
  it("janela limitada mostra limite real e paginação consulta novamente com offset correto", async () => {
    vi.mocked(invoke).mockImplementation((comando, argumentos) => Promise.resolve(comando === "listar_extracoes" ? [] : resposta(argumentos && "pedido" in argumentos ? argumentos.pedido as PedidoUniversal : pedido, { recuperados_na_janela: 21, janela_limitada: true })));
    render(elemento()); await pesquisar(); await screen.findByRole("list", { name: "Resultados integrados" });
    expect(screen.getByText("JANELA LIMITADA")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    expect(invoke).toHaveBeenCalledWith("pesquisar_universal", { casoId: caso.id, operacaoId: expect.any(String), pedido, deslocamento: 20, limite: 20 });
  });
  it("texto/títulos/motivos permanecem inertes, sem HTML, links externos ou comandos", async () => {
    const perigoso = '<script>SINTÉTICO</script><img src="https://nao-acessar.invalid/">';
    vi.mocked(invoke).mockImplementation((comando) => Promise.resolve(comando === "listar_extracoes" ? [] : resposta(pedido, { itens: [{ ...registro, titulo: perigoso, trecho: perigoso }] })));
    render(elemento()); await pesquisar();
    const resultados = await screen.findByRole("list", { name: "Resultados integrados" });
    expect(resultados.querySelector("script,img")).toBeNull();
    expect(within(resultados).getByRole("link").textContent).toBe(perigoso);
  });
  it("recusa retorno de outro caso/filtros antes da apresentação", async () => {
    vi.mocked(invoke).mockImplementation((comando) => Promise.resolve(comando === "listar_extracoes" ? [] : resposta(pedido, { caso_id: "outro-caso-sintetico" })));
    render(elemento()); await pesquisar();
    expect(await screen.findByRole("alert")).toHaveTextContent("não corresponde ao caso");
    expect(screen.queryByRole("list", { name: "Resultados integrados" })).not.toBeInTheDocument();
  });
  it("sinal de cancelamento não libera o formulário nem se apresenta como conclusão", async () => {
    const trabalho = pendente<ResultadoUniversal>();
    vi.mocked(invoke).mockImplementation((comando) => comando === "listar_extracoes" ? Promise.resolve([]) : comando === "cancelar_descoberta" ? Promise.resolve(true) : trabalho.promessa);
    render(elemento()); await pesquisar();
    const chamada = vi.mocked(invoke).mock.calls.find(([comando]) => comando === "pesquisar_universal")!;
    await userEvent.click(screen.getByRole("button", { name: "Cancelar consulta integrada" }));
    expect(invoke).toHaveBeenCalledWith("cancelar_descoberta", { operacaoId: (chamada[1] as { operacaoId: string }).operacaoId });
    expect(screen.getByRole("button", { name: "Pesquisar índices locais" })).toBeDisabled();
    expect(screen.getByText(/Aguardando a resposta final/)).toBeInTheDocument();
    await act(async () => trabalho.rejeitar({ codigo: "descoberta_cancelada", mensagem: "Interrupção SINTÉTICA", campos: [] }));
    expect(screen.getByRole("alert")).toHaveTextContent("cancelada pelo backend");
    expect(screen.getByRole("button", { name: "Pesquisar índices locais" })).toBeEnabled();
  });
  it("mudança de caso ignora resposta anterior e cancela somente seu ID de operação", async () => {
    const trabalho = pendente<ResultadoUniversal>();
    vi.mocked(invoke).mockImplementation((comando) => comando === "listar_extracoes" ? Promise.resolve([]) : comando === "cancelar_descoberta" ? Promise.resolve(true) : trabalho.promessa);
    const tela = render(elemento()); await pesquisar();
    tela.rerender(elemento({ ...caso, id: "outro-caso-sintetico" }));
    await act(async () => trabalho.resolver(resposta()));
    expect(screen.queryByRole("list", { name: "Resultados integrados" })).not.toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "cancelar_descoberta")).toHaveLength(1);
  });
});
