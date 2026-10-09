/**
 * Manifesto de provedores da estação (E11).
 *
 * O DTO é deliberadamente separado dos contratos de caso: a exportação ts-rs
 * será regenerada pela CI quando o toolchain Rust/Windows estiver disponível.
 * Este adaptador contém apenas configuração não sensível; nenhum segredo ou
 * conteúdo de evidência pode ser colocado aqui.
 */
export type ProvedorEstacao = {
  id: string;
  nome: string;
  tipo: string;
  modo: "local" | "externo";
  habilitado: boolean;
  caminho: string | null;
  versao: string | null;
  observacao: string | null;
  criado_em: string;
  atualizado_em: string;
};

export type NovoProvedorEstacao = Omit<ProvedorEstacao, "criado_em" | "atualizado_em">;

export const ACOES_ESTACAO = [
  ["executar_provedor", "Executar provider local"],
  ["enriquecer_imei", "Consultar IMEI local"],
  ["criar_backup", "Criar backup do catálogo"],
  ["restaurar_backup", "Restaurar configurações"],
] as const;

export type PermissaoEstacao = {
  usuario: string;
  acao: (typeof ACOES_ESTACAO)[number][0];
  permitido: boolean;
  criado_em: string;
  atualizado_em: string;
};

export type NovaPermissaoEstacao = Pick<PermissaoEstacao, "usuario" | "acao" | "permitido">;

export type ResultadoExecucaoProvedor = {
  provedor_id: string;
  acao: string;
  sucesso: boolean;
  codigo_saida: number | null;
  saida: string;
  erro: string | null;
  duracao_ms: number;
};

export type ResultadoImeiLocal = {
  imei: string;
  provedor_id: string;
  provedor_versao: string | null;
  encontrado: boolean;
  fabricante: string | null;
  modelo: string | null;
  fonte: string;
  observacao: string;
};

export type ResultadoBackupEstacao = {
  caminho: string;
  sha256: string;
  componentes: string[];
};

export type ResultadoRestauracaoEstacao = {
  origem: string;
  provedores_restaurados: number;
  permissoes_restauradas: number;
};
