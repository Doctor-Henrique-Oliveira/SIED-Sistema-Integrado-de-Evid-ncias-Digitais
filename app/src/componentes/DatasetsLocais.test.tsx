import { invoke } from "@tauri-apps/api/core";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AmostraSupervisionada, AvaliacaoSupervisionada, Caso, DatasetSupervisionado, DocumentoIndexado, EntidadeDocumento } from "../api/cliente";
import { DatasetsLocais, trechoUtf8 } from "./DatasetsLocais";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
// Dados exclusivamente SINTÉTICOS e nenhum arquivo real ou chamada externa.
const caso = { id: "caso-sintetico", fuso_horario: "America/Sao_Paulo" } as Caso;
const dataset: DatasetSupervisionado = { id: 11, caso_id: caso.id, nome: "Avaliação SINTÉTICA", versao: 1, autorizacao: "Autorização SINTÉTICA para teste local.", retencao: "Retenção SINTÉTICA declarada.", criado_em: "2026-10-06T12:00:00Z", autor: "Analista SINTÉTICO", total_amostras: 0, total_versoes: 0 };
const texto = "SINTÉTICO 🚀 e-mail: exemplo@example.invalid";
const literal = "exemplo@example.invalid";
const inicio = new TextEncoder().encode(texto.slice(0, texto.indexOf(literal))).length;
const fim = inicio + new TextEncoder().encode(literal).length;
const documento: DocumentoIndexado = { id: 41, caso_id: caso.id, extracao_id: "extracao-sintetica", arquivo_id: 27, caminho: "NOTA-SINTETICA.txt", sha256_origem: "ab".repeat(32), sha256_texto: "cd".repeat(32), texto, pagina: null, total_paginas: null, motor_id: "texto_utf8_nativo", motor_versao: "0.1.0", executado_em: "2026-10-06T12:00:00Z", entidades_limitadas: false, entidades_motor_versao: "regras-sinteticas-1" };
const entidade: EntidadeDocumento = { id: 73, documento_id: 41, tipo: "email", inicio_utf8: inicio, fim_utf8: fim, literal, regra_versao: "email-sintetico-1", estado: "nao_revisado", nota: null, revista_em: null, revisor: null };
const amostra: AmostraSupervisionada = { id: 81, dataset_id: dataset.id, documento_id: documento.id, versao: 1, inicio_utf8: inicio, fim_utf8: fim, trecho: literal, sha256_texto_documento: documento.sha256_texto, sha256_origem: documento.sha256_origem, sha256_amostra: "ef".repeat(32), entidades_motor_versao: documento.entidades_motor_versao, tipos_previstos: ["email"], esperado: true, previsto: true, estado_rotulo: "confirmado", nota: null, criado_em: "2026-10-06T12:05:00Z", revisor: "Analista SINTÉTICO" };
const avaliacao: AvaliacaoSupervisionada = { dataset_id: dataset.id, total_amostras: 0, total_versoes: 0, elegiveis: 0, nao_revisadas: 0, inconclusivas: 0, descartadas: 0, tp: 0, fp: 0, fn: 0, tn: 0, precision: null, recall: null, f1: null, acuracia: null, cobertura: null, motores_avaliados: [], limites: ["Recall restrito aos intervalos manualmente anotados. Dados SINTÉTICOS."] };
function pendente<T>() { let resolver!: (valor: T) => void; const promessa = new Promise<T>((resolve) => { resolver = resolve; }); return { promessa, resolver }; }
beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((comando) => {
    if (comando === "listar_datasets") return Promise.resolve({ total: 1, itens: [dataset] });
    if (comando === "amostras_dataset") return Promise.resolve({ total: 0, itens: [] });
    if (comando === "criar_dataset") return Promise.resolve(dataset);
    if (comando === "obter_documento") return Promise.resolve(documento);
    if (comando === "entidades_documento") return Promise.resolve([entidade]);
    if (comando === "adicionar_amostra") return Promise.resolve(amostra);
    if (comando === "avaliar_dataset") return Promise.resolve(avaliacao);
    if (comando === "cancelar_descoberta") return Promise.resolve(true);
    return Promise.reject(new Error("Comando não previsto na fixture SINTÉTICA."));
  });
});
async function abrirDataset() { await userEvent.click(await screen.findByRole("button", { name: "Abrir dataset #11" })); await screen.findByRole("region", { name: "Dataset autorizado aberto" }); }
async function abrirDocumento() { await userEvent.type(screen.getByRole("spinbutton", { name: "ID do documento indexado" }), "41"); await userEvent.click(screen.getByRole("button", { name: "Abrir texto para anotação" })); await screen.findByRole("region", { name: "Texto de referência da anotação" }); }

describe("intervalos reais UTF-8", () => {
  it("preserva literal após acentos/emoji e rejeita fronteira Unicode parcial e intervalo excessivo", () => {
    expect(trechoUtf8(texto, inicio, fim)).toBe(literal);
    expect(trechoUtf8("🚀", 0, 1)).toBeNull();
    expect(trechoUtf8(texto, -1, fim)).toBeNull();
    expect(trechoUtf8("a".repeat(4097), 0, 4097)).toBeNull();
  });
});
describe("datasets locais explicitamente autorizados", () => {
  it("não cria nem coleta amostras automaticamente e exige autorização/retenção/consentimento explícitos", async () => {
    render(<DatasetsLocais caso={caso} />); await screen.findByRole("list", { name: "Datasets locais" });
    expect(vi.mocked(invoke).mock.calls.map(([comando]) => comando)).toEqual(["listar_datasets"]);
    await userEvent.click(screen.getByText("Criar dataset autorizado"));
    expect(screen.getByRole("button", { name: "Criar dataset" })).toBeDisabled();
    await userEvent.type(screen.getByRole("textbox", { name: "Nome do dataset" }), "Avaliação SINTÉTICA");
    await userEvent.type(screen.getByRole("textbox", { name: "Autorização declarada" }), dataset.autorizacao);
    await userEvent.type(screen.getByRole("textbox", { name: "Retenção declarada" }), dataset.retencao);
    expect(screen.getByRole("button", { name: "Criar dataset" })).toBeDisabled();
    await userEvent.click(screen.getByRole("checkbox", { name: /Confirmo a autorização/ }));
    await userEvent.click(screen.getByRole("button", { name: "Criar dataset" }));
    expect(invoke).toHaveBeenCalledWith("criar_dataset", { casoId: caso.id, nome: dataset.nome, autorizacao: dataset.autorizacao, retencao: dataset.retencao });
    expect(await screen.findByRole("region", { name: "Dataset autorizado aberto" })).toBeInTheDocument();
  });
  it("carrega documento/candidatos reais e registra somente offsets/rótulo/nota por confirmação", async () => {
    render(<DatasetsLocais caso={caso} />); await abrirDataset(); await abrirDocumento();
    expect(invoke).toHaveBeenCalledWith("obter_documento", { casoId: caso.id, documentoId: 41 });
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Ocorrência detectada para seleção" }), "73");
    expect(screen.getByLabelText("Trecho selecionado para amostra").textContent).toBe(literal);
    expect(screen.getByText(/ocorrência exata detectada \(email\)/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Registrar amostra" })).toBeDisabled();
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Estado do rótulo humano" }), "confirmado");
    await userEvent.click(screen.getByRole("checkbox", { name: /Autorizo registrar este intervalo/ }));
    await userEvent.click(screen.getByRole("button", { name: "Registrar amostra" }));
    expect(invoke).toHaveBeenCalledWith("adicionar_amostra", { casoId: caso.id, datasetId: dataset.id, pedido: { documento_id: 41, inicio_utf8: inicio, fim_utf8: fim, esperado: true, previsto: true, estado_rotulo: "confirmado", nota: null } });
    expect(await screen.findByRole("status")).toHaveTextContent("Texto/origem não foram alterados");
    expect(vi.mocked(invoke).mock.calls.some(([comando]) => comando === "adicionar_evidencia" || comando === "gerar_relatorio" || comando.includes("treinar"))).toBe(false);
  });
  it("intervalo manual sem ocorrência corresponde a previsão negativa, sem fabricar candidato", async () => {
    render(<DatasetsLocais caso={caso} />); await abrirDataset(); await abrirDocumento();
    await userEvent.type(screen.getByRole("spinbutton", { name: "Início do intervalo UTF-8" }), "0");
    await userEvent.type(screen.getByRole("spinbutton", { name: "Fim do intervalo UTF-8" }), "4");
    expect(screen.getByLabelText("Trecho selecionado para amostra").textContent).toBe("SINT");
    expect(screen.getByText(/nenhuma ocorrência exata detectada/)).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Estado do rótulo humano" })).toHaveValue("nao_revisado");
  });
  it("recusa documento de outro caso e não habilita registro de amostra", async () => {
    const original = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation((comando, argumentos) => comando === "obter_documento" ? Promise.resolve({ ...documento, caso_id: "outro-caso-sintetico" }) : original(comando, argumentos));
    render(<DatasetsLocais caso={caso} />); await abrirDataset();
    await userEvent.type(screen.getByRole("spinbutton", { name: "ID do documento indexado" }), "41");
    await userEvent.click(screen.getByRole("button", { name: "Abrir texto para anotação" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("não correspondem à origem selecionada");
    expect(screen.queryByRole("button", { name: "Registrar amostra" })).not.toBeInTheDocument();
  });
  it("dados desconhecidos nas métricas são não calculáveis, nunca zero/confiança criminal", async () => {
    render(<DatasetsLocais caso={caso} />); await abrirDataset();
    expect(vi.mocked(invoke).mock.calls.some(([comando]) => comando === "avaliar_dataset")).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Avaliar rótulos conferidos" }));
    const metricas = await screen.findByRole("region", { name: "Métricas da avaliação local" });
    expect(within(metricas).getAllByText("Não calculável")).toHaveLength(5);
    expect(within(metricas).getByRole("table", { name: "Matriz de confusão anotada" })).toHaveTextContent("FN: 0");
    expect(within(metricas).getByText(/Recall restrito aos intervalos/)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("avaliar_dataset", { casoId: caso.id, datasetId: dataset.id, operacaoId: expect.any(String) });
  });
  it("texto de autorização, amostra e notas permanecem inertes", async () => {
    const perigoso = "<script>SINTÉTICO</script>";
    vi.mocked(invoke).mockImplementation((comando) => Promise.resolve(comando === "listar_datasets" ? { total: 1, itens: [{ ...dataset, autorizacao: perigoso }] } : { total: 1, itens: [{ ...amostra, trecho: perigoso, nota: perigoso }] }));
    render(<DatasetsLocais caso={caso} />); await abrirDataset();
    const amostras = await screen.findByRole("list", { name: "Amostras supervisionadas" });
    expect(amostras.querySelector("script,img,a")).toBeNull(); expect(amostras).toHaveTextContent(perigoso);
  });
  it("mudança de caso descarta conclusão tardia da avaliação", async () => {
    const trabalho = pendente<AvaliacaoSupervisionada>(); const original = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation((comando, argumentos) => comando === "avaliar_dataset" ? trabalho.promessa : comando === "listar_datasets" && argumentos && "casoId" in argumentos && argumentos.casoId !== caso.id ? Promise.resolve({ total: 0, itens: [] }) : original(comando, argumentos));
    const tela = render(<DatasetsLocais caso={caso} />); await abrirDataset();
    await userEvent.click(screen.getByRole("button", { name: "Avaliar rótulos conferidos" }));
    tela.rerender(<DatasetsLocais caso={{ ...caso, id: "outro-caso-sintetico" }} />);
    await act(async () => trabalho.resolver(avaliacao));
    expect(screen.queryByRole("region", { name: "Métricas da avaliação local" })).not.toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.filter(([comando]) => comando === "cancelar_descoberta")).toHaveLength(1);
  });
});
