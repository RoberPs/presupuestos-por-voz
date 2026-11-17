import { useEffect, useState, type FormEvent } from "react";
import { Button } from "../../shared/components/Button";
import { ConfirmDialog } from "../../shared/components/ConfirmDialog";
import { Field } from "../../shared/components/Field";
import { es } from "../../shared/i18n/es";
import type { AppError, Rate, RateInput, Unit } from "../../shared/ipc/types";
import { UnitSelect } from "./UnitSelect";
import styles from "./RateFormDialog.module.css";

export type RateDialogMode = { kind: "create"; initialConcept?: string } | { kind: "edit"; rate: Rate };

type Props = {
  mode: RateDialogMode | null;
  units: Unit[];
  onSave: (input: RateInput, mode: RateDialogMode) => Promise<void>;
  onClose: () => void;
  onEditExisting: (id: number) => void;
};

/** Precio en céntimos → texto editable "1234,56". */
export function centsToInput(cents: number): string {
  return (cents / 100).toFixed(2).replace(".", ",");
}

const empty: RateInput = { concept: "", unitLabel: "", price: "" };

export function RateFormDialog({ mode, units, onSave, onClose, onEditExisting }: Props) {
  const [values, setValues] = useState<RateInput>(empty);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [duplicate, setDuplicate] = useState<{ id: number; message: string } | null>(null);
  const [generalError, setGeneralError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!mode) return;
    setErrors({});
    setDuplicate(null);
    setGeneralError(null);
    setValues(
      mode.kind === "edit"
        ? {
            concept: mode.rate.concept,
            unitLabel: mode.rate.unit.label,
            price: centsToInput(mode.rate.priceCents),
          }
        : { ...empty, concept: mode.initialConcept ?? "" },
    );
  }, [mode]);

  if (!mode) return null;

  const set = (key: keyof RateInput) => (e: { target: { value: string } }) =>
    setValues((v) => ({ ...v, [key]: e.target.value }));

  const submit = async (e?: FormEvent) => {
    e?.preventDefault();
    setSaving(true);
    setErrors({});
    setDuplicate(null);
    setGeneralError(null);
    try {
      await onSave(values, mode);
    } catch (err) {
      const appError = err as AppError;
      if (appError.kind === "validation") {
        const map: Record<string, string> = {};
        for (const fe of appError.errors) if (!(fe.field in map)) map[fe.field] = fe.message;
        setErrors(map);
      } else if (appError.kind === "duplicateRate") {
        setDuplicate({ id: appError.existingRateId, message: appError.message });
      } else {
        setGeneralError(appError.message ?? es.app.genericError);
      }
    } finally {
      setSaving(false);
    }
  };

  const title = mode.kind === "edit" ? es.rates.form.editTitle : es.rates.form.createTitle;

  return (
    <ConfirmDialog open title={title} onDismiss={onClose} actions={[]}>
      <form onSubmit={submit} noValidate className={styles.form}>
        <Field label={es.rates.form.concept} required error={errors.concept}>
          <input
            value={values.concept}
            onChange={set("concept")}
            placeholder={es.rates.form.conceptPlaceholder}
            data-autofocus
          />
        </Field>
        <Field
          label={es.rates.form.unit}
          required
          help={es.rates.form.unitHelp}
          error={errors.unitLabel}
        >
          <UnitSelect units={units} value={values.unitLabel} onChange={set("unitLabel")} />
        </Field>
        <Field
          label={es.rates.form.price}
          required
          suffix={es.rates.form.priceSuffix}
          error={errors.price}
        >
          <input value={values.price} onChange={set("price")} inputMode="decimal" />
        </Field>

        {duplicate ? (
          <div role="alert" className={styles.alert}>
            <p>{duplicate.message}</p>
            <Button variant="secondary" onClick={() => onEditExisting(duplicate.id)}>
              {es.rates.form.editExisting}
            </Button>
          </div>
        ) : null}
        {generalError ? (
          <p role="alert" className={styles.alert}>
            {generalError}
          </p>
        ) : null}

        <div className={styles.actions}>
          <Button onClick={onClose}>{es.common.cancel}</Button>
          <Button type="submit" variant="primary" disabled={saving}>
            {saving ? es.common.saving : es.common.save}
          </Button>
        </div>
      </form>
    </ConfirmDialog>
  );
}
