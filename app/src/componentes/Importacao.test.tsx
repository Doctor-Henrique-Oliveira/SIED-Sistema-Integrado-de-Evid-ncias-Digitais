import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import type { Caso, EstadoImportacao, EventoImportacao } from "../api/cliente";
import { PainelImportacao } from "./Importacao";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const caso = { id: "c1", fuso_horario: "America/Sao_Paulo" } as Caso;

function estado(parcial: Partial<EstadoImportacao>): EstadoImportacao {
  return {
    extracao_id: "x1",
    dispositivo_id: "d1",
    terminada: false,
    extracao: null,
    erro: null,
    progresso: {
      etapa: "inventariando",
      arquivos_total: 10,
      arquivos_feitos: 4,
      bytes_total: 2000,
      bytes_feitos: 500,
      tarefas_total: 0,
      tarefas_feitas: 0,
      artefatos: 0,
      avisos: 0,
      item_atual: "dados/mmssms.db",
      mensagem: null,
    },
    ...parcial,
  };
}

function montar(e: EstadoImportacao) {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <MemoryRouter>
        <PainelImportacao caso={caso} estado={e} aoDispensar={() => {}} />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("PainelImportacao", () => {
  it("mostra etapa, progresso e permite cancelar durante a importação", () => {
    montar(estado({}));
    expect(screen.getByText(/Calculando hashes/)).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "25");
    expect(screen.getByText("Arquivos: 4 de 10")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancelar importação" })).toBeEnabled();
  });

  it("mostra o resumo e o link quando termina", () => {
    montar(
      estado({
        terminada: true,
        extracao: {
          status: "concluida",
          total_arquivos: 5,
          total_bytes: 12500,
          total_artefatos: 6,
          total_avisos: 0,
          erro: null,
        } as EstadoImportacao["extracao"],
      }),
    );
    expect(screen.getByText("Concluída")).toBeInTheDocument();
    expect(screen.getByText(/6 registros interpretados/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Abrir extração" })).toHaveAttribute("href", "/casos/c1/extracoes/x1");
  });
});

// Eventos SINTÉTICOS, no formato que o backend devolve.
const ev = (seq: number, nivel: EventoImportacao["nivel"], mensagem: string): EventoImportacao => ({
  seq,
  ts_ms: 1_767_261_600_000 + seq * 1000,
  nivel,
  mensagem,
});

describe("console da importação", () => {
  it("busca os eventos pela sequência e mostra horário, decorrido e nível", async () => {
    const lotes: Record<number, EventoImportacao[]> = {
      0: [ev(1, "info", "Iniciando importação com Pasta genérica 1.0.0."), ev(2, "ok", "Cópia conferida: SHA-256 igual ao do inventário.")],
      2: [ev(3, "aviso", "MMS (Android): parte sem mensagem."), ev(4, "erro", "Não foi possível ler a/b.db.")],
    };
    const pedidos: number[] = [];
    vi.mocked(invoke).mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd !== "eventos_importacao") return undefined;
      const desde = (args as { desde: number }).desde;
      pedidos.push(desde);
      return lotes[desde] ?? [];
    });
    montar(estado({}));
    const log = await screen.findByRole("log", { name: "Eventos da importação" });
    expect(await within(log).findByText("Não foi possível ler a/b.db.", { exact: false })).toBeInTheDocument();
    const linhas = within(log).getAllByRole("listitem");
    expect(linhas).toHaveLength(4);
    // 10:00:01 em São Paulo; decorrido desde o primeiro evento.
    expect(linhas[0]).toHaveTextContent("[07:00:01]");
    expect(linhas[3]).toHaveTextContent("+00:03");
    expect(linhas[1]).toHaveAttribute("data-nivel", "ok");
    expect(linhas[2]).toHaveAttribute("data-nivel", "aviso");
    expect(pedidos.slice(0, 2)).toEqual([0, 2]);
    expect(screen.getByText(/1 linha\(s\) de aviso/)).toBeInTheDocument();
    expect(screen.getByText(/1 linha\(s\) de erro/)).toBeInTheDocument();
    expect(screen.getByText("ao vivo")).toBeInTheDocument();
  });

  it("recolhe e expande o console", async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => (cmd === "eventos_importacao" ? [ev(1, "info", "Iniciando.")] : undefined));
    const usuario = userEvent.setup();
    montar(estado({}));
    const botao = await screen.findByRole("button", { name: /Console da importação/ });
    expect(botao).toHaveAttribute("aria-expanded", "true");
    await usuario.click(botao);
    expect(botao).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("log")).not.toBeInTheDocument();
  });
});
