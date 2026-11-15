#!/usr/bin/env python3
"""Voci delle battute del Prologo, generate con Piper LOCALE e ffmpeg (nessuna rete). ADR 0050.

Legge le battute («> testo») delle missioni (`assets/quests/prologo/{missioni,secondarie,eventi}.txt`,
dette da chi è nella condizione `quando: parla <abitante>` della tappa) e delle confidenze
(`assets/world/segreti.txt`, dette dall'abitante della sezione) e dei compagni di strada
(`assets/world/compagni.txt`, righe «chiave: battuta» nella sezione del compagno) e dei fumetti
(`assets/world/battute.txt`, stesso formato). Per ogni battuta scrive
`assets/audio/voci/prologo/<abitante>-<hash>.ogg`, con la voce della persona in `assets/ai/<abitante>.txt`.

L'hash è FNV-1a a 64 bit del testo della battuta così com'è nel file (con `{nome}`): il gioco lo
ricalcola (`voice::line_clip`) e trova la clip senza bisogno di una mappa. Nella voce `{nome}` diventa
«viaggiatore», perché il nome del giocatore non si può registrare prima.

Genera solo i clip mancanti (`--force` li rifà tutti) e toglie quelli di battute che non esistono più.
Richiede Piper e le voci italiane installate: `./scripts/setup_ai.sh --tts`.

Uso: scripts/gen_voci_prologo.py [--force] [--piper PERCORSO] [--elenco]
"""
import argparse
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gen_tts_voci import convert_to_ogg, data_dir, find_piper, synthesize  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
MISSIONS = [ROOT / f"assets/quests/prologo/{n}.txt" for n in ("missioni", "secondarie", "eventi")]
SECRETS = ROOT / "assets/world/segreti.txt"
COMPANIONS = ROOT / "assets/world/compagni.txt"
BARKS = ROOT / "assets/world/battute.txt"
PERSONE = ROOT / "assets/ai"
OUT = ROOT / "assets/audio/voci/prologo"
MALE = "it_IT-riccardo-x_low"


def fnv1a(text: str) -> str:
    h = 0xCBF29CE484222325
    for b in text.encode("utf-8"):
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def slug(name: str) -> str:
    return name.lower().replace(" ", "-")


def clip_name(speaker: str, line: str) -> str:
    return f"{slug(speaker)}-{fnv1a(line.strip())}"


def mission_lines(path: Path):
    """(abitante, battuta) per ogni tappa con `quando: parla`."""
    speaker, lines = None, []
    out = []

    def flush():
        if speaker:
            out.extend((speaker, l) for l in lines)

    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line.startswith("["):
            flush()
            speaker, lines = None, []
        elif line.startswith(">"):
            # «> [giovane] testo»: la battuta per un'età sola (ADR 0047); la voce è del solo testo.
            lines.append(re.sub(r"^\[(giovane|adulto|maturo)\]\s*", "", line[1:].strip()))
        elif line.startswith("quando:"):
            m = re.match(r"quando:\s*parla\s+(.+)$", line)
            if m:
                speaker = m.group(1).strip()
    flush()
    return out


def secret_lines(path: Path):
    out, speaker = [], None
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line.startswith("[") and line.endswith("]"):
            speaker = line[1:-1].strip()
        elif line.startswith(">") and speaker:
            out.append((speaker, line[1:].strip()))
    return out


def companion_lines(path: Path):
    out, speaker = [], None
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            speaker = line[1:-1].strip()
        elif ":" in line and speaker:
            out.append((speaker, line.split(":", 1)[1].strip()))
    return out


def persona(speaker: str):
    voce, scale = MALE, 1.0
    path = PERSONE / f"{slug(speaker)}.txt"
    if path.is_file():
        for raw in path.read_text(encoding="utf-8").splitlines():
            line = raw.strip()
            if line.startswith("voce:"):
                voce = line.split(":", 1)[1].strip() or MALE
            elif line.startswith("velocita:"):
                try:
                    scale = float(line.split(":", 1)[1].strip().replace(",", "."))
                except ValueError:
                    pass
    return voce, scale


def spoken(line: str) -> str:
    return line.replace("{nome}", "viaggiatore")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--force", action="store_true")
    ap.add_argument("--piper")
    ap.add_argument("--elenco", action="store_true", help="mostra le battute senza generare nulla")
    args = ap.parse_args()
    pairs = [p for path in MISSIONS for p in mission_lines(path)] + secret_lines(SECRETS) + companion_lines(COMPANIONS) + companion_lines(BARKS)
    wanted = {clip_name(s, l): (s, l) for s, l in pairs}
    if args.elenco:
        for name, (s, l) in sorted(wanted.items()):
            print(f"{name}\t{s}\t{l}")
        print(f"{len(wanted)} battute")
        return
    piper = find_piper(args.piper)
    OUT.mkdir(parents=True, exist_ok=True)
    made = 0
    for name, (speaker, line) in sorted(wanted.items()):
        out = OUT / f"{name}.ogg"
        if out.exists() and not args.force:
            continue
        voce, scale = persona(speaker)
        model = data_dir() / "ai" / "voices" / f"{voce}.onnx"
        if not model.is_file():
            sys.exit(f"Voce non installata: {model.name}")
        with tempfile.TemporaryDirectory() as tmp:
            wav = Path(tmp) / "clip.wav"
            synthesize(piper, model, scale, spoken(line), wav)
            convert_to_ogg(wav, out)
        made += 1
        print(f"  {name}: {line}")
    stale = [p for p in OUT.glob("*.ogg") if p.stem not in wanted]
    for p in stale:
        p.unlink()
    print(f"{made} voci nuove, {len(stale)} tolte, {len(wanted)} in tutto")


if __name__ == "__main__":
    main()
