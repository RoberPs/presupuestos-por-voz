import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderApp } from "../../test/renderApp";
import { mockRatesBackend } from "../../test/ratesBackend";

const rowFor = (concept: string) => {
  const cell = screen.getByRole("cell", { name: concept });
  return cell.closest("tr") as HTMLElement;
};

describe("RatesPage (US2)", () => {
  it("lista vacía: muestra el estado de primer uso con acceso para crear la primera", async () => {
    mockRatesBackend();
    renderApp("/tarifas");
    expect(await screen.findByText("Aún no tienes tarifas")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Crear primera tarifa" })).toBeInTheDocument();
  });

  it("indica que los precios no incluyen IVA", async () => {
    mockRatesBackend();
    renderApp("/tarifas");
    expect(await screen.findByText("Los precios se guardan sin IVA")).toBeInTheDocument();
  });

  it("crear una tarifa la añade a la lista", async () => {
    const user = userEvent.setup();
    mockRatesBackend();
    renderApp("/tarifas");
    await user.click(await screen.findByRole("button", { name: "Crear primera tarifa" }));
    const dialog = await screen.findByRole("dialog", { name: "Nueva tarifa" });
    await user.type(within(dialog).getByLabelText(/Concepto/), "Instalación de grifo");
    await user.type(within(dialog).getByLabelText(/Unidad/), "unidad");
    await user.type(within(dialog).getByLabelText(/Precio/), "45,50");
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));

    expect(await screen.findByRole("cell", { name: "Instalación de grifo" })).toBeInTheDocument();
    expect(within(rowFor("Instalación de grifo")).getByText("45,50 €")).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("editar el precio muestra el precio nuevo", async () => {
    const user = userEvent.setup();
    mockRatesBackend([{ concept: "Pintura", unit: "hora", price: 25 }]);
    renderApp("/tarifas");
    await screen.findByRole("cell", { name: "Pintura" });
    await user.click(within(rowFor("Pintura")).getByRole("button", { name: "Editar Pintura" }));
    const dialog = await screen.findByRole("dialog", { name: "Editar tarifa" });
    const price = within(dialog).getByLabelText(/Precio/);
    expect(price).toHaveValue("25,00");
    await user.clear(price);
    await user.type(price, "30");
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));
    await waitFor(() => expect(within(rowFor("Pintura")).getByText("30,00 €")).toBeInTheDocument());
  });

  it("borrar: cancelar la mantiene y confirmar la quita", async () => {
    const user = userEvent.setup();
    mockRatesBackend([{ concept: "Pintura", unit: "m²", price: 8 }]);
    renderApp("/tarifas");
    await screen.findByRole("cell", { name: "Pintura" });

    await user.click(within(rowFor("Pintura")).getByRole("button", { name: "Borrar Pintura" }));
    let dialog = await screen.findByRole("dialog", { name: "¿Borrar la tarifa?" });
    await user.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(screen.getByRole("cell", { name: "Pintura" })).toBeInTheDocument();

    await user.click(within(rowFor("Pintura")).getByRole("button", { name: "Borrar Pintura" }));
    dialog = await screen.findByRole("dialog", { name: "¿Borrar la tarifa?" });
    await user.click(within(dialog).getByRole("button", { name: "Borrar" }));
    await waitFor(() =>
      expect(screen.queryByRole("cell", { name: "Pintura" })).not.toBeInTheDocument(),
    );
    expect(await screen.findByText("Aún no tienes tarifas")).toBeInTheDocument();
  });

  it("duplicado: muestra el mensaje y permite editar la tarifa existente", async () => {
    const user = userEvent.setup();
    mockRatesBackend([{ concept: "Instalación de grifo", unit: "unidad", price: 45 }]);
    renderApp("/tarifas");
    await screen.findByRole("cell", { name: "Instalación de grifo" });
    await user.click(screen.getByRole("button", { name: "Nueva tarifa" }));
    const dialog = await screen.findByRole("dialog", { name: "Nueva tarifa" });
    await user.type(within(dialog).getByLabelText(/Concepto/), "instalacion de GRIFO");
    await user.type(within(dialog).getByLabelText(/Unidad/), "unidad");
    await user.type(within(dialog).getByLabelText(/Precio/), "50");
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));

    expect(await within(dialog).findByRole("alert")).toHaveTextContent(/Ya existe la tarifa/);
    await user.click(within(dialog).getByRole("button", { name: "Editar la existente" }));
    const edit = await screen.findByRole("dialog", { name: "Editar tarifa" });
    expect(within(edit).getByLabelText(/Concepto/)).toHaveValue("Instalación de grifo");
  });
});
