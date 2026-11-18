import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { es } from "../../shared/i18n/es";
import type { QuoteDetailsInput } from "../../shared/ipc/types";
import { detailsFixture, mockQuoteBackend, preparationFixture } from "../../test/quoteBackend";
import { QuotePdfDialog } from "./QuotePdfDialog";

const t = es.quotePdf;

const renderDialog = (onClose = vi.fn()) => {
  render(
    <MemoryRouter>
      <QuotePdfDialog onClose={onClose} />
    </MemoryRouter>,
  );
  return onClose;
};

const confirmButton = () => screen.findByRole("button", { name: t.confirm });

describe("Confirmación del PDF (US1)", () => {
  it("muestra el número, las fechas, las líneas y el total", async () => {
    mockQuoteBackend();
    renderDialog();

    expect(await screen.findByLabelText(new RegExp(t.form.quoteNumber))).toHaveValue("2026-001");
    expect(screen.getByText("18/11/2026")).toBeInTheDocument();
    expect(screen.getByText("18/12/2026")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("163,35 €")).toBeInTheDocument();
  });

  it("cancelar no genera el PDF", async () => {
    const user = userEvent.setup();
    const backend = mockQuoteBackend();
    const onClose = renderDialog();

    await confirmButton();
    await user.click(screen.getByRole("button", { name: es.common.cancel }));

    expect(onClose).toHaveBeenCalled();
    expect(backend.called("quote_generate")).toHaveLength(0);
  });

  it("sin nombre del cliente no genera y muestra el motivo", async () => {
    const user = userEvent.setup();
    const blocked = preparationFixture({
      details: detailsFixture({ clientName: "" }),
      blockers: [{ kind: "clientNameRequired" }],
      confirmationToken: null,
    });
    const backend = mockQuoteBackend({
      quote_prepare: () => blocked,
      quote_details_save: () => blocked,
    });
    renderDialog();

    await user.click(await confirmButton());

    expect(await screen.findByText(t.blockers.clientNameRequired)).toBeInTheDocument();
    expect(backend.called("quote_generate")).toHaveLength(0);
  });

  it("corregir un dato lo guarda al salir del campo", async () => {
    const user = userEvent.setup();
    const backend = mockQuoteBackend();
    renderDialog();

    await user.type(await screen.findByLabelText(new RegExp(t.form.clientPhone)), "611222333");
    await user.tab();

    await waitFor(() => expect(backend.called("quote_details_save")).toHaveLength(1));
    const saved = backend.called("quote_details_save")[0].args.details as QuoteDetailsInput;
    expect(saved.clientPhone).toBe("611222333");
    expect(saved.clientName).toBe("Ana Ruiz");
  });

  it("un solo clic guarda lo escrito y genera con el token devuelto", async () => {
    const user = userEvent.setup();
    const backend = mockQuoteBackend();
    renderDialog();

    await user.type(await screen.findByLabelText(new RegExp(t.form.clientPhone)), "611222333");
    await user.click(await confirmButton());

    await waitFor(() => expect(backend.called("quote_generate")).toHaveLength(1));
    expect(backend.called("quote_generate")[0].args).toEqual({
      confirmationToken: "token-guardado",
      allowMissingLogo: false,
    });

    const order = backend.calls.map((call) => call.cmd);
    expect(order.lastIndexOf("quote_details_save")).toBeLessThan(order.indexOf("quote_generate"));
    const saved = backend.called("quote_details_save").at(-1)?.args.details as QuoteDetailsInput;
    expect(saved.clientPhone).toBe("611222333");
  });

  it("el botón solo se desactiva mientras se genera", async () => {
    const user = userEvent.setup();
    let finish: (value: unknown) => void = () => undefined;
    mockQuoteBackend({
      quote_generate: () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    });
    renderDialog();

    await user.type(await screen.findByLabelText(new RegExp(t.form.clientPhone)), "6");
    expect(await confirmButton()).toBeEnabled();

    await user.click(await confirmButton());

    expect(await screen.findByRole("button", { name: t.generating })).toBeDisabled();
    finish({ outcome: "cancelled" });
    expect(await confirmButton()).toBeEnabled();
  });
});
