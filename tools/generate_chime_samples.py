"""Generate the chime WAV samples bundled with Patina.

The three tones reproduce the specifications the source project tuned by ear
(`clear` 880 Hz, `bright` 1318.5 Hz, `deep` 440 Hz), rendered offline so the app
ships plain assets instead of synthesizing at playback time. See
`docs/adr/0003-chimes-are-bundled-wav-samples.md`.

Run from the repository root:

    python3 tools/generate_chime_samples.py

Re-run only when a tone specification changes; the output is committed.
"""

from __future__ import annotations

import io
import math
import struct
import wave
from pathlib import Path

SAMPLE_RATE = 44100
CHANNELS = 1
SAMPLE_WIDTH = 2
MASTER_GAIN = 0.85
TAIL_FADE_SEC = 0.012

# freq / duration / decay rate / attack time / partials as (harmonic, amplitude).
TONE_SPECS = {
    "clear": {
        "freq": 880.0,
        "dur": 0.45,
        "decay": 6.5,
        "attack": 0.006,
        "partials": ((1.0, 1.0), (2.0, 0.14)),
    },
    "bright": {
        "freq": 1318.5,
        "dur": 0.36,
        "decay": 9.0,
        "attack": 0.003,
        "partials": ((1.0, 0.70), (2.0, 0.50), (3.0, 0.25), (4.0, 0.12)),
    },
    "deep": {
        "freq": 440.0,
        "dur": 0.90,
        "decay": 3.0,
        "attack": 0.014,
        "partials": ((0.5, 0.50), (1.0, 1.00), (1.5, 0.18), (2.0, 0.08)),
    },
}

OUTPUT_DIR = Path(__file__).resolve().parents[1] / "src-tauri" / "sounds"


def render_tone(name: str) -> bytes:
    """Render one tone to 16-bit mono WAV bytes at full volume."""
    spec = TONE_SPECS[name]
    count = max(1, int(SAMPLE_RATE * spec["dur"]))
    samples = [0.0] * count
    for i in range(count):
        t = i / SAMPLE_RATE
        # Attack ramp avoids a click; the exponential decay is what separates the tones.
        env = (1.0 - math.exp(-t / spec["attack"])) * math.exp(-spec["decay"] * t)
        acc = 0.0
        for harmonic, amplitude in spec["partials"]:
            acc += amplitude * math.sin(2.0 * math.pi * spec["freq"] * harmonic * t)
        samples[i] = acc * env

    # Linear fade-out stops the truncation from popping.
    tail = min(count, max(1, int(SAMPLE_RATE * TAIL_FADE_SEC)))
    for i in range(count - tail, count):
        samples[i] *= (count - i) / tail

    peak = max(abs(value) for value in samples) or 1.0
    gain = MASTER_GAIN / peak
    frames = struct.pack(
        f"<{count}h", *(int(round(value * gain * 32767)) for value in samples)
    )

    buffer = io.BytesIO()
    with wave.open(buffer, "wb") as handle:
        handle.setnchannels(CHANNELS)
        handle.setsampwidth(SAMPLE_WIDTH)
        handle.setframerate(SAMPLE_RATE)
        handle.writeframes(frames)
    return buffer.getvalue()


def main() -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for name in TONE_SPECS:
        data = render_tone(name)
        path = OUTPUT_DIR / f"chime-{name}.wav"
        path.write_bytes(data)
        print(f"wrote {path.relative_to(OUTPUT_DIR.parents[1])} ({len(data)} bytes)")


if __name__ == "__main__":
    main()
