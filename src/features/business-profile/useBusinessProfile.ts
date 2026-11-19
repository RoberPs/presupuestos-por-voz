import { useCallback, useEffect, useRef, useState } from "react";
import {
  isAppError,
  logoPickAndSet,
  logoRemove,
  profileSave,
  profileValidate,
} from "../../shared/ipc/client";
import type {
  BusinessProfileInput,
  BusinessProfileView,
  FieldError,
  LogoSetResult,
} from "../../shared/ipc/types";
import { es } from "../../shared/i18n/es";
import { useProfileContext } from "./ProfileContext";

export type FieldErrorMap = Record<string, string>;

export const VALIDATE_DEBOUNCE_MS = 300;

export function toErrorMap(errors: FieldError[]): FieldErrorMap {
  const map: FieldErrorMap = {};
  for (const e of errors) if (!(e.field in map)) map[e.field] = e.message;
  return map;
}

function errorMessage(e: unknown): string {
  return isAppError(e) && "message" in e ? e.message : es.app.genericError;
}

export function useBusinessProfile() {
  const { profile, setProfile, refresh } = useProfileContext();
  const [fieldErrors, setFieldErrors] = useState<FieldErrorMap>({});
  const [saving, setSaving] = useState(false);
  const [generalError, setGeneralError] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>();
  const validationSeq = useRef(0);

  useEffect(() => () => clearTimeout(timer.current), []);

  /** Guarda (borradores incluidos). Devuelve el perfil guardado o `null` si no se ha podido. */
  const save = useCallback(
    async (input: BusinessProfileInput): Promise<BusinessProfileView | null> => {
      clearTimeout(timer.current);
      validationSeq.current++;
      setSaving(true);
      setGeneralError(null);
      try {
        const view = await profileSave(input);
        setProfile(view);
        setFieldErrors(toErrorMap(view.missingFields));
        return view;
      } catch (e) {
        if (isAppError(e) && e.kind === "validation") setFieldErrors(toErrorMap(e.errors));
        else setGeneralError(errorMessage(e));
        return null;
      } finally {
        setSaving(false);
      }
    },
    [setProfile],
  );

  /** Valida sin guardar, con debounce de 300 ms. */
  const validate = useCallback((input: BusinessProfileInput) => {
    clearTimeout(timer.current);
    timer.current = setTimeout(async () => {
      const seq = ++validationSeq.current;
      try {
        const errors = await profileValidate(input);
        if (seq === validationSeq.current) setFieldErrors(toErrorMap(errors));
      } catch {
        // La validación en vivo es una ayuda: si falla, se valida al guardar.
      }
    }, VALIDATE_DEBOUNCE_MS);
  }, []);

  /** Abre el diálogo y guarda el logo. Lanza el mensaje de rechazo para mostrarlo junto al logo. */
  const pickLogo = useCallback(async (): Promise<LogoSetResult | null> => {
    try {
      const result = await logoPickAndSet();
      if (result) await refresh();
      return result;
    } catch (e) {
      throw new Error(errorMessage(e));
    }
  }, [refresh]);

  const removeLogo = useCallback(async () => {
    try {
      const view = await logoRemove();
      setProfile(view);
    } catch (e) {
      throw new Error(errorMessage(e));
    }
  }, [setProfile]);

  return {
    profile,
    loading: profile === undefined,
    fieldErrors,
    setFieldErrors,
    saving,
    generalError,
    save,
    validate,
    pickLogo,
    removeLogo,
  };
}
