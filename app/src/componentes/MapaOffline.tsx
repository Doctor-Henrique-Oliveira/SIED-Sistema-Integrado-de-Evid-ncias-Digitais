import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import type { AchadoArtefato, Caso } from "../api/cliente";
import { useLinhaDoTempo } from "../api/consultas";
import { destinoRegistro } from "./LinhaTempo";
import { Aviso, Carregando, Selo, Vazio } from "./Estrutura";
import { Botao } from "./Botao";

const LIMITE = 200;

/** Mapa esquemático local: projeção dos pontos do caso, sem tiles ou rede. */
export function MapaOffline({ caso }: { caso: Caso }) {
  const [aberto, setAberto] = useState(false);
  const pontos = useLinhaDoTempo(caso.id, { com_coordenadas: true }, 0, LIMITE, aberto);
  const coordenadas = useMemo(() => (pontos.data?.itens ?? []).filter((x) => x.artefato.latitude !== null && x.artefato.longitude !== null), [pontos.data]);
  const limites = useMemo(() => {
    if (coordenadas.length === 0) return null;
    const latitudes = coordenadas.map((x) => x.artefato.latitude!);
    const longitudes = coordenadas.map((x) => x.artefato.longitude!);
    return { minLat: Math.min(...latitudes), maxLat: Math.max(...latitudes), minLon: Math.min(...longitudes), maxLon: Math.max(...longitudes) };
  }, [coordenadas]);
  function ponto(a: AchadoArtefato) {
    if (!limites || a.artefato.latitude === null || a.artefato.longitude === null) return { x: 0, y: 0 };
    const dx = Math.max(limites.maxLon - limites.minLon, 0.000001);
    const dy = Math.max(limites.maxLat - limites.minLat, 0.000001);
    return { x: 8 + ((a.artefato.longitude - limites.minLon) / dx) * 84, y: 92 - ((a.artefato.latitude - limites.minLat) / dy) * 84 };
  }
  return <section aria-label="Mapa offline do caso" className="painel p-3">
    <div className="flex flex-wrap items-center justify-between gap-2"><div><h2 className="text-sm font-semibold text-texto">Mapa offline</h2><p className="mt-1 text-xs text-texto-2">Projeção local dos pontos extraídos; não usa mapas externos nem infere trajetos.</p></div><Botao tamanho="p" variante="secundario" onClick={() => setAberto((x) => !x)}>{aberto ? "Ocultar mapa" : "Abrir mapa"}</Botao></div>
    {aberto && <div className="mt-3 space-y-3">
      {pontos.isPending ? <Carregando texto="Carregando pontos com coordenadas…" /> : pontos.isError ? <Aviso>{pontos.error.message}</Aviso> : pontos.data?.total === 0 ? <Vazio titulo="Nenhum ponto com coordenadas">A extração não possui registros com latitude e longitude utilizáveis.</Vazio> : <>
        {pontos.data && pontos.data.total > LIMITE && <Aviso tipo="info">Exibindo {LIMITE} de {pontos.data.total} pontos. Use a linha do tempo para filtrar período ou extração antes de analisar lotes maiores.</Aviso>}
        {limites && <svg viewBox="0 0 100 100" role="img" aria-label="Pontos de localização do caso" className="h-72 w-full rounded-md border border-borda bg-fundo">
          <path d="M8 8V92H92" fill="none" stroke="currentColor" strokeOpacity=".2" strokeWidth=".35" />
          {coordenadas.map((achado) => { const p = ponto(achado); const destino = destinoRegistro(caso.id, achado); return <Link key={achado.artefato.id} to={destino.para}><circle cx={p.x} cy={p.y} r="1.6" fill="var(--color-ouro)" stroke="var(--color-fundo)" strokeWidth=".6"><title>{`${achado.artefato.latitude}, ${achado.artefato.longitude} · registro #${achado.artefato.id}`}</title></circle></Link>; })}
        </svg>}
        <div className="flex flex-wrap items-center gap-2 text-xs text-texto-2"><Selo tom="info">{coordenadas.length} ponto(s) nesta janela</Selo><span>Lat {limites?.minLat.toFixed(5)}–{limites?.maxLat.toFixed(5)} · Lon {limites?.minLon.toFixed(5)}–{limites?.maxLon.toFixed(5)}</span></div>
        <p className="text-2xs text-texto-3">Cada ponto é navegável para o registro de origem. Proximidade espacial não estabelece deslocamento, autoria ou causalidade.</p>
      </>}
    </div>}
  </section>;
}
