import { mockIPC } from "@tauri-apps/api/mocks";
import type { QuoteDetailsInput, QuotePreparationView } from "../shared/ipc/types";

export const detailsFixture = (over: Partial<QuoteDetailsInput> = {}): QuoteDetailsInput => ({
  clientName: "Ana Ruiz",
  clientAddress: null,
  clientPhone: null,
  clientEmail: null,
  clientTaxId: null,
  quoteNumber: "2026-001",
  validityDays: 30,
  ...over,
});

export const preparationFixture = (
  over: Partial<QuotePreparationView> = {},
): QuotePreparationView => ({
  details: detailsFixture(),
  issueDateDisplay: "18/11/2026",
  validUntilDisplay: "18/12/2026",
  lineCount: 2,
  totalDisplay: "163,35 €",
  blockers: [],
  warnings: [],
  confirmationToken: "token-1",
  ...over,
});

type Handler = (args: Record<string, unknown>) => unknown;

/** Backend simulado del diálogo del PDF. Cada comando se puede sustituir. */
export function mockQuoteBackend(overrides: Record<string, Handler> = {}) {
  const calls: { cmd: string; args: Record<string, unknown> }[] = [];
  const handlers: Record<string, Handler> = {
    quote_prepare: () => preparationFixture(),
    quote_details_save: (args) =>
      preparationFixture({
        details: args.details as QuoteDetailsInput,
        confirmationToken: "token-guardado",
      }),
    quote_generate: () => ({
      outcome: "saved",
      fileName: "Presupuesto 2026-001 2026-11-18.pdf",
      fullPath: "C:\\Presupuestos\\Presupuesto 2026-001 2026-11-18.pdf",
      pageCount: 1,
    }),
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
    called: (cmd: string) => calls.filter((c) => c.cmd === cmd),
  };
}
