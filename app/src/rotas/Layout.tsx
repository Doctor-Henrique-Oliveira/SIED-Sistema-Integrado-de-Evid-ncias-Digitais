import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { useCasoAtual, useFecharCaso, useInfoApp } from "../api/consultas";
import { Aviso } from "../componentes/Estrutura";
import { identificacaoCaso } from "../util/formatar";

function ItemMenu({ para, children }: { para: string; children: React.ReactNode }) {
  return (
    <NavLink
      to={para}
      end
      className={({ isActive }) =>
        `block truncate rounded-lg px-3 py-2 text-sm transition-colors ${
          isActive ? "bg-painel-claro font-medium text-white" : "text-slate-300 hover:bg-painel-claro/60 hover:text-white"
        }`
      }
    >
      {children}
    </NavLink>
  );
}

export function Layout() {
  const info = useInfoApp();
  const casoAtual = useCasoAtual();
  const fechar = useFecharCaso();
  const navegar = useNavigate();
  const caso = casoAtual.data?.caso;

  return (
    <div className="flex h-full">
      <aside className="flex w-64 shrink-0 flex-col bg-painel text-slate-200">
        <div className="flex items-center gap-3 px-5 py-5">
          <img src="/icone.svg" alt="" className="size-9" />
          <div>
            <p className="text-lg font-semibold leading-tight text-white">SIED</p>
            <p className="text-[11px] leading-tight text-slate-400">Sistema Integrado de Evidências Digitais</p>
          </div>
        </div>

        <nav className="flex-1 space-y-6 px-3" aria-label="Navegação principal">
          <div className="space-y-1">
            <ItemMenu para="/">Casos</ItemMenu>
          </div>

          {caso && (
            <div>
              <p className="px-3 pb-2 text-[11px] font-semibold uppercase tracking-wider text-slate-400">Caso aberto</p>
              <div className="space-y-1">
                <ItemMenu para={`/casos/${caso.id}`}>{identificacaoCaso(caso)}</ItemMenu>
              </div>
              <button
                type="button"
                onClick={() => fechar.mutate(undefined, { onSuccess: () => navegar("/") })}
                className="mt-2 px-3 text-xs text-slate-400 hover:text-white"
              >
                Fechar caso
              </button>
            </div>
          )}
        </nav>

        <footer className="border-t border-white/10 px-5 py-4 text-[11px] leading-relaxed text-slate-400">
          {info.data && (
            <>
              <p className="truncate" title={`${info.data.usuario}@${info.data.maquina}`}>
                {info.data.usuario} · {info.data.maquina}
              </p>
              <p>Versão {info.data.versao} · Offline</p>
            </>
          )}
        </footer>
      </aside>

      <main className="min-w-0 flex-1 overflow-y-auto">
        <div className="mx-auto max-w-6xl px-8 py-8">
          {info.isError ? (
            <Aviso>
              <p className="font-medium">O SIED não conseguiu iniciar.</p>
              <p className="mt-1">{info.error.message}</p>
            </Aviso>
          ) : (
            <Outlet />
          )}
        </div>
      </main>
    </div>
  );
}
