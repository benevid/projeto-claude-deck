#ifndef CONFIG_H
#define CONFIG_H

// ============================================================
// Clow Deck — Guition JC4832W535 (ESP32-S3, AXS15231B QSPI + touch)
// Deck fisico do Claude Code: periferico BLE "burro" dirigido pelo agente
// (agent/). Pinos herdados do bring-up validado (REFERENCIA-HARDWARE-LVGL.md).
// ============================================================

// ── Firmware / protocolo ─────────────────────────────────
#define FW_VERSION        "0.3"
#define FW_MAJOR          0
#define FW_MINOR          3
#define PROTO_VERSION     1              // protocol/PROTOCOL.md
#define DECK_NAME         "Clow Deck"
// Autoria exibida na tela SOBRE (Ajustes > sobre). As fontes Montserrat
// embutidas so tem ASCII: nada de acentos aqui.
#define DEV_NAME          "Benevid Felix Silva"
#define DEV_EMAIL         "benevid@gmail.com"
#define DEV_URL           "github.com/benevid/projeto-claude-deck"

// 1 = exige link cifrado+autenticado (bonding com passkey na tela) nas
// characteristics de dados. 0 = so para debug de bancada.
#define DECK_BLE_SECURE   1

// ── Orientacao da tela ───────────────────────────────────
// O painel e nativamente 320x480 (retrato). O LVGL desenha em coordenadas
// logicas e o flush copia para o canvas nativo conforme a orientacao:
//   0 = retrato nativo (copia direta; USB em cima)
//   2 = retrato invertido 180 graus (USB embaixo)  <- padrao do deck
//   3 = paisagem 270 graus CW (USB a esquerda; pipeline do Usage Stick, so referencia —
//       a UI e desenhada para retrato)
// Display e touch SEMPRE na mesma orientacao (TOUCH_ROTATION deriva daqui).
#define DECK_ORIENTATION  2
#if DECK_ORIENTATION == 3
  #define LV_HOR 480
  #define LV_VER 320
#else
  #define LV_HOR 320
  #define LV_VER 480
#endif

// ── Geometria da grade (INFO §4.1; design/THEME.md §4) ───
// Retrato: 3 colunas x 4 linhas. Linhas 0-1 = 6 celulas de sessao; linha 2 = 3
// celulas utilitarias/botoes; linha 3 = faixa livre (sem divisorias no case 3D).
#define DECK_COLS          3
#define DECK_ROWS          4
#define DECK_SESSION_CELLS 6
#define DECK_CELLS         12
#define DECK_PROTO_SESSIONS 8            // entradas no payload SESSIONS (protocolo inalterado)
#define DECK_LABEL_LEN     12
#define DECK_CUSTOM_MAX    16

// ── UUIDs (PROTOCOL.md §2) ───────────────────────────────
#define UUID_SERVICE   "c1a0de00-0dec-4a11-8000-c10dec000001"
#define UUID_INFO      "c1a0de01-0dec-4a11-8000-c10dec000001"
#define UUID_SESSIONS  "c1a0de02-0dec-4a11-8000-c10dec000001"
#define UUID_EVENT     "c1a0de03-0dec-4a11-8000-c10dec000001"
#define UUID_USAGE     "c1a0de04-0dec-4a11-8000-c10dec000001"
#define UUID_CONFIG    "c1a0de05-0dec-4a11-8000-c10dec000001"

// ── Temporizacoes ────────────────────────────────────────
#define SESSIONS_TIMEOUT_MS  10000       // sem SESSIONS por 10 s = "agente parado"
#define LONG_PRESS_MS        400
// Render LVGL: 1 = PARTIAL (faixa de 40 linhas em RAM interna; so o que mudou e
// redesenhado; QSPI uma vez por frame). 0 = FULL (tela inteira na PSRAM, como o Usage Stick).
#define DECK_RENDER_PARTIAL  1
#define DECK_STRIP_LINES     40
#define ANIM_TICK_MS         100

// ── Display QSPI (AXS15231B) ─────────────────────────────
#define TFT_CS    45
#define TFT_SCK   47
#define TFT_SDA0  21
#define TFT_SDA1  48
#define TFT_SDA2  40
#define TFT_SDA3  39
#define TFT_BL    1
#define TFT_TE    38

#define SCREEN_WIDTH   LV_HOR
#define SCREEN_HEIGHT  LV_VER
#define QSPI_FREQ      40000000UL

// ── Touch I2C (AXS15231B) ────────────────────────────────
#define TOUCH_SDA  4
#define TOUCH_SCL  8
#define TOUCH_INT  3
#define TOUCH_ADDR 0x3B
// casa com DECK_ORIENTATION (0 nativo, 2 invertido, 3 paisagem) — nunca mude um sem o outro
#define TOUCH_ROTATION DECK_ORIENTATION

// ── NVS ──────────────────────────────────────────────────
#define NVS_NAMESPACE   "clowdeck"
#define BRI_DEFAULT     200
#define BRI_MIN         10

#endif // CONFIG_H
