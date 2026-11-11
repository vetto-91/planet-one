#!/usr/bin/env python3
"""Ascolta una registrazione del gioco (WAV del pilota automatico, comando `registra`) e la descrive.

Uso: scripts/ascolta.py file.wav [--trascrivi] [--uscita cartella]

Scrive accanto al file (o nella cartella d'uscita):
- `<nome>-spettro.png`: spettrogramma (ffmpeg `showspectrumpic`), frequenze in verticale, tempo in orizzontale;
- `<nome>-volume.png`: andamento del volume (ffmpeg `showwavespic`);
- `<nome>.md`: rapporto con volume medio e massimo, saturazione, silenzi, eventi (colpi, passi),
  regolarità dei passi, bande di frequenza e, con `--trascrivi`, il parlato riconosciuto da whisper.cpp
  locale (`~/.local/share/planet-one/ai/stt/whisper-cli`, nessun servizio in rete).

Servono solo ffmpeg e numpy. Tutto in locale.
"""

import argparse
import os
import subprocess
import sys

import numpy as np

RATE = 22050
WIN = 0.05  # finestra di analisi (s)


def decode(path):
    """Campioni mono float in [-1, 1] a RATE Hz."""
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", path, "-ac", "1", "-ar", str(RATE), "-f", "s16le", "-"],
        check=True,
        capture_output=True,
    ).stdout
    return np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0


def stereo_peaks(path):
    """Picco per canale (per scoprire suoni solo a destra o a sinistra)."""
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", path, "-ac", "2", "-ar", str(RATE), "-f", "s16le", "-"],
        check=True,
        capture_output=True,
    ).stdout
    x = np.frombuffer(raw, dtype=np.int16).astype(np.float32).reshape(-1, 2) / 32768.0
    return np.sqrt(np.mean(x**2, axis=0)) if len(x) else np.zeros(2)


def db(x):
    return 20.0 * np.log10(np.maximum(x, 1e-9))


def frames(x):
    n = int(RATE * WIN)
    k = len(x) // n
    return x[: k * n].reshape(k, n)


def bands(x):
    """Energia relativa per banda (bassi, medi, acuti, sibili)."""
    if len(x) < RATE:
        return {}
    spec = np.abs(np.fft.rfft(x * np.hanning(len(x)))) ** 2
    freqs = np.fft.rfftfreq(len(x), 1.0 / RATE)
    total = spec.sum() + 1e-12
    out = {}
    for name, lo, hi in [
        ("bassi 20-250 Hz", 20, 250),
        ("medio-bassi 250-1000 Hz", 250, 1000),
        ("medi 1-4 kHz", 1000, 4000),
        ("acuti 4-11 kHz", 4000, 11025),
    ]:
        out[name] = float(spec[(freqs >= lo) & (freqs < hi)].sum() / total)
    return out


def onsets(env_db):
    """Istanti in cui il volume sale di colpo (colpi, passi, campane)."""
    rise = np.diff(env_db, prepend=env_db[:1])
    floor = np.percentile(env_db, 30)
    idx = np.where((rise > 6.0) & (env_db > floor + 6.0))[0]
    # Non più di un evento ogni 0,15 s.
    picked = []
    for i in idx:
        if not picked or (i - picked[-1]) * WIN > 0.15:
            picked.append(i)
    return np.array(picked) * WIN


def transcribe(path):
    base = os.path.expanduser("~/.local/share/planet-one/ai/stt")
    exe = os.path.join(base, "whisper-cli")
    model = os.path.join(base, "models", "ggml-base.bin")
    if not (os.path.exists(exe) and os.path.exists(model)):
        return "(whisper.cpp locale non installato)"
    wav16 = path + ".16k.wav"
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-i", path, "-ac", "1", "-ar", "16000", wav16], check=True
    )
    out = subprocess.run(
        [exe, "-m", model, "-l", "it", "-nt", "-f", wav16], capture_output=True, text=True
    )
    os.remove(wav16)
    return out.stdout.strip() or "(nessun parlato riconosciuto)"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("wav")
    ap.add_argument("--trascrivi", action="store_true")
    ap.add_argument("--uscita")
    a = ap.parse_args()
    stem = os.path.splitext(os.path.basename(a.wav))[0]
    out_dir = a.uscita or os.path.dirname(os.path.abspath(a.wav))
    os.makedirs(out_dir, exist_ok=True)

    x = decode(a.wav)
    dur = len(x) / RATE
    if dur < 0.5:
        print(f"{a.wav}: registrazione vuota o troppo corta ({dur:.2f} s)")
        return 1
    f = frames(x)
    rms = np.sqrt(np.mean(f**2, axis=1))
    env = db(rms)
    peak = float(np.max(np.abs(x)))
    clip = float(np.mean(np.abs(x) > 0.985))
    silent = float(np.mean(env < -55.0))
    left, right = stereo_peaks(a.wav)
    ev = onsets(env)
    gaps = np.diff(ev)
    steady = ""
    if len(gaps) >= 4:
        cv = float(np.std(gaps) / (np.mean(gaps) + 1e-9))
        steady = (
            f"{len(ev)} eventi, uno ogni {np.mean(gaps):.2f} s in media "
            f"(regolarità: variazione {cv*100:.0f}%; i passi a ritmo costante stanno sotto il 25%)"
        )
    else:
        steady = f"{len(ev)} eventi"

    spec_png = os.path.join(out_dir, f"{stem}-spettro.png")
    wave_png = os.path.join(out_dir, f"{stem}-volume.png")
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-i", a.wav, "-lavfi",
         "showspectrumpic=s=1200x400:legend=1:scale=log", spec_png],
        check=False,
    )
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-i", a.wav, "-lavfi",
         "showwavespic=s=1200x200:split_channels=1", wave_png],
        check=False,
    )

    lines = [
        f"# Ascolto di `{os.path.basename(a.wav)}`",
        "",
        f"- Durata: {dur:.1f} s",
        f"- Volume medio: {db(np.sqrt(np.mean(x**2))):.1f} dBFS; massimo su 50 ms: {env.max():.1f} dBFS; "
        f"picco: {db(peak):.1f} dBFS",
        f"- Saturazione: {clip*100:.2f}% dei campioni oltre il 98,5% "
        + ("**(si sente gracchiare)**" if clip > 0.001 else "(nessuna)"),
        f"- Silenzio (sotto −55 dBFS): {silent*100:.0f}% del tempo",
        f"- Canali: sinistro {db(left):.1f} dBFS, destro {db(right):.1f} dBFS "
        + ("**(sbilanciato)**" if abs(db(left) - db(right)) > 6 else "(bilanciati)"),
        f"- Eventi (attacchi improvvisi): {steady}",
        "- Bande di frequenza (quota dell'energia): "
        + ", ".join(f"{k} {v*100:.0f}%" for k, v in bands(x).items()),
        "",
        f"![spettrogramma]({os.path.basename(spec_png)})",
        f"![volume]({os.path.basename(wave_png)})",
    ]
    if a.trascrivi:
        lines += ["", "## Parlato (whisper.cpp locale)", "", transcribe(a.wav)]
    report = os.path.join(out_dir, f"{stem}.md")
    with open(report, "w") as fh:
        fh.write("\n".join(lines) + "\n")
    print("\n".join(lines[:9]))
    print(f"rapporto: {report}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
