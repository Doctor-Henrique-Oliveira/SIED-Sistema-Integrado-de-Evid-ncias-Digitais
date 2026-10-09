#!/usr/bin/env node
// Transporte dos bytes produzidos pelo ts-rs; este script não gera declarações TS.
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const LIMITES = Object.freeze({ arquivo: 64 * 1024, total: 8 * 1024 * 1024, arquivos: 512, log: 64 * 1024 * 1024 });
const PASTA_TIPOS = "app/src/api/tipos";
const FONTES_FIXAS = [".cargo/config.toml", "Cargo.lock", "Cargo.toml"];
const FASES = ["armazenamento", "workspace"];
const SHA40 = /^[a-f0-9]{40}$/u;
const SHA256 = /^[a-f0-9]{64}$/u;
const NOME_TS = /^[A-Za-z][A-Za-z0-9_]*\.ts$/u;
const BASE64 = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/u;

function exigir(condicao, mensagem) {
  if (!condicao) throw new Error(mensagem);
}

export function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function objetoExato(valor, chaves, contexto) {
  exigir(valor !== null && typeof valor === "object" && !Array.isArray(valor), `${contexto}: objeto esperado.`);
  const atuais = Object.keys(valor).sort();
  exigir(JSON.stringify(atuais) === JSON.stringify([...chaves].sort()), `${contexto}: campos desconhecidos ou ausentes.`);
}

function validarHead(head) {
  exigir(typeof head === "string" && SHA40.test(head), "HEAD esperado deve ser um SHA Git completo de 40 caracteres minúsculos.");
}

function validarFase(fase) {
  exigir(FASES.includes(fase), "Fase deve ser armazenamento ou workspace.");
}

function fontePermitida(caminho) {
  if (typeof caminho !== "string" || /[\\\0\r\n]/u.test(caminho)) return false;
  if (caminho.split("/").some((parte) => !parte || parte === "." || parte === "..")) return false;
  const noWorkspace = caminho.startsWith("crates/") || caminho.startsWith("app/src-tauri/");
  return FONTES_FIXAS.includes(caminho) || (noWorkspace && (caminho.endsWith(".rs") || caminho.endsWith("/Cargo.toml")));
}

export function validarFontes(fontes) {
  exigir(Array.isArray(fontes) && fontes.length >= FONTES_FIXAS.length && fontes.length <= 10000, "Lista de fontes inválida.");
  let anterior = "";
  const nomes = new Set();
  for (const fonte of fontes) {
    objetoExato(fonte, ["caminho", "sha256"], "Fonte");
    exigir(fontePermitida(fonte.caminho) && typeof fonte.sha256 === "string" && SHA256.test(fonte.sha256), "Caminho ou hash de fonte inválido.");
    exigir(fonte.caminho > anterior && !nomes.has(fonte.caminho.toLowerCase()), "Fontes duplicadas ou fora de ordem.");
    anterior = fonte.caminho;
    nomes.add(fonte.caminho.toLowerCase());
  }
  exigir(FONTES_FIXAS.every((caminho) => nomes.has(caminho.toLowerCase())), "Fingerprint das fontes obrigatórias ausente.");
  return fontes.map(({ caminho, sha256: hash }) => ({ caminho, sha256: hash }));
}

export function validarMetadados(valor) {
  objetoExato(valor, ["versao", "fase", "head", "checkout", "arvore", "fontes"], "Metadados");
  exigir(valor.versao === 1, "Versão de transporte desconhecida.");
  validarHead(valor.head);
  validarFase(valor.fase);
  exigir(typeof valor.checkout === "string" && SHA40.test(valor.checkout), "SHA do checkout inválido.");
  exigir(typeof valor.arvore === "string" && SHA40.test(valor.arvore), "SHA da árvore do checkout inválido.");
  return { versao: 1, fase: valor.fase, head: valor.head, checkout: valor.checkout, arvore: valor.arvore, fontes: validarFontes(valor.fontes) };
}

function git(repo, argumentos) {
  return execFileSync("git", argumentos, { cwd: repo, encoding: "utf8", maxBuffer: 4 * 1024 * 1024, windowsHide: true }).trimEnd();
}

// Também rejeita junctions/symlinks nos diretórios intermediários no Windows.
function verificarCaminho(repo, relativo, permitirAusente = false) {
  const partes = relativo.split("/");
  exigir(partes.every((parte) => parte && parte !== "." && parte !== ".." && !/[\\\0\r\n]/u.test(parte)), "Caminho relativo inválido.");
  let atual = path.resolve(repo);
  const raiz = fs.lstatSync(atual);
  exigir(raiz.isDirectory() && !raiz.isSymbolicLink(), "Raiz do repositório deve ser um diretório sem symlink.");
  for (let indice = 0; indice < partes.length; indice += 1) {
    atual = path.join(atual, partes[indice]);
    let estado;
    try { estado = fs.lstatSync(atual); } catch (erro) {
      if (permitirAusente && erro.code === "ENOENT") continue;
      throw erro;
    }
    exigir(!estado.isSymbolicLink(), `Symlink recusado: ${relativo}.`);
    if (indice < partes.length - 1) exigir(estado.isDirectory(), `Diretório intermediário inválido: ${relativo}.`);
  }
  return atual;
}

export function coletarFontes(repo) {
  const escopo = [".cargo/config.toml", "Cargo.toml", "Cargo.lock", "crates", "app/src-tauri"];
  const outros = git(repo, ["ls-files", "--others", "-z", "--", ...escopo]).split("\0").filter(Boolean).filter(fontePermitida);
  exigir(outros.length === 0, "Fontes Rust/Cargo não rastreadas: recuperação/exportação recusada.");
  const registrosHead = git(repo, ["ls-tree", "-r", "-z", "HEAD", "--", ...escopo]).split("\0").filter(Boolean).map((registro) => {
    const campos = registro.match(/^(\d{6}) (blob|tree|commit) ([a-f0-9]{40})\t(.*)$/u);
    exigir(campos !== null, "Registro da árvore Git inválido.");
    return { modo: campos[1], tipo: campos[2], oid: campos[3], caminho: campos[4] };
  }).filter(({ caminho }) => fontePermitida(caminho)).sort((a, b) => a.caminho < b.caminho ? -1 : a.caminho > b.caminho ? 1 : 0);
  exigir(registrosHead.length >= FONTES_FIXAS.length && registrosHead.length <= 10000, "Quantidade de fontes inválida.");
  const indice = git(repo, ["ls-files", "--stage", "-z", "--", ...escopo]).split("\0").filter(Boolean).map((registro) => {
    const campos = registro.match(/^(\d{6}) ([a-f0-9]{40}) ([0-3])\t(.*)$/u);
    exigir(campos !== null, "Registro do índice Git inválido.");
    return { modo: campos[1], oid: campos[2], estagio: campos[3], caminho: campos[4] };
  }).filter(({ caminho }) => fontePermitida(caminho));
  exigir(indice.length === registrosHead.length, "Fontes locais diferem do HEAD: índice divergente.");
  const porNome = new Map(indice.map((entrada) => [entrada.caminho, entrada]));
  for (const entrada of registrosHead) {
    const atual = porNome.get(entrada.caminho);
    exigir(entrada.tipo === "blob" && atual && atual.estagio === "0" && atual.oid === entrada.oid && atual.modo === entrada.modo, "Fontes locais diferem do HEAD: alteração staged ou conflito no índice.");
    const arquivo = verificarCaminho(repo, entrada.caminho);
    exigir(fs.lstatSync(arquivo).isFile(), `Fonte não é arquivo regular: ${entrada.caminho}.`);
  }
  // --stdin-paths aplica os filtros Git do caminho (inclusive core.autocrlf),
  // sem -w: confere bytes do worktree mesmo com assume-unchanged, sem gravar objetos.
  const oidsLocais = execFileSync("git", ["hash-object", "--stdin-paths"], {
    cwd: repo, encoding: "utf8", maxBuffer: 4 * 1024 * 1024, windowsHide: true,
    input: registrosHead.map(({ caminho }) => JSON.stringify(caminho)).join("\n") + "\n",
  }).trimEnd().split("\n");
  exigir(oidsLocais.length === registrosHead.length && oidsLocais.every((oid, i) => oid.trim() === registrosHead[i].oid), "Fontes locais diferem do HEAD: conteúdo unstaged divergente.");
  // Fingerprints SHA-256 dos blobs canônicos Git, não do CRLF do Windows.
  const blobs = execFileSync("git", ["cat-file", "--batch"], {
    cwd: repo, maxBuffer: 64 * 1024 * 1024, windowsHide: true,
    input: registrosHead.map(({ oid }) => oid).join("\n") + "\n",
  });
  let posicao = 0;
  const fontes = registrosHead.map((entrada) => {
    const fimCabecalho = blobs.indexOf(10, posicao);
    exigir(fimCabecalho >= 0, "Resposta Git incompleta.");
    const cabecalho = blobs.subarray(posicao, fimCabecalho).toString("utf8").match(/^([a-f0-9]{40}) blob (\d+)$/u);
    exigir(cabecalho && cabecalho[1] === entrada.oid, "Objeto Git inesperado.");
    const tamanho = Number(cabecalho[2]);
    const inicio = fimCabecalho + 1;
    const fim = inicio + tamanho;
    exigir(Number.isSafeInteger(tamanho) && tamanho >= 0 && fim < blobs.length && blobs[fim] === 10, "Tamanho do blob Git inválido.");
    posicao = fim + 1;
    return { caminho: entrada.caminho, sha256: sha256(blobs.subarray(inicio, fim)) };
  });
  exigir(posicao === blobs.length, "Conteúdo Git excedente na resposta.");
  return validarFontes(fontes);
}

export function coletarMetadados(repo, head, fase) {
  validarHead(head);
  validarFase(fase);
  return validarMetadados({ versao: 1, fase, head, checkout: git(repo, ["rev-parse", "HEAD"]), arvore: git(repo, ["rev-parse", "HEAD^{tree}"]), fontes: coletarFontes(repo) });
}

function conferirRegistros(arquivos) {
  exigir(Array.isArray(arquivos) && arquivos.length > 0 && arquivos.length <= LIMITES.arquivos, "Quantidade de contratos inválida.");
  let anterior = "";
  let total = 0;
  const nomes = new Set();
  return arquivos.map((arquivo) => {
    objetoExato(arquivo, ["nome", "sha256", "bytes", "base64"], "Contrato");
    exigir(typeof arquivo.nome === "string" && NOME_TS.test(arquivo.nome), "Nome de contrato inválido; caminhos e traversal não são permitidos.");
    exigir(arquivo.nome > anterior && !nomes.has(arquivo.nome.toLowerCase()), "Contratos duplicados ou fora de ordem.");
    anterior = arquivo.nome;
    nomes.add(arquivo.nome.toLowerCase());
    exigir(Number.isSafeInteger(arquivo.bytes) && arquivo.bytes > 0 && arquivo.bytes <= LIMITES.arquivo, "Tamanho do contrato inválido.");
    total += arquivo.bytes;
    exigir(total <= LIMITES.total, "Contratos excedem o tamanho total permitido.");
    exigir(typeof arquivo.sha256 === "string" && SHA256.test(arquivo.sha256), "Hash do contrato inválido.");
    exigir(typeof arquivo.base64 === "string" && arquivo.base64.length <= Math.ceil(LIMITES.arquivo / 3) * 4 && BASE64.test(arquivo.base64), "Base64 do contrato inválido.");
    const conteudo = Buffer.from(arquivo.base64, "base64");
    exigir(conteudo.toString("base64") === arquivo.base64, "Base64 não canônico.");
    exigir(conteudo.length === arquivo.bytes && sha256(conteudo) === arquivo.sha256, "Bytes ou SHA-256 do contrato divergentes.");
    return { nome: arquivo.nome, sha256: arquivo.sha256, bytes: arquivo.bytes, base64: arquivo.base64, conteudo };
  });
}

export function hashManifesto(metadados, arquivos) {
  const meta = validarMetadados(metadados);
  const lista = arquivos.map(({ nome, sha256: hash, bytes }) => ({ nome, sha256: hash, bytes }));
  return sha256(Buffer.from(JSON.stringify({ metadados: meta, arquivos: lista }), "utf8"));
}

export function criarBloco(metadados, entradas) {
  const meta = validarMetadados(metadados);
  const registros = entradas.map(({ nome, conteudo }) => {
    exigir(Buffer.isBuffer(conteudo), "Conteúdo deve conter os bytes originais do arquivo.");
    return { nome, sha256: sha256(conteudo), bytes: conteudo.length, base64: conteudo.toString("base64") };
  }).sort((a, b) => a.nome < b.nome ? -1 : a.nome > b.nome ? 1 : 0);
  conferirRegistros(registros);
  return [
    `SIED_TS_V1_BEGIN ${JSON.stringify(meta)}`,
    ...registros.map((registro) => `SIED_TS_V1_FILE ${JSON.stringify(registro)}`),
    `SIED_TS_V1_END ${JSON.stringify({ total: registros.length, sha256manifest: hashManifesto(meta, registros) })}`,
  ].join("\n") + "\n";
}

export function exportarContratos(repo, head, fase) {
  const metadados = coletarMetadados(repo, head, fase);
  const pasta = verificarCaminho(repo, PASTA_TIPOS);
  exigir(fs.lstatSync(pasta).isDirectory(), "Pasta de tipos inválida.");
  const nomes = fs.readdirSync(pasta).filter((nome) => nome.endsWith(".ts"));
  exigir(nomes.length <= LIMITES.arquivos, "Quantidade de contratos excedida.");
  const entradas = nomes.map((nome) => {
    exigir(NOME_TS.test(nome), "Nome de contrato inválido.");
    const arquivo = verificarCaminho(repo, `${PASTA_TIPOS}/${nome}`);
    const estado = fs.lstatSync(arquivo);
    exigir(estado.isFile() && estado.size > 0 && estado.size <= LIMITES.arquivo, "Contrato deve ser um arquivo regular dentro do limite.");
    return { nome, conteudo: fs.readFileSync(arquivo) };
  });
  return criarBloco(metadados, entradas);
}

export function decodificarLog(texto, head, fase, fontesLocais) {
  validarHead(head);
  validarFase(fase);
  exigir(typeof texto === "string" && Buffer.byteLength(texto, "utf8") <= LIMITES.log, "Log inválido ou maior que o limite.");
  const fontes = validarFontes(fontesLocais);
  const blocos = [];
  let ativo = null;
  for (const linha of texto.split(/\r?\n/u)) {
    // gh run view --log também inclui prefixos de job/etapa/timestamp.
    const marcador = linha.match(/(?:^|\s)SIED_TS_V1_(BEGIN|FILE|END)\s(.*)$/u);
    if (!marcador) {
      exigir(!linha.includes("SIED_TS_V1_"), "Marcador de transporte malformado.");
      continue;
    }
    let valor;
    try { valor = JSON.parse(marcador[2]); } catch { throw new Error("JSON do transporte inválido."); }
    if (marcador[1] === "BEGIN") {
      exigir(ativo === null, "Bloco anterior incompleto.");
      ativo = { metadados: validarMetadados(valor), arquivos: [] };
    } else if (marcador[1] === "FILE") {
      exigir(ativo !== null, "Contrato fora de um bloco.");
      exigir(ativo.arquivos.length < LIMITES.arquivos, "Quantidade de contratos excedida.");
      ativo.arquivos.push(valor);
    } else {
      exigir(ativo !== null, "Fim de bloco sem início.");
      objetoExato(valor, ["total", "sha256manifest"], "Fim do bloco");
      const arquivos = conferirRegistros(ativo.arquivos);
      exigir(valor.total === arquivos.length && typeof valor.sha256manifest === "string" && SHA256.test(valor.sha256manifest), "Contagem ou manifesto inválidos.");
      exigir(hashManifesto(ativo.metadados, arquivos) === valor.sha256manifest, "SHA-256 do manifesto divergente.");
      if (ativo.metadados.head === head && ativo.metadados.fase === fase) blocos.push({ ...ativo, arquivos, sha256manifest: valor.sha256manifest });
      ativo = null;
    }
  }
  exigir(ativo === null, "Bloco incompleto: nenhum contrato foi recuperado.");
  exigir(blocos.length === 1, "É necessário exatamente um bloco completo do HEAD e da fase esperados.");
  const bloco = blocos[0];
  exigir(JSON.stringify(bloco.metadados.fontes) === JSON.stringify(fontes), "As fontes locais diferem do checkout que gerou os contratos.");
  return bloco;
}

export function recuperarContratos(repo, texto, head, fase) {
  validarHead(head);
  exigir(git(repo, ["rev-parse", "HEAD"]) === head, "HEAD local difere do HEAD esperado; recuperação recusada.");
  const bloco = decodificarLog(texto, head, fase, coletarFontes(repo));
  const pasta = verificarCaminho(repo, PASTA_TIPOS, true);
  if (fs.existsSync(pasta)) exigir(fs.lstatSync(pasta).isDirectory(), "Pasta de tipos inválida.");
  // Valida TODOS os destinos antes da primeira criação/gravação de arquivo.
  const destinos = bloco.arquivos.map((arquivo) => {
    const destino = verificarCaminho(repo, `${PASTA_TIPOS}/${arquivo.nome}`, true);
    if (fs.existsSync(destino)) exigir(fs.lstatSync(destino).isFile(), "Destino de contrato não é arquivo regular.");
    return { destino, conteudo: arquivo.conteudo };
  });
  fs.mkdirSync(pasta, { recursive: true });
  for (const { destino, conteudo } of destinos) fs.writeFileSync(destino, conteudo);
  return { total: destinos.length, head: bloco.metadados.head, checkout: bloco.metadados.checkout, fase: bloco.metadados.fase, sha256manifest: bloco.sha256manifest };
}

function executarCli(argumentos) {
  const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
  if (argumentos[0] === "exportar" && argumentos.length === 2) {
    process.stdout.write(exportarContratos(repo, process.env.SIED_CONTRATOS_HEAD, argumentos[1]));
    return;
  }
  if (argumentos[0] === "recuperar" && argumentos.length === 4) {
    exigir(fs.statSync(argumentos[1]).size <= LIMITES.log, "Log excede o tamanho permitido.");
    const log = fs.readFileSync(argumentos[1]);
    exigir(log.length <= LIMITES.log, "Log excede o tamanho permitido.");
    const texto = new TextDecoder("utf-8", { fatal: true }).decode(log);
    console.log(JSON.stringify(recuperarContratos(repo, texto, argumentos[2], argumentos[3])));
    return;
  }
  throw new Error("Uso: exportar armazenamento|workspace; recuperar <logPath> <headSHA40> <fase>.");
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try { executarCli(process.argv.slice(2)); } catch (erro) {
    console.error(`Transporte de contratos recusado: ${erro.message}`);
    process.exitCode = 1;
  }
}
