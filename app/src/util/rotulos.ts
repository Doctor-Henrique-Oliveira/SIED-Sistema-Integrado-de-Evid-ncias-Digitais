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
  whatsapp_business: "WhatsApp Business",
  telegram: "Telegram",
};

export function rotuloApp(app: string | null | undefined): string {
  if (!app) return "—";
  return nomeApp[app] ?? app;
}

export const nomeEstadoRegistro: Record<string, string> = {
  ativo: "Ativo",
  apagado: "Apagado",
  recuperado_wal: "Recuperado do WAL",
  desconhecido: "Desconhecido",
};

export const nomeConfianca: Record<string, string> = { alta: "Alta", media: "Média", baixa: "Baixa" };

export const nomeTipoArtefato: Record<string, string> = {
  mensagem: "Mensagem",
  conversa: "Registro da conversa",
  contato: "Contato",
  chamada: "Chamada",
};

/** Subtipos emitidos pelos parsers (o código bruto fica nos detalhes técnicos). */
export const nomeSubtipo: Record<string, string> = {
  texto: "Texto",
  imagem: "Imagem",
  audio: "Áudio",
  video: "Vídeo",
  documento: "Documento",
  gif: "GIF",
  figurinha: "Figurinha",
  contato: "Contato",
  contatos: "Contatos",
  localizacao: "Localização",
  localizacao_tempo_real: "Localização em tempo real",
  sistema: "Mensagem do sistema",
  apagada: "Apagada",
  desconhecido: "Tipo desconhecido",
  individual: "Individual",
  grupo: "Grupo",
};

export function rotuloSubtipo(s: string): string {
  return nomeSubtipo[s] ?? s;
}

export const nomeTipoConversa: Record<string, string> = {
  individual: "Individual",
  grupo: "Grupo",
  status: "Status",
  transmissao: "Lista de transmissão",
  canal: "Canal",
  desconhecido: "Desconhecido",
};

/** Como o anexo informado pelo app foi procurado no inventário da extração. */
export const nomeVinculoAnexo: Record<string, string> = {
  caminho_exato: "Arquivo na extração (mesmo caminho)",
  sufixo_caminho: "Arquivo na extração (mesmo caminho final e tamanho)",
  nome_e_tamanho: "Arquivo na extração (mesmo nome e tamanho)",
  ambiguo: "Mais de um arquivo possível: nenhum foi ligado",
  tamanho_diferente: "Arquivo com o mesmo nome, mas tamanho diferente: não ligado",
  nao_encontrado: "Não encontrado na extração",
};
