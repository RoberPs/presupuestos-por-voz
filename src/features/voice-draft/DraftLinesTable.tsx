import { useState } from "react";
import { Link } from "react-router-dom";
import { Banner } from "../../shared/components/Banner";
import { Button } from "../../shared/components/Button";
import { ConfirmDialog } from "../../shared/components/ConfirmDialog";
import { es } from "../../shared/i18n/es";
import type { DraftLineView, DraftView } from "../../shared/ipc/types";
import { AddLineDialog } from "./AddLineDialog";
import { DraftLineRow } from "./DraftLineRow";
import type { DraftSession } from "./useDraftSession";
import styles from "./DraftPage.module.css";

const t = es.draft.lines;

type Props = { draft: DraftView; session: DraftSession };

export function DraftLinesTable({ draft, session }: Props) {
  const { rates } = session;
  const [adding, setAdding] = useState(false);
  const [toDelete, setToDelete] = useState<DraftLineView | null>(null);

  const confirmDelete = () => {
    const line = toDelete;
    setToDelete(null);
    if (line) void session.deleteLine(line.id);
  };

  return (
    <section className={styles.panel} aria-labelledby="lines-heading">
      <div className={styles.sectionHeader}>
        <h2 id="lines-heading">{t.heading}</h2>
        <Button onClick={() => setAdding(true)}>{t.add}</Button>
      </div>

      {rates !== null && rates.length === 0 ? (
        <Banner variant="warning" action={<Link to="/tarifas">{t.createRates}</Link>}>
          {t.noRates}
        </Banner>
      ) : null}

      {draft.lines.length === 0 ? (
        <p>{t.empty}</p>
      ) : (
        <div className={styles.tableWrap}>
          <table className={styles.table}>
            <thead>
              <tr>
                <th scope="col">{t.columns.concept}</th>
                <th scope="col" className={styles.num}>
                  {t.columns.quantity}
                </th>
                <th scope="col">{t.columns.unit}</th>
                <th scope="col" className={styles.num}>
                  {t.columns.price}
                </th>
                <th scope="col" className={styles.num}>
                  {t.columns.amount}
                </th>
                <th scope="col">
                  <span className="visually-hidden">{t.columns.actions}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {draft.lines.map((line, index) => (
                <DraftLineRow
                  key={line.id}
                  line={line}
                  position={index + 1}
                  errors={session.lineErrors[line.id] ?? []}
                  highlighted={session.pendingLineIds.includes(line.id)}
                  onUpdate={(patch) => void session.updateLine(line.id, patch)}
                  onAssignRate={(rateId) => void session.assignRate(line.id, rateId)}
                  onDelete={() => setToDelete(line)}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}

      {draft.unparsedFragments.length > 0 ? (
        <div className={styles.unparsed}>
          <h3>{t.unparsedHeading}</h3>
          <p className={styles.hint}>{t.unparsedHelp}</p>
          <ul>
            {draft.unparsedFragments.map((fragment, index) => (
              <li key={`${index}-${fragment}`}>{fragment}</li>
            ))}
          </ul>
        </div>
      ) : null}

      {adding ? (
        <AddLineDialog
          rates={rates ?? []}
          onAdd={session.addLine}
          onClose={() => setAdding(false)}
        />
      ) : null}

      <ConfirmDialog
        open={toDelete !== null}
        title={t.deleteConfirmTitle}
        message={toDelete ? t.deleteConfirmMessage(toDelete.concept) : undefined}
        onDismiss={() => setToDelete(null)}
        actions={[
          { label: es.common.cancel, onClick: () => setToDelete(null) },
          { label: es.common.delete, onClick: confirmDelete, variant: "danger" },
        ]}
      />
    </section>
  );
}
