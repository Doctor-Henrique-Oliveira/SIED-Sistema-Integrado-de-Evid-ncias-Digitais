import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import type { CasoId } from "../api/cliente";
import { useAbrirPastaCaso, useCasos, useInfoApp, useRemoverCasoCatalogo } from "../api/consultas";
import { Botao } from "../componentes/Botao";
import { Aviso, Carregando, Selo } from "../componentes/Estrutura";
import { Icone } from "../componentes/Icone";
import { dataHora, identificacaoCaso } from "../util/formatar";

export function PaginaCasos() {
  const casos = useCasos();
  const info = useInfoApp();
  const abrirPasta = useAbrirPastaCaso();
  const removerCaso = useRemoverCasoCatalogo();
  const navegar = useNavigate();
  const [filtro, setFiltro] = useState("");
  const [confirmacao, setConfirmacao] = useState<CasoId | null>(null);

  async function escolherPasta() {
    const pasta = await open({ directory: true, title: "Selecione a pasta que contém caso.sied.db" });
    if (typeof pasta === "string") abrirPasta.mutate(pasta, { onSuccess: (c) => navegar(`/casos/${c.caso.id}`) });
  }

  const termo = filtro.trim().toLowerCase();
  const lista = (casos.data ?? []).filter((c) => !termo ||
    [c.numero_procedimento, c.numero_bo, c.unidade, c.responsavel_analise].some((v) => v?.toLowerCase().includes(termo)));

  return (
    <div className="welcome-sied">
      <header className="welcome-sied__topo">
        <div className="flex items-center gap-3">
          <span className="grid size-9 place-items-center rounded-md border border-ouro/45 text-ouro"><Icone nome="escudo" className="size-5" /></span>
          <div><p className="text-base font-semibold tracking-[0.18em] text-texto">SIED</p><p className="text-2xs text-texto-3">Estação de análise forense digital</p></div>
        </div>
        <div className="flex items-center gap-2 text-2xs text-texto-3">
          <span className="size-1.5 rounded-full bg-ok" aria-hidden /> LOCAL · OFFLINE
          {info.data && <span className="hidden border-l border-borda pl-2 sm:inline">{info.data.usuario} @ {info.data.maquina}</span>}
        </div>
      </header>

      <section className="welcome-sied__hero" aria-labelledby="welcome-titulo">
        <p className="sobrelinha text-ouro!">Estação pronta</p>
        <h1 id="welcome-titulo">Inicie uma sessão de análise</h1>
        <p>Crie uma área de trabalho forense ou abra um caso SIED já existente nesta estação.</p>
      </section>

      {abrirPasta.isError && <Aviso>{abrirPasta.error.message}</Aviso>}
      {removerCaso.isError && <Aviso>{removerCaso.error.message}</Aviso>}

      <section className="welcome-sied__acoes" aria-label="Ações iniciais">
        <button type="button" className="welcome-sied__acao" onClick={() => navegar("/casos/novo")}>
          <span className="welcome-sied__icone"><Icone nome="mais" className="size-6" /></span>
          <span><strong>Novo caso</strong><small>Crie uma nova área de trabalho forense.</small></span>
          <Icone nome="direita" className="ml-auto size-5 text-ouro" />
        </button>
        <button type="button" className="welcome-sied__acao" onClick={escolherPasta} disabled={abrirPasta.isPending}>
          <span className="welcome-sied__icone"><Icone nome="arquivos" className="size-6" /></span>
          <span><strong>{abrirPasta.isPending ? "Validando caso…" : "Abrir caso existente"}</strong><small>Abra uma pasta criada anteriormente pelo SIED.</small></span>
          <Icone nome="direita" className="ml-auto size-5 text-ouro" />
        </button>
      </section>

      <section className="welcome-sied__recentes" aria-labelledby="recentes-titulo">
        <div className="flex flex-wrap items-end justify-between gap-3 border-b border-borda pb-2">
          <div><p className="sobrelinha">Catálogo local</p><h2 id="recentes-titulo" className="mt-0.5 text-base font-semibold text-texto">Casos nesta estação</h2></div>
          {(casos.data?.length ?? 0) > 4 && <input type="search" value={filtro} onChange={(e) => setFiltro(e.target.value)} placeholder="Filtrar casos" aria-label="Filtrar casos" className="entrada w-64" />}
        </div>
        {casos.isPending ? <Carregando /> : casos.isError ? <Aviso>{casos.error.message}</Aviso> : casos.data.length === 0 ? (
          <p className="py-5 text-sm text-texto-3">Nenhum caso catalogado. As duas ações acima são o ponto de partida.</p>
        ) : (
          <ul className="welcome-sied__lista" aria-label="Lista de casos">
            {lista.slice(0, 6).map((c) => (
              <li key={c.caso_id} className="catalogo-caso">
                <div className="catalogo-caso__linha flex items-center">
                  <Link to={c.disponivel ? `/casos/${c.caso_id}` : "#"} aria-disabled={!c.disponivel} className={`min-w-0 flex-1 ${c.disponivel ? "" : "pointer-events-none opacity-55"}`}>
                    <Icone nome="casos" className="size-4 shrink-0 text-texto-3" />
                    <span className="min-w-0 flex-1"><strong>{identificacaoCaso(c)}</strong><small>{[c.unidade, c.responsavel_analise].filter(Boolean).join(" · ") || "Sem metadados adicionais"}</small></span>
                    {c.disponivel ? <time>{dataHora(c.ultimo_acesso_em)}</time> : <Selo tom="atencao">Indisponível</Selo>}
                    <Icone nome="direita" className="size-4 text-texto-4" />
                  </Link>
                  <button
                    type="button"
                    className="catalogo-caso__remover"
                    aria-label={`Remover ${identificacaoCaso(c)} do catálogo local`}
                    aria-expanded={confirmacao === c.caso_id}
                    title="Remover do catálogo local"
                    disabled={removerCaso.isPending}
                    onClick={() => setConfirmacao((atual) => atual === c.caso_id ? null : c.caso_id)}
                  >
                    <Icone nome="fechar" className="size-4" />
                  </button>
                </div>
                {confirmacao === c.caso_id && (
                  <div className="catalogo-caso__confirmacao" role="alertdialog" aria-label="Confirmar remoção do catálogo">
                    <Icone nome="alerta" className="mt-0.5 size-4 shrink-0 text-erro" />
                    <div className="min-w-0 flex-1">
                      <p className="text-sm font-medium text-texto">Remover este caso do catálogo local?</p>
                      <p className="mt-0.5 text-xs text-texto-2">A pasta, o banco e todas as evidências permanecerão intactos. O caso poderá ser catalogado novamente por “Abrir caso existente”.</p>
                    </div>
                    <div className="flex shrink-0 items-center gap-2">
                      <Botao tamanho="p" variante="fantasma" onClick={() => setConfirmacao(null)}>Cancelar</Botao>
                      <Botao
                        tamanho="p"
                        variante="perigo"
                        carregando={removerCaso.isPending}
                        onClick={() => removerCaso.mutate(c.caso_id, { onSuccess: () => setConfirmacao(null) })}
                      >
                        Remover do catálogo
                      </Botao>
                    </div>
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      <footer className="welcome-sied__rodape"><Icone nome="cadeado" className="size-3.5 text-ok" /> Seus dados permanecem nesta estação. O SIED não utiliza internet.</footer>
    </div>
  );
}
