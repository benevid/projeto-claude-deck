#ifndef DECK_TYPES_H
#define DECK_TYPES_H
// ============================================================
// Tipos do protocolo (PROTOCOL.md) usados em assinaturas de funcao do .ino.
// Ficam num header porque o arduino-cli gera os prototipos das funcoes do
// .ino ANTES das definicoes de tipo do proprio .ino ("does not name a type").
// ============================================================
#include <stdint.h>
#include "config.h"

// §4.2 — estado de cada celula de sessao
enum SessState : uint8_t {
  SS_EMPTY = 0, SS_UNKNOWN, SS_WORKING, SS_ATTENTION, SS_DONE, SS_IDLE, SS_ERROR, SS_DEAD
};
// §4.2 — permission_mode
enum SessMode : uint8_t {
  SM_UNKNOWN = 0, SM_DEFAULT, SM_ACCEPT_EDITS, SM_PLAN, SM_BYPASS, SM_DONT_ASK
};

struct SessEntry {
  uint8_t  sid;                       // 0 = celula vazia
  uint8_t  state;                     // SessState
  uint8_t  mode;                      // SessMode
  uint8_t  flags;                     // bit0 ativa, bit1 sem hooks, bit2 engine Codex
  uint16_t age;                       // segundos no estado (no momento do envio)
  char     label[DECK_LABEL_LEN + 1]; // ASCII, terminado
};

struct DeckModel {
  bool      valid;                    // ja recebeu ao menos um SESSIONS nesta conexao
  uint8_t   flags;                    // bit0 agente pronto, bit1 voz, bit2 usage
  uint8_t   active;                   // 0..7 ou 0xFF
  SessEntry s[DECK_PROTO_SESSIONS];   // 8 no protocolo; o deck mostra as 6 primeiras
  uint32_t  rxMs;                     // millis() do ultimo SESSIONS
};

struct UsageModel {
  bool     valid;
  uint8_t  p5, p7;                    // 255 = n/d
  uint32_t reset5, reset7, hostNow;   // epoch
  uint32_t rxMs;
};

struct CustomCmd {
  char label[DECK_LABEL_LEN + 1];
  bool confirm;                       // rotulo veio com '!' na frente
};

// §4.3 — EVENT.kind
#define EV_CELL_TAP     1
#define EV_CELL_HOLD    2
#define EV_CELL_RELEASE 3
#define EV_ACTION       4
#define EV_DECK         5

// §4.3 — ACTION.arg
#define ACT_FOCUS        0x01
#define ACT_MODE_CYCLE   0x10
#define ACT_COMPACT      0x11
#define ACT_CLEAR        0x12
#define ACT_ESC          0x13
#define ACT_ENTER        0x14
#define ACT_ACK          0x15
#define ACT_TAB          0x16
#define ACT_APPROVE      0x17
#define ACT_INIT         0x18
#define ACT_VOICE_START  0x20
#define ACT_VOICE_STOP   0x21
#define ACT_VOICE_CANCEL 0x22
#define ACT_CUSTOM_BASE  0x30

// §4.3 — DECK.cell
#define DECK_HELLO  1
#define DECK_BONDED 2
#define DECK_PAGE   3
#define DECK_STATS       4   // arg = media ms do laco (PROTOCOL §4.3)

#define CELL_ACTIVE 0xFF

// Estados de tela (aqui por causa dos prototipos automaticos do .ino)
enum State { ST_BOOT, ST_SEARCH, ST_GRID, ST_SESSION, ST_CMD, ST_SETTINGS, ST_ABOUT };

// Paginas reportadas em DECK/PAGE
#define PAGE_GRID     0
#define PAGE_SESSION  1
#define PAGE_CMD      2
#define PAGE_SETTINGS 3

#endif // DECK_TYPES_H
