// Espejo exacto de specs/001-business-profile-rates/contracts/ipc-commands.md.
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
  | "CONCEPT_REQUIRED";

export type FieldError = { field: string; code: ErrorCode; message: string };

export type LogoRejectCode = "LOGO_TOO_LARGE" | "LOGO_UNSUPPORTED_FORMAT" | "LOGO_CORRUPT";

export type AppError =
  | { kind: "validation"; errors: FieldError[] }
  | { kind: "duplicateRate"; existingRateId: number; message: string }
  | { kind: "notFound"; message: string }
  | { kind: "logoRejected"; code: LogoRejectCode; message: string }
  | { kind: "storage"; message: string };

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
