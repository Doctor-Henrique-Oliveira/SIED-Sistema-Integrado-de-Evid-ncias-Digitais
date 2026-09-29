// Telas com volume real: listas virtualizadas, filtros no banco, localizar em
// conversa longa, miniaturas e progresso da importação. Dados SINTÉTICOS.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { ArquivoExtracao, ArtefatoResumo, Caso, Conversa, EstadoImportacao } from "../api/cliente";
import { BarraSuperior } from "../componentes/BarraSuperior";
import { PainelImportacao } from "../componentes/Importacao";
import { PaginaConversas } from "./PaginaConversas";
import { PaginaMidias } from "./PaginasModulos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// O jsdom não calcula layout: a área de rolagem ganha 600 px de altura (e um
// conteúdo alto o bastante para rolar) para o virtualizador saber quantas linhas
// cabem, e scrollTo passa a mover a rolagem.
const medidas = { offsetHeight: 600, offsetWidth: 800, clientHeight: 600, scrollHeight: 10_000_000 } as const;
const originais = {
  medidas: Object.fromEntries(Object.keys(medidas).map((k) => [k, Object.getOwnPropertyDescriptor(HTMLElement.prototype, k) ?? Object.getOwnPropertyDescriptor(Element.prototype, k)])),
  rolar: Element.prototype.scrollTo,
};
beforeAll(() => {
  for (const [k, v] of Object.entries(medidas)) Object.defineProperty(HTMLElement.prototype, k, { configurable: true, get: () => v });
  Element.prototype.scrollTo = function (this: Element, opcoes?: ScrollToOptions | number) {
    const topo = typeof opcoes === "object" ? (opcoes.top ?? 0) : 0;
    this.scrollTop = topo;
    this.dispatchEvent(new Event("scroll"));
  } as Element["scrollTo"];
});
afterAll(() => {
  for (const [k, d] of Object.entries(originais.medidas)) {
    if (Object.getOwnPropertyDescriptor(HTMLElement.prototype, k) && d) Object.defineProperty(HTMLElement.prototype, k, d);
    else delete (HTMLElement.prototype as unknown as Record<string, unknown>)[k];
  }
  Element.prototype.scrollTo = originais.rolar;
});

const caso = { id: "c1", fuso_horario: "America/Sao_Paulo", numero_procedimento: "IP 1/2026", numero_bo: null } as Caso;
const TOTAL_CONVERSAS = 250;
const TOTAL_MENSAGENS = 1200;
const TOTAL_MIDIAS = 300;

function conversa(i: number): Conversa {
  return {
    id: i + 1,
    extracao_id: "x1",
    app: "sms",
    id_externo: String(i + 1),
    total_mensagens: i === 0 ? TOTAL_MENSAGENS : 1,
    enviadas: 0,
    com_anexo: 0,
    primeira_ms: Date.UTC(2026, 0, 1),
    ultima_ms: Date.UTC(2026, 0, 1),
    participantes: [{ identificador: `+55619000${String(i).padStart(5, "0")}`, mensagens: 1, nome: null }],
    total_participantes: 1,
    ultima_mensagem: `Última da conversa ${i + 1}`,
    titulo: null,
    tipo: null,
    registro_id: null,
  };
}

function mensagem(i: number): ArtefatoResumo {
  return {
    id: i + 1,
    tipo: "mensagem",
    subtipo: "texto",
    app: "sms",
    remetente: "+5561900000000",
    destinatarios: [],
    de_mim: false,
    data_hora_ms: Date.UTC(2026, 0, 1) + i * 60_000,
    conteudo: `Mensagem sintética ${i + 1}`,
    conversa_id_externo: "1",
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

function arquivo(i: number): ArquivoExtracao {
  return {
    id: i + 1,
    caminho: `DCIM/IMG_${String(i + 1).padStart(4, "0")}.jpg`,
    tamanho: 2048,
    modificado_em: null,
    criado_em: null,
    sha256: "ab".repeat(32),
    assinatura: "jpeg",
    mime: "image/jpeg",
    categoria: "imagem",
    erro: null,
  } as ArquivoExtracao;
}

function pagina<T>(total: number, gerar: (i: number) => T, desl: number, lim: number) {
  const itens = Array.from({ length: Math.max(0, Math.min(lim, total - desl)) }, (_, k) => gerar(desl + k));
  return { total, itens };
}

// 1×1 PNG transparente, só para o <img> ter uma fonte.
const PNG = "data:image/jpeg;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

function responder(cmd: string, a: Record<string, unknown> = {}): unknown {
  const desl = Number(a.deslocamento ?? 0);
  const lim = Number(a.limite ?? 100);
  switch (cmd) {
    case "caso_atual":
      return { caso, pasta: "C:/Casos/c1" };
    case "listar_extracoes":
      return [{ id: "x1", dispositivo_id: "d1", importador_nome: "Pasta genérica", iniciada_em: "2026-01-01T12:00:00Z" }];
    case "listar_dispositivos":
      return [{ id: "d1", apelido: "Celular 1" }];
    case "apps_das_conversas":
      return [
        { app: "sms", conversas: 200 },
        { app: "whatsapp", conversas: 50 },
      ];
    case "listar_conversas": {
      // Com qualquer filtro, o "banco" devolve só a conversa 7.
      if (a.texto || a.app || a.tipo || a.comAnexo) return { total: 1, itens: [conversa(6)] };
      return pagina(TOTAL_CONVERSAS, conversa, desl, lim);
    }
    case "obter_conversa":
      return conversa(Number(a.conversaId) - 1);
    case "mensagens_conversa":
      if (Number(a.conversaId) === 2)
        return {
          total: 1,
          itens: [{ ...mensagem(0), id: 5000, subtipo: "imagem", nome_arquivo: "IMG-20260101-WA0001.jpg", mime: "image/jpeg", tamanho: 2048, anexo_arquivo_id: 1, anexo_vinculo: "unico" }],
        };
      return pagina(TOTAL_MENSAGENS, mensagem, desl, lim);
    case "localizar_na_conversa":
      return {
        total: 3,
        itens: [
          { id: 900, posicao: 899 },
          { id: 1000, posicao: 999 },
          { id: 1100, posicao: 1099 },
        ],
      };
    case "posicao_mensagem":
      return null;
    case "totais_por_categoria":
      return [{ categoria: "imagem", arquivos: TOTAL_MIDIAS, bytes: TOTAL_MIDIAS * 2048 }];
    case "listar_execucoes":
      return [];
    case "listar_arquivos":
      return pagina(TOTAL_MIDIAS, arquivo, desl, lim);
    case "miniatura":
      return Number(a.arquivoId) === 2
        ? { estado: "origem_divergente", dados: null, motivo: "O arquivo na extração difere do inventário (SHA-256)." }
        : { estado: "pronta", dados: PNG, motivo: null };
    default:
      return undefined;
  }
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (cmd: string, a?: unknown) => responder(cmd, (a ?? {}) as Record<string, unknown>));
});

function chamadas(cmd: string) {
  return vi.mocked(invoke).mock.calls.filter(([c]) => c === cmd).map(([, a]) => (a ?? {}) as Record<string, unknown>);
}

function montar(rota: string, caminho: string, elemento: ReactNode) {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter initialEntries={[rota]}>
        <Routes>
          <Route path={caminho} element={elemento} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("conversas com volume real", () => {
  it("lista longa só desenha as linhas visíveis e busca as páginas seguintes no banco", async () => {
    montar("/casos/c1/conversas", "/casos/:casoId/conversas/:conversaId?", <PaginaConversas />);
    const lista = await screen.findByRole("list", { name: "Lista de conversas" });
    expect(await within(lista).findByText("Última da conversa 1")).toBeInTheDocument();
    expect(screen.getByText(`${TOTAL_CONVERSAS} conversas`)).toBeInTheDocument();
    const linhas = within(lista).getAllByRole("listitem");
    expect(linhas.length).toBeGreaterThan(0);
    expect(linhas.length).toBeLessThan(40);
    expect(within(lista).queryByText(`Última da conversa ${TOTAL_CONVERSAS}`)).toBeNull();

    // Rolar até o fim traz a última página do banco.
    const area = lista.parentElement!;
    area.scrollTo({ top: 1_000_000 });
    expect(await within(lista).findByText(`Última da conversa ${TOTAL_CONVERSAS}`)).toBeInTheDocument();
    expect(chamadas("listar_conversas").some((a) => a.deslocamento === 200)).toBe(true);
  });

  it("filtros vão para o banco e valem para o caso inteiro", async () => {
    const usuario = userEvent.setup();
    montar("/casos/c1/conversas", "/casos/:casoId/conversas/:conversaId?", <PaginaConversas />);
    await screen.findByText(`${TOTAL_CONVERSAS} conversas`);

    await usuario.click(screen.getByRole("button", { name: "WhatsApp (50)" }));
    await usuario.click(screen.getByRole("button", { name: "Grupos" }));
    await usuario.click(screen.getByRole("button", { name: /Com anexo/ }));
    await usuario.type(screen.getByRole("searchbox", { name: "Filtrar conversas" }), "61 90000");

    expect(await screen.findByText("1 conversa com os filtros")).toBeInTheDocument();
    await waitFor(() =>
      expect(chamadas("listar_conversas")).toContainEqual(
        expect.objectContaining({ texto: "61 90000", app: "whatsapp", tipo: "grupo", comAnexo: true, deslocamento: 0 }),
      ),
    );
    await usuario.click(screen.getByRole("button", { name: "Limpar filtros" }));
    expect(await screen.findByText(`${TOTAL_CONVERSAS} conversas`)).toBeInTheDocument();
  });

  it("conversa longa é virtualizada e localizar pergunta ao banco e rola até o achado", async () => {
    const usuario = userEvent.setup();
    montar("/casos/c1/conversas/1", "/casos/:casoId/conversas/:conversaId?", <PaginaConversas />);
    const area = await screen.findByLabelText("Mensagens da conversa");
    expect(await within(area).findByText("Mensagem sintética 1")).toBeInTheDocument();
    const bolhas = within(area).getAllByRole("button");
    expect(bolhas.length).toBeLessThan(60);
    expect(within(area).queryByText(`Mensagem sintética ${TOTAL_MENSAGENS}`)).toBeNull();

    await usuario.type(screen.getByRole("searchbox", { name: "Localizar na conversa" }), "sintética 9");
    expect(await screen.findByText("1 de 3")).toBeInTheDocument();
    expect(chamadas("localizar_na_conversa")).toContainEqual(expect.objectContaining({ conversaId: 1, termo: "sintética 9" }));
    // A mensagem 900 (posição 899) está na segunda página: o banco é consultado e ela aparece.
    // (O trecho achado vem destacado, então o texto está dividido em elementos.)
    await waitFor(() => expect(area.textContent).toContain("Mensagem sintética 900"), { timeout: 3000 });
    expect(chamadas("mensagens_conversa").some((a) => a.deslocamento === 500)).toBe(true);
  });
});

describe("anexos nas conversas", () => {
  it("anexo de imagem ligado ao inventário mostra a miniatura conferida", async () => {
    montar("/casos/c1/conversas/2", "/casos/:casoId/conversas/:conversaId?", <PaginaConversas />);
    const anexo = await screen.findByLabelText("Anexo");
    expect(within(anexo).getByText("IMG-20260101-WA0001.jpg")).toBeInTheDocument();
    await waitFor(() => expect(anexo.querySelector('[data-miniatura="pronta"] img')).not.toBeNull());
    expect(chamadas("miniatura")).toContainEqual({ arquivoId: 1 });
  });
});

describe("mídias com volume real", () => {
  it("grade longa virtualizada com miniaturas conferidas e aviso quando o arquivo difere", async () => {
    montar("/casos/c1/midias", "/casos/:casoId/midias", <PaginaMidias />);
    const grade = await screen.findByRole("list", { name: "Mídias" });
    expect(await screen.findByText(`${TOTAL_MIDIAS} itens · carregados conforme a rolagem`)).toBeInTheDocument();
    const cartoes = within(grade).getAllByRole("button");
    expect(cartoes.length).toBeGreaterThan(0);
    expect(cartoes.length).toBeLessThan(TOTAL_MIDIAS);

    await waitFor(() => expect(grade.querySelectorAll('[data-miniatura="pronta"] img').length).toBeGreaterThan(0));
    await waitFor(() => expect(within(grade).getByText("Arquivo difere do inventário")).toBeInTheDocument());
    expect(chamadas("miniatura")).toContainEqual({ arquivoId: 1 });
  });
});

describe("progresso da importação", () => {
  function estado(progresso: Partial<EstadoImportacao["progresso"]>): EstadoImportacao {
    return {
      extracao_id: "x1",
      dispositivo_id: "d1",
      terminada: false,
      extracao: null,
      erro: null,
      progresso: {
        etapa: "processando",
        arquivos_total: 10,
        arquivos_feitos: 10,
        bytes_total: 2000,
        bytes_feitos: 2000,
        tarefas_total: 4,
        tarefas_feitas: 1,
        artefatos: 120,
        avisos: 2,
        item_atual: "data/data/com.android.providers.telephony/databases/mmssms.db",
        mensagem: null,
        inicio_ms: Date.now() - 125_000,
        etapa_inicio_ms: Date.now() - 5_000,
        fase_tarefa: "copiando_verificando",
        copia_bytes_total: 4_000_000,
        copia_bytes_feitos: 1_000_000,
        ...progresso,
      },
    };
  }

  function painel(e: EstadoImportacao) {
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter>
          <PainelImportacao caso={caso} estado={e} aoDispensar={() => {}} />
        </MemoryRouter>
      </QueryClientProvider>,
    );
  }

  it("mostra a etapa atual, a fração dos bancos e a cópia verificada em curso", () => {
    painel(estado({}));
    const passos = screen.getByRole("list", { name: "Etapas da importação" });
    expect(within(passos).getAllByRole("listitem")).toHaveLength(4);
    expect(within(passos).getByText("Cópia verificada e leitura").closest("li")).toHaveAttribute("aria-current", "step");
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "25");
    expect(screen.getByText(/Copiando para a pasta do caso e conferindo o SHA-256: 1 MB de 4 MB \(25%\)/)).toBeInTheDocument();
    expect(screen.getByText("Decorrido: 2 min 5 s")).toBeInTheDocument();
    expect(screen.getByText("Avisos: 2")).toBeInTheDocument();
  });

  it("etapa sem medida deixa a barra indeterminada", () => {
    painel(estado({ etapa: "contando", fase_tarefa: undefined, copia_bytes_total: undefined, copia_bytes_feitos: undefined }));
    expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");
    expect(screen.queryByText(/Copiando para a pasta/)).toBeNull();
  });

  it("inventário mostra taxa e tempo restante medidos desde o início da etapa", () => {
    painel(
      estado({
        etapa: "inventariando",
        bytes_total: 10_000_000,
        bytes_feitos: 4_000_000,
        etapa_inicio_ms: Date.now() - 4_000,
        fase_tarefa: undefined,
        copia_bytes_total: undefined,
        copia_bytes_feitos: undefined,
      }),
    );
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "40");
    expect(screen.getByText(/B\/s · restam cerca de 6 s/)).toBeInTheDocument();
  });

  it("a barra superior resume etapa e percentual", async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) =>
      cmd === "estado_importacao" ? estado({}) : responder(cmd),
    );
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter>
          <BarraSuperior caso={caso} info={undefined} />
        </MemoryRouter>
      </QueryClientProvider>,
    );
    const pilula = await screen.findByRole("link", { name: /Importando/ });
    await waitFor(() => expect(pilula).toHaveTextContent("Importando · etapa 4/4 · 25%"));
    expect(pilula).toHaveAttribute("href", "/casos/c1/extracoes");
  });
});
