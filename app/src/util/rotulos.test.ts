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

  it("expõe divergência de fonte como atenção", () => {
    const diagnostico = diagnosticoCheckpoint("fonte_divergente", "interrompida");
    expect(diagnostico.tipo).toBe("atencao");
    expect(diagnostico.titulo).toBe("Checkpoint com fonte divergente");
    expect(diagnostico.detalhe).toContain("não coincide");
  });
});
