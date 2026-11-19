import { screen, within } from "@testing-library/react";
import { renderApp } from "../../test/renderApp";
import {
  draftFixture,
  lineFixture,
  mockDraftBackend,
  pricePendingFixture,
  rateFixture,
  totalsFixture,
  transcribedSession,
} from "../../test/draftBackend";
import type { DraftView } from "../../shared/ipc/types";

const renderDraft = (draft: DraftView, overrides = {}) => {
  mockDraftBackend(transcribedSession({ draft }), overrides);
  renderApp("/presupuesto");
};

const rowFor = async (concept: string) =>
  (await screen.findByDisplayValue(concept)).closest("tr") as HTMLElement;

describe("DraftLinesTable (US1)", () => {
  it("muestra las líneas completas con sus importes y los totales", async () => {
    renderDraft(draftFixture());
    const row = await rowFor("Bajante");
    expect(within(row).getByDisplayValue("3")).toBeInTheDocument();
    expect(within(row).getByDisplayValue("metro")).toBeInTheDocument();
    expect(within(row).getByDisplayValue("25,00")).toBeInTheDocument();
    expect(within(row).getByText("75,00 €")).toBeInTheDocument();
    expect(screen.getByText("135,00 €")).toBeInTheDocument();
    expect(screen.getByText("IVA (21,00 %)")).toBeInTheDocument();
    expect(screen.getByText("28,35 €")).toBeInTheDocument();
    expect(screen.getByText("163,35 €")).toBeInTheDocument();
  });

  it("muestra el texto transcrito junto al borrador", async () => {
    renderDraft(draftFixture());
    expect(
      await screen.findByDisplayValue("cambiar bajante, 3 metros, mano de obra 2 horas"),
    ).toBeInTheDocument();
  });

  it("marca el precio pendiente sin mostrar precio ni importe", async () => {
    renderDraft(
      draftFixture({
        lines: [pricePendingFixture()],
        totals: totalsFixture({ pendingCount: 1, isProvisional: true }),
      }),
    );
    const row = await rowFor("cambiar grifo");
    expect(within(row).getByText("Precio pendiente")).toBeInTheDocument();
    expect(within(row).getByText("No hay ninguna tarifa para este concepto.")).toBeInTheDocument();
    expect(within(row).queryByText(/€/)).not.toBeInTheDocument();
  });

  it("marca la cantidad pendiente y enseña lo dictado", async () => {
    renderDraft(
      draftFixture({
        lines: [
          lineFixture({
            quantity: null,
            quantityIssue: "rectified",
            amountCents: null,
            amountDisplay: null,
            sourceFragment: "bajante, 3 metros, no, mejor 4",
          }),
        ],
        totals: totalsFixture({ pendingCount: 1, isProvisional: true }),
      }),
    );
    const row = await rowFor("Bajante");
    expect(within(row).getByText("Cantidad pendiente")).toBeInTheDocument();
    expect(
      within(row).getByText("Te has corregido al dictar. Escribe la cantidad correcta."),
    ).toBeInTheDocument();
    expect(
      within(row).getByText("Has dicho: «bajante, 3 metros, no, mejor 4»"),
    ).toBeInTheDocument();
  });

  it("ofrece las tarifas sugeridas con su precio", async () => {
    renderDraft(
      draftFixture({
        lines: [
          pricePendingFixture({
            concept: "tubo bajante",
            pendingReason: "similarRate",
            suggestions: [rateFixture()],
          }),
        ],
      }),
    );
    const row = await rowFor("tubo bajante");
    expect(within(row).getByText("Tarifas que pueden encajar:")).toBeInTheDocument();
    expect(within(row).getByText(/Bajante de hierro/)).toBeInTheDocument();
    expect(within(row).getByText(/40,00 €/)).toBeInTheDocument();
  });

  it("muestra aparte lo que no se ha entendido", async () => {
    renderDraft(draftFixture({ unparsedFragments: ["12 metros", "bueno"] }));
    expect(
      await screen.findByRole("heading", { name: "Partes del texto que no se han entendido" }),
    ).toBeInTheDocument();
    expect(screen.getByText("12 metros")).toBeInTheDocument();
    expect(screen.getByText("bueno")).toBeInTheDocument();
  });

  it("indica que el total es provisional y cuántas líneas faltan", async () => {
    renderDraft(
      draftFixture({
        lines: [lineFixture(), pricePendingFixture(), pricePendingFixture({ id: 3 })],
        totals: totalsFixture({ pendingCount: 2, isProvisional: true }),
      }),
    );
    expect(
      await screen.findByText("Total provisional: faltan 2 líneas por completar."),
    ).toBeInTheDocument();
  });

  it("sin tarifas ofrece un acceso para crearlas", async () => {
    renderDraft(draftFixture({ lines: [pricePendingFixture()] }), { rates_list: () => [] });
    const link = await screen.findByRole("link", { name: "Crear tarifas" });
    expect(link).toHaveAttribute("href", "/tarifas");
  });

  it("con tarifas no muestra ese acceso", async () => {
    renderDraft(draftFixture());
    await rowFor("Bajante");
    expect(screen.queryByRole("link", { name: "Crear tarifas" })).not.toBeInTheDocument();
  });
});
