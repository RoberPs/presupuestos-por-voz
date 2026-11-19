import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { Link, RouterProvider, createMemoryRouter } from "react-router-dom";
import { providerFuture, routerFuture } from "../../app/routes";
import { UnsavedChangesGuard } from "./UnsavedChangesGuard";

function Editor({ onSave }: { onSave: () => Promise<boolean> }) {
  const [dirty, setDirty] = useState(false);
  return (
    <div>
      <h1>Editor</h1>
      <button onClick={() => setDirty(true)}>Cambiar</button>
      <Link to="/otra">Ir a otra</Link>
      <UnsavedChangesGuard when={dirty} onSave={onSave} />
    </div>
  );
}

function setup(onSave: () => Promise<boolean> = async () => true) {
  const router = createMemoryRouter(
    [
      { path: "/", element: <Editor onSave={onSave} /> },
      { path: "/otra", element: <h1>Otra</h1> },
    ],
    { initialEntries: ["/"], future: routerFuture },
  );
  render(<RouterProvider router={router} future={providerFuture} />);
  return router;
}

describe("UnsavedChangesGuard", () => {
  it("sin cambios navega sin preguntar", async () => {
    const user = userEvent.setup();
    setup();
    await user.click(screen.getByRole("link", { name: "Ir a otra" }));
    expect(await screen.findByRole("heading", { name: "Otra" })).toBeInTheDocument();
  });

  it("con cambios pregunta; Cancelar se queda y Descartar navega", async () => {
    const user = userEvent.setup();
    setup();
    await user.click(screen.getByRole("button", { name: "Cambiar" }));
    await user.click(screen.getByRole("link", { name: "Ir a otra" }));
    await user.click(await screen.findByRole("button", { name: "Cancelar" }));
    expect(screen.getByRole("heading", { name: "Editor" })).toBeInTheDocument();

    await user.click(screen.getByRole("link", { name: "Ir a otra" }));
    await user.click(await screen.findByRole("button", { name: "Descartar" }));
    expect(await screen.findByRole("heading", { name: "Otra" })).toBeInTheDocument();
  });

  it("Guardar llama a onSave y navega; si no se guarda, se queda", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn().mockResolvedValueOnce(false).mockResolvedValueOnce(true);
    setup(onSave);
    await user.click(screen.getByRole("button", { name: "Cambiar" }));

    await user.click(screen.getByRole("link", { name: "Ir a otra" }));
    await user.click(await screen.findByRole("button", { name: "Guardar" }));
    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(1));
    expect(screen.getByRole("heading", { name: "Editor" })).toBeInTheDocument();

    await user.click(screen.getByRole("link", { name: "Ir a otra" }));
    await user.click(await screen.findByRole("button", { name: "Guardar" }));
    expect(await screen.findByRole("heading", { name: "Otra" })).toBeInTheDocument();
    expect(onSave).toHaveBeenCalledTimes(2);
  });
});
