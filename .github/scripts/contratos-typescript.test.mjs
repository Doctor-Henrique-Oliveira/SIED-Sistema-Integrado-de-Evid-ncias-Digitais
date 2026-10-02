import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { coletarFontes, coletarMetadados, criarBloco, decodificarLog, exportarContratos, hashManifesto, LIMITES, recuperarContratos, sha256 } from "./contratos-typescript.mjs";

// Apenas textos sintéticos de transporte: nenhuma declaração DTO é inventada.
const TEXTO = Buffer.from("// SINTETICO fixture do transporte\n", "utf8");
const FASE = "armazenamento";

function git(repo, args) {
  return execFileSync("git", args, { cwd: repo, encoding: "utf8", windowsHide: true, stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function escrever(repo, relativo, texto = TEXTO) {
  const destino = path.join(repo, relativo);
  fs.mkdirSync(path.dirname(destino), { recursive: true });
  fs.writeFileSync(destino, texto);
}

function fixture(t) {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "sied-transporte-sintetico-"));
  t.after(() => fs.rmSync(repo, { recursive: true, force: true }));
  git(repo, ["init", "-q"]);
  for (const nome of ["Cargo.toml", "Cargo.lock", ".cargo/config.toml", "crates/sintetico/Cargo.toml", "crates/sintetico/src/lib.rs", "app/src-tauri/Cargo.toml", "app/src-tauri/src/erro.rs"]) escrever(repo, nome);
  git(repo, ["add", "--", "Cargo.toml", "Cargo.lock", ".cargo/config.toml", "crates", "app/src-tauri"]);
  git(repo, ["-c", "user.name=SINTETICO", "-c", "user.email=sintetico@example.invalid", "-c", "commit.gpgsign=false", "commit", "-qm", "SINTETICO fixture do transporte"]);
  const head = git(repo, ["rev-parse", "HEAD"]);
  const meta = coletarMetadados(repo, head, FASE);
  const entradas = [{ nome: "SinteticoA.ts", conteudo: TEXTO }, { nome: "SinteticoB.ts", conteudo: TEXTO }];
  const log = criarBloco(meta, entradas);
  return { repo, head, meta, entradas, log };
}

function editarRegistro(log, indice, atualizar, recalcular = false) {
  const linhas = log.trimEnd().split("\n");
  const metadados = JSON.parse(linhas[0].slice("SIED_TS_V1_BEGIN ".length));
  const registros = linhas.slice(1, -1).map((linha) => JSON.parse(linha.slice("SIED_TS_V1_FILE ".length)));
  atualizar(registros[indice]);
  const fim = JSON.parse(linhas.at(-1).slice("SIED_TS_V1_END ".length));
  if (recalcular) fim.sha256manifest = hashManifesto(metadados, registros);
  return [`SIED_TS_V1_BEGIN ${JSON.stringify(metadados)}`, ...registros.map((registro) => `SIED_TS_V1_FILE ${JSON.stringify(registro)}`), `SIED_TS_V1_END ${JSON.stringify(fim)}`].join("\n") + "\n";
}

function semEscrita(f, log, padrao) {
  const pasta = path.join(f.repo, "app/src/api/tipos");
  assert.throws(() => recuperarContratos(f.repo, log, f.head, FASE), padrao);
  assert.equal(fs.existsSync(pasta), false, "falha não deve nem criar a pasta dos contratos");
}

test("roundtrip conserva bytes, hashes e identifica o checkout real", (t) => {
  const f = fixture(t);
  const prefixado = f.log.trimEnd().split("\n").map((linha) => `Job\tEtapa\t2026-10-02T00:00:00.000Z ${linha}`).join("\r\n");
  const info = recuperarContratos(f.repo, prefixado, f.head, FASE);
  assert.equal(info.total, 2);
  assert.equal(info.checkout, f.head);
  for (const entrada of f.entradas) assert.deepEqual(fs.readFileSync(path.join(f.repo, "app/src/api/tipos", entrada.nome)), TEXTO);
  assert.equal(exportarContratos(f.repo, f.head, FASE), f.log);
});

test("manifesto é determinístico, ligado à fase e independente da ordem de entrada", (t) => {
  const f = fixture(t);
  assert.equal(criarBloco(f.meta, [...f.entradas].reverse()), f.log);
  const outro = criarBloco({ ...f.meta, fase: "workspace" }, f.entradas);
  assert.notEqual(outro, f.log);
  assert.equal(decodificarLog(f.log + outro, f.head, "workspace", coletarFontes(f.repo)).metadados.fase, "workspace");
});

test("checkout de merge distinto é aceito somente com fontes locais idênticas", (t) => {
  const f = fixture(t);
  const log = criarBloco({ ...f.meta, checkout: "a".repeat(40), arvore: "b".repeat(40) }, f.entradas);
  assert.equal(recuperarContratos(f.repo, log, f.head, FASE).checkout, "a".repeat(40));
});

test("bloco incompleto não cria arquivo ou diretório", (t) => {
  const f = fixture(t);
  semEscrita(f, f.log.split("SIED_TS_V1_END")[0], /incompleto/u);
});

test("hash corrompido no último arquivo não publica nem o primeiro", (t) => {
  const f = fixture(t);
  semEscrita(f, editarRegistro(f.log, 1, (registro) => { registro.sha256 = "0".repeat(64); }, true), /divergentes/u);
});

test("Base64 não canônico e espaços são recusados", (t) => {
  const f = fixture(t);
  for (const base64 of ["AA=A", "!!!!", `${TEXTO.toString("base64")}\n`, "AB=="]) {
    semEscrita(f, editarRegistro(f.log, 1, (registro) => { registro.base64 = base64; }, true), /Base64|divergentes/u);
  }
});

test("traversal, caminhos Windows e extensões desconhecidas são recusados", (t) => {
  const f = fixture(t);
  for (const nome of ["../Escape.ts", "C:\\Escape.ts", "/Escape.ts", "Sub/Escape.ts", "Sintetico.txt"]) {
    semEscrita(f, editarRegistro(f.log, 0, (registro) => { registro.nome = nome; }, true), /Nome/u);
  }
});

test("nomes duplicados, colisão de caixa e ordem adulterada são recusados", (t) => {
  const f = fixture(t);
  for (const nome of ["SinteticoA.ts", "sinteticoa.ts", "A.ts"]) {
    semEscrita(f, editarRegistro(f.log, 1, (registro) => { registro.nome = nome; }, true), /duplicados|ordem/u);
  }
});

test("HEAD, fase e árvore inválidos não são aceitos", (t) => {
  const f = fixture(t);
  for (const campo of ["head", "checkout", "arvore"]) assert.throws(() => criarBloco({ ...f.meta, [campo]: "123" }, f.entradas), /SHA|HEAD/u);
  assert.throws(() => decodificarLog(f.log, "f".repeat(40), FASE, f.meta.fontes), /exatamente/u);
  assert.throws(() => decodificarLog(f.log, f.head, "workspace", f.meta.fontes), /exatamente/u);
  assert.throws(() => criarBloco({ ...f.meta, fase: "desconhecida" }, f.entradas), /Fase/u);
});

test("manifesto e contagem adulterados falham sem escrita", (t) => {
  const f = fixture(t);
  for (const fim of [{ total: 2, sha256manifest: "0".repeat(64) }, { total: 1, sha256manifest: "0".repeat(64) }]) {
    semEscrita(f, f.log.replace(/SIED_TS_V1_END .*\n/u, `SIED_TS_V1_END ${JSON.stringify(fim)}\n`), /manifesto|Contagem/u);
  }
});

test("campos desconhecidos, JSON inválido, blocos duplicados e marcadores órfãos falham", (t) => {
  const f = fixture(t);
  const alterado = editarRegistro(f.log, 0, (registro) => { registro.extra = true; }, true);
  for (const log of [alterado, f.log.replace(/SIED_TS_V1_FILE .*\n/u, "SIED_TS_V1_FILE {\n"), f.log + f.log, "SIED_TS_V1_FILE {}\n", "SIED_TS_V1_END {}\n", "SIED_TS_V1_BEGIN\n"]) {
    semEscrita(f, log, /campos|JSON|exatamente|fora|início|malformado/u);
  }
});

test("limites de arquivo, total e quantidade não são ignorados", (t) => {
  const f = fixture(t);
  assert.throws(() => criarBloco(f.meta, [{ nome: "Sintetico.ts", conteudo: Buffer.alloc(LIMITES.arquivo + 1, 65) }]), /Tamanho/u);
  assert.throws(() => criarBloco(f.meta, Array.from({ length: LIMITES.arquivos + 1 }, (_, i) => ({ nome: `Sintetico${i}.ts`, conteudo: TEXTO }))), /Quantidade/u);
  const entradas = Array.from({ length: 129 }, (_, i) => ({ nome: `Sintetico${i}.ts`, conteudo: Buffer.alloc(LIMITES.arquivo, 65) }));
  assert.throws(() => criarBloco(f.meta, entradas), /total/u);
});

test("fonte Rust local modificada, removida ou nova rastreada impede recuperação", (t) => {
  const f = fixture(t);
  escrever(f.repo, "crates/sintetico/src/lib.rs", "// SINTETICO fonte divergente\n");
  semEscrita(f, f.log, /Fontes locais diferem/u);
  escrever(f.repo, "crates/sintetico/src/lib.rs");
  fs.unlinkSync(path.join(f.repo, "app/src-tauri/src/erro.rs"));
  semEscrita(f, f.log, /ENOENT/u);
  escrever(f.repo, "app/src-tauri/src/erro.rs");
  escrever(f.repo, "crates/sintetico/src/nova.rs");
  git(f.repo, ["add", "--", "crates/sintetico/src/nova.rs"]);
  semEscrita(f, f.log, /Fontes locais diferem/u);
});

test("fingerprints incluem Cargo/config/lock dos crates e Rust rastreado, não dados de caso", (t) => {
  const f = fixture(t);
  escrever(f.repo, "casos/NAO_LER.txt", "// SINTETICO fora do transporte\n");
  assert.deepEqual(coletarFontes(f.repo), f.meta.fontes);
  for (const caminho of ["Cargo.toml", "Cargo.lock", ".cargo/config.toml", "crates/sintetico/Cargo.toml", "app/src-tauri/Cargo.toml"]) {
    escrever(f.repo, caminho, "// SINTETICO alterado\n");
    semEscrita(f, f.log, /Fontes locais diferem/u);
    escrever(f.repo, caminho);
  }
});

test("fontes CRLF e LF compartilham fingerprints canônicos sem normalizar contratos", (t) => {
  const f = fixture(t);
  git(f.repo, ["config", "core.autocrlf", "true"]);
  for (const { caminho } of f.meta.fontes) escrever(f.repo, caminho, TEXTO.toString("utf8").replaceAll("\n", "\r\n"));
  assert.deepEqual(coletarFontes(f.repo), f.meta.fontes);
  const contratoCRLF = Buffer.from("// SINTETICO fixture do transporte\r\n", "utf8");
  const log = criarBloco(f.meta, [{ nome: "Sintetico.ts", conteudo: contratoCRLF }]);
  recuperarContratos(f.repo, log, f.head, FASE);
  assert.deepEqual(fs.readFileSync(path.join(f.repo, "app/src/api/tipos/Sintetico.ts")), contratoCRLF);
});

test("fontes novas Rust/Cargo não rastreadas ou ignoradas impedem recuperação", (t) => {
  const f = fixture(t);
  escrever(f.repo, ".git/info/exclude", "*.rs\nCargo.toml\n");
  for (const caminho of ["crates/sintetico/src/nao-rastreado.rs", "crates/novo/Cargo.toml", "app/src-tauri/src/novo.rs"]) {
    escrever(f.repo, caminho);
    semEscrita(f, f.log, /não rastreadas/u);
    fs.unlinkSync(path.join(f.repo, caminho));
  }
});

test("alteração staged e assume-unchanged não ocultam fonte divergente", (t) => {
  const f = fixture(t);
  const caminho = "crates/sintetico/src/lib.rs";
  escrever(f.repo, caminho, "// SINTETICO staged\n");
  git(f.repo, ["add", "--", caminho]);
  semEscrita(f, f.log, /staged/u);
  escrever(f.repo, caminho);
  git(f.repo, ["add", "--", caminho]);
  git(f.repo, ["update-index", "--assume-unchanged", "--", caminho]);
  escrever(f.repo, caminho, "// SINTETICO unstaged\n");
  semEscrita(f, f.log, /unstaged/u);
});

test("destino posterior inválido não sobrescreve contrato anterior", (t) => {
  const f = fixture(t);
  const primeiro = path.join(f.repo, "app/src/api/tipos/SinteticoA.ts");
  escrever(f.repo, "app/src/api/tipos/SinteticoA.ts", "// SINTETICO anterior\n");
  fs.mkdirSync(path.join(f.repo, "app/src/api/tipos/SinteticoB.ts"));
  assert.throws(() => recuperarContratos(f.repo, f.log, f.head, FASE), /regular/u);
  assert.equal(fs.readFileSync(primeiro, "utf8"), "// SINTETICO anterior\n");
});

test("junction/symlink da pasta de destino é recusado antes de escrever", (t) => {
  const f = fixture(t);
  const externo = path.join(f.repo, "fora-dos-tipos");
  fs.mkdirSync(externo);
  fs.mkdirSync(path.join(f.repo, "app/src/api"), { recursive: true });
  fs.symlinkSync(externo, path.join(f.repo, "app/src/api/tipos"), process.platform === "win32" ? "junction" : "dir");
  assert.throws(() => recuperarContratos(f.repo, f.log, f.head, FASE), /Symlink/u);
  assert.deepEqual(fs.readdirSync(externo), []);
});

test("hashes de arquivos são calculados sobre bytes originais, não Base64", () => {
  assert.notEqual(sha256(TEXTO), sha256(TEXTO.toString("base64")));
});
