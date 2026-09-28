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
  return ms == null ? "—" : dataHora(new Date(ms).toISOString(), fuso);
}

/** Tamanho de arquivo (unidades decimais, como o Windows Explorer usa KB/MB). */
export function tamanho(n: number | null | undefined): string {
  return n === 0 ? "0 B" : bytes(n);
}

export function numero(n: number | null | undefined): string {
  return n == null ? "—" : n.toLocaleString("pt-BR");
}
