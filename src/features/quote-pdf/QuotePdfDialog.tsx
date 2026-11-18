import { useRef, useState } from "react";
import { useForm } from "react-hook-form";
import { useNavigate } from "react-router-dom";
import { Banner } from "../../shared/components/Banner";
import { ConfirmDialog } from "../../shared/components/ConfirmDialog";
import { es } from "../../shared/i18n/es";
import { draftApplyCurrentVat } from "../../shared/ipc/client";
import type { QuoteDetailsInput, QuotePreparationView } from "../../shared/ipc/types";
import { QuoteBlockers } from "./QuoteBlockers";
import { QuoteDetailsForm, type QuoteFormValues } from "./QuoteDetailsForm";
import { QuotePdfSaved, QuotePdfSaveError } from "./QuotePdfResult";
import { useQuotePdf } from "./useQuotePdf";
import styles from "./QuotePdf.module.css";

const t = es.quotePdf;

type Props = {
  onClose: () => void;
  /** Actualiza el IVA del borrador, para que la pantalla de detrás muestre los importes nuevos. */
  onApplyVat?: () => Promise<unknown>;
};

const toFormValues = (details: QuoteDetailsInput): QuoteFormValues => ({
  clientName: details.clientName,
  clientAddress: details.clientAddress ?? "",
  clientPhone: details.clientPhone ?? "",
  clientEmail: details.clientEmail ?? "",
  clientTaxId: details.clientTaxId ?? "",
  quoteNumber: details.quoteNumber,
  validityDays: String(details.validityDays),
});

const emptyToNull = (value: string) => (value.trim() === "" ? null : value);

/** `null` si la validez no es un número entero: el núcleo no puede recibirla. */
const toDetails = (values: QuoteFormValues): QuoteDetailsInput | null => {
  const text = values.validityDays.trim();
  if (!/^-?\d+$/.test(text)) return null;
  return {
    clientName: values.clientName,
    clientAddress: emptyToNull(values.clientAddress),
    clientPhone: emptyToNull(values.clientPhone),
    clientEmail: emptyToNull(values.clientEmail),
    clientTaxId: emptyToNull(values.clientTaxId),
    quoteNumber: values.quoteNumber,
    validityDays: Number(text),
  };
};

type ConfirmationProps = Required<Props> & {
  quote: ReturnType<typeof useQuotePdf>;
  preparation: QuotePreparationView;
};

/** Se monta con los datos ya cargados, para que el formulario nunca aparezca vacío. */
function QuotePdfConfirmation({ quote, preparation, onClose, onApplyVat }: ConfirmationProps) {
  const navigate = useNavigate();
  const initialValues = useRef(toFormValues(preparation.details));
  const form = useForm<QuoteFormValues>({ defaultValues: initialValues.current });
  const [validityError, setValidityError] = useState<string | undefined>();
  const [showBlockers, setShowBlockers] = useState(false);
  const lastSaved = useRef(JSON.stringify(toDetails(initialValues.current)));

  const readDetails = () => {
    const details = toDetails(form.getValues());
    setValidityError(details === null ? t.form.validityInvalid : undefined);
    return details;
  };

  const saveOnBlur = () => {
    const details = readDetails();
    if (details === null) return;
    const serialized = JSON.stringify(details);
    if (serialized === lastSaved.current) return;
    lastSaved.current = serialized;
    void quote.saveDetails(details);
  };

  const confirm = () => {
    const details = readDetails();
    if (details === null) return;
    lastSaved.current = JSON.stringify(details);
    setShowBlockers(true);
    void quote.confirm(details);
  };

  return (
    <ConfirmDialog
      open
      title={t.title}
      onDismiss={onClose}
      actions={[
        { label: es.common.cancel, onClick: onClose },
        {
          label: quote.generating ? t.generating : t.confirm,
          onClick: confirm,
          variant: "primary",
          disabled: quote.generating,
        },
      ]}
    >
      <dl className={styles.summary} aria-label={t.summary.heading}>
        <dt>{t.summary.issueDate}</dt>
        <dd>{preparation.issueDateDisplay}</dd>
        <dt>{t.summary.validUntil}</dt>
        <dd>{preparation.validUntilDisplay}</dd>
        <dt>{t.summary.lines}</dt>
        <dd>{preparation.lineCount}</dd>
        <dt>{t.summary.total}</dt>
        <dd>{preparation.totalDisplay}</dd>
      </dl>

      <QuoteBlockers
        blockers={preparation.blockers}
        warnings={preparation.warnings}
        showFormBlockers={showBlockers}
        onGoToLines={onClose}
        onGoToBusiness={() => navigate("/negocio")}
        onContinueWithoutLogo={quote.continueWithoutLogo}
        onApplyVat={() => void quote.applyVat(onApplyVat)}
      />

      {quote.saveError ? (
        <QuotePdfSaveError
          message={quote.saveError}
          retrying={quote.generating}
          onRetry={() => void quote.retry()}
        />
      ) : null}

      {quote.failure ? (
        <Banner variant="warning" role="alert">
          {quote.failure}
        </Banner>
      ) : null}

      <QuoteDetailsForm
        register={form.register}
        errors={quote.fieldErrors}
        validityError={validityError}
        onFieldBlur={saveOnBlur}
      />
    </ConfirmDialog>
  );
}

/** Resumen, datos del presupuesto y confirmación explícita antes de crear el archivo (FR-004). */
export function QuotePdfDialog({ onClose, onApplyVat = draftApplyCurrentVat }: Props) {
  const quote = useQuotePdf();

  if (quote.saved !== null) {
    return (
      <ConfirmDialog
        open
        title={t.title}
        onDismiss={onClose}
        actions={[{ label: t.result.close, onClick: onClose, variant: "primary" }]}
      >
        <QuotePdfSaved saved={quote.saved} />
      </ConfirmDialog>
    );
  }

  if (quote.preparation === null) {
    return (
      <ConfirmDialog
        open
        title={t.title}
        onDismiss={onClose}
        actions={[{ label: es.common.cancel, onClick: onClose }]}
      >
        <p>{quote.failure ?? t.loading}</p>
      </ConfirmDialog>
    );
  }

  return (
    <QuotePdfConfirmation
      quote={quote}
      preparation={quote.preparation}
      onClose={onClose}
      onApplyVat={onApplyVat}
    />
  );
}
