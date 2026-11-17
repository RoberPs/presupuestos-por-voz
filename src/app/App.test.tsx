import { screen } from "@testing-library/react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { renderApp } from "../test/renderApp";

describe("App", () => {
  it("muestra la navegación con las dos secciones", () => {
    mockIPC(() => null);
    renderApp("/negocio");
    expect(screen.getByRole("link", { name: "Mi negocio" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Tarifas" })).toBeInTheDocument();
  });
});
