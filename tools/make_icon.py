"""Draws the application icon: a mind map growing to the right (root node and
three branches) on a rounded blue tile. Writes icon.png (256), icon.ico
(16-256, embedded into the exe by build.rs) and icon-64.rgba (raw RGBA for
the window icon) into the given dir.

Usage: python3 tools/make_icon.py assets   (needs Pillow)"""
import sys
from PIL import Image, ImageDraw

S = 1024  # drawn large, scaled down for smoothness
out = sys.argv[1]

img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
d.rounded_rectangle([40, 40, S - 40, S - 40], radius=200, fill=(43, 108, 196, 255))

white = (255, 255, 255, 255)
root = (300, 512)
leaves = [(760, 270), (760, 512), (760, 754)]

def bezier(p0, p3, steps=60):
    mx = (p0[0] + p3[0]) / 2
    p1, p2 = (mx, p0[1]), (mx, p3[1])
    pts = []
    for i in range(steps + 1):
        t = i / steps
        x = (1-t)**3*p0[0] + 3*(1-t)**2*t*p1[0] + 3*(1-t)*t**2*p2[0] + t**3*p3[0]
        y = (1-t)**3*p0[1] + 3*(1-t)**2*t*p1[1] + 3*(1-t)*t**2*p2[1] + t**3*p3[1]
        pts.append((x, y))
    return pts

for leaf in leaves:
    # A thick stroke as a dense row of discs: no joint artifacts.
    for x, y in bezier((root[0] + 60, root[1]), (leaf[0] - 70, leaf[1]), steps=400):
        d.ellipse([x - 22, y - 22, x + 22, y + 22], fill=white)

# Root: a larger rounded box; leaves: smaller pills.
d.rounded_rectangle([root[0] - 170, root[1] - 95, root[0] + 90, root[1] + 95], radius=60, fill=white)
for x, y in leaves:
    d.rounded_rectangle([x - 90, y - 62, x + 170, y + 62], radius=62, fill=(255, 214, 102, 255))

big = img.resize((256, 256), Image.LANCZOS)
big.save(f"{out}/icon.png")
big.save(f"{out}/icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
with open(f"{out}/icon-64.rgba", "wb") as f:
    f.write(img.resize((64, 64), Image.LANCZOS).tobytes())
