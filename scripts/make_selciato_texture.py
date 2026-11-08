#!/usr/bin/env python3
"""Genera assets/textures/selciato.png: ciottoli low-poly continui (piastrellabile).

Solo locale (numpy + PIL). Cella di Voronoi con punti jitterati su una griglia che si ripete.
"""
import numpy as np
from pathlib import Path
from PIL import Image

SIZE = 512
GRID = 11          # ciottoli per lato
SEED = 7
rng = np.random.default_rng(SEED)

cell = SIZE / GRID
pts = []
for gy in range(GRID):
    for gx in range(GRID):
        pts.append(((gx + 0.15 + 0.7 * rng.random()) * cell, (gy + 0.15 + 0.7 * rng.random()) * cell))
pts = np.array(pts)
tones = rng.uniform(0.0, 1.0, len(pts))

ys, xs = np.mgrid[0:SIZE, 0:SIZE].astype(np.float32)
d1 = np.full((SIZE, SIZE), 1e9, np.float32)
d2 = np.full((SIZE, SIZE), 1e9, np.float32)
idx = np.zeros((SIZE, SIZE), np.int32)
for dx in (-SIZE, 0, SIZE):
    for dy in (-SIZE, 0, SIZE):
        for i, (px, py) in enumerate(pts):
            d = np.hypot(xs - (px + dx), ys - (py + dy))
            m1 = d < d1
            d2 = np.where(m1, d1, np.minimum(d2, d))
            idx = np.where(m1, i, idx)
            d1 = np.where(m1, d, d1)

edge = d2 - d1
base = np.array([[0.55, 0.52, 0.47], [0.43, 0.41, 0.40], [0.62, 0.57, 0.48], [0.50, 0.48, 0.50]])
col = base[(tones[idx] * len(base)).astype(int) % len(base)] * (0.9 + 0.2 * tones[idx])[..., None]
# Leggero rilievo: più chiaro al centro del ciottolo.
col *= (0.93 + 0.1 * np.clip(d2 - d1, 0, 12) / 12)[..., None]
mortar = np.clip(1.0 - edge / 3.5, 0, 1)[..., None]
col = col * (1 - mortar) + np.array([0.22, 0.20, 0.17]) * mortar
Image.fromarray((np.clip(col, 0, 1) * 255).astype(np.uint8)).save(Path(__file__).resolve().parent.parent / "assets/textures/selciato.png", optimize=True)
print("selciato.png generato")
