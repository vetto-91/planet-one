#!/usr/bin/env python3
"""Genera i clip OGG delle voci registrate con Piper (TTS LOCALE) e ffmpeg.

Tutto gira in locale, senza rete (vincolo del progetto: nessun servizio cloud, nemmeno in
sviluppo). Richiede Piper e almeno una voce italiana: `./scripts/setup_ai.sh --tts`.

Legge docs/voci/ladro.txt e docs/voci/viandante.txt e salva in:
  - assets/audio/voci/ladro/{ID}.ogg
  - assets/audio/voci/viandante/{ID}.ogg

Per sicurezza genera SOLO i clip mancanti (quelli esistenti, nati con un'altra voce, non vengono
toccati); usa --force per rigenerarli tutti con la voce di Piper.

Uso: scripts/gen_tts_voci.py [--force] [--voice NOME] [--piper PERCORSO]
"""
import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS_VOCI = ROOT / "docs/voci"
AUDIO_VOCI = ROOT / "assets/audio/voci"

# Voce Piper e velocità (length_scale: >1 = più lenta) per ogni personaggio.
CHARACTERS = {
    "ladro": {"txt": DOCS_VOCI / "ladro.txt", "voice": "it_IT-riccardo-x_low", "length_scale": 0.92},
    "viandante": {"txt": DOCS_VOCI / "viandante.txt", "voice": "it_IT-riccardo-x_low", "length_scale": 1.05},
}


def data_dir() -> Path:
    if sys.platform == "win32":
        return Path(os.environ.get("LOCALAPPDATA", ".")) / "planet-one"
    if sys.platform == "darwin":
        return Path.home() / "Library/Application Support/planet-one"
    return Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share")) / "planet-one"


def find_piper(explicit: str | None) -> Path:
    exe = "piper.exe" if sys.platform == "win32" else "piper"
    cands = [explicit, os.environ.get("PLANET_ONE_PIPER")]
    ai = data_dir() / "ai" / "piper"
    cands += [ai / "piper" / exe, ai / exe, shutil.which("piper")]
    for c in cands:
        if c and Path(c).is_file():
            return Path(c)
    sys.exit("Piper non trovato: esegui ./scripts/setup_ai.sh --tts (installazione locale una tantum).")


def parse_voci_file(path: Path) -> list[tuple[str, str, str]]:
    """Ritorna lista di (id_battuta, quando, testo)."""
    items = []
    pattern = re.compile(r"^([LV]-\d{3})\s*\|\s*([^|]+)\s*\|\s*(.+)$")
    for raw in path.read_text(encoding="utf-8").splitlines():
        m = pattern.match(raw.strip())
        if m:
            items.append((m.group(1).strip(), m.group(2).strip(), m.group(3).strip()))
    return items


LEXICON = Path(__file__).resolve().parent.parent / "assets" / "audio" / "voci" / "pronuncia.txt"


def lexicon() -> list[tuple[str, str]]:
    """Le coppie (parola, come leggerla) di `assets/audio/voci/pronuncia.txt`."""
    pairs = []
    if LEXICON.is_file():
        for raw in LEXICON.read_text(encoding="utf-8").splitlines():
            line = raw.strip()
            if line and not line.startswith("#") and "=>" in line:
                word, said = (p.strip() for p in line.split("=>", 1))
                if word and said:
                    pairs.append((word, said))
    return pairs


def respell(text: str) -> str:
    """Applica il lessico di pronuncia, a parole intere."""
    import re

    for word, said in lexicon():
        text = re.sub(rf"(?<!\w){re.escape(word)}(?!\w)", said, text)
    return text


def synthesize(piper: Path, model: Path, length_scale: float, text: str, out_wav: Path):
    """Piper locale: testo su stdin, WAV su file (nessuna rete). Il lessico di pronuncia si applica qui."""
    text = respell(text)
    subprocess.run(
        [str(piper), "--model", str(model), "--length_scale", f"{length_scale:.2f}", "--output_file", str(out_wav)],
        input=text.encode("utf-8"), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True,
    )


def convert_to_ogg(in_wav: Path, out_ogg: Path):
    """Converte in OGG Vorbis 48 kHz mono normalizzato con ffmpeg (locale)."""
    cmd = [
        "ffmpeg", "-y", "-i", str(in_wav),
        "-af", "highpass=f=80,loudnorm=I=-16:TP=-1.0:LRA=11,afade=t=in:ss=0:d=0.01",
        "-ar", "48000", "-ac", "1", "-c:a", "libvorbis", "-q:a", "6", str(out_ogg),
    ]
    subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
    # loudnorm non è affidabile sulle clip brevi: si alza a mano se il picco è basso.
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from normalize_voci import normalize_file

    normalize_file(out_ogg)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--force", action="store_true", help="rigenera anche i clip già esistenti")
    ap.add_argument("--voice", help="voce Piper da usare per tutti i personaggi (es. it_IT-paola-medium)")
    ap.add_argument("--piper", help="percorso dell'eseguibile di Piper")
    args = ap.parse_args()

    piper = find_piper(args.piper)
    voices = data_dir() / "ai" / "voices"
    for key, conf in CHARACTERS.items():
        out_dir = AUDIO_VOCI / key
        out_dir.mkdir(parents=True, exist_ok=True)
        model = voices / f"{args.voice or conf['voice']}.onnx"
        if not model.is_file():
            sys.exit(f"Voce non installata: {model.name} (./scripts/setup_ai.sh --tts --voice NOME)")
        items = parse_voci_file(conf["txt"])
        print(f"=== {key.upper()} ({len(items)} battute, voce {model.stem}) ===")
        for item_id, _when, text in items:
            out_ogg = out_dir / f"{item_id}.ogg"
            if out_ogg.exists() and not args.force:
                print(f"  [{item_id}] già presente, salto")
                continue
            # Battuta componibile (V-004): versione predefinita con i valori del chiosco.
            tts_text = text.replace("{direzione}", "nord-est").replace("{N}", "45")
            with tempfile.TemporaryDirectory() as tmp:
                wav = Path(tmp) / "clip.wav"
                print(f'  [{item_id}] "{tts_text}" -> {out_ogg.name}')
                synthesize(piper, model, conf["length_scale"], tts_text, wav)
                convert_to_ogg(wav, out_ogg)


if __name__ == "__main__":
    main()
