#!/usr/bin/env python3
"""
gen_mockups.py — mockups 320x480 das telas do Clow Deck para o README.

Fiel ao firmware: mesma geometria (design/THEME.md §4), mesmos tokens de cor
(C_* de clow_deck.ino), os MESMOS icones (reaproveita raster_icon/raster_logo/
pixel do gen_icons.py) e as MESMAS fontes (assets/fonts/Montserrat-SemiBold).

Saida: assets/mock-*.png (2x para telas nitidas no GitHub).
Uso:   python3 tools/gen_mockups.py
"""
import os
import sys

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_icons as G  # noqa: E402
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(
    os.path.abspath(__file__))), "assets"))
from icons_vec import ICONS  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "assets")
FONT_SB = os.path.join(ROOT, "assets", "fonts", "Montserrat-SemiBold.ttf")

# ---- tokens (espelham os #define C_* do firmware) ----------------------
C_BG_TOP, C_BG_BOTTOM = 0x0C0D0C, 0x141513
C_CELL, C_CELL_LINE, C_CELL_DIM = 0x131412, 0x3A3B39, 0x242523
C_STRIP, C_ACCENT_HI = 0x10110F, 0xE2957F
C_TEXT, C_MUTED, C_FAINT = 0xFAF9F5, 0xB0AEA5, 0x6B6960
C_ACCENT = 0xD97757
C_WORK, C_ATTN, C_DONE, C_IDLE, C_ERR = 0xA3E635, 0xF0B35B, 0x788C5D, 0x4E5A40, 0xC8524A
C_ON_ATTN = 0x141413
C_GLASS_HI, C_EDGE = 0x2F2E2B, 0xFAF9F5

# ---- geometria (identica ao firmware) ---------------------------------
W, H = 320, 480
COLS, GAP, CELL_W, CELL_H = 3, 4, 101, 115
R_CELL, R_CHIP, CELL_PAD = 18, 10, 10
STRIP_X, STRIP_Y = GAP, GAP + 3 * (CELL_H + GAP)
STRIP_W, STRIP_H = W - 2 * GAP, H - STRIP_Y - GAP
SC = 2  # supersample do mockup


def rgb(v):
    return ((v >> 16) & 255, (v >> 8) & 255, v & 255)


def mix(a, b, t):
    ca, cb = rgb(a), rgb(b)
    return tuple(round(ca[i] + (cb[i] - ca[i]) * t) for i in range(3))


def over_cell(col, opa):
    """igual ao over_cell() do firmware: mistura a cor do estado no C_CELL."""
    return mix(C_CELL, col, opa / 255)


def F(px):
    return ImageFont.truetype(FONT_SB, px * SC)


def cell_x(i):
    return GAP + (i % COLS) * (CELL_W + GAP)


def cell_y(i):
    return GAP + (i // COLS) * (CELL_H + GAP)


# ---- primitivas --------------------------------------------------------
def bg(im):
    """gradiente vertical + ceu estrelado (o mesmo tile A8 do firmware)."""
    d = ImageDraw.Draw(im)
    for y in range(H * SC):
        d.line([(0, y), (W * SC, y)], fill=mix(C_BG_TOP, C_BG_BOTTOM, y / (H * SC)))
    tw, th, data = G.sparkle_tile(160)
    tile = Image.new("L", (tw, th))
    tile.putdata(data)
    tile = tile.resize((tw * SC, th * SC), Image.LANCZOS)
    stars = Image.new("L", (W * SC, H * SC), 0)
    for ty in range(0, H * SC, th * SC):
        for tx in range(0, W * SC, tw * SC):
            stars.paste(tile, (tx, ty))
    stars = stars.point(lambda v: int(v * 22 / 255))          # image_opa 22
    im.paste(Image.new("RGB", im.size, rgb(C_ACCENT)), (0, 0), stars)


def panel(im, x, y, w, h, r, fill, border, bw=2, glow=None):
    """glass(): superficie quase preta + contorno; glow = sombra colorida."""
    d = ImageDraw.Draw(im, "RGBA")
    X, Y, Wd, Ht, R = x * SC, y * SC, w * SC, h * SC, r * SC
    if glow is not None:
        for i in range(9, 0, -1):
            a = int(26 * (1 - i / 9) ** 1.6)
            d.rounded_rectangle([X - i * SC // 2, Y - i * SC // 2,
                                 X + Wd + i * SC // 2, Y + Ht + i * SC // 2],
                                radius=R + i * SC // 2, outline=(*rgb(glow), a), width=SC)
    d.rounded_rectangle([X, Y, X + Wd, Y + Ht], radius=R,
                        fill=(*(fill if isinstance(fill, tuple) else rgb(fill)), 250),
                        outline=rgb(border) if not isinstance(border, tuple) else border,
                        width=bw * SC)


def stamp(im, mask_wh_data, x, y, col, opa=255, anchor="tl"):
    """cola um icone A8 (w,h,data) tingido — igual ao image_recolor do LVGL."""
    w, h, data = mask_wh_data
    m = Image.new("L", (w, h))
    m.putdata(data)
    m = m.resize((w * SC, h * SC), Image.LANCZOS)
    if opa < 255:
        m = m.point(lambda v: int(v * opa / 255))
    X, Y = x * SC, y * SC
    if anchor == "c":
        X, Y = X - m.size[0] // 2, Y - m.size[1] // 2
    im.paste(Image.new("RGB", m.size, rgb(col) if not isinstance(col, tuple) else col), (X, Y), m)


def text(im, s, x, y, font, col, anchor="la", ls=0):
    d = ImageDraw.Draw(im)
    c = rgb(col) if not isinstance(col, tuple) else col
    if ls == 0:
        d.text((x * SC, y * SC), s, font=font, fill=c, anchor=anchor)
        return
    # letter_space: desenha caractere a caractere
    widths = [d.textlength(ch, font=font) for ch in s]
    total = sum(widths) + ls * SC * (len(s) - 1)
    cx = x * SC - (total / 2 if anchor[0] == "m" else 0)
    for ch, wch in zip(s, widths):
        d.text((cx, y * SC), ch, font=font, fill=c, anchor="l" + anchor[1])
        cx += wch + ls * SC


# ---- icones (cache) ----------------------------------------------------
_ic = {}


def ico(name, size=40):
    k = (name, size)
    if k not in _ic:
        _ic[k] = G.raster_icon(ICONS[name], size)
    return _ic[k]


def logo(kind, size=40):
    k = ("logo:" + kind, size)
    if k not in _ic:
        fname, mode = G.LOGOS[kind]
        _ic[k] = G.raster_logo(fname, mode, size)
    return _ic[k]


def mascot(frame="clow_a", scale=2):
    k = ("m:" + frame, scale)
    if k not in _ic:
        rows = G.parse_pixel(os.path.join(ROOT, "assets", "pixel", frame + ".txt"))[3]
        _ic[k] = G.pixel_data(rows, scale)
    return _ic[k]


# ---- componentes -------------------------------------------------------
def btn_cell(im, i, icon_name, label, col, err=False):
    """cell_btn(): icone 40 no topo (+8) + rotulo MAIUSCULO embaixo."""
    x, y = cell_x(i), cell_y(i)
    panel(im, x, y, CELL_W, CELL_H, R_CELL, C_CELL, C_CELL_LINE)
    stamp(im, ico(icon_name), x + CELL_W // 2, y + CELL_PAD + 8 + 20, col, anchor="c")
    text(im, label.upper(), x + CELL_W / 2, y + CELL_H - CELL_PAD - 9,
         F(8), C_ERR if err else C_TEXT, anchor="ma", ls=1)


def empty_cell(im, i):
    x, y = cell_x(i), cell_y(i)
    panel(im, x, y, CELL_W, CELL_H, R_CELL, C_CELL, C_CELL_DIM)


def session_cell(im, i, label, state, engine, age=None, active=False):
    """celula de sessao da home: logo da engine ao fundo + rotulo + chip + dot."""
    x, y = cell_x(i), cell_y(i)
    fill, bd, glow, mo, mc, tx = C_CELL, C_CELL_DIM, None, 46, C_IDLE, C_TEXT
    if state == "work":
        fill, bd, glow, mo, mc = over_cell(C_WORK, 30), C_WORK, C_WORK, 150, C_WORK
    elif state == "attn":
        fill, bd, glow, mo, mc = rgb(C_ATTN), C_ATTN, C_ATTN, 210, C_ON_ATTN
        tx = C_ON_ATTN
    elif state == "done":
        fill, bd, glow, mo, mc = over_cell(C_DONE, 45), C_DONE, C_DONE, 120, C_DONE
    elif state == "idle":
        bd, mc = C_CELL_LINE, C_IDLE
    bw = 2
    if active:
        bw, bd, glow, mc = 3, C_ACCENT, C_ACCENT, C_ACCENT
        mo = max(mo, 200)
    panel(im, x, y, CELL_W, CELL_H, R_CELL, fill, bd, bw, glow)
    stamp(im, logo(engine), x + CELL_W // 2, y + CELL_H // 2 - 8, mc, mo, anchor="c")
    text(im, label, x + CELL_W / 2, y + CELL_H - CELL_PAD - 11, F(10), tx, anchor="ma", ls=1)
    # chip da engine (canto sup. dir.)
    chip = {"claude": "CC", "codexlogo": "CDX", "oclogo": "OC"}[engine]
    d = ImageDraw.Draw(im, "RGBA")
    fnt = F(12)
    cw = d.textlength(chip, font=fnt) / SC + 14
    cx0, cy0 = x + CELL_W - cw - CELL_PAD + 2, y + CELL_PAD - 2
    d.rounded_rectangle([cx0 * SC, cy0 * SC, (cx0 + cw) * SC, (cy0 + 20) * SC],
                        radius=R_CHIP * SC, fill=(*rgb(C_GLASS_HI), 55),
                        outline=(*rgb(C_FAINT), 160), width=SC)
    text(im, chip, cx0 + cw / 2, cy0 + 10, fnt, C_MUTED, anchor="mm")
    # dot de estado (canto sup. esq.)
    # no flash "on" do ATTENTION o fundo e a propria cor: o ponto usa o contraste
    dot = {"work": C_WORK, "attn": C_ON_ATTN, "done": C_DONE, "idle": C_IDLE}.get(state, C_FAINT)
    d.ellipse([(x + CELL_PAD) * SC, (y + CELL_PAD + 3) * SC,
               (x + CELL_PAD + 8) * SC, (y + CELL_PAD + 11) * SC], fill=rgb(dot))
    if age:
        text(im, age, x + CELL_PAD + 12, y + CELL_PAD + 2, F(10), C_MUTED)


def strip_panel(im):
    panel(im, STRIP_X, STRIP_Y, STRIP_W, STRIP_H, R_CELL, C_STRIP, C_CELL_DIM)


# ---- telas -------------------------------------------------------------
def screen():
    im = Image.new("RGB", (W * SC, H * SC))
    bg(im)
    return im


def mock_home():
    im = screen()
    session_cell(im, 0, "clow-deck", "work", "claude", "2m", active=True)
    session_cell(im, 1, "api-server", "attn", "codexlogo", "12s")
    session_cell(im, 2, "site-novo", "done", "oclogo", "1m")
    session_cell(im, 3, "docs", "idle", "claude", "8m")
    session_cell(im, 4, "infra", "work", "claude", "40s")
    empty_cell(im, 5)
    btn_cell(im, 6, "lang", "idioma", C_ACCENT_HI)
    text(im, "EN", cell_x(6) + CELL_W / 2, cell_y(6) + CELL_H / 2 + 14, F(20), C_ACCENT, anchor="mm")
    btn_cell(im, 7, "bright", "brilho", C_ACCENT_HI)
    text(im, "80%", cell_x(7) + CELL_W / 2, cell_y(7) + CELL_H / 2 + 14, F(20), C_ACCENT, anchor="mm")
    btn_cell(im, 8, "gear", "ajustes", C_ACCENT_HI)
    strip_panel(im)
    stamp(im, ico("bt", 16), STRIP_X + STRIP_W - CELL_PAD - 16, STRIP_Y + CELL_PAD + 2, C_WORK)
    stamp(im, mascot("clow_a", 2), STRIP_X + STRIP_W // 2 - 80, STRIP_Y + STRIP_H // 2, C_ACCENT, anchor="c")
    text(im, "CLOW DECK", STRIP_X + STRIP_W / 2 + 38, STRIP_Y + STRIP_H / 2, F(18), C_TEXT, anchor="mm", ls=3)
    return im


def mock_session():
    im = screen()
    btn_cell(im, 0, "back", "voltar", C_MUTED)
    btn_cell(im, 1, "focus", "focar", C_ACCENT)
    btn_cell(im, 2, "voice", "voz", C_TEXT)
    btn_cell(im, 3, "mode", "modo", C_TEXT)
    btn_cell(im, 4, "esc", "esc", C_TEXT)
    btn_cell(im, 5, "enter", "enter", C_TEXT)
    btn_cell(im, 6, "tab", "tab", C_DONE)
    btn_cell(im, 7, "compact", "/compact", C_TEXT)
    btn_cell(im, 8, "next", "mais", C_MUTED)
    strip_panel(im)
    text(im, "clow-deck", STRIP_X + CELL_PAD, STRIP_Y + 14, F(14), C_TEXT)
    text(im, "trabalhando  •  vs code  •  CC", STRIP_X + CELL_PAD, STRIP_Y + 52, F(12), C_MUTED)
    text(im, "segure a voz para falar  •  toque em focar para ir a janela",
         STRIP_X + CELL_PAD, STRIP_Y + 76, F(10), C_FAINT)
    return im


def mock_cmd():
    im = screen()
    btn_cell(im, 0, "back", "voltar", C_MUTED)
    btn_cell(im, 1, "plus", "nova sessao", C_ERR, err=True)
    btn_cell(im, 2, "cmd", "/init", C_ACCENT)
    btn_cell(im, 3, "exit", "/exit", C_ERR, err=True)
    btn_cell(im, 4, "cmd", "/review", C_ACCENT)
    btn_cell(im, 5, "cmd", "/test", C_ACCENT)
    for i in (6, 7):
        empty_cell(im, i)
    btn_cell(im, 8, "next", "mais", C_MUTED)
    strip_panel(im)
    text(im, "alvo: clow-deck (sessao ativa)", STRIP_X + CELL_PAD, STRIP_Y + 14, F(14), C_TEXT)
    text(im, "5 comandos  •  pagina 1/1", STRIP_X + CELL_PAD, STRIP_Y + 52, F(12), C_MUTED)
    text(im, "customizados vem do agente (CONFIG)", STRIP_X + CELL_PAD, STRIP_Y + 76, F(10), C_FAINT)
    return im


def mock_search():
    """ui_search(): mascote grande em y=62, titulo em 168, subtitulo em 198,
    linha 2 com placeholders e a faixa com status do BLE."""
    im = screen()
    stamp(im, mascot("clow_a", 4), (W - 96) // 2, 62, C_ACCENT)
    text(im, "procurando host", W / 2, 168, F(20), C_TEXT, anchor="ma")
    text(im, "Clow Deck  •  deck do Claude Code", W / 2, 198, F(12), C_MUTED, anchor="ma")
    for i in (6, 7, 8):
        empty_cell(im, i)
    strip_panel(im)
    text(im, "anunciando por BLE  •  sem pareamento", STRIP_X + CELL_PAD, STRIP_Y + 14, F(14), C_TEXT)
    text(im, "98:A3:16:F1:C3:7D  •  fw 0.3", STRIP_X + CELL_PAD, STRIP_Y + 52, F(12), C_MUTED)
    text(im, "abra o agente no computador", STRIP_X + CELL_PAD, STRIP_Y + 74, F(12), C_MUTED)
    return im


def mark(scale=6):
    """assets/clow-mark.png — o mascote em coral, fundo transparente (cabecalho do README)."""
    w, h, data = mascot("clow_a", scale)
    m = Image.new("L", (w, h))
    m.putdata(data)
    im = Image.new("RGBA", (w, h), (*rgb(C_ACCENT), 0))
    im.paste(Image.new("RGBA", (w, h), (*rgb(C_ACCENT), 255)), (0, 0), m)
    return im


def main():
    os.makedirs(OUT, exist_ok=True)
    mk = mark()
    pm = os.path.join(OUT, "clow-mark.png")
    mk.save(pm)
    print(f"  {os.path.relpath(pm, ROOT)}  {mk.size[0]}x{mk.size[1]}")
    for name, fn in [("home", mock_home), ("session", mock_session),
                     ("cmd", mock_cmd), ("search", mock_search)]:
        im = fn()
        p = os.path.join(OUT, f"mock-{name}.png")
        im.save(p)
        print(f"  {os.path.relpath(p, ROOT)}  {im.size[0]}x{im.size[1]}")


if __name__ == "__main__":
    main()
