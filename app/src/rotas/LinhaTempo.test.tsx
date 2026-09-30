import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { AchadoArtefato, ArtefatoResumo, Caso, FiltroLinhaTempo } from "../api/cliente";
import { PaginaConsultas, PaginaLocalizacoes } from "./PaginasEstruturais";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Dados SINTÉTICOS (nenhum dado real).
const caso = { id: "c1", fuso_horario: "America/Sao_Paulo", numero_procedimento: "IP 1/2026", numero_bo: null } as Caso;

const base: ArtefatoResumo = {
  id: 0,
  tipo: "mensagem",
  subtipo: "texto",
  app: "whatsapp",
  remetente: null,
  destinatarios: [],
  de_mim: false,
  data_hora_ms: null,
  conteudo: null,
  conversa_id_externo: null,
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

// 01/01/2026 10:00 e 02/01/2026 09:00 em São Paulo (UTC-3).
const chamada: AchadoArtefato = {
  artefato: { ...base, id: 5, tipo: "chamada", subtipo: "perdida", app: "telefone", remetente: "+5521900000013", data_hora_ms: 1_767_272_400_000 },
  extracao_id: "x1",
  trecho: null,
  conversa_id: null,
};
const mensagem: AchadoArtefato = {
  artefato: {
    ...base,
    id: 9,
    remetente: "5511900000011@s.whatsapp.net",
    conteudo: "Mensagem sintética",
    data_hora_ms: 1_767_355_200_000,
  },
  extracao_id: "x1",
  trecho: null,
  conversa_id: 3,
};
const local: AchadoArtefato = {
  ...mensagem,
  artefato: { ...mensagem.artefato, id: 10, subtipo: "localizacao", conteudo: "Local sintético", latitude: -23.55052, longitude: -46.633308 },
};

let pedidos: { cmd: string; filtro: FiltroLinhaTempo }[] = [];

beforeEach(() => {
  pedidos = [];
  vi.mocked(invoke).mockImplementation(async (cmd: string, args?: unknown) => {
    const a = args as { filtro: FiltroLinhaTempo } | undefined;
    if (a?.filtro) pedidos.push({ cmd, filtro: a.filtro });
    switch (cmd) {
      case "caso_atual":
        return { caso, pasta: "C:/Casos/c1" };
      case "listar_extracoes":
        return [];
      case "resumo_linha_do_tempo":
        return {
          por_tipo: [
            { tipo: "mensagem", total: 1 },
            { tipo: "chamada", total: 1 },
          ],
          apps: [
            { tipo: "telefone", total: 1 },
            { tipo: "whatsapp", total: 1 },
          ],
          primeira_ms: chamada.artefato.data_hora_ms,
          ultima_ms: mensagem.artefato.data_hora_ms,
          sem_data: 2,
        };
      case "linha_do_tempo": {
        const f = a!.filtro;
        let itens = f.com_coordenadas ? [local] : [chamada, mensagem];
        if (f.tipos?.length) itens = itens.filter((i) => f.tipos!.includes(i.artefato.tipo));
        if (f.recentes_primeiro) itens = [...itens].reverse();
        return { total: itens.length, itens };
      }
      default:
        return undefined;
    }
  });
});

function montar(elemento: ReactNode, rota: string, caminho: string) {
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

describe("linha do tempo (Consultas)", () => {
  it("agrupa por dia no fuso do caso e liga cada registro à sua tela", async () => {
    montar(<PaginaConsultas />, "/casos/c1/consultas", "/casos/:casoId/consultas");
    const dia1 = await screen.findByRole("region", { name: /1 de janeiro de 2026/ });
    const dia2 = screen.getByRole("region", { name: /2 de janeiro de 2026/ });
    expect(within(dia1).getByText("10:00:00")).toBeInTheDocument();
    expect(within(dia1).getByRole("link")).toHaveAttribute("href", "/casos/c1/contatos?aba=chamadas&registro=5");
    expect(within(dia2).getByRole("link")).toHaveAttribute("href", "/casos/c1/conversas/3?registro=9");
    expect(within(dia2).getByText("Mensagem sintética")).toBeInTheDocument();
    // Sem data: contados à parte, nunca na lista.
    const resumo = screen.getByRole("region", { name: "Resumo: Linha do tempo" });
    expect(within(resumo).getByText("Sem data utilizável").closest("div")).toHaveTextContent("2");
  });

  it("filtra por tipo e inverte a ordem pelo backend", async () => {
    const usuario = userEvent.setup();
    montar(<PaginaConsultas />, "/casos/c1/consultas", "/casos/:casoId/consultas");
    await usuario.click(await screen.findByRole("button", { name: "Chamada (1)" }));
    expect(await screen.findAllByRole("link")).toHaveLength(1);
    expect(pedidos.filter((p) => p.cmd === "linha_do_tempo").at(-1)!.filtro.tipos).toEqual(["chamada"]);

    await usuario.click(screen.getByRole("button", { name: /Todos/ }));
    await usuario.click(screen.getByRole("checkbox", { name: "Mais recentes primeiro" }));
    const ultimo = pedidos.filter((p) => p.cmd === "linha_do_tempo").at(-1)!;
    expect(ultimo.filtro.recentes_primeiro).toBe(true);
    // O resumo não depende da ordem.
    expect(pedidos.filter((p) => p.cmd === "resumo_linha_do_tempo").every((p) => !p.filtro.recentes_primeiro)).toBe(true);
  });

  it("consultas salvas continuam marcadas como indisponíveis", async () => {
    const usuario = userEvent.setup();
    montar(<PaginaConsultas />, "/casos/c1/consultas", "/casos/:casoId/consultas");
    await usuario.click(await screen.findByRole("tab", { name: "Consultas salvas" }));
    expect(screen.getByText(/Espaço reservado para guardar consultas/)).toBeInTheDocument();
  });
});

describe("Localizações", () => {
  it("lista só registros com coordenadas, com latitude e longitude como lidas", async () => {
    montar(<PaginaLocalizacoes />, "/casos/c1/localizacoes", "/casos/:casoId/localizacoes");
    expect(await screen.findByText("-23.55052, -46.633308")).toBeInTheDocument();
    expect(pedidos.filter((p) => p.cmd === "linha_do_tempo").every((p) => p.filtro.com_coordenadas === true)).toBe(true);
    expect(screen.getByText("Mapa offline")).toBeInTheDocument();
  });
});
