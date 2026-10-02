import type { VocabularioPesquisa } from "../api/cliente";

export const MAX_RAMOS = 8;
export const MAX_TERMOS = 31;
const MAX_JSON = 200 * 1024;

function objeto(valor: unknown): valor is Record<string, unknown> {
  return typeof valor === "object" && valor !== null && !Array.isArray(valor);
}

function somenteCampos(valor: Record<string, unknown>, campos: string[]) {
  if (Object.keys(valor).some((chave) => !campos.includes(chave))) {
    throw new Error("O JSON contém campos não reconhecidos. Importe somente um vocabulário de pesquisa do SIED.");
  }
}

function texto(valor: unknown, campo: string, maximo: number): string {
  if (typeof valor !== "string" || !valor.trim() || valor.includes("\0") || [...valor].length > maximo) {
    throw new Error(`${campo}: informe texto não vazio com até ${maximo} caracteres, sem NUL.`);
  }
  return valor.trim();
}

/** Somente dados: nunca eval, URL, acesso a arquivo ou alteração silenciosa do vocabulário. */
export function importarVocabularioPesquisa(json: string): VocabularioPesquisa {
  if (new TextEncoder().encode(json).length > MAX_JSON) throw new Error("O JSON excede o limite de 200 KiB.");
  let valor: unknown;
  try { valor = JSON.parse(json); } catch { throw new Error("JSON inválido. Revise o texto antes de importar."); }
  if (!objeto(valor)) throw new Error("O JSON deve conter um objeto de vocabulário.");
  // Um pacote exportado mantém sua metadata para conferência. A importação cria
  // uma NOVA versão local; não reaproveita ID, autor, hash ou data como se locais.
  if ("dados" in valor) {
    somenteCampos(valor, ["formato", "dados", "id", "versao", "sha256", "autor", "criado_em"]);
    if ("formato" in valor && valor.formato !== "sied-vocabulario-pesquisa-v1") {
      throw new Error("Formato de vocabulário não suportado.");
    }
    valor = valor.dados;
  }
  if (!objeto(valor)) throw new Error("Dados do vocabulário ausentes ou inválidos.");
  somenteCampos(valor, ["nome", "origem", "ramos"]);
  if (!Array.isArray(valor.ramos) || valor.ramos.length === 0 || valor.ramos.length > MAX_RAMOS) {
    throw new Error(`O vocabulário deve ter uma lista com 1 a ${MAX_RAMOS} ramos.`);
  }
  const ramos = valor.ramos.map((ramo: unknown, indice: number) => {
    if (!objeto(ramo)) throw new Error(`Ramo ${indice + 1} inválido.`);
    somenteCampos(ramo, ["nome", "habilitado", "termos"]);
    if (typeof ramo.habilitado !== "boolean" || !Array.isArray(ramo.termos)) {
      throw new Error(`Ramo ${indice + 1}: habilitação e lista de termos são obrigatórias.`);
    }
    return {
      nome: texto(ramo.nome, `Nome do ramo ${indice + 1}`, 100),
      habilitado: ramo.habilitado,
      termos: ramo.termos.map((termo: unknown) => texto(termo, "Termo", 200)),
    };
  });
  if (ramos.reduce((n, ramo) => n + ramo.termos.length, 0) > MAX_TERMOS) {
    throw new Error(`O vocabulário admite até ${MAX_TERMOS} termos, incluindo ramos desativados.`);
  }
  return { nome: texto(valor.nome, "Nome", 100), origem: texto(valor.origem, "Origem", 300), ramos };
}

export function termosHabilitados(dados: VocabularioPesquisa): string[] {
  return dados.ramos.filter((ramo) => ramo.habilitado).flatMap((ramo) => ramo.termos);
}
