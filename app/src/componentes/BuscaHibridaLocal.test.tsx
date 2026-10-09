import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Caso } from "../api/cliente";
import type { ResultadoBuscaHibrida } from "../api/embeddings";
import { BuscaHibridaLocal } from "./BuscaHibridaLocal";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const caso = { id: "caso-sintetico", fuso_horario: "America/Sao_Paulo" } as Caso;
const resultado: ResultadoBuscaHibrida = {
  consulta: "telefone",
  motor_id: "hash-embedding-local",
  motor_versao: "tfidf-hash-v1",
  peso_semantico: 0.6,
  avisos: ["Scores são ranking, não confiança."],
  itens: {
    total: 1,
    itens: [{ origem_tipo: "artefato", origem_id: 7, extracao_id: "extracao-sintetica", arquivo_id: null, titulo: "Mensagem", trecho: "telefone encontrado", score_hibrido: 0.8, score_semantico: 0.9, score_textual: 1 / 61, inicio_ms: null, fim_ms: null, sinais: [{ mecanismo: "textual", motivo: "Correspondência textual local", contribuicao: 0.1, posicao: 1, motor_id: "sqlite-fts5", motor_versao: "unicode61" }, { mecanismo: "semantico_local", motivo: "Similaridade vetorial local · 0.900", contribuicao: 0.54, posicao: null, motor_id: "hash-embedding-local", motor_versao: "tfidf-hash-v1" }] }],
  },
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((comando) => {
    if (comando === "indexar_embeddings") return Promise.resolve({ examinados: 2, criados: 2, reutilizados: 0, ignorados: 0, motor_id: "hash-embedding-local", motor_versao: "tfidf-hash-v1" });
    if (comando === "buscar_hibrida") return Promise.resolve(resultado);
    if (comando === "cancelar_descoberta") return Promise.resolve(true);
    return Promise.reject(new Error("comando inesperado"));
  });
});

describe("BuscaHibridaLocal", () => {
  it("indexa e apresenta os mecanismos que explicam o ranking", async () => {
    render(<BuscaHibridaLocal caso={caso} />);
    await userEvent.click(screen.getByRole("button", { name: "Indexar conteúdo local" }));
    expect(invoke).toHaveBeenCalledWith("indexar_embeddings", expect.objectContaining({ casoId: caso.id }));
    await userEvent.type(screen.getByRole("textbox", { name: "Consulta híbrida" }), "telefone");
    await userEvent.click(screen.getByRole("button", { name: "Buscar" }));
    expect(invoke).toHaveBeenCalledWith("buscar_hibrida", expect.objectContaining({ casoId: caso.id, texto: "telefone" }));
    expect(await screen.findByText("Por que este resultado apareceu?")).toBeInTheDocument();
    await userEvent.click(screen.getByText("Por que este resultado apareceu?"));
    expect(screen.getByText("Correspondência textual")).toBeInTheDocument();
    expect(screen.getByText("Similaridade semântica local")).toBeInTheDocument();
    expect(screen.getAllByText(/não confiança/).length).toBeGreaterThan(0);
  });
});
