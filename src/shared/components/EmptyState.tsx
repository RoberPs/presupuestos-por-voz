import type { ReactNode } from "react";
import styles from "./EmptyState.module.css";

type Props = { title: string; message?: ReactNode; action?: ReactNode };

export function EmptyState({ title, message, action }: Props) {
  return (
    <section className={styles.empty} aria-live="polite">
      <h2>{title}</h2>
      {message ? <p>{message}</p> : null}
      {action}
    </section>
  );
}
