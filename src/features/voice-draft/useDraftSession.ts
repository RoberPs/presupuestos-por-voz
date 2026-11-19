import { Channel } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { es } from "../../shared/i18n/es";
import {
  dictationCancelProcessing,
  dictationDiscard,
  dictationImport,
  dictationProcess,
  dictationRecover,
  dictationStart,
  dictationStop,
  dictationSubscribe,
  draftApplyCurrentVat,
  draftDiscard,
  draftGenerateFromText,
  draftLineAdd,
  draftLineAssignRate,
  draftLineDelete,
  draftLineUpdate,
  draftMarkReviewed,
  draftTextSave,
  isAppError,
  ratesList,
  sessionGet,
} from "../../shared/ipc/client";
import type {
  ConfirmationReason,
  DraftGenerateArgs,
  DraftLineInput,
  DraftLinePatch,
  DraftView,
  FieldError,
  ProcessingEvent,
  ProcessingStage,
  Rate,
  RecordingEvent,
  SessionActivity,
  SessionView,
} from "../../shared/ipc/types";

export type DictationNotice = "limitReached" | "deviceLost" | "noSpeech" | "cancelled";

export type PendingConfirmation = {
  action: "record" | "import" | "generate";
  reason: ConfirmationReason;
  message: string;
  /** Texto y modo que se generarán al confirmar. */
  generate?: Pick<DraftGenerateArgs, "text" | "mode">;
};

export type DictationFailure = { message: string; microphone: boolean };

const errorMessage = (e: unknown) =>
  isAppError(e) && "message" in e ? e.message : es.app.genericError;

/** Estado de la pantalla «Presupuesto». Los importes llegan siempre calculados por el núcleo. */
export function useDraftSession() {
  const [session, setSession] = useState<SessionView | null>(null);
  const [rates, setRates] = useState<Rate[] | null>(null);
  const [activity, setActivity] = useState<SessionActivity>({ kind: "idle" });
  const [elapsedMs, setElapsedMs] = useState(0);
  const [level, setLevel] = useState(0);
  const [limitWarning, setLimitWarning] = useState(false);
  const [stage, setStage] = useState<ProcessingStage | null>(null);
  const [percent, setPercent] = useState<number | null>(null);
  const [notice, setNotice] = useState<DictationNotice | null>(null);
  const [failure, setFailure] = useState<DictationFailure | null>(null);
  const [confirmation, setConfirmation] = useState<PendingConfirmation | null>(null);
  const [lineErrors, setLineErrors] = useState<Record<number, FieldError[]>>({});
  const [pendingLineIds, setPendingLineIds] = useState<number[]>([]);
  const [reviewError, setReviewError] = useState<string | null>(null);
  const [textError, setTextError] = useState<string | null>(null);

  const stopping = useRef(false);
  const processingHere = useRef(false);

  const applySession = useCallback((next: SessionView) => {
    setSession(next);
    setActivity(next.activity);
  }, []);

  const clearMessages = useCallback(() => {
    setNotice(null);
    setFailure(null);
    setLimitWarning(false);
  }, []);

  const refresh = useCallback(async () => {
    applySession(await sessionGet());
  }, [applySession]);

  const onProcessingEvent = useCallback(
    (event: ProcessingEvent) => {
      if (event.type === "stage") setStage(event.stage);
      if (event.type === "progress") setPercent(event.percent);
      // Al volver a la pantalla, el proceso lo lanzó una vista anterior.
      if (event.type === "finished" && !processingHere.current) void refresh();
    },
    [refresh],
  );

  const process = useCallback(async () => {
    clearMessages();
    processingHere.current = true;
    setActivity({ kind: "processing", percent: null });
    setStage(null);
    setPercent(null);
    try {
      const result = await dictationProcess(new Channel<ProcessingEvent>(onProcessingEvent));
      applySession(result.session);
      if (result.outcome === "noSpeech") setNotice("noSpeech");
      if (result.outcome === "cancelled") setNotice("cancelled");
    } catch (e) {
      setFailure({ message: errorMessage(e), microphone: false });
      await refresh();
    } finally {
      processingHere.current = false;
    }
  }, [applySession, clearMessages, onProcessingEvent, refresh]);

  const stop = useCallback(
    async (processAfter: boolean) => {
      if (stopping.current) return;
      stopping.current = true;
      try {
        const next = await dictationStop();
        applySession(next);
        if (next.dictation?.status !== "recorded") {
          setNotice("noSpeech");
        } else if (processAfter) {
          await process();
        }
      } catch (e) {
        setFailure({ message: errorMessage(e), microphone: false });
        await refresh();
      } finally {
        stopping.current = false;
      }
    },
    [applySession, process, refresh],
  );

  const onRecordingEvent = useCallback(
    (event: RecordingEvent) => {
      if (event.type === "level") {
        setElapsedMs(event.elapsedMs);
        setLevel(event.level);
      } else if (event.type === "limitWarning") {
        setLimitWarning(true);
      } else if (event.reason === "limitReached") {
        setNotice("limitReached");
        void stop(true);
      } else {
        setNotice("deviceLost");
        void stop(false);
      }
    },
    [stop],
  );

  const handleStartError = useCallback((e: unknown, action: "record" | "import") => {
    if (isAppError(e) && e.kind === "confirmationRequired") {
      setConfirmation({ action, reason: e.reason, message: e.message });
    } else {
      setFailure({
        message: errorMessage(e),
        microphone: isAppError(e) && e.kind === "microphone",
      });
    }
  }, []);

  const startRecording = useCallback(
    async (confirmed = false) => {
      clearMessages();
      try {
        await dictationStart(confirmed, new Channel<RecordingEvent>(onRecordingEvent));
        setElapsedMs(0);
        setLevel(0);
        setActivity({ kind: "recording", elapsedMs: 0 });
      } catch (e) {
        handleStartError(e, "record");
      }
    },
    [clearMessages, handleStartError, onRecordingEvent],
  );

  const stopRecording = useCallback(() => stop(true), [stop]);

  const importAudio = useCallback(
    async (confirmed = false) => {
      clearMessages();
      try {
        const next = await dictationImport(confirmed);
        if (!next) return;
        applySession(next);
        await process();
      } catch (e) {
        handleStartError(e, "import");
      }
    },
    [applySession, clearMessages, handleStartError, process],
  );

  const generateDraft = useCallback(
    async (text: string, mode: DraftGenerateArgs["mode"], confirmed = false) => {
      clearMessages();
      setTextError(null);
      try {
        const draft = await draftGenerateFromText({ text, mode, confirmed });
        setSession((current) =>
          current
            ? {
                ...current,
                draft,
                pendingText: null,
                dictation: mode === "new" ? null : current.dictation,
                recovery: mode === "new" ? null : current.recovery,
              }
            : current,
        );
        setLineErrors({});
        setPendingLineIds([]);
        setReviewError(null);
      } catch (e) {
        if (isAppError(e) && e.kind === "confirmationRequired") {
          setConfirmation({
            action: "generate",
            reason: e.reason,
            message: e.message,
            generate: { text, mode },
          });
        } else if (isAppError(e) && e.kind === "validation") {
          setTextError(e.errors[0]?.message ?? es.app.genericError);
        } else {
          setFailure({ message: errorMessage(e), microphone: false });
        }
      }
    },
    [clearMessages],
  );

  const saveText = useCallback(async (text: string) => {
    try {
      await draftTextSave(text);
    } catch (e) {
      setTextError(errorMessage(e));
    }
  }, []);

  const confirmPending = useCallback(() => {
    const pending = confirmation;
    setConfirmation(null);
    if (pending?.action === "record") void startRecording(true);
    if (pending?.action === "import") void importAudio(true);
    if (pending?.generate) void generateDraft(pending.generate.text, pending.generate.mode, true);
  }, [confirmation, generateDraft, importAudio, startRecording]);

  const dismissConfirmation = useCallback(() => setConfirmation(null), []);

  const cancelProcessing = useCallback(() => dictationCancelProcessing(), []);

  const recover = useCallback(async () => {
    clearMessages();
    try {
      const next = await dictationRecover();
      applySession(next);
      if (next.dictation?.status === "recorded") await process();
    } catch (e) {
      setFailure({ message: errorMessage(e), microphone: false });
    }
  }, [applySession, clearMessages, process]);

  const discardAudio = useCallback(async () => {
    clearMessages();
    try {
      applySession(await dictationDiscard());
    } catch (e) {
      setFailure({ message: errorMessage(e), microphone: false });
    }
  }, [applySession, clearMessages]);

  /** Muestra siempre el borrador que devuelve el núcleo, sin calcular nada aquí. */
  const applyDraft = useCallback((draft: DraftView) => {
    setSession((current) => (current ? { ...current, draft } : current));
    setPendingLineIds([]);
    setReviewError(null);
    setFailure(null);
  }, []);

  const editFailure = useCallback((e: unknown) => {
    setFailure({ message: errorMessage(e), microphone: false });
  }, []);

  const updateLine = useCallback(
    async (lineId: number, patch: DraftLinePatch) => {
      try {
        applyDraft(await draftLineUpdate(lineId, patch));
        setLineErrors((current) => ({ ...current, [lineId]: [] }));
      } catch (e) {
        if (isAppError(e) && e.kind === "validation") {
          setLineErrors((current) => ({ ...current, [lineId]: e.errors }));
        } else {
          editFailure(e);
        }
      }
    },
    [applyDraft, editFailure],
  );

  /** Devuelve los errores por campo, o una lista vacía si la línea se ha añadido. */
  const addLine = useCallback(
    async (input: DraftLineInput): Promise<FieldError[]> => {
      try {
        applyDraft(await draftLineAdd(input));
        return [];
      } catch (e) {
        if (isAppError(e) && e.kind === "validation") return e.errors;
        editFailure(e);
        return [];
      }
    },
    [applyDraft, editFailure],
  );

  const assignRate = useCallback(
    async (lineId: number, rateId: number) => {
      try {
        applyDraft(await draftLineAssignRate(lineId, rateId));
        setLineErrors((current) => ({ ...current, [lineId]: [] }));
      } catch (e) {
        editFailure(e);
      }
    },
    [applyDraft, editFailure],
  );

  const deleteLine = useCallback(
    async (lineId: number) => {
      try {
        applyDraft(await draftLineDelete(lineId));
      } catch (e) {
        editFailure(e);
      }
    },
    [applyDraft, editFailure],
  );

  const markReviewed = useCallback(async () => {
    try {
      applyDraft(await draftMarkReviewed());
    } catch (e) {
      if (isAppError(e) && e.kind === "draftHasPending") {
        setPendingLineIds(e.pendingLineIds);
        setReviewError(es.draft.review.pendingError);
      } else {
        setReviewError(errorMessage(e));
      }
    }
  }, [applyDraft]);

  const applyCurrentVat = useCallback(async () => {
    try {
      applyDraft(await draftApplyCurrentVat());
    } catch (e) {
      editFailure(e);
    }
  }, [applyDraft, editFailure]);

  const discardDraft = useCallback(async () => {
    clearMessages();
    try {
      applySession(await draftDiscard());
      setLineErrors({});
      setPendingLineIds([]);
      setReviewError(null);
    } catch (e) {
      editFailure(e);
    }
  }, [applySession, clearMessages, editFailure]);

  // La carga inicial solo debe ocurrir una vez al abrir la pantalla.
  const onRecordingRef = useRef(onRecordingEvent);
  onRecordingRef.current = onRecordingEvent;
  const onProcessingRef = useRef(onProcessingEvent);
  onProcessingRef.current = onProcessingEvent;

  useEffect(() => {
    let active = true;
    (async () => {
      try {
        const [loaded, loadedRates] = await Promise.all([sessionGet(), ratesList()]);
        if (!active) return;
        setRates(loadedRates);
        setSession(loaded);
        setActivity(loaded.activity);
        if (loaded.activity.kind === "recording") setElapsedMs(loaded.activity.elapsedMs);
        if (loaded.activity.kind === "processing") setPercent(loaded.activity.percent);
        if (loaded.activity.kind !== "idle") {
          await dictationSubscribe(
            new Channel<RecordingEvent>((event) => onRecordingRef.current(event)),
            new Channel<ProcessingEvent>((event) => onProcessingRef.current(event)),
          );
        }
      } catch (e) {
        if (active) setFailure({ message: errorMessage(e), microphone: false });
      }
    })();
    return () => {
      active = false;
    };
  }, []);

  return {
    session,
    rates,
    activity,
    elapsedMs,
    level,
    limitWarning,
    stage,
    percent,
    notice,
    failure,
    confirmation,
    startRecording,
    stopRecording,
    importAudio,
    process,
    cancelProcessing,
    recover,
    discardAudio,
    confirmPending,
    dismissConfirmation,
    lineErrors,
    pendingLineIds,
    reviewError,
    updateLine,
    addLine,
    assignRate,
    deleteLine,
    markReviewed,
    applyCurrentVat,
    discardDraft,
    textError,
    generateDraft,
    saveText,
  };
}

export type DraftSession = ReturnType<typeof useDraftSession>;
