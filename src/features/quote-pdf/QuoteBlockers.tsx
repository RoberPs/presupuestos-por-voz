import { Banner } from "../../shared/components/Banner";
import { Button } from "../../shared/components/Button";
import { es } from "../../shared/i18n/es";
import type { QuoteBlocker, QuoteWarning } from "../../shared/ipc/types";
import styles from "./QuotePdf.module.css";

const t = es.quotePdf;
const fields = es.business.fields;

const PROFILE_FIELD_LABELS: Record<string, string> = {
  tradeName: fields.tradeName,
  legalName: fields.legalName,
  taxId: fields.taxId,
  phone: fields.phone,
  email: fields.email,
  website: fields.website,
  "address.street": fields.street,
  "address.postalCode": fields.postalCode,
  "address.city": fields.city,
  "address.province": fields.province,
};

type Props = {
  blockers: QuoteBlocker[];
  warnings: QuoteWarning[];
  /** El nombre y el número se señalan cuando el profesional intenta confirmar. */
  showFormBlockers: boolean;
  onGoToLines: () => void;
  onGoToBusiness: () => void;
  onContinueWithoutLogo: () => void;
  onApplyVat: () => void;
};

/** Motivos que impiden generar el PDF y avisos que no lo impiden (FR-002, FR-003 y FR-005). */
export function QuoteBlockers({
  blockers,
  warnings,
  showFormBlockers,
  onGoToLines,
  onGoToBusiness,
  onContinueWithoutLogo,
  onApplyVat,
}: Props) {
  const visible = blockers.filter(
    (blocker) =>
      showFormBlockers ||
      (blocker.kind !== "clientNameRequired" && blocker.kind !== "quoteNumberRequired"),
  );

  const renderBlocker = (blocker: QuoteBlocker) => {
    switch (blocker.kind) {
      case "noLines":
        return (
          <>
            <p>{t.blockers.noLines}</p>
            <Button onClick={onGoToLines}>{t.blockers.goToLines}</Button>
          </>
        );
      case "pendingLines":
        return (
          <>
            <p>{t.blockers.pendingLines}</p>
            <ul className={styles.reasons}>
              {blocker.lines.map((line) => (
                <li key={line.id}>{line.concept}</li>
              ))}
            </ul>
            <Button onClick={onGoToLines}>{t.blockers.goToLines}</Button>
          </>
        );
      case "notReviewed":
        return (
          <>
            <p>{t.blockers.notReviewed}</p>
            <Button onClick={onGoToLines}>{t.blockers.goToReview}</Button>
          </>
        );
      case "profileIncomplete":
        return (
          <>
            <p>{t.blockers.profileIncomplete}</p>
            <ul className={styles.reasons}>
              {blocker.missingFields.map((field) => (
                <li key={field.field}>{PROFILE_FIELD_LABELS[field.field] ?? field.field}</li>
              ))}
            </ul>
            <Button onClick={onGoToBusiness}>{t.blockers.goToBusiness}</Button>
          </>
        );
      case "clientNameRequired":
        return <p>{t.blockers.clientNameRequired}</p>;
      case "quoteNumberRequired":
        return <p>{t.blockers.quoteNumberRequired}</p>;
      case "unsupportedCharacters":
        return (
          <>
            <p>{t.blockers.unsupportedCharacters}</p>
            <ul className={styles.reasons}>
              {blocker.fields.map((field) => (
                <li key={field.field}>
                  {t.blockers.unsupportedField(field.label, field.characters.join(" "))}
                </li>
              ))}
            </ul>
          </>
        );
      case "logoUnreadable":
        return (
          <>
            <p>{t.blockers.logoUnreadable}</p>
            <div className={styles.choices}>
              <Button onClick={onContinueWithoutLogo}>{t.blockers.continueWithoutLogo}</Button>
              <Button onClick={onGoToBusiness}>{t.blockers.goToBusiness}</Button>
            </div>
          </>
        );
    }
  };

  const renderWarning = (warning: QuoteWarning) => {
    switch (warning.kind) {
      case "noLogo":
        return <p>{t.warnings.noLogo}</p>;
      case "logoUnreadable":
        return <p>{t.warnings.logoUnreadable}</p>;
      case "vatChanged":
        return (
          <>
            <p>{t.warnings.vatChanged(warning.appliedRatePercent, warning.currentRatePercent)}</p>
            <Button onClick={onApplyVat}>{t.warnings.applyVat}</Button>
          </>
        );
      case "unparsedFragments":
        return <p>{t.warnings.unparsedFragments(warning.count)}</p>;
    }
  };

  if (visible.length === 0 && warnings.length === 0) return null;

  return (
    <div className={styles.notices}>
      {visible.length > 0 ? (
        <Banner variant="warning" role="alert">
          <strong>{t.blockers.heading}</strong>
          {visible.map((blocker) => (
            <div key={blocker.kind} className={styles.notice}>
              {renderBlocker(blocker)}
            </div>
          ))}
        </Banner>
      ) : null}
      {warnings.map((warning) => (
        <Banner key={warning.kind} variant="warning">
          <div className={styles.notice}>{renderWarning(warning)}</div>
        </Banner>
      ))}
    </div>
  );
}
