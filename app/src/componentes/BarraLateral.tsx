// Barra lateral fixa: marca, caso aberto, os 12 módulos e o estado da estação.
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
];

const configuracoes: Modulo = { rotulo: "Configurações", icone: "config", para: "/configuracoes", exato: true };

function Item({ m, casoId }: { m: Modulo; casoId: string | undefined }) {
  const precisaCaso = m.para.includes(":caso");
  const base = "group flex items-center gap-2.5 rounded-lg px-2.5 py-[7px] text-[13px] transition-colors";
  if (precisaCaso && !casoId) {
    return (
      <span
        aria-disabled="true"
        title="Abra um caso para usar este módulo"
        className={`${base} cursor-not-allowed text-texto-3/60`}
      >
        <Icone nome={m.icone} className="size-[18px]" />
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
          isActive
            ? "bg-ouro/12 font-medium text-ouro-claro shadow-[inset_2px_0_0_var(--color-ouro)]"
            : "text-texto-2 hover:bg-sup-2 hover:text-texto"
        }`
      }
    >
      {({ isActive }) => (
        <>
          <Icone nome={m.icone} className={`size-[18px] ${isActive ? "text-ouro" : "text-texto-3 group-hover:text-texto-2"}`} />
          {m.rotulo}
        </>
      )}
    </NavLink>
  );
}

export function BarraLateral({ caso, info }: { caso: Caso | undefined; info: InfoApp | undefined }) {
  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-borda bg-lateral">
      <div className="flex items-center gap-2.5 px-4 pb-3 pt-4">
        <span className="grid size-9 place-items-center rounded-lg bg-gradient-to-b from-ouro/25 to-ouro/5 text-ouro ring-1 ring-ouro/40">
          <Icone nome="escudo" className="size-5" />
        </span>
        <div className="min-w-0">
          <p className="text-[15px] font-semibold leading-tight tracking-[0.12em] text-texto">SIED</p>
          <p className="truncate text-[10px] leading-tight text-texto-3">Sistema Integrado de Evidências Digitais</p>
        </div>
      </div>

      <div className="mx-3 mb-2 rounded-lg border border-borda bg-sup px-3 py-2">
        <p className="text-[10px] font-semibold uppercase tracking-wider text-texto-3">Caso aberto</p>
        {caso ? (
          <p className="truncate text-[13px] font-medium text-ouro-claro" title={identificacaoCaso(caso)}>
            {identificacaoCaso(caso)}
          </p>
        ) : (
          <p className="text-[13px] text-texto-3">Nenhum</p>
        )}
      </div>

      <nav className="flex-1 space-y-3 overflow-y-auto px-3 py-1" aria-label="Navegação principal">
        {grupos.map((g) => (
          <div key={g.titulo}>
            <p className="px-2.5 pb-1 text-[10px] font-semibold uppercase tracking-wider text-texto-3">{g.titulo}</p>
            <div className="space-y-0.5">
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
      <footer className="space-y-1 border-t border-borda px-4 py-3 text-[11px] text-texto-3">
        <p className="flex items-center gap-1.5">
          <span className="size-1.5 rounded-full bg-ok" aria-hidden />
          <span className="text-texto-2">Offline</span> · dados locais
        </p>
        {info && <p className="tabular">Versão {info.versao}</p>}
      </footer>
    </aside>
  );
}
