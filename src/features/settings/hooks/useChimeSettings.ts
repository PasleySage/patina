import { useCallback, useEffect, useRef, useState } from "react";

import {
  commitChimeSettings,
  loadChimeSettings,
  type ChimeSettings,
  type ChimeTone,
} from "../services/chimeSettingsService.ts";

type ChimePanelState = {
  settings: ChimeSettings;
  busy: boolean;
  /// True when the stored settings could not be read, so the panel is showing defaults
  /// rather than the user's real configuration.
  loadFailed: boolean;
  setEnabled: (nextEnabled: boolean) => void;
  setVolume: (nextVolume: number) => void;
  setTone: (nextTone: ChimeTone) => void;
  preview: () => void;
};

const DEFAULT_CHIME: ChimeSettings = { enabled: true, volume: 60, tone: "clear" };

/// Chime settings are owned by their own backend command rather than the shared app
/// settings draft, because a chime is a property of alerts and must be committed the
/// moment it changes — a chime left unsaved would still use the previous tone, so the
/// preview button would lie.
export function useChimeSettings(): ChimePanelState {
  const [settings, setSettings] = useState<ChimeSettings>(DEFAULT_CHIME);
  const [busy, setBusy] = useState(false);
  const [loadFailed, setLoadFailed] = useState(false);
  /// Last value the backend confirmed, used to roll a failed commit back. Deliberately
  /// not derived from `settings`: the optimistic value is already in `settings`, so
  /// reading the rollback source from there would restore the change that just failed.
  const lastCommitted = useRef<ChimeSettings>(DEFAULT_CHIME);
  /// Newest value the controls are showing, so a rapid second change builds on the first
  /// instead of on a stale closure value.
  const pending = useRef<ChimeSettings>(DEFAULT_CHIME);

  useEffect(() => {
    let cancelled = false;
    loadChimeSettings()
      .then((loaded) => {
        if (!cancelled) {
          lastCommitted.current = loaded;
          pending.current = loaded;
          setSettings(loaded);
        }
      })
      .catch((error) => {
        // Surface it: a silent fallback to defaults looks identical to "the chime is
        // off", which is indistinguishable from a working mute.
        console.error("[chime] failed to load chime settings", error);
        setLoadFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const commit = useCallback(async (next: ChimeSettings, preview = false) => {
    setBusy(true);
    pending.current = next;
    // Show the change immediately so the control never lags behind the click; the
    // committed result replaces it once the backend has normalized the values.
    setSettings(next);
    try {
      const saved = await commitChimeSettings(next, preview);
      lastCommitted.current = saved;
      pending.current = saved;
      setSettings(saved);
      setLoadFailed(false);
    } catch (error) {
      console.error("[chime] failed to save chime settings", error);
      setLoadFailed(true);
      // Roll back to the last value the backend confirmed, not to the optimistic one.
      setSettings(lastCommitted.current);
    } finally {
      setBusy(false);
    }
  }, []);

  return {
    settings,
    busy,
    loadFailed,
    setEnabled: (enabled) => void commit({ ...pending.current, enabled }),
    setVolume: (volume) => void commit({ ...pending.current, volume }),
    setTone: (tone) => void commit({ ...pending.current, tone }),
    preview: () => void commit(pending.current, true),
  };
}
