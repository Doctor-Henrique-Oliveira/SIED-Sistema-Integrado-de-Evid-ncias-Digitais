import type { Caso } from "../api/cliente";
import { Cabecalho } from "../componentes/Estrutura";
import { SecaoExtracoes } from "../componentes/SecoesCaso";
import { identificacaoCaso } from "../util/formatar";
import { ComCasoAberto } from "./PaginaCaso";

export function PaginaExtracoes() {
  return <ComCasoAberto>{(caso) => <Extracoes caso={caso} />}</ComCasoAberto>;
}

function Extracoes({ caso }: { caso: Caso }) {
  return (
    <>
      <Cabecalho
        icone="extracao"
        sobre={identificacaoCaso(caso)}
        titulo="Extrações"
        subtitulo="Pastas importadas, somente leitura. Cada importação tem manifesto SHA-256 e registro completo do processamento."
      />
      <SecaoExtracoes caso={caso} />
    </>
  );
}
