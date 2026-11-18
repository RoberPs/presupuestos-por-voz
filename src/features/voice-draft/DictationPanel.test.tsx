import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderApp } from "../../test/renderApp";
import {
  mockDraftBackend,
  recordedSession,
  sessionFixture,
  transcribedSession,
} from "../../test/draftBackend";
import type { AppError, SessionView } from "../../shared/ipc/types";

const startButton = () => screen.findByRole("button", { name: "Empezar a dictar" });
const stopButton = () => screen.findByRole("button", { name: "Parar" });

const micError: AppError = {
  kind: "microphone",
  code: "MIC_ACCESS_DENIED",
  message: "Windows no deja usar el micrófono.",
};

describe("DictationPanel (US1)", () => {
  it("grabar y parar con el botón genera el borrador", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend();
    renderApp("/presupuesto");

    await user.click(await startButton());
    expect(backend.called("dictation_start")[0].args.confirmReplace).toBe(false);

    await user.click(await stopButton());

    await waitFor(() => expect(backend.called("dictation_process")).toHaveLength(1));
    expect(await screen.findByText("163,35 €")).toBeInTheDocument();
  });

  it("la tecla F9 empieza y para la grabación", async () => {
    const backend = mockDraftBackend();
    renderApp("/presupuesto");
    await startButton();

    fireEvent.keyDown(window, { key: "F9" });
    await stopButton();
    expect(backend.called("dictation_start")).toHaveLength(1);

    fireEvent.keyDown(window, { key: "F9" });
    await waitFor(() => expect(backend.called("dictation_stop")).toHaveLength(1));
  });

  it("muestra el tiempo de grabación y avisa del límite", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend();
    renderApp("/presupuesto");
    await user.click(await startButton());
    await stopButton();

    backend.emitRecording({ type: "level", elapsedMs: 75_000, level: 0.5 });
    expect(screen.getByText("Tiempo de grabación: 1:15")).toBeInTheDocument();

    backend.emitRecording({ type: "limitWarning", remainingMs: 60_000 });
    expect(screen.getByText("Queda 1 minuto de grabación.")).toBeInTheDocument();
  });

  it("al llegar al límite procesa lo grabado sin más pasos", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend();
    renderApp("/presupuesto");
    await user.click(await startButton());
    await stopButton();

    backend.emitRecording({ type: "stopped", reason: "limitReached", durationMs: 600_000 });

    await waitFor(() => expect(backend.called("dictation_process")).toHaveLength(1));
  });

  it("si se desconecta el micrófono conserva lo grabado y ofrece procesarlo", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend();
    renderApp("/presupuesto");
    await user.click(await startButton());
    await stopButton();

    backend.emitRecording({ type: "stopped", reason: "deviceLost", durationMs: 4_000 });

    expect(
      await screen.findByText("El micrófono se ha desconectado. Lo grabado se ha guardado."),
    ).toBeInTheDocument();
    expect(backend.called("dictation_stop")).toHaveLength(1);
    expect(backend.called("dictation_process")).toHaveLength(0);
    await user.click(await screen.findByRole("button", { name: "Procesar el audio" }));
    await waitFor(() => expect(backend.called("dictation_process")).toHaveLength(1));
  });

  it("muestra el progreso y permite cancelar sin perder el audio", async () => {
    const user = userEvent.setup();
    let finish: (value: unknown) => void = () => {};
    const backend = mockDraftBackend(recordedSession(), {
      dictation_process: () => new Promise((resolve) => (finish = resolve)),
    });
    renderApp("/presupuesto");

    await user.click(await screen.findByRole("button", { name: "Procesar el audio" }));
    expect(await screen.findByText("Procesando el dictado…")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(backend.called("dictation_cancel_processing")).toHaveLength(1);

    finish({ outcome: "cancelled", session: recordedSession() });
    expect(
      await screen.findByText("Has cancelado el proceso. Tu audio sigue guardado."),
    ).toBeInTheDocument();
  });

  it("informa de la etapa y del porcentaje del proceso", async () => {
    const user = userEvent.setup();
    let channel: { onmessage: (e: unknown) => void } | undefined;
    mockDraftBackend(recordedSession(), {
      dictation_process: (args) => {
        channel = args.onProgress as typeof channel;
        return new Promise(() => {});
      },
    });
    renderApp("/presupuesto");
    await user.click(await screen.findByRole("button", { name: "Procesar el audio" }));
    await screen.findByText("Procesando el dictado…");

    const { act } = await import("@testing-library/react");
    act(() => channel?.onmessage({ type: "stage", stage: "transcribing" }));
    act(() => channel?.onmessage({ type: "progress", percent: 40 }));

    expect(screen.getByText("Pasando el audio a texto…")).toBeInTheDocument();
    expect(screen.getByText("40 %")).toBeInTheDocument();
  });

  it("un error del micrófono explica el problema y ofrece alternativas", async () => {
    const user = userEvent.setup();
    mockDraftBackend(sessionFixture(), {
      dictation_start: () => {
        throw micError;
      },
    });
    renderApp("/presupuesto");

    await user.click(await startButton());

    expect(await screen.findByRole("alert")).toHaveTextContent(micError.message);
    expect(
      screen.getByText("También puedes importar un audio o escribir la visita a mano."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Importar un audio" })).toBeInTheDocument();
  });

  it("pide confirmación antes de sustituir un borrador", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend(transcribedSession(), {
      dictation_start: (args) => {
        if (!args.confirmReplace) {
          const error: AppError = {
            kind: "confirmationRequired",
            reason: "replaceDraft",
            message: "Ya tienes un presupuesto en curso.",
          };
          throw error;
        }
        return null;
      },
    });
    renderApp("/presupuesto");

    await user.click(await startButton());
    const dialog = await screen.findByRole("dialog", {
      name: "¿Sustituir el presupuesto en curso?",
    });
    await user.click(within(dialog).getByRole("button", { name: "Continuar" }));

    await stopButton();
    expect(backend.called("dictation_start").at(-1)?.args.confirmReplace).toBe(true);
  });

  it("al cancelar la confirmación no se graba", async () => {
    const user = userEvent.setup();
    mockDraftBackend(transcribedSession(), {
      dictation_start: () => {
        const error: AppError = {
          kind: "confirmationRequired",
          reason: "replaceDraft",
          message: "x",
        };
        throw error;
      },
    });
    renderApp("/presupuesto");
    await user.click(await startButton());
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(await startButton()).toBeInTheDocument();
  });

  it("importar un audio lo procesa igual que un dictado", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend();
    renderApp("/presupuesto");
    await user.click(await screen.findByRole("button", { name: "Importar un audio" }));
    await waitFor(() => expect(backend.called("dictation_process")).toHaveLength(1));
  });

  it("si se cancela el diálogo de importar no ocurre nada", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend(sessionFixture(), { dictation_import: () => null });
    renderApp("/presupuesto");
    await user.click(await screen.findByRole("button", { name: "Importar un audio" }));
    await waitFor(() => expect(backend.called("dictation_import")).toHaveLength(1));
    expect(backend.called("dictation_process")).toHaveLength(0);
  });

  it("un audio rechazado muestra el motivo", async () => {
    const user = userEvent.setup();
    const error: AppError = {
      kind: "audioRejected",
      code: "AUDIO_TOO_LONG",
      message: "El audio dura más de 10 minutos.",
    };
    mockDraftBackend(sessionFixture(), {
      dictation_import: () => {
        throw error;
      },
    });
    renderApp("/presupuesto");
    await user.click(await screen.findByRole("button", { name: "Importar un audio" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(error.message);
  });

  it("avisa si no se ha oído nada y ofrece repetir", async () => {
    const user = userEvent.setup();
    const noSpeech: SessionView = {
      ...recordedSession(),
      dictation: { ...recordedSession().dictation!, status: "noSpeech" },
    };
    mockDraftBackend(recordedSession(), {
      dictation_process: () => ({ outcome: "noSpeech", session: noSpeech }),
    });
    renderApp("/presupuesto");
    await user.click(await screen.findByRole("button", { name: "Procesar el audio" }));
    expect(
      await screen.findByText(
        "No se ha oído nada en el audio. Puedes repetir el dictado o escribir la visita.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Repetir el dictado" })).toBeInTheDocument();
  });

  it("parar sin audio avisa de que no se ha oído nada", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend(sessionFixture(), {
      dictation_stop: () => sessionFixture(),
    });
    renderApp("/presupuesto");
    await user.click(await startButton());
    await user.click(await stopButton());
    expect(
      await screen.findByText(
        "No se ha oído nada en el audio. Puedes repetir el dictado o escribir la visita.",
      ),
    ).toBeInTheDocument();
    expect(backend.called("dictation_process")).toHaveLength(0);
  });

  it("un fallo de la transcripción permite reintentar", async () => {
    const user = userEvent.setup();
    const error: AppError = {
      kind: "transcription",
      code: "TRANSCRIPTION_FAILED",
      message: "No se ha podido pasar el audio a texto.",
    };
    const failed: SessionView = {
      ...recordedSession(),
      dictation: {
        ...recordedSession().dictation!,
        status: "failed",
        errorCode: "TRANSCRIPTION_FAILED",
      },
    };
    const backend = mockDraftBackend(failed, {
      dictation_process: () => {
        throw error;
      },
    });
    renderApp("/presupuesto");
    await user.click(await screen.findByRole("button", { name: "Reintentar" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(error.message);
    expect(backend.called("dictation_process")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "Reintentar" })).toBeInTheDocument();
  });

  it("avisa de la calidad baja del audio", async () => {
    const session = transcribedSession();
    mockDraftBackend({
      ...session,
      dictation: { ...session.dictation!, lowQuality: true },
    });
    renderApp("/presupuesto");
    expect(
      await screen.findByText("El audio se oye con dificultad. Revisa bien el texto."),
    ).toBeInTheDocument();
  });

  it("informa sin bloquear de que la transcripción se ha hecho en el equipo (US3)", async () => {
    mockDraftBackend(transcribedSession());
    renderApp("/presupuesto");
    const notice = await screen.findByText(
      "El audio se ha pasado a texto en este equipo, sin enviar nada a Internet.",
    );
    expect(notice.closest('[role="alert"]')).toBeNull();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(await startButton()).toBeEnabled();
  });

  it("no muestra ese aviso si el texto no procede de un dictado", async () => {
    mockDraftBackend(transcribedSession({ dictation: null }));
    renderApp("/presupuesto");
    await startButton();
    expect(screen.queryByText(/en este equipo, sin enviar nada/)).not.toBeInTheDocument();
  });

  it("ofrece recuperar una grabación interrumpida", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend(sessionFixture({ recovery: { durationMs: 83_000 } }));
    renderApp("/presupuesto");
    expect(await screen.findByText("Hay una grabación interrumpida de 1:23.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Recuperar la grabación" }));
    await waitFor(() => expect(backend.called("dictation_recover")).toHaveLength(1));
  });

  it("al volver a la pantalla sigue viendo la grabación y puede pararla", async () => {
    const user = userEvent.setup();
    const backend = mockDraftBackend(
      sessionFixture({ activity: { kind: "recording", elapsedMs: 12_000 } }),
    );
    renderApp("/presupuesto");

    expect(await stopButton()).toBeInTheDocument();
    expect(screen.getByText("Tiempo de grabación: 0:12")).toBeInTheDocument();
    await waitFor(() => expect(backend.called("dictation_subscribe")).toHaveLength(1));

    backend.emitRecording({ type: "level", elapsedMs: 20_000, level: 0.2 });
    expect(screen.getByText("Tiempo de grabación: 0:20")).toBeInTheDocument();

    await user.click(await stopButton());
    await waitFor(() => expect(backend.called("dictation_stop")).toHaveLength(1));
  });

  it("al volver a la pantalla sigue viendo el proceso en curso", async () => {
    const backend = mockDraftBackend(
      sessionFixture({ activity: { kind: "processing", percent: 30 } }),
    );
    renderApp("/presupuesto");
    expect(await screen.findByText("Procesando el dictado…")).toBeInTheDocument();
    expect(screen.getByText("30 %")).toBeInTheDocument();
    await waitFor(() => expect(backend.called("dictation_subscribe")).toHaveLength(1));
  });
});
