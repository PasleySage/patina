# Patina

Local-first Windows desktop time tracking. This file is only a glossary — it defines what
words mean, not how they are implemented.

## Language

### Tools and alerts

**Tool**:
One of the lightweight local utilities in the Tools area: a reminder, a stopwatch, a
countdown, or a Pomodoro. Tool state is separate from automatic tracking records.
_Avoid_: Timer (when the whole group is meant), Widget

**Tool alert**:
The in-app notice a tool raises when it reaches a due moment. A tool alert always has a
kind, and every kind reaches the user through the same alert surface.
_Avoid_: Notification (reserved for the operating system's own surface), Popup

**Alert kind**:
What caused a tool alert — a scheduled reminder firing, a countdown reaching zero, a
Pomodoro phase ending, or an app usage limit being exceeded. Kinds share one delivery
path, so behaviour that applies to "alerts" applies to every kind.
_Avoid_: Alert type, Alert level

**Focus phase / Short break / Long break**:
The three phases of a Pomodoro run. A finished focus phase advances to a break; every
`long break every` finished focus phases advance to a long break instead.
_Avoid_: Session, Cycle (a cycle is the counted repetition, not a phase)

### Sound

**Chime**:
The short sound a tool plays when a tool alert fires. A chime is additive: it never
replaces the alert surface, and the alert still arrives when the chime cannot play.
_Avoid_: Ringtone, Alarm, Notification sound (the operating system's own sound)

**Chime tone**:
Which of the bundled chime samples plays. A tone is a fixed asset, not a synthesized
waveform — swapping in a new sample is a data change, not a code change.
_Avoid_: Preset, Sound effect

**Chime volume**:
The 0–100 playback level of the chime, applied in the audio pipeline rather than to the
system master volume. A chime volume of zero means the chime is silent.
_Avoid_: Loudness, Gain (the user-facing control is a percentage)

**Chime enabled**:
Whether a chime plays at all. Independent of chime volume: a user can silence a chime
without losing their tone choice, and can keep a tone choice while muted.
_Avoid_: Mute, Sound on

### Audio sensing

**Audio snapshot**:
What Patina observes about *other* applications' audio sessions — which processes are
playing media, and whether that playback is sustained. Used to decide whether the user
is in a low-interaction situation such as a video or a lecture.
_Avoid_: Chime, Output level (a snapshot observes other apps; it never measures what
Patina itself plays)

The boundary between **Chime** and **Audio snapshot** is deliberate: a chime is Patina
producing sound on purpose, an audio snapshot is Patina noticing sound produced by
others. A chime does become visible to the audio snapshot while it plays — the two are
not separated at the audio API — so what keeps them apart is that tracking only accepts a
signal whose source identity matches the foreground window, and Patina is not a
recognized media application. See
[ADR-0004](./docs/adr/0004-chime-playback-isolated-from-audio-sensing.md).
