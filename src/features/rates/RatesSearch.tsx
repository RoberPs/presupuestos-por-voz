import { useEffect, useId, useRef, useState } from "react";
import { es } from "../../shared/i18n/es";
import styles from "./RatesSearch.module.css";

export const SEARCH_DEBOUNCE_MS = 150;

type Props = {
  /** Se llama con el texto ya estabilizado (debounce de 150 ms; al limpiar, de inmediato). */
  onSearch: (query: string) => void;
};

export function RatesSearch({ onSearch }: Props) {
  const id = useId();
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const onSearchRef = useRef(onSearch);
  onSearchRef.current = onSearch;
  const first = useRef(true);

  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    if (value === "") {
      onSearchRef.current("");
      return;
    }
    const t = setTimeout(() => onSearchRef.current(value), SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(t);
  }, [value]);

  return (
    <div className={styles.search} role="search">
      <label htmlFor={id} className={styles.label}>
        {es.rates.search.label}
      </label>
      <div className={styles.row}>
        <input
          ref={inputRef}
          id={id}
          type="search"
          value={value}
          placeholder={es.rates.search.placeholder}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape" && value) {
              e.preventDefault();
              setValue("");
            }
          }}
          className={styles.input}
          autoComplete="off"
        />
        {value ? (
          <button
            type="button"
            className={styles.clear}
            aria-label={es.rates.search.clear}
            onClick={() => {
              setValue("");
              inputRef.current?.focus();
            }}
          >
            ×
          </button>
        ) : null}
      </div>
    </div>
  );
}
