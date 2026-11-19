import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderApp } from "../../test/renderApp";
import {
  draftFixture,
  lineFixture,
  mockDraftBackend,
  sessionFixture,
  transcribedSession,
} from "../../test/draftBackend";
import type { AppError } from "../../shared/ipc/types";

const LABEL = "Lo que has contado en la visita";

const confirmationError = (reason: "discardManualChanges" | "replaceDraft"): AppError => ({
  kind: "confirmationRequired",
  reason,
  message: "Si sigues, se perderán.",
});

describe("VisitTextEditor (US4)", () => {
  it("guarda el texto poco después de dejar de escribir", async () => {
    const backend = mockDraftBackend(transcribedSession());
    renderApp("/presupuesto");

    // Cambios seguidos, sin depender de lo rápido que vaya el equipo al simular el teclado.
    const text = await screen.findByLabelText(LABEL);
    fireEvent.change(text, { target: { value: "bajante" } });
    fireEvent.change(text, { target: { value: "bajante 5" } });
    fireEvent.change(text, { target: { value: "bajante 5 metros" } });
    expect(backend.called("draft_text_save")).toHaveLength(0);

    await waitFor(
      () =>
        expect(backend.called("draft_text_save").at(-1)?.args).toEqual({
          text: "bajante 5 metros",
        }),
      { timeout: 2000 },
    );
    // El guardado espera a que se deje de escribir: no se guarda cada cambio.
    expect(backend.called("draft_text_save")).toHaveLength(1);
  });

  it("volver a generar aplica el texto y muestra el resultado", async () => {
    const user = userEvent.setup();
    const regenerated = draftFixture({
      sourceText: "bajante 5 metros",
      lines: [lineFixture({ quantity: "5", amountCents: 12500, amountDisplay: "125,00 €" })],
    });
    const backend = mockDraftBackend(transcribedSession(), {
      draft_generate_from_text: () => regenerated,
    });
    renderApp("/presupuesto");

    const text = await screen.findByLabelText(LABEL);
    await user.clear(text);
    await user.type(text, "bajante 5 metros");
    await user.click(screen.getByRole("button", { name: "Volver a generar" }));

    await waitFor(() =>
      expect(backend.called("draft_generate_from_text")[0].args).toEqual({
        text: "bajante 5 metros",
        mode: "regenerate",
        confirmed: false,
      }),
    );
    expect(await screen.findByText("125,00 €")).toBeInTheDocument();
  });

  it("avisa de que se perderán los cambios manuales y solo sigue si se confirma", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend(
      transcribedSession({ draft: draftFixture({ hasManualChanges: true }) }),
      {
        draft_generate_from_text: (args) => {
          if (!args.confirmed) throw confirmationError("discardManualChanges");
          return draftFixture();
        },
      },
    );
    renderApp("/presupuesto");

    await user.click(await screen.findByRole("button", { name: "Volver a generar" }));
    const dialog = await screen.findByRole("dialog", {
      name: "¿Perder los cambios hechos a mano?",
    });
    expect(backend.called("draft_generate_from_text")).toHaveLength(1);

    await user.click(within(dialog).getByRole("button", { name: "Continuar" }));

    await waitFor(() =>
      expect(backend.called("draft_generate_from_text").at(-1)?.args.confirmed).toBe(true),
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("al cancelar el aviso no se regenera", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend(transcribedSession(), {
      draft_generate_from_text: () => {
        throw confirmationError("discardManualChanges");
      },
    });
    renderApp("/presupuesto");
    await user.click(await screen.findByRole("button", { name: "Volver a generar" }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(backend.called("draft_generate_from_text")).toHaveLength(1);
  });
});

describe("Escribir la visita (US5)", () => {
  it("escribe el texto y genera el borrador sin dictar", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend(sessionFixture(), {
      draft_generate_from_text: () => draftFixture(),
    });
    renderApp("/presupuesto");

    await user.click(await screen.findByRole("button", { name: "Escribir la visita" }));
    await user.type(await screen.findByLabelText(LABEL), "cambiar bajante, 3 metros");
    await user.click(screen.getByRole("button", { name: "Generar presupuesto" }));

    await waitFor(() =>
      expect(backend.called("draft_generate_from_text")[0].args).toEqual({
        text: "cambiar bajante, 3 metros",
        mode: "new",
        confirmed: false,
      }),
    );
    expect(await screen.findByText("163,35 €")).toBeInTheDocument();
  });

  it("con el texto vacío indica que hay que escribir primero", async () => {
    const user = userEvent.setup();
    const error: AppError = {
      kind: "validation",
      errors: [
        {
          field: "text",
          code: "TEXT_REQUIRED",
          message: "Escribe primero lo que quieres presupuestar.",
        },
      ],
    };
    mockDraftBackend(sessionFixture(), {
      draft_generate_from_text: () => {
        throw error;
      },
    });
    renderApp("/presupuesto");

    await user.click(await screen.findByRole("button", { name: "Escribir la visita" }));
    await user.click(await screen.findByRole("button", { name: "Generar presupuesto" }));

    expect(
      await screen.findByText("Escribe primero lo que quieres presupuestar."),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(new RegExp(LABEL))).toHaveAttribute("aria-invalid", "true");
  });

  it("recupera el texto escrito que aún no tenía borrador", async () => {
    mockDraftBackend(sessionFixture({ pendingText: "cambiar grifo" }));
    renderApp("/presupuesto");
    expect(await screen.findByDisplayValue("cambiar grifo")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Generar presupuesto" })).toBeInTheDocument();
  });

  it("guarda el texto escrito aunque aún no haya borrador", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend();
    renderApp("/presupuesto");
    await user.click(await screen.findByRole("button", { name: "Escribir la visita" }));
    await user.type(await screen.findByLabelText(LABEL), "grifo");
    await waitFor(
      () => expect(backend.called("draft_text_save").at(-1)?.args).toEqual({ text: "grifo" }),
      { timeout: 2000 },
    );
  });

  it("si no se oye nada ofrece escribir la visita", async () => {
    const user = userEvent.setup();
    mockDraftBackend(sessionFixture(), { dictation_stop: () => sessionFixture() });
    renderApp("/presupuesto");
    await user.click(await screen.findByRole("button", { name: "Empezar a dictar" }));
    await user.click(await screen.findByRole("button", { name: "Parar" }));
    await screen.findByText(/No se ha oído nada/);
    await user.click(screen.getByRole("button", { name: "Escribir la visita" }));
    expect(await screen.findByLabelText(LABEL)).toBeInTheDocument();
  });
});
