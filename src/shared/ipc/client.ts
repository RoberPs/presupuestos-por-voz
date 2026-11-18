import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  BusinessProfileInput,
  BusinessProfileView,
  DraftGenerateArgs,
  DraftLineInput,
  DraftLinePatch,
  DraftView,
  FieldError,
  LogoSetResult,
  ProcessingEvent,
  ProcessResult,
  Rate,
  RateInput,
  RecordingEvent,
  SessionView,
  Unit,
} from "./types";

const APP_ERROR_KINDS = [
  "validation",
  "duplicateRate",
  "notFound",
  "logoRejected",
  "storage",
  "microphone",
  "audioRejected",
  "transcription",
  "confirmationRequired",
  "invalidState",
  "draftHasPending",
];

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

export const sessionGet = () => invoke<SessionView>("session_get");

export const dictationStart = (confirmReplace: boolean, onEvent: Channel<RecordingEvent>) =>
  invoke<void>("dictation_start", { confirmReplace, onEvent });
export const dictationStop = () => invoke<SessionView>("dictation_stop");
export const dictationImport = (confirmReplace: boolean) =>
  invoke<SessionView | null>("dictation_import", { confirmReplace });
export const dictationProcess = (onProgress: Channel<ProcessingEvent>) =>
  invoke<ProcessResult>("dictation_process", { onProgress });
export const dictationCancelProcessing = () => invoke<void>("dictation_cancel_processing");
export const dictationSubscribe = (
  onRecording: Channel<RecordingEvent>,
  onProgress: Channel<ProcessingEvent>,
) => invoke<void>("dictation_subscribe", { onRecording, onProgress });
export const dictationRecover = () => invoke<SessionView>("dictation_recover");
export const dictationDiscard = () => invoke<SessionView>("dictation_discard");

export const draftGenerateFromText = (args: DraftGenerateArgs) =>
  invoke<DraftView>("draft_generate_from_text", args);
export const draftTextSave = (text: string) => invoke<void>("draft_text_save", { text });
export const draftLineAdd = (input: DraftLineInput) =>
  invoke<DraftView>("draft_line_add", { input });
export const draftLineUpdate = (lineId: number, patch: DraftLinePatch) =>
  invoke<DraftView>("draft_line_update", { lineId, patch });
export const draftLineAssignRate = (lineId: number, rateId: number) =>
  invoke<DraftView>("draft_line_assign_rate", { lineId, rateId });
export const draftLineDelete = (lineId: number) =>
  invoke<DraftView>("draft_line_delete", { lineId });
export const draftApplyCurrentVat = () => invoke<DraftView>("draft_apply_current_vat");
export const draftMarkReviewed = () => invoke<DraftView>("draft_mark_reviewed");
export const draftDiscard = () => invoke<SessionView>("draft_discard");
