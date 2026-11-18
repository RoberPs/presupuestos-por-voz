import { useEffect, useId, useState } from "react";
import { Button } from "../../shared/components/Button";
import { es } from "../../shared/i18n/es";
import type { DraftLinePatch, DraftLineView, FieldError } from "../../shared/ipc/types";
import styles from "./DraftPage.module.css";

const t = es.draft.lines;

type CellProps = {
  label: string;
  value: string;
  error?: string;
  numeric?: boolean;
  onCommit: (value: string) => void;
};

/** Campo que guarda al salir de él o al pulsar Intro, solo si el valor ha cambiado. */
function EditableCell({ label, value, error, numeric, onCommit }: CellProps) {
  const [text, setText] = useState(value);
  const errorId = useId();
  useEffect(() => setText(value), [value]);

  const commit = () => {
    if (text.trim() !== value) onCommit(text);
  };

  return (
    <>
      <input
        className={numeric ? `${styles.cellInput} ${styles.cellNumber}` : styles.cellInput}
        aria-label={label}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errorId : undefined}
        inputMode={numeric ? "decimal" : undefined}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
        }}
      />
      {error ? (
        <span id={errorId} className={styles.fieldError}>
          {error}
        </span>
      ) : null}
    </>
  );
}

type Props = {
  line: DraftLineView;
  position: number;
  errors: FieldError[];
  highlighted: boolean;
  onUpdate: (patch: DraftLinePatch) => void;
  onAssignRate: (rateId: number) => void;
  onDelete: () => void;
};

export function DraftLineRow({
  line,
  position,
  errors,
  highlighted,
  onUpdate,
  onAssignRate,
  onDelete,
}: Props) {
  const pricePending = line.priceSource === "pending";
  const quantityPending = line.quantity === null;
  const errorFor = (field: string) => errors.find((e) => e.field === field)?.message;
  const rowClass = pricePending || quantityPending ? styles.pendingRow : undefined;

  return (
    <tr className={rowClass}>
      <td>
        <EditableCell
          label={t.conceptAria(position)}
          value={line.concept}
          error={errorFor("concept")}
          onCommit={(concept) => onUpdate({ concept })}
        />
        {highlighted ? (
          <span className={styles.fieldError}>{es.draft.review.linePending}</span>
        ) : null}
        {pricePending ? <span className={styles.mark}>{t.pricePending}</span> : null}
        {quantityPending ? <span className={styles.mark}>{t.quantityPending}</span> : null}
        {line.priceSource === "rate" && line.rateConcept ? (
          <span className={styles.detail}>{t.fromRate(line.rateConcept)}</span>
        ) : null}
        {line.priceSource === "manual" ? (
          <span className={styles.detail}>{t.manualPrice}</span>
        ) : null}
        {line.pendingReason ? (
          <span className={styles.detail}>{t.pendingReasons[line.pendingReason]}</span>
        ) : null}
        {line.quantityIssue ? (
          <span className={styles.detail}>{t.quantityIssues[line.quantityIssue]}</span>
        ) : null}
        {(pricePending || quantityPending) && line.sourceFragment ? (
          <span className={styles.detail}>{t.fragment(line.sourceFragment)}</span>
        ) : null}
        {line.suggestions.length > 0 ? (
          <div className={styles.suggestions}>
            <span className={styles.detail}>{t.suggestions}</span>
            <div className={styles.suggestionList}>
              {line.suggestions.map((rate) => (
                <Button key={rate.id} onClick={() => onAssignRate(rate.id)}>
                  {t.useSuggestion(rate.concept, rate.unit.label, rate.priceDisplay)}
                </Button>
              ))}
            </div>
          </div>
        ) : null}
      </td>
      <td className={styles.num}>
        <EditableCell
          numeric
          label={t.quantityAria(line.concept)}
          value={line.quantity ?? ""}
          error={errorFor("quantity")}
          onCommit={(quantity) => onUpdate({ quantity })}
        />
      </td>
      <td>
        <EditableCell
          label={t.unitAria(line.concept)}
          value={line.unitLabel ?? ""}
          error={errorFor("unitLabel")}
          onCommit={(unitLabel) => onUpdate({ unitLabel })}
        />
      </td>
      <td className={styles.num}>
        <EditableCell
          numeric
          label={t.priceAria(line.concept)}
          value={line.priceDisplay?.replace(" €", "") ?? ""}
          error={errorFor("price")}
          onCommit={(price) => onUpdate({ price })}
        />
      </td>
      <td className={styles.num}>{line.amountDisplay}</td>
      <td>
        <Button aria-label={t.deleteAria(line.concept)} onClick={onDelete}>
          {es.common.delete}
        </Button>
      </td>
    </tr>
  );
}
