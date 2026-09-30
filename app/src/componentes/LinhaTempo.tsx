// Linha do tempo do caso: registros com data de todas as extrações e apps, em ordem
// cronológica, agrupados por dia no fuso do caso. Só leitura: a ordem é a da data que o
// parser gravou; registros sem data ficam fora e são contados à parte.

import { useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import type { AchadoArtefato, CasoId, FiltroLinhaTempo } from "../api/cliente";
import { useExtracoes, useLinhaDoTempo, useResumoLinhaDoTempo } from "../api/consultas";
import { Botao } from "./Botao";
import { Aviso, Carregando, entrada, FaixaIndicadores, Filtros, Indicador, Paginacao, Selo, Vazio } from "./Estrutura";
import { dataHoraMs, dia, hora, limiteDia, numero } from "../util/formatar";
import { nomeTipoArtefato, rotuloApp, rotuloSubtipo } from "../util/rotulos";

const LIMITE = 100;

/** Onde abrir um registro: na conversa, em Contatos ou na extração. */
export function destinoRegistro(casoId: CasoId, a: AchadoArtefato): { para: string; acao: string } {
  const reg = a.artefato;
  if (a.conversa_id != null) return { para: `/casos/${casoId}/conversas/${a.conversa_id}?registro=${reg.id}`, acao: "abrir na conversa →" };
  if (reg.tipo === "chamada" || reg.tipo === "contato")
    return {
      para: `/casos/${casoId}/contatos?aba=${reg.tipo === "chamada" ? "chamadas" : "agenda"}&registro=${reg.id}`,
      acao: "abrir em Contatos →",
    };
  return { para: `/casos/${casoId}/extracoes/${a.extracao_id}`, acao: "abrir na extração →" };
}

function sentido(deMim: boolean | null): string {
  return deMim == null ? "" : deMim ? "Enviada" : "Recebida";
}

/** Chave do dia no fuso do caso (AAAA-MM-DD), para agrupar. */
function chaveDia(ms: number, fuso: string): string {
  return new Intl.DateTimeFormat("en-CA", { timeZone: fuso, year: "numeric", month: "2-digit", day: "2-digit" }).format(new Date(ms));
}

function Item({ a, casoId, fuso, coordenadas }: { a: AchadoArtefato; casoId: CasoId; fuso: string; coordenadas: boolean }) {
  const reg = a.artefato;
  const { para, acao } = destinoRegistro(casoId, a);
  const quem = reg.remetente ?? (reg.destinatarios.join(", ") || null);
  const texto = reg.conteudo ?? reg.nome_arquivo ?? (reg.subtipo && reg.subtipo !== reg.app ? rotuloSubtipo(reg.subtipo) : "—");
  return (
    <li>
      <Link to={para} className="group grid grid-cols-[4.5rem_minmax(0,1fr)] gap-3 px-4 py-2 transition-colors hover:bg-sup-2" data-teste="evento">
        <span className="tabular pt-0.5 font-mono text-xs text-texto-2">{hora(reg.data_hora_ms!, fuso)}</span>
        <span className="min-w-0">
          <span className="flex flex-wrap items-center gap-x-2.5 gap-y-1 text-2xs text-texto-3">
            <span className="font-medium uppercase text-texto-2">{rotuloApp(reg.app)}</span>
            <Selo tom={reg.tipo === "chamada" ? "ouro" : reg.tipo === "mensagem" ? "neutro" : "info"}>
              {nomeTipoArtefato[reg.tipo] ?? reg.tipo}
            </Selo>
            {reg.subtipo && reg.subtipo !== "texto" && reg.subtipo !== reg.app && rotuloSubtipo(reg.subtipo) !== texto && (
              <span>{rotuloSubtipo(reg.subtipo)}</span>
            )}
            {/* Na chamada o subtipo (recebida, efetuada, perdida...) já diz o sentido. */}
            {reg.tipo !== "chamada" && <span>{sentido(reg.de_mim)}</span>}
            {quem && <span className="truncate font-mono">{quem}</span>}
            {reg.estado_registro !== "ativo" && <Selo tom="atencao">{reg.estado_registro === "apagado" ? "Apagado" : reg.estado_registro}</Selo>}
            <span className="ml-auto text-ouro-claro opacity-0 transition-opacity group-hover:opacity-100">{acao}</span>
          </span>
          <span className="mt-0.5 block truncate text-sm text-texto">{texto}</span>
          {coordenadas && reg.latitude != null && reg.longitude != null && (
            <span className="mt-0.5 block font-mono text-xs text-info" data-teste="coordenadas">
              {reg.latitude}, {reg.longitude}
            </span>
          )}
        </span>
      </Link>
    </li>
  );
}

/** Lista agrupada por dia (mantém a ordem recebida, crescente ou decrescente). */
function PorDia({ itens, casoId, fuso, coordenadas }: { itens: AchadoArtefato[]; casoId: CasoId; fuso: string; coordenadas: boolean }) {
  const grupos: { chave: string; ms: number; itens: AchadoArtefato[] }[] = [];
  for (const a of itens) {
    const ms = a.artefato.data_hora_ms!;
    const chave = chaveDia(ms, fuso);
    const ultimo = grupos[grupos.length - 1];
    if (ultimo && ultimo.chave === chave) ultimo.itens.push(a);
    else grupos.push({ chave, ms, itens: [a] });
  }
  return (
    <div className="painel overflow-hidden">
      {grupos.map((g) => (
        <section key={`${g.chave}-${g.itens[0]!.artefato.id}`} aria-label={dia(g.ms, fuso)}>
          <h3 className="sticky top-0 z-10 flex items-center justify-between border-y border-borda bg-sup-2/95 px-4 py-1.5 text-xs font-medium text-texto-2 first:border-t-0">
            <span className="first-letter:uppercase">{dia(g.ms, fuso)}</span>
            <span className="tabular text-texto-3">{numero(g.itens.length)} nesta página</span>
          </h3>
          <ul className="divide-y divide-borda">
            {g.itens.map((a) => (
              <Item key={a.artefato.id} a={a} casoId={casoId} fuso={fuso} coordenadas={coordenadas} />
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}

export function LinhaDoTempo({ casoId, fuso, fixo = {}, coordenadas = false, rotulo = "Linha do tempo" }: {
  casoId: CasoId;
  fuso: string;
  /** Filtro sempre aplicado (ex.: só registros com coordenadas). */
  fixo?: FiltroLinhaTempo;
  coordenadas?: boolean;
  rotulo?: string;
}) {
  const extracoes = useExtracoes(casoId);
  const [tipo, setTipo] = useState<string | null>(null);
  const [app, setApp] = useState("");
  const [extracaoId, setExtracaoId] = useState("");
  const [de, setDe] = useState("");
  const [ate, setAte] = useState("");
  const [periodo, setPeriodo] = useState<{ de_ms?: number; ate_ms?: number }>({});
  const [recentes, setRecentes] = useState(false);
  const [deslocamento, setDeslocamento] = useState(0);

  const filtro: FiltroLinhaTempo = { ...fixo, ...periodo };
  if (tipo) filtro.tipos = [tipo];
  if (app) filtro.app = app;
  if (extracaoId) filtro.extracao_id = extracaoId;
  if (recentes) filtro.recentes_primeiro = true;

  const resumo = useResumoLinhaDoTempo(casoId, { ...filtro, recentes_primeiro: undefined });
  const pagina = useLinhaDoTempo(casoId, filtro, deslocamento, LIMITE);
  const r = resumo.data;

  const mudar = <T,>(f: (v: T) => void) => (v: T) => {
    f(v);
    setDeslocamento(0);
  };

  function aplicarPeriodo(e: FormEvent) {
    e.preventDefault();
    const p: { de_ms?: number; ate_ms?: number } = {};
    const deMs = de ? limiteDia(de, fuso) : null;
    const ateMs = ate ? limiteDia(ate, fuso, true) : null;
    if (deMs != null) p.de_ms = deMs;
    if (ateMs != null) p.ate_ms = ateMs;
    setPeriodo(p);
    setDeslocamento(0);
  }

  const total = r ? r.por_tipo.filter((t) => !tipo || t.tipo === tipo).reduce((s, t) => s + t.total, 0) : null;

  return (
    <div className="space-y-3">
      <FaixaIndicadores rotulo={`Resumo: ${rotulo}`} className="sm:grid-cols-2 xl:grid-cols-4">
        <Indicador rotulo="Registros com data" icone="relogio" tom="ouro" valor={total == null ? null : numero(total)} detalhe="que atendem aos filtros" />
        <Indicador
          rotulo="Primeiro registro"
          icone="relogio"
          valor={r?.primeira_ms != null ? <span className="text-base">{dataHoraMs(r.primeira_ms, fuso)}</span> : null}
          detalhe={`fuso do caso (${fuso})`}
        />
        <Indicador
          rotulo="Último registro"
          icone="relogio"
          valor={r?.ultima_ms != null ? <span className="text-base">{dataHoraMs(r.ultima_ms, fuso)}</span> : null}
          detalhe={`fuso do caso (${fuso})`}
        />
        <Indicador
          rotulo="Sem data utilizável"
          icone="alerta"
          tom={r && r.sem_data > 0 ? "atencao" : "neutro"}
          valor={r ? numero(r.sem_data) : null}
          detalhe="fora da lista (data ausente ou implausível)"
        />
      </FaixaIndicadores>

      <section aria-label="Filtros da linha do tempo" className="painel space-y-2.5 p-3">
        {r && r.por_tipo.length > 0 && (
          <Filtros<string | null>
            rotulo="Tipo de registro"
            atual={tipo}
            mudar={mudar(setTipo)}
            opcoes={[
              [null, `Todos (${numero(r.por_tipo.reduce((s, t) => s + t.total, 0))})`],
              ...r.por_tipo.map((t): [string, string] => [t.tipo, `${nomeTipoArtefato[t.tipo] ?? t.tipo} (${numero(t.total)})`]),
            ]}
          />
        )}
        <form onSubmit={aplicarPeriodo} className="flex flex-wrap items-center gap-3 text-xs text-texto-2">
          <select aria-label="App" value={app} onChange={(e) => mudar(setApp)(e.target.value)} className={entrada}>
            <option value="">Todos os apps</option>
            {r?.apps.map((a) => (
              <option key={a.tipo} value={a.tipo}>
                {rotuloApp(a.tipo)} ({numero(a.total)})
              </option>
            ))}
          </select>
          {(extracoes.data?.length ?? 0) > 1 && (
            <select aria-label="Extração" value={extracaoId} onChange={(e) => mudar(setExtracaoId)(e.target.value)} className={entrada}>
              <option value="">Todas as extrações</option>
              {extracoes.data?.map((x) => (
                <option key={x.id} value={x.id}>
                  {x.importador_nome} · {x.iniciada_em.slice(0, 10)}
                </option>
              ))}
            </select>
          )}
          <label className="flex items-center gap-2">
            De
            <input type="date" value={de} onChange={(e) => setDe(e.target.value)} className={entrada} />
          </label>
          <label className="flex items-center gap-2">
            Até
            <input type="date" value={ate} onChange={(e) => setAte(e.target.value)} className={entrada} />
          </label>
          <Botao type="submit" variante="secundario" tamanho="p">
            Aplicar período
          </Botao>
          <label className="ml-auto flex items-center gap-2">
            <input type="checkbox" checked={recentes} onChange={(e) => mudar(setRecentes)(e.target.checked)} />
            Mais recentes primeiro
          </label>
        </form>
      </section>

      {pagina.isPending ? (
        <Carregando texto="Montando a linha do tempo…" />
      ) : pagina.isError ? (
        <Aviso>{pagina.error.message}</Aviso>
      ) : pagina.data.total === 0 ? (
        <Vazio titulo="Nenhum registro com data" icone="relogio">
          {tipo || app || extracaoId || periodo.de_ms != null || periodo.ate_ms != null
            ? "Nenhum registro atende aos filtros escolhidos."
            : "Importe uma extração com mensagens, chamadas ou outros registros datados."}
        </Vazio>
      ) : (
        <>
          <p className="text-xs text-texto-3" role="status">
            {numero(pagina.data.total)} registro(s) em ordem {recentes ? "decrescente" : "crescente"} de data e hora. A ordem é a da data
            gravada pelo aparelho, convertida para o fuso do caso; o SIED não corrige relógios.
          </p>
          <PorDia itens={pagina.data.itens} casoId={casoId} fuso={fuso} coordenadas={coordenadas} />
          <Paginacao total={pagina.data.total} deslocamento={deslocamento} limite={LIMITE} mudar={setDeslocamento} />
        </>
      )}
    </div>
  );
}
