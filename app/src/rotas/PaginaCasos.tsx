import { open } from "@tauri-apps/plugin-dialog";
import { Link, useNavigate } from "react-router-dom";
import { useAbrirPastaCaso, useCasos } from "../api/consultas";
import { Botao } from "../componentes/Botao";
import { Aviso, Cabecalho, Carregando, Vazio } from "../componentes/Estrutura";
import { dataHora, identificacaoCaso } from "../util/formatar";

export function PaginaCasos() {
  const casos = useCasos();
  const abrirPasta = useAbrirPastaCaso();
  const navegar = useNavigate();

  async function escolherPasta() {
    const pasta = await open({ directory: true, title: "Selecione a pasta de um caso do SIED" });
    if (typeof pasta === "string") {
      abrirPasta.mutate(pasta, { onSuccess: (c) => navegar(`/casos/${c.caso.id}`) });
    }
  }

  return (
    <>
      <Cabecalho
        titulo="Casos"
        subtitulo="Cada caso fica numa pasta própria, com banco de dados autocontido."
        acoes={
          <>
            <Botao variante="secundario" onClick={escolherPasta} carregando={abrirPasta.isPending}>
              Abrir pasta de caso…
            </Botao>
            <Botao onClick={() => navegar("/casos/novo")}>Novo caso</Botao>
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
        <Vazio titulo="Nenhum caso nesta estação">
          Crie um caso para começar, ou abra a pasta de um caso existente.
        </Vazio>
      ) : (
        <ul className="grid gap-3" aria-label="Lista de casos">
          {casos.data.map((c) => (
            <li key={c.caso_id}>
              <Link
                to={c.disponivel ? `/casos/${c.caso_id}` : "#"}
                aria-disabled={!c.disponivel}
                className={`flex items-center gap-4 rounded-xl bg-white px-5 py-4 shadow-sm ring-1 ring-slate-200 transition ${
                  c.disponivel ? "hover:ring-marca-500" : "cursor-not-allowed opacity-60"
                }`}
              >
                <div className="min-w-0 flex-1">
                  <p className="truncate font-semibold text-slate-900">{identificacaoCaso(c)}</p>
                  <p className="mt-0.5 truncate text-sm text-slate-500">
                    {[c.numero_procedimento && c.numero_bo ? `BO ${c.numero_bo}` : null, c.unidade, c.responsavel_analise]
                      .filter(Boolean)
                      .join(" · ")}
                  </p>
                </div>
                <div className="text-right text-xs text-slate-500">
                  {c.disponivel ? (
                    <>
                      <p>Último acesso</p>
                      <p className="font-medium text-slate-700">{dataHora(c.ultimo_acesso_em)}</p>
                    </>
                  ) : (
                    <p className="font-medium text-amber-700" title={c.pasta}>
                      Pasta indisponível
                    </p>
                  )}
                </div>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
