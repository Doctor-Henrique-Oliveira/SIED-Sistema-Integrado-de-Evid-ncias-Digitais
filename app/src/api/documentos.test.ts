import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api, FalhaSied } from "./cliente";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

describe("fronteira documental", () => {
  it("envia apenas identidade do arquivo e guarda do caso ao backend", async () => {
    const resultado = { estado: "sem_suporte", texto: null, motivo: "Fixture SINTÉTICA sem motor." };
    vi.mocked(invoke).mockResolvedValue(resultado);
    expect(await api.lerDocumento(21, "caso-sintetico")).toEqual(resultado);
    expect(invoke).toHaveBeenCalledExactlyOnceWith("ler_documento", {
      arquivoId: 21,
      casoId: "caso-sintetico",
    });
  });

  it("preserva a recusa de contexto do Tauri como erro estruturado", async () => {
    vi.mocked(invoke).mockRejectedValue({ codigo: "conflito", mensagem: "Caso sintético mudou.", campos: [] });
    await expect(api.lerDocumento(21, "caso-sintetico")).rejects.toMatchObject({
      name: "FalhaSied",
      message: "Caso sintético mudou.",
      erro: { codigo: "conflito", campos: [] },
    });
  });

  it("solicita uma página de PDF sem enviar caminho/hash ou parâmetros de parser", async () => {
    const resultado = { estado: "sem_suporte", texto: null, motivo: "PDF SINTÉTICO de recusa." };
    vi.mocked(invoke).mockResolvedValue(resultado);
    expect(await api.lerDocumento(21, "caso-sintetico", 2)).toEqual(resultado);
    expect(invoke).toHaveBeenCalledExactlyOnceWith("ler_documento", { arquivoId: 21, casoId: "caso-sintetico", pagina: 2 });
  });

  it("normaliza uma falha de transporte sem tratá-la como leitura pronta", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("Transporte sintético indisponível."));
    await expect(api.lerDocumento(21, "caso-sintetico")).rejects.toBeInstanceOf(FalhaSied);
  });
});
