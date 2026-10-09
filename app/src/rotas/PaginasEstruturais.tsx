// Localizações (lista cronológica dos registros com coordenadas; o mapa offline ainda
// não existe) e Consultas (linha do tempo do caso; consultas salvas ainda por vir).
// Nenhum dado fictício: só o que o banco do caso já tem.
// Configurações mostra, somente leitura, como esta estação está configurada.

import { useState, type ReactNode } from "react";
import type { Caso } from "../api/cliente";
import { useInfoApp, useProvedoresEstacao, useSalvarProvedorEstacao } from "../api/consultas";
import type { NovoProvedorEstacao } from "../api/cliente";
import { Abas, Aviso, Cabecalho, Cartao, Carregando, Definicoes, Indisponivel, Selo } from "../componentes/Estrutura";
import { Botao } from "../componentes/Botao";
import { LinhaDoTempo } from "../componentes/LinhaTempo";
import { MapaOffline } from "../componentes/MapaOffline";
import { identificacaoCaso } from "../util/formatar";
import { ComCasoAberto } from "./PaginaCaso";

export function PaginaLocalizacoes() {
  return <ComCasoAberto>{(caso) => <Localizacoes caso={caso} />}</ComCasoAberto>;
}

const SO_COORDENADAS = { com_coordenadas: true };

function Localizacoes({ caso }: { caso: Caso }) {
  return (
    <>
      <Cabecalho
        icone="localizacoes"
        sobre={identificacaoCaso(caso)}
        titulo="Localizações"
        subtitulo="Registros com coordenadas extraídos dos aparelhos, em ordem cronológica, com latitude e longitude como lidas."
      />
      <div className="space-y-4">
        <MapaOffline caso={caso} />
        <LinhaDoTempo casoId={caso.id} fuso={caso.fuso_horario} fixo={SO_COORDENADAS} coordenadas rotulo="Localizações" />
      </div>
    </>
  );
}

type AbaConsultas = "linha" | "salvas";

export function PaginaConsultas() {
  const [aba, setAba] = useState<AbaConsultas>("linha");
  return (
    <ComCasoAberto>
      {(caso) => (
        <>
          <Cabecalho
            icone="consultas"
            sobre={identificacaoCaso(caso)}
            titulo="Consultas"
            subtitulo="Consultas locais sobre os dados do caso. Nada sai desta estação."
          />
          <div className="space-y-3">
            <Abas<AbaConsultas>
              rotulo="Consultas do caso"
              atual={aba}
              mudar={setAba}
              abas={[
                ["linha", "Linha do tempo"],
                ["salvas", "Consultas salvas"],
              ]}
            />
            {aba === "linha" ? (
              <LinhaDoTempo casoId={caso.id} fuso={caso.fuso_horario} />
            ) : (
              <Indisponivel
                titulo="Consultas salvas"
                icone="consultas"
                previsto={["Consultas salvas sobre os dados do caso", "Histórico com data, analista e resultado", "Execução local, sem internet"]}
              >
                Espaço reservado para guardar consultas e reexecutá-las. O SIED não acessa serviços externos: tudo o que for consultado
                aqui sairá do banco local do caso.
              </Indisponivel>
            )}
          </div>
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
  const provedores = useProvedoresEstacao();
  const salvar = useSalvarProvedorEstacao();
  const [formulario, setFormulario] = useState<NovoProvedorEstacao>({
    id: "",
    nome: "",
    tipo: "transcricao",
    modo: "local",
    habilitado: false,
    caminho: "",
    versao: "",
    observacao: "",
  });
  const [mensagem, setMensagem] = useState<string | null>(null);
  const podeSalvar = formulario.id.trim() !== "" && formulario.nome.trim() !== "" && formulario.tipo.trim() !== "";

  function atualizar<K extends keyof NovoProvedorEstacao>(campo: K, valor: NovoProvedorEstacao[K]) {
    setFormulario((atual) => ({ ...atual, [campo]: valor }));
    setMensagem(null);
  }

  function enviar() {
    if (!podeSalvar) return;
    setMensagem(null);
    salvar.mutate(
      { ...formulario, caminho: formulario.caminho?.trim() || null, versao: formulario.versao?.trim() || null, observacao: formulario.observacao?.trim() || null },
      {
        onSuccess: () => {
          setMensagem("Provedor registrado no catálogo da estação.");
          setFormulario((atual) => ({ ...atual, id: "", nome: "", caminho: "", versao: "", observacao: "", habilitado: false }));
        },
        onError: (erro) => setMensagem(erro instanceof Error ? erro.message : String(erro)),
      },
    );
  }
  return (
    <>
      <Cabecalho
        icone="config"
        titulo="Configurações"
        subtitulo="Configuração local da estação, com provedores desabilitados por padrão e sem saída automática de dados."
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
            <ul className="space-y-2 text-sm text-texto-2">
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
          <Cartao
            titulo="Provedores locais"
            icone="disco"
            acoes={<Selo tom="ok">Sem segredos</Selo>}
          >
            <p className="mb-4 max-w-3xl text-sm text-texto-2">
              O catálogo guarda somente o manifesto do motor: nome, tipo, caminho e versão. O SIED não executa o provedor nesta tela,
              não abre rede e nunca grava tokens ou conteúdo de casos. Provedores externos permanecem bloqueados nesta etapa.
            </p>
            {mensagem && <div className="mb-4"><Aviso tipo={mensagem.includes("registrado") ? "sucesso" : "erro"}>{mensagem}</Aviso></div>}
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              <label className="text-sm text-texto-2">ID estável<input className="entrada mt-1 w-full" value={formulario.id} onChange={(e) => atualizar("id", e.target.value)} placeholder="vosk-pt-br" /></label>
              <label className="text-sm text-texto-2">Nome exibido<input className="entrada mt-1 w-full" value={formulario.nome} onChange={(e) => atualizar("nome", e.target.value)} placeholder="Vosk local" /></label>
              <label className="text-sm text-texto-2">Tipo<input className="entrada mt-1 w-full" value={formulario.tipo} onChange={(e) => atualizar("tipo", e.target.value)} placeholder="transcricao, ocr, visao…" /></label>
              <label className="text-sm text-texto-2 sm:col-span-2">Caminho do executável (opcional)<input className="entrada mt-1 w-full font-mono text-xs" value={formulario.caminho ?? ""} onChange={(e) => atualizar("caminho", e.target.value)} placeholder="C:\\Ferramentas\\motor.exe" /></label>
              <label className="text-sm text-texto-2">Versão declarada<input className="entrada mt-1 w-full" value={formulario.versao ?? ""} onChange={(e) => atualizar("versao", e.target.value)} placeholder="não informado" /></label>
              <label className="flex items-center gap-2 self-end pb-2 text-sm text-texto-2"><input type="checkbox" checked={formulario.habilitado} onChange={(e) => atualizar("habilitado", e.target.checked)} /> habilitar uso local</label>
              <label className="text-sm text-texto-2 sm:col-span-2 lg:col-span-3">Observação operacional<textarea className="entrada mt-1 min-h-16 w-full" value={formulario.observacao ?? ""} onChange={(e) => atualizar("observacao", e.target.value)} placeholder="Como o motor foi instalado/configurado" /></label>
            </div>
            <div className="mt-4 flex items-center gap-3"><Botao tamanho="p" icone="mais" disabled={!podeSalvar} carregando={salvar.isPending} onClick={enviar}>Registrar provedor</Botao><span className="text-xs text-texto-3">Habilitar não executa nada automaticamente.</span></div>
            <div className="mt-5 border-t border-borda pt-4">
              <p className="sobrelinha mb-2">Manifestos registrados</p>
              {provedores.isPending ? <Carregando texto="Lendo catálogo de provedores…" /> : provedores.isError ? <Aviso>Não foi possível ler o catálogo de provedores.</Aviso> : provedores.data?.length ? <ul className="divide-y divide-borda rounded border border-borda">{provedores.data.map((p) => <li key={p.id} className="flex flex-wrap items-center justify-between gap-3 px-3 py-2.5"><div><p className="text-sm font-medium text-texto">{p.nome} <span className="font-mono text-2xs text-texto-3">{p.id}</span></p><p className="text-xs text-texto-3">{p.tipo} · {p.modo} · {p.versao ?? "versão não informada"}</p>{p.caminho && <p className="mt-0.5 break-all font-mono text-2xs text-texto-3">{p.caminho}</p>}</div><Selo tom={p.habilitado ? "ok" : "neutro"}>{p.habilitado ? "HABILITADO" : "DESABILITADO"}</Selo></li>)}</ul> : <p className="text-sm text-texto-3">Nenhum provedor registrado nesta estação.</p>}
            </div>
          </Cartao>
        </div>
      ) : null}
    </>
  );
}
