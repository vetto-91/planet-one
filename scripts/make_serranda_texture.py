#!/usr/bin/env python3
"""Genera assets/textures/serranda.png: pannello di assi con la scritta CHIUSO."""
import subprocess

from PIL import Image, ImageDraw, ImageFont

W, H = 1024, 400
im = Image.new("RGB", (W, H), (96, 66, 42))
d = ImageDraw.Draw(im)
# assi orizzontali con venature e fughe scure
import random

random.seed(11)
y = 0
while y < H:
    h = random.randint(42, 58)
    base = random.randint(-14, 14)
    col = (96 + base, 66 + base, 42 + base // 2)
    d.rectangle([0, y, W, y + h - 3], fill=col)
    for _ in range(26):  # venature
        yy = y + random.randint(3, max(4, h - 6))
        x0 = random.randint(0, W - 200)
        d.line([(x0, yy), (x0 + random.randint(80, 260), yy + random.randint(-1, 1))],
               fill=(col[0] - 14, col[1] - 12, col[2] - 8), width=1)
    d.rectangle([0, y + h - 3, W, y + h], fill=(44, 30, 20))
    y += h
# cornice
d.rectangle([0, 0, W - 1, H - 1], outline=(40, 27, 18), width=14)
# scritta CHIUSO (bianco sporco con contorno scuro)
font_path = subprocess.run(["fc-match", "-f", "%{file}", "sans:bold"], capture_output=True, text=True).stdout
size = 40
while True:
    font = ImageFont.truetype(font_path, size)
    box = d.textbbox((0, 0), "CHIUSO", font=font)
    if box[2] - box[0] >= W * 0.74 or size > 400:
        break
    size += 4
x = (W - (box[2] - box[0])) / 2 - box[0]
yy = (H - (box[3] - box[1])) / 2 - box[1]
for dx in (-4, 0, 4):
    for dy in (-4, 0, 4):
        d.text((x + dx, yy + dy), "CHIUSO", font=font, fill=(30, 20, 14))
d.text((x, yy), "CHIUSO", font=font, fill=(232, 222, 196))
im.save("assets/textures/serranda.png", optimize=True)
print("scritto assets/textures/serranda.png")
