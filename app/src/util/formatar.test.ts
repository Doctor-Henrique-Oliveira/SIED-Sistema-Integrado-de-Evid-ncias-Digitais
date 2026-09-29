import { describe, expect, it } from "vitest";
import {
  bytes,
  dataHora,
  duracao,
  extensao,
  hora,
  identificacaoCaso,
  lerCapacidade,
  limiteDia,
  nomeArquivo,
  ouNulo,
  servicoDeNuvem,
} from "./formatar";

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

describe("limiteDia", () => {
  it("converte o dia no fuso do caso para o intervalo em UTC", () => {
    // Meia-noite em Brasília = 03:00 UTC.
    expect(limiteDia("2026-01-01", "America/Sao_Paulo")).toBe(Date.UTC(2026, 0, 1, 3));
    expect(limiteDia("2026-01-01", "America/Sao_Paulo", true)).toBe(Date.UTC(2026, 0, 2, 3) - 1);
    expect(limiteDia("2026-01-01", "UTC")).toBe(Date.UTC(2026, 0, 1));
    expect(limiteDia("01/01/2026")).toBeNull();
  });

  it("mostra a hora com segundos no fuso do caso", () => {
    expect(hora(Date.UTC(2026, 0, 1, 3, 4, 5), "America/Sao_Paulo")).toBe("00:04:05");
  });
});

describe("duração e caminhos", () => {
  it("duração legível entre dois instantes", () => {
    expect(duracao("2026-01-01T10:00:00Z", "2026-01-01T10:00:42Z")).toBe("42 s");
    expect(duracao("2026-01-01T10:00:00Z", "2026-01-01T10:02:13Z")).toBe("2 min 13 s");
    expect(duracao("2026-01-01T10:00:00Z", "2026-01-01T12:05:00Z")).toBe("2 h 5 min");
    expect(duracao("2026-01-01T10:00:00Z", null)).toBe("—");
  });

  it("extensão e nome do arquivo", () => {
    expect(extensao("dados/telefonia/mmssms.DB")).toBe("db");
    expect(extensao("pasta\\sem_extensao")).toBe("");
    expect(extensao(".oculto")).toBe("");
    expect(nomeArquivo("a/b/IMG_0001.dat")).toBe("IMG_0001.dat");
  });

  it("pasta sincronizada com a nuvem", () => {
    expect(servicoDeNuvem("C:\\Users\\analista\\OneDrive\\Documentos\\SIED\\Casos")).toBe("OneDrive");
    expect(servicoDeNuvem("C:\\Users\\a\\OneDrive - Polícia Civil\\Casos")).toBe("OneDrive");
    expect(servicoDeNuvem("D:\\Dropbox\\Casos")).toBe("Dropbox");
    expect(servicoDeNuvem("G:\\Meu Drive\\Casos")).toBe("Google Drive");
    expect(servicoDeNuvem("C:\\Users\\a\\Documents\\SIED\\Casos")).toBeNull();
    expect(servicoDeNuvem("C:\\SIED\\OneDriveAntigo\\Casos")).toBeNull();
    expect(servicoDeNuvem("/home/a/SIED/Casos")).toBeNull();
  });
});
