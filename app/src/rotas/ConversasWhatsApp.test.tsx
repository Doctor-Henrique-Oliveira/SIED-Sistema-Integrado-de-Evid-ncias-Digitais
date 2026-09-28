import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { ArtefatoResumo, Conversa } from "../api/cliente";
import { PaginaConversas, tituloConversa } from "./PaginaConversas";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Dados SINTÉTICOS, no formato que o parser do WhatsApp grava.
const BRUNO = "5511900000012@s.whatsapp.net";
const CARLA = "5521900000013@s.whatsapp.net";

const grupo: Conversa = {
  id: 9,
  extracao_id: "x1",
  app: "whatsapp",
  id_externo: "120363000000000001@g.us",
  total_mensagens: 6,
  enviadas: 1,
  com_anexo: 2,
  primeira_ms: Date.UTC(2026, 0, 2, 12),
  ultima_ms: Date.UTC(2026, 0, 2, 13),
  participantes: [
    { identificador: BRUNO, mensagens: 2, nome: "Bruno Sintético" },
    { identificador: CARLA, mensagens: 1, nome: null },
  ],
  total_participantes: 2,
  ultima_mensagem: "Mensagem encaminhada (sintética)",
  titulo: "Grupo sintético de teste",
  tipo: "grupo",
  registro_id: 77,
};

function msg(id: number, campos: Partial<ArtefatoResumo>): ArtefatoResumo {
  return {
    id,
    tipo: "mensagem",
    subtipo: "texto",
    app: "whatsapp",
    remetente: null,
    destinatarios: [],
    de_mim: false,
    data_hora_ms: Date.UTC(2026, 0, 2, 12, id),
    conteudo: null,
    conversa_id_externo: grupo.id_externo,
    estado_registro: "ativo",
    id_externo: `SINT${id}`,
    resposta_a_id_externo: null,
    nome_arquivo: null,
    mime: null,
    tamanho: null,
    anexo_arquivo_id: null,
    anexo_vinculo: null,
    latitude: null,
    longitude: null,
    ...campos,
  };
}

const mensagens = [
  msg(1, { subtipo: "sistema", conteudo: null }),
  msg(2, { remetente: BRUNO, conteudo: "Bom dia, grupo (sintético)" }),
  msg(3, {
    remetente: CARLA,
    subtipo: "documento",
    conteudo: "Segue o documento",
    nome_arquivo: "relatorio-sintetico.pdf",
    mime: "application/pdf",
    tamanho: 128,
    anexo_vinculo: "ambiguo",
  }),
  msg(4, {
    remetente: BRUNO,
    subtipo: "imagem",
    nome_arquivo: "IMG-20260102-WA0001.jpg",
    tamanho: 64,
    anexo_arquivo_id: 12,
    anexo_vinculo: "sufixo_caminho",
  }),
  msg(5, { remetente: BRUNO, subtipo: "localizacao", latitude: -23.55052, longitude: -46.633308, conteudo: "Local sintético" }),
  msg(6, { de_mim: true, conteudo: "Recebi", resposta_a_id_externo: "SINT2" }),
  msg(7, { remetente: CARLA, subtipo: "apagada" }),
];

vi.mocked(invoke).mockImplementation(async (cmd: string) => {
  const respostas: Record<string, unknown> = {
    caso_atual: { caso: { id: "c1", fuso_horario: "America/Sao_Paulo" }, pasta: "C:/Casos/c1" },
    listar_extracoes: [],
    listar_conversas: { total: 1, itens: [grupo] },
    obter_conversa: grupo,
    mensagens_conversa: { total: mensagens.length, itens: mensagens },
  };
  if (cmd === "detalhe_artefato") return new Promise(() => {}); // fica carregando
  return respostas[cmd];
});

function montar(rota: string) {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter initialEntries={[rota]}>
        <Routes>
          <Route path="/casos/:casoId/conversas/:conversaId?" element={<PaginaConversas />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("conversas do WhatsApp", () => {
  it("título vem do app; sem ele, nomes dos contatos antes dos identificadores", () => {
    expect(tituloConversa(grupo)).toBe("Grupo sintético de teste");
    expect(tituloConversa({ ...grupo, titulo: null })).toBe(`Bruno Sintético, ${CARLA}`);
  });

  it("mostra grupo, remetentes com nome, sistema, anexos, localização, resposta e apagada", async () => {
    montar("/casos/c1/conversas/9");
    const chat = await screen.findByRole("region", { name: "Chat" });
    expect(within(chat).getByRole("heading", { name: "Grupo sintético de teste" })).toBeInTheDocument();

    const area = await screen.findByLabelText("Mensagens da conversa");
    const bolhas = within(area).getAllByRole("button");
    expect(bolhas).toHaveLength(7);
    const [sistema, texto, doc, foto, local, resposta, apagada] = bolhas as [
      HTMLElement,
      HTMLElement,
      HTMLElement,
      HTMLElement,
      HTMLElement,
      HTMLElement,
      HTMLElement,
    ];

    // Mensagem de sistema centralizada, sem lado.
    expect(sistema.parentElement).toHaveClass("justify-center");
    expect(within(sistema).getByText("Mensagem do sistema")).toBeInTheDocument();

    // Remetente: nome do contato quando existe, sempre com o identificador.
    expect(within(texto).getByText("Bruno Sintético")).toBeInTheDocument();
    expect(within(texto).getByText(BRUNO)).toBeInTheDocument();
    expect(within(doc).getByText(CARLA)).toBeInTheDocument();

    // Anexos: ligado ao inventário ou o motivo de não estar.
    const anexoDoc = within(doc).getByLabelText("Anexo");
    expect(within(anexoDoc).getByText("relatorio-sintetico.pdf")).toBeInTheDocument();
    expect(within(anexoDoc).getByText("Mais de um arquivo possível")).toBeInTheDocument();
    expect(within(anexoDoc).getByText("application/pdf · 128 B")).toBeInTheDocument();
    expect(within(within(foto).getByLabelText("Anexo")).getByText("Na extração")).toBeInTheDocument();

    expect(within(local).getByText("-23.550520, -46.633308")).toBeInTheDocument();
    expect(within(local).getByText("Local sintético")).toBeInTheDocument();

    const citada = within(resposta).getByLabelText("Mensagem respondida");
    expect(within(citada).getByText("Bruno Sintético")).toBeInTheDocument();
    expect(within(citada).getByText("Bom dia, grupo (sintético)")).toBeInTheDocument();
    expect(resposta.parentElement).toHaveClass("justify-end");

    expect(within(apagada).getByText("Apagada")).toBeInTheDocument();
    expect(within(apagada).getByText(/mensagem apagada/)).toBeInTheDocument();
  });

  it("contexto traz tipo, nomes dos participantes e o registro da conversa no app", async () => {
    montar("/casos/c1/conversas/9");
    const contexto = await screen.findByRole("complementary", { name: "Contexto da conversa" });
    expect(within(contexto).getByText("Título")).toBeInTheDocument();
    expect(within(contexto).getByText("Grupo")).toBeInTheDocument();
    expect(within(contexto).getByText("Bruno Sintético")).toBeInTheDocument();
    expect(within(contexto).getByText(/Nomes como gravados nos contatos do próprio app/)).toBeInTheDocument();

    await userEvent.click(within(contexto).getByRole("button", { name: "Abrir registro da conversa no app" }));
    expect(screen.getByRole("button", { name: "Detalhes técnicos" })).toHaveAttribute("aria-pressed", "true");
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("detalhe_artefato", { artefatoId: 77 });
  });
});
