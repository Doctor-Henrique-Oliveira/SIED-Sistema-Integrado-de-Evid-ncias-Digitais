import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { CasoAberto, InfoApp } from "../api/cliente";
import { PaginaCasos } from "../rotas/PaginaCasos";
import { ErroExperiencia, ForensicBootConsole, SplashScreen } from "./Experiencia";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const aberto = {
  caso: {
    id: "caso-sintetico",
    numero_procedimento: "SINTÉTICO-001/2026",
    numero_bo: null,
    fuso_horario: "America/Sao_Paulo",
  },
  pasta: "C:/Casos/SINTETICO-001",
} as CasoAberto;

const info = { usuario: "analista", maquina: "ESTACAO-01", versao: "0.1.0" } as InfoApp;

afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

describe("experiência premium de entrada", () => {
  it("encerra a splash no tempo definido", () => {
    vi.useFakeTimers();
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: false })));
    const terminou = vi.fn();
    render(<SplashScreen aoTerminar={terminou} duracao={500} />);
    expect(screen.getByText("SIED")).toBeInTheDocument();
    expect(terminou).not.toHaveBeenCalled();
    vi.advanceTimersByTime(500);
    expect(terminou).toHaveBeenCalledOnce();
  });

  it("mantém a apresentação completa por 3,8 segundos", () => {
    vi.useFakeTimers();
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: false })));
    const terminou = vi.fn();
    render(<SplashScreen aoTerminar={terminou} />);
    expect(screen.getByText("Sistema Integrado de Evidências Digitais")).toBeInTheDocument();
    expect(screen.getByText("Inicializando estação forense local")).toBeInTheDocument();
    vi.advanceTimersByTime(3799);
    expect(terminou).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(terminou).toHaveBeenCalledOnce();
  });

  it("reduz a splash quando o sistema pede menos movimento", () => {
    vi.useFakeTimers();
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: true })));
    const terminou = vi.fn();
    render(<SplashScreen aoTerminar={terminou} duracao={5000} />);
    vi.advanceTimersByTime(179);
    expect(terminou).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(terminou).toHaveBeenCalledOnce();
  });

  it("mostra welcome sem sidebar e mantém as duas ações dominantes", async () => {
    vi.mocked(invoke).mockImplementation(async (comando: string) => {
      if (comando === "info_app") return info;
      if (comando === "listar_casos") return [];
      return null;
    });
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <MemoryRouter><PaginaCasos /></MemoryRouter>
      </QueryClientProvider>,
    );
    expect(await screen.findByRole("button", { name: /Novo caso/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Abrir caso existente/ })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Navegação principal" })).not.toBeInTheDocument();
  });

  it("console usa a verificação real e preserva a ordem dos marcos", async () => {
    vi.mocked(invoke).mockResolvedValue({ registros: 12, hash_final: "ab".repeat(32) });
    const pronto = vi.fn();
    render(<ForensicBootConsole aberto={aberto} info={info} aoPronto={pronto} aoErro={vi.fn()} />);
    expect(screen.getByText("Pasta de caso reconhecida")).toBeInTheDocument();
    await screen.findByText("Trilha de auditoria conferida", {}, { timeout: 5000 });
    const linhas = screen.getAllByRole("listitem").map((item) => item.textContent);
    expect(linhas[0]).toContain("Pasta de caso reconhecida");
    expect(linhas[1]).toContain("Banco do caso aberto");
    expect(linhas[2]).toContain("Caso identificado");
    expect(linhas[3]).toContain("Trilha de auditoria conferida");
    await waitFor(() => expect(pronto).toHaveBeenCalledOnce(), { timeout: 6000 });
  });

  it("erro de abertura permanece visível e oferece recuperação", async () => {
    const voltar = vi.fn();
    const tentar = vi.fn();
    render(<ErroExperiencia mensagem="Banco incompatível" aoVoltar={voltar} aoTentar={tentar} />);
    expect(screen.getByText("Banco incompatível")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Tentar novamente" }));
    await userEvent.click(screen.getByRole("button", { name: "Voltar" }));
    expect(tentar).toHaveBeenCalledOnce();
    expect(voltar).toHaveBeenCalledOnce();
  });
});
