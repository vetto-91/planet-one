#!/usr/bin/env python3
"""Genera le tabelle dei cartelli del villaggio in assets/textures/cartelli/<chiave>.png.

Solo locale (PIL + font di sistema DejaVu). Per cambiare nomi/testi modifica SIGNS e rilancia.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).resolve().parent.parent / "assets/textures/cartelli"
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"
# chiave -> (testo grande, testo piccolo)
SIGNS = {
    "fontevivo": ("FONTEVIVO", "borgo murato"),
    "porta-nord": ("PORTA NORD", "Fontevivo"),
    "porta-sud": ("PORTA SUD", "Fontevivo"),
    "piazza": ("Piazza", "Fontevivo"),
    "chiesa": ("Chiesa", "San Fontino"),
    "locanda": ("Locanda", "vitto e alloggio"),
    "forno": ("Forno", "pane fresco"),
    "fabbro": ("Fabbro", ""),
    "faro": ("Faro", "a sud, sulla scogliera"),
    "porticciolo": ("Porticciolo", "barche e pesca"),
}
W, H = 512, 192


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    big = ImageFont.truetype(FONT, 78)
    small = ImageFont.truetype(FONT, 34)
    for key, (a, b) in SIGNS.items():
        im = Image.new("RGB", (W, H), (150, 104, 60))
        d = ImageDraw.Draw(im)
        for y in range(0, H, 16):  # venature del legno
            d.line([(0, y), (W, y + 3)], fill=(132, 90, 50), width=2)
        d.rectangle([6, 6, W - 7, H - 7], outline=(70, 42, 20), width=8)
        fs = big
        while d.textlength(a, font=fs) > W - 60:
            fs = ImageFont.truetype(FONT, fs.size - 6)
        ty = 28 if b else 52
        d.text((W / 2, ty + 38), a, font=fs, fill=(46, 26, 12), anchor="mm")
        if b:
            d.text((W / 2, H - 46), b, font=small, fill=(70, 42, 20), anchor="mm")
        im.save(OUT / f"{key}.png", optimize=True)
    print(f"{len(SIGNS)} cartelli generati in {OUT}")


if __name__ == "__main__":
    main()
