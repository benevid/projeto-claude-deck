#ifndef BLE_LINK_H
#define BLE_LINK_H
// ============================================================
// BleLink — GATT server do Clow Deck (NimBLE-Arduino 2.x), PROTOCOL.md §1-§3.
//
// Modelo de threads: os callbacks do NimBLE rodam na task do host. Eles SO
// copiam bytes para um ring de frames crus (sob spinlock) e setam flags.
// Tudo o mais — remontagem (§3), entrega de mensagens, notify de EVENT,
// re-advertising — acontece em tick()/sendEvent(), chamados do loop().
// Este modulo nao sabe nada de LVGL nem do significado dos payloads.
// ============================================================
#include <Arduino.h>
#include "config.h"

// ids de characteristic (XX do UUID) para mensagens remontadas
#define CHR_SESSIONS 2
#define CHR_USAGE    4
#define CHR_CONFIG   5

struct BleMsg {
  uint8_t        chr;      // CHR_*
  uint16_t       len;      // bytes de payload (sem o header de framing)
  const uint8_t *data;     // valido ate o proximo tick()
};

namespace BleLink {
  void     begin();                       // init NimBLE + servico + advertising
  void     tick();                        // do loop(): remonta frames, garante advertising

  bool     isConnected();
  bool     isSubscribed();                // central assinou EVENT
  bool     isPairing();                   // passkey deve estar na tela
  bool     isAuthenticated();             // link cifrado+autenticado
  uint32_t passkey();
  uint16_t mtu();
  uint8_t  lastDisconnect();              // motivo da ultima queda (HCI ou 0x80|host), 0 = nenhuma
  int      numBonds();
  const char *mac();                      // "AA:BB:CC:DD:EE:FF"
  const char *lastError();                // ultimo erro de init (ou "")
  uint32_t heapAfterInit();

  // flags "take": true uma unica vez por ocorrencia
  bool     takeConnected();
  bool     takeDisconnected();
  bool     takeBonded();

  // proxima mensagem completa (SESSIONS/USAGE/CONFIG); false se nao ha
  bool     nextMessage(BleMsg &m);

  // deck -> agente (§4.3). Retorna false se ninguem assinou EVENT.
  bool     sendEvent(uint8_t kind, uint8_t cell, uint8_t arg);

  void     forgetBonds();                 // apaga bonds + derruba conexao + re-anuncia
}

#endif // BLE_LINK_H
