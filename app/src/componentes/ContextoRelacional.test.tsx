import { invoke } from "@tauri-apps/api/core";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Caso, ContextoRelacoes } from "../api/cliente";
import { ContextoRelacional } from "./ContextoRelacional";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const caso = { id: "caso-SINTETICO", fuso_horario: "America/Sao_Paulo" } as Caso;
const origem = { chave: "caso:caso-SINTETICO/artefato:11", tipo: "artefato", rotulo: "Registro #11", artefato_id: 11, arquivo_id: null, conversa_id: null, extracao_id: "extracao-SINTETICA", hash_origem: null };
const arquivo = { chave: "caso:caso-SINTETICO/arquivo:7", tipo: "arquivo", rotulo: "Arquivo #7 SINTÉTICO", artefato_id: null, arquivo_id: 7, conversa_id: null, extracao_id: "extracao-SINTETICA", hash_origem: "ab".repeat(32) };
const evento = { chave: "caso:caso-SINTETICO/artefato:12", tipo: "artefato", rotulo: "Registro #12", artefato_id: 12, arquivo_id: null, conversa_id: null, extracao_id: "extracao-SINTETICA", hash_origem: null };
const contexto = {
  caso_id: caso.id, origem: { artefato: { id: 11, tipo: "mensagem", data_hora_ms: 1_800_000_000_000 }, extracao_id: origem.extracao_id, conversa_id: null, trecho: null },
  nos: [origem, arquivo, evento],
  arestas: [
    { de: origem.chave, para: arquivo.chave, classe: "direta", motivo: "Referência de origem SINTÉTICA registrada.", mecanismo: "referencia_origem_inventario_v1", score_contexto: null },
    { de: origem.chave, para: evento.chave, classe: "sugerida", motivo: "Dois horários SINTÉTICOS próximos; não há causalidade.", mecanismo: "proximidade_utc_linear_janela_v1", score_contexto: 0.5 },
  ],
  eventos: { total: 1, itens: [{ artefato: { id: 12, tipo: "mensagem", data_hora_ms: 1_800_000_020_000 }, extracao_id: origem.extracao_id, conversa_id: null, trecho: null }] },
  janela_minutos: 30, limitado: false, avisos: ["A proximidade temporal não prova causalidade."],
} as ContextoRelacoes;

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((comando) => comando === "contexto_relacoes" ? Promise.resolve(contexto) : Promise.reject(new Error("Comando não previsto no teste SINTÉTICO.")));
});

describe("contexto relacional experimental", () => {
  it("consulta explicitamente e preserva classe, motivo, score contextual e navegação original", async () => {
    render(<MemoryRouter><ContextoRelacional caso={caso} artefatoId={11} /></MemoryRouter>);
    expect(invoke).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Explorar até 30 eventos" }));
    expect(invoke).toHaveBeenCalledWith("contexto_relacoes", { casoId: caso.id, operacaoId: expect.any(String), artefatoId: 11, janelaMinutos: 30, limite: 30 });
    const ligacoes = await screen.findByRole("list", { name: "Ligações do grafo" });
    expect(within(ligacoes).getByText("DIRETA")).toBeInTheDocument();
    expect(within(ligacoes).getByText("SUGERIDA")).toBeInTheDocument();
    await userEvent.click(within(ligacoes).getByRole("button", { name: /SUGERIDA/ }));
    const explicacao = screen.getByRole("region", { name: "Explicação da ligação selecionada" });
    expect(explicacao).toHaveTextContent("Dois horários SINTÉTICOS próximos; não há causalidade.");
    expect(explicacao).toHaveTextContent("0.500 · não é probabilidade de vínculo");
    await userEvent.click(within(ligacoes).getByRole("button", { name: /DIRETA/ }));
    expect(explicacao).toHaveTextContent("Referência de origem SINTÉTICA registrada.");
    expect(within(explicacao).getByRole("link", { name: "Abrir Arquivo #7 SINTÉTICO" })).toHaveAttribute("href", "/casos/caso-SINTETICO/arquivos?extracao=extracao-SINTETICA&arquivo=7");
    expect(screen.getByRole("list", { name: "Eventos na janela" })).toHaveTextContent("registro #12");
  });

  it("rejeita resposta associada a outro caso e impede exibição de ligações", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ ...contexto, caso_id: "outro-caso-SINTETICO" });
    render(<MemoryRouter><ContextoRelacional caso={caso} artefatoId={11} /></MemoryRouter>);
    await userEvent.click(screen.getByRole("button", { name: "Explorar até 30 eventos" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("não corresponde ao registro e à janela");
    expect(screen.queryByRole("list", { name: "Ligações do grafo" })).not.toBeInTheDocument();
  });
});
