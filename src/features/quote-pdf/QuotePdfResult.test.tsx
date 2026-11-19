import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { es } from "../../shared/i18n/es";
import type { PdfSaveCode } from "../../shared/ipc/types";
import { mockQuoteBackend } from "../../test/quoteBackend";
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

const confirm = async (user: ReturnType<typeof userEvent.setup>) =>
  user.click(await screen.findByRole("button", { name: t.confirm }));

const CODES: [PdfSaveCode, string][] = [
  ["PDF_ACCESS_DENIED", "No tienes permiso para guardar en esa carpeta."],
  ["PDF_DISK_FULL", "No queda espacio suficiente."],
  ["PDF_LOCATION_UNAVAILABLE", "Esa carpeta ya no está disponible."],
  ["PDF_FILE_IN_USE", "El archivo está abierto en otro programa."],
  ["PDF_WRITE_FAILED", "No se ha podido guardar el PDF."],
];

describe("Resultado del guardado (US3)", () => {
  it("muestra el nombre y la ubicación completa del archivo", async () => {
    const user = userEvent.setup();
    mockQuoteBackend();
    const onClose = renderDialog();

    await confirm(user);

    expect(await screen.findByText(t.result.saved)).toBeInTheDocument();
    expect(screen.getByText("Presupuesto 2026-001 2026-11-18.pdf")).toBeInTheDocument();
    expect(
      screen.getByText("C:\\Presupuestos\\Presupuesto 2026-001 2026-11-18.pdf"),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: t.result.close }));
    expect(onClose).toHaveBeenCalled();
  });

  it("cancelar el diálogo vuelve a la confirmación sin error", async () => {
    const user = userEvent.setup();
    mockQuoteBackend({ quote_generate: () => ({ outcome: "cancelled" }) });
    renderDialog();

    await confirm(user);

    expect(await screen.findByRole("button", { name: t.confirm })).toBeEnabled();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.queryByText(t.result.saved)).not.toBeInTheDocument();
  });

  it.each(CODES)("%s muestra su mensaje y permite reintentar", async (code, message) => {
    const user = userEvent.setup();
    let attempts = 0;
    const backend = mockQuoteBackend({
      quote_generate: () => {
        attempts += 1;
        return attempts === 1
          ? Promise.reject({ kind: "pdfSave", code, message })
          : {
              outcome: "saved",
              fileName: "Presupuesto.pdf",
              fullPath: "D:\\Otra\\Presupuesto.pdf",
              pageCount: 1,
            };
      },
    });
    renderDialog();

    await confirm(user);
    expect(await screen.findByText(message)).toBeInTheDocument();
    const saves = backend.called("quote_details_save").length;

    await user.click(screen.getByRole("button", { name: t.result.retry }));

    expect(await screen.findByText(t.result.saved)).toBeInTheDocument();
    expect(screen.getByText("D:\\Otra\\Presupuesto.pdf")).toBeInTheDocument();
    expect(backend.called("quote_details_save")).toHaveLength(saves);
    await waitFor(() => expect(backend.called("quote_generate")).toHaveLength(2));
    expect(backend.called("quote_generate")[1].args.confirmationToken).toBe(
      backend.called("quote_generate")[0].args.confirmationToken,
    );
  });

  it("si el presupuesto ha cambiado, avisa y vuelve a prepararlo", async () => {
    const user = userEvent.setup();
    const backend = mockQuoteBackend({
      quote_generate: () =>
        Promise.reject({ kind: "quoteChanged", message: t.changed }),
    });
    renderDialog();

    await confirm(user);

    expect(await screen.findByText(t.changed)).toBeInTheDocument();
    await waitFor(() => expect(backend.called("quote_prepare")).toHaveLength(2));
    expect(screen.queryByRole("button", { name: t.result.retry })).not.toBeInTheDocument();
  });
});
