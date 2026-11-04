from PIL import Image
import math

size = 256
img = Image.new("L", (size, size))
px = img.load()

for y in range(size):
    ny = y / (size - 1) * 2.0 - 1.0
    for x in range(size):
        nx = x / (size - 1) * 2.0 - 1.0

        # Componenti sinusoidali per colline morbide.
        h = 0.0
        h += 0.45 * math.sin(nx * 3.0) * math.cos(ny * 4.0)
        h += 0.30 * math.sin(nx * 5.0 + 1.7) * math.cos(ny * 3.0 + 2.3)
        h += 0.18 * math.sin(nx * 7.0 + 3.1) * math.cos(ny * 6.0 + 0.9)
        h += 0.08 * math.sin(nx * 11.0 + 0.4) * math.cos(ny * 9.0 + 5.2)

        # Piatto al centro (zona di spawn) e colline verso i bordi.
        d = math.hypot(nx, ny)
        center_mask = 1.0 - max(0.0, 1.0 - d / 0.35)  # 0 al centro, 1 fuori
        center_mask = center_mask * center_mask

        # Ammorte il terreno vicino al centro.
        h = h * (0.15 + 0.85 * center_mask)

        # Normalizza in 0..1.
        v = int(max(0.0, min(1.0, h * 0.5 + 0.5)) * 255)
        px[x, y] = v

img.save("assets/textures/terrain_heightmap.png")
print("OK", img.size)
