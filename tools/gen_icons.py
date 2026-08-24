#!/usr/bin/env python3
"""Gera firmware/clow_deck/icons.h a partir de duas fontes:

1. assets/pixel/*.txt — pixel-art A8 binaria (mascote clow_a/clow_b).
   Template:
       name: clow_a
       scale: 2,4,8        # escalas a emitir (nearest-neighbor)
       eye: x y w h        # (opcional, mascote) furos dos olhos em pixels-base
       ..#..               # '#' pixel aceso, '.' vazio; linhas de mesma largura
2. assets/icons_vec.py — icones outline (grade de design 24x24, traco ~2.1,
   pontas/juncoes redondas), rasterizados com Pillow em supersample 12x +
   reducao LANCZOS => A8 com anti-aliasing. Tamanhos emitidos por tag:
   4 -> 40 px (celulas), 2 -> 16 px (faixa/status). O sufixo eh TAG DE
   TAMANHO (compatibilidade com os call sites `ic_<nome>_4`/`_2` do .ino),
   nao mais fator de escala.

Saida: `ic_<name>_<tag>` como `lv_image_dsc_t` LV_COLOR_FORMAT_A8 (1 byte/px,
stride = w), colorido em runtime com image_recolor (+recolor_opa 255) e
esmaecido com image_opa. Rodar da raiz do repo: python3 tools/gen_icons.py
"""
import glob, math, os, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC_PIXEL = os.path.join(ROOT, "assets", "pixel")
OUT = os.path.join(ROOT, "firmware", "clow_deck", "icons.h")

SIZES = {4: 40, 2: 16}   # tag -> px emitido (ordem: maior primeiro)
GRID = 24.0              # grade de design dos icones vetoriais
SS = 12                  # fator de supersample da rasterizacao

sys.path.insert(0, os.path.join(ROOT, "assets"))
try:
    from icons_vec import ICONS as VEC_ICONS
except ImportError:
    sys.exit("assets/icons_vec.py nao encontrado")


# ---------------------------------------------------------------- pixel-art

def parse_pixel(path):
    name, scales, eyes, rows = None, [], [], []
    for raw in open(path, encoding="utf-8"):
        line = raw.rstrip("\n")
        if not line.strip():
            continue
        if line.startswith("name:"):
            name = line.split(":", 1)[1].strip()
        elif line.startswith("scale:"):
            scales = [int(s) for s in line.split(":", 1)[1].replace(" ", "").split(",") if s]
        elif line.startswith("eye:"):
            eyes.append(tuple(int(v) for v in line.split(":", 1)[1].split()))
        else:
            rows.append(line.strip())
    if not name or not rows:
        sys.exit(f"{path}: template incompleto")
    w = len(rows[0])
    if any(len(r) != w for r in rows):
        sys.exit(f"{path}: linhas com larguras diferentes")
    return name, scales or [4], eyes, rows


def pixel_data(rows, scale):
    w, h = len(rows[0]) * scale, len(rows) * scale
    data = []
    for r in rows:
        row = []
        for ch in r:
            row.extend([255 if ch == "#" else 0] * scale)
        for _ in range(scale):
            data.extend(row)
    return w, h, data


# ---------------------------------------------------------------- vetorial

def raster_icon(spec, size):
    """Rasteriza um icone da spec vetorial em A8 `size`x`size` com AA."""
    try:
        from PIL import Image, ImageDraw
    except ImportError:
        sys.exit("Pillow necessario para os icones outline: pip install Pillow")
    S = size * SS
    k = S / GRID                      # unidades de design -> px supersample
    stroke = spec.get("stroke", 2.1)
    w = max(1, round(stroke * k))     # largura do traco em px supersample
    img = Image.new("L", (S, S), 0)
    d = ImageDraw.Draw(img)

    def P(x, y):
        return (x * k, y * k)

    def cap(x, y):                    # ponta redonda (px de design)
        cx, cy = P(x, y)
        d.ellipse([cx - w / 2, cy - w / 2, cx + w / 2, cy + w / 2], fill=255)

    def line(x1, y1, x2, y2):
        d.line([P(x1, y1), P(x2, y2)], fill=255, width=w)
        cap(x1, y1); cap(x2, y2)

    for prim in spec["prims"]:
        kind = prim[0]
        if kind == "line":
            line(*prim[1:])
        elif kind == "pline":
            pts = [P(x, y) for x, y in prim[1]]
            d.line(pts, fill=255, width=w, joint="curve")
            cap(*prim[1][0]); cap(*prim[1][-1])
        elif kind == "circle":
            cx, cy, r = prim[1:]
            out_r = r * k + w / 2
            d.ellipse([cx * k - out_r, cy * k - out_r, cx * k + out_r, cy * k + out_r],
                      outline=255, width=w)
        elif kind == "ellipse":
            cx, cy, rx, ry = prim[1:]
            ox, oy = rx * k + w / 2, ry * k + w / 2
            d.ellipse([cx * k - ox, cy * k - oy, cx * k + ox, cy * k + oy],
                      outline=255, width=w)
        elif kind == "arc":
            cx, cy, r, a0, a1 = prim[1:]
            out_r = r * k + w / 2
            d.arc([cx * k - out_r, cy * k - out_r, cx * k + out_r, cy * k + out_r],
                  a0, a1, fill=255, width=w)
            for a in (a0, a1):
                cap(cx + r * math.cos(math.radians(a)),
                    cy + r * math.sin(math.radians(a)))
        elif kind == "rrect":
            x, y, rw, rh, r = prim[1:]
            d.rounded_rectangle([x * k - w / 2, y * k - w / 2,
                                 (x + rw) * k + w / 2, (y + rh) * k + w / 2],
                                radius=r * k + w / 2, outline=255, width=w)
        elif kind == "dot":
            cx, cy, r = prim[1:]
            d.ellipse([P(cx - r, cy - r), P(cx + r, cy + r)], fill=255)
        elif kind == "arrowhead":
            tx, ty, ang, ln = prim[1:]
            for s in (35, -35):
                a = math.radians(ang + 180 + s)
                line(tx, ty, tx + ln * math.cos(a), ty + ln * math.sin(a))
        else:
            sys.exit(f"primitiva desconhecida: {kind}")

    from PIL import Image as _I, ImageFilter as _F
    img = img.resize((size, size), _I.LANCZOS)
    img = img.filter(_F.UnsharpMask(radius=1, percent=70, threshold=0))
    return size, size, list(img.getdata())


def sparkle_tile(size=160):
    """Tile de fundo A8 `size`x`size`, sem emenda (desenho com wrap): pontos e
    brilhos de 4 pontas esparsos, deterministicos (seed fixa). Ladrilhado na
    tela inteira e recolorido a baixa opacidade pelo firmware."""
    import random
    from PIL import Image, ImageDraw
    S = size * 4
    rng = random.Random(7)
    img = Image.new("L", (S, S), 0)
    d = ImageDraw.Draw(img)

    def wrapped(draw_at):
        for dx in (-S, 0, S):
            for dy in (-S, 0, S):
                draw_at(dx, dy)

    for _ in range(26):                       # pontos ("estrelas")
        x, y = rng.uniform(0, S), rng.uniform(0, S)
        r, a = rng.uniform(1.6, 4.5), rng.randint(70, 200)
        wrapped(lambda dx, dy, x=x, y=y, r=r, a=a:
                d.ellipse([x + dx - r, y + dy - r, x + dx + r, y + dy + r], fill=a))
    for _ in range(7):                        # brilhos de 4 pontas
        x, y = rng.uniform(0, S), rng.uniform(0, S)
        big, a = rng.uniform(10, 22), rng.randint(150, 255)
        ir = big * 0.32
        c = 0.7071 * ir
        wrapped(lambda dx, dy, x=x, y=y, big=big, c=c, a=a:
                d.polygon([(x + dx + big, y + dy), (x + dx + c, y + dy + c),
                           (x + dx, y + dy + big), (x + dx - c, y + dy + c),
                           (x + dx - big, y + dy), (x + dx - c, y + dy - c),
                           (x + dx, y + dy - big), (x + dx + c, y + dy - c)], fill=a))
    img = img.resize((size, size), Image.LANCZOS)
    return size, size, list(img.getdata())


# ---------------------------------------------------------------- logos raster

# Logos oficiais dos engines (assets/logos): viram A8 recoloravel como os demais.
# modo "alpha" = usa o canal alpha; "bgdist" = distancia da cor do canto (fundo).
LOGOS = {
    "claude": ("claudecode.webp", "bgdist"),
    "codexlogo": ("codex.png", "alpha"),
}

def raster_logo(fname, mode, size):
    from PIL import Image
    img = Image.open(os.path.join(ROOT, "assets", "logos", fname)).convert("RGBA")
    w, h = img.size
    px = img.load()
    mask = Image.new("L", (w, h), 0)
    mp = mask.load()
    if mode == "alpha":
        for y in range(h):
            for x in range(w):
                mp[x, y] = px[x, y][3]
    else:
        br, bg_, bb, _ = px[2, 2]
        for y in range(h):
            for x in range(w):
                r, g, b, a = px[x, y]
                d = abs(r - br) + abs(g - bg_) + abs(b - bb)
                mp[x, y] = 255 if (a > 128 and d > 90) else 0
    bbox = mask.getbbox()
    if bbox:
        mask = mask.crop(bbox)
    # quadrado com margem ~8%
    side = max(mask.size)
    pad = int(side * 0.08)
    sq = Image.new("L", (side + 2 * pad, side + 2 * pad), 0)
    sq.paste(mask, ((sq.size[0] - mask.size[0]) // 2, (sq.size[1] - mask.size[1]) // 2))
    out = sq.resize((size, size), Image.LANCZOS)
    return size, size, list(out.getdata())


# ---------------------------------------------------------------- emissao

def emit(out, ident, w, h, data):
    out.append(f"static const uint8_t {ident}_map[] = {{")
    for i in range(0, len(data), 32):
        out.append("  " + ",".join(str(v) for v in data[i:i + 32]) + ",")
    out.append("};")
    # lv_image_header_t (little-endian bitfields): magic, cf, flags, w, h, stride, reserved_2
    out.append(f"static const lv_image_dsc_t {ident} = {{")
    out.append(f"  {{ LV_IMAGE_HEADER_MAGIC, LV_COLOR_FORMAT_A8, 0, {w}, {h}, {w}, 0 }},")
    out.append(f"  sizeof({ident}_map), {ident}_map")
    out.append("};")
    out.append(f"#define {ident.upper()}_W {w}")
    out.append(f"#define {ident.upper()}_H {h}")


def main():
    out = ["// GERADO por tools/gen_icons.py — nao editar na mao.",
           "// Mascote: pixel-art de assets/pixel/*.txt (alpha binario, nearest-neighbor).",
           "// Icones: outline de assets/icons_vec.py (A8 com anti-aliasing; tag 4 = 40 px,",
           "// tag 2 = 16 px). Cor via lv_obj_set_style_image_recolor(+recolor_opa 255),",
           "// esmaecimento via lv_obj_set_style_image_opa.",
           "#ifndef ICONS_H", "#define ICONS_H", "#include <lvgl.h>", ""]

    files = sorted(glob.glob(os.path.join(SRC_PIXEL, "*.txt")))
    for f in files:
        name, scales, eyes, rows = parse_pixel(f)
        if name in VEC_ICONS:
            sys.exit(f"{f}: nome '{name}' duplicado em icons_vec.py")
        out.append(f"// ---- {name} ({len(rows[0])}x{len(rows)} base, pixel-art) ----")
        for s in scales:
            emit(out, f"ic_{name}_{s}", *pixel_data(rows, s))
        for i, (x, y, w, h) in enumerate(eyes):
            up = name.upper()
            out.append(f"#define {up}_EYE{i}_X {x}")
            out.append(f"#define {up}_EYE{i}_Y {y}")
            out.append(f"#define {up}_EYE{i}_W {w}")
            out.append(f"#define {up}_EYE{i}_H {h}")
        out.append("")

    for name in sorted(VEC_ICONS):
        out.append(f"// ---- {name} (outline 24x24) ----")
        for tag, size in SIZES.items():
            emit(out, f"ic_{name}_{tag}", *raster_icon(VEC_ICONS[name], size))
        out.append("")

    for name in sorted(LOGOS):
        fname, mode = LOGOS[name]
        out.append(f"// ---- {name} (logo oficial, raster {fname}) ----")
        for tag, size in SIZES.items():
            emit(out, f"ic_{name}_{tag}", *raster_logo(fname, mode, size))
        out.append("")

    out.append("// ---- bgtile (fundo estrelado, tile 160x160 sem emenda) ----")
    emit(out, "ic_bgtile_1", *sparkle_tile())
    out.append("")
    out.append("#endif // ICONS_H")
    with open(OUT, "w", encoding="utf-8") as fh:
        fh.write("\n".join(out) + "\n")
    print(f"{OUT}: {len(files)} pixel + {len(VEC_ICONS)} outline")


if __name__ == "__main__":
    main()
