#!/usr/bin/env python3
"""Aggiunge mercato, botteghe e campi a Fontevivo (solo piazzamenti nuovi, non tocca il resto).

- trasforma due case in botteghe: P-0205 -> fabbro (armaiolo), P-0206 -> arcere (solo se hanno ancora il tipo casa);
- 3 bancarelle in piazza, pagliai, spaventapasseri e 3 campi (grano/orto) fuori dalle mura.
I blocchi generati hanno `quando: generato da gen_mercato.py`: rilanciando lo script vengono sostituiti.
Convenzioni: x = Est, z = Sud; rotazione = verso del davanti (0 = +Z/sud, 90 = +X/est, 180 = nord, 270 = ovest).
"""
import math
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PL = ROOT / "debug/placements.txt"
TAG = "gen_mercato.py"

STALLS = [("bancarella-erbe", -5.2, 7.4, 90), ("bancarella-frutta", 3.6, 8.6, 270), ("bancarella-pesce", -5.2, 15.4, 90)]
EXTRAS = [("pagliaio", -6.5, -22.0, 0), ("pagliaio", -5.2, -25.5, 40), ("pagliaio", -8.5, 41.0, 20),
          ("spaventapasseri", -18.0, -26.0, 0), ("spaventapasseri", -20.0, -16.0, 30), ("spaventapasseri", -20.0, 40.5, 0)]
FIELDS = [("campo", [(-27, -33), (-9, -33), (-9, -21), (-27, -21)]),
          ("orto", [(-27, -19), (-13, -19), (-13, -12.5), (-27, -12.5)]),
          ("campo", [(-30, 34), (-11, 34), (-11, 47), (-30, 47)])]
SPACING = 0.95


def inside(poly, x, z):
    c = False
    j = len(poly) - 1
    for i in range(len(poly)):
        a, b = poly[i], poly[j]
        if (a[1] > z) != (b[1] > z) and x < (b[0] - a[0]) * (z - a[1]) / (b[1] - a[1]) + a[0]:
            c = not c
        j = i
    return c


def block(pid, kind, x, z, rot):
    return (f"\n[P-{pid:04d}] tipo: {kind} | modalità: singolo\nquando: generato da {TAG}\n"
            f"punto: x {x:.2f}, z {z:.2f}\nquantità: 1   seed: 1\nnota: mercato\npunti: ({x:.2f},{z:.2f})\nrotazioni: {rot}\n")


def main():
    text = PL.read_text(encoding="utf-8")
    parts = re.split(r"(?=\n\[P-\d+\])", text)
    text = "".join(p for p in parts if TAG not in p).rstrip("\n") + "\n"
    # Botteghe: cambia il tipo di due case generate.
    for pid, old, new in [("0205", "casa-media", "fabbro"), ("0206", "casa-piccola", "arcere")]:
        text = re.sub(rf"(\[P-{pid}\] tipo: ){old}", rf"\g<1>{new}", text)
    ids = [int(i) for i in re.findall(r"^\[P-(\d+)\]", text, re.M)]
    nxt = max(ids + [299]) + 1
    out = ""
    for kind, x, z, rot in STALLS + EXTRAS:
        out += block(nxt, kind, x, z, rot); nxt += 1
    for kind, poly in FIELDS:
        xs, zs = [p[0] for p in poly], [p[1] for p in poly]
        pts = []
        z = min(zs) + SPACING / 2
        while z < max(zs):
            x = min(xs) + SPACING / 2
            while x < max(xs):
                if inside(poly, x, z):
                    pts.append((x, z))
                x += SPACING
            z += SPACING
        corners = " ".join(f"({x:.2f},{z:.2f})" for x, z in poly)
        out += (f"\n[P-{nxt:04d}] tipo: {kind} | modalità: area-poligono\nquando: generato da {TAG}\n"
                f"angoli: {corners}\nquantità: {len(pts)}   seed: 1\nnota: mercato\n"
                f"punti: {' '.join(f'({x:.2f},{z:.2f})' for x, z in pts)}\n")
        nxt += 1
    PL.write_text(text + out, encoding="utf-8")
    print(f"mercato scritto: {nxt - 1} piazzamenti totali")


if __name__ == "__main__":
    main()
