import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderApp } from "../../test/renderApp";
import { mockRatesBackend } from "../../test/ratesBackend";

const seed = [
  { concept: "Instalación de grifo", unit: "unidad", price: 45 },
  { concept: "Cambio de grifo", unit: "unidad", price: 30 },
  { concept: "Pintura de pared", unit: "m²", price: 8 },
];

describe("Búsqueda de tarifas (US4)", () => {
  it("al escribir consulta rates_list con la búsqueda y muestra solo las coincidentes", async () => {
    const user = userEvent.setup();
    const backend = mockRatesBackend(seed);
    renderApp("/tarifas");
    await screen.findByRole("cell", { name: "Pintura de pared" });

    await user.type(screen.getByRole("searchbox", { name: "Buscar tarifa" }), "instalacion");
    await waitFor(() =>
      expect(screen.queryByRole("cell", { name: "Pintura de pared" })).not.toBeInTheDocument(),
    );
    expect(screen.getByRole("cell", { name: "Instalación de grifo" })).toBeInTheDocument();
    expect(screen.queryByRole("cell", { name: "Cambio de grifo" })).not.toBeInTheDocument();
    expect(
      backend.calls.some((c) => c.cmd === "rates_list" && c.args.query === "instalacion"),
    ).toBe(true);
  });

  it("sin resultados lo indica y ofrece crear la tarifa con el concepto precargado", async () => {
    const user = userEvent.setup();
    mockRatesBackend(seed);
    renderApp("/tarifas");
    await screen.findByRole("cell", { name: "Pintura de pared" });

    await user.type(screen.getByRole("searchbox", { name: "Buscar tarifa" }), "zzz");
    expect(await screen.findByText("No hay tarifas que coincidan con «zzz»")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Crear tarifa «zzz»" }));
    const dialog = await screen.findByRole("dialog", { name: "Nueva tarifa" });
    expect(within(dialog).getByLabelText(/Concepto/)).toHaveValue("zzz");
  });

  it("vaciar el buscador restaura la lista completa", async () => {
    const user = userEvent.setup();
    mockRatesBackend(seed);
    renderApp("/tarifas");
    await screen.findByRole("cell", { name: "Pintura de pared" });
    const search = screen.getByRole("searchbox", { name: "Buscar tarifa" });

    await user.type(search, "grifo");
    await waitFor(() =>
      expect(screen.queryByRole("cell", { name: "Pintura de pared" })).not.toBeInTheDocument(),
    );
    await user.click(screen.getByRole("button", { name: "Borrar búsqueda" }));
    expect(await screen.findByRole("cell", { name: "Pintura de pared" })).toBeInTheDocument();
    expect(screen.getAllByRole("row")).toHaveLength(4); // cabecera + 3
    expect(search).toHaveValue("");
  });

  it("lista vacía sin búsqueda: estado de primer uso, no de 'sin resultados'", async () => {
    mockRatesBackend();
    renderApp("/tarifas");
    expect(await screen.findByText("Aún no tienes tarifas")).toBeInTheDocument();
    expect(screen.queryByText(/No hay tarifas que coincidan/)).not.toBeInTheDocument();
  });
});
