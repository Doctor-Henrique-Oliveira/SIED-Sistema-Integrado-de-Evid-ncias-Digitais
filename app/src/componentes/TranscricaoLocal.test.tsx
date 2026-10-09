import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ArquivoExtracao, Caso } from "../api/cliente";
import { TranscricaoLocal } from "./TranscricaoLocal";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const caso = { id: "caso-SINTETICO" } as Caso;
const arquivo: ArquivoExtracao = { id: 17, caminho: "audio/SINTETICO.wav", sha256: "ab".repeat(32), tamanho: 1024, categoria: "audio", modificado_em: null, criado_em: null, assinatura: "wav", mime: "audio/wav", erro: null };
const semSuporte = { id: 1, caso_id: caso.id, extracao_id: "extracao-SINTETICA", arquivo_id: arquivo.id, caminho: arquivo.caminho, sha256_origem: arquivo.sha256, estado: "sem_suporte", idioma: null, motor_id: "nenhum-provedor-local", motor_versao: "1", iniciado_em: "2026-10-09T12:00:00Z", concluido_em: "2026-10-09T12:00:00Z", erro: "nenhum sidecar SRT/VTT encontrado", hash_derivado: "cd".repeat(32), total_segmentos: 0 } as const;

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((comando) => comando === "listar_transcricoes" ? Promise.resolve([]) : comando === "transcrever_local" ? Promise.resolve(semSuporte) : Promise.reject(new Error(`Comando não previsto no teste SINTÉTICO: ${comando}`)));
});

describe("transcrição local", () => {
  it("executa por identificador e mantém sem suporte explícito sem inventar texto", async () => {
    render(<TranscricaoLocal caso={caso} arquivo={arquivo} />);
    await userEvent.click(screen.getByRole("button", { name: /Executar local/i }));
    expect(invoke).toHaveBeenCalledWith("transcrever_local", { casoId: caso.id, arquivoId: arquivo.id });
    expect(await screen.findByText(/nenhum sidecar SRT\/VTT encontrado/)).toBeInTheDocument();
    expect(screen.queryByRole("list", { name: "Segmentos da transcrição" })).not.toBeInTheDocument();
  });
});
