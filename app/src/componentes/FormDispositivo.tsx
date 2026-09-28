import { useState, type FormEvent } from "react";
import { FalhaSied, type Dispositivo, type NovoDispositivo } from "../api/cliente";
import type { SistemaOperacional } from "../api/tipos/SistemaOperacional";
import { lerCapacidade, ouNulo } from "../util/formatar";
import { Botao } from "./Botao";
import { AreaTexto, Campo, Selecao } from "./Campo";
import { Aviso } from "./Estrutura";

const vazio = {
  apelido: "",
  fabricante: "",
  modelo: "",
  nome_comercial: "",
  sistema: "android" as SistemaOperacional,
  versao_sistema: "",
  imei1: "",
  imei2: "",
  serial: "",
  numero: "",
  operadora: "",
  capacidade: "",
  observacoes: "",
};

type Props = {
  salvar: (d: NovoDispositivo) => Promise<Dispositivo>;
  aoConcluir: (d: Dispositivo) => void;
  aoCancelar: () => void;
};

export function FormDispositivo({ salvar, aoConcluir, aoCancelar }: Props) {
  const [form, setForm] = useState(vazio);
  const [falha, setFalha] = useState<FalhaSied | null>(null);
  const [erroCapacidade, setErroCapacidade] = useState<string>();
  const [salvando, setSalvando] = useState(false);

  const campo = (nome: Exclude<keyof typeof vazio, "sistema">) => ({
    name: nome,
    value: form[nome],
    onChange: (e: { target: { value: string } }) => setForm((f) => ({ ...f, [nome]: e.target.value })),
    erro: falha?.doCampo(nome),
  });

  async function enviar(e: FormEvent) {
    e.preventDefault();
    const capacidade = lerCapacidade(form.capacidade);
    if (Number.isNaN(capacidade)) {
      setErroCapacidade("use um número, ex.: 128 GB");
      return;
    }
    setErroCapacidade(undefined);
    setSalvando(true);
    try {
      const d = await salvar({
        apelido: form.apelido,
        fabricante: ouNulo(form.fabricante),
        modelo: ouNulo(form.modelo),
        nome_comercial: ouNulo(form.nome_comercial),
        sistema: form.sistema,
        versao_sistema: ouNulo(form.versao_sistema),
        imei1: ouNulo(form.imei1),
        imei2: ouNulo(form.imei2),
        serial: ouNulo(form.serial),
        numero: ouNulo(form.numero),
        operadora: ouNulo(form.operadora),
        capacidade_bytes: capacidade,
        observacoes: ouNulo(form.observacoes),
      });
      setFalha(null);
      aoConcluir(d);
    } catch (err) {
      setFalha(err instanceof FalhaSied ? err : new FalhaSied({ codigo: "interno", mensagem: String(err), campos: [] }));
    } finally {
      setSalvando(false);
    }
  }

  return (
    <form onSubmit={enviar} noValidate aria-label="Cadastrar dispositivo" className="space-y-4">
      {falha && falha.erro.codigo !== "validacao" && <Aviso>{falha.message}</Aviso>}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
        <Campo rotulo="Identificação" obrigatorio placeholder="Ex.: Celular 1 – investigado" {...campo("apelido")} />
        <Campo rotulo="Fabricante" {...campo("fabricante")} />
        <Campo rotulo="Modelo" placeholder="Ex.: SM-A546E" {...campo("modelo")} />
        <Campo rotulo="Nome comercial" placeholder="Ex.: Galaxy A54" {...campo("nome_comercial")} />
        <Selecao
          rotulo="Sistema"
          name="sistema"
          value={form.sistema}
          onChange={(e) => setForm((f) => ({ ...f, sistema: e.target.value as SistemaOperacional }))}
        >
          <option value="android">Android</option>
          <option value="ios">iOS</option>
          <option value="outro">Outro</option>
          <option value="desconhecido">Desconhecido</option>
        </Selecao>
        <Campo rotulo="Versão do sistema" {...campo("versao_sistema")} />
        <Campo rotulo="IMEI 1" inputMode="numeric" {...campo("imei1")} />
        <Campo rotulo="IMEI 2" inputMode="numeric" {...campo("imei2")} />
        <Campo rotulo="Número de série" {...campo("serial")} />
        <Campo rotulo="Número da linha" {...campo("numero")} />
        <Campo rotulo="Operadora" {...campo("operadora")} />
        <Campo rotulo="Capacidade" placeholder="Ex.: 128 GB" {...campo("capacidade")} erro={erroCapacidade} />
        <div className="md:col-span-3">
          <AreaTexto rotulo="Observações" placeholder="Estado do aparelho, lacre, autorização..." {...campo("observacoes")} />
        </div>
      </div>
      <div className="flex justify-end gap-2">
        <Botao variante="fantasma" onClick={aoCancelar}>
          Cancelar
        </Botao>
        <Botao type="submit" carregando={salvando}>
          Salvar dispositivo
        </Botao>
      </div>
    </form>
  );
}
