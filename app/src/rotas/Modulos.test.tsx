import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { Caso } from "../api/cliente";
import { BarraLateral, grupos } from "../componentes/BarraLateral";
import { Indicador, SecaoRecolhivel } from "../componentes/Estrutura";
import { PaginaContatos, PaginaMidias } from "./PaginasModulos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Dados SINTÉTICOS.
const caso = { id: "c1", fuso_horario: "America/Sao_Paulo", numero_procedimento: "IP 1/2026", numero_bo: null } as Caso;

const respostas: Record<string, unknown> = {
  caso_atual: { caso, pasta: "C:/Casos/c1" },
  listar_identificadores: {
    total: 2,
    itens: [
      { identificador: "+5511900000001", app: "sms", conversas: 1, mensagens: 3, primeira_ms: 0, ultima_ms: 1000 },
      { identificador: "+5521900000002", app: "sms", conversas: 1, mensagens: 1, primeira_ms: 0, ultima_ms: 1000 },
    ],
  },
  listar_extracoes: [
    {
      id: "x1",
      dispositivo_id: "d1",
      importador_nome: "Pasta genérica",
      iniciada_em: "2026-01-01T12:00:00Z",
    },
  ],
  listar_dispositivos: [{ id: "d1", apelido: "Celular 1" }],
  totais_por_categoria: [
    { categoria: "imagem", arquivos: 2, bytes: 10 },
    { categoria: "banco_sqlite", arquivos: 1, bytes: 10 },
  ],
  listar_execucoes: [],
  listar_arquivos: {
    total: 1,
    itens: [
      {
        id: 7,
        caminho: "DCIM/IMG_0001.jpg",
        tamanho: 2048,
        modificado_em: null,
        criado_em: null,
        sha256: "ab".repeat(32),
        assinatura: "jpeg",
        mime: "image/jpeg",
        categoria: "imagem",
        erro: null,
      },
    ],
  },
};

vi.mocked(invoke).mockImplementation(async (cmd: string) => respostas[cmd]);

function montar(rota: string, elemento: ReactNode, caminho: string) {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter initialEntries={[rota]}>
        <Routes>
          <Route path={caminho} element={elemento} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("barra lateral", () => {
  it("tem os 12 módulos e desativa os do caso quando nenhum caso está aberto", () => {
    expect(grupos.flatMap((g) => g.modulos).length + 1).toBe(12); // + Configurações
    render(
      <MemoryRouter>
        <BarraLateral caso={undefined} info={undefined} />
      </MemoryRouter>,
    );
    const nav = screen.getByRole("navigation", { name: "Navegação principal" });
    expect(within(nav).getByRole("link", { name: "Casos" })).toHaveAttribute("href", "/");
    expect(within(nav).queryByRole("link", { name: "Conversas" })).not.toBeInTheDocument();
    expect(within(nav).getByText("Conversas").closest("[aria-disabled]")).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByRole("link", { name: "Configurações" })).toHaveAttribute("href", "/configuracoes");
  });

  it("aponta os módulos para o caso aberto", () => {
    render(
      <MemoryRouter>
        <BarraLateral caso={caso} info={undefined} />
      </MemoryRouter>,
    );
    expect(screen.getByRole("link", { name: "Conversas" })).toHaveAttribute("href", "/casos/c1/conversas");
    expect(screen.getByRole("link", { name: "Dashboard" })).toHaveAttribute("href", "/casos/c1");
    expect(screen.getByText("IP 1/2026")).toBeInTheDocument();
  });
});

describe("componentes", () => {
  it("indicador sem dado mostra traço, nunca um número inventado", () => {
    render(<Indicador rotulo="Conversas" icone="conversas" valor={null} />);
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("seção recolhível alterna com o teclado/mouse e informa o estado", async () => {
    render(
      <SecaoRecolhivel titulo="Origem">
        <p>conteúdo</p>
      </SecaoRecolhivel>,
    );
    const botao = screen.getByRole("button", { name: "Origem" });
    expect(botao).toHaveAttribute("aria-expanded", "true");
    await userEvent.click(botao);
    expect(botao).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByText("conteúdo")).not.toBeVisible();
  });
});

describe("módulos do caso", () => {
  it("contatos lista os identificadores como o parser leu", async () => {
    montar("/casos/c1/contatos", <PaginaContatos />, "/casos/:casoId/contatos");
    const tabelaIds = await screen.findByRole("table", { name: "Identificadores das conversas" });
    expect(within(tabelaIds).getByText("+5511900000001")).toBeInTheDocument();
    expect(screen.getByText("Identificadores (2)")).toBeInTheDocument();
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("listar_identificadores", { texto: null, deslocamento: 0, limite: 100 });
  });

  it("mídias pede só imagens, vídeos, áudios e documentos e mostra os detalhes do item", async () => {
    montar("/casos/c1/midias", <PaginaMidias />, "/casos/:casoId/midias");
    const grade = await screen.findByRole("list", { name: "Mídias" });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("listar_arquivos", {
      extracaoId: "x1",
      categoria: "midia",
      texto: null,
      deslocamento: 0,
      limite: 100,
    });
    expect(screen.getByRole("button", { name: "Todas (2)" })).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(within(grade).getByRole("button"));
    const detalhes = screen.getByLabelText("Metadados do arquivo");
    expect(within(detalhes).getByText("ab".repeat(32))).toBeInTheDocument();
    expect(within(detalhes).getByText("jpg")).toBeInTheDocument();
  });
});
