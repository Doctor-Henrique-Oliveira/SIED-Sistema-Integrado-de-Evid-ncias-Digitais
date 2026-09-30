// Módulos com a estrutura pronta e a funcionalidade ainda por vir: Localizações e
// Consultas. Nenhum dado fictício: só o que o banco do caso já tem.
// Configurações mostra, somente leitura, como esta estação está configurada.

import type { ReactNode } from "react";
import type { Caso } from "../api/cliente";
import { useInfoApp, useResumoCaso } from "../api/consultas";
import { Cabecalho, Cartao, Carregando, Definicoes, Indicador, Indisponivel, Selo } from "../componentes/Estrutura";
import { identificacaoCaso, numero } from "../util/formatar";
import { ComCasoAberto } from "./PaginaCaso";

export function PaginaLocalizacoes() {
  return <ComCasoAberto>{(caso) => <Localizacoes caso={caso} />}</ComCasoAberto>;
}

function Localizacoes({ caso }: { caso: Caso }) {
  const resumo = useResumoCaso(caso.id);
  return (
    <>
      <Cabecalho
        icone="localizacoes"
        sobre={identificacaoCaso(caso)}
        titulo="Localizações"
        subtitulo="Pontos com coordenadas extraídos dos aparelhos, em ordem cronológica."
      />
      <div className="space-y-4">
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <Indicador
            rotulo="Registros com coordenadas"
            icone="localizacoes"
            valor={resumo.data ? numero(resumo.data.localizacoes) : null}
            detalhe="gravados no banco do caso"
          />
        </div>
        <Indisponivel
          titulo="Linha do tempo de localizações"
          icone="mapa"
          previsto={[
            "Lista cronológica com data e hora no fuso do caso",
            "Latitude, longitude e precisão como lidas",
            "App, arquivo, tabela e registro de origem",
            "Mapa offline, sem serviço de mapas da internet",
          ]}
        >
          O leitor do WhatsApp já extrai coordenadas válidas e as exibe nas conversas. Este módulo dedicado ainda não possui lista ou
          mapa; quando forem implementados, cada ponto manterá a mesma origem técnica das mensagens e o mapa usará apenas dados locais.
        </Indisponivel>
      </div>
    </>
  );
}

export function PaginaConsultas() {
  return (
    <ComCasoAberto>
      {(caso) => (
        <>
          <Cabecalho icone="consultas" sobre={identificacaoCaso(caso)} titulo="Consultas" />
          <Indisponivel
            titulo="Consultas do caso"
            icone="consultas"
            previsto={["Consultas salvas sobre os dados do caso", "Histórico com data, analista e resultado", "Execução local, sem internet"]}
          >
            Espaço reservado para consultas sobre os dados do caso. O SIED não acessa serviços externos: tudo o que for consultado aqui
            sairá do banco local do caso.
          </Indisponivel>
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
            <ul className="space-y-2 text-[13px] text-texto-2">
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
