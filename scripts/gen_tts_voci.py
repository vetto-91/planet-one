#!/usr/bin/env python3
"""Genera i file audio OGG per le voci con edge-tts e ffmpeg.

Legge docs/voci/ladro.txt e docs/voci/viandante.txt e salva in:
  - assets/audio/voci/ladro/{ID}.ogg
  - assets/audio/voci/viandante/{ID}.ogg
"""
import asyncio
import os
import re
import subprocess
import tempfile
from pathlib import Path

# Assicura certificati CA corretti per SSL
if "SSL_CERT_FILE" not in os.environ:
    for cert_path in ["/etc/ssl/certs/ca-certificates.crt", "/etc/pki/tls/certs/ca-bundle.crt"]:
        if os.path.exists(cert_path):
            os.environ["SSL_CERT_FILE"] = cert_path
            break

import edge_tts

ROOT = Path(__file__).resolve().parent.parent
DOCS_VOCI = ROOT / "docs/voci"
AUDIO_VOCI = ROOT / "assets/audio/voci"

CHARACTERS = {
    "ladro": {
        "txt": DOCS_VOCI / "ladro.txt",
        "out_dir": AUDIO_VOCI / "ladro",
        "voice": "it-IT-DiegoNeural",
        "rate": "+8%",
        "pitch": "+4Hz",
    },
    "viandante": {
        "txt": DOCS_VOCI / "viandante.txt",
        "out_dir": AUDIO_VOCI / "viandante",
        "voice": "it-IT-GiuseppeMultilingualNeural",
        "rate": "-4%",
        "pitch": "-6Hz",
    },
}


def parse_voci_file(path: Path) -> list[tuple[str, str, str]]:
    """Ritorna lista di (id_battuta, quando, testo)."""
    items = []
    pattern = re.compile(r"^([LV]-\d{3})\s*\|\s*([^|]+)\s*\|\s*(.+)$")
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        m = pattern.match(line)
        if m:
            item_id = m.group(1).strip()
            when = m.group(2).strip()
            text = m.group(3).strip()
            items.append((item_id, when, text))
    return items


async def generate_speech(text: str, voice: str, rate: str, pitch: str, out_mp3: Path):
    communicate = edge_tts.Communicate(text, voice, rate=rate, pitch=pitch)
    await communicate.save(str(out_mp3))


def convert_to_ogg(in_mp3: Path, out_ogg: Path):
    """Converte in OGG Vorbis 48kHz mono normalizzato con ffmpeg."""
    cmd = [
        "ffmpeg", "-y", "-i", str(in_mp3),
        "-af", "highpass=f=80,loudnorm=I=-16:TP=-1.0:LRA=11,afade=t=in:ss=0:d=0.01",
        "-ar", "48000",
        "-ac", "1",
        "-c:a", "libvorbis",
        "-q:a", "6",
        str(out_ogg),
    ]
    subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
    # loudnorm non è affidabile sulle clip brevi: si alza a mano se il picco è basso.
    from normalize_voci import normalize_file

    normalize_file(out_ogg)


async def main():
    for char_key, conf in CHARACTERS.items():
        conf["out_dir"].mkdir(parents=True, exist_ok=True)
        items = parse_voci_file(conf["txt"])
        print(f"=== Elaborazione {char_key.upper()} ({len(items)} battute) ===")
        for item_id, when, text in items:
            # Per V-004 componibile, generiamo la versione predefinita con i valori dal chiosco
            tts_text = text
            if "{direzione}" in tts_text or "{N}" in tts_text:
                tts_text = tts_text.replace("{direzione}", "nord-est").replace("{N}", "45")

            out_ogg = conf["out_dir"] / f"{item_id}.ogg"
            with tempfile.NamedTemporaryFile(suffix=".mp3", delete=False) as tmp:
                tmp_path = Path(tmp.name)

            try:
                print(f"  [{item_id}] \"{tts_text}\" -> {out_ogg.name}")
                await generate_speech(
                    text=tts_text,
                    voice=conf["voice"],
                    rate=conf["rate"],
                    pitch=conf["pitch"],
                    out_mp3=tmp_path,
                )
                convert_to_ogg(tmp_path, out_ogg)
            finally:
                if tmp_path.exists():
                    tmp_path.unlink()

    print("\nCompletato con successo!")


if __name__ == "__main__":
    asyncio.run(main())
