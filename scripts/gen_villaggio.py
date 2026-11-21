#!/usr/bin/env python3
"""Genera il layout iniziale del villaggio (Fontevivo): edifici, lampioni, cartelli, stradine.

Scrive in debug/placements.txt i piazzamenti marcati `nota: villaggio` (quelli precedenti con la
stessa nota vengono sostituiti; gli altri non si toccano) e riscrive il blocco generato delle
strade in assets/world/villaggio.txt. Poi in gioco (F4) si sposta/ruota/elimina tutto a mano.
Con --preview salva anche un'anteprima dall'alto (scratch/villaggio_preview.png).

Convenzioni: x = Est, z = Sud. Rotazione (gradi) = verso del "davanti" (porta): 0 = +Z (sud),
90 = +X (est), 180 = -Z (nord), 270 = -X (ovest).
"""
import math
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLACEMENTS = ROOT / "debug/placements.txt"
SPEC = ROOT / "assets/world/villaggio.txt"
TAG = "villaggio"

# (tipo, x, z, rotazione, nota)
BUILDINGS = [
    ("pozzo", -1.2, 10.0, 0, ""),
    ("chiesa", -14.0, 12.0, 90, ""),
    # lungo la via Est (z ~ 10.5)
    ("locanda", 16.5, 3.6, 0, ""),
    ("casa-grande", 13.0, 16.3, 180, ""),
    ("forno", 5.5, 19.8, 270, ""),
    # lungo la via Ovest (z ~ 0.4)
    ("casa-media", -9.0, -4.8, 0, ""),
    ("casa-piccola", -17.5, -4.2, 0, ""),
    ("casa-piccola", -9.5, 4.8, 180, ""),
    ("casa-media", -19.5, 4.5, 180, ""),
    # a sud
    ("casa-media", -7.0, 21.0, 90, ""),
    ("casa-piccola", -14.5, 22.0, 180, ""),  # porta sulla via Sud-Ovest, non verso le mura
]
LAMPS = [
    (-3.0, -5.5), (2.9, -6.4), (-3.4, 0.0), (-3.4, 15.0), (3.2, 18.0), (2.4, 24.0), (-3.2, 23.0),
    (-6.8, 7.0), (4.6, 7.0), (-6.8, 13.5), (4.6, 13.5),
    (8.0, 8.4), (-12.0, 2.0), (-20.0, 2.0),
]
# cartelli: (chiave texture, x, z, rotazione)
SIGNS = [
    ("fontevivo", 4.0, -12.0, 180),
    ("porta-nord", -3.6, -12.0, 180),
    ("fontevivo", -2.8, 31.0, 0),
    ("porta-sud", 4.6, 31.0, 0),
    ("piazza", 2.6, -3.2, 180),
    ("chiesa", -4.2, 8.2, 270),
    ("locanda", 4.8, 12.5, 90),
    ("forno", 4.5, 17.2, 90),
]
# strade secondarie (polilinee): larghezza 2.4
ROADS = [
    ("piccola", 2.4, [(3.0, 10.0), (12.0, 10.5), (22.0, 10.8)]),            # via Est
    ("piccola", 2.4, [(-0.6, 0.8), (-10.0, 0.4), (-22.0, 0.4)]),            # via Ovest
    ("piccola", 2.4, [(-1.0, 17.5), (-6.0, 17.1), (-14.5, 18.6)]),          # via Sud-Ovest (fra la bancarella del pesce e la casa)
    ("piccola", 2.6, [(-5.5, 10.2), (-7.2, 12.0)]),                          # sagrato
]
FOOT = {"chiesa": 9.0, "locanda": 6.5, "fabbro": 6.0, "stalla": 5.5, "casa-grande": 5.0,
        "casa-media": 4.2, "casa": 4.2, "casa-piccola": 3.6, "forno": 3.8, "pozzo": 1.6}
DEPTH = {"chiesa": 17.0, "locanda": 6.5, "fabbro": 5.0, "stalla": 5.0, "casa-grande": 6.0,
         "casa-media": 5.0, "casa-piccola": 4.0, "forno": 4.5, "pozzo": 2.4}

MAIN = [(0.21, -32.0), (0.21, -8.3), (0.1, -3.0), (-0.5, 2.0), (-1.0, 7.0), (-1.2, 10.0), (-0.9, 16.0),
        (0.0, 22.0), (0.41, 26.5), (0.41, 48.0)]


def poly_from_placements(text):
    pts = []
    for m in re.finditer(r"^\[P-(\d+)\] tipo: muro.*?\npunti: \(([-\d.]+),([-\d.]+)\)", text, re.S | re.M):
        pts.append((int(m.group(1)), float(m.group(2)), float(m.group(3))))
    pts.sort()
    return [(x, z) for _, x, z in pts]


def inside(poly, x, z):
    c = False
    j = len(poly) - 1
    for i in range(len(poly)):
        a, b = poly[i], poly[j]
        if (a[1] > z) != (b[1] > z) and x < (b[0] - a[0]) * (z - a[1]) / (b[1] - a[1]) + a[0]:
            c = not c
        j = i
    return c


def dist_seg(p, a, b):
    ax, az = a; bx, bz = b
    dx, dz = bx - ax, bz - az
    l2 = dx * dx + dz * dz
    t = 0 if l2 == 0 else max(0, min(1, ((p[0] - ax) * dx + (p[1] - az) * dz) / l2))
    return math.dist(p, (ax + t * dx, az + t * dz))


def dist_poly(poly, p):
    return min(dist_seg(p, poly[i], poly[(i + 1) % len(poly)]) for i in range(len(poly)))


def front(rot):
    r = math.radians(rot)
    return (math.sin(r), math.cos(r))


def block(pid, kind, x, z, rot, note):
    n = f"\nnota: {note}" if note else ""
    return (f"\n[P-{pid:04d}] tipo: {kind} | modalità: singolo\nquando: generato da gen_villaggio.py\n"
            f"punto: x {x:.2f}, z {z:.2f}\nquantità: 1   seed: 1{n}\npunti: ({x:.2f},{z:.2f})\nrotazioni: {rot}\n")


def main():
    text = PLACEMENTS.read_text(encoding="utf-8")
    # Rimuove i blocchi generati in precedenza.
    parts = re.split(r"(?=\n\[P-\d+\])", text)
    kept = [p for p in parts if not re.search(r"^nota: %s$" % TAG, p, re.M) and "gen_villaggio.py" not in p]
    text = "".join(kept).rstrip("\n") + "\n"
    ids = [int(i) for i in re.findall(r"^\[P-(\d+)\]", text, re.M)]
    nxt = max(ids + [199]) + 1
    poly = poly_from_placements(text)
    if len(poly) < 3:
        sys.exit("Servono almeno 3 piazzamenti 'muro' in debug/placements.txt")
    out = ""
    problems = []
    placed = []
    for kind, x, z, rot, _ in BUILDINGS:
        d = dist_poly(poly, (x, z))
        if not inside(poly, x, z) or d < DEPTH.get(kind, 5) / 2 + 1.0:
            problems.append(f"{kind} ({x},{z}) troppo vicino al muro/fuori ({d:.1f} m)")
        for k2, x2, z2 in placed:
            if math.dist((x, z), (x2, z2)) < (FOOT[kind] + FOOT[k2]) * 0.8:
                problems.append(f"{kind} ({x},{z}) sovrapposto a {k2} ({x2},{z2})")
        for hx, hz, hr in [(6.5, 0.7, 4.5)]:
            if math.dist((x, z), (hx, hz)) < FOOT[kind] * 0.8 + hr:
                problems.append(f"{kind} ({x},{z}) troppo vicino alla casetta")
        placed.append((kind, x, z))
        out += block(nxt, kind, x, z, rot, TAG); nxt += 1
    for x, z in LAMPS:
        out += block(nxt, "lampione", x, z, 0, TAG); nxt += 1
    for key, x, z, rot in SIGNS:
        out += block(nxt, "cartello", x, z, rot, key + " villaggio".replace(" villaggio", "")) ; nxt += 1
    PLACEMENTS.write_text(text + out, encoding="utf-8")

    # Strade: porte delle case collegate alla rete.
    roads = [("grande", 4.2, MAIN), ("piazza", 11.0, [(-1.2, 10.0), (-1.2, 10.01)])] + [(k, w, p) for k, w, p in ROADS]
    net = [pt for _, _, pts in roads for pt in zip(pts, pts[1:])]
    for kind, x, z, rot, _ in BUILDINGS:
        if kind == "pozzo":
            continue
        fx, fz = front(rot)
        door = (x + fx * (DEPTH[kind] / 2 + 1.0), z + fz * (DEPTH[kind] / 2 + 1.0))
        best = min(net, key=lambda s: dist_seg(door, s[0], s[1]))
        # punto più vicino sul segmento
        ax, az = best[0]; bx, bz = best[1]
        dx, dz = bx - ax, bz - az
        l2 = dx * dx + dz * dz
        t = 0 if l2 == 0 else max(0, min(1, ((door[0] - ax) * dx + (door[1] - az) * dz) / l2))
        q = (ax + t * dx, az + t * dz)
        if math.dist(door, q) > 0.8:
            roads.append(("stradina", 1.6, [door, q]))
    lines = ["# --- strade (generate da scripts/gen_villaggio.py: rilanciare lo script le rigenera) ---"]
    for kind, w, pts in roads:
        lines.append(f"strada: {kind} {w} | " + ", ".join(f"{x:.2f} {z:.2f}" for x, z in pts))
    spec = SPEC.read_text(encoding="utf-8")
    spec = re.split(r"\n# --- strade \(generate", spec)[0].rstrip("\n")
    spec = re.sub(r"^strada:.*\n?", "", spec, flags=re.M).rstrip("\n")
    SPEC.write_text(spec + "\n\n" + "\n".join(lines) + "\n", encoding="utf-8")
    print(f"scritti {nxt - 1} piazzamenti totali, {len(roads)} strade")
    for p in problems:
        print("ATTENZIONE:", p)
    if "--preview" in sys.argv:
        from PIL import Image, ImageDraw
        sc = 10
        x0, z0, x1, z1 = -34, -36, 36, 52
        im = Image.new("RGB", ((x1 - x0) * sc, (z1 - z0) * sc), (96, 130, 70))
        d = ImageDraw.Draw(im)
        m = lambda p: ((p[0] - x0) * sc, (p[1] - z0) * sc)
        for kind, w, pts in roads:
            d.line([m(p) for p in pts], fill=(170, 160, 140), width=int(w * sc), joint="curve")
        d.polygon([m(p) for p in poly], outline=(60, 60, 60))
        d.line([m(p) for p in poly + [poly[0]]], fill=(40, 40, 40), width=8)
        d.rectangle([m((3.5, -2.3)), m((9.5, 3.7))], fill=(150, 110, 70), outline=(0, 0, 0))
        for kind, x, z, rot, _ in BUILDINGS:
            w = FOOT[kind] * 1.3 if kind != "chiesa" else 7.4
            dd = DEPTH[kind] if kind in DEPTH else 5
            if kind == "pozzo":
                d.ellipse([m((x - 1.2, z - 1.2)), m((x + 1.2, z + 1.2))], fill=(80, 120, 200))
                continue
            fx, fz = front(rot)
            hw, hd = (w if kind != "chiesa" else 7.4) / 2, dd / 2
            if kind != "chiesa":
                hw = {"casa-piccola": 2.5, "casa-media": 3.1, "casa-grande": 3.5, "locanda": 5.0, "fabbro": 4.0,
                      "forno": 2.75, "stalla": 4.5}[kind]
            ux, uz = -fz, fx
            corners = [(x + ux * a + fx * b, z + uz * a + fz * b) for a, b in [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)]]
            d.polygon([m(c) for c in corners], fill=(200, 90, 70), outline=(0, 0, 0))
            d.line([m(corners[2]), m(corners[3])], fill=(255, 255, 0), width=4)
        for x, z in LAMPS:
            d.ellipse([m((x - .3, z - .3)), m((x + .3, z + .3))], fill=(255, 230, 100))
        for key, x, z, rot in SIGNS:
            d.rectangle([m((x - .6, z - .3)), m((x + .6, z + .3))], fill=(120, 80, 40))
        out_png = Path("/tmp/villaggio_preview.png")
        im.save(out_png)
        print("anteprima:", out_png)


if __name__ == "__main__":
    main()
