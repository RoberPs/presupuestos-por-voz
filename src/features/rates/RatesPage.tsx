import { useState } from "react";
import { Banner } from "../../shared/components/Banner";
import { Button } from "../../shared/components/Button";
import { ConfirmDialog } from "../../shared/components/ConfirmDialog";
import { EmptyState } from "../../shared/components/EmptyState";
import { useToast } from "../../shared/components/Toast";
import { es } from "../../shared/i18n/es";
import type { Rate, RateInput } from "../../shared/ipc/types";
import { RateFormDialog, type RateDialogMode } from "./RateFormDialog";
import { useRates } from "./useRates";
import styles from "./RatesPage.module.css";

export function RatesPage() {
  const { showToast } = useToast();
  const { rates, units, loading, lastError, create, update, remove } = useRates();
  const [dialog, setDialog] = useState<RateDialogMode | null>(null);
  const [toDelete, setToDelete] = useState<Rate | null>(null);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const save = async (input: RateInput, mode: RateDialogMode) => {
    if (mode.kind === "edit") {
      await update(mode.rate.id, input);
      showToast(es.rates.updated);
    } else {
      await create(input);
      showToast(es.rates.created);
    }
    setDialog(null);
  };

  const editExisting = (id: number) => {
    const rate = rates.find((r) => r.id === id);
    if (rate) setDialog({ kind: "edit", rate });
  };

  const confirmDelete = async () => {
    const rate = toDelete;
    setToDelete(null);
    if (!rate) return;
    try {
      await remove(rate.id);
      setDeleteError(null);
      showToast(es.rates.deleted);
    } catch (e) {
      setDeleteError((e as { message?: string }).message ?? es.app.genericError);
    }
  };

  return (
    <div>
      <header className={styles.header}>
        <h1>{es.rates.title}</h1>
        <Button variant="primary" onClick={() => setDialog({ kind: "create" })}>
          {es.rates.newRate}
        </Button>
      </header>
      <p className={styles.intro}>{es.rates.intro}</p>
      <Banner variant="info">{es.rates.vatNotice}</Banner>

      {lastError || deleteError ? (
        <Banner variant="warning" role="alert">
          {deleteError ??
            (lastError && "message" in lastError ? lastError.message : es.app.genericError)}
        </Banner>
      ) : null}

      {loading ? (
        <p>{es.app.loading}</p>
      ) : rates.length === 0 ? (
        <EmptyState
          title={es.rates.emptyTitle}
          message={es.rates.emptyMessage}
          action={
            <Button variant="primary" onClick={() => setDialog({ kind: "create" })}>
              {es.rates.createFirst}
            </Button>
          }
        />
      ) : (
        <RatesTable
          rates={rates}
          onEdit={(rate) => setDialog({ kind: "edit", rate })}
          onDelete={setToDelete}
        />
      )}

      <RateFormDialog
        mode={dialog}
        units={units}
        onSave={save}
        onClose={() => setDialog(null)}
        onEditExisting={editExisting}
      />

      <ConfirmDialog
        open={toDelete !== null}
        title={es.rates.deleteConfirmTitle}
        message={toDelete ? es.rates.deleteConfirmMessage(toDelete.concept) : undefined}
        onDismiss={() => setToDelete(null)}
        actions={[
          { label: es.common.cancel, onClick: () => setToDelete(null) },
          { label: es.common.delete, onClick: confirmDelete, variant: "danger" },
        ]}
      />
    </div>
  );
}

type TableProps = {
  rates: Rate[];
  onEdit: (rate: Rate) => void;
  onDelete: (rate: Rate) => void;
};

export function RatesTable({ rates, onEdit, onDelete }: TableProps) {
  return (
    <table className={styles.table}>
      <thead>
        <tr>
          <th scope="col">{es.rates.columns.concept}</th>
          <th scope="col">{es.rates.columns.unit}</th>
          <th scope="col" className={styles.num}>
            {es.rates.columns.price}
          </th>
          <th scope="col">
            <span className="visually-hidden">{es.rates.columns.actions}</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {rates.map((r) => (
          <tr key={r.id}>
            <td>{r.concept}</td>
            <td>{r.unit.label}</td>
            <td className={styles.num}>{r.priceDisplay}</td>
            <td className={styles.actions}>
              <Button aria-label={es.rates.editAria(r.concept)} onClick={() => onEdit(r)}>
                {es.common.edit}
              </Button>
              <Button aria-label={es.rates.deleteAria(r.concept)} onClick={() => onDelete(r)}>
                {es.common.delete}
              </Button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
