// Módulos do caso: Mídias e Arquivos (Contatos fica em PaginaContatos.tsx). Só mostram o que já está gravado
// no banco do caso; o que ainda não existe é dito com clareza.

import type { Caso } from "../api/cliente";
import { InventarioArquivos, SeletorExtracao, useExtracaoEscolhida } from "../componentes/Arquivos";
import { Aviso, Cabecalho, Carregando, LinkBotao, Vazio } from "../componentes/Estrutura";
import type { NomeIcone } from "../componentes/Icone";
import { identificacaoCaso } from "../util/formatar";
import { ComCasoAberto } from "./PaginaCaso";

function SemExtracao({ caso, icone }: { caso: Caso; icone: NomeIcone }) {
  return (
    <Vazio
      titulo="Nenhuma extração importada"
      icone={icone}
      acao={
        <LinkBotao para={`/casos/${caso.id}/extracoes`} icone="extracao">
          Ir para Extrações
        </LinkBotao>
      }
    >
      Importe uma extração para ver o inventário aqui.
    </Vazio>
  );
}

function PorExtracao({ caso, midias }: { caso: Caso; midias?: boolean }) {
  const { extracoes, id, setEscolhida } = useExtracaoEscolhida(caso);
  const icone: NomeIcone = midias ? "midias" : "arquivos";
  return (
    <>
      <Cabecalho
        icone={icone}
        sobre={identificacaoCaso(caso)}
        titulo={midias ? "Mídias" : "Arquivos"}
        subtitulo={
          midias
            ? "Imagens, vídeos, áudios e documentos inventariados, classificados pela assinatura do arquivo."
            : "Inventário completo de cada extração, com SHA-256, datas do sistema de arquivos e o parser que leu cada banco."
        }
        acoes={<SeletorExtracao caso={caso} valor={id} mudar={setEscolhida} />}
      />
      {extracoes.isPending ? (
        <Carregando />
      ) : extracoes.isError ? (
        <Aviso>{extracoes.error.message}</Aviso>
      ) : id == null ? (
        <SemExtracao caso={caso} icone={icone} />
      ) : (
        <div className="space-y-3">
          {midias && (
            <Aviso tipo="info">
              As miniaturas das imagens são geradas do arquivo da extração só depois de conferir o SHA-256 com o inventário, e ficam
              guardadas na pasta do caso. Vídeos e áudios aparecem sem miniatura e sem reprodução nesta versão. Selecione um item para
              ver caminho, datas e SHA-256.
            </Aviso>
          )}
          <InventarioArquivos key={id} caso={caso} extracaoId={id} midias={midias} />
        </div>
      )}
    </>
  );
}

export function PaginaMidias() {
  return <ComCasoAberto>{(caso) => <PorExtracao caso={caso} midias />}</ComCasoAberto>;
}

export function PaginaArquivos() {
  return <ComCasoAberto>{(caso) => <PorExtracao caso={caso} />}</ComCasoAberto>;
}

export { PaginaContatos } from "./PaginaContatos";
