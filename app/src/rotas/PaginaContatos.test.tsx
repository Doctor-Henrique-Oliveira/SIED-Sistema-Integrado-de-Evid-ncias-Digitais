import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { Caso, Chamada, ContatoAgenda } from "../api/cliente";
import { PaginaContatos } from "./PaginaContatos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Dados SINTÉTICOS (nenhum dado real).
const caso = { id: "c1", fuso_horario: "America/Sao_Paulo", numero_procedimento: "IP 1/2026", numero_bo: null } as Caso;

const ana: ContatoAgenda = {
  id: 11,
  extracao_id: "x1",
  app: "contatos_android",
  nome: "Ana Sintética",
  conta: "perito.sintetico@example.com",
  tipo_conta: "com.google",
  telefones: [{ numero: "(11) 90000-0011", tipo: "celular", rotulo: null }],
  emails: ["ana@example.com"],
  empresa: null,
  id_externo: "1",
  estado_registro: "ativo",
  favorito: true,
};
const eduardo: ContatoAgenda = {
  ...ana,
  id: 14,
  nome: "Eduardo Sintético",
  conta: null,
  tipo_conta: null,
  telefones: [],
  emails: [],
  id_externo: "4",
  estado_registro: "apagado",
  favorito: false,
};

const recebida: Chamada = {
  id: 21,
  extracao_id: "x1",
  subtipo: "recebida",
  de_mim: false,
  numero: "+5511900000011",
  numero_especial: null,
  data_hora_ms: 1_700_000_000_000,
  duracao_s: 125,
  nome_no_registro: "Ana",
  apresentacao: "permitido",
  conta_telefonica: null,
  recursos: [],
  estado_registro: "ativo",
  agenda: [{ id: 11, nome: "Ana Sintética", estado_registro: "ativo" }],
};
const privada: Chamada = {
  ...recebida,
  id: 28,
  numero: null,
  numero_especial: "-2",
  nome_no_registro: null,
  agenda: [],
};
const restrita: Chamada = {
  ...recebida,
  id: 26,
  subtipo: "perdida",
  numero: null,
  duracao_s: 0,
  nome_no_registro: null,
  apresentacao: "restrito",
  agenda: [],
};

const detalhe = (id: number) => ({
  resumo: { id, tipo: "chamada", app: "telefone", subtipo: "recebida", estado_registro: "ativo", data_hora_ms: null },
  referencias: [
    {
      principal: true,
      papel_arquivo: "calllog",
      arquivo_caminho: "data/data/com.android.providers.contacts/databases/calllog.db",
      arquivo_sha256: "cd".repeat(32),
      tabela: "calls",
      id_registro: "1",
      deslocamento: null,
      consulta: null,
      valores_brutos: { _id: 1, number: "+5511900000011" },
    },
  ],
  metadados: {},
  parser: "android_chamadas",
  parser_versao: "1.0.0",
  importador: "generico",
  importador_versao: "1.0.0",
  confianca: "alta",
  origem_caminho: "C:/extracao",
  hash_manifesto: "ef".repeat(32),
  hash_registro: "01".repeat(32),
  data_hora_original: null,
  formato_data: null,
});

const respostas: Record<string, (a?: Record<string, unknown>) => unknown> = {
  caso_atual: () => ({ caso, pasta: "C:/Casos/c1" }),
  apps_dos_contatos: () => [
    { app: "contatos_android", total: 2 },
    { app: "whatsapp", total: 1 },
  ],
  listar_contatos: () => ({ total: 2, itens: [ana, eduardo] }),
  resumo_chamadas: () => ({
    total: 2,
    por_subtipo: [
      { subtipo: "perdida", total: 1 },
      { subtipo: "recebida", total: 1 },
    ],
    duracao_total_s: 125,
    numeros_distintos: 1,
  }),
  listar_chamadas: () => ({ total: 3, itens: [recebida, restrita, privada] }),
  marcacoes_evidencias: () => ({ registros: [[21, 3]], arquivos: [] }),
  detalhe_artefato: (a) => detalhe(Number(a?.artefatoId)),
  adicionar_evidencia: () => ({ numero: 4, itens: 1 }),
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (cmd: string, args?: unknown) => respostas[cmd]?.(args as Record<string, unknown>));
});

function Local() {
  const l = useLocation();
  return <p data-testid="local">{l.search}</p>;
}

function montar(rota: string) {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter initialEntries={[rota]}>
        <Routes>
          <Route
            path="/casos/:casoId/contatos"
            element={
              <>
                <PaginaContatos />
                <Local />
              </>
            }
          />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("contatos: agenda", () => {
  it("lista os contatos como gravados, com conta, situação e filtro por origem", async () => {
    montar("/casos/c1/contatos");
    const t = await screen.findByRole("table", { name: "Contatos" });
    expect(within(t).getByText("Ana Sintética")).toBeInTheDocument();
    expect(within(t).getByText("(11) 90000-0011")).toBeInTheDocument();
    expect(within(t).getByText("perito.sintetico@example.com")).toBeInTheDocument();
    expect(within(t).getByText("Apagado")).toBeInTheDocument();
    expect(within(t).getByText("Favorito")).toBeInTheDocument();
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("listar_contatos", { filtro: {}, deslocamento: 0, limite: 100 });

    await userEvent.click(screen.getByRole("button", { name: "WhatsApp (1)" }));
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("listar_contatos", { filtro: { app: "whatsapp" }, deslocamento: 0, limite: 100 });
  });

  it("filtra por nome ou telefone e mostra o detalhe com a origem técnica", async () => {
    montar("/casos/c1/contatos");
    const campo = await screen.findByRole("searchbox", { name: "Filtrar contatos" });
    await userEvent.type(campo, "90000-0011{Enter}");
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("listar_contatos", {
      filtro: { texto: "90000-0011" },
      deslocamento: 0,
      limite: 100,
    });

    await userEvent.click(await screen.findByRole("button", { name: "Ana Sintética" }));
    const card = screen.getByRole("region", { name: "Contato selecionado" });
    expect(within(card).getByText("ana@example.com")).toBeInTheDocument();
    expect(await screen.findByText("data/data/com.android.providers.contacts/databases/calllog.db")).toBeInTheDocument();

    await userEvent.click(within(card).getByRole("button", { name: "Adicionar às evidências" }));
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("adicionar_evidencia", { nova: { artefato_id: 11 } });

    // "Ver chamadas" leva à aba de chamadas filtrada pelo número.
    await userEvent.click(within(card).getByRole("button", { name: "Ver chamadas" }));
    expect(screen.getByRole("tab", { name: "Chamadas" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByTestId("local").textContent).toBe("?aba=chamadas&q=%2811%29+90000-0011");
  });

  it("sem contatos, explica o que importar em vez de inventar dados", async () => {
    respostas.listar_contatos = () => ({ total: 0, itens: [] });
    montar("/casos/c1/contatos");
    expect(await screen.findByText("Nenhum contato importado")).toBeInTheDocument();
    respostas.listar_contatos = () => ({ total: 2, itens: [ana, eduardo] });
  });
});

describe("contatos: chamadas", () => {
  it("mostra resumo, tipo, número ou apresentação, nome no registro e vínculo com a agenda", async () => {
    montar("/casos/c1/contatos?aba=chamadas");
    const faixa = await screen.findByRole("region", { name: "Resumo das chamadas" });
    expect(await within(faixa).findByText("2 min 05 s")).toBeInTheDocument();
    const t = await screen.findByRole("table", { name: "Chamadas" });
    const linhas = within(t).getAllByRole("row");
    expect(within(linhas[1]!).getByText("+5511900000011")).toBeInTheDocument();
    expect(within(linhas[1]!).getByText("Recebida")).toBeInTheDocument();
    expect(within(linhas[1]!).getByText("Ana")).toBeInTheDocument();
    expect(within(linhas[1]!).getByText("Ana Sintética")).toBeInTheDocument();
    expect(within(linhas[1]!).getByText("E-3")).toBeInTheDocument();
    expect(within(linhas[2]!).getByText("Número restrito (privado)")).toBeInTheDocument();
    expect(within(linhas[2]!).getByText("Perdida")).toBeInTheDocument();
    // Código antigo no lugar do número: explicado, nunca exibido como telefone.
    expect(within(linhas[3]!).getByText("Número privado (código -2)")).toBeInTheDocument();
  });

  it("filtra por tipo e por texto vindo da URL", async () => {
    montar("/casos/c1/contatos?aba=chamadas&q=Ana");
    await screen.findByRole("table", { name: "Chamadas" });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("listar_chamadas", { filtro: { texto: "Ana" }, deslocamento: 0, limite: 100 });
    await userEvent.click(screen.getByRole("button", { name: "Perdida (1)" }));
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("listar_chamadas", {
      filtro: { texto: "Ana", subtipo: "perdida" },
      deslocamento: 0,
      limite: 100,
    });
  });

  it("abre a chamada indicada na URL (vinda da busca) com o painel técnico", async () => {
    montar("/casos/c1/contatos?aba=chamadas&registro=21");
    const card = await screen.findByRole("region", { name: "Chamada selecionada" });
    expect(within(card).getByText("E-3")).toBeInTheDocument();
    expect(within(card).queryByRole("button", { name: "Adicionar às evidências" })).not.toBeInTheDocument();
    expect(await screen.findByLabelText("Detalhes técnicos do registro")).toBeInTheDocument();
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("detalhe_artefato", { artefatoId: 21 });
  });

  it("sem chamadas no caso, diz o que importar", async () => {
    respostas.resumo_chamadas = () => ({ total: 0, por_subtipo: [], duracao_total_s: 0, numeros_distintos: 0 });
    montar("/casos/c1/contatos?aba=chamadas");
    expect(await screen.findByText("Nenhuma chamada importada")).toBeInTheDocument();
  });
});
