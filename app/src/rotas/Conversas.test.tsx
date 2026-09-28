import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { ArtefatoResumo, Conversa } from "../api/cliente";
import { PaginaBusca, TrechoDestacado } from "./PaginaBusca";
import { PaginaConversas, tituloConversa } from "./PaginaConversas";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Dados SINTÉTICOS.
const conversa: Conversa = {
  id: 5,
  extracao_id: "x1",
  app: "sms",
  id_externo: "7",
  total_mensagens: 2,
  enviadas: 1,
  com_anexo: 0,
  primeira_ms: Date.UTC(2026, 0, 1, 12),
  ultima_ms: Date.UTC(2026, 0, 2, 12),
  participantes: [{ identificador: "+5511900000001", mensagens: 2, nome: null }],
  total_participantes: 1,
  ultima_mensagem: "Até amanhã",
  titulo: null,
  tipo: null,
  registro_id: null,
};

function msg(id: number, deMim: boolean, ms: number, conteudo: string): ArtefatoResumo {
  return {
    id,
    tipo: "mensagem",
    subtipo: "texto",
    app: "sms",
    remetente: deMim ? null : "+5511900000001",
    destinatarios: deMim ? ["+5511900000001"] : [],
    de_mim: deMim,
    data_hora_ms: ms,
    conteudo,
    conversa_id_externo: "7",
    estado_registro: "ativo",
    id_externo: null,
    resposta_a_id_externo: null,
    nome_arquivo: null,
    mime: null,
    tamanho: null,
    anexo_arquivo_id: null,
    anexo_vinculo: null,
    latitude: null,
    longitude: null,
  };
}

const respostas: Record<string, unknown> = {
  caso_atual: { caso: { id: "c1", fuso_horario: "America/Sao_Paulo" }, pasta: "C:/Casos/c1" },
  listar_extracoes: [],
  listar_conversas: { total: 1, itens: [conversa] },
  obter_conversa: conversa,
  mensagens_conversa: {
    total: 2,
    itens: [msg(1, false, Date.UTC(2026, 0, 1, 12), "Oi, tudo bem?"), msg(2, true, Date.UTC(2026, 0, 2, 12), "Até amanhã")],
  },
  detalhe_artefato: null,
  buscar: {
    modo: "texto",
    artefatos: {
      total: 1,
      itens: [
        {
          artefato: msg(2, true, Date.UTC(2026, 0, 2, 12), "Até amanhã"),
          extracao_id: "x1",
          trecho: "\u0001Até\u0002 amanhã",
          conversa_id: 5,
        },
      ],
    },
    arquivos: { total: 0, itens: [] },
  },
};

vi.mocked(invoke).mockImplementation(async (cmd: string) => {
  if (cmd === "detalhe_artefato") return new Promise(() => {}); // fica carregando
  return respostas[cmd];
});

function montar(rota: string) {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter initialEntries={[rota]}>
        <Routes>
          <Route path="/casos/:casoId/conversas/:conversaId?" element={<PaginaConversas />} />
          <Route path="/casos/:casoId/busca" element={<PaginaBusca />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("conversas", () => {
  it("título usa os participantes e indica quantos faltam", () => {
    expect(tituloConversa(conversa)).toBe("+5511900000001");
    expect(tituloConversa({ ...conversa, total_participantes: 3 })).toBe("+5511900000001 e mais 2");
    expect(tituloConversa({ ...conversa, participantes: [] })).toBe("Conversa 7");
  });

  it("reconstrói o chat com lados, separadores de dia e alterna para a visão técnica", async () => {
    montar("/casos/c1/conversas/5");
    const lista = await screen.findByRole("list", { name: "Lista de conversas" });
    expect(within(lista).getByText("Até amanhã")).toBeInTheDocument();

    const area = await screen.findByLabelText("Mensagens da conversa");
    const bolhas = within(area).getAllByRole("button");
    expect(bolhas).toHaveLength(2);
    const [recebida, enviada] = bolhas as [HTMLElement, HTMLElement];
    // Recebida à esquerda com o remetente; enviada à direita.
    expect(recebida.parentElement).toHaveClass("justify-start");
    expect(within(recebida).getByText("+5511900000001")).toBeInTheDocument();
    expect(enviada.parentElement).toHaveClass("justify-end");
    // Um separador por dia, no fuso do caso.
    expect(within(area).getByText(/1 de janeiro de 2026/)).toBeInTheDocument();
    expect(within(area).getByText(/2 de janeiro de 2026/)).toBeInTheDocument();
    expect(within(recebida).getByText("09:00:00")).toBeInTheDocument();

    expect(screen.queryByText("Detalhes técnicos", { selector: "h2" })).not.toBeInTheDocument();
    await userEvent.click(enviada);
    expect(screen.getByRole("button", { name: "Detalhes técnicos" })).toHaveAttribute("aria-pressed", "true");
    expect(enviada).toHaveAttribute("aria-pressed", "true");
  });
});

describe("busca", () => {
  it("destaca o termo no trecho", () => {
    render(<TrechoDestacado texto={"antes \u0001termo\u0002 depois"} />);
    expect(screen.getByText("termo").tagName).toBe("MARK");
    expect(screen.getByText("antes")).toBeInTheDocument();
  });

  it("busca e leva o registro para a conversa", async () => {
    montar("/casos/c1/busca");
    await userEvent.type(await screen.findByLabelText("Texto da busca"), "ate");
    await userEvent.click(screen.getByRole("button", { name: "Buscar" }));
    const achado = await screen.findByRole("link", { name: /amanhã/ });
    expect(achado).toHaveAttribute("href", "/casos/c1/conversas/5?registro=2");
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("buscar", { filtro: { texto: "ate" }, deslocamento: 0, limite: 50 });
  });
});
