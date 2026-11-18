import type { UseFormRegister } from "react-hook-form";
import { Field } from "../../shared/components/Field";
import { es } from "../../shared/i18n/es";
import type { FieldError } from "../../shared/ipc/types";
import styles from "./QuotePdf.module.css";

const t = es.quotePdf.form;

/** Valores del formulario: los opcionales vacíos son texto vacío hasta enviarlos al núcleo. */
export type QuoteFormValues = {
  clientName: string;
  clientAddress: string;
  clientPhone: string;
  clientEmail: string;
  clientTaxId: string;
  quoteNumber: string;
  validityDays: string;
};

type Props = {
  register: UseFormRegister<QuoteFormValues>;
  errors: FieldError[];
  validityError?: string;
  onFieldBlur: () => void;
};

/** Cliente, número y validez del presupuesto (FR-004). */
export function QuoteDetailsForm({ register, errors, validityError, onFieldBlur }: Props) {
  const errorFor = (field: string) => errors.find((e) => e.field === field)?.message;
  const text = (name: keyof QuoteFormValues) => register(name, { onBlur: onFieldBlur });

  return (
    <section className={styles.form} aria-label={t.heading}>
      <h3>{t.heading}</h3>
      <Field label={t.clientName} required error={errorFor("clientName")}>
        <input className={styles.input} autoComplete="off" data-autofocus {...text("clientName")} />
      </Field>
      <Field label={t.clientAddress} error={errorFor("clientAddress")}>
        <input className={styles.input} autoComplete="off" {...text("clientAddress")} />
      </Field>
      <Field label={t.clientPhone} error={errorFor("clientPhone")}>
        <input className={styles.input} autoComplete="off" {...text("clientPhone")} />
      </Field>
      <Field label={t.clientEmail} error={errorFor("clientEmail")}>
        <input className={styles.input} autoComplete="off" {...text("clientEmail")} />
      </Field>
      <Field label={t.clientTaxId} error={errorFor("clientTaxId")}>
        <input className={styles.input} autoComplete="off" {...text("clientTaxId")} />
      </Field>
      <Field label={t.quoteNumber} required error={errorFor("quoteNumber")}>
        <input className={styles.input} autoComplete="off" {...text("quoteNumber")} />
      </Field>
      <Field
        label={t.validityDays}
        required
        help={t.validityHelp}
        error={validityError ?? errorFor("validityDays")}
      >
        <input className={styles.input} inputMode="numeric" {...text("validityDays")} />
      </Field>
    </section>
  );
}
