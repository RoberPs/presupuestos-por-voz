import { useState } from "react";
import { ConfirmDialog } from "../../shared/components/ConfirmDialog";
import { Field } from "../../shared/components/Field";
import { es } from "../../shared/i18n/es";
import type { DraftLineInput, FieldError, Rate } from "../../shared/ipc/types";
import styles from "./DraftPage.module.css";

const t = es.draft.add;

type Props = {
  rates: Rate[];
  onAdd: (input: DraftLineInput) => Promise<FieldError[]>;
  onClose: () => void;
};

/** Añade una línea eligiendo una tarifa o escribiéndola a mano (FR-025). */
export function AddLineDialog({ rates, onAdd, onClose }: Props) {
  const [mode, setMode] = useState<"rate" | "manual">(rates.length > 0 ? "rate" : "manual");
  const [rateId, setRateId] = useState("");
  const [concept, setConcept] = useState("");
  const [quantity, setQuantity] = useState("");
  const [unitLabel, setUnitLabel] = useState("");
  const [price, setPrice] = useState("");
  const [errors, setErrors] = useState<FieldError[]>([]);
  const [rateError, setRateError] = useState<string | undefined>();
  const [saving, setSaving] = useState(false);

  const errorFor = (field: string) => errors.find((e) => e.field === field)?.message;

  const submit = async () => {
    setRateError(undefined);
    if (mode === "rate" && rateId === "") {
      setRateError(t.rateRequired);
      return;
    }
    const input: DraftLineInput =
      mode === "rate"
        ? { kind: "rate", rateId: Number(rateId), quantity }
        : { kind: "manual", concept, quantity, unitLabel, price };
    setSaving(true);
    const result = await onAdd(input);
    setSaving(false);
    setErrors(result);
    if (result.length === 0) onClose();
  };

  return (
    <ConfirmDialog
      open
      title={t.title}
      onDismiss={onClose}
      actions={[
        { label: es.common.cancel, onClick: onClose },
        { label: t.submit, onClick: () => void submit(), variant: "primary", disabled: saving },
      ]}
    >
      <div className={styles.addForm}>
        <div className={styles.modeChoice}>
          <label>
            <input
              type="radio"
              name="add-line-mode"
              checked={mode === "rate"}
              disabled={rates.length === 0}
              onChange={() => setMode("rate")}
            />
            {t.fromRate}
          </label>
          <label>
            <input
              type="radio"
              name="add-line-mode"
              checked={mode === "manual"}
              onChange={() => setMode("manual")}
            />
            {t.manual}
          </label>
        </div>
        {rates.length === 0 ? <p className={styles.hint}>{t.noRates}</p> : null}

        {mode === "rate" ? (
          <Field label={t.rate} required error={rateError}>
            <select value={rateId} onChange={(e) => setRateId(e.target.value)}>
              <option value="">{t.chooseRate}</option>
              {rates.map((rate) => (
                <option key={rate.id} value={rate.id}>
                  {`${rate.concept} (${rate.priceDisplay} por ${rate.unit.label})`}
                </option>
              ))}
            </select>
          </Field>
        ) : (
          <Field label={t.concept} required error={errorFor("concept")}>
            <input value={concept} onChange={(e) => setConcept(e.target.value)} />
          </Field>
        )}

        <Field label={t.quantity} required error={errorFor("quantity")}>
          <input
            inputMode="decimal"
            value={quantity}
            onChange={(e) => setQuantity(e.target.value)}
          />
        </Field>

        {mode === "manual" ? (
          <>
            <Field label={t.unit} required error={errorFor("unitLabel")}>
              <input value={unitLabel} onChange={(e) => setUnitLabel(e.target.value)} />
            </Field>
            <Field label={t.price} required error={errorFor("price")} suffix={t.priceSuffix}>
              <input inputMode="decimal" value={price} onChange={(e) => setPrice(e.target.value)} />
            </Field>
          </>
        ) : null}
      </div>
    </ConfirmDialog>
  );
}
