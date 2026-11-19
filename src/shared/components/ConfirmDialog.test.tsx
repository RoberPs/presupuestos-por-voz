import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { ConfirmDialog } from "./ConfirmDialog";

function Harness({ onConfirm = () => {} }: { onConfirm?: () => void }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button onClick={() => setOpen(true)}>Abrir</button>
      <ConfirmDialog
        open={open}
        title="¿Seguro?"
        message="Mensaje"
        onDismiss={() => setOpen(false)}
        actions={[
          { label: "Cancelar", onClick: () => setOpen(false) },
          { label: "Aceptar", onClick: onConfirm, variant: "primary" },
        ]}
      />
    </>
  );
}

describe("ConfirmDialog (accesibilidad)", () => {
  it("es un diálogo modal con nombre y descripción", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "Abrir" }));
    const dialog = screen.getByRole("dialog", { name: "¿Seguro?" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveAccessibleDescription("Mensaje");
  });

  it("Esc lo cierra y devuelve el foco al botón que lo abrió", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const opener = screen.getByRole("button", { name: "Abrir" });
    await user.click(opener);
    expect(screen.getByRole("button", { name: "Cancelar" })).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(opener).toHaveFocus();
  });

  it("el tabulador no sale del diálogo", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("button", { name: "Abrir" }));
    const cancel = screen.getByRole("button", { name: "Cancelar" });
    const accept = screen.getByRole("button", { name: "Aceptar" });
    await user.tab();
    expect(accept).toHaveFocus();
    await user.tab();
    expect(cancel).toHaveFocus();
    await user.tab({ shift: true });
    expect(accept).toHaveFocus();
  });
});
