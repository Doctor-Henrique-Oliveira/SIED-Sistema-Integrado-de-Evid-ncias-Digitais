import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import type { Caso, CandidatoRevisao } from "../api/cliente";
import { FilaRevisao } from "./FilaRevisao";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const caso = { id: "caso-fila-sintetico", fuso_horario: "America/Sao_Paulo" } as Caso;
const candidato: CandidatoRevisao = {
  id: 73, documento_id: 41, caso_id: caso.id, extracao_id: "extracao-sintetica", arquivo_id: 27,
  caminho: "documentos/NOTA-SINTETICA.txt", tipo: "email", literal: "candidato@example.invalid",
  inicio_utf8: 4, fim_utf8: 31, estado: "nao_revisado", nota: null, motor_versao: "regras-sinteticas-1",
  sha256_origem: "ab".repeat(32), sha256_texto: "cd".repeat(32), revista_em: null, revisor: null,
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((comando) => {
    if (comando === "fila_revisao") return Promise.resolve({ total: 1, itens: [candidato] });
    if (comando === "revisar_entidade") return Promise.resolve({ ...candidato, estado: "confirmado", nota: "conferido" });
    return Promise.reject(new Error(`Comando sintético inesperado: ${comando}`));
  });
});

describe("fila de revisão humana", () => {
  it("carrega sob demanda, mantém proveniência e registra decisão separada", async () => {
    render(<MemoryRouter><FilaRevisao caso={caso} /></MemoryRouter>);
    expect(invoke).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Abrir fila de revisão" }));
    expect(await screen.findByText("candidato@example.invalid")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Abrir arquivo original" })).toHaveAttribute("href", "/casos/caso-fila-sintetico/arquivos?extracao=extracao-sintetica&arquivo=27");
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Decisão humana" }), "confirmado");
    await userEvent.type(screen.getByRole("textbox", { name: "Nota da revisão (opcional)" }), "conferido");
    await userEvent.click(screen.getByRole("button", { name: "Registrar decisão" }));
    expect(invoke).toHaveBeenCalledWith("revisar_entidade", { casoId: caso.id, entidadeId: 73, estado: "confirmado", nota: "conferido" });
  });
});
