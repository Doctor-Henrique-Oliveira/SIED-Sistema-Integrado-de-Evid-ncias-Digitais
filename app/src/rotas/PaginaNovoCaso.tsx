import { open } from "@tauri-apps/plugin-dialog";
import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { normalizarErro, type NovoCaso } from "../api/cliente";
import { useCriarCaso, useInfoApp, usePastaEmRede } from "../api/consultas";
import { Botao } from "../componentes/Botao";
import { AreaTexto, Campo } from "../componentes/Campo";
import { Aviso, Cabecalho, Cartao } from "../componentes/Estrutura";
import { localDaPasta, ouNulo } from "../util/formatar";

const vazio = {
  procedimento: "",
  numero_procedimento: "",
  numero_bo: "",
  investigado: "",
  vitima: "",
  delegado: "",
  responsavel_analise: "",
  unidade: "",
  observacoes: "",
};

export function PaginaNovoCaso() {
  const info = useInfoApp();
  const criar = useCriarCaso();
  const navegar = useNavigate();
  const [form, setForm] = useState(vazio);
  const [pastaBase, setPastaBase] = useState<string | null>(null);

  const falha = criar.error ? normalizarErro(criar.error) : null;
  const erro = (campo: string) => falha?.doCampo(campo);
  const campo = (nome: keyof typeof vazio) => ({
    name: nome,
    value: form[nome],
    onChange: (e: { target: { value: string } }) => setForm((f) => ({ ...f, [nome]: e.target.value })),
    erro: erro(nome),
  });

  const pastaExibida = pastaBase ?? info.data?.pasta_casos_padrao ?? null;
  const emRede = usePastaEmRede(pastaExibida);
  const nuvem = pastaExibida ? localDaPasta(pastaExibida, emRede.data) : null;

  async function escolherPasta() {
    const p = await open({ directory: true, title: "Pasta onde os casos serão criados" });
    if (typeof p === "string") setPastaBase(p);
  }

  function enviar(e: FormEvent) {
    e.preventDefault();
    const dados: NovoCaso = {
      procedimento: ouNulo(form.procedimento),
      numero_procedimento: ouNulo(form.numero_procedimento),
      numero_bo: ouNulo(form.numero_bo),
      investigado: ouNulo(form.investigado),
      vitima: ouNulo(form.vitima),
      delegado: ouNulo(form.delegado),
      responsavel_analise: form.responsavel_analise,
      unidade: form.unidade,
      observacoes: ouNulo(form.observacoes),
    };
    criar.mutate({ dados, pastaBase }, { onSuccess: (c) => navegar(`/casos/${c.caso.id}`) });
  }

  return (
    <form onSubmit={enviar} noValidate>
      <Cabecalho
        icone="casos"
        sobre="Casos"
        titulo="Novo caso"
        subtitulo="Informe o procedimento ao qual a análise está vinculada."
      />

      {falha && falha.erro.codigo !== "validacao" && (
        <div className="mb-4">
          <Aviso>{falha.message}</Aviso>
        </div>
      )}

      <div className="grid max-w-5xl gap-4">
        <Cartao titulo="Procedimento" icone="documento">
          <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
            <Campo rotulo="Tipo de procedimento" placeholder="Ex.: Inquérito Policial" {...campo("procedimento")} />
            <Campo rotulo="Número do procedimento" placeholder="Ex.: IP 0123/2026" {...campo("numero_procedimento")} />
            <Campo rotulo="Número do BO" {...campo("numero_bo")} ajuda="Informe o procedimento, o BO ou ambos." />
            <Campo rotulo="Investigado" {...campo("investigado")} />
            <Campo rotulo="Vítima" {...campo("vitima")} />
            <Campo rotulo="Delegado(a)" {...campo("delegado")} />
          </div>
        </Cartao>

        <Cartao titulo="Análise" icone="usuario">
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <Campo rotulo="Responsável pela análise" obrigatorio {...campo("responsavel_analise")} />
            <Campo rotulo="Unidade" obrigatorio {...campo("unidade")} />
            <div className="md:col-span-2">
              <AreaTexto rotulo="Observações" {...campo("observacoes")} />
            </div>
          </div>
        </Cartao>

        <Cartao titulo="Armazenamento" icone="disco">
          <div className="flex flex-wrap items-center justify-between gap-3 text-sm">
            <div className="min-w-0">
              <p className="text-texto-2">O caso será criado numa nova pasta dentro de:</p>
              <p className="truncate font-mono text-xs text-texto">{pastaExibida ?? "…"}</p>
            </div>
            <Botao variante="secundario" onClick={escolherPasta}>
              Alterar pasta…
            </Botao>
          </div>
          {nuvem && (
            <div className="mt-3">
              <Aviso tipo="atencao">
                Esta pasta parece estar em {nuvem === "compartilhamento de rede" ? "um" : "uma pasta do"} {nuvem}. Os
                dados do caso sairiam desta estação, e a sincronização ou a rede podem danificar o banco do caso enquanto
                ele está aberto. Prefira uma pasta local, como C:\SIED\Casos.
              </Aviso>
            </div>
          )}
        </Cartao>
      </div>

      <div className="mt-5 flex max-w-5xl justify-end gap-2">
        <Botao variante="fantasma" onClick={() => navegar("/")}>
          Cancelar
        </Botao>
        <Botao type="submit" carregando={criar.isPending}>
          Criar caso
        </Botao>
      </div>
    </form>
  );
}
