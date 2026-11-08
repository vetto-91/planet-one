#!/usr/bin/env python3
"""Strada a sud, faro e porticciolo di Fontevivo (solo piazzamenti nuovi, non tocca il resto).

La costa a sud sta a z ~ 97 per x tra -10 e 10 (calcolata dalla forma dell'isola in `terrain.rs`).
Scrive: la strada dal cancello sud al porto (in `assets/world/villaggio.txt`, blocco proprio), un faro sulla
scogliera a ovest, un molo con tre barche, una casetta di pescatori, lampioni e cartelli.
I blocchi hanno `quando: generato da gen_porto.py`: rilanciando lo script vengono sostituiti.
Convenzioni: x = Est, z = Sud; rotazione = verso del davanti (0 = +Z/sud, 90 = +X/est, 180 = nord, 270 = ovest).
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PL = ROOT / "debug/placements.txt"
SPEC = ROOT / "assets/world/villaggio.txt"
TAG = "gen_porto.py"

ITEMS = [
    ("faro", -14.0, 94.0, 90),
    ("molo", 8.5, 96.3, 0),
    ("barca", 5.9, 99.2, 4), ("barca", 11.1, 102.8, 355), ("barca", 5.9, 106.0, 8),
    ("casa-piccola", 3.2, 90.5, 270),
    ("lampione", 2.8, 56.0, 0), ("lampione", -3.0, 66.0, 0), ("lampione", 2.8, 76.0, 0),
    ("lampione", -3.0, 85.0, 0), ("lampione", 2.6, 91.5, 0), ("lampione", 6.5, 94.4, 0),
    ("lampione", -7.5, 91.8, 0),
    ("cartello:faro", -3.6, 53.0, 180), ("cartello:porticciolo", 3.6, 53.0, 180),
    ("cartello:faro", -3.4, 86.5, 180), ("cartello:porticciolo", 3.4, 86.5, 180),
]
ROADS = [
    ("grande", 4.2, [(0.41, 48.0), (0.0, 60.0), (-0.6, 72.0), (-0.4, 84.0), (0.6, 91.0), (3.5, 94.2), (8.5, 95.4)]),
    ("piccola", 2.6, [(0.6, 91.0), (-5.0, 92.4), (-10.0, 93.4), (-11.2, 94.0)]),
]


def block(pid, kind, x, z, rot, note):
    n = f"\nnota: {note}" if note else ""
    return (f"\n[P-{pid:04d}] tipo: {kind} | modalità: singolo\nquando: generato da {TAG}\n"
            f"punto: x {x:.2f}, z {z:.2f}\nquantità: 1   seed: 1{n}\npunti: ({x:.2f},{z:.2f})\nrotazioni: {rot}\n")


def main():
    text = PL.read_text(encoding="utf-8")
    parts = re.split(r"(?=\n\[P-\d+\])", text)
    text = "".join(p for p in parts if TAG not in p).rstrip("\n") + "\n"
    ids = [int(i) for i in re.findall(r"^\[P-(\d+)\]", text, re.M)]
    nxt = max(ids + [399]) + 1
    out = ""
    for kind, x, z, rot in ITEMS:
        note = "porto"
        if ":" in kind:
            kind, note = kind.split(":")
        out += block(nxt, kind, x, z, rot, note); nxt += 1
    PL.write_text(text + out, encoding="utf-8")
    spec = SPEC.read_text(encoding="utf-8")
    spec = re.split(r"\n# --- strada del porto", spec)[0].rstrip("\n")
    lines = ["# --- strada del porto (generata da scripts/gen_porto.py) ---"]
    for kind, w, pts in ROADS:
        lines.append(f"strada: {kind} {w} | " + ", ".join(f"{x:.2f} {z:.2f}" for x, z in pts))
    SPEC.write_text(spec + "\n\n" + "\n".join(lines) + "\n", encoding="utf-8")
    print(f"porto scritto: {nxt - 1} piazzamenti totali")


if __name__ == "__main__":
    main()
