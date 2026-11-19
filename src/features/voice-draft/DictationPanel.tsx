import { useEffect, useRef } from "react";
import { Banner } from "../../shared/components/Banner";
import { Button } from "../../shared/components/Button";
import { ConfirmDialog } from "../../shared/components/ConfirmDialog";
import { es } from "../../shared/i18n/es";
import type { ConfirmationReason } from "../../shared/ipc/types";
import type { DraftSession } from "./useDraftSession";
import styles from "./DraftPage.module.css";

const t = es.draft.dictation;

const CONFIRM_TITLES: Record<ConfirmationReason, string> = {
  replaceDraft: es.draft.confirm.replaceDraftTitle,
  replacePendingAudio: es.draft.confirm.replacePendingAudioTitle,
  discardManualChanges: es.draft.confirm.discardManualChangesTitle,
};

/** `83000` → `"1:23"`. */
export function formatDuration(ms: number) {
  const seconds = Math.floor(ms / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

type Props = { dictation: DraftSession; onWrite: () => void };

export function DictationPanel({ dictation: d, onWrite }: Props) {
  const { activity, session, notice, failure, confirmation } = d;
  const recording = activity.kind === "recording";
  const processing = activity.kind === "processing";
  const idle = activity.kind === "idle";
  const current = session?.dictation ?? null;
  const noSpeech = idle && (notice === "noSpeech" || current?.status === "noSpeech");

  // F9 empieza y para la grabación desde cualquier punto de la pantalla (R12).
  const shortcut = useRef<() => void>(() => {});
  shortcut.current = () => {
    if (confirmation) return;
    if (recording) void d.stopRecording();
    else if (idle) void d.startRecording();
  };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "F9") return;
      e.preventDefault();
      shortcut.current();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <section className={styles.panel} aria-labelledby="dictation-heading">
      <h2 id="dictation-heading">{t.heading}</h2>

      {failure ? (
        <Banner variant="warning" role="alert">
          {failure.message}
          {failure.microphone ? <span className={styles.block}>{t.alternatives}</span> : null}
        </Banner>
      ) : null}

      {notice === "deviceLost" ? <Banner variant="warning">{t.deviceLost}</Banner> : null}
      {notice === "limitReached" ? <Banner>{t.limitReached}</Banner> : null}
      {notice === "cancelled" ? <Banner>{t.cancelled}</Banner> : null}

      {idle && session?.recovery ? (
        <Banner
          variant="warning"
          action={
            <span className={styles.bannerActions}>
              <Button variant="primary" onClick={() => void d.recover()}>
                {t.recover}
              </Button>
              <Button onClick={() => void d.discardAudio()}>{t.discardRecovery}</Button>
            </span>
          }
        >
          {t.recovery(formatDuration(session.recovery.durationMs))}
        </Banner>
      ) : null}

      {idle && current?.status === "recorded" ? (
        <Banner
          action={
            <span className={styles.bannerActions}>
              <Button variant="primary" onClick={() => void d.process()}>
                {t.process}
              </Button>
              <Button onClick={() => void d.discardAudio()}>{t.discardAudio}</Button>
            </span>
          }
        >
          {t.pendingAudio}
        </Banner>
      ) : null}

      {idle && current?.status === "failed" ? (
        <Banner
          variant="warning"
          action={
            <span className={styles.bannerActions}>
              <Button variant="primary" onClick={() => void d.process()}>
                {t.retry}
              </Button>
              <Button onClick={() => void d.discardAudio()}>{t.discardAudio}</Button>
            </span>
          }
        >
          {t.failedAudio}
        </Banner>
      ) : null}

      {noSpeech ? (
        <Banner
          variant="warning"
          action={<Button onClick={() => void d.startRecording()}>{t.repeat}</Button>}
        >
          {t.noSpeech}
        </Banner>
      ) : null}

      {idle && current?.lowQuality ? <Banner variant="warning">{t.lowQuality}</Banner> : null}
      {idle && current?.transcribedLocally ? <Banner>{t.transcribedLocally}</Banner> : null}

      {recording ? (
        <div className={styles.recording}>
          <Button
            variant="danger"
            className={styles.recordButton}
            onClick={() => void d.stopRecording()}
          >
            {t.stop}
          </Button>
          <div role="status" aria-live="polite">
            <p className={styles.recordingLabel}>{t.recording}</p>
            <p>{t.elapsed(formatDuration(d.elapsedMs))}</p>
          </div>
          <meter
            className={styles.level}
            aria-label={t.levelLabel}
            min={0}
            max={1}
            value={d.level}
          />
          {d.limitWarning ? <Banner variant="warning">{t.limitWarning}</Banner> : null}
        </div>
      ) : null}

      {processing ? (
        <div className={styles.processing} role="status" aria-live="polite">
          <p className={styles.recordingLabel}>{t.processing}</p>
          {d.stage ? <p>{t.stages[d.stage]}</p> : null}
          {d.percent !== null ? (
            <>
              <progress className={styles.level} max={100} value={d.percent} />
              <p>{t.percent(d.percent)}</p>
            </>
          ) : null}
          <Button onClick={() => void d.cancelProcessing()}>{t.cancel}</Button>
        </div>
      ) : null}

      {idle ? (
        <div className={styles.startRow}>
          <Button
            variant="primary"
            className={styles.recordButton}
            onClick={() => void d.startRecording()}
          >
            {t.start}
          </Button>
          <div className={styles.startOptions}>
            <p className={styles.hint}>{t.shortcut}</p>
            <div className={styles.bannerActions}>
              <Button onClick={() => void d.importAudio()}>{t.import}</Button>
              <Button onClick={onWrite}>{t.write}</Button>
            </div>
          </div>
        </div>
      ) : null}

      <ConfirmDialog
        open={confirmation !== null}
        title={confirmation ? CONFIRM_TITLES[confirmation.reason] : ""}
        message={confirmation?.message}
        onDismiss={d.dismissConfirmation}
        actions={[
          { label: es.common.cancel, onClick: d.dismissConfirmation },
          { label: es.draft.confirm.continue, onClick: d.confirmPending, variant: "primary" },
        ]}
      />
    </section>
  );
}
