// Hooks de dados (TanStack Query). A UI nunca guarda cópias próprias do estado do
// backend: lê daqui e invalida após cada alteração.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type CasoId, type NovoCaso, type NovoDispositivo } from "./cliente";

export const chaves = {
  info: ["info"] as const,
  casos: ["casos"] as const,
  casoAtual: ["caso-atual"] as const,
  dispositivos: (casoId: CasoId) => ["dispositivos", casoId] as const,
  auditoria: (casoId: CasoId) => ["auditoria", casoId] as const,
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
    },
  });
}
