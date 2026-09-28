// Textos exibidos para valores codificados do backend.

export const nomeEtapa: Record<string, string> = {
  preparando: "Preparando",
  contando: "Contando arquivos",
  inventariando: "Calculando hashes e classificando arquivos",
  planejando: "Identificando bancos de dados conhecidos",
  processando: "Interpretando dados",
  concluida: "Concluída",
  cancelada: "Cancelada",
  falhou: "Falhou",
};

export const nomeStatusExtracao: Record<string, string> = {
  em_andamento: "Em andamento",
  concluida: "Concluída",
  cancelada: "Cancelada",
  falhou: "Falhou",
  interrompida: "Interrompida",
};

export const nomeStatusExecucao: Record<string, string> = {
  em_andamento: "Em andamento",
  ok: "Concluída",
  parcial: "Concluída com avisos",
  falhou: "Falhou",
  cancelada: "Cancelada",
  interrompida: "Interrompida",
};

export const nomeCategoria: Record<string, string> = {
  banco_sqlite: "Bancos SQLite",
  imagem: "Imagens",
  video: "Vídeos",
  audio: "Áudios",
  documento: "Documentos",
  compactado: "Compactados",
  texto: "Texto",
  vazio: "Vazios",
  outro: "Outros",
};

export const nomeSuporte: Record<string, string> = {
  completo: "Reconhecida",
  parcial: "Aceita (parcial)",
  nao_implementado: "Ainda não disponível",
  nao_reconhecido: "Não reconhecida",
};

/** Tom visual por situação: verde (ok), âmbar (atenção), vermelho (erro), cinza. */
export function tomStatus(status: string): "ok" | "atencao" | "erro" | "neutro" {
  if (status === "concluida" || status === "ok") return "ok";
  if (status === "parcial" || status === "cancelada" || status === "interrompida") return "atencao";
  if (status === "falhou") return "erro";
  return "neutro";
}

export const nomeApp: Record<string, string> = {
  sms: "SMS",
  whatsapp: "WhatsApp",
  telegram: "Telegram",
};

export function rotuloApp(app: string | null | undefined): string {
  if (!app) return "—";
  return nomeApp[app] ?? app;
}
