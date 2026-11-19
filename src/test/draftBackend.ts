import { act } from "@testing-library/react";
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Channel } from "@tauri-apps/api/core";
import type {
  DraftLineView,
  DraftTotalsView,
  DraftView,
  ProcessingEvent,
  Rate,
  RecordingEvent,
  SessionView,
} from "../shared/ipc/types";
import { profileFixture } from "./fixtures";

export const totalsFixture = (over: Partial<DraftTotalsView> = {}): DraftTotalsView => ({
  baseCents: 13500,
  baseDisplay: "135,00 €",
  vatRatePercent: "21,00",
  vatCents: 2835,
  vatDisplay: "28,35 €",
  totalCents: 16335,
  totalDisplay: "163,35 €",
  pendingCount: 0,
  isProvisional: false,
  ...over,
});

export const lineFixture = (over: Partial<DraftLineView> = {}): DraftLineView => ({
  id: 1,
  concept: "Bajante",
  quantity: "3",
  quantityIssue: null,
  unitLabel: "metro",
  priceCents: 2500,
  priceDisplay: "25,00 €",
  amountCents: 7500,
  amountDisplay: "75,00 €",
  priceSource: "rate",
  rateId: 1,
  rateConcept: "Bajante",
  pendingReason: null,
  suggestions: [],
  sourceFragment: "cambiar bajante, 3 metros",
  ...over,
});

export const pricePendingFixture = (over: Partial<DraftLineView> = {}): DraftLineView =>
  lineFixture({
    id: 2,
    concept: "cambiar grifo",
    quantity: "2",
    unitLabel: "unidad",
    priceCents: null,
    priceDisplay: null,
    amountCents: null,
    amountDisplay: null,
    priceSource: "pending",
    rateId: null,
    rateConcept: null,
    pendingReason: "noRate",
    sourceFragment: "cambiar grifo, 2 unidades",
    ...over,
  });

export const rateFixture = (over: Partial<Rate> = {}): Rate => ({
  id: 7,
  concept: "Bajante de hierro",
  unit: { id: 3, label: "metro", isPredefined: true },
  priceCents: 4000,
  priceDisplay: "40,00 €",
  ...over,
});

export const draftFixture = (over: Partial<DraftView> = {}): DraftView => ({
  status: "draft",
  sourceText: "cambiar bajante, 3 metros, mano de obra 2 horas",
  textOrigin: "transcribed",
  textCorrected: false,
  hasManualChanges: false,
  lines: [
    lineFixture(),
    lineFixture({
      id: 2,
      concept: "Mano de obra",
      quantity: "2",
      unitLabel: "hora",
      priceCents: 3000,
      priceDisplay: "30,00 €",
      amountCents: 6000,
      amountDisplay: "60,00 €",
      rateId: 2,
      rateConcept: "Mano de obra",
      sourceFragment: "mano de obra 2 horas",
    }),
  ],
  unparsedFragments: [],
  totals: totalsFixture(),
  vatChange: null,
  ...over,
});

export const sessionFixture = (over: Partial<SessionView> = {}): SessionView => ({
  activity: { kind: "idle" },
  dictation: null,
  draft: null,
  pendingText: null,
  recovery: null,
  ...over,
});

export const recordedSession = (): SessionView =>
  sessionFixture({
    dictation: {
      origin: "recorded",
      originalFileName: null,
      durationMs: 5000,
      status: "recorded",
      lowQuality: false,
      errorCode: null,
      transcribedLocally: false,
    },
  });

export const transcribedSession = (over: Partial<SessionView> = {}): SessionView =>
  sessionFixture({
    dictation: {
      origin: "recorded",
      originalFileName: null,
      durationMs: 5000,
      status: "transcribed",
      lowQuality: false,
      errorCode: null,
      transcribedLocally: true,
    },
    draft: draftFixture(),
    ...over,
  });

type Handler = (args: Record<string, unknown>) => unknown;

/** Backend simulado de la pantalla «Presupuesto». Cada comando se puede sustituir. */
export function mockDraftBackend(
  initial: SessionView = sessionFixture(),
  overrides: Record<string, Handler> = {},
) {
  const calls: { cmd: string; args: Record<string, unknown> }[] = [];
  const channels: {
    recording?: Channel<RecordingEvent>;
    processing?: Channel<ProcessingEvent>;
  } = {};

  const handlers: Record<string, Handler> = {
    profile_get: () => profileFixture(),
    rates_list: () => [rateFixture()],
    units_list: () => [],
    session_get: () => initial,
    dictation_start: (args) => {
      channels.recording = args.onEvent as Channel<RecordingEvent>;
      return null;
    },
    dictation_stop: () => recordedSession(),
    dictation_process: (args) => {
      channels.processing = args.onProgress as Channel<ProcessingEvent>;
      return { outcome: "draftReady", session: transcribedSession() };
    },
    dictation_subscribe: (args) => {
      channels.recording = args.onRecording as Channel<RecordingEvent>;
      channels.processing = args.onProgress as Channel<ProcessingEvent>;
      return null;
    },
    dictation_cancel_processing: () => null,
    dictation_import: () => recordedSession(),
    dictation_recover: () => recordedSession(),
    dictation_discard: () => sessionFixture(),
    ...overrides,
  };

  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    const handler = handlers[cmd];
    return handler ? handler(a) : null;
  });

  return {
    calls,
    handlers,
    called: (cmd: string) => calls.filter((c) => c.cmd === cmd),
    emitRecording: (event: RecordingEvent) => act(() => channels.recording?.onmessage(event)),
    emitProcessing: (event: ProcessingEvent) => act(() => channels.processing?.onmessage(event)),
  };
}
