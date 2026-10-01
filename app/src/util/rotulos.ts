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
  telefone: "Telefone",
  contatos_android: "Agenda (Android)",
  iped: "IPED (lista de itens)",
  aleapp: "ALEAPP",
  ileapp: "iLEAPP",
  xleapp: "ALEAPP/iLEAPP",
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
  arquivo: "Arquivo",
  outro: "Outro registro",
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
  recebida: "Recebida",
  efetuada: "Efetuada",
  perdida: "Perdida",
  correio_de_voz: "Correio de voz",
  rejeitada: "Rejeitada",
  bloqueada: "Bloqueada",
  atendida_em_outro_aparelho: "Atendida em outro aparelho",
  mms_nao_baixado: "MMS não baixado",
  item: "Item do caso",
  recuperado: "Recuperado (carving)",
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

/** Como o número foi apresentado na chamada (`presentation` do Android). */
export const nomeApresentacao: Record<string, string> = {
  permitido: "Número apresentado",
  restrito: "Número restrito (privado)",
  desconhecido: "Número desconhecido",
  telefone_publico: "Telefone público",
};

/** Códigos gravados no lugar do número pelas versões antigas do Android. */
export const nomeNumeroEspecial: Record<string, string> = {
  "-1": "Número desconhecido (código -1)",
  "-2": "Número privado (código -2)",
  "-3": "Telefone público (código -3)",
  "-4": "Número pago (código -4)",
};

/** Recursos da chamada (bits de `features`). */
export const nomeRecurso: Record<string, string> = {
  video: "Vídeo",
  transferida_de_outro_aparelho: "Transferida de outro aparelho",
  hd: "HD",
  wifi: "Wi-Fi",
  discagem_assistida: "Discagem assistida",
  rtt: "Texto em tempo real (RTT)",
  volte: "VoLTE",
};

/** Tipo do telefone na agenda (`Phone.TYPE_*`). */
export const nomeTipoTelefone: Record<string, string> = {
  personalizado: "Personalizado",
  residencial: "Residencial",
  celular: "Celular",
  trabalho: "Trabalho",
  fax_trabalho: "Fax do trabalho",
  fax_residencial: "Fax residencial",
  pager: "Pager",
  outro: "Outro",
  retorno: "Retorno",
  carro: "Carro",
  empresa_principal: "Principal da empresa",
  isdn: "ISDN",
  principal: "Principal",
  outro_fax: "Outro fax",
  radio: "Rádio",
  telex: "Telex",
  tty_tdd: "TTY/TDD",
  celular_trabalho: "Celular do trabalho",
  pager_trabalho: "Pager do trabalho",
  assistente: "Assistente",
  mms: "MMS",
};
