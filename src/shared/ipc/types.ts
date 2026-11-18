// Espejo exacto de specs/001-business-profile-rates/contracts/ipc-commands.md,
// specs/002-voice-quote-draft/contracts/ipc-commands.md y
// specs/003-quote-pdf-export/contracts/ipc-commands.md.
// Cualquier cambio se hace primero en el contrato y después aquí y en Rust.

export type ErrorCode =
  | "REQUIRED"
  | "TOO_LONG"
  | "TAX_ID_INVALID"
  | "POSTAL_CODE_INVALID"
  | "PHONE_INVALID"
  | "EMAIL_INVALID"
  | "VAT_OUT_OF_RANGE"
  | "PRICE_REQUIRED"
  | "PRICE_INVALID_FORMAT"
  | "PRICE_TOO_MANY_DECIMALS"
  | "PRICE_NOT_POSITIVE"
  | "UNIT_REQUIRED"
  | "CONCEPT_REQUIRED"
  | "QUANTITY_REQUIRED"
  | "QUANTITY_INVALID_FORMAT"
  | "QUANTITY_TOO_MANY_DECIMALS"
  | "QUANTITY_NOT_POSITIVE"
  | "AMOUNT_TOO_LARGE"
  | "TEXT_REQUIRED"
  | "VALIDITY_INVALID";

export type FieldError = { field: string; code: ErrorCode; message: string };

export type LogoRejectCode = "LOGO_TOO_LARGE" | "LOGO_UNSUPPORTED_FORMAT" | "LOGO_CORRUPT";

export type AppError =
  | { kind: "validation"; errors: FieldError[] }
  | { kind: "duplicateRate"; existingRateId: number; message: string }
  | { kind: "notFound"; message: string }
  | { kind: "logoRejected"; code: LogoRejectCode; message: string }
  | { kind: "storage"; message: string }
  | { kind: "microphone"; code: MicrophoneCode; message: string }
  | { kind: "audioRejected"; code: AudioRejectCode; message: string }
  | { kind: "transcription"; code: TranscriptionCode; message: string }
  | { kind: "confirmationRequired"; reason: ConfirmationReason; message: string }
  | { kind: "invalidState"; message: string }
  | { kind: "draftHasPending"; pendingLineIds: number[]; message: string }
  | { kind: "quoteBlocked"; blockers: QuoteBlocker[]; message: string }
  | { kind: "quoteChanged"; message: string }
  | { kind: "pdfSave"; code: PdfSaveCode; message: string }
  | { kind: "pdfRender"; message: string };

export type MicrophoneCode = "MIC_NOT_FOUND" | "MIC_ACCESS_DENIED" | "MIC_UNAVAILABLE";
export type AudioRejectCode = "AUDIO_UNSUPPORTED_FORMAT" | "AUDIO_CORRUPT" | "AUDIO_TOO_LONG";
export type TranscriptionCode = "MODEL_UNAVAILABLE" | "TRANSCRIPTION_FAILED";
export type ConfirmationReason = "replaceDraft" | "replacePendingAudio" | "discardManualChanges";

export type Address = { street: string; postalCode: string; city: string; province: string };

export type LogoView = {
  dataUrl: string;
  mime: "image/png" | "image/jpeg";
  widthPx: number;
  heightPx: number;
  lowResolution: boolean;
};

export type BusinessProfileView = {
  tradeName: string;
  legalName: string | null;
  taxId: string;
  phone: string;
  email: string | null;
  website: string | null;
  address: Address;
  vatRatePercent: string;
  currency: "EUR";
  logo: LogoView | null;
  status: "complete" | "incomplete";
  missingFields: FieldError[];
};

export type BusinessProfileInput = {
  tradeName: string;
  legalName?: string;
  taxId: string;
  phone: string;
  email?: string;
  website?: string;
  address: Address;
  vatRatePercent: string;
};

export type LogoSetResult = { logo: LogoView; warnings: string[] };

export type Unit = { id: number; label: string; isPredefined: boolean };

export type Rate = {
  id: number;
  concept: string;
  unit: Unit;
  priceCents: number;
  priceDisplay: string;
};

export type RateInput = { concept: string; unitLabel: string; price: string };

// --- Feature 002: dictado y borrador ---

export type SessionActivity =
  | { kind: "idle" }
  | { kind: "recording"; elapsedMs: number }
  | { kind: "processing"; percent: number | null };

export type SessionView = {
  activity: SessionActivity;
  dictation: DictationView | null;
  draft: DraftView | null;
  pendingText: string | null;
  recovery: { durationMs: number } | null;
};

export type DictationView = {
  origin: "recorded" | "imported";
  originalFileName: string | null;
  durationMs: number;
  status: "recorded" | "processing" | "transcribed" | "failed" | "noSpeech";
  lowQuality: boolean;
  errorCode: TranscriptionCode | null;
  transcribedLocally: boolean;
};

export type DraftTotalsView = {
  baseCents: number;
  baseDisplay: string;
  vatRatePercent: string;
  vatCents: number;
  vatDisplay: string;
  totalCents: number;
  totalDisplay: string;
  pendingCount: number;
  isProvisional: boolean;
};

export type DraftView = {
  status: "draft" | "reviewed";
  sourceText: string;
  textOrigin: "transcribed" | "typed";
  textCorrected: boolean;
  hasManualChanges: boolean;
  lines: DraftLineView[];
  unparsedFragments: string[];
  totals: DraftTotalsView;
  vatChange: { currentRatePercent: string } | null;
};

export type QuantityIssue = "missing" | "ambiguous" | "rectified";
export type PendingReason = "noRate" | "ambiguousRate" | "similarRate" | "unitMismatch";

export type DraftLineView = {
  id: number;
  concept: string;
  quantity: string | null;
  quantityIssue: QuantityIssue | null;
  unitLabel: string | null;
  priceCents: number | null;
  priceDisplay: string | null;
  amountCents: number | null;
  amountDisplay: string | null;
  priceSource: "rate" | "manual" | "pending";
  rateId: number | null;
  rateConcept: string | null;
  pendingReason: PendingReason | null;
  suggestions: Rate[];
  sourceFragment: string | null;
};

export type RecordingEvent =
  | { type: "level"; elapsedMs: number; level: number }
  | { type: "limitWarning"; remainingMs: number }
  | { type: "stopped"; reason: "limitReached" | "deviceLost"; durationMs: number };

export type ProcessingStage = "preparing" | "transcribing" | "buildingDraft";
export type ProcessingOutcome = "draftReady" | "noSpeech" | "cancelled" | "failed";

export type ProcessingEvent =
  | { type: "stage"; stage: ProcessingStage }
  | { type: "progress"; percent: number }
  | { type: "finished"; outcome: ProcessingOutcome };

export type ProcessResult = {
  outcome: "draftReady" | "noSpeech" | "cancelled";
  session: SessionView;
};

export type DraftGenerateArgs = {
  text: string;
  mode: "new" | "regenerate";
  confirmed: boolean;
};

export type DraftLineInput =
  | { kind: "rate"; rateId: number; quantity: string }
  | { kind: "manual"; concept: string; quantity: string; unitLabel: string; price: string };

export type DraftLinePatch = {
  concept?: string;
  quantity?: string;
  unitLabel?: string;
  price?: string;
};

// --- Feature 003: PDF del presupuesto ---

export type PdfSaveCode =
  | "PDF_ACCESS_DENIED"
  | "PDF_DISK_FULL"
  | "PDF_LOCATION_UNAVAILABLE"
  | "PDF_FILE_IN_USE"
  | "PDF_WRITE_FAILED";

export type QuoteDetailsInput = {
  clientName: string;
  clientAddress: string | null;
  clientPhone: string | null;
  clientEmail: string | null;
  clientTaxId: string | null;
  quoteNumber: string;
  validityDays: number;
};

export type QuoteBlocker =
  | { kind: "noLines" }
  | { kind: "pendingLines"; lines: { id: number; concept: string }[] }
  | { kind: "notReviewed" }
  | { kind: "profileIncomplete"; missingFields: FieldError[] }
  | { kind: "clientNameRequired" }
  | { kind: "quoteNumberRequired" }
  | {
      kind: "unsupportedCharacters";
      fields: { field: string; label: string; characters: string[] }[];
    }
  | { kind: "logoUnreadable" };

export type QuoteWarning =
  | { kind: "noLogo" }
  | { kind: "logoUnreadable" }
  | { kind: "vatChanged"; appliedRatePercent: string; currentRatePercent: string }
  | { kind: "unparsedFragments"; count: number };

export type QuotePreparationView = {
  details: QuoteDetailsInput;
  issueDateDisplay: string;
  validUntilDisplay: string;
  lineCount: number;
  totalDisplay: string;
  blockers: QuoteBlocker[];
  warnings: QuoteWarning[];
  confirmationToken: string | null;
};

export type QuoteGenerateResult =
  | { outcome: "saved"; fileName: string; fullPath: string; pageCount: number }
  | { outcome: "cancelled" };
