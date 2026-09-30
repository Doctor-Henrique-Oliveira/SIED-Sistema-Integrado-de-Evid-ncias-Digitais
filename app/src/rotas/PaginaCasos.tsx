import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAbrirPastaCaso, useCasos } from "../api/consultas";
import { Botao } from "../componentes/Botao";
import { Aviso, Cabecalho, Carregando, entrada, Selo, Vazio } from "../componentes/Estrutura";
import { Icone } from "../componentes/Icone";
import { dataHora, identificacaoCaso } from "../util/formatar";

export function PaginaCasos() {
  const casos = useCasos();
  const abrirPasta = useAbrirPastaCaso();
  const navegar = useNavigate();
  const [filtro, setFiltro] = useState("");

  async function escolherPasta() {
    const pasta = await open({ directory: true, title: "Selecione a pasta de um caso do SIED" });
    if (typeof pasta === "string") {
      abrirPasta.mutate(pasta, { onSuccess: (c) => navegar(`/casos/${c.caso.id}`) });
    }
  }

  const termo = filtro.trim().toLowerCase();
  const lista = (casos.data ?? []).filter(
    (c) =>
      !termo ||
      [c.numero_procedimento, c.numero_bo, c.unidade, c.responsavel_analise].some((v) => v?.toLowerCase().includes(termo)),
  );

  return (
    <>
      <Cabecalho
        icone="casos"
        titulo="Casos"
        subtitulo="Cada caso fica numa pasta própria, com banco de dados autocontido e trilha de auditoria."
        acoes={
          <>
            <Botao variante="secundario" icone="casos" onClick={escolherPasta} carregando={abrirPasta.isPending}>
              Abrir pasta de caso…
            </Botao>
            <Botao icone="mais" onClick={() => navegar("/casos/novo")}>
              Novo caso
            </Botao>
          </>
        }
      />

      {abrirPasta.isError && (
        <div className="mb-4">
          <Aviso>{abrirPasta.error.message}</Aviso>
        </div>
      )}

      {casos.isPending ? (
        <Carregando />
      ) : casos.isError ? (
        <Aviso>{casos.error.message}</Aviso>
      ) : casos.data.length === 0 ? (
        <Vazio titulo="Nenhum caso nesta estação" icone="casos">
          Crie um caso para começar, ou abra a pasta de um caso existente.
        </Vazio>
      ) : (
        <div className="painel">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-borda px-4 py-2.5">
            <p className="text-sm text-texto-2">
              <span className="tabular font-semibold text-texto">{casos.data.length}</span>{" "}
              {casos.data.length === 1 ? "caso catalogado" : "casos catalogados"}
            </p>
            <input
              type="search"
              value={filtro}
              onChange={(e) => setFiltro(e.target.value)}
              placeholder="Filtrar por número, BO, unidade ou responsável"
              aria-label="Filtrar casos"
              className={`${entrada} w-80`}
            />
          </div>
          <ul className="divide-y divide-borda" aria-label="Lista de casos">
            {lista.map((c) => (
              <li key={c.caso_id}>
                <Link
                  to={c.disponivel ? `/casos/${c.caso_id}` : "#"}
                  aria-disabled={!c.disponivel}
                  className={`flex items-center gap-4 px-4 py-3 transition-colors ${
                    c.disponivel ? "hover:bg-sup-2" : "cursor-not-allowed opacity-60"
                  }`}
                >
                  <Icone nome="casos" className="size-4 shrink-0 text-ouro" />
                  <div className="min-w-0 flex-1">
                    <p className="truncate font-semibold text-texto">{identificacaoCaso(c)}</p>
                    <div className="mt-0.5 truncate text-xs text-texto-2">
                      {[c.numero_procedimento && c.numero_bo ? `BO ${c.numero_bo}` : null, c.unidade, c.responsavel_analise]
                        .filter(Boolean)
                        .join(" · ")}
                    </div>
                  </div>
                  <div className="hidden text-right text-xs text-texto-3 md:block">
                    <div className="sobrelinha">Criado em</div>
                    <div className="tabular text-texto-2">{dataHora(c.criado_em)}</div>
                  </div>
                  <div className="w-44 text-right text-xs text-texto-3">
                    {c.disponivel ? (
                      <>
                        <div className="sobrelinha">Último acesso</div>
                        <div className="tabular text-texto-2">{dataHora(c.ultimo_acesso_em)}</div>
                      </>
                    ) : (
                      <span title={c.pasta}>
                        <Selo tom="atencao">Pasta indisponível</Selo>
                      </span>
                    )}
                  </div>
                  <Icone nome="direita" className="size-4 text-texto-4" />
                </Link>
              </li>
            ))}
            {lista.length === 0 && <li className="px-4 py-6 text-center text-sm text-texto-2">Nenhum caso com esse filtro.</li>}
          </ul>
        </div>
      )}
    </>
  );
}
