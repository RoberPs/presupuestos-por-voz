import { useEffect, useId, useRef, type ReactNode } from "react";
import { Button } from "./Button";
import styles from "./ConfirmDialog.module.css";

export type DialogAction = {
  label: string;
  onClick: () => void;
  variant?: "primary" | "secondary" | "danger";
  disabled?: boolean;
};

type Props = {
  open: boolean;
  title: string;
  message?: ReactNode;
  children?: ReactNode;
  actions: DialogAction[];
  /** Se llama con Esc o al pulsar fuera. */
  onDismiss: () => void;
};

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** Diálogo modal accesible: foco atrapado, Esc para cerrar y foco devuelto al disparador. */
export function ConfirmDialog({ open, title, message, children, actions, onDismiss }: Props) {
  const titleId = useId();
  const messageId = useId();
  const ref = useRef<HTMLDivElement>(null);
  const onDismissRef = useRef(onDismiss);
  onDismissRef.current = onDismiss;

  useEffect(() => {
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    const node = ref.current;
    const first = node?.querySelector<HTMLElement>("[data-autofocus]") ??
      node?.querySelector<HTMLElement>(FOCUSABLE);
    first?.focus();

    const onKey = (e: KeyboardEvent) => {
      if (!node) return;
      if (e.key === "Escape") {
        e.stopPropagation();
        onDismissRef.current();
        return;
      }
      if (e.key !== "Tab") return;
      const items = Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE));
      if (items.length === 0) return;
      const firstEl = items[0];
      const lastEl = items[items.length - 1];
      if (e.shiftKey && document.activeElement === firstEl) {
        e.preventDefault();
        lastEl.focus();
      } else if (!e.shiftKey && document.activeElement === lastEl) {
        e.preventDefault();
        firstEl.focus();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      previous?.focus?.();
    };
  }, [open]);

  if (!open) return null;

  return (
    <div className={styles.backdrop} onMouseDown={(e) => e.target === e.currentTarget && onDismiss()}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={message ? messageId : undefined}
        className={styles.dialog}
      >
        <h2 id={titleId}>{title}</h2>
        {message ? (
          <div id={messageId} className={styles.message}>
            {message}
          </div>
        ) : null}
        {children}
        {actions.length > 0 ? (
          <div className={styles.actions}>
            {actions.map((a) => (
              <Button key={a.label} variant={a.variant} onClick={a.onClick} disabled={a.disabled}>
                {a.label}
              </Button>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}
