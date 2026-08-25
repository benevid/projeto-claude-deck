/**
 * Clow Deck — deck fisico do Claude Code (tela touch LVGL + BLE)
 * Placa: Guition JC4832W535 (ESP32-S3, AXS15231B QSPI), 320x480 RETRATO.
 *
 * Periferico "burro": o agente no computador (agent/) descobre as sessoes do
 * Claude Code, recebe os hooks e escreve o estado de cada celula por BLE
 * (protocol/PROTOCOL.md). Este firmware so desenha e devolve os toques.
 * Zero segredo aqui: sem Wi-Fi, sem token, sem TLS.
 *
 * Visual: tema "Deep Space Glass" (design/THEME.md) + mascote/icones pixel-art
 * A8 (design/ICONS.md, gerados em icons.h). Regra da grade (case 3D): linhas
 * 0-1 = 6 celulas de sessao, linha 2 = 3 celulas utilitarias/botoes, linha 3 =
 * faixa livre — em TODAS as telas.
 *
 * Estrutura herdada do Usage Stick: pipeline de display/touch validado no
 * bring-up, request_state()/render_state(), overlays em lv_layer_top() e
 * animacoes procedurais no loop().
 */
#include <Arduino.h>
#include <Arduino_GFX_Library.h>
#include <lvgl.h>
#include <Wire.h>
#include <Preferences.h>
#include <math.h>
#include "config.h"
#include "deck_types.h"
#include "touch.h"
#include "icons.h"        // GERADO por tools/gen_icons.py (assets/pixel/*.txt)
#include "fonts_theme.h"  // Montserrat SemiBold/Bold (lv_font_conv)
#include "ble_link.h"

// ============================================================
// Tema Deep Space Glass (design/THEME.md) — tokens
// ============================================================
// Paleta: marca Claude (skill brand-guidelines-anthropic) sobre vidro escuro:
// dark #141413, light #faf9f5, mid #b0aea5 · laranja #d97757 (acento), azul #6a9bcc
// (trabalhando), verde #788c5d (pronto); atencao em ambar p/ nao confundir com o acento.
#define C_BG_TOP      0x0C0D0C
#define C_BG_BOTTOM   0x141513
#define C_GLASS       0x262624
#define C_GLASS_HI    0x2F2E2B
#define C_GLASS_PRESS 0x3B3A36
#define C_GLASS_GLOW  0x48342B   // rodape das teclas: vidro + ~25% coral profundo
#define C_CELL        0x131412   // celula quase preta (variante B — contorno luminoso)
#define C_CELL_LINE   0x3A3B39   // borda neutra de celula ocupada
#define C_CELL_DIM    0x242523   // borda de celula vazia
#define C_STRIP       0x10110F   // fundo da faixa
#define C_ACCENT_HI   0xE2957F   // coral claro (icones utilitarios, dark mode)
#define C_EDGE        0xFAF9F5
#define C_TEXT        0xFAF9F5
#define C_MUTED       0xB0AEA5
#define C_FAINT       0x6B6960
#define C_ACCENT      0xD97757
#define C_ACCENT_DEEP 0xB35F42
#define C_WORK        0xA3E635   // verde limao (padrao do usuario; era azul)
#define C_ATTN        0xF0B35B
#define C_DONE        0x788C5D
#define C_IDLE        0x4E5A40
#define C_ERR         0xC8524A
#define C_ON_ATTN     0x141413
#define GLASS_OPA     250
#define EDGE_OPA      30
#define SHINE_OPA     40
#define R_CELL        18
#define R_BTN         16
#define R_CHIP        10
#define R_BOX         20
#define CELL_PAD      10
#define BULLET        "  \xE2\x80\xA2  "

// ---- Idioma (0 = portugues, 1 = english; NVS "lang") ----
static uint8_t g_lang = 0;
#define TRS(pt, en) (g_lang ? (en) : (pt))

// ---- Hardware ----
Arduino_Canvas *gfx = nullptr;
static uint16_t *canvas_fb = nullptr;
AXS15231B_Touch touch_dev(TOUCH_SCL, TOUCH_SDA, TOUCH_INT, TOUCH_ADDR, TOUCH_ROTATION);
Preferences g_prefs;

// ---- Estado de tela ----
static State g_state = ST_BOOT;
static State g_pending = ST_BOOT;
static bool  g_dirty = false;

// ---- Modelo recebido do agente ----
static DeckModel  g_model = {};
static UsageModel g_usage = {};
static CustomCmd  g_custom[DECK_CUSTOM_MAX];
static int        g_customN = 0;
static int        g_selCell = -1;          // celula aberta na pagina SESSION
static uint8_t    g_bri = BRI_DEFAULT;
static bool       g_voice = false;         // push-to-talk em curso
static uint32_t   g_voiceHintUntil = 0;
static bool       g_agentStale = false;    // 10 s sem SESSIONS
static bool       g_bondedToast = false;
static int        g_cmdPage = 0;

// ============================================================
// Geometria da grade (design/THEME.md §4) — igual em todas as telas
// ============================================================
#define COLS     3
#define GAP      4
#define CELL_W   101
#define CELL_H   115
#define GRID_CELLS 9                                  // celulas 0..8 (linhas 0..2)
#define STRIP_X  GAP
#define STRIP_Y  (GAP + 3 * (CELL_H + GAP))           // 361
#define STRIP_W  (LV_HOR - 2 * GAP)                   // 312
#define STRIP_H  (LV_VER - STRIP_Y - GAP)             // 115
#define UTIL_LANG 6
#define UTIL_BRI  7
#define UTIL_GEAR 8
static int cell_x(int i) { return GAP + (i % COLS) * (CELL_W + GAP); }
static int cell_y(int i) { return GAP + (i / COLS) * (CELL_H + GAP); }

// ---- Ponteiros de UI (zerados em render_state) ----
struct CellUI {
  lv_obj_t *box, *masc, *lbl, *dot, *chip, *chipLbl, *age;
  uint32_t lastBg, lastGr, lastBd, lastTx, lastMc; int lastBw, lastMo;
};
struct GridUI {
  CellUI c[DECK_SESSION_CELLS];
  lv_obj_t *langVal, *briVal, *info1, *info2, *bleIco, *warnIco;
};
static GridUI g_grid;
struct SearchUI { lv_obj_t *cont, *img, *lid[2], *title, *sub, *status, *foot; int baseY; bool frameB; };
static SearchUI g_search;
struct SessUI { lv_obj_t *title, *stateLbl, *info, *voiceLbl, *voiceIco, *approveBtn, *denyBtn; };
static SessUI g_sess;
struct StripUI { lv_obj_t *l1, *l2; };
static StripUI g_strip;
static lv_obj_t *g_pkScrim = nullptr;      // overlay do passkey

// Confirmacao (overlay). Os botoes so setam o pedido; o loop() executa e
// apaga o overlay — nunca deletar objetos de dentro do callback deles.
#define PL_ACT(p)   ((p) & 0xFF)
#define PL_CELL(p)  (((p) >> 8) & 0xFF)
#define PL(act, c)  ((uint32_t)(act) | ((uint32_t)(c) << 8))
#define PL_FORGET   0x10000                 // op local: esquecer pareamento
#define PL_CONFIRM  0x20000                 // pede confirmacao antes
#define PL_TOGRID   0x40000                 // volta para a grade depois
struct ConfirmUI { lv_obj_t *scrim; uint32_t payload; int req; };   // req: 0 nada, 1 sim, 2 nao
static ConfirmUI g_cf;

// ---- Forward declarations ----
static void render_state();
static void request_state(State s) { g_pending = s; g_dirty = true; }
static void grid_apply();
static void grid_anim(uint32_t now);
static void grid_tick_1s();
static void show_confirm(const char *title, const char *sub, uint32_t payload);
static void do_payload(uint32_t p);

// ============================================================
// Pipeline de display/touch (validado no bring-up; orientacao em config.h)
// ============================================================
// Canvas nativo 320x480 (retrato). DECK_ORIENTATION escolhe a copia:
//   0: direta · 2: 180 graus · 3: 270 graus CW (paisagem, Usage Stick)
static inline void put_px(int lx, int ly, uint16_t v) {
#if DECK_ORIENTATION == 0
  canvas_fb[ly * 320 + lx] = v;
#elif DECK_ORIENTATION == 2
  canvas_fb[(479 - ly) * 320 + (319 - lx)] = v;
#else
  canvas_fb[(479 - lx) * 320 + ly] = v;
#endif
}
#if DECK_RENDER_PARTIAL
// PARTIAL: o LVGL entrega so a area invalidada (faixa de ate DECK_STRIP_LINES linhas,
// renderizada em RAM interna); copiamos a area para o canvas (PSRAM) e empurramos o
// canvas inteiro por QSPI uma unica vez, no ultimo pedaco do frame.
static void disp_flush_cb(lv_display_t *disp, const lv_area_t *area, uint8_t *px_map) {
  const uint16_t *src = (const uint16_t *)px_map;
  const int w = lv_area_get_width(area);
  for (int ly = area->y1; ly <= area->y2; ly++) {
    const uint16_t *row = src + (ly - area->y1) * w;
    for (int lx = area->x1; lx <= area->x2; lx++) put_px(lx, ly, row[lx - area->x1]);
  }
  if (lv_display_flush_is_last(disp)) gfx->flush();
  lv_disp_flush_ready(disp);
}
#else
static void disp_flush_cb(lv_display_t *disp, const lv_area_t *area, uint8_t *px_map) {
  uint16_t *src = (uint16_t *)px_map;
  for (int ly = 0; ly < LV_VER; ly++) {
    uint16_t *src_row = src + ly * LV_HOR;
    for (int lx = 0; lx < LV_HOR; lx++) put_px(lx, ly, src_row[lx]);
  }
  gfx->flush();
  lv_disp_flush_ready(disp);
}
#endif
static void touch_read_cb(lv_indev_t *indev, lv_indev_data_t *data) {
  uint16_t x, y;
  if (touch_dev.touched()) {
    touch_dev.readData(&x, &y);
    data->point.x = x; data->point.y = y;
    data->state = LV_INDEV_STATE_PRESSED;
  } else {
    data->state = LV_INDEV_STATE_RELEASED;
  }
}

// ============================================================
// Helpers de UI (vidro, icones, texto)
// ============================================================
static lv_obj_t *mklabel(lv_obj_t *p, const char *txt, const lv_font_t *font, uint32_t color) {
  lv_obj_t *l = lv_label_create(p);
  lv_label_set_text(l, txt);
  lv_obj_set_style_text_font(l, font, 0);
  lv_obj_set_style_text_color(l, lv_color_hex(color), 0);
  return l;
}
static void no_box(lv_obj_t *o) {
  lv_obj_set_style_bg_opa(o, 0, 0);
  lv_obj_set_style_border_width(o, 0, 0);
  lv_obj_set_style_pad_all(o, 0, 0);
  lv_obj_clear_flag(o, LV_OBJ_FLAG_SCROLLABLE);
}
static lv_obj_t *rrect(lv_obj_t *p, int x, int y, int w, int h, int r, uint32_t col) {
  lv_obj_t *o = lv_obj_create(p);
  lv_obj_set_pos(o, x, y); lv_obj_set_size(o, w, h);
  lv_obj_set_style_radius(o, r, 0);
  lv_obj_set_style_bg_color(o, lv_color_hex(col), 0);
  lv_obj_set_style_border_width(o, 0, 0);
  lv_obj_set_style_pad_all(o, 0, 0);
  lv_obj_clear_flag(o, LV_OBJ_FLAG_SCROLLABLE);
  lv_obj_clear_flag(o, LV_OBJ_FLAG_CLICKABLE);
  return o;
}
// Superficie de vidro escuro: fundo translucido sobre o gradiente, borda 1 px clara
// de baixa opacidade e um fio de luz no topo. `clickable` adiciona o estado pressionado.
static lv_obj_t *glass(lv_obj_t *p, int x, int y, int w, int h, int r, bool clickable) {
  lv_obj_t *o = lv_obj_create(p);
  lv_obj_set_pos(o, x, y); lv_obj_set_size(o, w, h);
  // variante B (contorno luminoso): superficie quase preta, contorno 2 px; o "glow"
  // dos estados e uma sombra colorida aplicada pelo grid_anim
  lv_obj_set_style_bg_color(o, lv_color_hex(C_CELL), 0);
  lv_obj_set_style_bg_opa(o, GLASS_OPA, 0);
  lv_obj_set_style_border_width(o, 2, 0);
  lv_obj_set_style_border_color(o, lv_color_hex(C_CELL_DIM), 0);
  lv_obj_set_style_border_opa(o, 255, 0);
  lv_obj_set_style_radius(o, r, 0);
  lv_obj_set_style_pad_all(o, CELL_PAD, 0);
  lv_obj_clear_flag(o, LV_OBJ_FLAG_SCROLLABLE);
  if (clickable) {
    lv_obj_add_flag(o, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_set_style_bg_color(o, lv_color_hex(0x1E1F1D), LV_STATE_PRESSED);  // eleva por claridade
    lv_obj_set_style_bg_opa(o, 255, LV_STATE_PRESSED);
    lv_obj_set_style_translate_y(o, 1, LV_STATE_PRESSED);
  } else {
    lv_obj_clear_flag(o, LV_OBJ_FLAG_CLICKABLE);
  }
  return o;
}
// icone pixel-art A8 colorido
static lv_obj_t *icon(lv_obj_t *p, const lv_image_dsc_t *dsc, uint32_t col, uint8_t opa) {
  lv_obj_t *img = lv_image_create(p);
  lv_image_set_src(img, dsc);
  lv_obj_set_style_image_recolor(img, lv_color_hex(col), 0);
  lv_obj_set_style_image_recolor_opa(img, 255, 0);
  lv_obj_set_style_image_opa(img, opa, 0);
  lv_obj_clear_flag(img, LV_OBJ_FLAG_CLICKABLE);
  return img;
}
static void icon_color(lv_obj_t *img, uint32_t col) {
  if (img) lv_obj_set_style_image_recolor(img, lv_color_hex(col), 0);
}
// mistura `col` sobre a celula quase preta (k 0..255)
static uint32_t over_cell(uint32_t col, uint8_t k) {
  uint32_t r = ((C_CELL >> 16) & 0xFF) + ((((col >> 16) & 0xFF) - ((C_CELL >> 16) & 0xFF)) * k) / 255;
  uint32_t g = ((C_CELL >> 8) & 0xFF) + ((((col >> 8) & 0xFF) - ((C_CELL >> 8) & 0xFF)) * k) / 255;
  uint32_t b = (C_CELL & 0xFF) + (((col & 0xFF) - (C_CELL & 0xFF)) * k) / 255;
  return (r << 16) | (g << 8) | b;
}
// rotulo maiusculo (ASCII) — hierarquia tipografica do tema Stitch
static const char *upcase_lbl(const char *s) {
  static char b[20];
  int i = 0;
  for (; s[i] && i < 19; i++) b[i] = (s[i] >= 'a' && s[i] <= 'z') ? (char)(s[i] - 32) : s[i];
  b[i] = 0;
  return b;
}
// celula vazia/placeholder (mantem a grade visivel em todas as telas)
static lv_obj_t *cell_placeholder(lv_obj_t *scr, int i) {
  lv_obj_t *b = glass(scr, cell_x(i), cell_y(i), CELL_W, CELL_H, R_CELL, false);
  lv_obj_set_style_opa(b, 90, 0);
  return b;
}
// celula-botao: icone @4 em cima + rotulo curto embaixo
static void nav_cb(lv_event_t *e) { request_state((State)(intptr_t)lv_event_get_user_data(e)); }
static lv_obj_t *cell_btn(lv_obj_t *scr, int i, const lv_image_dsc_t *ic, const char *txt,
                          uint32_t col, lv_event_cb_t cb, void *ud) {
  lv_obj_t *b = glass(scr, cell_x(i), cell_y(i), CELL_W, CELL_H, R_CELL, true);
  lv_obj_set_style_border_color(b, lv_color_hex(C_CELL_LINE), 0);
  lv_obj_t *im = icon(b, ic, col, 255);
  lv_obj_align(im, LV_ALIGN_TOP_MID, 0, 8);
  lv_obj_t *l = mklabel(b, upcase_lbl(txt), &font_ms_sb_8, col == C_ERR ? C_ERR : C_TEXT);
  lv_obj_set_style_text_letter_space(l, 1, 0);
  lv_obj_set_width(l, CELL_W - 2 * CELL_PAD - 2);
  lv_obj_set_style_text_align(l, LV_TEXT_ALIGN_CENTER, 0);
  lv_label_set_long_mode(l, LV_LABEL_LONG_WRAP);
  lv_obj_align(l, LV_ALIGN_BOTTOM_MID, 0, 0);
  if (cb) lv_obj_add_event_cb(b, cb, LV_EVENT_CLICKED, ud);
  return b;
}
static lv_obj_t *back_cell(lv_obj_t *scr, State to) {
  return cell_btn(scr, 0, &ic_back_4, TRS("voltar", "back"), C_MUTED, nav_cb, (void *)(intptr_t)to);
}
// faixa livre (linha 3): vidro + duas sub-linhas de texto
static lv_obj_t *strip(lv_obj_t *scr) {
  lv_obj_t *s = glass(scr, STRIP_X, STRIP_Y, STRIP_W, STRIP_H, R_CELL, false);
  g_strip.l1 = mklabel(s, "", &font_ms_sb_14, C_TEXT);
  lv_obj_set_pos(g_strip.l1, 0, 4);
  lv_obj_set_width(g_strip.l1, STRIP_W - 2 * CELL_PAD - 2);
  lv_label_set_long_mode(g_strip.l1, LV_LABEL_LONG_WRAP);
  g_strip.l2 = mklabel(s, "", &lv_font_montserrat_12, C_MUTED);
  lv_obj_set_pos(g_strip.l2, 0, 50);
  lv_obj_set_width(g_strip.l2, STRIP_W - 2 * CELL_PAD - 2);
  lv_label_set_long_mode(g_strip.l2, LV_LABEL_LONG_WRAP);
  return s;
}
// botao pequeno dentro da faixa (layout livre)
static lv_obj_t *strip_btn(lv_obj_t *s, const lv_image_dsc_t *ic, const char *txt, uint32_t col,
                           lv_event_cb_t cb, void *ud) {
  lv_obj_t *b = lv_obj_create(s);
  lv_obj_set_size(b, 96, 40);
  lv_obj_set_style_bg_color(b, lv_color_hex(C_CELL), 0);
  lv_obj_set_style_bg_color(b, lv_color_hex(0x1E1F1D), LV_STATE_PRESSED);
  lv_obj_set_style_border_width(b, 2, 0);
  lv_obj_set_style_border_color(b, lv_color_hex(col), 0);
  lv_obj_set_style_border_opa(b, 220, 0);
  lv_obj_set_style_radius(b, 20, 0);                  // pilula (tema Stitch)
  lv_obj_set_style_pad_all(b, 0, 0);
  lv_obj_clear_flag(b, LV_OBJ_FLAG_SCROLLABLE);
  lv_obj_add_flag(b, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_set_ext_click_area(b, 6);
  lv_obj_t *im = icon(b, ic, col, 255);
  lv_obj_align(im, LV_ALIGN_LEFT_MID, 10, 0);
  lv_obj_t *l = mklabel(b, upcase_lbl(txt), &font_ms_sb_12, col);
  lv_obj_align(l, LV_ALIGN_LEFT_MID, 34, 0);
  if (cb) lv_obj_add_event_cb(b, cb, LV_EVENT_CLICKED, ud);
  return b;
}

// ---- nomes / cores ----
static const char *mode_name(uint8_t m) {
  switch (m) {
    case SM_DEFAULT:      return "ask";
    case SM_ACCEPT_EDITS: return "edits";
    case SM_PLAN:         return "plan";
    case SM_BYPASS:       return "bypass";
    case SM_DONT_ASK:     return "auto";
    default:              return "--";
  }
}
static const char *state_name(uint8_t s) {
  switch (s) {
    case SS_UNKNOWN:   return TRS("sem hooks", "no hooks");
    case SS_WORKING:   return TRS("trabalhando", "working");
    case SS_ATTENTION: return TRS("precisa de voce", "needs you");
    case SS_DONE:      return TRS("terminou", "finished");
    case SS_IDLE:      return TRS("ocioso", "idle");
    case SS_ERROR:     return TRS("erro", "error");
    case SS_DEAD:      return TRS("encerrada", "ended");
    default:           return TRS("vazio", "empty");
  }
}
static uint32_t state_color(uint8_t s) {
  switch (s) {
    case SS_WORKING:   return C_WORK;
    case SS_ATTENTION: return C_ATTN;
    case SS_DONE:      return C_DONE;
    case SS_IDLE:      return C_IDLE;
    case SS_ERROR:     return C_ERR;
    case SS_DEAD:      return C_FAINT;
    case SS_UNKNOWN:   return C_MUTED;
    default:           return C_FAINT;
  }
}
// mistura "k/255 de col sobre o vidro" (cor solida — barata)
static uint32_t over_glass(uint32_t col, uint8_t k) {
  return lv_color_to_u32(lv_color_mix(lv_color_hex(col), lv_color_hex(C_GLASS), k)) & 0xFFFFFF;
}
static uint32_t sess_age(const SessEntry &e) {
  if (!g_model.valid) return 0;
  return (uint32_t)e.age + (millis() - g_model.rxMs) / 1000;
}
static void fmt_age(uint32_t s, char *out, int sz) {
  if (s < 60)        snprintf(out, sz, "%us", (unsigned)s);
  else if (s < 3600) snprintf(out, sz, "%um", (unsigned)(s / 60));
  else               snprintf(out, sz, "%uh", (unsigned)(s / 3600));
}
static bool cell_occupied(int i) {
  return g_model.valid && i >= 0 && i < DECK_SESSION_CELLS &&
         g_model.s[i].sid != 0 && g_model.s[i].state != SS_EMPTY;
}
static bool has_active() {
  return g_model.valid && g_model.active != CELL_ACTIVE && cell_occupied(g_model.active);
}
static void sanitize_label(const uint8_t *src, char *dst) {
  int n = 0;
  for (int i = 0; i < DECK_LABEL_LEN; i++) {
    uint8_t c = src[i];
    if (c == 0) break;
    dst[n++] = (c >= 32 && c < 127) ? (char)c : '?';
  }
  dst[n] = 0;
}

// ============================================================
// NVS
// ============================================================
static void load_persisted() {
  g_prefs.begin(NVS_NAMESPACE, false);
  int b = g_prefs.getUChar("bri", BRI_DEFAULT);
  g_bri = (b < BRI_MIN) ? BRI_MIN : (b > 255 ? 255 : b);
  g_lang = g_prefs.getUChar("lang", 0) ? 1 : 0;
}
static void apply_brightness() { ledcWrite(TFT_BL, g_bri); }
static void save_brightness() { g_prefs.putUChar("bri", g_bri); apply_brightness(); }
static void save_lang()       { g_prefs.putUChar("lang", g_lang); }

// ============================================================
// Protocolo — parse dos payloads (PROTOCOL.md §4)
// ============================================================
static bool parse_sessions(const uint8_t *d, uint16_t n) {
  if (n < 4 || d[0] != PROTO_VERSION) return false;
  uint8_t cnt = d[2];
  if (cnt > DECK_PROTO_SESSIONS) cnt = DECK_PROTO_SESSIONS;
  if (n < (uint16_t)(4 + 18 * cnt)) return false;
  DeckModel m = {};
  m.valid = true; m.flags = d[1]; m.active = d[3];
  if (m.active != CELL_ACTIVE && m.active >= DECK_SESSION_CELLS) m.active = CELL_ACTIVE;   // 6-7: fora da tela
  for (int i = 0; i < cnt; i++) {
    const uint8_t *e = d + 4 + 18 * i;
    SessEntry &s = m.s[i];
    s.sid   = e[0];
    s.state = e[1] > SS_DEAD ? SS_UNKNOWN : e[1];
    s.mode  = e[2] > SM_DONT_ASK ? SM_UNKNOWN : e[2];
    s.flags = e[3];
    s.age   = (uint16_t)(e[4] | (e[5] << 8));
    sanitize_label(e + 6, s.label);
    if (s.sid == 0) s.state = SS_EMPTY;
  }
  m.rxMs = millis();
  g_model = m;
  return true;
}
static bool parse_usage(const uint8_t *d, uint16_t n) {
  if (n < 15 || d[0] != PROTO_VERSION) return false;
  auto u32 = [](const uint8_t *p) -> uint32_t {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
  };
  g_usage.valid = true;
  g_usage.p5 = d[1]; g_usage.p7 = d[2];
  g_usage.reset5 = u32(d + 3); g_usage.reset7 = u32(d + 7); g_usage.hostNow = u32(d + 11);
  g_usage.rxMs = millis();
  return true;
}
// retorna bitmask do que mudou: 1 brilho, 2 idioma, 4 comandos
static int parse_config(const uint8_t *d, uint16_t n) {
  if (n < 1 || d[0] != PROTO_VERSION) return 0;
  int changed = 0;
  uint16_t i = 1;
  while (i + 2 <= n) {
    uint8_t t = d[i], len = d[i + 1];
    const uint8_t *v = d + i + 2;
    if (i + 2 + len > n) break;
    if (t == 1 && len >= 1) {
      int b = v[0]; g_bri = b < BRI_MIN ? BRI_MIN : b; save_brightness(); changed |= 1;
    } else if (t == 2 && len >= 1) {
      uint8_t l = v[0] ? 1 : 0;
      if (l != g_lang) { g_lang = l; save_lang(); changed |= 2; }
    } else if (t == 3 && len >= 1) {
      int cnt = v[0];
      if (cnt > DECK_CUSTOM_MAX) cnt = DECK_CUSTOM_MAX;
      if (1 + cnt * DECK_LABEL_LEN <= len) {
        g_customN = 0;
        for (int k = 0; k < cnt; k++) {
          char tmp[DECK_LABEL_LEN + 1];
          sanitize_label(v + 1 + k * DECK_LABEL_LEN, tmp);
          CustomCmd &c = g_custom[g_customN++];
          c.confirm = (tmp[0] == '!');
          strlcpy(c.label, c.confirm ? tmp + 1 : tmp, sizeof(c.label));
        }
        changed |= 4;
      }
    }
    i += 2 + len;
  }
  return changed;
}

// ---- envio de acoes ----
static void send_action(uint8_t act, uint8_t cell) { BleLink::sendEvent(EV_ACTION, cell, act); }
static void send_page(uint8_t page) { BleLink::sendEvent(EV_DECK, DECK_PAGE, page); }

// ============================================================
// Tela: SEARCH — procurando host (mascote + status BLE na faixa)
// ============================================================
static void search_status_text() {
  if (!g_strip.l1) return;
  char s[96];
  const char *err = BleLink::lastError();
  if (err[0]) snprintf(s, sizeof(s), "BLE: %s", err);
  else if (!BleLink::isConnected()) {
    int nb = BleLink::numBonds();
    if (nb > 0) snprintf(s, sizeof(s), TRS("anunciando por BLE" BULLET "%d pareado(s)",
                                           "advertising over BLE" BULLET "%d bonded"), nb);
    else        snprintf(s, sizeof(s), "%s", TRS("anunciando por BLE" BULLET "sem pareamento",
                                                 "advertising over BLE" BULLET "not paired"));
  }
  else if (BleLink::isPairing())     snprintf(s, sizeof(s), "%s", TRS("pareando...", "pairing..."));
  else if (!BleLink::isSubscribed()) snprintf(s, sizeof(s), "%s", TRS("conectado, aguardando agente", "connected, waiting for agent"));
  else                               snprintf(s, sizeof(s), "%s", TRS("conectado, aguardando sessoes", "connected, waiting for sessions"));
  lv_label_set_text(g_strip.l1, s);
  if (g_search.title)
    lv_label_set_text(g_search.title, BleLink::isConnected() ? TRS("conectado!", "connected!")
                                                             : TRS("procurando host...", "searching host..."));
  if (g_search.img) icon_color(g_search.img, BleLink::isConnected() ? C_DONE : C_ACCENT);
  for (int k = 0; k < 2; k++)
    if (g_search.lid[k]) lv_obj_set_style_bg_color(g_search.lid[k], lv_color_hex(BleLink::isConnected() ? C_DONE : C_ACCENT), 0);
}
static void search_foot_text() {
  if (!g_strip.l2) return;
  char s[128];
  snprintf(s, sizeof(s), "%s" BULLET "fw %s\nheap %uk" BULLET "mtu %u" BULLET "dc %02X" BULLET "%s",
           BleLink::mac(), FW_VERSION,
           (unsigned)(ESP.getFreeHeap() / 1024), (unsigned)BleLink::mtu(), (unsigned)BleLink::lastDisconnect(),
           TRS("abra o agente no computador", "run the agent on your computer"));
  lv_label_set_text(g_strip.l2, s);
}
static void ui_search() {
  lv_obj_t *scr = lv_screen_active();
  // mascote grande no centro da area das linhas 0-1 (a caixa inteira flutua no loop)
  lv_obj_t *c = lv_obj_create(scr);
  no_box(c);
  g_search.baseY = 62;
  lv_obj_set_pos(c, (LV_HOR - IC_CLOW_A_4_W) / 2, g_search.baseY);
  lv_obj_set_size(c, IC_CLOW_A_4_W, IC_CLOW_A_4_H);
  g_search.img = icon(c, &ic_clow_a_4, C_ACCENT, 255);
  lv_obj_set_pos(g_search.img, 0, 0);
  for (int k = 0; k < 2; k++) {
    int ex = (k == 0 ? CLOW_A_EYE0_X : CLOW_A_EYE1_X) * 4, ey = CLOW_A_EYE0_Y * 4;
    g_search.lid[k] = rrect(c, ex, ey, CLOW_A_EYE0_W * 4, CLOW_A_EYE0_H * 4, 0, C_ACCENT);
    lv_obj_add_flag(g_search.lid[k], LV_OBJ_FLAG_HIDDEN);
  }
  g_search.cont = c;
  // segurar o mascote abre os ajustes (da p/ ajustar brilho sem agente)
  lv_obj_add_flag(c, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_set_ext_click_area(c, 24);
  lv_obj_add_event_cb(c, [](lv_event_t *e) { request_state(ST_SETTINGS); }, LV_EVENT_LONG_PRESSED, NULL);

  g_search.title = mklabel(scr, "", &lv_font_montserrat_20, C_TEXT);
  lv_obj_set_width(g_search.title, LV_HOR);
  lv_obj_set_style_text_align(g_search.title, LV_TEXT_ALIGN_CENTER, 0);
  lv_obj_set_pos(g_search.title, 0, 168);
  char sub[64];
  snprintf(sub, sizeof(sub), "%s" BULLET "%s", DECK_NAME, TRS("deck do Claude Code", "Claude Code deck"));
  g_search.sub = mklabel(scr, sub, &lv_font_montserrat_12, C_MUTED);
  lv_obj_set_width(g_search.sub, LV_HOR);
  lv_obj_set_style_text_align(g_search.sub, LV_TEXT_ALIGN_CENTER, 0);
  lv_obj_set_pos(g_search.sub, 0, 198);

  // linha 2: placeholders (grade sempre presente)
  for (int i = 6; i < GRID_CELLS; i++) cell_placeholder(scr, i);
  strip(scr);
  search_status_text();
  search_foot_text();
}

// ============================================================
// Overlay: passkey (so enquanto ha pareamento em curso)
// ============================================================
static lv_obj_t *overlay_scrim(uint8_t opa) {
  lv_obj_t *s = lv_obj_create(lv_layer_top());
  lv_obj_set_pos(s, 0, 0); lv_obj_set_size(s, LV_HOR, LV_VER);
  lv_obj_set_style_bg_color(s, lv_color_hex(C_BG_TOP), 0);
  lv_obj_set_style_bg_opa(s, opa, 0);
  lv_obj_set_style_border_width(s, 0, 0);
  lv_obj_set_style_radius(s, 0, 0);
  lv_obj_set_style_pad_all(s, 0, 0);
  lv_obj_clear_flag(s, LV_OBJ_FLAG_SCROLLABLE);
  lv_obj_add_flag(s, LV_OBJ_FLAG_CLICKABLE);          // absorve toques
  return s;
}
static lv_obj_t *overlay_box(lv_obj_t *s, int w, int h) {
  lv_obj_t *box = glass(s, 0, 0, w, h, R_BOX, false);
  lv_obj_set_style_bg_color(box, lv_color_hex(C_GLASS_HI), 0);
  lv_obj_set_style_bg_opa(box, 250, 0);
  lv_obj_set_style_pad_all(box, 16, 0);
  lv_obj_add_flag(box, LV_OBJ_FLAG_CLICKABLE);        // nao deixa o clique vazar p/ o scrim
  lv_obj_center(box);
  return box;
}
static void passkey_show() {
  if (g_pkScrim) return;
  lv_obj_t *s = overlay_scrim(235);
  g_pkScrim = s;
  lv_obj_t *box = overlay_box(s, 288, 230);
  lv_obj_t *im = icon(box, &ic_bt_4, C_ACCENT, 255);
  lv_obj_align(im, LV_ALIGN_TOP_MID, 0, -4);
  lv_obj_t *t = mklabel(box, TRS("Pareamento Bluetooth", "Bluetooth pairing"), &font_ms_sb_18, C_TEXT);
  lv_obj_align(t, LV_ALIGN_TOP_MID, 0, 34);
  lv_obj_t *h = mklabel(box, TRS("digite este codigo no computador", "type this code on your computer"),
                        &lv_font_montserrat_12, C_MUTED);
  lv_obj_align(h, LV_ALIGN_TOP_MID, 0, 60);
  char pk[16];
  uint32_t p = BleLink::passkey();
  snprintf(pk, sizeof(pk), "%03u %03u", (unsigned)(p / 1000), (unsigned)(p % 1000));
  lv_obj_t *pkbox = rrect(box, 0, 0, 256, 72, R_BTN, C_GLASS);
  lv_obj_align(pkbox, LV_ALIGN_TOP_MID, 0, 86);
  lv_obj_set_style_border_width(pkbox, 2, 0);
  lv_obj_set_style_border_color(pkbox, lv_color_hex(C_ACCENT), 0);
  lv_obj_t *pkl = mklabel(pkbox, pk, &font_ms_b_40, C_TEXT);
  lv_obj_set_style_text_letter_space(pkl, 3, 0);
  lv_obj_center(pkl);
  lv_obj_t *f = mklabel(box, TRS("o codigo muda a cada reinicio do deck", "the code changes on every deck reboot"),
                        &lv_font_montserrat_12, C_FAINT);
  lv_obj_align(f, LV_ALIGN_BOTTOM_MID, 0, 0);
}
static void passkey_hide() {
  if (!g_pkScrim) return;
  lv_obj_delete(g_pkScrim);
  g_pkScrim = nullptr;
}

// ============================================================
// Overlay: confirmacao (Confirmar / Cancelar)
// ============================================================
static void confirm_yes_cb(lv_event_t *e) { (void)e; g_cf.req = 1; }
static void confirm_no_cb(lv_event_t *e)  { (void)e; g_cf.req = 2; }
static void confirm_close() {
  if (g_cf.scrim) lv_obj_delete(g_cf.scrim);
  g_cf.scrim = nullptr; g_cf.payload = 0; g_cf.req = 0;
}
static lv_obj_t *pill_btn(lv_obj_t *p, const char *txt, uint32_t bg, uint32_t fg) {
  lv_obj_t *b = lv_button_create(p);
  lv_obj_set_style_bg_color(b, lv_color_hex(bg), 0);
  lv_obj_set_style_bg_color(b, lv_color_mix(lv_color_hex(C_TEXT), lv_color_hex(bg), 50), LV_STATE_PRESSED);
  lv_obj_set_style_radius(b, R_BTN, 0);
  lv_obj_set_style_shadow_width(b, 0, 0);
  lv_obj_set_style_border_width(b, 1, 0);
  lv_obj_set_style_border_color(b, lv_color_hex(C_EDGE), 0);
  lv_obj_set_style_border_opa(b, EDGE_OPA, 0);
  lv_obj_center(mklabel(b, txt, &lv_font_montserrat_16, fg));
  return b;
}
static void show_confirm(const char *title, const char *sub, uint32_t payload) {
  confirm_close();
  g_cf.payload = payload;
  lv_obj_t *s = overlay_scrim(200);
  g_cf.scrim = s;
  lv_obj_add_event_cb(s, confirm_no_cb, LV_EVENT_CLICKED, NULL);   // fora da caixa = cancela
  lv_obj_t *box = overlay_box(s, 288, 210);
  lv_obj_t *t = mklabel(box, title, &font_ms_sb_18, C_TEXT);
  lv_obj_set_pos(t, 0, 0);
  lv_obj_t *u = mklabel(box, sub, &lv_font_montserrat_12, C_MUTED);
  lv_obj_set_pos(u, 0, 34);
  lv_obj_set_width(u, 256);
  lv_label_set_long_mode(u, LV_LABEL_LONG_WRAP);
  lv_obj_t *no = pill_btn(box, TRS("Cancelar", "Cancel"), C_GLASS, C_TEXT);
  lv_obj_set_size(no, 120, 48);
  lv_obj_align(no, LV_ALIGN_BOTTOM_LEFT, 0, 0);
  lv_obj_add_event_cb(no, confirm_no_cb, LV_EVENT_CLICKED, NULL);
  lv_obj_t *yes = pill_btn(box, TRS("Confirmar", "Confirm"), C_ERR, C_ON_ATTN);
  lv_obj_set_size(yes, 120, 48);
  lv_obj_align(yes, LV_ALIGN_BOTTOM_RIGHT, 0, 0);
  lv_obj_add_event_cb(yes, confirm_yes_cb, LV_EVENT_CLICKED, NULL);
}
// executa um payload (acao BLE e/ou op local)
static void do_payload(uint32_t p) {
  if (p & PL_FORGET) {
    BleLink::forgetBonds();
    g_model.valid = false;
    request_state(ST_SEARCH);
    return;
  }
  send_action(PL_ACT(p), PL_CELL(p));
  if (p & PL_TOGRID) request_state(ST_GRID);
}
// botao de acao generico: user_data = payload (PL_* flags)
static void act_btn_cb(lv_event_t *e) {
  uint32_t p = (uint32_t)(intptr_t)lv_event_get_user_data(e);
  if (p & PL_CONFIRM) {
    uint8_t a = PL_ACT(p);
    const char *what = (a == ACT_CLEAR) ? "/clear" : (a >= ACT_CUSTOM_BASE) ? TRS("comando", "command") : "?";
    char t[40], s[96];
    snprintf(t, sizeof(t), TRS("Enviar %s?", "Send %s?"), what);
    snprintf(s, sizeof(s), "%s", (a == ACT_CLEAR)
             ? TRS("Apaga o contexto da sessao. Nao da para desfazer.", "Wipes the session context. Cannot be undone.")
             : TRS("O comando sera digitado na sessao.", "The command will be typed into the session."));
    show_confirm(t, s, p & ~PL_CONFIRM);
    return;
  }
  do_payload(p);
}

// ============================================================
// Tela: HOME (grade) — 6 sessoes + 3 utilitarios + faixa de status
// ============================================================
static void cell_ev_cb(lv_event_t *e) {
  int i = (int)(intptr_t)lv_event_get_user_data(e);
  lv_event_code_t code = lv_event_get_code(e);
  if (!cell_occupied(i)) return;
  if (code == LV_EVENT_SHORT_CLICKED) {
    BleLink::sendEvent(EV_CELL_TAP, i, 0);
  } else if (code == LV_EVENT_LONG_PRESSED) {
    BleLink::sendEvent(EV_CELL_HOLD, i, 0);
    g_selCell = i;
    request_state(ST_SESSION);
  }
}
static void lang_toggle_cb(lv_event_t *e) { (void)e; g_lang ^= 1; save_lang(); request_state(g_state); }
static const uint8_t BRI_LEVELS[3] = { 80, 170, 255 };
static void bri_cycle_cb(lv_event_t *e) {
  (void)e;
  int next = 0;
  for (int i = 0; i < 3; i++) if (g_bri <= BRI_LEVELS[i]) { next = (i + 1) % 3; break; }
  g_bri = BRI_LEVELS[next];
  save_brightness();
  if (g_grid.briVal) { char s[8]; snprintf(s, sizeof(s), "%d%%", (int)(g_bri * 100 / 255)); lv_label_set_text(g_grid.briVal, s); }
}
static void util_hold_cb(lv_event_t *e) { (void)e; request_state(ST_SETTINGS); }

static void ui_grid() {
  lv_obj_t *scr = lv_screen_active();
  for (int i = 0; i < DECK_SESSION_CELLS; i++) {
    CellUI &c = g_grid.c[i];
    c.box = glass(scr, cell_x(i), cell_y(i), CELL_W, CELL_H, R_CELL, true);
    lv_obj_add_event_cb(c.box, cell_ev_cb, LV_EVENT_ALL, (void *)(intptr_t)i);
    // mascote ao fundo (criado primeiro = fica atras), esmaecido e tingido pelo estado
    c.masc = icon(c.box, &ic_claude_4, C_FAINT, 0);   // trocado por engine no grid_apply
    lv_obj_align(c.masc, LV_ALIGN_CENTER, 0, -8);   // mascote central (tema Stitch)
    c.lbl = mklabel(c.box, "---", &font_ms_sb_10, C_TEXT);
    lv_obj_set_width(c.lbl, CELL_W - 2 * CELL_PAD - 2);
    lv_obj_set_style_text_align(c.lbl, LV_TEXT_ALIGN_CENTER, 0);
    lv_obj_set_style_text_letter_space(c.lbl, 1, 0);
    lv_label_set_long_mode(c.lbl, LV_LABEL_LONG_WRAP);
    lv_obj_align(c.lbl, LV_ALIGN_BOTTOM_MID, 0, 0);
    c.dot = rrect(c.box, 0, 3, 8, 8, 4, C_FAINT);
    c.chip = lv_obj_create(c.box);
    lv_obj_set_size(c.chip, LV_SIZE_CONTENT, 20);
    lv_obj_set_style_radius(c.chip, R_CHIP, 0);
    lv_obj_set_style_pad_hor(c.chip, 7, 0);
    lv_obj_set_style_pad_ver(c.chip, 0, 0);
    lv_obj_set_style_border_width(c.chip, 1, 0);
    lv_obj_set_style_border_opa(c.chip, 160, 0);
    lv_obj_set_style_border_color(c.chip, lv_color_hex(C_FAINT), 0);
    lv_obj_set_style_bg_color(c.chip, lv_color_hex(C_GLASS_HI), 0);
    lv_obj_set_style_bg_opa(c.chip, 55, 0);
    lv_obj_clear_flag(c.chip, LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_clear_flag(c.chip, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_align(c.chip, LV_ALIGN_TOP_RIGHT, 2, -2);    // badge flutuante (tema Stitch)
    c.chipLbl = mklabel(c.chip, "--", &font_ms_sb_12, C_MUTED);
    lv_obj_center(c.chipLbl);
    c.age = mklabel(c.box, "", &font_ms_sb_10, C_MUTED);
    lv_obj_set_pos(c.age, 12, 2);
    c.lastBg = c.lastBd = c.lastTx = c.lastMc = 0xFFFFFFFF; c.lastBw = c.lastMo = -1;
  }
  // linha 2 — utilitarios: idioma, brilho, ajustes
  {
    lv_obj_t *b = cell_btn(scr, UTIL_LANG, &ic_lang_4, TRS("idioma", "language"), C_ACCENT_HI, lang_toggle_cb, NULL);
    g_grid.langVal = mklabel(b, g_lang ? "EN" : "PT", &lv_font_montserrat_20, C_ACCENT);
    lv_obj_align(g_grid.langVal, LV_ALIGN_CENTER, 0, 14);
    lv_obj_add_event_cb(b, util_hold_cb, LV_EVENT_LONG_PRESSED, NULL);
  }
  {
    lv_obj_t *b = cell_btn(scr, UTIL_BRI, &ic_bright_4, TRS("brilho", "brightness"), C_ACCENT_HI, bri_cycle_cb, NULL);
    char s[8]; snprintf(s, sizeof(s), "%d%%", (int)(g_bri * 100 / 255));
    g_grid.briVal = mklabel(b, s, &lv_font_montserrat_20, C_ACCENT);
    lv_obj_align(g_grid.briVal, LV_ALIGN_CENTER, 0, 14);
    lv_obj_add_event_cb(b, util_hold_cb, LV_EVENT_LONG_PRESSED, NULL);
  }
  cell_btn(scr, UTIL_GEAR, &ic_gear_4, TRS("ajustes", "settings"), C_ACCENT_HI, nav_cb, (void *)(intptr_t)ST_SETTINGS);

  // faixa: marca ClowDeck (mascote + nome); status do link fica nos icones do canto
  lv_obj_t *s = strip(scr);
  g_grid.bleIco = icon(s, &ic_bt_2, C_WORK, 255);
  lv_obj_align(g_grid.bleIco, LV_ALIGN_TOP_RIGHT, 0, 2);
  g_grid.warnIco = icon(s, &ic_stale_2, C_ATTN, 255);
  lv_obj_align(g_grid.warnIco, LV_ALIGN_TOP_RIGHT, -24, 2);
  lv_obj_add_flag(g_grid.warnIco, LV_OBJ_FLAG_HIDDEN);
  lv_obj_t *m = icon(s, &ic_clow_a_2, C_ACCENT, 255);
  lv_obj_align(m, LV_ALIGN_CENTER, -80, 0);
  lv_obj_t *bn = mklabel(s, "CLOW DECK", &font_ms_sb_18, C_TEXT);
  lv_obj_set_style_text_letter_space(bn, 3, 0);
  lv_obj_align(bn, LV_ALIGN_CENTER, 38, 0);
  grid_apply();
  grid_tick_1s();
}

// aplica o modelo nas celulas (textos/chips/pontos); cores animadas em grid_anim
static void grid_apply() {
  if (!g_grid.c[0].box) return;
  for (int i = 0; i < DECK_SESSION_CELLS; i++) {
    CellUI &c = g_grid.c[i];
    const SessEntry &e = g_model.s[i];
    bool occ = cell_occupied(i);
    lv_label_set_text(c.lbl, occ ? e.label : "---");
    lv_obj_set_style_text_decor(c.lbl, e.state == SS_DEAD ? LV_TEXT_DECOR_STRIKETHROUGH : LV_TEXT_DECOR_NONE, 0);
    lv_obj_set_style_bg_color(c.dot, lv_color_hex(state_color(e.state)), 0);
    lv_image_set_src(c.masc, (e.flags & 8) ? &ic_oclogo_4 : (e.flags & 4) ? &ic_codexlogo_4 : &ic_claude_4);
    if (occ) {
      lv_obj_clear_flag(c.dot, LV_OBJ_FLAG_HIDDEN);
      lv_obj_clear_flag(c.chip, LV_OBJ_FLAG_HIDDEN);
      lv_obj_clear_flag(c.masc, LV_OBJ_FLAG_HIDDEN);
      lv_label_set_text(c.chipLbl, (e.flags & 8) ? "OC" : (e.flags & 4) ? "CDX" : (e.flags & 2) ? TRS("sem hooks", "no hooks") : mode_name(e.mode));
      lv_obj_set_style_opa(c.box, e.state == SS_DEAD ? 140 : 255, 0);
    } else {
      lv_obj_add_flag(c.dot, LV_OBJ_FLAG_HIDDEN);
      lv_obj_add_flag(c.chip, LV_OBJ_FLAG_HIDDEN);
      lv_obj_add_flag(c.masc, LV_OBJ_FLAG_HIDDEN);
      lv_label_set_text(c.age, "");
      lv_obj_set_style_opa(c.box, 110, 0);
    }
    c.lastBg = c.lastBd = c.lastTx = c.lastMc = 0xFFFFFFFF; c.lastBw = c.lastMo = -1;   // forca reaplicar
  }
  grid_anim(millis());
}

// animacoes por estado (ANIM_TICK_MS): so escreve estilo quando algo muda
static void grid_anim(uint32_t now) {
  for (int i = 0; i < DECK_SESSION_CELLS; i++) {
    CellUI &c = g_grid.c[i];
    if (!c.box) continue;
    const SessEntry &e = g_model.s[i];
    bool occ = cell_occupied(i);
    bool active = occ && ((g_model.active == i) || (e.flags & 1));
    uint32_t bg = C_CELL, glow = 0, tx = C_TEXT, bd = C_CELL_DIM, mc = state_color(e.state);
    int bw = 2, mo = 0;
    if (occ) {
      switch (e.state) {
        case SS_WORKING: {
          float ph = (now % 1200) / 1200.0f;
          uint8_t k = (uint8_t)((sinf(ph * 6.2831853f) + 1.0f) * 14.0f);     // 0..28: pulso sutil
          bg = over_cell(C_WORK, k);
          bd = C_WORK; glow = C_WORK;
          mo = 110;
          break;
        }
        case SS_ATTENTION: {
          bool on = ((now / 500) & 1) == 0;
          bg = on ? C_ATTN : over_cell(C_ATTN, 45);
          tx = on ? C_ON_ATTN : C_TEXT;
          mc = on ? C_ON_ATTN : C_ATTN;
          bd = C_ATTN; glow = C_ATTN;
          mo = 150;
          break;
        }
        case SS_DONE: {
          bool on = sess_age(e) < 60 && (((now / 1000) & 1) == 0);
          bg = over_cell(C_DONE, on ? 45 : 20);
          bd = C_DONE; glow = C_DONE;
          mo = 70;
          break;
        }
        case SS_IDLE:    bd = C_CELL_LINE; mo = 46; break;
        case SS_ERROR:   bg = over_cell(C_ERR, 30); bd = C_ERR; glow = C_ERR; mo = 70; break;
        case SS_DEAD:    tx = C_FAINT; mo = 40; break;
        case SS_UNKNOWN: bd = C_CELL_LINE; mo = 46; break;
        default:         mo = 46; break;
      }
    } else {
      tx = C_FAINT;
    }
    if (active) { bw = 3; bd = C_ACCENT; glow = C_ACCENT; mc = C_ACCENT; if (mo && mo < 200) mo = 200; }
    if (bg != c.lastBg) { lv_obj_set_style_bg_color(c.box, lv_color_hex(bg), 0); c.lastBg = bg; }
    if (glow != c.lastGr) {                        // lastGr agora guarda a cor do glow
      lv_obj_set_style_shadow_width(c.box, glow ? 14 : 0, 0);
      lv_obj_set_style_shadow_spread(c.box, glow ? 2 : 0, 0);
      lv_obj_set_style_shadow_offset_y(c.box, 0, 0);
      lv_obj_set_style_shadow_color(c.box, lv_color_hex(glow), 0);
      lv_obj_set_style_shadow_opa(c.box, 110, 0);
      c.lastGr = glow;
    }
    if (tx != c.lastTx) { lv_obj_set_style_text_color(c.lbl, lv_color_hex(tx), 0); c.lastTx = tx; }
    if (bw != c.lastBw) { lv_obj_set_style_border_width(c.box, bw, 0); c.lastBw = bw; }
    if (bd != c.lastBd) { lv_obj_set_style_border_color(c.box, lv_color_hex(bd), 0); c.lastBd = bd; }
    if (mc != c.lastMc) {
      icon_color(c.masc, mc);
      lv_obj_set_style_bg_color(c.chip, lv_color_hex(mc), 0);
      lv_obj_set_style_border_color(c.chip, lv_color_hex(mc), 0);
      lv_obj_set_style_text_color(c.chipLbl, lv_color_hex(mc), 0);
      c.lastMc = mc;
    }
    if (mo != c.lastMo) { lv_obj_set_style_image_opa(c.masc, (lv_opa_t)mo, 0); c.lastMo = mo; }
  }
}

// 1 s: idades, faixa de status, aviso de agente parado
static void grid_tick_1s() {
  if (!g_grid.c[0].box) return;
  int open = 0, attn = 0, done = 0;
  for (int i = 0; i < DECK_SESSION_CELLS; i++) {
    CellUI &c = g_grid.c[i];
    const SessEntry &e = g_model.s[i];
    if (cell_occupied(i)) {
      open++;
      if (e.state == SS_ATTENTION) attn++;
      if (e.state == SS_DONE) done++;
    }
    if (cell_occupied(i) && (e.state == SS_DONE || e.state == SS_ATTENTION)) {
      char a[8]; fmt_age(sess_age(e), a, sizeof(a));
      lv_label_set_text(c.age, a);
      lv_obj_set_style_text_color(c.age, lv_color_hex(e.state == SS_ATTENTION ? C_ON_ATTN : C_MUTED), 0);
    } else if (c.age) lv_label_set_text(c.age, "");
  }
  if (g_grid.bleIco) icon_color(g_grid.bleIco, BleLink::isConnected() ? C_WORK : C_FAINT);
  if (g_grid.warnIco) {
    if (g_agentStale) lv_obj_clear_flag(g_grid.warnIco, LV_OBJ_FLAG_HIDDEN);
    else              lv_obj_add_flag(g_grid.warnIco, LV_OBJ_FLAG_HIDDEN);
  }
}

// ============================================================
// Tela: SESSION — 9 botoes nas celulas + info na faixa
// ============================================================
static void voice_set(const char *txt, uint32_t col) {
  if (g_sess.voiceLbl) { lv_label_set_text(g_sess.voiceLbl, txt); lv_obj_set_style_text_color(g_sess.voiceLbl, lv_color_hex(col), 0); }
  icon_color(g_sess.voiceIco, col);
}
static void voice_idle_text() {
  bool avail = g_model.valid && (g_model.flags & 2);
  voice_set(avail ? TRS("voz: segure", "voice: hold") : TRS("voz", "voice"), avail ? C_TEXT : C_MUTED);
}
static void voice_ev_cb(lv_event_t *e) {
  lv_event_code_t code = lv_event_get_code(e);
  uint8_t c = (uint8_t)g_selCell;
  if (code == LV_EVENT_LONG_PRESSED) {
    g_voice = true;
    send_action(ACT_VOICE_START, c);
    voice_set(TRS("gravando...", "recording..."), C_ERR);
  } else if ((code == LV_EVENT_RELEASED || code == LV_EVENT_PRESS_LOST) && g_voice) {
    g_voice = false;
    send_action(ACT_VOICE_STOP, c);
    voice_idle_text();
  } else if (code == LV_EVENT_SHORT_CLICKED) {
    g_voiceHintUntil = millis() + 2000;
    voice_set(TRS("segure e fale", "hold and talk"), C_ACCENT);
  }
}
// pisca aprovar/negar quando o modelo espera Allow/Reject (estado ATTENTION)
static void session_anim(uint32_t now) {
  static bool lastOn = false, wasAttn = false;
  if (!g_sess.approveBtn || g_selCell < 0) { wasAttn = false; return; }
  bool attn = g_model.s[g_selCell].state == SS_ATTENTION;
  bool on = attn && (((now / 500) & 1) == 0);
  if (attn) {
    if (on != lastOn || !wasAttn) {
      lv_obj_t *b[2] = { g_sess.approveBtn, g_sess.denyBtn };
      for (int i = 0; i < 2; i++) {
        if (!b[i]) continue;
        lv_obj_set_style_border_width(b[i], 3, 0);
        lv_obj_set_style_border_color(b[i], lv_color_hex(C_ATTN), 0);
        lv_obj_set_style_bg_color(b[i], lv_color_hex(on ? over_cell(C_ATTN, 70) : C_CELL), 0);
      }
      lastOn = on;
    }
  } else if (wasAttn) {
    lv_obj_t *b[2] = { g_sess.approveBtn, g_sess.denyBtn };
    for (int i = 0; i < 2; i++) {
      if (!b[i]) continue;
      lv_obj_set_style_border_width(b[i], 2, 0);
      lv_obj_set_style_border_color(b[i], lv_color_hex(C_CELL_LINE), 0);
      lv_obj_set_style_bg_color(b[i], lv_color_hex(C_CELL), 0);
    }
  }
  wasAttn = attn;
}

static void session_apply() {
  if (!g_sess.title || g_selCell < 0) return;
  const SessEntry &e = g_model.s[g_selCell];
  char s[96];
  snprintf(s, sizeof(s), "%s", e.label[0] ? e.label : "---");
  lv_label_set_text(g_sess.title, s);
  lv_label_set_text(g_sess.stateLbl, state_name(e.state));
  lv_obj_set_style_text_color(g_sess.stateLbl, lv_color_hex(state_color(e.state)), 0);
  char a[8]; fmt_age(sess_age(e), a, sizeof(a));
  bool active = (g_model.active == g_selCell) || (e.flags & 1);
  snprintf(s, sizeof(s), TRS("modo %s" BULLET "%s" BULLET "celula %d%s", "mode %s" BULLET "%s" BULLET "cell %d%s"),
           mode_name(e.mode), a, g_selCell + 1, active ? TRS("  (ativa)", "  (active)") : "");
  lv_label_set_text(g_sess.info, s);
}
static void ui_session() {
  lv_obj_t *scr = lv_screen_active();
  uint8_t c = (uint8_t)g_selCell;
  back_cell(scr, ST_GRID);
  cell_btn(scr, 1, &ic_focus_4,   TRS("focar", "focus"),   C_ACCENT, act_btn_cb, (void *)(intptr_t)PL(ACT_FOCUS, c));
  bool cdx = (g_model.s[g_selCell].flags & 0x0C) != 0; // engine externo (Codex/opencode): sem /voice, com aprovar/negar
  if (cdx) {
    // sem /voice no Codex: a celula vira APROVAR (pedido pendente -> 'y' no TUI)
    g_sess.approveBtn = cell_btn(scr, 2, &ic_ack_4, TRS("aprovar", "approve"), C_DONE, act_btn_cb, (void *)(intptr_t)PL(ACT_APPROVE, c));
    g_sess.voiceIco = NULL;
    g_sess.voiceLbl = NULL;
  } else {
    lv_obj_t *b = cell_btn(scr, 2, &ic_voice_4, "", C_TEXT, NULL, NULL);
    g_sess.voiceIco = lv_obj_get_child(b, 0);          // filhos: 0 icone, 1 rotulo
    g_sess.voiceLbl = lv_obj_get_child(b, 1);
    lv_obj_add_event_cb(b, voice_ev_cb, LV_EVENT_ALL, NULL);
    voice_idle_text();
  }
  if (cdx)
    g_sess.denyBtn = cell_btn(scr, 3, &ic_esc_4, TRS("negar", "deny"), C_ERR, act_btn_cb, (void *)(intptr_t)PL(ACT_ESC, c));
  else
    cell_btn(scr, 3, &ic_mode_4,  TRS("modo", "mode"),     C_TEXT, act_btn_cb, (void *)(intptr_t)PL(ACT_MODE_CYCLE, c));
  cell_btn(scr, 4, &ic_esc_4,     "esc",                   C_TEXT, act_btn_cb, (void *)(intptr_t)PL(ACT_ESC, c));
  cell_btn(scr, 5, &ic_enter_4,   "enter",                 C_TEXT, act_btn_cb, (void *)(intptr_t)PL(ACT_ENTER, c));
  cell_btn(scr, 6, &ic_tab_4,     "tab",                   C_DONE, act_btn_cb, (void *)(intptr_t)PL(ACT_TAB, c));
  cell_btn(scr, 7, &ic_compact_4, "/compact",              C_TEXT, act_btn_cb, (void *)(intptr_t)PL(ACT_COMPACT, c));
  cell_btn(scr, 8, &ic_next_4,    TRS("mais", "more"),     C_MUTED, nav_cb, (void *)(intptr_t)ST_CMD);

  lv_obj_t *s = strip(scr);
  g_sess.title = g_strip.l1;
  lv_obj_set_style_text_font(g_sess.title, &font_ms_sb_18, 0);
  lv_obj_set_width(g_sess.title, 170);
  g_sess.stateLbl = mklabel(s, "", &lv_font_montserrat_12, C_MUTED);
  lv_obj_set_pos(g_sess.stateLbl, 0, 30);
  g_sess.info = g_strip.l2;
  lv_obj_set_pos(g_sess.info, 0, 56);
  lv_obj_set_width(g_sess.info, 170);
  lv_obj_t *m = icon(s, (g_model.s[g_selCell].flags & 8) ? &ic_oclogo_2 : (g_model.s[g_selCell].flags & 4) ? &ic_codexlogo_2 : &ic_claude_2,
                     state_color(g_model.s[g_selCell].state), 190);
  lv_obj_align(m, LV_ALIGN_BOTTOM_RIGHT, 0, 0);
  session_apply();
}

// ============================================================
// Tela: CMD — comandos (paginados) para a sessao selecionada/ativa
// ============================================================
static void cmd_next_cb(lv_event_t *e) { (void)e; g_cmdPage++; request_state(ST_CMD); }
static void ui_cmd() {
  lv_obj_t *scr = lv_screen_active();
  uint8_t target = (g_selCell >= 0 && cell_occupied(g_selCell)) ? (uint8_t)g_selCell : CELL_ACTIVE;
  back_cell(scr, (g_selCell >= 0 && cell_occupied(g_selCell)) ? ST_SESSION : ST_GRID);

  struct Item { const lv_image_dsc_t *ic; const char *t; uint32_t col, pl; };
  // so o que NAO esta na pagina da sessao (sem repetir) + /init + customs
  Item items[3 + DECK_CUSTOM_MAX];
  int n = 0;
  // "nova sessao": o agente mapeia por engine (claude /clear; codex e opencode /new)
  items[n++] = { &ic_plus_4,  TRS("nova sessao", "new session"), C_ERR, PL(ACT_CLEAR, target) | PL_CONFIRM };
  items[n++] = { &ic_cmd_4,   "/init",  C_ACCENT, PL(ACT_INIT, target) };
  items[n++] = { &ic_exit_4,  "/exit",  C_ERR,    PL(ACT_EXIT, target) | PL_CONFIRM };
  for (int i = 0; i < g_customN; i++)
    items[n++] = { &ic_cmd_4, g_custom[i].label, C_ACCENT,
                   PL(ACT_CUSTOM_BASE + i, target) | (g_custom[i].confirm ? PL_CONFIRM : 0) };
  const int per = 8;
  int pages = (n + per - 1) / per;
  if (pages < 1) pages = 1;
  if (g_cmdPage >= pages) g_cmdPage = 0;
  int start = g_cmdPage * per;
  bool more = (start + per) < n;
  int slots = more ? per - 1 : per;
  for (int k = 0; k < per; k++) {
    int idx = start + k;
    int cell = 1 + k;
    if (k < slots && idx < n)
      cell_btn(scr, cell, items[idx].ic, items[idx].t, items[idx].col, act_btn_cb, (void *)(intptr_t)items[idx].pl);
    else if (k == per - 1 && more)
      cell_btn(scr, cell, &ic_next_4, TRS("mais", "more"), C_MUTED, cmd_next_cb, NULL);
    else
      cell_placeholder(scr, cell);
  }
  strip(scr);
  char s[96];
  if (target != CELL_ACTIVE)
    snprintf(s, sizeof(s), TRS("alvo: %s", "target: %s"), g_model.s[target].label);
  else if (has_active())
    snprintf(s, sizeof(s), TRS("alvo: %s (sessao ativa)", "target: %s (active session)"), g_model.s[g_model.active].label);
  else
    snprintf(s, sizeof(s), "%s", TRS("nenhuma sessao ativa — toque numa sessao antes", "no active session — tap a session first"));
  lv_label_set_text(g_strip.l1, s);
  lv_obj_set_style_text_color(g_strip.l1, lv_color_hex(target != CELL_ACTIVE || has_active() ? C_TEXT : C_ATTN), 0);
  snprintf(s, sizeof(s), TRS("%d comando(s)" BULLET "pagina %d/%d" BULLET "customizados vem do agente (CONFIG)",
                             "%d command(s)" BULLET "page %d/%d" BULLET "custom ones come from the agent (CONFIG)"),
           n, g_cmdPage + 1, pages);
  lv_label_set_text(g_strip.l2, s);
}

// ============================================================
// Tela: SETTINGS / ABOUT
// ============================================================
static void settings_strip_text() {
  if (!g_strip.l1) return;
  char s[96];
  snprintf(s, sizeof(s), TRS("brilho %d%%" BULLET "idioma %s" BULLET "%d pareamento(s)",
                             "brightness %d%%" BULLET "language %s" BULLET "%d bond(s)"),
           (int)(g_bri * 100 / 255), g_lang ? "EN" : "PT", BleLink::numBonds());
  lv_label_set_text(g_strip.l1, s);
  snprintf(s, sizeof(s), "%s" BULLET "fw %s" BULLET "proto %d", BleLink::mac(), FW_VERSION, PROTO_VERSION);
  lv_label_set_text(g_strip.l2, s);
}
static void bri_cb(lv_event_t *e) {
  int d = (int)(intptr_t)lv_event_get_user_data(e);
  int b = (int)g_bri + d;
  if (b < BRI_MIN) b = BRI_MIN;
  if (b > 255) b = 255;
  g_bri = (uint8_t)b;
  save_brightness();
  settings_strip_text();
}
static void forget_cb(lv_event_t *e) {
  (void)e;
  show_confirm(TRS("Esquecer pareamento?", "Forget pairing?"),
               TRS("Apaga todos os bonds BLE. O computador tambem precisa esquecer o deck nos ajustes de Bluetooth.",
                   "Deletes all BLE bonds. The computer must also forget the deck in its Bluetooth settings."),
               PL_FORGET);
}
static void ui_settings() {
  lv_obj_t *scr = lv_screen_active();
  back_cell(scr, BleLink::isConnected() && g_model.valid ? ST_GRID : ST_SEARCH);
  cell_btn(scr, 1, &ic_bright_4, TRS("brilho -", "dimmer"),   C_TEXT, bri_cb, (void *)(intptr_t)-25);
  cell_btn(scr, 2, &ic_bright_4, TRS("brilho +", "brighter"), C_ACCENT, bri_cb, (void *)(intptr_t)25);
  cell_btn(scr, 3, &ic_lang_4,   g_lang ? "EN > PT" : "PT > EN", C_TEXT, lang_toggle_cb, NULL);
  cell_btn(scr, 4, &ic_bt_4,     TRS("esquecer par", "forget pair"), C_ERR, forget_cb, NULL);
  cell_btn(scr, 5, &ic_cmd_4,    TRS("sobre", "about"),      C_MUTED, nav_cb, (void *)(intptr_t)ST_ABOUT);
  for (int i = 6; i < GRID_CELLS; i++) cell_placeholder(scr, i);
  strip(scr);
  settings_strip_text();
}
static void ui_about() {
  lv_obj_t *scr = lv_screen_active();
  back_cell(scr, ST_SETTINGS);
  lv_obj_t *c = cell_placeholder(scr, 1);
  lv_obj_set_style_opa(c, 255, 0);
  lv_obj_t *m = icon(c, &ic_clow_b_4, C_ACCENT, 255);
  lv_obj_center(m);
  for (int i = 2; i < GRID_CELLS; i++) cell_placeholder(scr, i);
  strip(scr);
  lv_obj_set_style_text_font(g_strip.l1, &lv_font_montserrat_12, 0);
  lv_obj_set_pos(g_strip.l2, 0, 0);
  lv_obj_add_flag(g_strip.l1, LV_OBJ_FLAG_HIDDEN);
  char s[400];
  snprintf(s, sizeof(s),
           "Clow Deck" BULLET "fw %s" BULLET "proto %d\n"
           "BLE %s" BULLET "%s\nmtu %u" BULLET "bonds %d" BULLET "%s: %s" BULLET "%s: %s\n"
           "heap %uk" BULLET "psram %uk" BULLET "pos-BLE %uk\n%s",
           FW_VERSION, PROTO_VERSION,
           BleLink::mac(), DECK_BLE_SECURE ? TRS("cifrado", "encrypted") : TRS("ABERTO (debug)", "OPEN (debug)"),
           (unsigned)BleLink::mtu(), BleLink::numBonds(),
           TRS("conectado", "connected"), BleLink::isConnected() ? TRS("sim", "yes") : TRS("nao", "no"),
           TRS("autenticado", "authenticated"), BleLink::isAuthenticated() ? TRS("sim", "yes") : TRS("nao", "no"),
           (unsigned)(ESP.getFreeHeap() / 1024), (unsigned)(ESP.getFreePsram() / 1024),
           (unsigned)(BleLink::heapAfterInit() / 1024),
           TRS("Periferico burro: o agente manda o estado; o deck so desenha e devolve toques.",
               "Dumb peripheral: the agent sends state; the deck only draws and reports taps."));
  lv_label_set_text(g_strip.l2, s);
  lv_obj_set_style_text_line_space(g_strip.l2, 2, 0);
}

// ============================================================
// Render do estado atual
// ============================================================
static void render_state() {
  State prev = g_state;
  g_state = g_pending;
  // overlays vivem em lv_layer_top: zera os ponteiros antes de limpar
  passkey_hide();
  confirm_close();
  lv_obj_clean(lv_layer_top());
  memset(&g_grid, 0, sizeof(g_grid));
  memset(&g_search, 0, sizeof(g_search));
  memset(&g_sess, 0, sizeof(g_sess));
  memset(&g_strip, 0, sizeof(g_strip));
  g_voice = false;
  if (g_state != ST_CMD) g_cmdPage = 0;

  lv_obj_t *scr = lv_screen_active();
  lv_obj_clean(scr);
  // fundo: gradiente vertical profundo (design/THEME.md)
  lv_obj_set_style_bg_color(scr, lv_color_hex(C_BG_TOP), 0);
  lv_obj_set_style_bg_grad_color(scr, lv_color_hex(C_BG_BOTTOM), 0);
  lv_obj_set_style_bg_grad_dir(scr, LV_GRAD_DIR_VER, 0);
  lv_obj_set_style_bg_opa(scr, LV_OPA_COVER, 0);
  // ceu estrelado sutil (tile A8 sem emenda, recolorido no acento) — todas as telas
  {
    lv_obj_t *sky = lv_image_create(scr);
    lv_image_set_src(sky, &ic_bgtile_1);
    lv_obj_set_size(sky, LV_HOR, LV_VER);
    lv_image_set_inner_align(sky, LV_IMAGE_ALIGN_TILE);
    lv_obj_set_pos(sky, 0, 0);
    lv_obj_set_style_image_recolor(sky, lv_color_hex(C_ACCENT), 0);
    lv_obj_set_style_image_recolor_opa(sky, 255, 0);
    lv_obj_set_style_image_opa(sky, 22, 0);
    lv_obj_clear_flag(sky, LV_OBJ_FLAG_CLICKABLE);
  }

  switch (g_state) {
    case ST_SEARCH:   ui_search();   break;
    case ST_GRID:     ui_grid();     break;
    case ST_SESSION:  ui_session();  break;
    case ST_CMD:      ui_cmd();      break;
    case ST_SETTINGS: ui_settings(); break;
    case ST_ABOUT:    ui_about();    break;
    default: break;
  }
  // DECK/PAGE (§4.3) quando a pagina visivel muda
  uint8_t page = (g_state == ST_SESSION) ? PAGE_SESSION : (g_state == ST_CMD) ? PAGE_CMD
               : (g_state == ST_SETTINGS || g_state == ST_ABOUT) ? PAGE_SETTINGS : PAGE_GRID;
  if (g_state != prev) send_page(page);
}

// ============================================================
// Mensagens do agente
// ============================================================
static void handle_msg(const BleMsg &m) {
  if (m.chr == CHR_SESSIONS) {
    bool was = g_model.valid;
    if (!parse_sessions(m.data, m.len)) return;
    g_agentStale = false;
    if (!was && g_state == ST_SEARCH) { request_state(ST_GRID); return; }
    if (g_state == ST_GRID) { grid_apply(); grid_tick_1s(); }
    else if (g_state == ST_SESSION) {
      if (!cell_occupied(g_selCell)) request_state(ST_GRID);
      else session_apply();
    }
  } else if (m.chr == CHR_USAGE) {
    if (parse_usage(m.data, m.len) && g_state == ST_GRID) grid_tick_1s();
  } else if (m.chr == CHR_CONFIG) {
    int ch = parse_config(m.data, m.len);
    if ((ch & 2) || ((ch & 4) && g_state == ST_CMD)) request_state(g_state);
    else if ((ch & 1) && g_grid.briVal) { char s[8]; snprintf(s, sizeof(s), "%d%%", (int)(g_bri * 100 / 255)); lv_label_set_text(g_grid.briVal, s); }
  }
}

// ============================================================
// setup / loop
// ============================================================
void setup() {
  Serial.begin(115200);
  delay(300);
  Serial.println("\n=== Clow Deck " FW_VERSION " ===");

  // Display (pipeline validado: Canvas nativo 320x480, rotation=0; orientacao no flush)
  Arduino_DataBus *bus = new Arduino_ESP32QSPI(TFT_CS, TFT_SCK, TFT_SDA0, TFT_SDA1, TFT_SDA2, TFT_SDA3);
  Arduino_GFX *g = new Arduino_AXS15231B(bus, GFX_NOT_DEFINED, 0, false, 320, 480);
  gfx = new Arduino_Canvas(320, 480, g, 0, 0, 0);
  if (!gfx->begin(QSPI_FREQ)) { Serial.println("FATAL display"); while (1) delay(1000); }
  gfx->fillScreen(0x0000); gfx->flush();
  canvas_fb = gfx->getFramebuffer();

  ledcAttach(TFT_BL, 5000, 8);
  touch_dev.begin();

  // LVGL
  lv_init();
  lv_tick_set_cb([]() -> uint32_t { return millis(); });
#if DECK_RENDER_PARTIAL
  uint32_t bufSize = LV_HOR * DECK_STRIP_LINES * sizeof(lv_color_t);
  lv_color_t *buf = (lv_color_t *)heap_caps_aligned_alloc(16, bufSize, MALLOC_CAP_INTERNAL | MALLOC_CAP_8BIT);
  if (!buf) { Serial.println("FATAL RAM"); while (1) delay(1000); }
  lv_display_t *disp = lv_display_create(LV_HOR, LV_VER);
  lv_display_set_flush_cb(disp, disp_flush_cb);
  lv_display_set_buffers(disp, buf, NULL, bufSize, LV_DISPLAY_RENDER_MODE_PARTIAL);
#else
  uint32_t bufSize = LV_HOR * LV_VER * sizeof(lv_color_t);
  lv_color_t *buf = (lv_color_t *)heap_caps_malloc(bufSize, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
  if (!buf) { Serial.println("FATAL PSRAM"); while (1) delay(1000); }
  lv_display_t *disp = lv_display_create(LV_HOR, LV_VER);
  lv_display_set_flush_cb(disp, disp_flush_cb);
  lv_display_set_buffers(disp, buf, NULL, bufSize, LV_DISPLAY_RENDER_MODE_FULL);
#endif
  lv_indev_t *indev = lv_indev_create();
  lv_indev_set_type(indev, LV_INDEV_TYPE_POINTER);
  lv_indev_set_read_cb(indev, touch_read_cb);
  lv_indev_set_long_press_time(indev, LONG_PRESS_MS);

  load_persisted();
  apply_brightness();

  Serial.printf("[BLE] heap antes: %u\n", (unsigned)ESP.getFreeHeap());
  BleLink::begin();
  Serial.printf("[BLE] heap depois: %u  mac=%s  nome=%s  %s\n",
                (unsigned)ESP.getFreeHeap(), BleLink::mac(), DECK_NAME,
                BleLink::lastError()[0] ? BleLink::lastError() : "advertising");

  request_state(ST_SEARCH);
}

void loop() {
  // estatistica do laco (DECK/STATS a cada 10 s): media de ms por iteracao
  static uint32_t statT0 = 0, statN = 0, statAccum = 0;
  uint32_t itStart = millis();
  lv_task_handler();
  BleLink::tick();
  {
    uint32_t now0 = millis();
    statAccum += now0 - itStart; statN++;
    if (now0 - statT0 >= 10000) {
      if (statT0 && statN) {
        uint32_t avg = statAccum / statN;
        BleLink::sendEvent(EV_DECK, DECK_STATS, (uint8_t)(avg > 255 ? 255 : avg));
      }
      statT0 = now0; statN = 0; statAccum = 0;
    }
  }

  // eventos do link
  if (BleLink::takeDisconnected()) {
    g_model.valid = false;
    g_usage.valid = false;
    g_agentStale = false;
    if (g_state != ST_SETTINGS && g_state != ST_ABOUT) request_state(ST_SEARCH);
  }
  if (BleLink::takeConnected()) search_status_text();
  if (BleLink::takeBonded()) { BleLink::sendEvent(EV_DECK, DECK_BONDED, 0); g_bondedToast = true; }
  BleMsg m;
  while (BleLink::nextMessage(m)) handle_msg(m);

  // confirmacao pendente (executada aqui, fora dos callbacks)
  if (g_cf.req) {
    int req = g_cf.req; uint32_t p = g_cf.payload;
    confirm_close();
    if (req == 1) do_payload(p);
  }

  if (g_dirty) { g_dirty = false; render_state(); }

  // passkey overlay acompanha o estado do pareamento
  if (BleLink::isPairing() && !g_pkScrim) passkey_show();
  else if (!BleLink::isPairing() && g_pkScrim) passkey_hide();

  uint32_t now = millis();
  static uint32_t last1s = 0, lastAnim = 0, blinkAt = 0, frameAt = 0;
  static bool blinkClosed = false;

  if (g_state == ST_SEARCH) {
    if (now - last1s > 1000) { last1s = now; search_status_text(); search_foot_text(); }
    if (now - lastAnim > 80 && g_search.cont) {
      lastAnim = now;
      float ph = now / 600.0f;
      lv_obj_set_y(g_search.cont, g_search.baseY + (int)(3.0f * sinf(ph)));
    }
    if (now - frameAt > 600 && g_search.img) {            // pernas: frame A <-> B
      frameAt = now; g_search.frameB = !g_search.frameB;
      lv_image_set_src(g_search.img, g_search.frameB ? &ic_clow_b_4 : &ic_clow_a_4);
    }
    uint32_t bp = blinkClosed ? 150 : 3200;
    if (now - blinkAt > bp) {
      blinkAt = now; blinkClosed = !blinkClosed;
      for (int k = 0; k < 2; k++) {
        if (!g_search.lid[k]) continue;
        if (blinkClosed) lv_obj_clear_flag(g_search.lid[k], LV_OBJ_FLAG_HIDDEN);
        else             lv_obj_add_flag(g_search.lid[k], LV_OBJ_FLAG_HIDDEN);
      }
    }
  } else if (g_state == ST_GRID) {
    if (now - lastAnim > ANIM_TICK_MS) { lastAnim = now; grid_anim(now); }
    if (now - last1s > 1000) { last1s = now; grid_tick_1s(); }
  } else if (g_state == ST_SESSION) {
    if (now - lastAnim > ANIM_TICK_MS) { lastAnim = now; session_anim(now); }
    if (now - last1s > 1000) { last1s = now; session_apply(); }
    if (!g_voice && g_voiceHintUntil && now > g_voiceHintUntil) { g_voiceHintUntil = 0; voice_idle_text(); }
  }

  // agente parado: conectado + ja teve SESSIONS + 10 s em silencio
  bool stale = BleLink::isConnected() && g_model.valid && (now - g_model.rxMs > SESSIONS_TIMEOUT_MS);
  if (stale != g_agentStale) { g_agentStale = stale; if (g_state == ST_GRID) grid_tick_1s(); }

  delay(5);
}
