import { invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  BusinessProfileInput,
  BusinessProfileView,
  FieldError,
  LogoSetResult,
  Rate,
  RateInput,
  Unit,
} from "./types";

const APP_ERROR_KINDS = ["validation", "duplicateRate", "notFound", "logoRejected", "storage"];

export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    APP_ERROR_KINDS.includes((e as { kind: unknown }).kind as string)
  );
}

export const profileGet = () => invoke<BusinessProfileView | null>("profile_get");
export const profileSave = (input: BusinessProfileInput) =>
  invoke<BusinessProfileView>("profile_save", { input });
export const profileValidate = (input: BusinessProfileInput) =>
  invoke<FieldError[]>("profile_validate", { input });
export const logoPickAndSet = () => invoke<LogoSetResult | null>("logo_pick_and_set");
export const logoRemove = () => invoke<BusinessProfileView>("logo_remove");

export const unitsList = () => invoke<Unit[]>("units_list");

export const ratesList = (query?: string) => invoke<Rate[]>("rates_list", { query: query ?? null });
export const ratesCreate = (input: RateInput) => invoke<Rate>("rates_create", { input });
export const ratesUpdate = (id: number, input: RateInput) =>
  invoke<Rate>("rates_update", { id, input });
export const ratesDelete = (id: number) => invoke<void>("rates_delete", { id });
