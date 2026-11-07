#!/usr/bin/env python3
"""Porta al picco -1 dB le clip vocali troppo basse in assets/audio/voci/.

`loudnorm` di ffmpeg non funziona sulle clip molto brevi (es. "Ok, ciao!" usciva
a -32 LUFS contro i -15 delle altre): qui si misura il picco con `volumedetect`
e si applica un guadagno. Le clip con picco sopra SOGLIA_DB non vengono toccate.

    python3 scripts/normalize_voci.py            # tutte le clip
    python3 scripts/normalize_voci.py file.ogg   # solo quelle indicate
"""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VOCI = ROOT / "assets/audio/voci"
SOGLIA_DB = -3.0  # sotto questo picco la clip è "bassa"
TARGET_DB = -1.0  # picco desiderato


def peak_db(path: Path) -> float:
    out = subprocess.run(
        ["ffmpeg", "-hide_banner", "-nostats", "-i", str(path), "-af", "volumedetect", "-f", "null", "-"],
        capture_output=True, text=True,
    ).stderr
    return float(re.search(r"max_volume: (-?[\d.]+) dB", out).group(1))


def normalize_file(path: Path) -> bool:
    """Rialza la clip se il picco è sotto SOGLIA_DB. Ritorna True se modificata."""
    peak = peak_db(path)
    if peak >= SOGLIA_DB:
        return False
    gain = TARGET_DB - peak
    tmp = path.with_suffix(".tmp.ogg")
    subprocess.run(
        ["ffmpeg", "-y", "-i", str(path), "-af", f"volume={gain:.1f}dB", "-ar", "48000", "-ac", "1",
         "-c:a", "libvorbis", "-q:a", "6", str(tmp)],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True,
    )
    tmp.replace(path)
    print(f"{path.name}: picco {peak:.1f} dB -> {TARGET_DB:.1f} dB (guadagno {gain:+.1f} dB)")
    return True


if __name__ == "__main__":
    files = [Path(a) for a in sys.argv[1:]] or sorted(VOCI.rglob("*.ogg"))
    changed = sum(normalize_file(f) for f in files)
    print(f"{changed} clip modificate su {len(files)}")
