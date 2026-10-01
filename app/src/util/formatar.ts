const FUSO_PADRAO = "America/Sao_Paulo";

/** Data e hora em pt-BR no fuso do caso (padrão: Brasília). */
export function dataHora(iso: string | null | undefined, fuso: string = FUSO_PADRAO): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return new Intl.DateTimeFormat("pt-BR", {
    timeZone: fuso,
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(d);
}

/** Capacidade em unidades decimais (como fabricantes anunciam: 128 GB). */
export function bytes(n: number | null | undefined): string {
  if (n == null) return "—";
  const unidades = ["B", "KB", "MB", "GB", "TB"];
  let v = n;
  let i = 0;
  while (v >= 1000 && i < unidades.length - 1) {
    v /= 1000;
    i++;
  }
  const casas = v >= 100 || i === 0 ? 0 : 1;
  return `${v.toLocaleString("pt-BR", { maximumFractionDigits: casas })} ${unidades[i]}`;
}

/** Converte "128 GB", "64gb", "512" (GB) em bytes. Retorna null se vazio; NaN se inválido. */
export function lerCapacidade(texto: string): number | null {
  const t = texto.trim().toUpperCase().replace(",", ".");
  if (!t) return null;
  const m = /^(\d+(?:\.\d+)?)\s*(TB|GB|MB)?$/.exec(t);
  if (!m) return Number.NaN;
  const fator = { TB: 1e12, GB: 1e9, MB: 1e6 }[m[2] ?? "GB"] ?? 1e9;
  return Math.round(Number(m[1]) * fator);
}

/** Texto vazio vira null (o backend trata ausente e vazio da mesma forma). */
export function ouNulo(texto: string): string | null {
  const t = texto.trim();
  return t ? t : null;
}

export function identificacaoCaso(c: {
  numero_procedimento: string | null;
  numero_bo: string | null;
}): string {
  return c.numero_procedimento ?? (c.numero_bo ? `BO ${c.numero_bo}` : "Caso sem número");
}

/** Data a partir de milissegundos UTC, no fuso do caso. */
export function dataHoraMs(ms: number | null | undefined, fuso: string = FUSO_PADRAO): string {
  if (ms == null) return "—";
  const d = dataValida(ms);
  return d ? dataHora(d.toISOString(), fuso) : foraDoIntervalo(ms);
}

/** `Date` exibível, ou `null` (fora de ±8,64e15 ms o JavaScript dá "Invalid Date", e
 *  formatá-la lança exceção e derrubaria a tela inteira). */
function dataValida(ms: number): Date | null {
  const d = new Date(ms);
  return Number.isNaN(d.getTime()) ? null : d;
}

function foraDoIntervalo(ms: number): string {
  return `data fora do intervalo (${ms})`;
}

/** Tamanho de arquivo (unidades decimais, como o Windows Explorer usa KB/MB). */
export function tamanho(n: number | null | undefined): string {
  return n === 0 ? "0 B" : bytes(n);
}

export function numero(n: number | null | undefined): string {
  return n == null ? "—" : n.toLocaleString("pt-BR");
}

/** Dia por extenso no fuso do caso, para os separadores da conversa. */
export function dia(ms: number, fuso: string = FUSO_PADRAO): string {
  const d = dataValida(ms);
  if (!d) return foraDoIntervalo(ms);
  return new Intl.DateTimeFormat("pt-BR", {
    timeZone: fuso,
    weekday: "long",
    day: "2-digit",
    month: "long",
    year: "numeric",
  }).format(d);
}

/** Hora com segundos no fuso do caso (o segundo importa em análise forense). */
export function hora(ms: number, fuso: string = FUSO_PADRAO): string {
  const d = dataValida(ms);
  if (!d) return foraDoIntervalo(ms);
  return new Intl.DateTimeFormat("pt-BR", {
    timeZone: fuso,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(d);
}

/** Início de um dia (AAAA-MM-DD) no fuso do caso, em ms UTC. `fim` pega o último ms do dia. */
export function limiteDia(data: string, fuso: string = FUSO_PADRAO, fim = false): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(data);
  if (!m) return null;
  const [a, me, d] = [Number(m[1]), Number(m[2]) - 1, Number(m[3])];
  // Descobre o deslocamento do fuso naquele dia e corrige (ida e volta pelo Intl).
  const utc = Date.UTC(a, me, d + (fim ? 1 : 0));
  const partes = new Intl.DateTimeFormat("en-US", {
    timeZone: fuso,
    hourCycle: "h23",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).formatToParts(new Date(utc));
  const v = (t: string) => Number(partes.find((p) => p.type === t)?.value);
  const comoLocal = Date.UTC(v("year"), v("month") - 1, v("day"), v("hour"), v("minute"), v("second"));
  const ms = utc - (comoLocal - utc);
  return fim ? ms - 1 : ms;
}

/** Duração entre dois instantes ISO ("2 min 13 s"); "—" se faltar um deles. */
export function duracao(inicio: string | null | undefined, fim: string | null | undefined): string {
  if (!inicio || !fim) return "—";
  const ms = new Date(fim).getTime() - new Date(inicio).getTime();
  if (!Number.isFinite(ms) || ms < 0) return "—";
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min ${s % 60} s`;
  return `${Math.floor(m / 60)} h ${m % 60} min`;
}

/** Extensão do arquivo em minúsculas, sem o ponto ("" quando não há). */
export function extensao(caminho: string): string {
  const nome = caminho.split(/[\\/]/).pop() ?? "";
  const i = nome.lastIndexOf(".");
  return i > 0 ? nome.slice(i + 1).toLowerCase() : "";
}

/** Último componente do caminho. */
export function nomeArquivo(caminho: string): string {
  return caminho.split(/[\\/]/).pop() ?? caminho;
}

/**
 * Serviço de nuvem que sincroniza a pasta, pelo nome de algum componente do caminho
 * (no Windows 11 a pasta Documentos costuma ficar dentro do OneDrive), ou pasta de rede
 * (caminho UNC). `null` quando parece local. Unidade de rede mapeada (Z:) não é
 * reconhecível pelo caminho: `localDaPasta` junta a resposta do sistema operacional.
 */
export function servicoDeNuvem(caminho: string): string | null {
  // Pasta de rede (\\servidor\pasta): o SQLite em modo WAL não é confiável fora do disco local.
  if (/^(\\\\|\/\/)/.test(caminho)) return "compartilhamento de rede";
  for (const parte of caminho.split(/[\\/]/)) {
    const p = parte.toLowerCase();
    if (p === "onedrive" || p.startsWith("onedrive - ")) return "OneDrive";
    if (p === "dropbox" || p.startsWith("dropbox (")) return "Dropbox";
    if (p === "google drive" || p === "googledrive" || p === "meu drive" || p === "my drive") return "Google Drive";
    if (p === "iclouddrive" || p === "icloud drive") return "iCloud";
  }
  return null;
}

/** Onde a pasta está, pelo caminho e pela resposta do sistema (`emRede`). `null` se local. */
export function localDaPasta(caminho: string, emRede: boolean | undefined): string | null {
  return servicoDeNuvem(caminho) ?? (emRede ? "compartilhamento de rede" : null);
}

/** Duração em segundos: "42 s", "2 min 05 s", "1 h 02 min 03 s". */
export function duracaoSegundos(s: number | null | undefined): string {
  if (s == null) return "—";
  const h = Math.floor(s / 3600);
  const min = Math.floor((s % 3600) / 60);
  const seg = s % 60;
  const dois = (n: number) => String(n).padStart(2, "0");
  if (h > 0) return `${h} h ${dois(min)} min ${dois(seg)} s`;
  if (min > 0) return `${min} min ${dois(seg)} s`;
  return `${seg} s`;
}
