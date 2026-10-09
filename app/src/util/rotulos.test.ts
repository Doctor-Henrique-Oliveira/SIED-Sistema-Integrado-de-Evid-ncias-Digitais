import { describe, expect, it } from "vitest";
import { diagnosticoCheckpoint } from "./rotulos";

describe("diagnosticoCheckpoint", () => {
  it("identifica checkpoint concluído como encerrado", () => {
    expect(diagnosticoCheckpoint("concluida", "concluida")).toEqual({
      tipo: "sucesso",
      titulo: "Checkpoint final: Concluída",
      detalhe: "A importação foi encerrada. Este checkpoint não é candidato à recuperação.",
    });
  });

  it("não trata checkpoint parcial como autorização de retomada", () => {
    const diagnostico = diagnosticoCheckpoint("interrompida", "interrompida");
    expect(diagnostico.tipo).toBe("atencao");
    expect(diagnostico.detalhe).toContain("retomada automática permanece desativada");
  });
});
