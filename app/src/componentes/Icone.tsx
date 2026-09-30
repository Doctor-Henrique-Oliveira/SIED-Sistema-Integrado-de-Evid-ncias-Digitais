// Ícones de traço (24×24), desenhados para o SIED e embutidos no app: nada vem da
// internet. Todos usam `currentColor`, então herdam a cor do texto.

const caminhos = {
  painel: "M4 13h6V4H4zM14 20h6v-9h-6zM14 4v4h6V4zM4 20h6v-3H4z",
  casos: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
  dispositivo: "M8 2.5h8a1.5 1.5 0 0 1 1.5 1.5v16a1.5 1.5 0 0 1-1.5 1.5H8A1.5 1.5 0 0 1 6.5 20V4A1.5 1.5 0 0 1 8 2.5zM11 18.5h2",
  extracao: "M12 3v11m0 0-4-4m4 4 4-4M4 15v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3",
  conversas: "M21 12a8 8 0 0 1-11.6 7.1L4 20.5l1.4-4.9A8 8 0 1 1 21 12z",
  midias: "M4 5h16v14H4zM4 16l5-5 4 4 2-2 5 5M15.5 9.5h.01",
  arquivos: "M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M9 13h6M9 17h6",
  contatos: "M16 20v-1.5A3.5 3.5 0 0 0 12.5 15h-5A3.5 3.5 0 0 0 4 18.5V20M10 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM20 20v-1.5a3.5 3.5 0 0 0-2.5-3.35M15.5 4.15a3.5 3.5 0 0 1 0 6.7",
  localizacoes: "M12 21s-7-6.2-7-11.5a7 7 0 0 1 14 0C19 14.8 12 21 12 21zM12 12a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z",
  consultas: "M4 6h10M4 12h6M4 18h6M16.5 17.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM19 20l1.5 1.5",
  relatorios: "M6 3h9l4 4v14H6zM15 3v4h4M9 12h7M9 16h7M9 8h3",
  config:
    "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z",
  busca: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4",
  escudo: "M12 3 4.5 6v5.5c0 4.6 3.2 8.4 7.5 9.5 4.3-1.1 7.5-4.9 7.5-9.5V6z",
  cadeado: "M6 11h12v10H6zM8.5 11V7.5a3.5 3.5 0 0 1 7 0V11",
  relogio: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5l3 2",
  hash: "M5 9h14M5 15h14M10 4 8 20M16 4l-2 16",
  alerta: "M12 4 2.5 20h19zM12 10v4M12 17h.01",
  check: "M5 12.5 10 17l9-10",
  info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 8h.01",
  fechar: "M6 6l12 12M18 6 6 18",
  abaixo: "m6 9 6 6 6-6",
  direita: "m9 6 6 6-6 6",
  voltar: "M15 6l-6 6 6 6",
  video: "M4 6h11v12H4zM15 10l5-3v10l-5-3",
  audio: "M9 18V6l10-2v12M9 18a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0zM19 16a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0z",
  documento: "M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5",
  banco: "M12 8c4.4 0 8-1.3 8-3s-3.6-3-8-3-8 1.3-8 3 3.6 3 8 3zM4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3",
  compactado: "M5 4h14v16H5zM12 4v2m0 2v2m0 2v2m-1 2h2v3h-2z",
  terminal: "M4 5h16v14H4zM7.5 9.5l3 2.5-3 2.5M12.5 15h4",
  usuario: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4.5 20.5a7.5 7.5 0 0 1 15 0",
  estacao: "M3 5h18v11H3zM8 20h8M12 16v4",
  disco: "M4 6h16v12H4zM4 13h16M16.5 15.5h.01",
  grupo: "M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM2.5 20v-1a4.5 4.5 0 0 1 4.5-4.5h4A4.5 4.5 0 0 1 15.5 19v1M16 4.2a3.5 3.5 0 0 1 0 6.6M21.5 20v-1a4.5 4.5 0 0 0-3-4.2",
  mais: "M12 5v14M5 12h14",
  externo: "M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5",
  integridade: "M12 3 4.5 6v5.5c0 4.6 3.2 8.4 7.5 9.5 4.3-1.1 7.5-4.9 7.5-9.5V6zM8.5 12l2.5 2.5 4.5-5",
  mapa: "M9 4 3 6.5v13.5l6-2.5 6 2.5 6-2.5V4l-6 2.5zM9 4v13.5M15 6.5V20",
  telefone:
    "M6.6 3.5h2.6l1.4 4.1-2 1.4a12 12 0 0 0 6.4 6.4l1.4-2 4.1 1.4v2.6a2 2 0 0 1-2.2 2A16.5 16.5 0 0 1 4.6 5.7a2 2 0 0 1 2-2.2z",
  lista: "M8 6h13M8 12h13M8 18h13M3.5 6h.01M3.5 12h.01M3.5 18h.01",
} as const;

export type NomeIcone = keyof typeof caminhos;

export function Icone({ nome, className = "size-4", titulo }: { nome: NomeIcone; className?: string; titulo?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`shrink-0 ${className}`}
      aria-hidden={titulo ? undefined : true}
      role={titulo ? "img" : undefined}
    >
      {titulo && <title>{titulo}</title>}
      <path d={caminhos[nome]} />
    </svg>
  );
}
