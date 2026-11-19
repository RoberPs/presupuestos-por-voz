import { useCallback, useEffect, useState } from "react";
import { useBlocker, type BlockerFunction } from "react-router-dom";
import { es } from "../i18n/es";
import { ConfirmDialog } from "./ConfirmDialog";

type Props = {
  /** Hay cambios sin guardar. */
  when: boolean;
  /** Guarda; devuelve true si se ha guardado y se puede salir. */
  onSave: () => Promise<boolean>;
};

/** Avisa al salir de la pantalla con cambios sin guardar: Guardar, Descartar o Cancelar (FR-009). */
export function UnsavedChangesGuard({ when, onSave }: Props) {
  const shouldBlock = useCallback<BlockerFunction>(
    ({ currentLocation, nextLocation }) =>
      when && currentLocation.pathname !== nextLocation.pathname,
    [when],
  );
  const blocker = useBlocker(shouldBlock);
  const [saving, setSaving] = useState(false);

  // Cierre o recarga de la ventana.
  useEffect(() => {
    if (!when) return;
    const onBeforeUnload = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = "";
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, [when]);

  if (blocker.state !== "blocked") return null;

  const save = async () => {
    setSaving(true);
    try {
      const ok = await onSave();
      if (ok) blocker.proceed();
      else blocker.reset();
    } finally {
      setSaving(false);
    }
  };

  return (
    <ConfirmDialog
      open
      title={es.unsaved.title}
      message={es.unsaved.message}
      onDismiss={() => blocker.reset()}
      actions={[
        { label: es.unsaved.cancel, onClick: () => blocker.reset() },
        { label: es.unsaved.discard, onClick: () => blocker.proceed(), variant: "danger" },
        { label: es.unsaved.save, onClick: save, variant: "primary", disabled: saving },
      ]}
    />
  );
}
