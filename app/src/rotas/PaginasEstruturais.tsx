// Localizações (lista cronológica dos registros com coordenadas; o mapa offline ainda
// não existe) e Consultas (linha do tempo do caso; consultas salvas ainda por vir).
// Nenhum dado fictício: só o que o banco do caso já tem.
// Configurações mostra, somente leitura, como esta estação está configurada.

import { useState, type ReactNode } from "react";
import type { Caso } from "../api/cliente";
import { useInfoApp } from "../api/consultas";
import { Abas, Cabecalho, Cartao, Carregando, Definicoes, Indisponivel, Selo } from "../componentes/Estrutura";
import { LinhaDoTempo } from "../componentes/LinhaTempo";
import { identificacaoCaso } from "../util/formatar";
import { ComCasoAberto } from "./PaginaCaso";

export function PaginaLocalizacoes() {
  return <ComCasoAberto>{(caso) => <Localizacoes caso={caso} />}</ComCasoAberto>;
}

const SO_COORDENADAS = { com_coordenadas: true };

function Localizacoes({ caso }: { caso: Caso }) {
  return (
    <>
      <Cabecalho
        icone="localizacoes"
        sobre={identificacaoCaso(caso)}
        titulo="Localizações"
        subtitulo="Registros com coordenadas extraídos dos aparelhos, em ordem cronológica, com latitude e longitude como lidas."
      />
      <div className="space-y-4">
        <LinhaDoTempo casoId={caso.id} fuso={caso.fuso_horario} fixo={SO_COORDENADAS} coordenadas rotulo="Localizações" />
        <Indisponivel titulo="Mapa offline" icone="mapa" previsto={["Pontos no mapa sem serviço de mapas da internet", "Mesma origem técnica de cada registro"]}>
          O SIED não usa internet e ainda não tem mapas locais. Por isso as coordenadas aparecem só em lista; cada ponto abre o registro
          de origem (conversa, arquivo, tabela e registro).
        </Indisponivel>
      </div>
    </>
  );
}

type AbaConsultas = "linha" | "salvas";

export function PaginaConsultas() {
  const [aba, setAba] = useState<AbaConsultas>("linha");
  return (
    <ComCasoAberto>
      {(caso) => (
        <>
          <Cabecalho
            icone="consultas"
            sobre={identificacaoCaso(caso)}
            titulo="Consultas"
            subtitulo="Consultas locais sobre os dados do caso. Nada sai desta estação."
          />
          <div className="space-y-3">
            <Abas<AbaConsultas>
              rotulo="Consultas do caso"
              atual={aba}
              mudar={setAba}
              abas={[
                ["linha", "Linha do tempo"],
                ["salvas", "Consultas salvas"],
              ]}
            />
            {aba === "linha" ? (
              <LinhaDoTempo casoId={caso.id} fuso={caso.fuso_horario} />
            ) : (
              <Indisponivel
                titulo="Consultas salvas"
                icone="consultas"
                previsto={["Consultas salvas sobre os dados do caso", "Histórico com data, analista e resultado", "Execução local, sem internet"]}
              >
                Espaço reservado para guardar consultas e reexecutá-las. O SIED não acessa serviços externos: tudo o que for consultado
                aqui sairá do banco local do caso.
              </Indisponivel>
            )}
          </div>
        </>
      )}
    </ComCasoAberto>
  );
}

// ---------------------------------------------------------------- Configurações

function Valor({ children }: { children: ReactNode }) {
  return <span className="break-all font-mono text-xs">{children}</span>;
}

export function PaginaConfiguracoes() {
  const info = useInfoApp();
  return (
    <>
      <Cabecalho
        icone="config"
        titulo="Configurações"
        subtitulo="Como esta estação está configurada. Somente leitura nesta versão."
      />
      {info.isPending ? (
        <Carregando />
      ) : info.data ? (
        <div className="grid max-w-5xl gap-4">
          <Cartao titulo="Estação" icone="estacao">
            <Definicoes
              itens={[
                ["Versão do SIED", info.data.versao],
                ["Usuário", info.data.usuario],
                ["Estação", info.data.maquina],
              ]}
            />
          </Cartao>
          <Cartao titulo="Pastas" icone="disco">
            <Definicoes
              colunas={2}
              itens={[
                ["Casos (padrão para novos casos)", <Valor>{info.data.pasta_casos_padrao}</Valor>],
                ["Dados do aplicativo (catálogo)", <Valor>{info.data.pasta_dados}</Valor>],
                ["Logs", <Valor>{info.data.pasta_logs}</Valor>],
              ]}
            />
          </Cartao>
          <Cartao titulo="Segurança e rede" icone="cadeado">
            <ul className="space-y-2 text-sm text-texto-2">
              <li className="flex items-center gap-2">
                <Selo tom="ok">Offline</Selo> O SIED não usa internet; nada sai desta estação.
              </li>
              <li className="flex items-center gap-2">
                <Selo tom="ok">Somente leitura</Selo> Pastas de extração nunca recebem gravação; parsers leem cópias verificadas.
              </li>
              <li className="flex items-center gap-2">
                <Selo tom="neutro">Tema</Selo> Escuro (único tema nesta versão).
              </li>
            </ul>
          </Cartao>
        </div>
      ) : null}
    </>
  );
}
