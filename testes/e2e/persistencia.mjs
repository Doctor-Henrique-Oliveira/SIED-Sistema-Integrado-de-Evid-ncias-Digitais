// Teste ponta a ponta do app REAL (binário Tauri + WebView) via WebDriver.
//
// Fluxo: iniciar o SIED → criar caso → cadastrar dispositivo → fechar o programa →
// abrir novamente → conferir que caso, dispositivo e auditoria persistiram.
//
// Requisitos (Linux/CI): tauri-driver (cargo install tauri-driver), WebKitWebDriver e
// um display (Xvfb). Uso:
//   node testes/e2e/persistencia.mjs <caminho do binário sied-desktop>
// Dados de teste ficam numa pasta temporária (SIED_PASTA_DADOS / SIED_PASTA_CASOS).
// Todos os dados usados são sintéticos.

import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync, existsSync } from "node:fs";
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
    const pagina = await s.esperarTexto("Galaxy A54");
    const esperados = ["Inquérito Policial", "4567/2026", "Analista de Teste", "DP Central", IMEI, "128 GB",
      "Caso criado", "Dispositivo cadastrado", "Caso aberto"];
    if (process.env.SIED_E2E_WMCTRL) esperados.push("Caso fechado");
    for (const esperado of esperados) {
      if (!pagina.includes(esperado)) throw new Error(`não encontrado após reabrir: ${esperado}`);
    }
    await s.clicar(botao("Verificar integridade"));
    await s.esperarTexto("Trilha íntegra");
    await s.capturar("sied-3-caso-reaberto");
    await s.fecharJanela();
    console.log(`\nOK: dados persistiram após fechar e reabrir.\nPASTA_DADOS=${env.SIED_PASTA_DADOS}\nPASTA_CASOS=${env.SIED_PASTA_CASOS}`);
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
