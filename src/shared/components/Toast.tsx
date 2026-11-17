import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import styles from "./Toast.module.css";

type ToastContextValue = { showToast: (message: string) => void };

const ToastContext = createContext<ToastContextValue>({ showToast: () => {} });

export const useToast = () => useContext(ToastContext);

const DURATION_MS = 3500;

/** Confirmaciones no bloqueantes, anunciadas con aria-live="polite". */
export function ToastProvider({ children }: { children: ReactNode }) {
  const [message, setMessage] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>();

  const showToast = useCallback((m: string) => {
    setMessage(m);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setMessage(null), DURATION_MS);
  }, []);

  useEffect(() => () => clearTimeout(timer.current), []);

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      <div className={styles.region} aria-live="polite" role="status">
        {message ? <div className={styles.toast}>{message}</div> : null}
      </div>
    </ToastContext.Provider>
  );
}
