import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { es } from "../../shared/i18n/es";
import type { QuotePreparationView } from "../../shared/ipc/types";
import { mockQuoteBackend, preparationFixture } from "../../test/quoteBackend";
import { QuotePdfDialog } from "./QuotePdfDialog";

const t = es.quotePdf;

const blocked = (over: Partial<QuotePreparationView>) =>
  preparationFixture({ confirmationToken: null, ...over });

const renderDialog = (onClose = vi.fn()) => {
  render(
    <MemoryRouter initialEntries={["/presupuesto"]}>
      <Routes>
        <Route path="/presupuesto" element={<QuotePdfDialog onClose={onClose} />} />
        <Route path="/negocio" element={<p>Pantalla de mi negocio</p>} />
      </Routes>
    </MemoryRouter>,
  );
  return onClose;
};

describe("Bloqueos y avisos del PDF (US2)", () => {
  it("enumera las líneas pendientes y permite ir a completarlas", async () => {
    const user = userEvent.setup();
    mockQuoteBackend({
      quote_prepare: () =>
        blocked({
          blockers: [
            {
              kind: "pendingLines",
              lines: [
                { id: 1, concept: "Cambiar grifo" },
                { id: 2, concept: "Bajante" },
              ],
            },
          ],
        }),
    });
    const onClose = renderDialog();

    expect(await screen.findByText("Cambiar grifo")).toBeInTheDocument();
    expect(screen.getByText("Bajante")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: t.blockers.goToLines }));
    expect(onClose).toHaveBeenCalled();
  });

  it("pide revisar un borrador sin dar por bueno", async () => {
    mockQuoteBackend({
      quote_prepare: () => blocked({ blockers: [{ kind: "notReviewed" }] }),
    });
    renderDialog();
    expect(await screen.findByText(t.blockers.notReviewed)).toBeInTheDocument();
  });

  it("indica los datos que faltan y lleva a Mi negocio", async () => {
    const user = userEvent.setup();
    mockQuoteBackend({
      quote_prepare: () =>
        blocked({
          blockers: [
            {
              kind: "profileIncomplete",
              missingFields: [
                { field: "taxId", code: "REQUIRED", message: "Este dato es obligatorio." },
              ],
            },
          ],
        }),
    });
    renderDialog();

    expect(await screen.findByText(t.blockers.profileIncomplete)).toBeInTheDocument();
    expect(screen.getByText(es.business.fields.taxId)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: t.blockers.goToBusiness }));
    expect(await screen.findByText("Pantalla de mi negocio")).toBeInTheDocument();
  });

  it("muestra el dato y el carácter que no se pueden imprimir", async () => {
    mockQuoteBackend({
      quote_prepare: () =>
        blocked({
          blockers: [
            {
              kind: "unsupportedCharacters",
              fields: [{ field: "clientName", label: "Nombre del cliente", characters: ["😀"] }],
            },
          ],
        }),
    });
    renderDialog();
    expect(
      await screen.findByText(t.blockers.unsupportedField("Nombre del cliente", "😀")),
    ).toBeInTheDocument();
  });

  it("muestra los avisos sin impedir generar", async () => {
    const user = userEvent.setup();
    const warned = preparationFixture({
      warnings: [
        { kind: "noLogo" },
        { kind: "vatChanged", appliedRatePercent: "21", currentRatePercent: "10" },
        { kind: "unparsedFragments", count: 2 },
      ],
    });
    const backend = mockQuoteBackend({
      quote_prepare: () => warned,
      quote_details_save: () => warned,
    });
    renderDialog();

    expect(await screen.findByText(t.warnings.noLogo)).toBeInTheDocument();
    expect(screen.getByText(t.warnings.vatChanged("21", "10"))).toBeInTheDocument();
    expect(screen.getByText(t.warnings.unparsedFragments(2))).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: t.confirm }));
    await waitFor(() => expect(backend.called("quote_generate")).toHaveLength(1));
  });

  it("actualizar el IVA vuelve a preparar el presupuesto", async () => {
    const user = userEvent.setup();
    let applied = false;
    const backend = mockQuoteBackend({
      quote_prepare: () =>
        applied
          ? blocked({ blockers: [{ kind: "notReviewed" }] })
          : preparationFixture({
              warnings: [
                { kind: "vatChanged", appliedRatePercent: "21", currentRatePercent: "10" },
              ],
            }),
      draft_apply_current_vat: () => {
        applied = true;
        return null;
      },
    });
    renderDialog();

    await user.click(await screen.findByRole("button", { name: t.warnings.applyVat }));

    expect(backend.called("draft_apply_current_vat")).toHaveLength(1);
    expect(await screen.findByText(t.blockers.notReviewed)).toBeInTheDocument();
  });

  it("permite seguir sin un logo que no se puede leer", async () => {
    const user = userEvent.setup();
    const backend = mockQuoteBackend({
      quote_prepare: (args) =>
        args.allowMissingLogo
          ? preparationFixture({ warnings: [{ kind: "logoUnreadable" }] })
          : blocked({ blockers: [{ kind: "logoUnreadable" }] }),
    });
    renderDialog();

    expect(await screen.findByText(t.blockers.logoUnreadable)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: t.blockers.continueWithoutLogo }));

    expect(await screen.findByText(t.warnings.logoUnreadable)).toBeInTheDocument();
    expect(backend.called("quote_prepare").at(-1)?.args).toEqual({ allowMissingLogo: true });

    await user.click(screen.getByRole("button", { name: t.confirm }));
    await waitFor(() =>
      expect(backend.called("quote_generate")[0]?.args.allowMissingLogo).toBe(true),
    );
  });
});
