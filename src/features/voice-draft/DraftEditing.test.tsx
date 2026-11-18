import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderApp } from "../../test/renderApp";
import {
  draftFixture,
  lineFixture,
  mockDraftBackend,
  pricePendingFixture,
  rateFixture,
  sessionFixture,
  totalsFixture,
  transcribedSession,
} from "../../test/draftBackend";
import type { AppError, DraftView } from "../../shared/ipc/types";

const edited = draftFixture({
  hasManualChanges: true,
  lines: [
    lineFixture({ quantity: "4", amountCents: 10000, amountDisplay: "100,00 €" }),
    draftFixture().lines[1],
  ],
  totals: totalsFixture({
    baseDisplay: "160,00 €",
    vatDisplay: "33,60 €",
    totalDisplay: "193,60 €",
  }),
});

const renderDraft = (draft: DraftView, overrides = {}) => {
  const backend = mockDraftBackend(transcribedSession({ draft }), overrides);
  renderApp("/presupuesto");
  return backend;
};

const rowFor = async (concept: string) =>
  (await screen.findByDisplayValue(concept)).closest("tr") as HTMLElement;

describe("Edición del borrador (US2)", () => {
  it("cambiar una cantidad muestra los importes que devuelve el núcleo", async () => {
    const user = userEvent.setup();
    const backend = renderDraft(draftFixture(), { draft_line_update: () => edited });
    const row = await rowFor("Bajante");

    const quantity = within(row).getByLabelText("Cantidad de Bajante");
    await user.clear(quantity);
    await user.type(quantity, "4");
    await user.tab();

    await waitFor(() =>
      expect(backend.called("draft_line_update")[0].args).toEqual({
        lineId: 1,
        patch: { quantity: "4" },
      }),
    );
    expect(await screen.findByText("100,00 €")).toBeInTheDocument();
    expect(screen.getByText("160,00 €")).toBeInTheDocument();
    expect(screen.getByText("33,60 €")).toBeInTheDocument();
    expect(screen.getByText("193,60 €")).toBeInTheDocument();
  });

  it("no llama al núcleo si el valor no ha cambiado", async () => {
    const user = userEvent.setup();
    const backend = renderDraft(draftFixture());
    const row = await rowFor("Bajante");
    await user.click(within(row).getByLabelText("Cantidad de Bajante"));
    await user.tab();
    expect(backend.called("draft_line_update")).toHaveLength(0);
  });

  it("muestra los errores junto al campo", async () => {
    const user = userEvent.setup();
    const error: AppError = {
      kind: "validation",
      errors: [
        {
          field: "quantity",
          code: "QUANTITY_NOT_POSITIVE",
          message: "La cantidad debe ser mayor que cero.",
        },
      ],
    };
    renderDraft(draftFixture(), {
      draft_line_update: () => {
        throw error;
      },
    });
    const row = await rowFor("Bajante");
    const quantity = within(row).getByLabelText("Cantidad de Bajante");
    await user.clear(quantity);
    await user.type(quantity, "0");
    await user.tab();

    expect(await within(row).findByText("La cantidad debe ser mayor que cero.")).toBeVisible();
    expect(quantity).toHaveAttribute("aria-invalid", "true");
  });

  it("muestra de qué tarifa procede el precio y si se ha escrito a mano", async () => {
    renderDraft(
      draftFixture({
        lines: [
          lineFixture(),
          lineFixture({
            id: 2,
            concept: "Desplazamiento",
            priceSource: "manual",
            rateId: null,
            rateConcept: null,
          }),
        ],
      }),
    );
    expect(within(await rowFor("Bajante")).getByText("Tarifa: Bajante")).toBeInTheDocument();
    expect(
      within(await rowFor("Desplazamiento")).getByText("Precio escrito a mano"),
    ).toBeInTheDocument();
  });

  it("elegir una sugerencia completa la línea", async () => {
    const user = userEvent.setup();
    const backend = renderDraft(
      draftFixture({
        lines: [pricePendingFixture({ suggestions: [rateFixture()] })],
      }),
      { draft_line_assign_rate: () => draftFixture() },
    );
    await user.click(
      await screen.findByRole("button", { name: "Usar Bajante de hierro (40,00 € por metro)" }),
    );
    await waitFor(() =>
      expect(backend.called("draft_line_assign_rate")[0].args).toEqual({ lineId: 2, rateId: 7 }),
    );
  });

  it("añadir una línea desde una tarifa", async () => {
    const user = userEvent.setup();
    const backend = renderDraft(draftFixture(), { draft_line_add: () => edited });
    await user.click(await screen.findByRole("button", { name: "Añadir línea" }));
    const dialog = await screen.findByRole("dialog", { name: "Añadir línea" });

    await user.selectOptions(within(dialog).getByLabelText(/Tarifa/), "7");
    await user.type(within(dialog).getByLabelText(/Cantidad/), "2");
    await user.click(within(dialog).getByRole("button", { name: "Añadir" }));

    await waitFor(() =>
      expect(backend.called("draft_line_add")[0].args).toEqual({
        input: { kind: "rate", rateId: 7, quantity: "2" },
      }),
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("añadir una línea escrita a mano y ver sus errores", async () => {
    const user = userEvent.setup();
    const error: AppError = {
      kind: "validation",
      errors: [{ field: "price", code: "PRICE_REQUIRED", message: "Escribe el precio." }],
    };
    const backend = renderDraft(draftFixture(), {
      draft_line_add: () => {
        throw error;
      },
    });
    await user.click(await screen.findByRole("button", { name: "Añadir línea" }));
    const dialog = await screen.findByRole("dialog", { name: "Añadir línea" });

    await user.click(within(dialog).getByRole("radio", { name: "Escribir a mano" }));
    await user.type(within(dialog).getByLabelText(/Concepto/), "Desplazamiento");
    await user.type(within(dialog).getByLabelText(/Cantidad/), "1");
    await user.type(within(dialog).getByLabelText(/Unidad/), "unidad");
    await user.click(within(dialog).getByRole("button", { name: "Añadir" }));

    expect(await within(dialog).findByText("Escribe el precio.")).toBeInTheDocument();
    expect(backend.called("draft_line_add")[0].args).toEqual({
      input: {
        kind: "manual",
        concept: "Desplazamiento",
        quantity: "1",
        unitLabel: "unidad",
        price: "",
      },
    });
  });

  it("sin elegir tarifa pide que se elija una", async () => {
    const user = userEvent.setup();
    const backend = renderDraft(draftFixture());
    await user.click(await screen.findByRole("button", { name: "Añadir línea" }));
    const dialog = await screen.findByRole("dialog", { name: "Añadir línea" });
    await user.click(within(dialog).getByRole("button", { name: "Añadir" }));
    expect(await within(dialog).findByText("Elige una tarifa.")).toBeInTheDocument();
    expect(backend.called("draft_line_add")).toHaveLength(0);
  });

  it("borrar una línea pide confirmación", async () => {
    const user = userEvent.setup();
    const backend = renderDraft(draftFixture(), { draft_line_delete: () => edited });
    const row = await rowFor("Bajante");

    await user.click(within(row).getByRole("button", { name: "Borrar Bajante" }));
    let dialog = await screen.findByRole("dialog", { name: "¿Borrar la línea?" });
    await user.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(backend.called("draft_line_delete")).toHaveLength(0);

    await user.click(within(row).getByRole("button", { name: "Borrar Bajante" }));
    dialog = await screen.findByRole("dialog", { name: "¿Borrar la línea?" });
    await user.click(within(dialog).getByRole("button", { name: "Borrar" }));
    await waitFor(() => expect(backend.called("draft_line_delete")[0].args).toEqual({ lineId: 1 }));
  });

  it("dar por bueno un borrador completo lo indica como revisado", async () => {
    const user = userEvent.setup();
    renderDraft(draftFixture(), {
      draft_mark_reviewed: () => draftFixture({ status: "reviewed" }),
    });
    await user.click(await screen.findByRole("button", { name: "Dar por bueno" }));
    expect(await screen.findByText("Presupuesto revisado")).toBeInTheDocument();
  });

  it("con líneas pendientes señala las que faltan", async () => {
    const user = userEvent.setup();
    const error: AppError = {
      kind: "draftHasPending",
      pendingLineIds: [2],
      message: "Faltan líneas por completar.",
    };
    renderDraft(
      draftFixture({
        lines: [lineFixture(), pricePendingFixture()],
        totals: totalsFixture({ pendingCount: 1, isProvisional: true }),
      }),
      {
        draft_mark_reviewed: () => {
          throw error;
        },
      },
    );
    await user.click(await screen.findByRole("button", { name: "Dar por bueno" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Completa las líneas marcadas antes de darlo por bueno.",
    );
    const row = await rowFor("cambiar grifo");
    expect(within(row).getByText("Falta completar esta línea")).toBeInTheDocument();
    expect(
      within(await rowFor("Bajante")).queryByText("Falta completar esta línea"),
    ).not.toBeInTheDocument();
  });

  it("avisa del cambio de IVA y permite actualizarlo", async () => {
    const user = userEvent.setup();
    const backend = renderDraft(draftFixture({ vatChange: { currentRatePercent: "10,00" } }), {
      draft_apply_current_vat: () => draftFixture(),
    });
    expect(
      await screen.findByText("El IVA de tu negocio ha cambiado al 10,00 %."),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Actualizar el IVA" }));
    await waitFor(() => expect(backend.called("draft_apply_current_vat")).toHaveLength(1));
    await waitFor(() =>
      expect(
        screen.queryByText("El IVA de tu negocio ha cambiado al 10,00 %."),
      ).not.toBeInTheDocument(),
    );
  });

  it("empezar un presupuesto nuevo pide confirmación", async () => {
    const user = userEvent.setup();
    const backend = renderDraft(draftFixture(), { draft_discard: () => sessionFixture() });
    await user.click(await screen.findByRole("button", { name: "Nuevo presupuesto" }));
    const dialog = await screen.findByRole("dialog", {
      name: "¿Empezar un presupuesto nuevo?",
    });
    await user.click(within(dialog).getByRole("button", { name: "Borrar y empezar de nuevo" }));

    await waitFor(() => expect(backend.called("draft_discard")).toHaveLength(1));
    await waitFor(() =>
      expect(screen.queryByRole("heading", { name: "Líneas del presupuesto" })).toBeNull(),
    );
  });
});
