# Chime playback is isolated from audio sensing

Patina already contains a Windows audio module (`platform/windows/audio.rs`) that
enumerates audio sessions via `IMMDeviceEnumerator`. It does not play anything: it
observes which *other* applications are producing media, and feeds that into sustained
participation tracking so that watching a video is not misread as idle time. Adding a
chime introduces the risk that Patina's own output is observed as a media session and
changes the user's tracking classification — a bug that would surface as "my hours became
wrong after I turned on sounds", with no obvious cause.

## The obvious answer is wrong

`PlaySoundW` and `IMMDeviceEnumerator` are different Windows APIs, and it is tempting to
conclude that a player cannot be seen by an enumerator. Measured on this machine against
the real audio stack, that conclusion is false: while a chime plays, Windows creates a
session for the playing process, and `PlaySoundW`'s process appears in the enumerator's
results within one probe interval. The session disappears when the sample ends, but it is
there for the duration.

So isolation does not come from the two APIs being separate.

## What actually isolates them

The tracking path filters by identity, twice, and a chime fails both:

1. `resolve_signal_for_window` returns a session only when it matches the foreground
   window, falling back to the first active session otherwise. A chime session can be
   returned by that fallback.
2. `select_matched_signal` then re-evaluates that candidate and accepts only a full
   identity match, so a fallback chime is rejected.

`SustainedParticipationAppIdentity` is a closed enumeration of known media applications
(Chrome, Edge, Firefox, VLC, …). Patina is not a member, so a Patina session always
carries `source_app_identity: None` and can never match a foreground window.

`tests::chime_session_is_not_matched_against_an_unrelated_foreground_window`,
`tests::chime_session_never_reaches_the_matched_signal`, and
`tests::a_chime_does_not_change_status_for_an_idle_but_present_window` exercise the
production functions with a Patina session and an unrelated foreground window, so the
conclusion is locked rather than assumed.

**Consequences**: no self-exemption signal is needed, and none should be added — it
would be a second mechanism guarding a case the identity filter already covers, and it
would have to be kept in step with the tracking path. If a future change made Patina
eligible for `SustainedParticipationAppIdentity`, or relaxed `select_matched_signal` to
accept fallbacks, these tests fail and the isolation has to be re-established
explicitly. `Chime` and `AudioSnapshot` stay separate terms in `CONTEXT.md` because the
distinction is real even though it is enforced downstream rather than at the audio API.
