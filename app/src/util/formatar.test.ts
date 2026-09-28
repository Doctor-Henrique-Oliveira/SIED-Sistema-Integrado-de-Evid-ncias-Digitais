import { describe, expect, it } from "vitest";
import { bytes, dataHora, identificacaoCaso, lerCapacidade, ouNulo } from "./formatar";

describe("formatar", () => {
  it("data em pt-BR no fuso de Brasília", () => {
    expect(dataHora("2026-09-28T17:30:05Z")).toBe("28/09/2026, 14:30:05");
    expect(dataHora(null)).toBe("—");
  });

  it("capacidade", () => {
    expect(bytes(128_000_000_000)).toBe("128 GB");
    expect(bytes(1_500_000_000_000)).toBe("1,5 TB");
    expect(lerCapacidade("128")).toBe(128e9);
    expect(lerCapacidade("1,5 tb")).toBe(1.5e12);
    expect(lerCapacidade("")).toBeNull();
    expect(lerCapacidade("muito")).toBeNaN();
  });

  it("campos vazios viram null", () => {
    expect(ouNulo("  ")).toBeNull();
    expect(ouNulo(" x ")).toBe("x");
  });

  it("identificação do caso", () => {
    expect(identificacaoCaso({ numero_procedimento: "IP 1", numero_bo: "9" })).toBe("IP 1");
    expect(identificacaoCaso({ numero_procedimento: null, numero_bo: "9" })).toBe("BO 9");
  });
});
