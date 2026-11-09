#!/usr/bin/env python3
"""Disegna le icone degli oggetti (assets/textures/icone/*.png, 128 px con trasparenza).

Tutto generato in locale con PIL. I nomi dei file corrispondono a `ItemKind::icon()` in `src/inventory.rs`.
Uso: python3 -I scripts/make_item_icons.py assets/textures/icone
"""
import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

S = 512  # lato di lavoro, ridotto a 128 alla fine


def layer():
    return Image.new("RGBA", (S, S), (0, 0, 0, 0))


def shaded_polygon(d, pts, light, dark):
    """Poligono diviso in due metà lungo l'asse verticale, per un effetto di rilievo."""
    d.polygon(pts, fill=dark)
    cx = sum(p[0] for p in pts) / len(pts)
    left = [(min(x, cx), y) for x, y in pts]
    d.polygon(left, fill=light)


def sword(length, width, guard_w):
    im = layer()
    d = ImageDraw.Draw(im)
    cx, tip = S / 2, S / 2 - length / 2
    base = tip + length * 0.72
    shaded_polygon(d, [(cx, tip), (cx + width / 2, tip + 44), (cx + width / 2, base), (cx - width / 2, base), (cx - width / 2, tip + 44)], (232, 238, 246, 255), (150, 160, 176, 255))
    d.line([(cx, tip + 30), (cx, base)], fill=(120, 130, 146, 255), width=4)
    d.rounded_rectangle([cx - guard_w / 2, base, cx + guard_w / 2, base + 24], 8, fill=(214, 170, 62, 255), outline=(136, 98, 28, 255), width=4)
    d.rectangle([cx - 15, base + 24, cx + 15, base + length * 0.22], fill=(92, 58, 32, 255))
    for k in range(4):
        y = base + 34 + k * (length * 0.22 - 30) / 4
        d.line([(cx - 15, y), (cx + 15, y + 10)], fill=(58, 36, 20, 255), width=5)
    py = base + length * 0.22 + 14
    d.ellipse([cx - 22, py - 20, cx + 22, py + 24], fill=(214, 170, 62, 255), outline=(136, 98, 28, 255), width=4)
    return im.rotate(45, resample=Image.BICUBIC, center=(S / 2, S / 2))


def axe():
    im = layer()
    d = ImageDraw.Draw(im)
    cx = S / 2 - 30
    d.rounded_rectangle([cx - 13, 60, cx + 13, 462], 10, fill=(122, 80, 42, 255), outline=(70, 44, 22, 255), width=4)
    head = [(cx, 96), (cx + 96, 62), (cx + 150, 150), (cx + 110, 246), (cx, 196)]
    d.polygon(head, fill=(140, 150, 166, 255), outline=(78, 86, 100, 255))
    d.polygon([(cx + 100, 72), (cx + 150, 150), (cx + 116, 234), (cx + 96, 150)], fill=(226, 232, 242, 255))
    d.rectangle([cx - 20, 110, cx + 20, 186], fill=(84, 90, 104, 255))
    return im.rotate(-28, resample=Image.BICUBIC, center=(S / 2, S / 2))


def shield():
    im = layer()
    d = ImageDraw.Draw(im)
    cx = S / 2
    outline = [(cx - 170, 70), (cx + 170, 70), (cx + 170, 250)]
    for t in range(1, 25):
        a = t / 24
        outline.append((cx + 170 * (1 - a) ** 0.72, 250 + 210 * a))
    for t in range(24, 0, -1):
        a = t / 24
        outline.append((cx - 170 * (1 - a) ** 0.72, 250 + 210 * a))
    outline.append((cx - 170, 250))
    d.polygon(outline, fill=(214, 170, 62, 255))
    inner = [(cx + (x - cx) * 0.86, 76 + (y - 70) * 0.88) for x, y in outline]
    d.polygon(inner, fill=(44, 86, 170, 255))
    half = [(x, y) for x, y in inner if x <= cx + 1]
    d.polygon(half, fill=(66, 116, 208, 255))
    d.rectangle([cx - 14, 76, cx + 14, 430], fill=(214, 170, 62, 255))
    d.rectangle([cx - 150, 196, cx + 150, 224], fill=(214, 170, 62, 255))
    d.ellipse([cx - 44, 166, cx + 44, 254], fill=(240, 208, 112, 255), outline=(136, 98, 28, 255), width=5)
    return im


def bow():
    im = layer()
    d = ImageDraw.Draw(im)
    top, bottom = (170, 40), (170, 472)
    pts = []
    for k in range(61):
        t = k / 60
        x = 170 + 200 * math.sin(math.pi * t)
        pts.append((x, 40 + 432 * t))
    for w, col in [(30, (70, 44, 22, 255)), (20, (150, 102, 54, 255))]:
        d.line(pts, fill=col, width=w, joint="curve")
    d.line([top, bottom], fill=(232, 228, 212, 255), width=6)
    d.rounded_rectangle([348, 222, 392, 290], 8, fill=(62, 40, 24, 255))
    return im


def arrow(im, x0, y0, x1, y1):
    d = ImageDraw.Draw(im)
    ang = math.atan2(y1 - y0, x1 - x0)
    ux, uy = math.cos(ang), math.sin(ang)
    nx, ny = -uy, ux
    d.line([(x0, y0), (x1, y1)], fill=(170, 124, 72, 255), width=14)
    d.polygon([(x1 + ux * 60, y1 + uy * 60), (x1 + nx * 26, y1 + ny * 26), (x1 - nx * 26, y1 - ny * 26)], fill=(196, 204, 216, 255), outline=(100, 108, 122, 255))
    for k in range(3):
        bx, by = x0 + ux * (6 + k * 26), y0 + uy * (6 + k * 26)
        d.polygon([(bx, by), (bx + nx * 34 - ux * 22, by + ny * 34 - uy * 22), (bx + nx * 34 + ux * 8, by + ny * 34 + uy * 8), (bx + ux * 30, by + uy * 30)], fill=(214, 72, 58, 255))
        d.polygon([(bx, by), (bx - nx * 34 - ux * 22, by - ny * 34 - uy * 22), (bx - nx * 34 + ux * 8, by - ny * 34 + uy * 8), (bx + ux * 30, by + uy * 30)], fill=(170, 48, 40, 255))


def arrows():
    im = layer()
    arrow(im, 90, 420, 400, 110)
    arrow(im, 60, 300, 330, 90)
    arrow(im, 210, 450, 440, 200)
    return im


def potion():
    im = layer()
    d = ImageDraw.Draw(im)
    cx = S / 2
    d.ellipse([cx - 150, 176, cx + 150, 470], fill=(190, 214, 228, 150), outline=(210, 228, 240, 255), width=8)
    d.rectangle([cx - 48, 96, cx + 48, 212], fill=(190, 214, 228, 170), outline=(210, 228, 240, 255), width=6)
    d.chord([cx - 136, 192, cx + 136, 456], 8, 172, fill=(214, 38, 48, 255))
    d.chord([cx - 136, 192, cx + 136, 456], 90, 172, fill=(240, 66, 72, 255))
    d.rounded_rectangle([cx - 56, 50, cx + 56, 118], 10, fill=(158, 112, 66, 255), outline=(96, 64, 34, 255), width=5)
    d.ellipse([cx - 104, 230, cx - 52, 330], fill=(255, 255, 255, 170))
    return im


def apple():
    im = layer()
    d = ImageDraw.Draw(im)
    cx = S / 2
    d.ellipse([cx - 174, 150, cx + 12, 462], fill=(206, 34, 42, 255))
    d.ellipse([cx - 12, 150, cx + 174, 462], fill=(170, 20, 34, 255))
    d.ellipse([cx - 80, 176, cx + 80, 440], fill=(190, 26, 38, 255))
    d.ellipse([cx - 134, 206, cx - 74, 310], fill=(255, 150, 150, 200))
    d.line([(cx, 182), (cx + 18, 92)], fill=(92, 58, 30, 255), width=18)
    d.polygon([(cx + 18, 118), (cx + 96, 50), (cx + 156, 76), (cx + 104, 140)], fill=(84, 168, 62, 255), outline=(50, 112, 40, 255))
    return im


def fish():
    im = layer()
    d = ImageDraw.Draw(im)
    d.polygon([(380, 256), (492, 158), (466, 256), (492, 354)], fill=(84, 124, 168, 255), outline=(50, 84, 122, 255))
    d.polygon([(220, 170), (310, 90), (350, 176)], fill=(84, 124, 168, 255))
    d.polygon([(240, 340), (320, 408), (340, 336)], fill=(84, 124, 168, 255))
    d.ellipse([34, 142, 410, 370], fill=(120, 168, 208, 255), outline=(60, 96, 138, 255), width=6)
    d.chord([34, 142, 410, 370], 0, 180, fill=(212, 228, 238, 255))
    for k in range(6):
        x = 190 + k * 34
        d.arc([x - 22, 190, x + 22, 240], 280, 80, fill=(70, 110, 150, 255), width=4)
    d.arc([96, 180, 170, 330], 300, 60, fill=(60, 96, 138, 255), width=7)
    d.ellipse([84, 200, 124, 240], fill=(250, 250, 250, 255), outline=(30, 40, 50, 255), width=4)
    d.ellipse([98, 212, 116, 232], fill=(16, 20, 26, 255))
    return im


def coin():
    im = layer()
    d = ImageDraw.Draw(im)
    c = S / 2
    d.ellipse([c - 190, c - 190, c + 190, c + 190], fill=(232, 182, 58, 255), outline=(146, 104, 24, 255), width=10)
    d.ellipse([c - 150, c - 150, c + 150, c + 150], fill=(250, 214, 104, 255), outline=(190, 142, 40, 255), width=7)
    star = []
    for k in range(10):
        r = 104 if k % 2 == 0 else 44
        a = -math.pi / 2 + k * math.pi / 5
        star.append((c + r * math.cos(a), c + r * math.sin(a)))
    d.polygon(star, fill=(196, 146, 40, 255))
    d.arc([c - 172, c - 172, c + 172, c + 172], 200, 300, fill=(255, 244, 200, 255), width=10)
    return im


def master_key():
    im = layer()
    d = ImageDraw.Draw(im)
    gold, dark = (232, 190, 80, 255), (136, 98, 28, 255)
    d.ellipse([96, 66, 276, 246], outline=dark, width=44)
    d.ellipse([106, 76, 266, 236], outline=gold, width=24)
    d.polygon([(218, 214), (250, 186), (420, 356), (388, 388)], fill=gold, outline=dark)
    d.polygon([(346, 346), (378, 314), (436, 372), (404, 404)], fill=gold, outline=dark)
    d.polygon([(292, 292), (324, 260), (382, 318), (350, 350)], fill=gold, outline=dark)
    return im


ICONS = {
    "spada-corta": lambda: sword(310, 58, 124),
    "spada-lunga": lambda: sword(430, 46, 150),
    "ascia": axe,
    "scudo": shield,
    "arco": bow,
    "frecce": arrows,
    "pozione": potion,
    "mela": apple,
    "pesce": fish,
    "moneta": coin,
    "passepartout": master_key,
}


def finish(im):
    alpha = im.getchannel("A").filter(ImageFilter.GaussianBlur(12)).point(lambda v: int(v * 0.55))
    shadow = Image.new("RGBA", im.size, (0, 0, 0, 0))
    shadow.putalpha(alpha)
    out = Image.new("RGBA", im.size, (0, 0, 0, 0))
    out.alpha_composite(shadow, (8, 12))
    out.alpha_composite(im)
    return out.resize((128, 128), Image.LANCZOS)


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for name, make in ICONS.items():
        finish(make()).save(out / f"{name}.png", optimize=True)
    print(len(ICONS), "icone scritte in", out)


if __name__ == "__main__":
    main()
