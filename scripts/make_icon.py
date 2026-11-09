#!/usr/bin/env python3
"""Genera le icone di Dropall a partire da assets/icon.svg.

Uso:  python3 scripts/make_icon.py
Richiede Pillow e Playwright con Chromium (serve solo per rendere l'SVG).
Output: assets/icon.png  assets/icon.ico  assets/icon.icns
"""
import io
from pathlib import Path

from PIL import Image
from playwright.sync_api import sync_playwright

OUT = Path(__file__).resolve().parent.parent / "assets"
svg = (OUT / "icon.svg").read_text(encoding="utf-8")

html = f"""<!doctype html><html><body style="margin:0;background:transparent">
<div style="width:1024px;height:1024px">{svg.replace('width="100%" height="100%"', 'width="1024" height="1024"')}</div>
</body></html>"""

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page(viewport={"width": 1024, "height": 1024})
    page.set_content(html)
    png = page.screenshot(omit_background=True, clip={"x": 0, "y": 0, "width": 1024, "height": 1024})
    browser.close()

icon = Image.open(io.BytesIO(png)).convert("RGBA")

icon.resize((512, 512), Image.LANCZOS).save(OUT / "icon.png")
sizes = [16, 24, 32, 48, 64, 128, 256]
icon.resize((256, 256), Image.LANCZOS).save(OUT / "icon.ico", sizes=[(s, s) for s in sizes])
icon.save(OUT / "icon.icns")
print("Icone create in", OUT)
