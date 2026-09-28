// Hooks de dados (TanStack Query). A UI nunca guarda cópias próprias do estado do
// backend: lê daqui e invalida após cada alteração.

import { keepPreviousData, useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import {
  api,
  type CasoId,
  type DispositivoId,
  type ExtracaoId,
  type FiltroBusca,
  type FiltroConversas,
  type NovoCaso,
  type NovoDispositivo,
} from "./cliente";

export const chaves = {
  info: ["info"] as const,
  casos: ["casos"] as const,
  casoAtual: ["caso-atual"] as const,
  dispositivos: (casoId: CasoId) => ["dispositivos", casoId] as const,
  auditoria: (casoId: CasoId) => ["auditoria", casoId] as const,
  importacao: ["importacao"] as const,
  extracoes: (casoId: CasoId) => ["extracoes", casoId] as const,
  extracao: (id: ExtracaoId) => ["extracao", id] as const,
  // Conversas e busca derivam das importações: invalidadas junto com elas.
  conversas: (casoId: CasoId) => ["conversas", casoId] as const,
  busca: (casoId: CasoId) => ["busca", casoId] as const,
  // Números do painel: mudam com dispositivos e importações.
  resumo: (casoId: CasoId) => ["resumo", casoId] as const,
};

export function useInfoApp() {
  return useQuery({ queryKey: chaves.info, queryFn: api.infoApp, staleTime: Infinity });
}

export function useCasos() {
  return useQuery({ queryKey: chaves.casos, queryFn: api.listarCasos });
}

export function useCasoAtual() {
  return useQuery({ queryKey: chaves.casoAtual, queryFn: api.casoAtual });
}

export function useDispositivos(casoId: CasoId | undefined) {
  return useQuery({
    queryKey: chaves.dispositivos(casoId ?? ""),
    queryFn: api.listarDispositivos,
    enabled: !!casoId,
  });
}

export function useAuditoria(casoId: CasoId | undefined) {
  return useQuery({
    queryKey: chaves.auditoria(casoId ?? ""),
    queryFn: () => api.listarAuditoria(20),
    enabled: !!casoId,
  });
}

function useAposAbrirCaso() {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: chaves.casos });
    void qc.invalidateQueries({ queryKey: chaves.casoAtual });
    void qc.invalidateQueries({ queryKey: ["auditoria"] });
  };
}

export function useCriarCaso() {
  const apos = useAposAbrirCaso();
  return useMutation({
    mutationFn: ({ dados, pastaBase }: { dados: NovoCaso; pastaBase: string | null }) =>
      api.criarCaso(dados, pastaBase),
    onSuccess: apos,
  });
}

export function useAbrirCaso() {
  const apos = useAposAbrirCaso();
  return useMutation({ mutationFn: api.abrirCaso, onSuccess: apos });
}

export function useAbrirPastaCaso() {
  const apos = useAposAbrirCaso();
  return useMutation({ mutationFn: api.abrirPastaCaso, onSuccess: apos });
}

export function useFecharCaso() {
  const apos = useAposAbrirCaso();
  return useMutation({ mutationFn: api.fecharCaso, onSuccess: apos });
}

export function useCadastrarDispositivo(casoId: CasoId) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (dados: NovoDispositivo) => api.cadastrarDispositivo(dados),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: chaves.dispositivos(casoId) });
      void qc.invalidateQueries({ queryKey: chaves.auditoria(casoId) });
      void qc.invalidateQueries({ queryKey: chaves.resumo(casoId) });
    },
  });
}

// ---------- Importação ----------

/** Acompanha a importação em curso: consulta a cada 500 ms até terminar. Quando
 * termina, atualiza extrações e atividades do caso. */
export function useEstadoImportacao(casoId: CasoId) {
  const qc = useQueryClient();
  const consulta = useQuery({
    queryKey: chaves.importacao,
    queryFn: api.estadoImportacao,
    refetchInterval: (q) => (q.state.data && !q.state.data.terminada ? 500 : false),
  });
  const terminada = consulta.data?.terminada;
  const anterior = useRef(terminada);
  useEffect(() => {
    if (anterior.current === false && terminada) {
      void qc.invalidateQueries({ queryKey: chaves.extracoes(casoId) });
      void qc.invalidateQueries({ queryKey: chaves.auditoria(casoId) });
      void qc.invalidateQueries({ queryKey: ["extracao"] });
      void qc.invalidateQueries({ queryKey: chaves.conversas(casoId) });
      void qc.invalidateQueries({ queryKey: chaves.busca(casoId) });
      void qc.invalidateQueries({ queryKey: chaves.resumo(casoId) });
    }
    anterior.current = terminada;
  }, [terminada, casoId, qc]);
  return consulta;
}

export function useIniciarImportacao(casoId: CasoId) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (p: { dispositivoId: DispositivoId; pasta: string; importador: string }) =>
      api.iniciarImportacao(p.dispositivoId, p.pasta, p.importador),
    onSuccess: (estado) => {
      qc.setQueryData(chaves.importacao, estado);
      void qc.invalidateQueries({ queryKey: chaves.extracoes(casoId) });
      void qc.invalidateQueries({ queryKey: chaves.auditoria(casoId) });
    },
  });
}

export function useCancelarImportacao() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: api.cancelarImportacao,
    onSuccess: () => void qc.invalidateQueries({ queryKey: chaves.importacao }),
  });
}

export function useExtracoes(casoId: CasoId) {
  return useQuery({ queryKey: chaves.extracoes(casoId), queryFn: api.listarExtracoes });
}

export function useExtracao(id: ExtracaoId) {
  return useQuery({ queryKey: chaves.extracao(id), queryFn: () => api.obterExtracao(id) });
}

export function useTotaisPorCategoria(id: ExtracaoId) {
  return useQuery({ queryKey: [...chaves.extracao(id), "categorias"], queryFn: () => api.totaisPorCategoria(id) });
}

export function useArquivos(
  id: ExtracaoId,
  filtro: { categoria: string | null; texto: string | null },
  deslocamento: number,
  limite: number,
) {
  return useQuery({
    queryKey: [...chaves.extracao(id), "arquivos", filtro, deslocamento, limite],
    queryFn: () => api.listarArquivos(id, filtro, deslocamento, limite),
    placeholderData: keepPreviousData,
  });
}

export function useExecucoes(id: ExtracaoId) {
  return useQuery({ queryKey: [...chaves.extracao(id), "execucoes"], queryFn: () => api.listarExecucoes(id) });
}

export function useArtefatos(id: ExtracaoId, deslocamento: number, limite: number) {
  return useQuery({
    queryKey: [...chaves.extracao(id), "artefatos", deslocamento, limite],
    queryFn: () => api.listarArtefatos(id, deslocamento, limite),
    placeholderData: keepPreviousData,
  });
}

export function useDetalheArtefato(artefatoId: number | null) {
  return useQuery({
    queryKey: ["artefato", artefatoId],
    queryFn: () => api.detalheArtefato(artefatoId!),
    enabled: artefatoId != null,
  });
}

export function useConversas(casoId: CasoId, filtro: FiltroConversas, deslocamento: number, limite: number) {
  return useQuery({
    queryKey: [...chaves.conversas(casoId), "lista", filtro, deslocamento, limite],
    queryFn: () => api.listarConversas(filtro, deslocamento, limite),
    placeholderData: keepPreviousData,
  });
}

/** Páginas extras de uma lista longa (a primeira vem do hook da lista), pedidas
 * conforme a rolagem: só o que está perto da área visível é carregado. */
export function usePaginasExtras<T>(chave: readonly unknown[], paginas: number[], buscar: (pagina: number) => Promise<T>) {
  return useQueries({
    queries: paginas.map((p) => ({
      queryKey: [...chave, "pagina", p],
      queryFn: () => buscar(p),
      staleTime: 60_000,
    })),
  });
}

export function useAppsDasConversas(casoId: CasoId, extracaoId: ExtracaoId | null) {
  return useQuery({
    queryKey: [...chaves.conversas(casoId), "apps", extracaoId],
    queryFn: () => api.appsDasConversas(extracaoId),
  });
}

export function usePosicaoMensagem(casoId: CasoId, conversaId: number, artefatoId: number | null, ativo: boolean) {
  return useQuery({
    queryKey: [...chaves.conversas(casoId), "posicao", conversaId, artefatoId],
    queryFn: () => api.posicaoMensagem(conversaId, artefatoId!),
    enabled: ativo && artefatoId != null,
  });
}

export function useLocalizarNaConversa(casoId: CasoId, conversaId: number, termo: string, ativo: boolean) {
  return useQuery({
    queryKey: [...chaves.conversas(casoId), "localizar", conversaId, termo],
    queryFn: () => api.localizarNaConversa(conversaId, termo, 5000),
    enabled: ativo && termo.trim() !== "",
    placeholderData: keepPreviousData,
  });
}

/** Miniatura: guardada enquanto o caso estiver aberto (o arquivo de origem é imutável). */
export function useMiniatura(casoId: CasoId, arquivoId: number | null) {
  return useQuery({
    queryKey: ["miniatura", casoId, arquivoId],
    queryFn: () => api.miniatura(arquivoId!),
    enabled: arquivoId != null,
    staleTime: Infinity,
    gcTime: 10 * 60_000,
    retry: false,
  });
}

export function useConversa(casoId: CasoId, conversaId: number | null) {
  return useQuery({
    queryKey: [...chaves.conversas(casoId), "detalhe", conversaId],
    queryFn: () => api.obterConversa(conversaId!),
    enabled: conversaId != null,
  });
}

export function useMensagensConversa(casoId: CasoId, conversaId: number | null, deslocamento: number, limite: number) {
  return useQuery({
    queryKey: [...chaves.conversas(casoId), "mensagens", conversaId, deslocamento, limite],
    queryFn: () => api.mensagensConversa(conversaId!, deslocamento, limite),
    enabled: conversaId != null,
    placeholderData: keepPreviousData,
  });
}

export function useBusca(casoId: CasoId, filtro: FiltroBusca | null, deslocamento: number, limite: number) {
  return useQuery({
    queryKey: [...chaves.busca(casoId), filtro, deslocamento, limite],
    queryFn: () => api.buscar(filtro!, deslocamento, limite),
    enabled: filtro != null && filtro.texto.trim() !== "",
    placeholderData: keepPreviousData,
  });
}

export function useResumoCaso(casoId: CasoId) {
  return useQuery({ queryKey: chaves.resumo(casoId), queryFn: api.resumoCaso });
}

/** Identificadores das conversas (contatos como o parser leu, sem deduzir quem são). */
export function useIdentificadores(casoId: CasoId, texto: string | null, deslocamento: number, limite: number) {
  return useQuery({
    queryKey: [...chaves.conversas(casoId), "identificadores", texto, deslocamento, limite],
    queryFn: () => api.listarIdentificadores(texto, deslocamento, limite),
    placeholderData: keepPreviousData,
  });
}
