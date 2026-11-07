#!/usr/bin/env python3
"""Prepara assets/textures/capanna.png dalla foto della casetta (4 viste 512x512).

Ingrandisce 2x, corregge l'insegna ("CUSTO" -> "GUSTO") e rende trasparente
l'apertura del banco sulla facciata (vista in alto a sinistra), così dal chiosco
si vede l'interno.

Uso: python3 scripts/make_capanna_texture.py Capanna-Viandante.jpg
"""
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFont

SRC = sys.argv[1] if len(sys.argv) > 1 else "Capanna-Viandante.jpg"
OUT = "assets/textures/capanna.png"
S = 2  # fattore di ingrandimento

im = Image.open(SRC).convert("RGBA").resize((512 * S, 512 * S), Image.LANCZOS)
d = ImageDraw.Draw(im)

# --- Insegna: seconda riga "(CUSTO DEL BOSCO)" -> "(GUSTO DEL BOSCO)" ---
x0, y0, x1, y1 = (63 * S, 127 * S, 129 * S, 138 * S)
bg = im.getpixel((60 * S, 131 * S))[:3]
d.rectangle([x0, y0, x1, y1], fill=bg)
font_path = subprocess.run(
    ["fc-match", "-f", "%{file}", "sans:bold"], capture_output=True, text=True
).stdout
text = "(GUSTO DEL BOSCO)"
size = 10
while True:
    font = ImageFont.truetype(font_path, size)
    w = d.textlength(text, font=font)
    if w >= (x1 - x0 - 2 * S) or size > 60:
        break
    size += 1
bbox = d.textbbox((0, 0), text, font=font)
tx = x0 + ((x1 - x0) - (bbox[2] - bbox[0])) / 2 - bbox[0]
ty = y0 + ((y1 - y0) - (bbox[3] - bbox[1])) / 2 - bbox[1]
d.text((tx, ty), text, font=font, fill=(208, 200, 182, 255))

# --- Apertura del banco: alfa 0 (vista in alto a sinistra, coordinate della foto) ---
ox0, oy0, ox1, oy1 = (47 * S, 140 * S, 145 * S, 178 * S)
px = im.load()
for y in range(oy0, oy1):
    for x in range(ox0, ox1):
        px[x, y] = (0, 0, 0, 0)

# --- Porta: ritaglio separato (pannello apribile) e apertura nella parete (alfa 0) ---
dx0, dy0, dx1, dy1 = 162, 131, 206, 218
src = im.copy()
door = src.crop((dx0 * S, dy0 * S, dx1 * S, dy1 * S)).convert("RGB")
door = door.resize((door.width * 2, door.height * 2), Image.LANCZOS)
door.save("assets/textures/porta.png", optimize=True)
for y in range((dy0 + 1) * S, (dy1 - 1) * S):
    for x in range((dx0 + 1) * S, (dx1 - 1) * S):
        px[x, y] = (0, 0, 0, 0)

im.save(OUT, optimize=True)
print("scritto", OUT, im.size)
