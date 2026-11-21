#!/usr/bin/env python3
"""Luoghi da scoprire sull'isola (solo i propri piazzamenti e sentieri, non tocca il resto).

I blocchi hanno `quando: generato da gen_luoghi.py`: rilanciando lo script vengono sostituiti.
Spostandoli con F4 e rilanciandolo si perde lo spostamento: dopo il primo giro non serve rilanciarlo.
Convenzioni: x = Est, z = Sud; rotazione = verso del davanti (0 = +Z/sud, 90 = +X/est, 180 = nord, 270 = ovest).
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PL = ROOT / "debug/placements.txt"
SPEC = ROOT / "assets/world/villaggio.txt"
TAG = "gen_luoghi.py"
MARK = "# --- sentieri dei luoghi (generati da scripts/gen_luoghi.py) ---"
ITEMS = [
    ("campo-ladro", 29.0, -31.0, 0, ""),
    # Fattoria: casa, stalla, pagliai.
    ("casa-grande", -42.0, -30.0, 90, ""), ("stalla", -42.0, -42.0, 90, ""),
    ("pagliaio", -35.0, -44.5, 0, ""), ("pagliaio", -48.5, -36.0, 0, ""),
    ("mulino", 0.5, -66.0, 0, ""),
    ("torre-rovina", 58.5, -58.5, 315, ""),
    ("rovine", -56.0, 62.0, 30, ""),
    ("grotta", 90.5, 18.0, 180, ""),
    ("banditi", 76.0, 40.0, 40, ""),
    ("santuario", -66.0, 22.0, 90, ""),
    # Cartelli sui sentieri e panchine nei punti panoramici.
    ("cartello", 3.0, -50.0, 0, "mulino"), ("cartello", -34.5, -33.0, 90, "fattoria"),
    ("cartello", 44.0, -51.0, 0, "torre"), ("cartello", -46.0, 54.0, 0, "rovine"),
    ("cartello", 82.0, 8.5, 270, "grotta"), ("cartello", -70.0, 16.0, 90, "santuario"),
    ("panchina", 53.5, -51.0, 200, ""), ("panchina", -61.5, 19.0, 270, ""),
]
PATHS = [
    (1.6, [(-32.0, -36.0), (-37.0, -36.0), (-37.0, -30.0)]),
    (1.6, [(-37.0, -36.0), (-37.0, -42.0)]),
    (1.4, [(-58.0, 50.0), (-56.0, 58.0)]),
    (1.4, [(-70.0, 20.0), (-63.0, 22.0)]),
    (1.4, [(70.0, 35.0), (76.0, 36.0)]),
    (1.4, [(29.0, -44.5), (29.0, -33.5)]),
    # Dal cartello della grotta al sentiero che porta al piede della rampa d'ingresso: prima i
    # sentieri entravano nel cumulo della grotta (segnalazione 2026-11-19_21-50-57).
    (1.4, [(82.0, 8.5), (84.0, 4.0)]),
]


def main():
    text = PL.read_text(encoding="utf-8")
    parts = re.split(r"(?=\n\[P-\d+\])", text)
    text = "".join(p for p in parts if TAG not in p).rstrip("\n") + "\n"
    ids = [int(i) for i in re.findall(r"^\[P-(\d+)\]", text, re.M)]
    pid = max(ids + [699]) + 1
    for kind, x, z, rot, note in ITEMS:
        text += (f"\n[P-{pid:04d}] tipo: {kind} | modalità: singolo\nquando: generato da {TAG}\n"
                 f"punto: x {x:.2f}, z {z:.2f}\nquantità: 1   seed: 1\nnota: {note or 'luoghi'}\n"
                 f"punti: ({x:.2f},{z:.2f})\nrotazioni: {rot}\n")
        pid += 1
    PL.write_text(text, encoding="utf-8")
    spec = SPEC.read_text(encoding="utf-8")
    spec = re.sub(r"\n" + re.escape(MARK) + r"\n(strada:[^\n]*\n)+", "\n", spec).rstrip("\n") + "\n"
    lines = [f"strada: sentiero {w} | " + ", ".join(f"{a:.2f} {b:.2f}" for a, b in pts) for w, pts in PATHS]
    SPEC.write_text(spec + f"\n{MARK}\n" + "\n".join(lines) + "\n", encoding="utf-8")
    print(f"{len(ITEMS)} luoghi scritti, ultimo P-{pid - 1:04d}")


if __name__ == "__main__":
    main()
