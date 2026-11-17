import { forwardRef, useId, type InputHTMLAttributes } from "react";
import type { Unit } from "../../shared/ipc/types";

type Props = Omit<InputHTMLAttributes<HTMLInputElement>, "list"> & { units: Unit[] };

/**
 * Combobox accesible (input + datalist): propone las unidades guardadas y deja escribir una nueva.
 * Recibe id y atributos aria desde `Field`.
 */
export const UnitSelect = forwardRef<HTMLInputElement, Props>(function UnitSelect(
  { units, ...rest },
  ref,
) {
  const listId = useId();
  return (
    <>
      <input ref={ref} list={listId} autoComplete="off" {...rest} />
      <datalist id={listId}>
        {units.map((u) => (
          <option key={u.id} value={u.label} />
        ))}
      </datalist>
    </>
  );
});
