import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PaginaNovoCaso } from "./PaginaNovoCaso";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

function montar(emRede: boolean) {
  vi.mocked(invoke).mockImplementation(async (cmd: string) => {
    if (cmd === "info_app")
      return {
        versao: "0.1.0",
        usuario: "analista",
        maquina: "ESTACAO",
        pasta_dados: "C:\\dados",
        pasta_logs: "C:\\dados\\logs",
        pasta_casos_padrao: "Z:\\SIED\\Casos",
      };
    if (cmd === "pasta_em_rede") return emRede;
    throw new Error(`inesperado: ${cmd}`);
  });
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter>
        <PaginaNovoCaso />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("Novo caso numa unidade mapeada", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("avisa quando o sistema diz que a unidade é de rede", async () => {
    montar(true);
    expect(await screen.findByText(/parece estar em um compartilhamento de rede/)).toBeInTheDocument();
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("pasta_em_rede", { caminho: "Z:\\SIED\\Casos" });
  });

  it("não avisa quando a unidade é local", async () => {
    montar(false);
    expect(await screen.findByText("Z:\\SIED\\Casos")).toBeInTheDocument();
    await vi.waitFor(() => expect(vi.mocked(invoke)).toHaveBeenCalledWith("pasta_em_rede", { caminho: "Z:\\SIED\\Casos" }));
    expect(screen.queryByText(/parece estar em/)).not.toBeInTheDocument();
  });
});
