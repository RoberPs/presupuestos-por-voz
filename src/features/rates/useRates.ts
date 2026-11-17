import { useCallback, useEffect, useRef, useState } from "react";
import {
  isAppError,
  ratesCreate,
  ratesDelete,
  ratesList,
  ratesUpdate,
  unitsList,
} from "../../shared/ipc/client";
import type { AppError, Rate, RateInput, Unit } from "../../shared/ipc/types";
import { es } from "../../shared/i18n/es";

function toAppError(e: unknown): AppError {
  return isAppError(e) ? e : { kind: "storage", message: es.app.genericError };
}

/** Tarifas y unidades. Con `query` filtra en el núcleo (sin distinguir mayúsculas ni tildes). */
export function useRates(query = "") {
  const [rates, setRates] = useState<Rate[]>([]);
  const [units, setUnits] = useState<Unit[]>([]);
  const [loading, setLoading] = useState(true);
  const [lastError, setLastError] = useState<AppError | null>(null);
  const queryRef = useRef(query);
  queryRef.current = query;
  const seq = useRef(0);

  const reload = useCallback(async () => {
    const current = ++seq.current;
    try {
      const q = queryRef.current.trim();
      const [r, u] = await Promise.all([ratesList(q || undefined), unitsList()]);
      if (current !== seq.current) return;
      setRates(r);
      setUnits(u);
      setLastError(null);
    } catch (e) {
      if (current === seq.current) setLastError(toAppError(e));
    } finally {
      if (current === seq.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload, query]);

  const mutate = useCallback(
    async <T>(op: () => Promise<T>): Promise<T> => {
      try {
        const result = await op();
        await reload();
        return result;
      } catch (e) {
        throw toAppError(e);
      }
    },
    [reload],
  );

  const create = useCallback((input: RateInput) => mutate(() => ratesCreate(input)), [mutate]);
  const update = useCallback(
    (id: number, input: RateInput) => mutate(() => ratesUpdate(id, input)),
    [mutate],
  );
  const remove = useCallback((id: number) => mutate(() => ratesDelete(id)), [mutate]);

  return { rates, units, loading, lastError, reload, create, update, remove };
}
