#!/usr/bin/env python3
"""Genera le icone di Dropall (PNG, ICO, ICNS) con Pillow.

Uso:  python3 scripts/make_icon.py
Output: assets/icon.png  assets/icon.ico  assets/icon.icns
"""
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

S = 2048          # risoluzione di lavoro (poi ridotta: antialiasing di qualità)
OUT = Path(__file__).resolve().parent.parent / "assets"
OUT.mkdir(exist_ok=True)


def gradient(size, c1, c2):
    """Gradiente diagonale c1 (alto-sinistra) -> c2 (basso-destra)."""
    base = Image.new("RGB", (size, size))
    px = base.load()
    for y in range(size):
        for x in range(size):
            t = (x + y) / (2 * (size - 1))
            px[x, y] = tuple(int(c1[i] + (c2[i] - c1[i]) * t) for i in range(3))
    return base


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


# --- sfondo: quadrato arrotondato con gradiente arancione (come l'interfaccia) ---
# (il gradiente si calcola a bassa risoluzione e si ingrandisce: è liscio)
small = gradient(256, (255, 168, 64), (222, 72, 8))
bg = small.resize((S, S), Image.BICUBIC).convert("RGBA")

mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle(
    (int(S * 0.04), int(S * 0.04), int(S * 0.96), int(S * 0.96)),
    radius=int(S * 0.225), fill=255)

icon = Image.new("RGBA", (S, S), (0, 0, 0, 0))
icon.paste(bg, (0, 0), mask)

# --- riflesso lucido in alto -------------------------------------------------
gloss = Image.new("RGBA", (S, S), (0, 0, 0, 0))
gd = ImageDraw.Draw(gloss)
gd.ellipse((-S * 0.25, -S * 0.78, S * 1.25, S * 0.50), fill=(255, 255, 255, 34))
gloss = gloss.filter(ImageFilter.GaussianBlur(S * 0.006))
gloss = ImageChops.multiply(gloss, Image.merge("RGBA", (mask, mask, mask, mask)))
icon = Image.alpha_composite(icon, gloss)

# --- simbolo: tre elementi (file, musica, video) che "cadono" in un vassoio ---
sym = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(sym)
W = (255, 255, 255, 255)
SOFT = (255, 255, 255, 120)

def streak(x, y0, y1, w=0.034):
    """scia verticale semitrasparente: dà il senso della caduta"""
    d.rounded_rectangle((x - S * w / 2, y0, x + S * w / 2, y1),
                        radius=int(S * w / 2), fill=SOFT)

def rounded_poly(points):
    m = Image.new("L", (S, S), 0)
    ImageDraw.Draw(m).polygon(points, fill=255)
    m = m.filter(ImageFilter.GaussianBlur(S * 0.02)).point(
        lambda v: 255 if v > 150 else 0).filter(ImageFilter.GaussianBlur(2))
    sym.paste(W, (0, 0), m)

xs = (S * 0.275, S * 0.50, S * 0.725)
# scie (altezze diverse: sembrano cadere in momenti diversi)
streak(xs[0], S * 0.15, S * 0.28)
streak(xs[1], S * 0.10, S * 0.22)
streak(xs[2], S * 0.18, S * 0.31)
# file: quadrato arrotondato con angolo piegato
d.rounded_rectangle((xs[0] - S * 0.09, S * 0.35, xs[0] + S * 0.09, S * 0.55),
                    radius=int(S * 0.03), fill=W)
# musica: cerchio con foro (disco)
d.ellipse((xs[1] - S * 0.10, S * 0.33, xs[1] + S * 0.10, S * 0.53), fill=W)
# video: triangolo "play" con angoli stondati
rounded_poly([(xs[2] - S * 0.07, S * 0.36), (xs[2] - S * 0.07, S * 0.56),
              (xs[2] + S * 0.105, S * 0.46)])
# fori/dettagli in colore di sfondo (ritaglio trasparente)
cut = Image.new("L", (S, S), 255)
cd = ImageDraw.Draw(cut)
cd.ellipse((xs[1] - S * 0.03, S * 0.40, xs[1] + S * 0.03, S * 0.46), fill=0)
cd.polygon([(xs[0] + S * 0.035, S * 0.35), (xs[0] + S * 0.09, S * 0.35),
            (xs[0] + S * 0.09, S * 0.405)], fill=0)
sym.putalpha(ImageChops.multiply(sym.split()[3], cut))
d = ImageDraw.Draw(sym)
# vassoio a "U" ampio
d.rounded_rectangle((S * 0.19, S * 0.67, S * 0.81, S * 0.755),
                    radius=int(S * 0.042), fill=W)
d.rounded_rectangle((S * 0.19, S * 0.58, S * 0.265, S * 0.755),
                    radius=int(S * 0.038), fill=W)
d.rounded_rectangle((S * 0.735, S * 0.58, S * 0.81, S * 0.755),
                    radius=int(S * 0.038), fill=W)

# ombra morbida sotto il simbolo
shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
shadow.paste((90, 28, 0, 130), (0, int(S * 0.012)), sym.split()[3])
shadow = shadow.filter(ImageFilter.GaussianBlur(S * 0.018))
icon = Image.alpha_composite(icon, shadow)
icon = Image.alpha_composite(icon, sym)

# --- export -----------------------------------------------------------------
png512 = icon.resize((512, 512), Image.LANCZOS)
png512.save(OUT / "icon.png")

sizes = [16, 24, 32, 48, 64, 128, 256]
ico_src = icon.resize((256, 256), Image.LANCZOS)
ico_src.save(OUT / "icon.ico", sizes=[(s, s) for s in sizes])

icon.resize((1024, 1024), Image.LANCZOS).save(OUT / "icon.icns")
print("Icone create in", OUT)
