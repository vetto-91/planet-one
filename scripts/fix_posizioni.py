#!/usr/bin/env python3
"""Sposta fuori dagli edifici i lampioni e i cartelli finiti dentro (v0.0.42.0).

Agisce solo sui piazzamenti che sono ancora alle coordinate segnalate: se l'utente li ha già
spostati con F4 non li tocca. Rilanciabile senza danni.
"""
import re
from pathlib import Path

PL = Path(__file__).resolve().parent.parent / "debug/placements.txt"
# id -> (da, a, rotazione)
MOVES = {
    "0221": ((4.60, 13.50), (3.30, 17.20), 0),    # lampione dentro la casa 0064
    "0231": ((4.80, 12.50), (14.50, 8.60), 180),  # cartello della locanda dentro la casa
    "0220": ((-6.80, 13.50), (-3.80, 15.00), 0),  # lampione dentro la chiesa
    "0601": ((5.20, 29.00), (7.50, 33.00), 120),   # carretto contro la torre della porta sud
    "0410": ((2.60, 91.50), (5.60, 93.60), 0),    # lampione dentro la casa del porto
}


def main():
    text = PL.read_text(encoding="utf-8")
    parts = re.split(r"(?=\n\[P-\d+\])", text)
    done = 0
    for k, p in enumerate(parts):
        m = re.match(r"\n?\[P-(\d+)\]", p)
        if not m or m.group(1) not in MOVES:
            continue
        old, new, rot = MOVES[m.group(1)]
        needle = f"({old[0]:.2f},{old[1]:.2f})"
        if needle not in p:
            continue
        p = p.replace(needle, f"({new[0]:.2f},{new[1]:.2f})")
        p = re.sub(r"punto: x [-\d.]+, z [-\d.]+", f"punto: x {new[0]:.2f}, z {new[1]:.2f}", p)
        p = re.sub(r"rotazioni: .*", f"rotazioni: {rot}", p)
        parts[k] = p
        done += 1
    PL.write_text("".join(parts), encoding="utf-8")
    print(f"{done} oggetti spostati")


if __name__ == "__main__":
    main()
