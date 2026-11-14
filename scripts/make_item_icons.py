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


def lockpick():
    """Due ferri sottili di bronzo incrociati: il grimaldello e il tensore."""
    im = layer()
    d = ImageDraw.Draw(im)
    light, dark, wood = (214, 160, 92, 255), (120, 78, 34, 255), (92, 58, 32, 255)
    # Tensore: una L piatta.
    d.line([(120, 400), (392, 128)], fill=dark, width=34)
    d.line([(120, 400), (392, 128)], fill=light, width=20)
    d.line([(392, 128), (430, 166)], fill=dark, width=34)
    d.line([(392, 128), (430, 166)], fill=light, width=20)
    # Grimaldello: manico di legno e punta a uncino.
    d.line([(100, 150), (210, 260)], fill=(58, 36, 20, 255), width=56)
    d.line([(100, 150), (210, 260)], fill=wood, width=40)
    d.line([(210, 260), (400, 430)], fill=dark, width=26)
    d.line([(210, 260), (400, 430)], fill=light, width=14)
    d.line([(400, 430), (428, 396)], fill=dark, width=26)
    d.line([(400, 430), (428, 396)], fill=light, width=14)
    return im


def wood():
    """Tre tronchi accatastati, con gli anelli sulla testa."""
    im = layer()
    d = ImageDraw.Draw(im)
    bark, dark, heart = (132, 90, 52, 255), (78, 50, 26, 255), (214, 176, 120, 255)
    for cx, cy in [(190, 330), (330, 330), (260, 215)]:
        d.rounded_rectangle([cx - 150, cy - 62, cx + 110, cy + 62], 60, fill=bark, outline=dark, width=8)
        d.ellipse([cx + 48, cy - 62, cx + 172, cy + 62], fill=heart, outline=dark, width=8)
        d.ellipse([cx + 78, cy - 32, cx + 142, cy + 32], outline=(170, 128, 78, 255), width=6)
        d.line([(cx - 110, cy - 20), (cx + 30, cy - 20)], fill=dark, width=5)
    return im


def stone():
    """Un blocco di pietra squadrato, a tre facce."""
    im = layer()
    d = ImageDraw.Draw(im)
    top, left, right, edge = (196, 198, 204, 255), (150, 152, 160, 255), (112, 114, 124, 255), (70, 72, 82, 255)
    d.polygon([(256, 110), (430, 190), (256, 270), (82, 190)], fill=top, outline=edge)
    d.polygon([(82, 190), (256, 270), (256, 420), (82, 340)], fill=left, outline=edge)
    d.polygon([(430, 190), (256, 270), (256, 420), (430, 340)], fill=right, outline=edge)
    d.line([(150, 250), (190, 330)], fill=edge, width=5)
    d.line([(330, 300), (380, 270)], fill=edge, width=5)
    return im


def relic_tablet():
    im = layer(); d = ImageDraw.Draw(im)
    d.rounded_rectangle([120, 90, 392, 430], 26, fill=(176, 150, 112, 255), outline=(98, 78, 52, 255), width=10)
    for k in range(6):
        y = 140 + k * 46
        d.line([(160, y), (352 - (k % 3) * 40, y)], fill=(96, 72, 46, 255), width=9)
    return im


def relic_coin(rim, face, mark):
    im = layer(); d = ImageDraw.Draw(im)
    d.ellipse([86, 86, 426, 426], fill=rim, outline=(110, 80, 20, 255), width=10)
    d.ellipse([136, 136, 376, 376], fill=face)
    d.polygon([(256, 170), (330, 300), (182, 300)], outline=mark, width=12)
    d.ellipse([236, 236, 276, 276], fill=mark)
    return im


def relic_quipu(n):
    im = layer(); d = ImageDraw.Draw(im)
    d.line([(80, 120), (432, 120)], fill=(120, 84, 48, 255), width=20)
    colors = [(200, 60, 50, 255), (60, 120, 190, 255), (220, 180, 60, 255), (70, 150, 80, 255)]
    for k in range(n):
        x = 100 + k * (312 // max(n - 1, 1))
        c = colors[k % len(colors)]
        d.line([(x, 120), (x + 10, 410)], fill=c, width=10)
        for j in range(1 + k % 3):
            y = 190 + j * 70 + (k * 23) % 40
            d.ellipse([x - 14, y - 14, x + 18, y + 18], fill=c, outline=(60, 40, 20, 255), width=4)
    return im


def relic_disc():
    im = layer(); d = ImageDraw.Draw(im)
    d.ellipse([70, 70, 442, 442], fill=(92, 140, 112, 255), outline=(52, 86, 66, 255), width=12)
    pts = [(256 + math.cos(t / 6) * t * 0.9, 256 + math.sin(t / 6) * t * 0.9) for t in range(0, 180, 3)]
    d.line(pts, fill=(170, 210, 180, 255), width=8)
    return im


def relic_tags():
    im = layer(); d = ImageDraw.Draw(im)
    d.arc([140, 40, 372, 260], 200, 340, fill=(150, 150, 160, 255), width=8)
    for k, x in enumerate([150, 236, 322]):
        d.rounded_rectangle([x - 46, 200 + k * 20, x + 46, 380 + k * 20], 18, fill=(170, 172, 182, 255), outline=(90, 92, 102, 255), width=8)
        d.line([(x - 22, 260 + k * 20), (x + 22, 260 + k * 20)], fill=(90, 92, 102, 255), width=8)
    return im


def relic_bell():
    im = layer(); d = ImageDraw.Draw(im)
    d.polygon([(186, 120), (326, 120), (390, 400), (122, 400)], fill=(214, 168, 70, 255), outline=(130, 92, 30, 255))
    d.rectangle([226, 80, 286, 124], fill=(180, 110, 60, 255))
    d.ellipse([226, 380, 286, 440], fill=(120, 80, 30, 255))
    d.line([(150, 330), (362, 330)], fill=(190, 110, 60, 255), width=14)
    return im


def relic_compass():
    im = layer(); d = ImageDraw.Draw(im)
    d.ellipse([80, 80, 432, 432], fill=(200, 160, 80, 255), outline=(120, 90, 40, 255), width=14)
    d.ellipse([120, 120, 392, 392], fill=(236, 230, 210, 255))
    d.polygon([(256, 140), (286, 256), (256, 372), (226, 256)], fill=(60, 60, 70, 255))
    d.polygon([(256, 140), (286, 256), (226, 256)], fill=(200, 50, 50, 255))
    d.ellipse([240, 240, 272, 272], fill=(190, 30, 40, 255))
    return im


def relic_key():
    im = layer(); d = ImageDraw.Draw(im)
    green, dark = (90, 160, 130, 255), (40, 90, 70, 255)
    d.ellipse([90, 90, 250, 250], outline=dark, width=40)
    d.ellipse([100, 100, 240, 240], outline=green, width=22)
    d.polygon([(220, 210), (250, 180), (420, 350), (390, 380)], fill=green, outline=dark)
    d.polygon([(350, 330), (380, 300), (430, 350), (400, 380)], fill=green, outline=dark)
    return im


def relic_bulb():
    im = layer(); d = ImageDraw.Draw(im)
    d.ellipse([136, 70, 376, 310], fill=(255, 240, 170, 220), outline=(180, 150, 70, 255), width=8)
    d.line([(226, 230), (246, 170), (266, 230), (286, 170)], fill=(220, 120, 40, 255), width=6)
    d.rectangle([196, 300, 316, 400], fill=(160, 160, 170, 255), outline=(90, 90, 100, 255), width=6)
    for y in (330, 360):
        d.line([(196, y), (316, y)], fill=(90, 90, 100, 255), width=6)
    return im


def relic_map(ink):
    im = layer(); d = ImageDraw.Draw(im)
    d.rectangle([110, 110, 402, 402], fill=(226, 206, 160, 255), outline=(150, 120, 80, 255), width=8)
    d.ellipse([80, 90, 140, 422], fill=(206, 182, 132, 255), outline=(150, 120, 80, 255), width=6)
    d.ellipse([372, 90, 432, 422], fill=(206, 182, 132, 255), outline=(150, 120, 80, 255), width=6)
    pts = [(150 + k * 30, 256 + math.sin(k) * 60) for k in range(9)]
    d.line(pts, fill=ink, width=10)
    d.line([(160, 170), (340, 340)], fill=ink, width=5)
    return im


def relic_focaccia():
    im = layer(); d = ImageDraw.Draw(im)
    d.ellipse([80, 140, 432, 380], fill=(214, 168, 96, 255), outline=(150, 100, 50, 255), width=10)
    for x, y in [(180, 220), (260, 200), (340, 230), (220, 290), (300, 300)]:
        d.ellipse([x - 16, y - 12, x + 16, y + 12], fill=(170, 120, 60, 255))
    return im


def relic_feather():
    im = layer(); d = ImageDraw.Draw(im)
    d.line([(140, 420), (370, 90)], fill=(140, 120, 90, 255), width=10)
    for k in range(14):
        t = k / 13
        x, y = 140 + (370 - 140) * t, 420 + (90 - 420) * t
        d.line([(x, y), (x - 90 * (1 - t * 0.6), y - 30)], fill=(236, 240, 246, 255), width=14)
        d.line([(x, y), (x + 60 * (1 - t * 0.6), y + 40)], fill=(220, 226, 236, 255), width=14)
    d.ellipse([110, 400, 170, 460], outline=(200, 160, 80, 255), width=10)
    return im


def relic_book(cover, band):
    im = layer(); d = ImageDraw.Draw(im)
    d.rounded_rectangle([110, 80, 402, 432], 18, fill=cover, outline=(60, 40, 24, 255), width=10)
    d.rectangle([110, 80, 150, 432], fill=band)
    d.line([(190, 180), (360, 180)], fill=(230, 210, 170, 255), width=10)
    d.line([(190, 230), (330, 230)], fill=(230, 210, 170, 255), width=10)
    return im


def relic_seedbox():
    im = layer(); d = ImageDraw.Draw(im)
    d.rectangle([100, 200, 412, 400], fill=(150, 104, 60, 255), outline=(80, 52, 28, 255), width=10)
    d.polygon([(100, 200), (160, 130), (452, 130), (412, 200)], fill=(176, 126, 76, 255), outline=(80, 52, 28, 255))
    for x, y in [(170, 260), (230, 300), (300, 250), (350, 330), (250, 360)]:
        d.ellipse([x - 16, y - 10, x + 16, y + 10], fill=(232, 196, 70, 255))
    return im


def relic_letter():
    im = layer(); d = ImageDraw.Draw(im)
    d.polygon([(100, 140), (412, 120), (420, 380), (110, 400)], fill=(236, 220, 180, 255), outline=(150, 120, 80, 255))
    for k in range(5):
        y = 180 + k * 40
        d.line([(140, y), (380 - (k % 2) * 50, y - 4)], fill=(90, 70, 50, 255), width=6)
    d.ellipse([330, 320, 390, 380], fill=(170, 40, 40, 255))
    return im


def armor(body, edge, rings):
    im = layer(); d = ImageDraw.Draw(im)
    pts = [(150, 90), (210, 120), (302, 120), (362, 90), (430, 160), (380, 210), (370, 430), (142, 430), (132, 210), (82, 160)]
    d.polygon(pts, fill=body, outline=edge)
    d.line(pts + [pts[0]], fill=edge, width=10)
    d.arc([206, 80, 306, 170], 0, 180, fill=edge, width=10)
    if rings:
        for y in range(200, 420, 26):
            for x in range(160, 360, 26):
                d.ellipse([x, y, x + 18, y + 18], outline=edge, width=3)
    else:
        for y in (240, 300, 360):
            d.line([(150, y), (362, y)], fill=edge, width=8)
    return im


def pack(body, flap, strap, wide=1.0, tall=1.0):
    im = layer()
    d = ImageDraw.Draw(im)
    c = S / 2
    hw, top, bottom = 150 * wide, c - 170 * tall, c + 190 * tall
    d.arc([c - 80, top - 66, c + 80, top + 70], 180, 360, fill=strap, width=22)
    d.rounded_rectangle([c - hw, top, c + hw, bottom], 46, fill=body, outline=(40, 28, 16, 255), width=10)
    d.rounded_rectangle([c - hw - 8, top - 6, c + hw + 8, top + (bottom - top) * 0.42], 40, fill=flap, outline=(40, 28, 16, 255), width=10)
    d.rounded_rectangle([c - 26, top + (bottom - top) * 0.32, c + 26, top + (bottom - top) * 0.52], 8, fill=(214, 170, 62, 255), outline=(136, 98, 28, 255), width=5)
    d.rounded_rectangle([c - hw * 0.62, c + 70 * tall, c + hw * 0.62, bottom - 26], 18, outline=(40, 28, 16, 200), width=7)
    return im


def potion_of(color, dark):
    im = layer()
    d = ImageDraw.Draw(im)
    c = S / 2
    d.rectangle([c - 44, 70, c + 44, 170], fill=(206, 224, 236, 200), outline=(90, 110, 130, 255), width=7)
    d.ellipse([c - 150, 140, c + 150, 440], fill=(206, 224, 236, 220), outline=(90, 110, 130, 255), width=9)
    d.chord([c - 138, 152, c + 138, 428], 8, 172, fill=color, outline=dark, width=6)
    d.rounded_rectangle([c - 54, 46, c + 54, 96], 12, fill=(130, 90, 52, 255), outline=(70, 44, 22, 255), width=6)
    d.ellipse([c - 92, 212, c - 40, 300], fill=(255, 255, 255, 110))
    return im


def belt():
    im = layer()
    d = ImageDraw.Draw(im)
    c = S / 2
    d.arc([60, 110, 452, 402], 0, 360, fill=(104, 66, 34, 255), width=62)
    d.arc([76, 126, 436, 386], 0, 360, fill=(70, 44, 22, 255), width=6)
    d.rounded_rectangle([c - 74, 196, c + 74, 316], 22, fill=(214, 170, 62, 255), outline=(136, 98, 28, 255), width=9)
    star = []
    for k in range(10):
        r = 44 if k % 2 == 0 else 18
        a = -math.pi / 2 + k * math.pi / 5
        star.append((c + r * math.cos(a), 256 + r * math.sin(a)))
    d.polygon(star, fill=(150, 90, 220, 255), outline=(90, 50, 150, 255))
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
    "bisaccia": lambda: pack((156, 120, 74, 255), (134, 100, 58, 255), (92, 66, 36, 255), 0.72, 0.62),
    "zaino-tela": lambda: pack((168, 164, 124, 255), (142, 138, 100, 255), (100, 82, 50, 255), 0.84, 0.8),
    "zaino-cuoio": lambda: pack((130, 78, 42, 255), (104, 60, 30, 255), (70, 44, 22, 255), 0.94, 0.92),
    "zaino-spedizione": lambda: pack((86, 112, 84, 255), (66, 90, 66, 255), (60, 46, 28, 255), 1.08, 1.06),
    "pozione-mulo": lambda: potion_of((70, 130, 230, 255), (34, 70, 140, 255)),
    "cintura": belt,
    "grimaldello": lockpick,
    "legname": wood,
    "pietra": stone,
    "armatura-cuoio": lambda: armor((140, 92, 52, 255), (80, 50, 26, 255), False),
    "armatura-maglia": lambda: armor((150, 154, 162, 255), (90, 94, 104, 255), True),
    "armatura-piastre": lambda: armor((196, 200, 208, 255), (100, 104, 116, 255), False),
    "tavoletta": relic_tablet,
    "moneta-madre": lambda: relic_coin((222, 186, 80, 255), (240, 210, 120, 255), (120, 60, 30, 255)),
    "quipu": lambda: relic_quipu(7),
    "disco": relic_disc,
    "quipu-maestro": lambda: relic_quipu(12),
    "piastrine": relic_tags,
    "campanaccio": relic_bell,
    "bussola": relic_compass,
    "chiave-rame": relic_key,
    "bulbo": relic_bulb,
    "mappa-correnti": lambda: relic_map((60, 90, 170, 255)),
    "mappa-secche": lambda: relic_map((40, 120, 120, 255)),
    "focaccia": relic_focaccia,
    "amuleto": relic_feather,
    "registro": lambda: relic_book((70, 80, 110, 255), (200, 170, 90, 255)),
    "scatola-semi": relic_seedbox,
    "lettera": relic_letter,
    "taccuino": lambda: relic_book((110, 80, 50, 255), (60, 110, 60, 255)),
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
