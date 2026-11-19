import { useId, useState } from "react";
import { Banner } from "../../shared/components/Banner";
import { Button } from "../../shared/components/Button";
import { ConfirmDialog } from "../../shared/components/ConfirmDialog";
import { useToast } from "../../shared/components/Toast";
import { es } from "../../shared/i18n/es";
import type { LogoSetResult, LogoView } from "../../shared/ipc/types";
import styles from "./LogoPicker.module.css";

type Props = {
  logo: LogoView | null;
  onPick: () => Promise<LogoSetResult | null>;
  onRemove?: () => Promise<void>;
};

export function LogoPicker({ logo, onPick, onRemove }: Props) {
  const headingId = useId();
  const { showToast } = useToast();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);

  const pick = async () => {
    setBusy(true);
    setError(null);
    try {
      const result = await onPick();
      if (result) showToast(es.logo.saved);
    } catch (e) {
      setError(e instanceof Error ? e.message : es.app.genericError);
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setConfirmRemove(false);
    if (!onRemove) return;
    setBusy(true);
    setError(null);
    try {
      await onRemove();
      showToast(es.logo.removed);
    } catch (e) {
      setError(e instanceof Error ? e.message : es.app.genericError);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section aria-labelledby={headingId} className={styles.section}>
      <h2 id={headingId}>{es.business.sections.logo}</h2>
      <div className={styles.row}>
        <div className={styles.preview}>
          {logo ? (
            <img src={logo.dataUrl} alt={es.logo.alt} className={styles.image} />
          ) : (
            <span className={styles.none}>{es.logo.none}</span>
          )}
        </div>
        <div className={styles.controls}>
          <p className={styles.help}>{es.logo.help}</p>
          <div className={styles.buttons}>
            <Button variant="secondary" onClick={pick} disabled={busy}>
              {logo ? es.logo.change : es.logo.choose}
            </Button>
            {logo && onRemove ? (
              <Button variant="secondary" onClick={() => setConfirmRemove(true)} disabled={busy}>
                {es.logo.remove}
              </Button>
            ) : null}
          </div>
        </div>
      </div>
      {logo?.lowResolution ? <Banner variant="warning">{es.logo.lowResolution}</Banner> : null}
      {error ? (
        <p role="alert" className={styles.error}>
          {error}
        </p>
      ) : null}
      <ConfirmDialog
        open={confirmRemove}
        title={es.logo.removeConfirmTitle}
        message={es.logo.removeConfirmMessage}
        onDismiss={() => setConfirmRemove(false)}
        actions={[
          { label: es.common.cancel, onClick: () => setConfirmRemove(false) },
          { label: es.logo.remove, onClick: remove, variant: "danger" },
        ]}
      />
    </section>
  );
}
