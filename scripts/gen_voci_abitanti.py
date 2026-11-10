#!/usr/bin/env python3
"""Voci degli abitanti e dei congedi, generate con Piper LOCALE e ffmpeg (nessuna rete).

Per ogni persona in `assets/ai/<nome>.txt` legge la voce (`voce:`) e le prime battute (`saluto:`,
separate da « / ») e scrive `assets/audio/voci/abitanti/<nome>-<n>.ogg`. Poi genera
`assets/audio/voci/congedo/{m,f}-<n>.ogg`, i saluti di commiato con una voce maschile e una femminile.

Genera solo i clip mancanti (`--force` li rifà tutti). Richiede Piper e le voci italiane installate:
`./scripts/setup_ai.sh --tts`.

Uso: scripts/gen_voci_abitanti.py [--force] [--piper PERCORSO]
"""
import argparse
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gen_tts_voci import convert_to_ogg, data_dir, find_piper, synthesize  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
PERSONE = ROOT / "assets/ai"
OUT = ROOT / "assets/audio/voci"
MALE, FEMALE = "it_IT-riccardo-x_low", "it_IT-paola-medium"
CONGEDI = [
    "Arrivederci.",
    "A presto, viaggiatore.",
    "Buona strada!",
    "Che il cammino vi sia lieve.",
    "Tornate a trovarci.",
    "Fate buon viaggio.",
]


def persona(path: Path) -> tuple[str, list[str]]:
    voce, saluti = MALE, []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line.startswith("voce:"):
            voce = line.split(":", 1)[1].strip() or MALE
        elif line.startswith("saluto:"):
            saluti = [s.strip() for s in re.split(r"\s/\s", line.split(":", 1)[1]) if s.strip()]
    return voce, saluti


def make(piper: Path, voice: str, text: str, out: Path, force: bool, scale: float = 1.0):
    if out.exists() and not force:
        return
    model = data_dir() / "ai" / "voices" / f"{voice}.onnx"
    if not model.is_file():
        sys.exit(f"Voce non installata: {model.name}")
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        wav = Path(tmp) / "clip.wav"
        synthesize(piper, model, scale, text, wav)
        convert_to_ogg(wav, out)
    print(f"  {out.relative_to(OUT)}: {text}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--force", action="store_true")
    ap.add_argument("--piper")
    args = ap.parse_args()
    piper = find_piper(args.piper)
    for path in sorted(PERSONE.glob("*.txt")):
        voce, saluti = persona(path)
        for n, text in enumerate(saluti, 1):
            make(piper, voce, text, OUT / "abitanti" / f"{path.stem}-{n}.ogg", args.force)
    for sex, voice in (("m", MALE), ("f", FEMALE)):
        for n, text in enumerate(CONGEDI, 1):
            make(piper, voice, text, OUT / "congedo" / f"{sex}-{n}.ogg", args.force)


if __name__ == "__main__":
    main()
