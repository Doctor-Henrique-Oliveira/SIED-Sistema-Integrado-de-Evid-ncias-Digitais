import { describe, expect, it } from "vitest";
import type { Dispositivo, Extracao } from "../api/cliente";
import { fontesProntas } from "./PreparacaoCaso";

// Dados exclusivamente sintéticos: o gate depende do status real, não de números do Dashboard.
const dispositivo = { id: "disp-sintetico" } as Dispositivo;
const extracao = (status: Extracao["status"]) => ({ id: "ext-sintetica", status }) as Extracao;

describe("liberação visual do workspace", () => {
  it("mantém a preparação enquanto as consultas ainda não retornaram", () => {
    expect(fontesProntas(undefined, undefined)).toBe(false);
  });

  it("não libera a navegação com apenas um dispositivo", () => {
    expect(fontesProntas([dispositivo], [])).toBe(false);
  });

  it("não trata extração em andamento ou com falha como concluída", () => {
    expect(fontesProntas([dispositivo], [extracao("falhou")])).toBe(false);
    expect(fontesProntas([dispositivo], [extracao("em_andamento")])).toBe(false);
  });

  it("libera o workspace com dispositivo e pelo menos uma extração concluída", () => {
    expect(fontesProntas([dispositivo], [extracao("falhou"), extracao("concluida")])).toBe(true);
  });
});
