import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FalhaSied, type Dispositivo, type NovoDispositivo } from "../api/cliente";
import { FormDispositivo } from "./FormDispositivo";

describe("FormDispositivo", () => {
  it("envia os dados normalizados", async () => {
    const salvar = vi.fn(async (d: NovoDispositivo) => ({ ...d, id: "1" }) as unknown as Dispositivo);
    const aoConcluir = vi.fn();
    render(<FormDispositivo salvar={salvar} aoConcluir={aoConcluir} aoCancelar={() => {}} />);

    await userEvent.type(screen.getByLabelText(/Identificação/), "Celular 1");
    await userEvent.type(screen.getByLabelText("Capacidade"), "64");
    await userEvent.click(screen.getByRole("button", { name: "Salvar dispositivo" }));

    expect(salvar).toHaveBeenCalledOnce();
    const enviado = salvar.mock.calls[0]![0];
    expect(enviado.apelido).toBe("Celular 1");
    expect(enviado.sistema).toBe("android");
    expect(enviado.capacidade_bytes).toBe(64e9);
    expect(enviado.imei1).toBeNull();
    expect(aoConcluir).toHaveBeenCalledOnce();
  });

  it("mostra no campo o erro de validação do backend", async () => {
    const salvar = vi.fn(async () => {
      throw new FalhaSied({
        codigo: "validacao",
        mensagem: "corrija os campos destacados",
        campos: [{ campo: "imei1", mensagem: "IMEI inválido (dígito verificador não confere)" }],
      });
    });
    const aoConcluir = vi.fn();
    render(<FormDispositivo salvar={salvar} aoConcluir={aoConcluir} aoCancelar={() => {}} />);

    await userEvent.type(screen.getByLabelText(/Identificação/), "X");
    await userEvent.type(screen.getByLabelText("IMEI 1"), "490154203237519");
    await userEvent.click(screen.getByRole("button", { name: "Salvar dispositivo" }));

    expect(await screen.findByText(/dígito verificador/)).toBeInTheDocument();
    expect(screen.getByLabelText("IMEI 1")).toHaveAttribute("aria-invalid", "true");
    expect(aoConcluir).not.toHaveBeenCalled();
  });

  it("recusa capacidade inválida sem chamar o backend", async () => {
    const salvar = vi.fn();
    render(<FormDispositivo salvar={salvar} aoConcluir={() => {}} aoCancelar={() => {}} />);
    await userEvent.type(screen.getByLabelText(/Identificação/), "X");
    await userEvent.type(screen.getByLabelText("Capacidade"), "muito");
    await userEvent.click(screen.getByRole("button", { name: "Salvar dispositivo" }));
    expect(await screen.findByText(/use um número/)).toBeInTheDocument();
    expect(salvar).not.toHaveBeenCalled();
  });
});
