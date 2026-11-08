#!/usr/bin/env python3
"""Genera le skin degli abitanti ricolorando i vestiti blu di viandante.png (solo locale).

Le skin sono sviluppi di box (vedi src/figure.rs). Si cambia solo il colore dei vestiti (pixel blu):
pelle, capelli e cinture marroni restano. Output: assets/textures/abitante-<nome>.png
"""
import numpy as np
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "assets/textures/viandante.png"

# nome -> (nuovo hue 0..255, fattore saturazione, fattore luminosità)
VARIANTS = {
    "verde": (80, 0.85, 0.95),
    "rosso": (6, 0.95, 0.95),
    "ocra": (28, 0.9, 1.05),
    "viola": (200, 0.7, 0.95),
    "prete": (150, 0.08, 0.38),     # tonaca quasi nera
    "guardia": (35, 0.28, 0.80),    # grigio-verdastro (cotta)
    "rosa": (232, 0.55, 1.1),
}


def main():
    base = Image.open(SRC).convert("RGB")
    hsv = np.array(base.convert("HSV")).astype(np.float32)
    h, s, v = hsv[..., 0], hsv[..., 1], hsv[..., 2]
    blue = (h > 135) & (h < 178) & (s > 50)
    for name, (nh, ks, kv) in VARIANTS.items():
        out = hsv.copy()
        out[..., 0] = np.where(blue, nh, h)
        out[..., 1] = np.where(blue, np.clip(s * ks, 0, 255), s)
        out[..., 2] = np.where(blue, np.clip(v * kv, 0, 255), v)
        img = Image.fromarray(out.astype(np.uint8), "HSV").convert("RGB")
        img.save(ROOT / f"assets/textures/abitante-{name}.png", optimize=True)
        print("abitante-%s.png" % name)


if __name__ == "__main__":
    main()
