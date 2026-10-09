use crate::domain::settings::{
    normalize_chime_tone, ChimeSettings, CHIME_VOLUME_MAX, CHIME_VOLUME_MIN,
};
#[cfg(test)]
use crate::domain::settings::{CHIME_TONES, DEFAULT_CHIME_TONE};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Manager, Runtime};
use windows::core::PCWSTR;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};

/// Chime samples are bundled assets, so changing what a tone sounds like is a data
/// change — regenerate with `tools/generate_chime_samples.py` — not a code change.
const SOUNDS_DIR: &str = "sounds";

/// Played when a bundled sample cannot be read, so a chime is never silently absent just
/// because an asset failed to resolve.
const FALLBACK_ALIAS: &str = "SystemAsterisk";

/// `WAVE_FORMAT_PCM` — the only encoding the chime samples use.
const WAVE_FORMAT_PCM: u16 = 1;

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
/// Never reports failure: a chime is additive to the alert surface, so a machine with no
/// audio device, a missing sample, or a muted chime still gets the visual alert.
/// Problems are logged with the reason and the next step.
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
/// Dev and packaged layouts differ: `tauri dev` runs against the crate directory, while a
/// packaged build resolves against the installed resource directory. Both are checked so
/// one code path works in either mode.
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

/// Run one playback unit on a dedicated thread so the alert path never waits on it, and
/// so the playback guard is released exactly once.
///
/// `describe` is a plain string rather than a closure so nothing borrowed from the caller
/// has to outlive the call.
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

/// Play a bundled sample at the requested level.
///
/// `PlaySoundW` is used because it needs no exclusive device handle: it renders through
/// the shared mixer, so a chime never interrupts another app's audio. It accepts no
/// per-call volume, so the level is baked into the samples instead.
///
/// An earlier version set the device's wave volume through `waveOutSetVolume` to get a
/// per-call level. That was wrong twice over: a device identifier applies to every
/// application on that device, and the level was never restored. Scaling the samples
/// keeps the promise that a chime slider cannot change what the user's music sounds like.
fn play_sample(path: &Path, percent: u8) -> Result<(), String> {
    if percent >= CHIME_VOLUME_MAX {
        return play_as_is(path);
    }
    let file_bytes = std::fs::read(path)
        .map_err(|error| format!("reading {} failed: {error}", path.display()))?;
    let pcm = PcmSamples::parse(&file_bytes)
        .map_err(|reason| format!("{} is unusable: {reason}", path.display()))?;
    let quieter = pcm.scale_to(percent);
    if quieter.iter().all(|byte| *byte == 0) {
        // The requested level rounds the whole payload to silence. Playing the original
        // would be louder than the user asked for, so stay quiet instead.
        return Ok(());
    }
    // PlaySoundW reads a file, so the scaled samples go to a temporary one in the system
    // temp directory — the resource directory may not be writable, and the samples are
    // short enough that rewriting them costs nothing measurable.
    let staged = stage_scaled_sample(&pcm, &quieter)?;
    let result = play_as_is(&staged);
    remove_staged_sample(&staged);
    result
}

fn play_as_is(path: &Path) -> Result<(), String> {
    let wide_path = wide(&path.to_string_lossy());
    // SAFETY: `wide_path` is a NUL-terminated buffer that outlives the call, and
    // SND_FILENAME makes PlaySoundW read it as a path rather than an alias. SND_ASYNC
    // makes the call return before the sample finishes, so the playback thread is never
    // blocked for the sample's duration.
    let started = unsafe {
        PlaySoundW(
            PCWSTR(wide_path.as_ptr()),
            None,
            SND_FILENAME | SND_ASYNC | SND_NODEFAULT,
        )
    };
    if started.as_bool() {
        Ok(())
    } else {
        Err(format!("PlaySoundW could not play {}", path.display()))
    }
}

/// Play a system sound as the fallback when a bundled sample cannot be read.
///
/// `PlaySoundW` is the only route to a system alias and it exposes no volume, so this
/// path plays at the device's own level. That is the accepted trade-off: the fallback
/// exists for the rare case where the samples are missing, and being slightly louder
/// than requested beats being silent.
fn play_alias_sound(alias: &str, _percent: u8) -> Result<(), String> {
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

/// Mono 16-bit PCM samples plus the format they were recorded in, taken from a RIFF
/// container.
struct PcmSamples {
    format: SampleFormat,
    data: Vec<u8>,
}

/// The parts of `WAVEFORMATEX` this module needs, copied out because the Windows struct
/// is `#[repr(packed)]` and its fields cannot be referenced directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SampleFormat {
    channels: u16,
    sample_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

impl PcmSamples {
    /// Walk the RIFF chunk list rather than assuming a fixed header length.
    ///
    /// A canonical WAV places `data` at offset 36 or 44 depending on how much of the
    /// optional `fmt ` body is present, and any extra chunk moves it again. A hardcoded
    /// offset silently shifts the payload, so the device receives data that is not the
    /// recording.
    fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err("not a RIFF/WAVE file");
        }
        let mut cursor = 12usize;
        let mut format: Option<SampleFormat> = None;
        let mut data: Option<Vec<u8>> = None;

        while cursor + 8 <= bytes.len() {
            let chunk_id = &bytes[cursor..cursor + 4];
            let chunk_size = u32::from_le_bytes([
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
            ]) as usize;
            let body_start = cursor + 8;
            let body_end = body_start
                .checked_add(chunk_size)
                .filter(|end| *end <= bytes.len())
                .ok_or("a chunk claims more bytes than the file holds")?;

            match chunk_id {
                b"fmt " => {
                    if chunk_size < 16 {
                        return Err("the fmt chunk is too small to describe a format");
                    }
                    let u16_at = |offset: usize| {
                        u16::from_le_bytes([
                            bytes[body_start + offset],
                            bytes[body_start + offset + 1],
                        ])
                    };
                    let u32_at = |offset: usize| {
                        u32::from_le_bytes([
                            bytes[body_start + offset],
                            bytes[body_start + offset + 1],
                            bytes[body_start + offset + 2],
                            bytes[body_start + offset + 3],
                        ])
                    };
                    if u16_at(0) != WAVE_FORMAT_PCM {
                        return Err("samples are not uncompressed PCM");
                    }
                    format = Some(SampleFormat {
                        channels: u16_at(2),
                        sample_rate: u32_at(4),
                        block_align: u16_at(12),
                        bits_per_sample: u16_at(14),
                    });
                }
                b"data" if data.is_none() => {
                    data = Some(bytes[body_start..body_end].to_vec());
                }
                _ => {}
            }

            if format.is_some() && data.is_some() {
                break;
            }
            // RIFF chunks are word-aligned: an odd-sized chunk is followed by a pad byte.
            cursor = body_end + (chunk_size % 2);
        }

        let format = format.ok_or("no fmt chunk")?;
        let data = data.ok_or("no data chunk")?;
        if data.is_empty() {
            return Err("the data chunk is empty");
        }
        if format.block_align == 0 {
            return Err("the format declares a zero block alignment");
        }
        if format.sample_rate == 0 {
            return Err("the format declares no sample rate");
        }
        Ok(Self { format, data })
    }

    /// Return the samples scaled to `percent` of full scale, as 16-bit little-endian.
    ///
    /// The payload is scaled rather than the device, so no device, mixer, or system
    /// volume is touched. The peak is normalized first, so lowering the level lowers the
    /// loudest moment rather than only shrinking the already-quiet parts.
    fn scale_to(&self, percent: u8) -> Vec<u8> {
        if self.format.bits_per_sample != 16 {
            return Vec::new();
        }
        let frame = usize::from(self.format.block_align);
        if frame < 2 || self.data.len() < frame {
            return Vec::new();
        }
        let mut peak: i32 = 0;
        for index in (0..self.data.len()).step_by(frame) {
            peak = peak.max(i32::from(self.sample_at(index)).abs());
        }
        if peak == 0 {
            return Vec::new();
        }
        let target = peak * i32::from(percent) / i32::from(CHIME_VOLUME_MAX);
        let mut out = Vec::with_capacity(self.data.len());
        for index in (0..self.data.len()).step_by(2) {
            if index + 1 >= self.data.len() {
                break;
            }
            // Widen before scaling so the multiply cannot overflow.
            let scaled = i32::from(self.sample_at(index)) * target / peak;
            let clamped = scaled.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            out.extend_from_slice(&clamped.to_le_bytes());
        }
        out
    }

    /// Read the 16-bit sample starting at `offset`, or 0 past the end of the payload.
    fn sample_at(&self, offset: usize) -> i16 {
        self.data
            .get(offset..offset + 2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .unwrap_or(0)
    }
}

/// Where scaled samples are written before `PlaySoundW` reads them. The process id keeps
/// two instances from colliding.
fn staged_sample_path() -> PathBuf {
    std::env::temp_dir().join(format!("patina-chime-{}.wav", std::process::id()))
}

/// Write the scaled samples as a standalone WAV, reusing the source's own format so the
/// staged file is playable regardless of what the original used.
fn stage_scaled_sample(pcm: &PcmSamples, samples: &[u8]) -> Result<PathBuf, String> {
    use std::io::Write;

    let path = staged_sample_path();
    let data_len = u32::try_from(samples.len())
        .map_err(|_| "the scaled sample exceeds the WAV size field".to_string())?;
    let byte_rate = pcm.format.sample_rate * u32::from(pcm.format.block_align);

    let mut header = [0u8; 44];
    header[0..4].copy_from_slice(b"RIFF");
    header[4..8].copy_from_slice(&(36u32 + data_len).to_le_bytes());
    header[8..12].copy_from_slice(b"WAVE");
    header[12..16].copy_from_slice(b"fmt ");
    header[16..20].copy_from_slice(&16u32.to_le_bytes());
    header[20..22].copy_from_slice(&WAVE_FORMAT_PCM.to_le_bytes());
    header[22..24].copy_from_slice(&pcm.format.channels.to_le_bytes());
    header[24..28].copy_from_slice(&pcm.format.sample_rate.to_le_bytes());
    header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    header[32..34].copy_from_slice(&pcm.format.block_align.to_le_bytes());
    header[34..36].copy_from_slice(&pcm.format.bits_per_sample.to_le_bytes());
    header[36..40].copy_from_slice(b"data");
    header[40..44].copy_from_slice(&data_len.to_le_bytes());

    let mut file = std::fs::File::create(&path)
        .map_err(|error| format!("creating {} failed: {error}", path.display()))?;
    file.write_all(&header)
        .and_then(|_| file.write_all(samples))
        .map_err(|error| format!("writing {} failed: {error}", path.display()))?;
    Ok(path)
}

fn remove_staged_sample(path: &Path) {
    // PlaySoundW was given SND_ASYNC, so the file may still be open. A failure here
    // leaves a small file behind rather than interrupting playback, which is the right
    // trade: an orphaned temp file is cheaper than a truncated chime.
    if let Err(error) = std::fs::remove_file(path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            eprintln!(
                "[chime] could not remove the staged sample {}: {error}",
                path.display()
            );
        }
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

    fn bundled(tone: &str) -> PcmSamples {
        let path = PathBuf::from(SOUNDS_DIR).join(format!("chime-{tone}.wav"));
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("reading {} failed: {error}", path.display()));
        PcmSamples::parse(&bytes)
            .unwrap_or_else(|reason| panic!("{} failed to parse: {reason}", path.display()))
    }

    /// The bundled samples must survive the same parse the playback path uses.
    #[test]
    fn every_bundled_sample_parses_as_pcm() {
        for tone in CHIME_TONES {
            let pcm = bundled(tone);
            assert_eq!(pcm.format.bits_per_sample, 16, "{tone} is not 16-bit");
            assert_eq!(pcm.format.channels, 1, "{tone} is not mono");
            assert_eq!(
                pcm.format.sample_rate, 44_100,
                "{tone} has an unexpected rate"
            );
            assert!(
                pcm.format.block_align > 0,
                "{tone} declares no block alignment"
            );
            assert!(!pcm.data.is_empty(), "{tone} carries no samples");
            // A partial trailing frame would be played as noise.
            assert_eq!(
                pcm.data.len() % usize::from(pcm.format.block_align),
                0,
                "{tone} ends mid-frame"
            );
            assert!(
                pcm.data.len() > usize::from(pcm.format.block_align) * 100,
                "{tone} is suspiciously short"
            );
        }
    }

    #[test]
    fn a_non_riff_file_is_rejected() {
        assert!(PcmSamples::parse(b"not a wav file at all").is_err());
        assert!(PcmSamples::parse(b"").is_err());
    }

    /// A file whose `data` chunk sits at a different offset than a canonical header
    /// would place it must still parse — which is exactly what a fixed offset gets wrong.
    #[test]
    fn a_padded_chunk_before_data_is_skipped() {
        fn push_chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
            out.extend_from_slice(id);
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(body);
            if body.len() % 2 == 1 {
                out.push(0); // RIFF chunks are word-aligned
            }
        }

        let fmt_body = [
            1u8, 0, // WAVE_FORMAT_PCM
            1, 0, // channels
            0x44, 0xAC, 0, 0, // 44100
            0x88, 0x58, 0x01, 0, // byte rate
            2, 0, // block align
            16, 0, // bits
            0, 0, // cbSize
        ];
        let samples: [u8; 4] = [1, 2, 3, 4];

        let mut body: Vec<u8> = Vec::new();
        // An odd-sized chunk before `data`, so a parser that ignores the pad byte lands
        // one byte early and reads a corrupted payload.
        push_chunk(&mut body, b"LIST", b"INF");
        push_chunk(&mut body, b"fmt ", &fmt_body);
        push_chunk(&mut body, b"data", &samples);

        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(&body);

        let pcm = PcmSamples::parse(&bytes).expect("a padded chunk should be skipped");
        assert_eq!(pcm.format.sample_rate, 44_100);
        assert_eq!(
            pcm.data, samples,
            "the payload must start at the data chunk"
        );
    }

    /// The bundled samples place their payload at 44 bytes — the common "44-byte WAV
    /// header" figure. This test records that as a fact about the assets rather than an
    /// assumption in the parser: a future regeneration that changed the layout would
    /// fail here instead of silently shifting the payload.
    #[test]
    fn the_bundled_header_puts_the_payload_at_44() {
        let bytes = std::fs::read(PathBuf::from(SOUNDS_DIR).join("chime-clear.wav"))
            .expect("the bundled sample must be readable");
        let data_start = bytes
            .windows(4)
            .position(|window| window == b"data")
            .expect("a canonical WAV must contain a data chunk")
            + 8; // skip the chunk's size field
        assert_eq!(
            data_start, 44,
            "the bundled sample's payload offset changed; the parser walks chunks, but \
             anything assuming a fixed header would now be wrong"
        );
    }

    /// The volume contract: scaling must lower the samples and must never touch the
    /// device. This is what would break if a device-level volume call came back.
    #[test]
    fn scaling_lowers_the_peak_proportionally() {
        let pcm = bundled("clear");
        let peak_of = |data: &[u8]| -> i32 {
            data.chunks_exact(2)
                .map(|pair| i32::from(i16::from_le_bytes([pair[0], pair[1]]).abs()))
                .max()
                .unwrap_or(0)
        };
        let full_peak = peak_of(&pcm.data);
        assert!(full_peak > 0, "the sample must not be silent to begin with");

        let half = pcm.scale_to(50);
        assert_eq!(half.len(), pcm.data.len(), "scaling must not change length");
        let half_peak = peak_of(&half);
        assert!(
            half_peak < full_peak,
            "50% must be quieter than full scale ({half_peak} vs {full_peak})"
        );
        let expected = full_peak / 2;
        assert!(
            (half_peak - expected).abs() <= 2,
            "50% peak {half_peak} should be about {expected}"
        );

        assert_eq!(
            pcm.scale_to(100),
            pcm.data,
            "100% must be the original samples"
        );
    }

    #[test]
    fn a_silent_scale_produces_no_audible_output() {
        // 0% must not fall back to playing the original: that would be louder than asked.
        let pcm = bundled("clear");
        assert!(pcm.scale_to(0).iter().all(|byte| *byte == 0));
    }

    #[test]
    fn a_staged_sample_is_a_wav_the_parser_accepts() {
        let pcm = bundled("clear");
        let scaled = pcm.scale_to(60);
        let staged = stage_scaled_sample(&pcm, &scaled).expect("staging must succeed");

        // The header the app writes must satisfy the same parser, otherwise PlaySoundW
        // would be handed a file the app itself considers malformed.
        let written = std::fs::read(&staged).expect("the staged sample must be readable");
        let reparsed = PcmSamples::parse(&written).expect("the staged sample must reparse");
        assert_eq!(reparsed.data, scaled);
        assert_eq!(reparsed.format, pcm.format);

        remove_staged_sample(&staged);
        assert!(
            !staged.exists(),
            "the staged sample must not outlive the call that created it"
        );
    }
}
