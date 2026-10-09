import { invoke } from "@tauri-apps/api/core";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Caso, ResultadoMultimodal } from "../api/cliente";
import { AssistenteLocal, interpretarConsultaAssistente } from "./AssistenteLocal";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const caso = { id: "caso-assistente", fuso_horario: "America/Sao_Paulo" } as Caso;
const base: ResultadoMultimodal = { caso_id: caso.id, pedido: { filtro: { texto: "55119999" }, mecanismos: ["mensagens", "entidades"] }, algoritmo: "rrf-multimodal-local-v1-equal-mechanisms", executada_em: "2026-10-09T12:00:00Z", itens: { total: 1, itens: [{ chave: "arquivo:4", origem_tipo: "entidade", origem_id: 8, extracao_id: "ext-1", arquivo_id: 4, titulo: "mensagens/registro.db", trecho: "55119999", inicio_ms: null, fim_ms: null, hash_origem: "ab".repeat(32), score_ranking: 0.5, sinais: [{ mecanismo: "entidades", motivo: "Telefone literal candidato encontrado.", posicao: 1, contribuicao_rrf: 1 / 61, motor: "entidades-documentais", motor_versao: "1", hash_derivado: null }] }] }, recuperados_na_janela: 1, janela_limitada: false, avisos: [] };

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((comando, argumentos) => {
    if (comando === "pesquisar_multimodal") return Promise.resolve({ ...base, pedido: argumentos && "pedido" in argumentos ? argumentos.pedido as ResultadoMultimodal["pedido"] : base.pedido });
    if (comando === "cancelar_descoberta") return Promise.resolve(true);
    return Promise.reject(new Error("Comando não previsto."));
  });
});

describe("assistente local explicável", () => {
  it("interpreta consulta semântica por regras e conserva fontes", () => {
    const consulta = interpretarConsultaAssistente('quais documentos mencionam "55119999"?');
    expect(consulta.termo).toBe("55119999");
    expect(consulta.mecanismos).toEqual(expect.arrayContaining(["documentos", "entidades"]));
  });

  it("executa recuperação local e mostra resposta, origem e mecanismo", async () => {
    render(<MemoryRouter><AssistenteLocal caso={caso} /></MemoryRouter>);
    await userEvent.type(screen.getByRole("textbox", { name: "Pergunta ao assistente local" }), 'quais documentos mencionam "55119999"?');
    await userEvent.click(screen.getByRole("button", { name: "Consultar caso" }));
    const fontes = await screen.findByRole("list", { name: "Fontes da resposta do assistente" });
    expect(within(fontes).getByRole("link", { name: "mensagens/registro.db" })).toHaveAttribute("href", "/casos/caso-assistente/arquivos?extracao=ext-1&arquivo=4");
    expect(screen.getByText("SUPORTE ENCONTRADO")).toBeInTheDocument();
    await userEvent.click(screen.getByText("Ver fontes e sinais"));
    expect(screen.getByText(/Telefone literal candidato encontrado/)).toBeVisible();
    expect(invoke).toHaveBeenCalledWith("pesquisar_multimodal", expect.objectContaining({ casoId: caso.id, pedido: expect.objectContaining({ filtro: { texto: "55119999" } }) }));
  });
});
