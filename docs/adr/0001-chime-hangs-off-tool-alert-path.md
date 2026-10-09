# Chimes hang off the shared tool alert path

When any tool reaches a due moment it produces a `ToolAlert` and hands it to
`send_tool_alert`, the single choke point every alert kind already passes through. The
chime is played from that function rather than from each alert site, so all four kinds
(`Reminder`, `Countdown`, `Pomodoro`, `ActivityReminder`) sound without four separate
wiring changes, and a future alert kind is audible by construction.

The alternative — playing from the Pomodoro phase-completion site only — would make the
chime a property of one tool instead of of alerts, and would need a second decision the
first time a countdown or a usage-limit alert wanted the same treatment.

Playing here also keeps the existing alert surface untouched: `show_main_window` is still
driven by the `tools-alert` event listener in `app/tools.rs`. The chime is additive, so a
machine with no audio device, a missing sample, or a zero volume still gets the visual
alert.

**Consequences**: the chime must be cheap and must never fail the alert. Playback runs
off the alert path (a detached thread with a re-entrancy guard, mirroring how the source
project's chime behaved) so a blocking audio API cannot delay the alert. A chime that
cannot play is logged and forgotten; it never surfaces an error to the user.
