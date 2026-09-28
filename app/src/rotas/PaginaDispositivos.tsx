import type { Caso } from "../api/cliente";
import { Cabecalho } from "../componentes/Estrutura";
import { SecaoDispositivos } from "../componentes/SecoesCaso";
import { identificacaoCaso } from "../util/formatar";
import { ComCasoAberto } from "./PaginaCaso";

export function PaginaDispositivos() {
  return <ComCasoAberto>{(caso) => <Dispositivos caso={caso} />}</ComCasoAberto>;
}

function Dispositivos({ caso }: { caso: Caso }) {
  return (
    <>
      <Cabecalho
        icone="dispositivo"
        sobre={identificacaoCaso(caso)}
        titulo="Dispositivos"
        subtitulo="Aparelhos examinados neste caso. O IMEI é conferido pelo dígito verificador ao cadastrar."
      />
      <SecaoDispositivos caso={caso} />
    </>
  );
}
