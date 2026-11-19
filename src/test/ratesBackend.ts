import { mockIPC } from "@tauri-apps/api/mocks";
import type { AppError, Rate, RateInput, Unit } from "../shared/ipc/types";
import { profileFixture } from "./fixtures";

const norm = (s: string) =>
  s
    .normalize("NFD")
    .replace(/\p{Mn}/gu, "")
    .toLowerCase()
    .trim()
    .replace(/\s+/g, " ");

const eur = (cents: number) =>
  `${(cents / 100).toLocaleString("es-ES", { minimumFractionDigits: 2, maximumFractionDigits: 2, useGrouping: true })} €`;

/** Backend simulado en memoria para pruebas de la pantalla de tarifas. */
export function mockRatesBackend(initial: { concept: string; unit: string; price: number }[] = []) {
  const units: Unit[] = ["unidad", "hora", "metro", "m²", "metro lineal"].map((label, i) => ({
    id: i + 1,
    label,
    isPredefined: true,
  }));
  let rates: Rate[] = [];
  let nextId = 1;
  const calls: { cmd: string; args: Record<string, unknown> }[] = [];

  const unitFor = (label: string) => {
    const found = units.find((u) => norm(u.label) === norm(label));
    if (found) return found;
    const u = { id: units.length + 1, label: label.trim(), isPredefined: false };
    units.push(u);
    return u;
  };
  const build = (id: number, input: RateInput): Rate => {
    const cents = Math.round(Number(input.price.replace(".", "").replace(",", ".")) * 100);
    return {
      id,
      concept: input.concept.trim(),
      unit: unitFor(input.unitLabel),
      priceCents: cents,
      priceDisplay: eur(cents),
    };
  };
  const sorted = () => [...rates].sort((a, b) => norm(a.concept).localeCompare(norm(b.concept)));

  for (const r of initial) {
    rates.push(build(nextId++, { concept: r.concept, unitLabel: r.unit, price: String(r.price) }));
  }

  const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
    profile_get: () => profileFixture(),
    units_list: () => units,
    rates_list: (args) => {
      const q = typeof args.query === "string" ? norm(args.query) : "";
      return sorted().filter((r) => !q || norm(r.concept).includes(q));
    },
    rates_create: (args) => {
      const input = args.input as RateInput;
      const dup = rates.find(
        (r) =>
          norm(r.concept) === norm(input.concept) && norm(r.unit.label) === norm(input.unitLabel),
      );
      if (dup) {
        const err: AppError = {
          kind: "duplicateRate",
          existingRateId: dup.id,
          message: `Ya existe la tarifa «${dup.concept}» por ${dup.unit.label}.`,
        };
        throw err;
      }
      const rate = build(nextId++, input);
      rates.push(rate);
      return rate;
    },
    rates_update: (args) => {
      const id = args.id as number;
      const rate = build(id, args.input as RateInput);
      rates = rates.map((r) => (r.id === id ? rate : r));
      return rate;
    },
    rates_delete: (args) => {
      rates = rates.filter((r) => r.id !== args.id);
      return null;
    },
  };

  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    const h = handlers[cmd];
    return h ? h(a) : null;
  });

  return { calls, handlers, getRates: () => sorted() };
}
