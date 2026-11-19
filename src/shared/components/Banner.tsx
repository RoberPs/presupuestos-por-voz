import type { ReactNode } from "react";
import styles from "./Banner.module.css";

type Props = {
  variant?: "info" | "warning";
  children: ReactNode;
  action?: ReactNode;
  /** `alert` para avisos importantes que deben anunciarse; `status` por defecto. */
  role?: "status" | "alert";
};

export function Banner({ variant = "info", children, action, role = "status" }: Props) {
  return (
    <div className={`${styles.banner} ${styles[variant]}`} role={role}>
      <span className={styles.text}>{children}</span>
      {action ? <span className={styles.action}>{action}</span> : null}
    </div>
  );
}
