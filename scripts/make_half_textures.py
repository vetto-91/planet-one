#!/usr/bin/env python3
"""Copie a metà risoluzione delle texture dei materiali (ADR 0057).

Le schede video integrate usano la RAM di sistema: con «Texture ridotte» (menu Esc → Grafica, o il
preset «Integrata») il gioco carica `assets/textures/pbr/half/` invece di `assets/textures/pbr/`.

Uso: python3 scripts/make_half_textures.py   (da rilanciare dopo aver cambiato le texture PBR)
"""

from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "assets" / "textures" / "pbr"
DST = SRC / "half"


def main() -> None:
    DST.mkdir(exist_ok=True)
    count = 0
    for src in sorted(SRC.iterdir()):
        if not src.is_file() or src.suffix.lower() not in (".jpg", ".jpeg", ".png"):
            continue
        with Image.open(src) as im:
            size = (max(1, im.width // 2), max(1, im.height // 2))
            half = im.resize(size, getattr(Image, "Resampling", Image).LANCZOS)
            out = DST / src.name
            if src.suffix.lower() == ".png":
                half.save(out, optimize=True)
            else:
                half.save(out, quality=90)
        count += 1
    print(f"{count} texture ridotte in {DST.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
