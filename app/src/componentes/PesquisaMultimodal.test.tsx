import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Caso, ResultadoMultimodal } from "../api/cliente";
import { PesquisaMultimodal } from "./PesquisaMultimodal";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const caso = { id: "caso-multimodal", fuso_horario: "America/Sao_Paulo" } as Caso;
const resposta: ResultadoMultimodal = {
  caso_id: caso.id,
  pedido: { filtro: { texto: "SINTÉTICO" }, mecanismos: ["mensagens", "documentos"] },
  algoritmo: "rrf-multimodal-local-v1-equal-mechanisms",
  executada_em: "2026-10-09T12:00:00Z",
  itens: { total: 1, itens: [{ chave: "arquivo:7", origem_tipo: "documento", origem_id: 3, extracao_id: "ext-1", arquivo_id: 7, titulo: "nota-sintetica.pdf", trecho: "SINTÉTICO", inicio_ms: null, fim_ms: null, hash_origem: "ab".repeat(32), score_ranking: 0.42, sinais: [{ mecanismo: "documentos", motivo: "Correspondência no texto documental.", posicao: 1, contribuicao_rrf: 1 / 61, motor: "texto", motor_versao: "1", hash_derivado: "cd".repeat(32) }] }] },
  recuperados_na_janela: 1,
  janela_limitada: false,
  avisos: ["Ranking não é confiança."]
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((comando, argumentos) => {
    if (comando === "listar_extracoes") return Promise.resolve([]);
    if (comando === "pesquisar_multimodal") {
      const pedido = argumentos && "pedido" in argumentos ? argumentos.pedido as ResultadoMultimodal["pedido"] : resposta.pedido;
      return Promise.resolve({ ...resposta, pedido });
    }
    if (comando === "cancelar_descoberta") return Promise.resolve(true);
    return Promise.reject(new Error("Comando não previsto."));
  });
});

function tela() {
  return <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><MemoryRouter><PesquisaMultimodal caso={caso} /></MemoryRouter></QueryClientProvider>;
}

describe("pesquisa multimodal", () => {
  it("executa mecanismos escolhidos e explica a origem do candidato", async () => {
    render(tela());
    await userEvent.type(screen.getByRole("searchbox", { name: "Termo da pesquisa multimodal" }), "SINTÉTICO");
    await userEvent.click(screen.getByRole("checkbox", { name: "OCR" }));
    await userEvent.click(screen.getByRole("button", { name: "Pesquisar" }));
    expect(await screen.findByRole("list", { name: "Resultados multimodais" })).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("pesquisar_multimodal", expect.objectContaining({ pedido: expect.objectContaining({ filtro: { texto: "SINTÉTICO" }, mecanismos: expect.arrayContaining(["mensagens", "documentos"]) }) }));
    await userEvent.click(screen.getByText("Por que este resultado apareceu?"));
    expect(screen.getByText("Correspondência no texto documental.")).toBeVisible();
    expect(screen.getByRole("link", { name: "nota-sintetica.pdf" })).toHaveAttribute("href", "/casos/caso-multimodal/arquivos?extracao=ext-1&arquivo=7");
  });

  it("filtra sinais sem executar nova consulta", async () => {
    render(tela());
    await userEvent.type(screen.getByRole("searchbox", { name: "Termo da pesquisa multimodal" }), "SINTÉTICO");
    await userEvent.click(screen.getByRole("button", { name: "Pesquisar" }));
    await screen.findByRole("list", { name: "Resultados multimodais" });
    const chamadas = vi.mocked(invoke).mock.calls.length;
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Filtrar mecanismo multimodal" }), "visao");
    expect(within(screen.getByRole("region", { name: "Pesquisa multimodal local" })).getByText("0 exibido(s)")).toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls).toHaveLength(chamadas);
  });
});
