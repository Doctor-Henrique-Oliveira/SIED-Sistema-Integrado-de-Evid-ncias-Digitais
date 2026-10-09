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
