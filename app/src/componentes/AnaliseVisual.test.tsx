import { invoke } from "@tauri-apps/api/core";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ArquivoExtracao, Caso, ResultadoSimilarVisual, VisualIndexado } from "../api/cliente";
import { AnaliseVisual } from "./AnaliseVisual";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const caso = { id: "caso-SINTETICO" } as Caso;
const extracaoId = "extracao-SINTETICA";
const arquivo: ArquivoExtracao = { id: 17, caminho: "imagens/SINTETICA.png", sha256: "ab".repeat(32), tamanho: 1024, categoria: "imagem", modificado_em: null, criado_em: null, assinatura: "png", mime: "image/png", erro: null };
const visual = { id: 73, caso_id: caso.id, extracao_id: extracaoId, arquivo_id: arquivo.id, caminho: arquivo.caminho, sha256_origem: arquivo.sha256, dhash: "0011223344556677", largura: 100, altura: 80, luminancia_media: 42.5, motor_id: "visao_dhash_local", motor_versao: "motor-sintetico-1", executado_em: "2026-10-07T12:00:00Z", hash_derivado: "cd".repeat(32) } as VisualIndexado;
const semelhantes = { referencia: visual, descritores_examinados: 2, varredura_limitada: false, limites: ["Resultado SINTÉTICO não prova identidade."], itens: [
  { arquivo_id: 18, extracao_id: extracaoId, caminho: "imagens/COPIA-SINTETICA.png", sha256: arquivo.sha256, mecanismo: "sha256_exato", distancia_bits: null, score_tecnico: null },
  { arquivo_id: 19, extracao_id: extracaoId, caminho: "imagens/PROXIMA-SINTETICA.png", sha256: "ef".repeat(32), mecanismo: "dhash64_candidato", distancia_bits: 3, score_tecnico: 61 / 64 },
] } as ResultadoSimilarVisual;

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((comando) => comando === "indexar_visual" ? Promise.resolve(visual) : comando === "semelhantes_visuais" ? Promise.resolve(semelhantes) : Promise.reject(new Error("Comando não previsto no teste SINTÉTICO.")));
});

describe("análise visual local", () => {
  it("indexa por identificador conferido e navega para candidatos com mecanismos separados", async () => {
    render(<MemoryRouter><AnaliseVisual caso={caso} extracaoId={extracaoId} arquivo={arquivo} /></MemoryRouter>);
    expect(invoke).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Analisar imagem" }));
    expect(invoke).toHaveBeenCalledWith("indexar_visual", { casoId: caso.id, arquivoId: arquivo.id });
    expect(await screen.findByText(/100 × 80/)).toBeInTheDocument();
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Distância visual" }), "3");
    await userEvent.click(screen.getByRole("button", { name: "Buscar semelhantes no caso" }));
    expect(invoke).toHaveBeenCalledWith("semelhantes_visuais", { casoId: caso.id, operacaoId: expect.any(String), visualId: visual.id, maxBits: 3, limite: 30 });
    const lista = await screen.findByRole("list", { name: "Imagens semelhantes" });
    expect(within(lista).getByText("Mesmos bytes por SHA-256")).toBeInTheDocument();
    expect(within(lista).getByText(/Candidato dHash · distância 3\/64/)).toBeInTheDocument();
    expect(within(lista).getByRole("link", { name: "imagens/COPIA-SINTETICA.png" })).toHaveAttribute("href", "/casos/caso-SINTETICO/arquivos?extracao=extracao-SINTETICA&arquivo=18");
    expect(within(lista).getByRole("link", { name: "imagens/PROXIMA-SINTETICA.png" })).toHaveAttribute("href", "/casos/caso-SINTETICO/arquivos?extracao=extracao-SINTETICA&arquivo=19");
  });

  it("recusa retorno com origem divergente e não habilita consulta semelhante", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ ...visual, sha256_origem: "00".repeat(32) });
    render(<MemoryRouter><AnaliseVisual caso={caso} extracaoId={extracaoId} arquivo={arquivo} /></MemoryRouter>);
    await userEvent.click(screen.getByRole("button", { name: "Analisar imagem" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("não corresponde à imagem selecionada");
    expect(screen.queryByRole("button", { name: "Buscar semelhantes no caso" })).not.toBeInTheDocument();
  });

  it("explica por que imagem sem hash ou acima de 2 MiB não pode ser indexada", () => {
    const tela = render(<MemoryRouter><AnaliseVisual caso={caso} extracaoId={extracaoId} arquivo={{ ...arquivo, sha256: null }} /></MemoryRouter>);
    expect(screen.getByRole("button", { name: "Analisar imagem" })).toBeDisabled();
    expect(screen.getByText(/não tem SHA-256 conferível/)).toBeInTheDocument();
    tela.rerender(<MemoryRouter><AnaliseVisual caso={caso} extracaoId={extracaoId} arquivo={{ ...arquivo, tamanho: 2 * 1024 * 1024 + 1 }} /></MemoryRouter>);
    expect(screen.getByText(/até 2 MiB/)).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });
});
