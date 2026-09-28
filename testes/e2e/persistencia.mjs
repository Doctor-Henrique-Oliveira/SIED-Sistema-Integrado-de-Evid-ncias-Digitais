// Teste ponta a ponta do app REAL (binário Tauri + WebView) via WebDriver.
//
// Fluxo: iniciar o SIED → criar caso → cadastrar dispositivo → fechar o programa →
// abrir novamente → conferir que caso, dispositivo e auditoria persistiram →
// importar uma extração SINTÉTICA (testes/gerador) → conferir registros, origem,
// inventário e manifesto → buscar e abrir o registro na conversa → provar que a
// extração não mudou → fechar e reabrir.
//
// Requisitos (Linux/CI): tauri-driver (cargo install tauri-driver), WebKitWebDriver e
// um display (Xvfb). Uso:
//   node testes/e2e/persistencia.mjs <caminho do binário sied-desktop>
// Dados de teste ficam numa pasta temporária (SIED_PASTA_DADOS / SIED_PASTA_CASOS).
// Todos os dados usados são sintéticos.

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, writeFileSync, existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const binario = resolve(process.argv[2] ?? "target/debug/sied-desktop");
const saidaCapturas = process.env.SIED_E2E_CAPTURAS ?? tmpdir();
const raiz = mkdtempSync(join(tmpdir(), "sied-e2e-"));
const env = {
  ...process.env,
  SIED_PASTA_DADOS: join(raiz, "dados"),
  SIED_PASTA_CASOS: join(raiz, "Casos"),
  WEBKIT_DISABLE_DMABUF_RENDERER: "1",
  WEBKIT_DISABLE_COMPOSITING_MODE: "1",
};
const URL_WD = "http://127.0.0.1:4444";
const ELEMENTO = "element-6066-11e4-a52e-4f735466cecf";

const esperar = (ms) => new Promise((r) => setTimeout(r, ms));

const sha256 = (arquivo) => createHash("sha256").update(readFileSync(arquivo)).digest("hex");

/** Hash e data de modificação de cada arquivo da pasta. */
function retrato(pasta) {
  const r = {};
  const visitar = (p) => {
    for (const n of readdirSync(p, { withFileTypes: true })) {
      const c = join(p, n.name);
      if (n.isDirectory()) visitar(c);
      else r[c] = `${sha256(c)} ${statSync(c).mtimeMs}`;
    }
  };
  visitar(pasta);
  return JSON.stringify(r);
}

async function wd(metodo, caminho, corpo) {
  const r = await fetch(URL_WD + caminho, {
    method: metodo,
    headers: { "content-type": "application/json" },
    body: corpo ? JSON.stringify(corpo) : undefined,
  });
  const json = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(`${metodo} ${caminho}: ${r.status} ${JSON.stringify(json.value ?? json)}`);
  return json.value;
}

async function ateConseguir(descricao, fn, tempo = 15000) {
  const fim = Date.now() + tempo;
  let ultimo;
  while (Date.now() < fim) {
    try {
      const v = await fn();
      if (v) return v;
    } catch (e) {
      ultimo = e;
    }
    await esperar(250);
  }
  throw new Error(`tempo esgotado: ${descricao}${ultimo ? ` (${ultimo.message})` : ""}`);
}

class Sessao {
  static async abrir() {
    const v = await wd("POST", "/session", {
      capabilities: { alwaysMatch: { "tauri:options": { application: binario } } },
    });
    const s = new Sessao();
    s.id = v.sessionId;
    return s;
  }
  p(c) {
    return `/session/${this.id}${c}`;
  }
  async achar(xpath) {
    return ateConseguir(`elemento ${xpath}`, async () => {
      const el = await wd("POST", this.p("/element"), { using: "xpath", value: xpath });
      return el[ELEMENTO];
    });
  }
  async clicar(xpath) {
    await wd("POST", this.p(`/element/${await this.achar(xpath)}/click`), {});
  }
  async digitar(nome, texto) {
    const id = await this.achar(`//*[@name='${nome}']`);
    await wd("POST", this.p(`/element/${id}/value`), { text: texto });
  }
  async digitarEm(xpath, texto) {
    await wd("POST", this.p(`/element/${await this.achar(xpath)}/value`), { text: texto });
  }
  async conteudo() {
    return wd("POST", this.p("/execute/sync"), { script: "return document.body.textContent", args: [] });
  }
  async texto(xpath) {
    return wd("GET", this.p(`/element/${await this.achar(xpath)}/text`));
  }
  async esperarTexto(trecho, tempo) {
    return ateConseguir(`texto "${trecho}"`, async () => {
      // textContent: o /text do WebKitWebDriver omite elementos com text-overflow.
      const corpo = await this.conteudo();
      return corpo.includes(trecho) ? corpo : null;
    }, tempo);
  }
  async capturar(nome) {
    const png = await wd("GET", this.p("/screenshot"));
    const arq = join(saidaCapturas, `${nome}.png`);
    writeFileSync(arq, Buffer.from(png, "base64"));
    console.log(`  captura: ${arq}`);
  }
  /** Fecha a janela como o usuário faria (dispara o encerramento normal do app). */
  async fecharJanela() {
    if (process.env.SIED_E2E_WMCTRL) {
      // Com gerenciador de janelas: pede o fechamento como o botão "X" da janela.
      spawnSync("wmctrl", ["-c", "SIED —"]);
      await esperar(2000);
    } else {
      // Sem gerenciador de janelas, o WebDriver encerra o processo abruptamente
      // (equivale a queda de energia): o registro "caso_fechado" não é gravado.
      await wd("DELETE", this.p("/window")).catch(() => {});
    }
    await esperar(1500);
    await wd("DELETE", this.p("")).catch(() => {});
  }
}

const botao = (rotulo) => `//button[normalize-space()='${rotulo}']`;
const IMEI = "490154203237518"; // dígito verificador válido; não é de aparelho real conhecido

let atual = null;

async function principal() {
  if (!existsSync(binario)) throw new Error(`binário não encontrado: ${binario}`);
  console.log(`binário: ${binario}\ndados de teste: ${raiz}`);
  const driver = spawn("tauri-driver", [], { env, stdio: ["ignore", "inherit", "inherit"] });
  try {
    await ateConseguir("tauri-driver", async () => (await fetch(`${URL_WD}/status`)).ok, 10000);

    console.log("1. iniciar o SIED e criar um caso");
    let s = await Sessao.abrir();
    atual = s;
    await s.esperarTexto("Nenhum caso nesta estação");
    await s.clicar(botao("Novo caso"));
    await s.digitar("procedimento", "Inquérito Policial");
    await s.digitar("numero_procedimento", "IP 0123/2026");
    await s.digitar("numero_bo", "4567/2026");
    await s.digitar("delegado", "Dra. Exemplo (sintético)");
    await s.digitar("responsavel_analise", "Analista de Teste");
    await s.digitar("unidade", "DP Central");
    await s.clicar(botao("Criar caso"));
    await s.esperarTexto("Nenhum dispositivo cadastrado");

    console.log("2. cadastrar um dispositivo (com IMEI inválido primeiro)");
    await s.clicar(botao("Adicionar dispositivo"));
    await s.digitar("apelido", "Celular 1 – investigado");
    await s.digitar("fabricante", "Samsung");
    await s.digitar("modelo", "SM-A546E");
    await s.digitar("nome_comercial", "Galaxy A54");
    await s.digitar("versao_sistema", "14");
    await s.digitar("imei1", "490154203237519");
    await s.digitar("capacidade", "128 GB");
    await s.clicar(botao("Salvar dispositivo"));
    await s.esperarTexto("dígito verificador não confere");
    const imei = await s.achar("//*[@name='imei1']");
    await wd("POST", s.p(`/element/${imei}/clear`), {});
    await s.digitar("imei1", IMEI);
    await s.clicar(botao("Salvar dispositivo"));
    await s.esperarTexto("Galaxy A54");
    await s.capturar("sied-1-caso-criado");

    console.log("3. fechar o programa");
    await s.fecharJanela();

    console.log("4. abrir novamente e conferir");
    s = await Sessao.abrir();
    atual = s;
    await s.esperarTexto("IP 0123/2026");
    await s.capturar("sied-2-lista-apos-reabrir");
    await s.clicar("//a[.//p[normalize-space()='IP 0123/2026']]");
    await s.esperarTexto("Galaxy A54");
    // Dispositivos e atividades carregam separadamente: espera as duas seções.
    const pagina = await s.esperarTexto("Caso criado");
    const esperados = ["Inquérito Policial", "4567/2026", "Analista de Teste", "DP Central", IMEI, "128 GB",
      "Caso criado", "Dispositivo cadastrado", "Caso aberto"];
    if (process.env.SIED_E2E_WMCTRL) esperados.push("Caso fechado");
    for (const esperado of esperados) {
      if (!pagina.includes(esperado)) throw new Error(`não encontrado após reabrir: ${esperado}`);
    }
    await s.clicar(botao("Verificar integridade"));
    await s.esperarTexto("Trilha íntegra");
    await s.capturar("sied-3-caso-reaberto");

    console.log("5. importar uma extração sintética");
    const extracao = join(raiz, "Extracao-SINTETICA");
    const gerador = spawnSync("python3", ["testes/gerador/extracao_sintetica.py", extracao], { stdio: "inherit" });
    if (gerador.status !== 0) throw new Error("falha ao gerar a extração sintética");
    const antes = retrato(extracao);
    await s.clicar(botao("Importar extração"));
    await s.digitarEm("//input[@id='pasta-extracao']", extracao);
    await s.clicar(botao("Analisar"));
    await s.esperarTexto("Ainda não disponível"); // Avilla: só contrato, sem presumir estrutura
    await s.esperarTexto("Aceita (parcial)");
    await s.capturar("sied-4-assistente-importacao");
    await s.clicar(botao("Iniciar importação"));
    await s.esperarTexto("registros interpretados", 60000);
    await s.capturar("sied-5-importacao-concluida");

    console.log("6. conferir registros, origem técnica, inventário e manifesto");
    await s.clicar("//a[normalize-space()='Abrir extração']");
    await s.esperarTexto("Registros interpretados (6)");
    const tela = await s.esperarTexto("Mensagem que só existe no WAL");
    for (const esperado of ["Vamos nos encontrar amanhã às 10h?", "+5521900000002", "Recebida", "Enviada", "Pasta genérica"]) {
      if (!tela.includes(esperado)) throw new Error(`não encontrado na extração: ${esperado}`);
    }
    await s.clicar("//tr[td[contains(., 'Vamos nos encontrar')]]/td[1]");
    const tecnico = await s.esperarTexto("_id=3");
    for (const esperado of ["dados/com.android.providers.telephony/databases/mmssms.db", "SELECT * FROM sms ORDER BY _id",
      "android_sms v1.0.0", "unix_milissegundos"]) {
      if (!tecnico.includes(esperado)) throw new Error(`detalhe técnico ausente: ${esperado}`);
    }
    const hashDb = sha256(join(extracao, "dados/com.android.providers.telephony/databases/mmssms.db"));
    if (!tecnico.includes(hashDb)) throw new Error("SHA-256 do banco de origem não aparece nos detalhes técnicos");
    await s.capturar("sied-6-detalhes-tecnicos");

    const pastaCaso = join(env.SIED_PASTA_CASOS, readdirSync(env.SIED_PASTA_CASOS)[0]);
    const pastaImp = join(pastaCaso, "importacoes");
    const manifesto = join(pastaImp, readdirSync(pastaImp)[0], "manifesto.sha256");
    const hashManifesto = sha256(manifesto);
    if (!tecnico.includes(hashManifesto)) throw new Error("hash do manifesto na tela difere do arquivo");
    for (const linha of readFileSync(manifesto, "utf8").trim().split("\n")) {
      const [h, caminho] = linha.split(" *");
      if (sha256(join(extracao, caminho)) !== h) throw new Error(`manifesto não confere: ${caminho}`);
    }
    await s.clicar("//button[@role='tab'][starts-with(normalize-space(), 'Arquivos')]");
    const inventario = await s.esperarTexto("Bancos SQLite (1)");
    if (!inventario.includes("IMG_0001.dat")) throw new Error("arquivo de mídia ausente no inventário");
    await s.capturar("sied-7-inventario");
    if (retrato(extracao) !== antes) throw new Error("a pasta da extração foi alterada!");

    console.log("6b. buscar, abrir na conversa e ver a origem");
    await s.clicar("//a[normalize-space()='← Voltar ao caso']");
    await s.clicar("//a[normalize-space()='Buscar']");
    await s.digitarEm("//input[@id='texto-busca']", "encontrar");
    await s.clicar(botao("Buscar"));
    await s.esperarTexto("Registros (1)");
    await s.capturar("sied-9-busca");
    await s.clicar("//a[@data-teste='achado']");
    const chat = await s.esperarTexto("_id=3");
    for (const esperado of ["Oi, tudo bem? (mensagem sintética)", "Combinado.", "4 mensagens (2 enviadas)"]) {
      if (!chat.includes(esperado)) throw new Error(`conversa sem: ${esperado}`);
    }
    await s.capturar("sied-10-conversa-com-origem");
    await s.clicar("//a[normalize-space()='Buscar em tudo']");
    await s.digitarEm("//input[@id='texto-busca']", "(21) 90000-0002");
    await s.clicar(botao("Buscar"));
    await s.esperarTexto("Registros (1) · busca por telefone");
    await s.clicar("//a[normalize-space()='← Voltar ao caso']");
    await s.clicar("//a[normalize-space()='Conversas']");
    const conversas = await s.esperarTexto("Mensagem que só existe no WAL");
    if ((conversas.match(/mensagens ·/g) ?? []).length !== 3) throw new Error("esperadas 3 conversas");

    console.log("7. fechar, reabrir e conferir a extração e a auditoria");
    await s.fecharJanela();
    s = await Sessao.abrir();
    atual = s;
    await s.clicar("//a[.//p[normalize-space()='IP 0123/2026']]");
    const reaberto = await s.esperarTexto("Importação concluída");
    if (!reaberto.includes("Pasta genérica")) throw new Error("extração não listada após reabrir");
    await s.clicar(botao("Verificar integridade"));
    await s.esperarTexto("Trilha íntegra");
    await s.capturar("sied-8-caso-com-extracao");
    await s.fecharJanela();
    if (retrato(extracao) !== antes) throw new Error("a pasta da extração foi alterada!");
    console.log(`\nOK: dados persistiram; importação, busca e conversas conferidas; extração intacta.\nPASTA_DADOS=${env.SIED_PASTA_DADOS}\nPASTA_CASOS=${env.SIED_PASTA_CASOS}`);
  } catch (e) {
    if (atual) {
      await atual.capturar("sied-falha").catch(() => {});
      const corpo = await atual.conteudo().catch(() => "(sem corpo)");
      console.error(`--- texto da tela ---\n${corpo}\n---`);
    }
    throw e;
  } finally {
    driver.kill();
  }
}

principal().catch(async (e) => {
  console.error(`\nFALHOU: ${e.message}`);
  process.exit(1);
});
