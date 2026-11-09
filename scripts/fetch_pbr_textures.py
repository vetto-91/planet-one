#!/usr/bin/env python3
"""Scarica le texture PBR CC0 da Poly Haven (1k JPG) in assets_src/polyhaven/ e le copia in assets/textures/pbr/ (ADR 0016).

Serve una sola volta, in sviluppo: il gioco usa i file già copiati e non accede alla rete.
Uso: python3 -I scripts/fetch_pbr_textures.py <assets_src> <assets/textures/pbr>
"""
import io
import json
import shutil
import sys
import urllib.request
from pathlib import Path

from PIL import Image

UA = {"User-Agent": "planet-one-asset-fetch/1.0"}
# nome nel gioco: id Poly Haven
MATERIALS = {
    "grass": "leafy_grass",
    "sand": "coast_sand_01",
    "cobble": "cobblestone_floor_04",
    "plaster": "white_plaster_02",
    "planks": "weathered_brown_planks",
    "stone": "castle_wall_slates",
    "tiles": "ceramic_roof_01",
    "slate": "roof_slates_03",
    "thatch": "thatch_roof_angled",
    "cloth": "hessian_230",
    "iron": "rust_coarse_01",
}
ALLOWED_HOST = "https://dl.polyhaven.org/"


def fetch(url: str) -> bytes:
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=60) as r:
        return r.read()


def save(url: str, dest: Path) -> None:
    if not url.startswith(ALLOWED_HOST):
        raise SystemExit(f"URL inatteso: {url}")
    if not dest.exists():
        data = fetch(url)
        Image.open(io.BytesIO(data)).verify()  # deve essere davvero un'immagine
        dest.write_bytes(data)
    print(dest.name, round(dest.stat().st_size / 1e6, 2), "MB")


def main() -> None:
    src, out = Path(sys.argv[1]).resolve() / "polyhaven", Path(sys.argv[2]).resolve()
    src.mkdir(parents=True, exist_ok=True)
    out.mkdir(parents=True, exist_ok=True)
    for name, asset in MATERIALS.items():
        files = json.loads(fetch(f"https://api.polyhaven.com/files/{asset}"))
        for key, suffix in [("Diffuse", "diff"), ("nor_gl", "nor"), ("arm", "arm")]:
            dest = src / f"{name}_{suffix}.jpg"
            save(files[key]["1k"]["jpg"]["url"], dest)
            shutil.copyfile(dest, out / dest.name)
    files = json.loads(fetch("https://api.polyhaven.com/files/fir_tree_01"))
    for key, suffix in [("bark_diff", "bark_diff"), ("bark_nor_gl", "bark_nor"), ("bark_arm", "bark_arm"), ("twig_nor_gl", "twig_nor"), ("twig_arm", "twig_arm")]:
        dest = src / f"fir_{suffix}.jpg"
        save(files[key]["1k"]["jpg"]["url"], dest)
        shutil.copyfile(dest, out / dest.name)
    for key, name in [("twig_diff", "fir_twig_col.jpg"), ("twig_alpha", "fir_twig_alpha.jpg")]:
        save(files[key]["1k"]["jpg"]["url"], src / name)
    # Rametti: colore + alfa in un solo PNG.
    rgb = Image.open(src / "fir_twig_col.jpg").convert("RGB")
    alpha = Image.open(src / "fir_twig_alpha.jpg").convert("L").resize(rgb.size)
    rgb.putalpha(alpha)
    rgb.save(out / "fir_twig_diff.png", optimize=True)


if __name__ == "__main__":
    main()
