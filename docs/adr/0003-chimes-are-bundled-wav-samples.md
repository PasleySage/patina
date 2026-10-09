# Chimes are bundled WAV samples, not synthesized waveforms

The source project synthesizes its chime in memory (`wave` + `math.sin`) and plays the
bytes directly via `winsound.PlaySound(SND_MEMORY)`. That technique has no Windows
equivalent here: `PlaySound` in the `windows` crate takes a file path or a system alias,
not an in-memory buffer, so reproducing it would mean either writing a temporary WAV per
playback or adopting an audio crate for one sound.

Patina ships three WAV samples as bundle resources instead. A chime is a short, fixed,
low-dynamic-range asset; a real sample sounds better than a synthesized sine, and it can
be replaced by editing a file rather than retuning a table. The cost is one line in
`tauri.conf.json` and three small files in the repository.

`PlaySound` is still the playback API, and it is already non-blocking when called with
`SND_ASYNC`, so no new dependency and no blocking-thread workaround is needed. The
system-alias fallback covers a missing or unreadable sample.

**Consequences**: the samples are the only bundled binary assets this feature adds, so
they need a provenance note — these are synthesized offline from the source project's
tone specifications, so there is no third-party license attached. If they are ever
replaced with recordings, that provenance note must be replaced with the recording's
terms.
