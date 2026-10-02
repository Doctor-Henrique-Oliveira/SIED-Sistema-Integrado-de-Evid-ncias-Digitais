import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { Layout } from "./Layout";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    unmaximize: vi.fn(), setResizable: vi.fn(), setMaximizable: vi.fn(), setDecorations: vi.fn(), setSize: vi.fn(),
    setMinSize: vi.fn(), center: vi.fn(),
  }),
  currentMonitor: vi.fn().mockResolvedValue(null),
}));
vi.mock("@tauri-apps/api/dpi", () => ({ LogicalSize: class { constructor(public width: number, public height: number) {} } }));

function montar() {
  return render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter initialEntries={["/"]}>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<p>Tela inicial</p>} />
            <Route path="casos/:casoId" element={<p>Dashboard carregado</p>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

afterEach(() => { vi.useRealTimers(); vi.clearAllMocks(); });

describe("máquina de estados da experiência", () => {
  it("sem caso libera a welcome, mas não o workspace", async () => {
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: true })));
    vi.mocked(invoke).mockImplementation(async (comando: string) => comando === "info_app"
      ? { versao: "0.1.0", usuario: "analista", maquina: "ESTACAO", pasta_dados: "C:/Dados", pasta_casos_padrao: "C:/Casos" }
      : null);
    montar();
    expect(await screen.findByText("Tela inicial", {}, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Navegação principal" })).not.toBeInTheDocument();
  });

  it("caso aberto passa pelo console antes de liberar sidebar e workspace", async () => {
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: true })));
    const caso = { id: "c1", numero_procedimento: "SINTÉTICO-001", numero_bo: null, fuso_horario: "America/Sao_Paulo" };
    vi.mocked(invoke).mockImplementation(async (comando: string) => {
      if (comando === "info_app") return { versao: "0.1.0", usuario: "analista", maquina: "ESTACAO", pasta_dados: "C:/Dados", pasta_casos_padrao: "C:/Casos" };
      if (comando === "caso_atual") return { caso, pasta: "C:/Casos/c1" };
      if (comando === "verificar_auditoria") return { registros: 3, hash_final: "ab".repeat(32) };
      if (comando === "estado_importacao") return null;
      return [];
    });
    montar();
    expect(await screen.findByRole("main", { name: "Preparando workspace do caso" }, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Navegação principal" })).not.toBeInTheDocument();
    expect(await screen.findByRole("navigation", { name: "Navegação principal" }, { timeout: 7000 })).toBeInTheDocument();
    expect(await screen.findByText("Dashboard carregado", {}, { timeout: 1000 })).toBeInTheDocument();
  });
});
