import { Banner } from "../../shared/components/Banner";
import { Button } from "../../shared/components/Button";
import { es } from "../../shared/i18n/es";
import type { SavedQuotePdf } from "./useQuotePdf";
import styles from "./QuotePdf.module.css";

const t = es.quotePdf.result;

/** Confirma dónde ha quedado el archivo (FR-021). */
export function QuotePdfSaved({ saved }: { saved: SavedQuotePdf }) {
  return (
    <div role="status">
      <p className={styles.savedTitle}>{t.saved}</p>
      <dl className={styles.savedFile}>
        <dt>{t.fileName}</dt>
        <dd>{saved.fileName}</dd>
        <dt>{t.location}</dt>
        <dd>{saved.fullPath}</dd>
      </dl>
    </div>
  );
}

type ErrorProps = {
  message: string;
  retrying: boolean;
  onRetry: () => void;
};

/** Explica por qué no se ha podido guardar y permite elegir otra ubicación (FR-023). */
export function QuotePdfSaveError({ message, retrying, onRetry }: ErrorProps) {
  return (
    <Banner variant="warning" role="alert">
      <p className={styles.saveError}>{message}</p>
      <Button onClick={onRetry} disabled={retrying}>
        {t.retry}
      </Button>
    </Banner>
  );
}
