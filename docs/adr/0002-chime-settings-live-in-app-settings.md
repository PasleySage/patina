# Chime settings live in app settings, not in tool runtime settings

`ToolRuntimeSettings` looks like the natural home for chime settings — it already carries
countdown and Pomodoro durations, and it rides along in `ToolsRuntimeSnapshot`. It is
not. Nothing in the Rust side ever writes those fields: `data/tools_store.rs` builds the
snapshot without them, so they stay at `ToolRuntimeSettings::default()` forever. The
frontend uses them only to seed `useState` and submits the user's actual values through
`cmd_start_pomodoro`. It is a default-value carrier, not persisted settings.

Chime settings therefore go in `app_settings` (`chime_enabled`, `chime_volume`,
`chime_tone`), the only verified persistence path in the codebase, which survives
restart the way a user's expectation requires. Writing them also republishes them to the
playback module, because a write that reached only SQLite would leave the next alert
ringing with the previous settings until the app restarted.

**Consequences**: a future change that wants to persist tool durations properly should
fix `ToolRuntimeSettings` rather than extend it, and that fix is deliberately out of scope
here. Chime settings do not follow the tool timeline, because a chime is a property of
alerts rather than of any one tool — see
[0001](./0001-chime-hangs-off-tool-alert-path.md).

The alternative — a standalone JSON file, as the source project uses — was rejected: it
introduces a second settings store in a codebase that has an explicit single-owner rule
for settings, and a fork would have to be rewritten before it could be upstreamed.
