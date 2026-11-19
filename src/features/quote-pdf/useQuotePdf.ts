import { useCallback, useEffect, useRef, useState } from "react";
import { isAppError, quoteDetailsSave, quoteGenerate, quotePrepare } from "../../shared/ipc/client";
import type {
  FieldError,
  QuoteDetailsInput,
  QuoteGenerateResult,
  QuotePreparationView,
} from "../../shared/ipc/types";
import { es } from "../../shared/i18n/es";

type SaveOutcome = { view: QuotePreparationView } | { fieldErrors: FieldError[] } | null;

export type SavedQuotePdf = Extract<QuoteGenerateResult, { outcome: "saved" }>;

const messageOf = (e: unknown) =>
  isAppError(e) && "message" in e ? e.message : es.app.genericError;

/** Estado de la preparación, el guardado de los datos y la generación del PDF. */
export function useQuotePdf() {
  const [preparation, setPreparation] = useState<QuotePreparationView | null>(null);
  const [fieldErrors, setFieldErrors] = useState<FieldError[]>([]);
  const [failure, setFailure] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);
  const [saved, setSaved] = useState<SavedQuotePdf | null>(null);
  const allowMissingLogo = useRef(false);
  const confirmedToken = useRef<string | null>(null);
  // Los guardados se encadenan para que el último escrito sea siempre el último aplicado.
  const saves = useRef<Promise<unknown>>(Promise.resolve());

  const reload = useCallback(async () => {
    try {
      setPreparation(await quotePrepare(allowMissingLogo.current));
      setFailure(null);
    } catch (e) {
      setFailure(messageOf(e));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const saveDetails = useCallback((details: QuoteDetailsInput): Promise<SaveOutcome> => {
    const run = async (): Promise<SaveOutcome> => {
      try {
        const view = await quoteDetailsSave(details, allowMissingLogo.current);
        setPreparation(view);
        setFieldErrors([]);
        setFailure(null);
        return { view };
      } catch (e) {
        if (isAppError(e) && e.kind === "validation") {
          setFieldErrors(e.errors);
          return { fieldErrors: e.errors };
        }
        setFailure(messageOf(e));
        return null;
      }
    };
    const next = saves.current.then(run, run);
    saves.current = next;
    return next;
  }, []);

  /** Un fallo de guardado se puede reintentar; cualquier otro obliga a revisar de nuevo. */
  const generate = useCallback(
    async (token: string) => {
      setSaveError(null);
      try {
        const outcome = await quoteGenerate(token, allowMissingLogo.current);
        if (outcome.outcome === "saved") setSaved(outcome);
      } catch (e) {
        if (isAppError(e) && e.kind === "pdfSave") {
          setSaveError(e.message);
          return;
        }
        await reload();
        setFailure(messageOf(e));
      }
    },
    [reload],
  );

  /** Guarda lo escrito y, si no hay bloqueos, genera con el token que devuelve el núcleo. */
  const confirm = useCallback(
    async (details: QuoteDetailsInput) => {
      setGenerating(true);
      try {
        const result = await saveDetails(details);
        if (!result || !("view" in result)) return;
        const token = result.view.confirmationToken;
        if (token === null) return;
        confirmedToken.current = token;
        await generate(token);
      } finally {
        setGenerating(false);
      }
    },
    [generate, saveDetails],
  );

  /** Vuelve a elegir la ubicación sin repetir la confirmación (FR-023). */
  const retry = useCallback(async () => {
    if (confirmedToken.current === null) return;
    setGenerating(true);
    try {
      await generate(confirmedToken.current);
    } finally {
      setGenerating(false);
    }
  }, [generate]);

  const continueWithoutLogo = useCallback(() => {
    allowMissingLogo.current = true;
    void reload();
  }, [reload]);

  /** Actualiza el IVA del borrador: los importes cambian y hay que volver a revisarlo (FR-005b). */
  const applyVat = useCallback(
    async (apply: () => Promise<unknown>) => {
      try {
        await apply();
      } catch (e) {
        setFailure(messageOf(e));
      }
      await reload();
    },
    [reload],
  );

  return {
    preparation,
    fieldErrors,
    failure,
    saveError,
    generating,
    saved,
    saveDetails,
    confirm,
    retry,
    continueWithoutLogo,
    applyVat,
  };
}
