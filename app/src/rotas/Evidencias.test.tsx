import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { ArtefatoResumo, Conversa, Evidencia, RelatorioRegistrado } from "../api/cliente";
import { PaginaConversas } from "./PaginaConversas";
import { PaginaArquivos } from "./PaginasModulos";
import { PaginaRelatorios } from "./PaginaRelatorios";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Dados SINTÉTICOS.
const conversa: Conversa = {
  id: 5,
  extracao_id: "x1",
  app: "whatsapp",
  id_externo: "5511900000011@s.whatsapp.net",
  total_mensagens: 3,
  enviadas: 1,
  com_anexo: 0,
  primeira_ms: Date.UTC(2026, 0, 1, 12),
  ultima_ms: Date.UTC(2026, 0, 1, 12, 2),
  participantes: [{ identificador: "5511900000011@s.whatsapp.net", mensagens: 2, nome: "Ana Sintética" }],
  total_participantes: 1,
  ultima_mensagem: "Terceira",
  titulo: "Ana Sintética",
  tipo: "individual",
  registro_id: null,
};

function msg(id: number, deMim: boolean, minuto: number, conteudo: string): ArtefatoResumo {
  return {
    id,
    tipo: "mensagem",
    subtipo: "texto",
    app: "whatsapp",
    remetente: deMim ? null : "5511900000011@s.whatsapp.net",
    destinatarios: [],
    de_mim: deMim,
    data_hora_ms: Date.UTC(2026, 0, 1, 12, minuto),
    conteudo,
    conversa_id_externo: "5511900000011@s.whatsapp.net",
    estado_registro: "ativo",
    id_externo: null,
    resposta_a_id_externo: null,
    nome_arquivo: null,
    mime: null,
    tamanho: null,
    anexo_arquivo_id: null,
    anexo_vinculo: null,
    latitude: null,
    longitude: null,
  };
}

const mensagens = [msg(1, false, 0, "Primeira"), msg(2, true, 1, "Segunda"), msg(3, false, 2, "Terceira")];

function evidencia(n: number, extra: Partial<Evidencia>): Evidencia {
  return {
    id: 100 + n,
    numero: n,
    tipo: "registro",
    categoria: null,
    titulo: null,
    observacao: null,
    descricao_contextual: null,
    hash_no_momento: `${n}`.repeat(64),
    itens: 1,
    criada_por: "analista",
    criada_em: "2026-01-02T12:00:00Z",
    atualizada_em: "2026-01-02T12:00:00Z",
    registro: null,
    registro_fim: null,
    arquivo: null,
    conversa_id: null,
    ...extra,
  };
}

const lista: Evidencia[] = [
  evidencia(1, { registro: mensagens[0]!, conversa_id: 5, titulo: "Primeira mensagem", categoria: "Ameaças" }),
  evidencia(2, { tipo: "trecho", itens: 3, registro: mensagens[0]!, registro_fim: mensagens[2]!, conversa_id: 5 }),
  evidencia(3, {
    tipo: "arquivo",
    arquivo: {
      id: 7,
      extracao_id: "x1",
      caminho: "DCIM/Camera/foto-sintetica.png",
      sha256: "ab".repeat(32),
      tamanho: 2048,
      categoria: "imagem",
      mime: "image/png",
    },
  }),
];

const gerado: RelatorioRegistrado = {
  id: 1,
  modelo: "padrao-v1",
  secoes: {},
  evidencias: [
    { numero: 1, situacao: "conferida" },
    { numero: 2, situacao: "conferida" },
    { numero: 3, situacao: "divergente" },
  ],
  gerado_por: "analista",
  maquina: "ESTACAO",
  gerado_em: "2026-01-02T15:00:00Z",
  caminho: "relatorios/2026-01-02_120000_relatorio.pdf",
  sha256: "cd".repeat(32),
  tamanho: 123456,
  sied_versao: "0.1.0",
  auditoria_ate_seq: 42,
  auditoria_hash: "ef".repeat(32),
};

let respostas: Record<string, unknown>;
const chamadas: [string, unknown][] = [];

beforeEach(() => {
  chamadas.length = 0;
  respostas = {
    caso_atual: { caso: { id: "c1", fuso_horario: "America/Sao_Paulo", numero_procedimento: "IP 1/2026" }, pasta: "C:\\Casos\\c1" },
    listar_extracoes: [{ id: "x1", dispositivo_id: "d1", importador_nome: "Pasta genérica", iniciada_em: "2026-01-01T12:00:00Z" }],
    listar_dispositivos: [{ id: "d1", apelido: "Celular 1" }],
    listar_conversas: { total: 1, itens: [conversa] },
    obter_conversa: conversa,
    mensagens_conversa: { total: 3, itens: mensagens },
    marcacoes_evidencias: { registros: [[3, 1]], arquivos: [] },
    listar_evidencias: lista,
    listar_relatorios: [],
    adicionar_evidencia: evidencia(4, { registro: mensagens[1]! }),
    gerar_relatorio: gerado,
    totais_por_categoria: [{ categoria: "imagem", arquivos: 1, bytes: 2048 }],
    listar_execucoes: [],
    listar_arquivos: {
      total: 1,
      itens: [
        {
          id: 7,
          caminho: "DCIM/Camera/foto-sintetica.png",
          tamanho: 2048,
          modificado_em: null,
          criado_em: null,
          sha256: "ab".repeat(32),
          assinatura: "png",
          mime: "image/png",
          categoria: "imagem",
          erro: null,
        },
      ],
    },
    miniatura: { estado: "sem_suporte", dados: null, motivo: "sintético" },
  };
  vi.mocked(invoke).mockImplementation(async (cmd: string, args?: unknown) => {
    chamadas.push([cmd, args]);
    if (cmd === "detalhe_artefato") return new Promise(() => {});
    return respostas[cmd] ?? null;
  });
});

function montar(rota: string) {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter initialEntries={[rota]}>
        <Routes>
          <Route path="/casos/:casoId/conversas/:conversaId?" element={<PaginaConversas />} />
          <Route path="/casos/:casoId/relatorios" element={<PaginaRelatorios />} />
          <Route path="/casos/:casoId/arquivos" element={<PaginaArquivos />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

const args = (cmd: string) => chamadas.filter(([c]) => c === cmd).map(([, a]) => a);

describe("marcar evidências", () => {
  it("mensagem, trecho e o número E-n na bolha", async () => {
    montar("/casos/c1/conversas/5");
    const area = await screen.findByLabelText("Mensagens da conversa");
    // A terceira já é a evidência nº 1.
    const [primeira, segunda, terceira] = within(area).getAllByRole("button") as [HTMLElement, HTMLElement, HTMLElement];
    await waitFor(() => expect(within(terceira).getByText("E-1")).toBeInTheDocument());
    expect(within(primeira).queryByText(/^E-/)).not.toBeInTheDocument();

    // Mensagem avulsa.
    await userEvent.click(segunda);
    const barra = screen.getByRole("region", { name: "Evidência" });
    await userEvent.click(within(barra).getByRole("button", { name: "Adicionar às evidências" }));
    await waitFor(() => expect(args("adicionar_evidencia")).toEqual([{ nova: { artefato_id: 2 } }]));
    expect(await within(barra).findByText(/Adicionada como evidência nº 4/)).toBeInTheDocument();

    // Trecho: início na primeira, fim na terceira.
    await userEvent.click(primeira);
    await userEvent.click(within(screen.getByRole("region", { name: "Evidência" })).getByRole("button", { name: "Marcar início do trecho" }));
    const trecho = screen.getByRole("region", { name: "Evidência" });
    expect(within(trecho).getByRole("button", { name: "Adicionar trecho às evidências" })).toBeDisabled();
    await userEvent.click(terceira);
    const fim = screen.getByRole("region", { name: "Evidência" });
    expect(fim).toHaveTextContent("início no registro nº 1");
    expect(fim).toHaveTextContent("Fim no registro nº 3");
    await userEvent.click(within(fim).getByRole("button", { name: "Adicionar trecho às evidências" }));
    await waitFor(() => expect(args("adicionar_evidencia")[1]).toEqual({ nova: { artefato_id: 1, trecho_fim_id: 3 } }));
  });

  it("recusa do backend aparece na barra", async () => {
    respostas.marcacoes_evidencias = { registros: [], arquivos: [] };
    vi.mocked(invoke).mockImplementation(async (cmd: string, a?: unknown) => {
      chamadas.push([cmd, a]);
      if (cmd === "detalhe_artefato") return new Promise(() => {});
      if (cmd === "adicionar_evidencia") throw { codigo: "conflito", mensagem: "isto já está nas evidências (nº 1)", campos: [] };
      return respostas[cmd] ?? null;
    });
    montar("/casos/c1/conversas/5");
    const area = await screen.findByLabelText("Mensagens da conversa");
    await userEvent.click(within(area).getAllByRole("button")[0]!);
    await userEvent.click(screen.getByRole("button", { name: "Adicionar às evidências" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("isto já está nas evidências (nº 1)");
  });

  it("arquivo do inventário pelo painel de detalhes", async () => {
    montar("/casos/c1/arquivos");
    await userEvent.click(await screen.findByText("DCIM/Camera/foto-sintetica.png"));
    const painel = screen.getByLabelText("Evidência do arquivo");
    await userEvent.click(within(painel).getByRole("button", { name: "Adicionar às evidências" }));
    await waitFor(() => expect(args("adicionar_evidencia")).toEqual([{ nova: { arquivo_id: 7 } }]));
  });
});

describe("tela de relatórios", () => {
  it("lista, reordena, edita e remove com motivo", async () => {
    montar("/casos/c1/relatorios");
    const secao = await screen.findByRole("region", { name: "Evidências selecionadas" });
    const e1 = await within(secao).findByRole("listitem", { name: "Evidência E-1" });
    expect(within(e1).getByText("Primeira mensagem")).toBeInTheDocument();
    expect(within(e1).getByText("Ameaças")).toBeInTheDocument();
    expect(within(e1).getByRole("link", { name: "Ver na origem" })).toHaveAttribute("href", "/casos/c1/conversas/5?registro=1");
    const e2 = within(secao).getByRole("listitem", { name: "Evidência E-2" });
    expect(within(e2).getByText("Trecho · 3 mensagens")).toBeInTheDocument();
    expect(within(e2).getByText(/registros nº 1 a 3/)).toBeInTheDocument();
    const e3 = within(secao).getByRole("listitem", { name: "Evidência E-3" });
    expect(within(e3).getByText("foto-sintetica.png")).toBeInTheDocument();
    expect(within(e3).getByRole("button", { name: "Descer E-3" })).toBeDisabled();
    expect(within(e1).getByRole("button", { name: "Subir E-1" })).toBeDisabled();

    await userEvent.click(within(e3).getByRole("button", { name: "Subir E-3" }));
    await waitFor(() => expect(args("mover_evidencia")).toEqual([{ id: 103, numero: 2 }]));

    await userEvent.click(within(e1).getByRole("button", { name: "Editar textos" }));
    const form = within(e1).getByRole("form", { name: "Editar E-1" });
    const titulo = within(form).getByLabelText("Título");
    await userEvent.clear(titulo);
    await userEvent.type(titulo, "Mensagem de ameaça");
    await userEvent.type(within(form).getByLabelText("Observação do analista"), "Conferir com a vítima");
    await userEvent.click(within(form).getByRole("button", { name: "Salvar" }));
    await waitFor(() =>
      expect(args("alterar_evidencia")).toEqual([
        {
          id: 101,
          textos: { categoria: "Ameaças", titulo: "Mensagem de ameaça", observacao: "Conferir com a vítima", descricao_contextual: "" },
        },
      ]),
    );

    await userEvent.click(within(e2).getByRole("button", { name: "Remover" }));
    const rem = within(e2).getByRole("form", { name: "Remover E-2" });
    await userEvent.type(within(rem).getByLabelText("Motivo da remoção"), "duplicada");
    await userEvent.click(within(rem).getByRole("button", { name: "Remover E-2" }));
    await waitFor(() => expect(args("remover_evidencia")).toEqual([{ id: 102, motivo: "duplicada" }]));
  });

  it("gera o PDF com as seções escolhidas e mostra caminho, SHA-256 e conferência", async () => {
    montar("/casos/c1/relatorios");
    const cartao = await screen.findByRole("region", { name: "Gerar relatório" });
    await userEvent.type(within(cartao).getByLabelText("Título"), "Relatório sintético");
    await userEvent.click(within(cartao).getByLabelText("Capa"));
    await userEvent.type(within(cartao).getByLabelText("Texto da conclusão"), "Conclusão sintética");
    await userEvent.click(within(cartao).getByRole("button", { name: "Gerar PDF" }));
    await waitFor(() => expect(args("gerar_relatorio")).toHaveLength(1));
    const pedido = (args("gerar_relatorio")[0] as { pedido: Record<string, unknown> }).pedido;
    expect(pedido.titulo).toBe("Relatório sintético");
    expect(pedido.conclusao).toBe("Conclusão sintética");
    expect(pedido.observacoes).toBeUndefined();
    expect((pedido.secoes as Record<string, boolean>).capa).toBe(false);
    expect((pedido.secoes as Record<string, boolean>).integridade).toBe(true);

    const aviso = await within(cartao).findByText("Relatório gerado");
    const caixa = aviso.closest("[role=status]") as HTMLElement;
    expect(caixa).toHaveTextContent("C:\\Casos\\c1\\relatorios\\2026-01-02_120000_relatorio.pdf");
    expect(caixa).toHaveTextContent("cd".repeat(32));
    expect(caixa).toHaveTextContent("2 de 3 conferidas");
    expect(caixa).toHaveTextContent("divergentes: E-3");
  });

  it("sem evidências explica como marcar; lista os relatórios já gerados", async () => {
    respostas.listar_evidencias = [];
    respostas.listar_relatorios = [gerado];
    montar("/casos/c1/relatorios");
    expect(await screen.findByText("Nenhuma evidência selecionada")).toBeInTheDocument();
    const gerados = screen.getByRole("region", { name: "Relatórios gerados" });
    expect(await within(gerados).findByText("2026-01-02_120000_relatorio.pdf")).toBeInTheDocument();
    expect(within(gerados).getByText(/trilha até o registro 42/)).toBeInTheDocument();
  });
});
