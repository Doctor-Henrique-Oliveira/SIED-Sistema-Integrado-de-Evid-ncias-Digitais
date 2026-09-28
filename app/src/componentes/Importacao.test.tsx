import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import type { Caso, EstadoImportacao } from "../api/cliente";
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
