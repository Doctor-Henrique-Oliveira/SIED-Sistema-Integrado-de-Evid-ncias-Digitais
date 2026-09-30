// Miniatura de uma imagem do inventário. O backend lê o arquivo da extração (somente
// leitura), confere o SHA-256 com o inventário e só então reduz a imagem; quando não
// há miniatura, a tela mostra o ícone do tipo e o motivo, nunca uma imagem inventada.

import type { Caso } from "../api/cliente";
import { useMiniatura } from "../api/consultas";
import { Icone, type NomeIcone } from "./Icone";

const nomeEstado: Record<string, string> = {
  sem_suporte: "Sem miniatura",
  origem_indisponivel: "Origem indisponível",
  origem_divergente: "Arquivo difere do inventário",
  falhou: "Imagem ilegível",
};

/** Caixa com a miniatura, ou o ícone do tipo enquanto carrega ou quando não há. */
export function MiniaturaArquivo({ caso, arquivoId, icone, categoria, ajuste = "cobrir", className = "" }: {
  caso: Caso;
  arquivoId: number | null;
  icone: NomeIcone;
  categoria: string;
  /** "cobrir" preenche a caixa (grade); "conter" mostra a imagem inteira (detalhe). */
  ajuste?: "cobrir" | "conter";
  className?: string;
}) {
  const pedir = categoria === "imagem" ? arquivoId : null;
  const m = useMiniatura(caso.id, pedir);
  const pronta = m.data?.estado === "pronta" && m.data.dados;
  const motivo = m.data && m.data.estado !== "pronta" ? m.data.motivo : null;
  return (
    <span
      className={`relative grid place-items-center overflow-hidden bg-fundo text-texto-3 ${className}`}
      data-miniatura={pronta ? "pronta" : (m.data?.estado ?? (m.isPending && pedir != null ? "carregando" : "nenhuma"))}
    >
      {pronta ? (
        <img
          src={m.data!.dados!}
          alt=""
          draggable={false}
          className={`absolute inset-0 size-full ${ajuste === "cobrir" ? "object-cover" : "object-contain"}`}
        />
      ) : (
        <>
          <Icone nome={icone} className={`size-8 ${m.isPending && pedir != null ? "animate-pulse" : ""}`} />
          {motivo && m.data!.estado !== "sem_suporte" && (
            <span
              className="absolute inset-x-0 bottom-0 truncate bg-fundo/85 px-1.5 py-0.5 text-center text-3xs text-atencao"
              title={motivo}
            >
              {nomeEstado[m.data!.estado] ?? m.data!.estado}
            </span>
          )}
        </>
      )}
    </span>
  );
}

/** Frase para o painel de detalhes: de onde a miniatura veio ou por que não há. */
export function SobreMiniatura({ caso, arquivoId, categoria }: { caso: Caso; arquivoId: number; categoria: string }) {
  const m = useMiniatura(caso.id, categoria === "imagem" ? arquivoId : null);
  if (categoria !== "imagem") return <>Sem miniatura: só imagens são reduzidas nesta versão (vídeo e áudio não são decodificados).</>;
  if (m.isPending) return <>Gerando a miniatura…</>;
  if (!m.data) return <>Miniatura indisponível.</>;
  if (m.data.estado === "pronta")
    return <>Miniatura gerada do arquivo da extração depois de conferir o SHA-256 com o inventário (cópia reduzida na pasta do caso).</>;
  return <>{m.data.motivo}</>;
}
