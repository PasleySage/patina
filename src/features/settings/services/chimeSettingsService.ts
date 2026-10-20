import { invoke } from "@tauri-apps/api/core";

export const CHIME_TONE_OPTIONS = ["clear", "bright", "deep"] as const;
export type ChimeTone = (typeof CHIME_TONE_OPTIONS)[number];

export const CHIME_VOLUME_MIN = 0;
/// Above 100 the samples are amplified and then limited, so the chime can be pushed past
/// the bundled level without clipping. Kept in step with the Rust constant, which also
/// documents where the range stops paying for itself.
export const CHIME_VOLUME_MAX = 200;

export type ChimeSettings = {
  enabled: boolean;
  volume: number;
  tone: ChimeTone;
};

const isChimeTone = (value: unknown): value is ChimeTone =>
  typeof value === "string" && (CHIME_TONE_OPTIONS as readonly string[]).includes(value);

const clampVolume = (value: unknown): number => {
  const numeric = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numeric)) {
    return 60;
  }
  return Math.min(CHIME_VOLUME_MAX, Math.max(CHIME_VOLUME_MIN, Math.round(numeric)));
};

const normalize = (value: Partial<ChimeSettings> | null | undefined): ChimeSettings => ({
  enabled: value?.enabled ?? true,
  volume: clampVolume(value?.volume),
  tone: isChimeTone(value?.tone) ? value.tone : "clear",
});

export async function loadChimeSettings(): Promise<ChimeSettings> {
  return normalize(await invoke<Partial<ChimeSettings>>("cmd_get_chime_settings"));
}

/// Persist the settings and republish them to the playback module.
///
/// `preview` asks the backend to play the configured tone once so the user hears the
/// change immediately; it reuses the real playback path rather than a UI-side sound, so
/// a preview cannot differ from what a real alert sounds like.
export async function commitChimeSettings(
  settings: ChimeSettings,
  preview = false,
): Promise<ChimeSettings> {
  const normalized = normalize(settings);
  return normalize(
    await invoke<Partial<ChimeSettings>>("cmd_commit_chime_settings", {
      input: normalized,
      preview,
    }),
  );
}
