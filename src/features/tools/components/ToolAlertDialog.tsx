import { useLocaleText } from "../../../shared/i18n/index.ts";
import { AlarmClock, BellRing, TimerReset } from "lucide-react";
import { useCallback, useRef, useState, type MouseEvent, type ReactNode } from "react";
import QuietDialog from "../../../shared/components/QuietDialog.tsx";
import QuietButton from "../../../shared/components/QuietButton.tsx";

import type { ToolAlert } from "../../../shared/types/tools.ts";
import { useToolAlerts } from "../hooks/useToolAlerts.ts";
import { ToolsRuntimeService } from "../services/toolsRuntimeService.ts";
import { isDeliberateMouseClick } from "../services/pomodoroPhaseConfirmPolicy.ts";

function formatAlertTime(timestampMs: number) {
  return new Date(timestampMs).toLocaleString(undefined, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function alertIcon(alert: ToolAlert): ReactNode {
  if (alert.kind === "countdown") return <TimerReset size={17} />;
  if (alert.kind === "pomodoro") return <AlarmClock size={17} />;
  return <BellRing size={17} />;
}

export default function ToolAlertDialog() {
  const UI_TEXT = useLocaleText();
  const dismissButtonRef = useRef<HTMLButtonElement>(null);
  const { activeAlert, dismissActiveAlert } = useToolAlerts();
  const [confirmingPhase, setConfirmingPhase] = useState(false);
  const title = activeAlert?.title.trim() || UI_TEXT.tools.notificationStatus;
  const message = activeAlert?.body.trim() || UI_TEXT.tools.defaultReminderLabel;
  const occurredAtLabel = activeAlert
    ? UI_TEXT.tools.alertOccurredAt(formatAlertTime(activeAlert.occurredAt))
    : "";

  // A pomodoro phase that ran out holds the run until the user acknowledges it, so its
  // alert *is* the gate: the primary button advances the run instead of just clearing
  // the notice, and nothing else may clear it. Every other alert is a plain notice.
  const holdsPomodoroPhase = activeAlert?.kind === "pomodoro";

  const handleConfirmPomodoroPhase = useCallback(async () => {
    if (!holdsPomodoroPhase || confirmingPhase) return;

    setConfirmingPhase(true);
    try {
      await ToolsRuntimeService.confirmPomodoroPhase();
      dismissActiveAlert();
    } catch (error) {
      console.warn("confirm pomodoro phase from alert failed", error);
    } finally {
      setConfirmingPhase(false);
    }
  }, [confirmingPhase, dismissActiveAlert, holdsPomodoroPhase]);

  const handlePrimaryAction = useCallback((event: MouseEvent<HTMLButtonElement>) => {
    if (!holdsPomodoroPhase) {
      dismissActiveAlert();
      return;
    }
    // Only a real mouse press advances the run. Enter and Space on the focused button
    // produce a click whose `detail` is 0, and this dialog cannot be cleared any other
    // way, so admitting them would start the next phase on a stray keypress.
    if (!isDeliberateMouseClick(event.detail)) {
      return;
    }
    void handleConfirmPomodoroPhase();
  }, [dismissActiveAlert, handleConfirmPomodoroPhase, holdsPomodoroPhase]);

  return (
    <QuietDialog
      open={Boolean(activeAlert)}
      title={title}
      closeOnBackdrop={false}
      // While a phase is being held, dismissing is not an option: the run must not move
      // on and the notice must not vanish until the user acknowledges it.
      onClose={holdsPomodoroPhase ? () => {} : dismissActiveAlert}
      initialFocusRef={dismissButtonRef}
      surfaceClassName="tools-alert-dialog-surface"
      actions={(
        <QuietButton
          ref={dismissButtonRef}
          tone="primary"
          size="large"
          className="qp-dialog-action"
          busy={confirmingPhase}
          onClick={handlePrimaryAction}
        >
          {confirmingPhase ? UI_TEXT.common.processing : UI_TEXT.tools.alertDismiss}
        </QuietButton>
      )}
    >
      {activeAlert && (
        <div className="tools-alert-dialog-body">
          <div className="tools-alert-dialog-icon" aria-hidden="true">
            {alertIcon(activeAlert)}
          </div>
          <div className="tools-alert-dialog-copy">
            <p className="tools-alert-dialog-message">{message}</p>
            <p className="tools-alert-dialog-time">{occurredAtLabel}</p>
          </div>
        </div>
      )}
    </QuietDialog>
  );
}
