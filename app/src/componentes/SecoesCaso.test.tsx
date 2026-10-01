import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Caso, ConferenciaDerivados, DiferencaDerivado } from "../api/cliente";
import { SecaoAtividades } from "./SecoesCaso";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const caso = { id: "c1", fuso_horario: "America/Sao_Paulo" } as Caso;

const t = (tabela: string, parcial: Partial<DiferencaDerivado> = {}): DiferencaDerivado => ({
  tabela,
  esperadas: 10,
  faltando: 0,
  sobrando: 0,
  indice_integro: true,
  ...parcial,
});

function conferencia(tabelas: DiferencaDerivado[]): ConferenciaDerivados {
  return {
    tabelas,
    consistente: tabelas.every((x) => x.faltando === 0 && x.sobrando === 0 && x.indice_integro),
  };
}

const ok = conferencia([t("conversa"), t("conversa_participante"), t("busca_artefato"), t("busca_arquivo")]);

function montar(respostas: Record<string, unknown>) {
  vi.mocked(invoke).mockImplementation((cmd: string) =>
    cmd in respostas ? Promise.resolve(respostas[cmd]) : Promise.reject(new Error(`inesperado: ${cmd}`)),
  );
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter>
        <SecaoAtividades caso={caso} />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("Conversas e índices na auditoria", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("confere e informa quando batem com os registros", async () => {
    montar({ listar_auditoria: [], conferir_derivados: ok });
    const secao = screen.getByRole("region", { name: "Conversas e índices" });
    await userEvent.click(within(secao).getByRole("button", { name: "Conferir" }));
    expect(await within(secao).findByText("Conferem com os registros importados.")).toBeInTheDocument();
    expect(within(secao).queryByRole("button", { name: "Reconstruir" })).not.toBeInTheDocument();
  });

  it("lista as diferenças e reconstrói", async () => {
    const divergente = conferencia([
      t("conversa", { faltando: 1, sobrando: 2 }),
      t("conversa_participante"),
      t("busca_artefato", { indice_integro: false }),
      t("busca_arquivo"),
    ]);
    montar({
      listar_auditoria: [],
      conferir_derivados: divergente,
      reconstruir_derivados: { antes: divergente, depois: ok },
    });
    const secao = screen.getByRole("region", { name: "Conversas e índices" });
    await userEvent.click(within(secao).getByRole("button", { name: "Conferir" }));
    const lista = await within(secao).findByRole("list", { name: "Diferenças nos dados derivados" });
    expect(within(lista).getAllByRole("listitem").map((li) => li.textContent)).toEqual([
      "Conversas: 1 faltando, 2 a mais ou alteradas",
      "Índice de busca dos registros: índice danificado",
    ]);
    await userEvent.click(within(secao).getByRole("button", { name: "Reconstruir" }));
    expect(await within(secao).findByText(/Reconstruído: conversas e índices conferem/)).toBeInTheDocument();
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("reconstruir_derivados", undefined);
  });
});
