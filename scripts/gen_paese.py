#!/usr/bin/env python3
"""Oggetti del paese vissuto: carretti, botti, casse, panni e reti (solo i propri piazzamenti).

I blocchi hanno `quando: generato da gen_paese.py`: rilanciando lo script vengono sostituiti.
Spostandoli con F4 e rilanciandolo si perde lo spostamento: dopo il primo giro non serve rilanciarlo.
Convenzioni: x = Est, z = Sud; rotazione = verso del davanti (0 = +Z/sud, 90 = +X/est).
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PL = ROOT / "debug/placements.txt"
TAG = "gen_paese.py"
ITEMS = [
    # Dentro le mura, lontano da porte e percorsi.
    ("carretto", 13.0, 20.2, 80), ("carretto", 5.2, 29.0, 170),
    ("botti", 23.0, 3.5, 0), ("botti", 12.5, -2.6, 0), ("botti", 3.6, 24.8, 0),
    ("casse", 9.4, 20.4, 90), ("casse", 7.5, -4.6, 20),
    ("panni", -13.0, 24.8, 0), ("panni", 20.0, 11.0, 90),
    # Porto.
    ("reti", 14.5, 91.8, 90), ("reti", -4.5, 90.5, 0),
    ("botti", 4.6, 89.3, 0), ("casse", 12.0, 90.0, 40), ("casse", -8.0, 91.0, 0),
]


def main():
    text = PL.read_text(encoding="utf-8")
    parts = re.split(r"(?=\n\[P-\d+\])", text)
    text = "".join(p for p in parts if TAG not in p).rstrip("\n") + "\n"
    ids = [int(i) for i in re.findall(r"^\[P-(\d+)\]", text, re.M)]
    pid = max(ids + [599]) + 1
    for kind, x, z, rot in ITEMS:
        text += (f"\n[P-{pid:04d}] tipo: {kind} | modalità: singolo\nquando: generato da {TAG}\n"
                 f"punto: x {x:.2f}, z {z:.2f}\nquantità: 1   seed: 1\nnota: paese\n"
                 f"punti: ({x:.2f},{z:.2f})\nrotazioni: {rot}\n")
        pid += 1
    PL.write_text(text, encoding="utf-8")
    print(f"{len(ITEMS)} oggetti scritti, ultimo P-{pid - 1:04d}")


if __name__ == "__main__":
    main()
