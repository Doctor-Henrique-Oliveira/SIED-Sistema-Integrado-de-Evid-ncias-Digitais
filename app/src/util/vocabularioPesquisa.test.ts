import { describe, expect, it } from "vitest";
import { importarVocabularioPesquisa, termosHabilitados } from "./vocabularioPesquisa";

// Vocabulário SINTÉTICO, sem associação ou significado criminal atribuído.
const dados = {
  nome: "Vocabulário sintético", origem: "Fixture sintética para teste",
  ramos: [
    { nome: "Expressões", habilitado: true, termos: ["encontro", "reunião"] },
    { nome: "Emojis", habilitado: false, termos: ["👨‍👩‍👧‍👦", "🤝🏽"] },
  ],
};

describe("vocabulário de pesquisa local", () => {
  it("preserva sequências Unicode e não inclui ramos desativados nos termos efetivos", () => {
    const importado = importarVocabularioPesquisa(JSON.stringify(dados));
    expect(importado.ramos[1]?.termos).toEqual(["👨‍👩‍👧‍👦", "🤝🏽"]);
    expect(termosHabilitados(importado)).toEqual(["encontro", "reunião"]);
  });

  it("carrega somente dados de pacote exportado, não autor/hash/id externo como metadata local", () => {
    expect(importarVocabularioPesquisa(JSON.stringify({
      formato: "sied-vocabulario-pesquisa-v1", dados, id: 987, versao: 23,
      sha256: "a".repeat(64), autor: "Autor de fixture externa", criado_em: "2026-01-01T00:00:00Z",
    }))).toEqual(dados);
  });

  it.each([
    "não é JSON", "[]", "null", '{"dados":{},"formato":"outro"}',
    JSON.stringify({ ...dados, ramos: [] }),
    JSON.stringify({ ...dados, executar: "shell" }),
    JSON.stringify({ ...dados, nome: "SINTÉTICO\0" }),
    JSON.stringify({ ...dados, ramos: [{ nome: "Ramo", habilitado: true, termos: ["sintético\0"] }] }),
    JSON.stringify({ ...dados, ramos: [{ nome: "Ramo", habilitado: "sim", termos: [] }] }),
    JSON.stringify({ ...dados, ramos: [{ nome: "Ramo", habilitado: true, termos: ["a".repeat(201)] }] }),
    JSON.stringify({ ...dados, ramos: [{ nome: "Ramo", habilitado: true, termos: Array(32).fill("sintético") }] }),
    JSON.stringify({ ...dados, ramos: Array(9).fill({ nome: "Ramo", habilitado: false, termos: [] }) }),
    `{"nome":"Sintético","origem":"Fixture","ramos":[],"__proto__":{"url":"https://example.invalid"}}`,
  ])("rejeita JSON inválido/estrutura não suportada sem executar conteúdo: %s", (json) => {
    expect(() => importarVocabularioPesquisa(json)).toThrow();
  });

  it("limita tamanho do JSON antes de parsear", () => {
    expect(() => importarVocabularioPesquisa(" ".repeat(200 * 1024 + 1))).toThrow(/200 KiB/);
  });
});
