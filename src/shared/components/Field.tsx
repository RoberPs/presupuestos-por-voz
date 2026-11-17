import { cloneElement, isValidElement, useId, type ReactElement, type ReactNode } from "react";
import { es } from "../i18n/es";
import styles from "./Field.module.css";

type Props = {
  label: string;
  error?: string;
  help?: ReactNode;
  required?: boolean;
  suffix?: ReactNode;
  /** Un único control (input, select…). Recibe id, aria-invalid y aria-describedby. */
  children: ReactElement;
};

export function Field({ label, error, help, required, suffix, children }: Props) {
  const id = useId();
  const helpId = `${id}-help`;
  const errorId = `${id}-error`;
  const describedBy = [help ? helpId : null, error ? errorId : null].filter(Boolean).join(" ");

  const control = isValidElement(children)
    ? cloneElement(children as ReactElement<Record<string, unknown>>, {
        id,
        "aria-invalid": error ? true : undefined,
        "aria-describedby": describedBy || undefined,
        "aria-required": required || undefined,
      })
    : children;

  return (
    <div className={styles.field}>
      <label htmlFor={id} className={styles.label}>
        {label}
        {required ? (
          <span aria-hidden="true" className={styles.required}>
            {" "}
            *
          </span>
        ) : (
          <span className={styles.optional}> {es.common.optional}</span>
        )}
      </label>
      <div className={styles.controlRow}>
        {control}
        {suffix ? <span className={styles.suffix}>{suffix}</span> : null}
      </div>
      {help ? (
        <p id={helpId} className={styles.help}>
          {help}
        </p>
      ) : null}
      <p id={errorId} className={styles.error} aria-live="polite">
        {error ?? ""}
      </p>
    </div>
  );
}
