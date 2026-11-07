#!/usr/bin/env python3
"""Genera assets/textures/pietra.png: muratura di pietra a corsi, tileable (256x256)."""
import numpy as np
from PIL import Image

rng = np.random.default_rng(7)
N = 256
img = np.zeros((N, N, 3), dtype=np.float32)
mortar = np.array([62, 60, 56], dtype=np.float32)

# Corsi di altezza 40..56 px che sommano a 256; in ogni corso blocchi di larghezza 56..100 che sommano a 256.
def split(total, lo, hi):
    parts = []
    while total > 0:
        p = int(rng.integers(lo, hi + 1))
        if total - p < lo:
            p = total
        parts.append(p)
        total -= p
    return parts

y = 0
for h in split(N, 40, 56):
    x = int(rng.integers(0, 40))
    for w in split(N, 56, 100):
        base = rng.uniform(0.62, 0.92)
        col = np.array([150, 146, 138], dtype=np.float32) * base
        col += rng.normal(0, 4, 3)
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                img[yy % N, xx % N] = col
        # fughe (malta) sui bordi
        for yy in range(y, y + h):
            for t in range(3):
                img[yy % N, (x + t) % N] = mortar
        for xx in range(x, x + w):
            for t in range(3):
                img[(y + t) % N, xx % N] = mortar
        # leggera ombreggiatura in alto a sinistra / chiaro in basso a destra
        for yy in range(y + 3, y + h):
            for xx in range(x + 3, x + w):
                k = 1.0 + 0.10 * (1 - (yy - y) / h) * 0.5 - 0.05 * (xx - x) / w
                img[yy % N, xx % N] *= k
        x += w
    y += h

# rumore fine
img += rng.normal(0, 3.5, img.shape)
Image.fromarray(np.clip(img, 0, 255).astype(np.uint8)).save("assets/textures/pietra.png", optimize=True)
print("scritto assets/textures/pietra.png")
