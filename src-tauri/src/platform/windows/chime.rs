use crate::domain::settings::{
    normalize_chime_tone, ChimeSettings, CHIME_VOLUME_MAX, CHIME_VOLUME_MIN,
};
#[cfg(test)]
use crate::domain::settings::{CHIME_TONES, DEFAULT_CHIME_TONE};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Manager, Runtime};
use windows::core::PSTR;
use windows::Win32::Media::Audio::{
    waveOutClose, waveOutOpen, waveOutPrepareHeader, waveOutSetVolume, waveOutUnprepareHeader,
    waveOutWrite, HWAVEOUT, MIDI_WAVE_OPEN_TYPE, WAVEFORMATEX, WAVEHDR, WAVE_MAPPER,
};

/// `WAVEHDR_WHSTERB` — set once the device has consumed the buffer.
const WAVEHDR_WHSTERB: u32 = 0x0000_0008;

/// Chime samples bundled with the app, in `src-tauri/sounds`.
///
/// A tone is a fixed asset: changing what a tone sounds like is a data change
/// (regenerate with `tools/generate_chime_samples.py`), not a code change.
/// Played when a bundled sample is missing or unreadable, so a chime is never
/// silently absent just because an asset failed to resolve.
const FALLBACK_ALIAS: &str = "SystemAsterisk";

const SOUNDS_DIR: &str = "sounds";

/// Playback guard. Windows plays one sound per session at a time, so an overlapping
/// second call would cut the first one off mid-ring. Skipping is the better failure:
/// a chime is additive to the alert, and a truncated one sounds like a glitch.
static PLAYING: AtomicBool = AtomicBool::new(false);

/// Last committed settings, read on the alert path so playback never waits on I/O.
static SETTINGS_SLOT: Mutex<Option<ChimeSettings>> = Mutex::new(None);

pub fn remember_settings(settings: ChimeSettings) {
    if let Ok(mut slot) = SETTINGS_SLOT.lock() {
        *slot = Some(settings);
    }
}

fn current_settings() -> ChimeSettings {
    SETTINGS_SLOT
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or_default()
}

/// Play the configured chime, if it can play at all.
///
/// Never reports failure: a chime is additive to the alert surface, so a machine
/// with no audio device, a missing sample, or a muted chime still gets the visual
/// alert. Problems are logged with the reason and the next step.
pub fn play_chime<R: Runtime>(app: &AppHandle<R>) {
    let settings = current_settings();
    if !settings.enabled || settings.volume == CHIME_VOLUME_MIN {
        return;
    }
    let tone = normalize_chime_tone(&settings.tone).to_string();
    let volume = settings.volume;
    match resolve_sample_path(app, &tone) {
        Some(path) => {
            let describe = format!("failed to play {}", path.display());
            dispatch(move || play_sample(&path, volume), describe)
        }
        None => {
            eprintln!(
                "[chime] bundled sample for tone {tone} is missing from the resource \
                 directory; falling back to the system sound"
            );
            let describe = format!("failed to play the system sound {FALLBACK_ALIAS}");
            dispatch(move || play_alias_sound(FALLBACK_ALIAS, volume), describe);
        }
    }
}

/// Resolve a tone to a readable sample path.
///
/// Dev and packaged layouts differ: `tauri dev` runs against the crate directory,
/// while a packaged build resolves against the installed resource directory. Both
/// are checked so one code path works in either mode.
fn resolve_sample_path<R: Runtime>(app: &AppHandle<R>, tone: &str) -> Option<PathBuf> {
    let file_name = format!("chime-{tone}.wav");
    if let Ok(resource_dir) = app.path().resource_dir() {
        let packaged = resource_dir.join(SOUNDS_DIR).join(&file_name);
        if packaged.is_file() {
            return Some(packaged);
        }
    }
    let development = PathBuf::from(SOUNDS_DIR).join(&file_name);
    development.is_file().then_some(development)
}

fn wide(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Run one playback unit on a dedicated thread so the alert path never waits on it,
/// and so the playback guard is always released exactly once.
///
/// `describe` is a plain string rather than a closure so nothing borrowed from the
/// caller has to outlive the call.
fn dispatch(play: impl FnOnce() -> Result<(), String> + Send + 'static, describe: String) {
    if PLAYING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("patina-chime".to_string())
        .spawn(move || {
            if let Err(error) = play() {
                eprintln!("[chime] {describe}: {error}; the alert is unaffected");
            }
            PLAYING.store(false, Ordering::Release);
        });
    if spawned.is_err() {
        // The thread never ran, so the guard must be released here or the chime would
        // stay silent for the rest of the session.
        PLAYING.store(false, Ordering::Release);
        eprintln!("[chime] could not start the playback thread; the alert is unaffected");
    }
}

/// Sample format the bundled WAVs are generated in (`tools/generate_chime_samples.py`):
/// 16-bit mono PCM at 44.1 kHz. Declared here so playback does not depend on parsing the
/// file's header — a mismatched format would play at the wrong speed.
const SAMPLE_RATE: u32 = 44_100;
const CHANNELS: u16 = 1;
const BITS_PER_SAMPLE: u16 = 16;
/// `waveOutSetVolume` takes 0..=0xFFFF.
const WAVE_VOLUME_SCALE: u32 = 0xFFFF;
/// Size of a RIFF/WAVE header: "RIFF" + size + "WAVE" + "fmt " chunk (16-byte body)
/// + "data" size field, before any sample bytes.
const WAV_HEADER_BYTES: usize = 44;

fn pcm_format() -> WAVEFORMATEX {
    let block_align = CHANNELS * BITS_PER_SAMPLE / 8;
    WAVEFORMATEX {
        wFormatTag: 1, // WAVE_FORMAT_PCM
        nChannels: CHANNELS,
        nSamplesPerSec: SAMPLE_RATE,
        nAvgBytesPerSec: SAMPLE_RATE * u32::from(block_align),
        nBlockAlign: block_align,
        wBitsPerSample: BITS_PER_SAMPLE,
        cbSize: 0,
    }
}

/// Play a bundled sample on a device handle this function owns.
///
/// Holding the handle is what scopes the volume: `waveOutSetVolume` applies to **all
/// instances of a device** when given a device identifier (a null handle), but only to
/// the referenced instance when given a handle. So a chime slider here changes the
/// chime, never the user's music. `PlaySoundW` cannot give that guarantee because it
/// does not expose the handle it opens.
fn play_sample(path: &Path, percent: u8) -> Result<(), String> {
    let file_bytes = std::fs::read(path)
        .map_err(|error| format!("reading {} failed: {error}", path.display()))?;
    if file_bytes.len() <= WAV_HEADER_BYTES {
        return Err(format!(
            "{} is too short to contain audio samples",
            path.display()
        ));
    }
    let samples = file_bytes[WAV_HEADER_BYTES..].to_vec();
    play_pcm(&samples, percent)
}

fn play_pcm(samples: &[u8], percent: u8) -> Result<(), String> {
    let device = open_device()?;
    let result = (|| {
        if percent < CHIME_VOLUME_MAX {
            // Round rather than truncate so 1% is still audible.
            let scaled = u32::from(percent) * (WAVE_VOLUME_SCALE / u32::from(CHIME_VOLUME_MAX));
            // SAFETY: the handle is this scope's own, and the call is made before the
            // first write so the level applies to everything this handle plays.
            unsafe {
                waveOutSetVolume(Some(device), scaled);
            }
        }

        let mut header = WAVEHDR {
            lpData: PSTR(samples.as_ptr() as *mut u8),
            dwBufferLength: samples.len() as u32,
            dwBytesRecorded: 0,
            ..Default::default()
        };
        // SAFETY: `header` and the sample buffer both outlive the calls below — the
        // buffer is an owned `Vec` held by this function, and the header lives on this
        // stack frame until after the last use.
        unsafe {
            if waveOutPrepareHeader(device, &mut header, 0) != 0 {
                return Err(
                    "waveOutPrepareHeader failed; check the device supports PCM".to_string()
                );
            }
            if waveOutWrite(device, &mut header, 0) == 0 {
                return Err("waveOutWrite rejected the sample buffer".to_string());
            }
            // Block until the device reports the buffer consumed, so the buffer and
            // header are not freed while the device may still read them. The wait is
            // bounded by the sample's own length; it runs on the dedicated chime thread,
            // so the alert path is unaffected either way.
            while header.dwFlags & WAVEHDR_WHSTERB == 0 {
                std::thread::yield_now();
            }
            if waveOutUnprepareHeader(device, &mut header, 0) != 0 {
                return Err("waveOutUnprepareHeader failed".to_string());
            }
        }
        Ok(())
    })();
    // SAFETY: the handle came from open_device in this same function and is closed once.
    unsafe {
        waveOutClose(device);
    }
    result
}

/// Open a playback device for this process.
///
/// Failure is not fatal to the chime: the caller reports it and the alert still arrives
/// visually.
fn open_device() -> Result<HWAVEOUT, String> {
    let mut device = HWAVEOUT::default();
    let format = pcm_format();
    // SAFETY: `format` is a valid PCM description that outlives the call, and `device`
    // is a valid out-parameter. Flags 0 requests the default mapper with no callback,
    // which is polled through the header flags rather than an event.
    let result = unsafe {
        waveOutOpen(
            Some(&mut device),
            WAVE_MAPPER,
            &format,
            None,
            None,
            MIDI_WAVE_OPEN_TYPE(0),
        )
    };
    if result == 0 {
        Ok(device)
    } else {
        Err(format!(
            "no usable audio output device (waveOutOpen returned {result})"
        ))
    }
}

/// Play a system sound as the fallback when a bundled sample cannot be read.
///
/// `PlaySoundW` is the only route to a system alias, and it exposes no device handle, so
/// the chime volume cannot be scoped for this path. The alias therefore plays at the
/// device's own level. That is the accepted trade-off: the fallback exists for the rare
/// case where the samples are missing, and being slightly louder than requested beats
/// being silent or changing the user's volume for every other app.
fn play_alias_sound(alias: &str, _percent: u8) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};

    let wide_alias = wide(alias);
    // SAFETY: `wide_alias` is a NUL-terminated buffer that outlives the call, and
    // SND_ALIAS makes PlaySoundW resolve the name against the system sound scheme.
    let started = unsafe {
        PlaySoundW(
            PCWSTR(wide_alias.as_ptr()),
            None,
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
        )
    };
    if started.as_bool() {
        Ok(())
    } else {
        Err(format!(
            "PlaySoundW could not resolve the system sound {alias}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_tone_falls_back_to_default() {
        assert_eq!(normalize_chime_tone("clear"), "clear");
        assert_eq!(normalize_chime_tone("bright"), "bright");
        assert_eq!(normalize_chime_tone("deep"), "deep");
        assert_eq!(normalize_chime_tone("nope"), DEFAULT_CHIME_TONE);
        assert_eq!(normalize_chime_tone(""), DEFAULT_CHIME_TONE);
    }

    #[test]
    fn volume_is_clamped_to_the_playable_range() {
        assert_eq!(ChimeSettings::normalized(true, -1, "clear").volume, 0);
        assert_eq!(ChimeSettings::normalized(true, 0, "clear").volume, 0);
        assert_eq!(ChimeSettings::normalized(true, 55, "clear").volume, 55);
        assert_eq!(ChimeSettings::normalized(true, 100, "clear").volume, 100);
        assert_eq!(ChimeSettings::normalized(true, 101, "clear").volume, 100);
        assert_eq!(
            ChimeSettings::normalized(true, i64::MAX, "clear").volume,
            100
        );
    }

    #[test]
    fn dirty_stored_tone_is_repaired() {
        assert_eq!(ChimeSettings::normalized(true, 60, "bogus").tone, "clear");
        assert_eq!(ChimeSettings::normalized(true, 60, "deep").tone, "deep");
    }

    #[test]
    fn default_settings_are_playable() {
        let settings = ChimeSettings::default();
        assert!(settings.enabled);
        assert!(settings.volume > CHIME_VOLUME_MIN);
        assert!(CHIME_TONES.contains(&settings.tone.as_str()));
    }

    #[test]
    fn every_tone_has_a_bundled_sample() {
        for tone in CHIME_TONES {
            let file_name = format!("chime-{tone}.wav");
            // Cargo runs lib tests with the crate root as the working directory.
            let sample = PathBuf::from(SOUNDS_DIR).join(&file_name);
            assert!(
                sample.is_file(),
                "missing bundled chime sample: {file_name}"
            );
        }
    }
}
