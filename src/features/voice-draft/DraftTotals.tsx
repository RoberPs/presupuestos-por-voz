import { Banner } from "../../shared/components/Banner";
import { es } from "../../shared/i18n/es";
import type { DraftTotalsView } from "../../shared/ipc/types";
import styles from "./DraftPage.module.css";

const t = es.draft.totals;

type Props = { totals: DraftTotalsView };

export function DraftTotals({ totals }: Props) {
  return (
    <section className={styles.panel} aria-labelledby="totals-heading">
      <h2 id="totals-heading">{t.heading}</h2>
      <dl className={styles.totals} aria-live="polite">
        <div>
          <dt>{t.base}</dt>
          <dd>{totals.baseDisplay}</dd>
        </div>
        <div>
          <dt>{t.vat(totals.vatRatePercent)}</dt>
          <dd>{totals.vatDisplay}</dd>
        </div>
        <div className={styles.grandTotal}>
          <dt>{t.total}</dt>
          <dd>{totals.totalDisplay}</dd>
        </div>
      </dl>
      {totals.isProvisional ? (
        <Banner variant="warning">{t.provisional(totals.pendingCount)}</Banner>
      ) : null}
    </section>
  );
}
