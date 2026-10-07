import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { ArtefatoResumo, Caso, ConsultaDescobertaSalva, PedidoDescoberta, VersaoVocabularioPesquisa } from "../api/cliente";
import { PaginaBusca } from "./PaginaBusca";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Dados exclusivamente SINTÉTICOS, sem evidência real e sem chamadas externas.
const caso = { id: "c1", fuso_horario: "America/Sao_Paulo", numero_procedimento: "SINTÉTICO 1/2026", numero_bo: null } as Caso;
const reg: ArtefatoResumo = {
  id: 9, tipo: "mensagem", subtipo: "texto", app: "whatsapp", remetente: "Pessoa sintética", destinatarios: [],
  de_mim: false, data_hora_ms: null, conteudo: "Encontro sintético 👨‍👩‍👧‍👦 🤝🏽 com 192.0.2.1",
  conversa_id_externo: "grupo-sintetico", estado_registro: "ativo", id_externo: "9", resposta_a_id_externo: null,
  nome_arquivo: null, mime: null, tamanho: null, anexo_arquivo_id: null, anexo_vinculo: null, latitude: null, longitude: null,
};
const achado = { artefato: reg, extracao_id: "x1", trecho: null, conversa_id: 3 };
const vocab: VersaoVocabularioPesquisa = {
  id: 3, versao: 1, sha256: "b".repeat(64), criado_em: "2026-01-01T00:00:00Z", autor: "Analista sintético",
  dados: { nome: "Vocabulário sintético", origem: "Fixture de teste sintética", ramos: [
    { nome: "Grafias", habilitado: true, termos: ["reunião"] },
    { nome: "Emojis", habilitado: true, termos: ["🤝🏽"] },
  ] },
};
let historico: ConsultaDescobertaSalva[] = [];
let versoes = [vocab];
let rejeitarPagina = false;
let total = 1;
let adiarSalvar = false;
let terminarSalvar: (() => void) | null = null;
let adiarExecucao = false;
let adiarPagina = false;
let cancelamentoAceito = true;
let falhaCancelar = false;
let casoAtualId = "c1";
type OperacaoPendente = { id: string; concluir: () => void; interromper: (codigo: string) => void };
let pendente: OperacaoPendente | null = null;

function resposta(pedido: PedidoDescoberta, vocabularioId: number | null, id = 1) {
  const termos = [pedido.filtro.texto, ...pedido.termos].filter(Boolean);
  const consulta: ConsultaDescobertaSalva = {
    id, sha256: "c".repeat(64), criada_em: "2026-01-01T01:00:00Z", pedido, termos_efetivos: termos,
    algoritmo: "ai0-sintetico-v1", versao_indice: 3, vocabulario_id: vocabularioId, total_artefatos: total,
    total_arquivos: 0, ultimo_artefato_id: 9, ultimo_arquivo_id: 0,
  };
  return { consulta, resultado: {
    modo: pedido.modo, termos_efetivos: termos, algoritmo: consulta.algoritmo, executada_em: consulta.criada_em,
    artefatos: { total, itens: [{ achado, motivos: ["Correspondência sintética no campo conteúdo"], indicadores: pedido.modo === "indicadores" ? [{ tipo: "ip", valor: "192.0.2.1", campo: "conteudo" }] : [] }] },
    arquivos: { total: 0, itens: [] },
  } };
}

beforeEach(() => {
  historico = []; versoes = [vocab]; rejeitarPagina = false; total = 1; adiarSalvar = false; terminarSalvar = null;
  adiarExecucao = false; adiarPagina = false; cancelamentoAceito = true; falhaCancelar = false; casoAtualId = "c1"; pendente = null;
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (cmd: string, args?: unknown) => {
    const a = args as Record<string, unknown> | undefined;
    switch (cmd) {
      case "caso_atual": return { caso: { ...caso, id: casoAtualId }, pasta: "C:/Casos/SINTETICO" };
      case "listar_extracoes": return [{ id: "x1", importador_nome: "Importador sintético", iniciada_em: "2026-01-01T00:00:00Z" }];
      case "listar_vocabularios_pesquisa": return versoes;
      case "listar_consultas_descoberta": return historico;
      case "buscar": return { modo: "texto", artefatos: { total: 1, itens: [achado] }, arquivos: { total: 0, itens: [] } };
      case "executar_descoberta": {
        const r = resposta(a!.pedido as PedidoDescoberta, a!.vocabularioId as number | null, historico.length + 1);
        const concluir = () => { historico = [r.consulta, ...historico]; return r; };
        if (adiarExecucao) return new Promise((resolve, reject) => {
          pendente = { id: a!.operacaoId as string, concluir: () => resolve(concluir()), interromper: (codigo) => reject({ codigo, mensagem: "Interrupção sintética", campos: [] }) };
        });
        return concluir();
      }
      case "paginar_descoberta": {
        if (rejeitarPagina) throw { codigo: "validacao", mensagem: "O universo de dados mudou.", campos: [] };
        const r = resposta(historico[0]!.pedido, historico[0]!.vocabulario_id).resultado;
        if (adiarPagina) return new Promise((resolve, reject) => {
          pendente = { id: a!.operacaoId as string, concluir: () => resolve(r), interromper: (codigo) => reject({ codigo, mensagem: "Interrupção sintética", campos: [] }) };
        });
        return r;
      }
      case "cancelar_descoberta": {
        if (falhaCancelar) throw { codigo: "interno", mensagem: "Falha sintética ao sinalizar", campos: [] };
        return cancelamentoAceito;
      }
      case "salvar_vocabulario_pesquisa": {
        const v = { ...vocab, id: 4, versao: 2, dados: a!.dados as typeof vocab.dados };
        versoes = [v, ...versoes];
        if (adiarSalvar) return new Promise((resolve) => { terminarSalvar = () => resolve(v); });
        return v;
      }
      case "exportar_vocabulario_pesquisa": return JSON.stringify({ formato: "sied-vocabulario-pesquisa-v1", ...vocab });
      default: return undefined;
    }
  });
});

function montar(rota = "/casos/c1/busca") {
  return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })}>
    <MemoryRouter initialEntries={[rota]}><Routes><Route path="/casos/:casoId/busca" element={<PaginaBusca />} /></Routes></MemoryRouter>
  </QueryClientProvider>);
}

describe("busca tradicional preservada", () => {
  it("é o modo inicial e usa somente o comando tradicional, inclusive pela URL", async () => {
    montar("/casos/c1/busca?q=encontro");
    expect(await screen.findByRole("tab", { name: "Tradicional" })).toHaveAttribute("aria-selected", "true");
    expect(await screen.findByText(reg.conteudo!)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("buscar", { filtro: { texto: "encontro" }, deslocamento: 0, limite: 50 });
    expect(invoke).not.toHaveBeenCalledWith("executar_descoberta", expect.anything());
    expect(screen.getByRole("link", { name: /Pessoa sintética/ })).toHaveAttribute("href", "/casos/c1/conversas/3?registro=9");
  });
});

describe("descoberta local AI-0", () => {
  it("não executa ao digitar e envia somente expansões de ramos habilitados", async () => {
    const usuario = userEvent.setup(); montar();
    await usuario.click(await screen.findByRole("tab", { name: "Expansão" }));
    await usuario.selectOptions(await screen.findByRole("combobox", { name: "Versão do vocabulário" }), "3");
    await usuario.click(screen.getByRole("checkbox", { name: "Usar ramo Emojis" }));
    await usuario.type(screen.getByRole("searchbox", { name: "Consulta original" }), "encontro");
    expect(invoke).not.toHaveBeenCalledWith("executar_descoberta", expect.anything());
    await usuario.click(screen.getByRole("button", { name: "Pesquisar no caso" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Salve uma versão");
    expect(invoke).not.toHaveBeenCalledWith("executar_descoberta", expect.anything());
    await usuario.click(screen.getByText("Editar / importar / exportar vocabulário do caso"));
    await usuario.click(screen.getByRole("button", { name: "Salvar nova versão" }));
    await screen.findByText(/Versão salva no caso/);
    expect(invoke).toHaveBeenCalledWith("salvar_vocabulario_pesquisa", { casoId: "c1", dados: { ...vocab.dados, ramos: [vocab.dados.ramos[0], { ...vocab.dados.ramos[1], habilitado: false }] } });
    await usuario.click(screen.getByRole("button", { name: "Pesquisar no caso" }));
    await screen.findByRole("region", { name: "Resultados da descoberta" });
    expect(invoke).toHaveBeenCalledWith("executar_descoberta", { pedido: { filtro: { texto: "encontro" }, modo: "expansao", termos: ["reunião"] }, vocabularioId: 4, operacaoId: expect.any(String), casoId: "c1" });
    expect(screen.getAllByText("encontro · reunião").length).toBeGreaterThan(0);
  });

  it("mantém sequência literal de emojis/ZWJ e apresenta motivo e origem navegável", async () => {
    const usuario = userEvent.setup(); montar();
    await usuario.click(await screen.findByRole("tab", { name: "Literal / emojis" }));
    await usuario.type(screen.getByRole("searchbox", { name: "Consulta original" }), "👨‍👩‍👧‍👦 🤝🏽");
    await usuario.click(screen.getByRole("button", { name: "Pesquisar no caso" }));
    const resultados = await screen.findByRole("region", { name: "Resultados da descoberta" });
    expect(invoke).toHaveBeenCalledWith("executar_descoberta", { pedido: { filtro: { texto: "👨‍👩‍👧‍👦 🤝🏽" }, modo: "literal", termos: [] }, vocabularioId: null, operacaoId: expect.any(String), casoId: "c1" });
    const lista = within(resultados).getByRole("list", { name: "Registros da descoberta" });
    expect(within(lista).getByRole("link")).toHaveAttribute("href", "/casos/c1/conversas/3?registro=9");
    await usuario.click(within(lista).getByText("Por que este resultado apareceu?"));
    expect(within(lista).getByText("Correspondência sintética no campo conteúdo")).toBeVisible();
    expect(within(lista).getByText(/Origem: extração/)).toHaveTextContent("registro 9");
  });

  it("detecta candidatos de indicador sem texto, sem rede ou seleção automática de evidência", async () => {
    const usuario = userEvent.setup(); const rede = vi.spyOn(globalThis, "fetch"); montar();
    await usuario.click(await screen.findByRole("tab", { name: "Indicadores" }));
    expect(invoke).not.toHaveBeenCalledWith("executar_descoberta", expect.anything());
    await usuario.click(screen.getByRole("button", { name: "Pesquisar no caso" }));
    const lista = await screen.findByRole("list", { name: "Registros da descoberta" });
    await usuario.click(within(lista).getByText("Por que este resultado apareceu?"));
    expect(within(lista).getByText("192.0.2.1")).toBeVisible();
    expect(invoke).toHaveBeenCalledWith("executar_descoberta", { pedido: { filtro: { texto: "" }, modo: "indicadores", termos: [], indicador: "ip" }, vocabularioId: null, operacaoId: expect.any(String), casoId: "c1" });
    expect(invoke).not.toHaveBeenCalledWith("adicionar_evidencia", expect.anything());
    expect(rede).not.toHaveBeenCalled(); rede.mockRestore();
  });

  it("rejeita JSON malformado e não salva/importa sem revisão", async () => {
    const usuario = userEvent.setup(); montar();
    await usuario.click(await screen.findByRole("tab", { name: "Expansão" }));
    await usuario.click(screen.getByText("Editar / importar / exportar vocabulário do caso"));
    await usuario.type(screen.getByRole("textbox", { name: "JSON para importar" }), "isso não é JSON");
    await usuario.click(screen.getByRole("button", { name: "Validar e carregar JSON" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("JSON inválido");
    expect(invoke).not.toHaveBeenCalledWith("salvar_vocabulario_pesquisa", expect.anything());
  });

  it("salva importação como nova versão local apenas por ação explícita; exporta JSON somente leitura", async () => {
    const usuario = userEvent.setup(); montar();
    await usuario.click(await screen.findByRole("tab", { name: "Expansão" }));
    await usuario.click(screen.getByText("Editar / importar / exportar vocabulário do caso"));
    fireEvent.change(screen.getByRole("textbox", { name: "JSON para importar" }), { target: { value: JSON.stringify({ formato: "sied-vocabulario-pesquisa-v1", ...vocab }) } });
    await usuario.click(screen.getByRole("button", { name: "Validar e carregar JSON" }));
    expect(screen.getByRole("textbox", { name: "Nome do vocabulário" })).toHaveValue(vocab.dados.nome);
    expect(invoke).not.toHaveBeenCalledWith("salvar_vocabulario_pesquisa", expect.anything());
    await usuario.click(screen.getByRole("button", { name: "Salvar nova versão" }));
    expect(await screen.findByText(/Versão salva no caso/)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("salvar_vocabulario_pesquisa", { casoId: "c1", dados: vocab.dados });
    await usuario.click(screen.getByRole("button", { name: "Preparar JSON de exportação" }));
    const json = await screen.findByRole("textbox", { name: "JSON exportado" });
    expect(json).toHaveAttribute("readonly");
    expect(invoke).toHaveBeenCalledWith("exportar_vocabulario_pesquisa", { casoId: "c1", id: 4 });
    expect(invoke).not.toHaveBeenCalledWith("executar_descoberta", expect.anything());
  });

  it("integra filtros de extração, app, tipo e data sem incluir nomes de arquivo fora do escopo", async () => {
    const usuario = userEvent.setup(); montar();
    await usuario.click(await screen.findByRole("tab", { name: "Literal / emojis" }));
    await usuario.type(screen.getByRole("searchbox", { name: "Consulta original" }), "sintético");
    await usuario.selectOptions(screen.getByRole("combobox", { name: "Extração da descoberta" }), "x1");
    await usuario.selectOptions(screen.getByRole("combobox", { name: "App da descoberta" }), "whatsapp");
    await usuario.selectOptions(screen.getByRole("combobox", { name: "Tipo de registro" }), "mensagem");
    fireEvent.change(screen.getByLabelText("Data inicial da descoberta"), { target: { value: "2026-01-01" } });
    fireEvent.change(screen.getByLabelText("Data final da descoberta"), { target: { value: "2026-01-02" } });
    await usuario.click(screen.getByRole("button", { name: "Pesquisar no caso" }));
    const resultados = await screen.findByRole("region", { name: "Resultados da descoberta" });
    expect(invoke).toHaveBeenCalledWith("executar_descoberta", { pedido: { modo: "literal", termos: [], filtro: {
      texto: "sintético", extracao_id: "x1", app: "whatsapp", tipo: "mensagem", de_ms: 1767236400000, ate_ms: 1767409199999,
    } }, vocabularioId: null, operacaoId: expect.any(String), casoId: "c1" });
    expect(within(resultados).getByText(/Nomes de arquivo fora do escopo/)).toBeInTheDocument();
    expect(within(resultados).queryByRole("table", { name: "Arquivos da descoberta" })).not.toBeInTheDocument();
  });

  it("protege a edição durante gravação pendente e limpa status antigo após uma nova edição", async () => {
    const usuario = userEvent.setup(); montar();
    await usuario.click(await screen.findByRole("tab", { name: "Expansão" }));
    await usuario.selectOptions(await screen.findByRole("combobox", { name: "Versão do vocabulário" }), "3");
    await usuario.click(screen.getByText("Editar / importar / exportar vocabulário do caso"));
    adiarSalvar = true;
    await usuario.click(screen.getByRole("button", { name: "Salvar nova versão" }));
    const nome = screen.getByRole("textbox", { name: "Nome do vocabulário" });
    expect(nome).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: "Usar ramo Emojis" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Pesquisar no caso" })).toBeDisabled();
    await usuario.type(nome, "Edição que não deve sobrescrever resposta");
    expect(nome).toHaveValue(vocab.dados.nome);
    expect(terminarSalvar).not.toBeNull();
    await act(async () => { terminarSalvar!(); });
    await screen.findByText(/Versão salva no caso/);
    expect(nome).toBeEnabled();
    await usuario.type(nome, " · nova edição");
    expect(screen.queryByText(/Versão salva no caso/)).not.toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Versão do vocabulário" })).toHaveValue("");
    expect(screen.getByText(/Árvore alterada/)).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith("executar_descoberta", expect.anything());
  });

  it("paginando, usa ID do snapshot e deixa mudança de universo visível, sem nova execução implícita", async () => {
    total = 60; const usuario = userEvent.setup(); montar();
    await usuario.click(await screen.findByRole("tab", { name: "Literal / emojis" }));
    await usuario.type(screen.getByRole("searchbox", { name: "Consulta original" }), "sintético");
    await usuario.click(screen.getByRole("button", { name: "Pesquisar no caso" }));
    await screen.findByRole("region", { name: "Resultados da descoberta" });
    rejeitarPagina = true; await usuario.click(screen.getByRole("button", { name: "Próxima" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("universo de dados mudou");
    expect(invoke).toHaveBeenCalledWith("paginar_descoberta", { consultaId: 1, deslocamento: 50, limite: 50, operacaoId: expect.any(String), casoId: "c1" });
    expect(vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === "executar_descoberta")).toHaveLength(1);
    expect(screen.queryByRole("region", { name: "Resultados da descoberta" })).not.toBeInTheDocument();
  });

  it("exibe snapshot do histórico sem rodar; reexecução é ação explícita e conserva o anterior", async () => {
    historico = [resposta({ filtro: { texto: "consulta sintética", app: "whatsapp" }, modo: "literal", termos: [] }, null).consulta];
    const usuario = userEvent.setup(); montar();
    await usuario.click(await screen.findByRole("tab", { name: "Indicadores" }));
    await usuario.click(await screen.findByText("consulta sintética", { selector: "summary" }));
    expect(invoke).not.toHaveBeenCalledWith("executar_descoberta", expect.anything());
    expect(screen.getByText(/não é uma cópia congelada/)).toBeInTheDocument();
    await usuario.click(screen.getByRole("button", { name: "Reexecutar consulta #1" }));
    await waitFor(() => expect(historico).toHaveLength(2));
    expect(historico.map((h) => h.id)).toEqual([2, 1]);
  });
});

describe("consulta com limite cooperativo e cancelamento", () => {
  async function iniciarPendente() {
    adiarExecucao = true;
    const usuario = userEvent.setup();
    const montagem = montar();
    await usuario.click(await screen.findByRole("tab", { name: "Literal / emojis" }));
    await usuario.type(screen.getByRole("searchbox", { name: "Consulta original" }), "sintético");
    await usuario.click(screen.getByRole("button", { name: "Pesquisar no caso" }));
    await screen.findByRole("button", { name: "Cancelar consulta" });
    expect(pendente).not.toBeNull();
    return { usuario, montagem, op: pendente! };
  }

  it("sinal aceito mantém pendência até confirmação original e não publica parcial/histórico", async () => {
    const { usuario, op } = await iniciarPendente();
    await usuario.click(screen.getByRole("button", { name: "Cancelar consulta" }));
    expect(invoke).toHaveBeenCalledWith("cancelar_descoberta", { operacaoId: op.id });
    expect(screen.getByRole("button", { name: "Cancelamento solicitado" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Pesquisar no caso" })).toBeDisabled();
    expect(screen.queryByText(/^Consulta cancelada/)).not.toBeInTheDocument();
    expect(historico).toHaveLength(0);
    await act(async () => { op.interromper("descoberta_cancelada"); });
    expect(await screen.findByText(/^Consulta cancelada/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Pesquisar no caso" })).toBeEnabled();
    expect(screen.queryByRole("region", { name: "Resultados da descoberta" })).not.toBeInTheDocument();
    expect(historico).toHaveLength(0);
    expect(vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === "executar_descoberta")).toHaveLength(1);
  });

  it("classifica limite pelo código, sem timeout rígido, zero fictício ou reexecução automática", async () => {
    const { op } = await iniciarPendente();
    await act(async () => { op.interromper("descoberta_limite"); });
    expect(await screen.findByText(/^A consulta atingiu o limite cooperativo/)).toBeInTheDocument();
    expect(screen.getByText(/20 s.*Não é um timeout rígido/)).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Resultados da descoberta" })).not.toBeInTheDocument();
    expect(historico).toHaveLength(0);
    expect(vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === "executar_descoberta")).toHaveLength(1);
  });

  it.each(["descoberta_cancelada", "descoberta_limite"])("preserva a página confirmada e deslocamento ao paginar com %s", async (codigo) => {
    total = 60;
    const usuario = userEvent.setup(); montar();
    await usuario.click(await screen.findByRole("tab", { name: "Literal / emojis" }));
    await usuario.type(screen.getByRole("searchbox", { name: "Consulta original" }), "sintético");
    await usuario.click(screen.getByRole("button", { name: "Pesquisar no caso" }));
    await screen.findByRole("region", { name: "Resultados da descoberta" });
    const primeira = vi.mocked(invoke).mock.calls.find(([cmd]) => cmd === "executar_descoberta")?.[1] as { operacaoId: string };
    adiarPagina = true;
    await usuario.click(screen.getByRole("button", { name: "Próxima" }));
    const op = pendente!;
    expect(op.id).not.toBe(primeira.operacaoId);
    if (codigo === "descoberta_cancelada") await usuario.click(screen.getByRole("button", { name: "Cancelar consulta" }));
    await act(async () => { op.interromper(codigo); });
    expect(await screen.findByText(/A página anterior confirmada foi preservada/)).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Resultados da descoberta" })).toBeInTheDocument();
    expect(screen.getByText("1–50 de 60")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Anterior" })).toBeDisabled();
    expect(historico).toHaveLength(1);
  });

  it("sucesso original prevalece quando o cancelamento chega depois da conclusão", async () => {
    cancelamentoAceito = false;
    const { usuario, op } = await iniciarPendente();
    await usuario.click(screen.getByRole("button", { name: "Cancelar consulta" }));
    expect(await screen.findByText(/O sinal não foi aceito/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancelar consulta" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Pesquisar no caso" })).toBeDisabled();
    expect(screen.queryByText(/^Consulta cancelada/)).not.toBeInTheDocument();
    await act(async () => { op.concluir(); });
    expect(await screen.findByRole("region", { name: "Resultados da descoberta" })).toBeInTheDocument();
    expect(historico).toHaveLength(1);
    expect(screen.queryByText(/^Consulta cancelada/)).not.toBeInTheDocument();
  });

  it("falha ao solicitar cancelamento não libera operação nem inventa encerramento", async () => {
    falhaCancelar = true;
    const { usuario, op } = await iniciarPendente();
    await usuario.click(screen.getByRole("button", { name: "Cancelar consulta" }));
    expect(await screen.findByText(/Não foi possível solicitar cancelamento/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Pesquisar no caso" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Cancelar consulta" })).toBeEnabled();
    expect(screen.queryByText(/^Consulta cancelada/)).not.toBeInTheDocument();
    await act(async () => { op.concluir(); });
    expect(await screen.findByRole("region", { name: "Resultados da descoberta" })).toBeInTheDocument();
  });

  it("desmontar cancela somente o ID ativo e uma resposta tardia não aparece em outro caso", async () => {
    const { montagem, op, usuario } = await iniciarPendente();
    montagem.unmount();
    expect(invoke).toHaveBeenCalledWith("cancelar_descoberta", { operacaoId: op.id });
    casoAtualId = "c2"; historico = []; versoes = [];
    montar("/casos/c2/busca");
    await usuario.click(await screen.findByRole("tab", { name: "Literal / emojis" }));
    await act(async () => { op.concluir(); });
    expect(screen.queryByRole("region", { name: "Resultados da descoberta" })).not.toBeInTheDocument();
    expect(screen.queryByText(reg.conteudo!)).not.toBeInTheDocument();
    expect(screen.getByRole("searchbox", { name: "Consulta original" })).toHaveValue("");
    expect(vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === "cancelar_descoberta")).toHaveLength(1);
  });
});
