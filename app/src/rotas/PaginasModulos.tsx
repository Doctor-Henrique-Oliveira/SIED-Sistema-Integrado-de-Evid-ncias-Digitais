// Módulos do caso: Mídias, Arquivos e Contatos. Só mostram o que já está gravado
// no banco do caso; o que ainda não existe é dito com clareza.

import { useState } from "react";
import { Link } from "react-router-dom";
import type { Caso } from "../api/cliente";
import { useIdentificadores } from "../api/consultas";
import { InventarioArquivos, SeletorExtracao, useExtracaoEscolhida } from "../componentes/Arquivos";
import { Aviso, Cabecalho, Cartao, Carregando, entrada, LinkBotao, Paginacao, tabela, Vazio } from "../componentes/Estrutura";
import type { NomeIcone } from "../componentes/Icone";
import { dataHoraMs, identificacaoCaso, numero } from "../util/formatar";
import { rotuloApp } from "../util/rotulos";
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

// ---------------------------------------------------------------- Contatos

const LIMITE = 100;

export function PaginaContatos() {
  return <ComCasoAberto>{(caso) => <Contatos caso={caso} />}</ComCasoAberto>;
}

function Contatos({ caso }: { caso: Caso }) {
  const [texto, setTexto] = useState("");
  const [filtro, setFiltro] = useState<string | null>(null);
  const [deslocamento, setDeslocamento] = useState(0);
  const lista = useIdentificadores(caso.id, filtro, deslocamento, LIMITE);

  return (
    <>
      <Cabecalho
        icone="contatos"
        sobre={identificacaoCaso(caso)}
        titulo="Contatos"
        subtitulo="Identificadores (telefones, contas) que aparecem nas conversas, com quantas mensagens trocaram."
      />
      <div className="space-y-3">
        <Aviso tipo="info">
          Os identificadores aparecem exatamente como o parser leu. O SIED não junta números parecidos nem deduz a quem pertencem. O
          nome, quando existe, é o gravado nos contatos do próprio app na mesma extração (ex.: wa.db do WhatsApp). A agenda de
          contatos do Android será exibida aqui quando o leitor dela existir.
        </Aviso>
        <Cartao
          denso
          titulo={lista.data ? `Identificadores (${numero(lista.data.total)})` : "Identificadores"}
          icone="contatos"
          acoes={
            <form
              onSubmit={(e) => {
                e.preventDefault();
                setFiltro(texto.trim() || null);
                setDeslocamento(0);
              }}
            >
              <input
                type="search"
                value={texto}
                onChange={(e) => setTexto(e.target.value)}
                placeholder="Filtrar por identificador ou nome"
                aria-label="Filtrar identificadores"
                className={`${entrada} w-64`}
              />
            </form>
          }
        >
          {lista.isPending ? (
            <div className="px-4">
              <Carregando />
            </div>
          ) : lista.isError ? (
            <div className="p-4">
              <Aviso>{lista.error.message}</Aviso>
            </div>
          ) : lista.data.total === 0 ? (
            <div className="p-4">
              <Vazio titulo={filtro ? "Nenhum identificador ou nome com esse trecho" : "Nenhum identificador nas conversas"} icone="contatos">
                {filtro ? undefined : "Importe uma extração com mensagens para ver os identificadores aqui."}
              </Vazio>
            </div>
          ) : (
            <>
              <div className="overflow-x-auto">
                <table className={tabela.base} aria-label="Identificadores das conversas">
                  <thead className={tabela.cabeca}>
                    <tr>
                      <th className={tabela.th}>Identificador</th>
                      <th className={tabela.th}>Nome no app</th>
                      <th className={tabela.th}>App</th>
                      <th className={`${tabela.th} text-right`}>Conversas</th>
                      <th className={`${tabela.th} text-right`}>Mensagens</th>
                      <th className={tabela.th}>Primeira</th>
                      <th className={tabela.th}>Última</th>
                      <th className={tabela.th}>
                        <span className="sr-only">Ações</span>
                      </th>
                    </tr>
                  </thead>
                  <tbody className={tabela.corpo}>
                    {lista.data.itens.map((i) => (
                      <tr key={`${i.app}:${i.identificador}`} className={tabela.linha}>
                        <td className={`${tabela.td} font-mono text-xs text-texto`}>{i.identificador}</td>
                        <td className={`${tabela.td} text-texto`}>{i.nome ?? <span className="text-texto-3">—</span>}</td>
                        <td className={tabela.td}>{rotuloApp(i.app)}</td>
                        <td className={`${tabela.td} tabular text-right`}>{numero(i.conversas)}</td>
                        <td className={`${tabela.td} tabular text-right`}>{numero(i.mensagens)}</td>
                        <td className={`${tabela.td} tabular whitespace-nowrap text-xs text-texto-2`}>{dataHoraMs(i.primeira_ms, caso.fuso_horario)}</td>
                        <td className={`${tabela.td} tabular whitespace-nowrap text-xs text-texto-2`}>{dataHoraMs(i.ultima_ms, caso.fuso_horario)}</td>
                        <td className={`${tabela.td} text-right`}>
                          <Link
                            to={`/casos/${caso.id}/busca?q=${encodeURIComponent(i.identificador)}`}
                            className="whitespace-nowrap text-xs text-ouro-claro hover:underline"
                          >
                            Buscar ocorrências
                          </Link>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              {lista.data.total > LIMITE && (
                <div className="border-t border-borda px-3">
                  <Paginacao total={lista.data.total} deslocamento={deslocamento} limite={LIMITE} mudar={setDeslocamento} />
                </div>
              )}
            </>
          )}
        </Cartao>
      </div>
    </>
  );
}
