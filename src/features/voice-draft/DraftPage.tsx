import { useState } from "react";
import { Banner } from "../../shared/components/Banner";
import { Button } from "../../shared/components/Button";
import { ConfirmDialog } from "../../shared/components/ConfirmDialog";
import { es } from "../../shared/i18n/es";
import { DictationPanel } from "./DictationPanel";
import { DraftLinesTable } from "./DraftLinesTable";
import { DraftTotals } from "./DraftTotals";
import { useDraftSession } from "./useDraftSession";
import { VisitTextEditor } from "./VisitTextEditor";
import styles from "./DraftPage.module.css";

export function DraftPage() {
  const session = useDraftSession();
  const draft = session.session?.draft ?? null;
  const [confirmingNew, setConfirmingNew] = useState(false);
  const [writing, setWriting] = useState(false);
  const showEditor = draft !== null || writing || Boolean(session.session?.pendingText);

  const startNew = () => {
    setConfirmingNew(false);
    setWriting(false);
    void session.discardDraft();
  };

  return (
    <div className={styles.page}>
      <header className={styles.pageHeader}>
        <h1>{es.draft.title}</h1>
        {draft ? <Button onClick={() => setConfirmingNew(true)}>{es.draft.newDraft}</Button> : null}
      </header>
      <p className={styles.intro}>{es.draft.intro}</p>

      {session.session === null && session.failure === null ? (
        <p>{es.app.loading}</p>
      ) : (
        <DictationPanel dictation={session} onWrite={() => setWriting(true)} />
      )}

      {showEditor ? <VisitTextEditor session={session} /> : null}

      {draft ? (
        <>
          {draft.vatChange ? (
            <Banner
              variant="warning"
              action={
                <Button onClick={() => void session.applyCurrentVat()}>{es.draft.vat.apply}</Button>
              }
            >
              {es.draft.vat.changed(draft.vatChange.currentRatePercent)}
            </Banner>
          ) : null}

          <DraftLinesTable draft={draft} session={session} />
          <DraftTotals totals={draft.totals} />

          {session.reviewError ? (
            <Banner variant="warning" role="alert">
              {session.reviewError}
            </Banner>
          ) : null}

          <div className={styles.reviewRow}>
            {draft.status === "reviewed" ? (
              <p className={styles.reviewed} role="status">
                {es.draft.review.reviewed}
              </p>
            ) : (
              <Button variant="primary" onClick={() => void session.markReviewed()}>
                {es.draft.review.markReviewed}
              </Button>
            )}
          </div>
        </>
      ) : null}

      <ConfirmDialog
        open={confirmingNew}
        title={es.draft.newDraftConfirmTitle}
        message={es.draft.newDraftConfirmMessage}
        onDismiss={() => setConfirmingNew(false)}
        actions={[
          { label: es.common.cancel, onClick: () => setConfirmingNew(false) },
          { label: es.draft.newDraftConfirm, onClick: startNew, variant: "danger" },
        ]}
      />
    </div>
  );
}
