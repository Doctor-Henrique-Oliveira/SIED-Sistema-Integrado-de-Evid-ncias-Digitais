// Barra lateral fixa: marca, caso aberto, módulos e estado da estação.
// Módulos que dependem de um caso ficam desativados (com explicação) enquanto
// nenhum caso está aberto.

import { NavLink } from "react-router-dom";
import type { Caso, InfoApp } from "../api/cliente";
import { identificacaoCaso } from "../util/formatar";
import { Icone, type NomeIcone } from "./Icone";

type Modulo = {
  rotulo: string;
  icone: NomeIcone;
  /** Caminho; `:caso` vira o id do caso aberto. */
  para: string;
  exato?: boolean;
  dica?: string;
};

export const grupos: { titulo: string; modulos: Modulo[] }[] = [
  {
    titulo: "Geral",
    modulos: [
      { rotulo: "Dashboard", icone: "painel", para: "/casos/:caso", exato: true, dica: "Visão geral do caso aberto" },
      { rotulo: "Casos", icone: "casos", para: "/", exato: true, dica: "Casos desta estação" },
    ],
  },
  {
    titulo: "Evidências",
    modulos: [
      { rotulo: "Dispositivos", icone: "dispositivo", para: "/casos/:caso/dispositivos" },
      { rotulo: "Extrações", icone: "extracao", para: "/casos/:caso/extracoes" },
    ],
  },
  {
    titulo: "Análise",
    modulos: [
      { rotulo: "Conversas", icone: "conversas", para: "/casos/:caso/conversas" },
      { rotulo: "Mídias", icone: "midias", para: "/casos/:caso/midias" },
      { rotulo: "Arquivos", icone: "arquivos", para: "/casos/:caso/arquivos" },
      { rotulo: "Contatos", icone: "contatos", para: "/casos/:caso/contatos" },
      { rotulo: "Localizações", icone: "localizacoes", para: "/casos/:caso/localizacoes" },
      { rotulo: "Consultas", icone: "consultas", para: "/casos/:caso/consultas" },
    ],
  },
  {
    titulo: "Saída",
    modulos: [{ rotulo: "Relatórios", icone: "relatorios", para: "/casos/:caso/relatorios" }],
  },
  {
    titulo: "Controle",
    modulos: [{ rotulo: "Auditoria", icone: "integridade", para: "/casos/:caso/auditoria" }],
  },
];

export const configuracoes: Modulo = { rotulo: "Configurações", icone: "config", para: "/configuracoes", exato: true };

function Item({ m, casoId }: { m: Modulo; casoId: string | undefined }) {
  const precisaCaso = m.para.includes(":caso");
  const base = "sied-sidebar__item group relative flex h-9 items-center gap-2.5 rounded-md px-3 text-sm transition-colors";
  if (precisaCaso && !casoId) {
    return (
      <span
        aria-disabled="true"
        title="Abra um caso para usar este módulo"
        className={`${base} cursor-not-allowed text-texto-4`}
      >
        <Icone nome={m.icone} className="size-4" />
        {m.rotulo}
      </span>
    );
  }
  return (
    <NavLink
      to={m.para.replace(":caso", casoId ?? "")}
      end={m.exato}
      title={m.dica}
      className={({ isActive }) =>
        `${base} ${
          isActive ? "bg-sup-2 font-semibold text-texto ring-1 ring-inset ring-borda-2/70" : "text-texto-2 hover:bg-sup/80 hover:text-texto"
        }`
      }
    >
      {({ isActive }) => (
        <>
          {/* Marca do item ativo: filete dourado rente à borda da barra. */}
          {isActive && <span className="absolute -left-3 top-1.5 bottom-1.5 w-0.5 rounded-r-sm bg-ouro" aria-hidden />}
          <Icone nome={m.icone} className={`size-4 ${isActive ? "text-ouro" : "text-texto-3 group-hover:text-texto-2"}`} />
          {m.rotulo}
        </>
      )}
    </NavLink>
  );
}

export function BarraLateral({ caso, info }: { caso: Caso | undefined; info: InfoApp | undefined }) {
  return (
    <aside className="sied-sidebar flex w-[238px] shrink-0 flex-col border-r border-borda bg-lateral">
      <div className="sied-sidebar__marca flex h-[58px] shrink-0 items-center gap-3 border-b border-borda px-4">
        <span className="grid size-8 place-items-center rounded-md border border-ouro/45 bg-ouro/5 text-ouro">
          <Icone nome="escudo" className="size-4" />
        </span>
        <div className="min-w-0 leading-none">
          <p className="text-sm font-semibold tracking-[0.2em] text-texto">SIED</p>
          <p className="mt-1 truncate text-3xs uppercase tracking-[0.08em] text-texto-3">Estação forense</p>
        </div>
      </div>

      <div className="sied-sidebar__caso border-b border-borda px-4 py-3.5">
        <p className="sobrelinha flex items-center gap-2"><span className="sied-sidebar__pulso size-1.5 rounded-full bg-ok" aria-hidden /> Caso ativo</p>
        {caso ? (
          <div className="mt-1.5 min-w-0">
            <p className="truncate text-sm font-semibold tracking-tight text-ouro-claro" title={identificacaoCaso(caso)}>{identificacaoCaso(caso)}</p>
            <p className="mt-0.5 truncate text-2xs text-texto-3" title={caso.unidade}>{caso.unidade}</p>
          </div>
        ) : (
          <p className="mt-0.5 text-sm text-texto-3">Nenhum caso aberto</p>
        )}
      </div>

      <nav className="flex-1 space-y-4 overflow-y-auto px-3 py-4" aria-label="Navegação principal">
        {grupos.map((g) => (
          <div key={g.titulo} className="sied-sidebar__grupo">
            <p className="sobrelinha px-2.5 pb-1.5 text-texto-4!">{g.titulo}</p>
            <div className="space-y-px">
              {g.modulos.map((m) => (
                <Item key={m.rotulo} m={m} casoId={caso?.id} />
              ))}
            </div>
          </div>
        ))}
      </nav>

      <div className="border-t border-borda px-3 py-2">
        <Item m={configuracoes} casoId={caso?.id} />
      </div>
      <footer className="sied-sidebar__rodape flex items-center justify-between gap-2 whitespace-nowrap border-t border-borda px-4 py-2.5 text-2xs text-texto-3">
        <p className="flex items-center gap-1.5" title="O SIED não usa internet: tudo fica nesta estação">
          <span className="size-1.5 rounded-full bg-ok" aria-hidden />
          <span className="text-texto-2">Offline</span> · dados locais
        </p>
        {info && (
          <p className="tabular font-mono text-3xs" title={`Versão ${info.versao}`}>
            v{info.versao}
          </p>
        )}
      </footer>
    </aside>
  );
}
