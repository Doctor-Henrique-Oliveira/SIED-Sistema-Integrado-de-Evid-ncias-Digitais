import type { Caso } from "../api/cliente";
import { Cabecalho } from "../componentes/Estrutura";
import { SecaoAtividades } from "../componentes/SecoesCaso";
import { identificacaoCaso } from "../util/formatar";
import { ComCasoAberto } from "./PaginaCaso";

export function PaginaAuditoria() {
  return <ComCasoAberto>{(caso) => <Auditoria caso={caso} />}</ComCasoAberto>;
}

function Auditoria({ caso }: { caso: Caso }) {
  return (
    <div className="pagina-contida space-y-5">
      <Cabecalho
        icone="integridade"
        sobre={identificacaoCaso(caso)}
        titulo="Auditoria"
        subtitulo="Trilha de atividades, verificação de integridade e conferência dos dados derivados."
      />
      <SecaoAtividades caso={caso} />
    </div>
  );
}
