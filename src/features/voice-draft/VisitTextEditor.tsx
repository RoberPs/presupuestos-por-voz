import { useEffect, useId, useRef, useState } from "react";
import { Button } from "../../shared/components/Button";
import { es } from "../../shared/i18n/es";
import type { DraftSession } from "./useDraftSession";
import styles from "./DraftPage.module.css";

const t = es.draft.text;
/** Espera tras dejar de escribir antes de guardar. */
const SAVE_DELAY_MS = 500;

type Props = { session: DraftSession };

/** Texto de la visita: se corrige y regenera, o se escribe a mano sin dictar (FR-011 y FR-013). */
export function VisitTextEditor({ session }: Props) {
  const draft = session.session?.draft ?? null;
  const initial = draft?.sourceText ?? session.session?.pendingText ?? "";
  const fieldId = useId();
  const errorId = useId();

  const [text, setText] = useState(initial);
  const saved = useRef(initial);
  const latest = useRef(initial);
  latest.current = text;
  const { saveText } = session;

  // Un texto nuevo del núcleo (dictado o regeneración) sustituye al que se muestra.
  useEffect(() => {
    saved.current = initial;
    setText(initial);
  }, [initial]);

  useEffect(() => {
    if (text === saved.current) return;
    const timer = setTimeout(() => {
      saved.current = text;
      void saveText(text);
    }, SAVE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [saveText, text]);

  // Al salir de la pantalla se guarda lo último escrito (FR-035).
  useEffect(
    () => () => {
      if (latest.current !== saved.current) void saveText(latest.current);
    },
    [saveText],
  );

  const generate = () => {
    saved.current = text;
    void session.generateDraft(text, draft ? "regenerate" : "new");
  };

  return (
    <section className={styles.panel} aria-labelledby="text-heading">
      <h2 id="text-heading">{t.heading}</h2>
      <label htmlFor={fieldId} className={styles.textLabel}>
        {t.label}
      </label>
      <textarea
        id={fieldId}
        className={styles.textArea}
        rows={5}
        placeholder={t.placeholder}
        value={text}
        aria-invalid={session.textError ? true : undefined}
        aria-describedby={session.textError ? errorId : undefined}
        onChange={(e) => setText(e.target.value)}
      />
      {session.textError ? (
        <p id={errorId} className={styles.fieldError} aria-live="polite">
          {session.textError}
        </p>
      ) : null}
      {draft ? <p className={styles.hint}>{t.help}</p> : null}
      <div>
        <Button variant={draft ? "secondary" : "primary"} onClick={generate}>
          {draft ? t.regenerate : t.generate}
        </Button>
      </div>
    </section>
  );
}
