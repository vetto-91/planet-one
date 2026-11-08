#!/usr/bin/env python3
"""Casa delle guardie di Fontevivo (solo il suo piazzamento e la stradina, non tocca il resto).

La casa sta fuori dal cancello nord, a est della strada principale (nel villaggio non c'è più posto
libero); la porta guarda la strada e una stradina la collega. Le guardie fuori turno vanno lì a riposare.
I blocchi hanno `quando: generato da gen_guardie.py`: rilanciando lo script vengono sostituiti.
Convenzioni: x = Est, z = Sud; rotazione = verso del davanti (0 = +Z/sud, 90 = +X/est, 180 = nord, 270 = ovest).
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PL = ROOT / "debug/placements.txt"
SPEC = ROOT / "assets/world/villaggio.txt"
TAG = "gen_guardie.py"
HOUSE = ("caserma", 7.5, -16.5, 270)
# Porta: 3 m davanti al centro (profondità 6) + 0,8 m; la stradina arriva alla strada principale.
ROAD = ("piccola", 2.0, [(3.7, -16.5), (0.21, -16.5)])
MARK = "# --- strada della caserma (generata da scripts/gen_guardie.py) ---"


def main():
    text = PL.read_text(encoding="utf-8")
    parts = re.split(r"(?=\n\[P-\d+\])", text)
    text = "".join(p for p in parts if TAG not in p).rstrip("\n") + "\n"
    ids = [int(i) for i in re.findall(r"^\[P-(\d+)\]", text, re.M)]
    pid = max(ids + [499]) + 1
    kind, x, z, rot = HOUSE
    text += (f"\n[P-{pid:04d}] tipo: {kind} | modalità: singolo\nquando: generato da {TAG}\n"
             f"punto: x {x:.2f}, z {z:.2f}\nquantità: 1   seed: 1\nnota: guardie\n"
             f"punti: ({x:.2f},{z:.2f})\nrotazioni: {rot}\n")
    PL.write_text(text, encoding="utf-8")
    spec = SPEC.read_text(encoding="utf-8")
    spec = re.sub(r"\n" + re.escape(MARK) + r"\n[^\n]*\n", "\n", spec)
    kind, w, pts = ROAD
    line = f"strada: {kind} {w} | " + ", ".join(f"{a:.2f} {b:.2f}" for a, b in pts)
    marker = "# --- strada del porto"
    head, sep, tail = spec.partition(marker)
    SPEC.write_text(head.rstrip("\n") + f"\n\n{MARK}\n{line}\n\n" + sep + tail, encoding="utf-8")
    print(f"caserma scritta: P-{pid:04d}")


if __name__ == "__main__":
    main()
