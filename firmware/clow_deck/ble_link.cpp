// BleLink — ver ble_link.h. NimBLE-Arduino 2.5.x / core esp32 3.3.x.
#include "ble_link.h"
#include "deck_types.h"
#include <NimBLEDevice.h>
#include <esp_random.h>

// ---------- buffers compartilhados (task do host -> loop) ----------
#define RAW_SLOTS 8
#define RAW_MAX   512                       // NimBLE entrega long writes ja montados (ate 512)
#define MSG_MAX   768                       // §3: ate 15 frames; o deck aceita ate 768 B

struct RawFrame { uint8_t chr; uint16_t len; uint8_t data[RAW_MAX]; };
static RawFrame s_raw[RAW_SLOTS];
static volatile uint8_t s_rawHead = 0, s_rawTail = 0;
static portMUX_TYPE s_mux = portMUX_INITIALIZER_UNLOCKED;

struct Reasm { uint8_t total, next; uint16_t len; bool ready; uint8_t buf[MSG_MAX]; };
static Reasm s_re[6];                       // indexado por CHR_* (0..5)

// ---------- estado ----------
static NimBLEServer         *s_server = nullptr;
static NimBLECharacteristic *s_chrInfo = nullptr, *s_chrSessions = nullptr, *s_chrEvent = nullptr,
                            *s_chrUsage = nullptr, *s_chrConfig = nullptr;
static volatile bool s_connected = false, s_subscribed = false, s_pairing = false, s_auth = false;
static volatile bool s_fConnected = false, s_fDisconnected = false, s_fHello = false, s_fBonded = false;
// motivo da ultima desconexao, compactado p/ 1 byte: codigo HCI (reason >= 0x200)
// ou 0x80|erro do host NimBLE. Vai no arg do DECK/HELLO (PROTOCOL §4.3) e no rodape.
static volatile uint8_t s_lastDc = 0;
static volatile uint16_t s_mtu = 23;
static volatile uint16_t s_connHandle = BLE_HS_CONN_HANDLE_NONE;
static uint32_t s_passkey = 0;
static char     s_mac[20] = "--:--:--:--:--:--";
static char     s_err[48] = "";
static uint32_t s_heapAfter = 0;
static uint32_t s_advRetryMs = 0;

// ---------- callbacks (task do host: nada de LVGL, nada pesado) ----------
static void push_raw(uint8_t chr, const uint8_t *d, size_t n) {
  if (n == 0 || n > RAW_MAX) return;
  portENTER_CRITICAL(&s_mux);
  uint8_t next = (s_rawHead + 1) % RAW_SLOTS;
  if (next != s_rawTail) {                  // cheio: descarta (o proximo heartbeat refaz)
    RawFrame &f = s_raw[s_rawHead];
    f.chr = chr; f.len = (uint16_t)n; memcpy(f.data, d, n);
    s_rawHead = next;
  }
  portEXIT_CRITICAL(&s_mux);
}

class ServerCB : public NimBLEServerCallbacks {
  void onConnect(NimBLEServer *srv, NimBLEConnInfo &ci) override {
    s_connected = true; s_auth = false; s_subscribed = false;
    s_connHandle = ci.getConnHandle();
    s_fConnected = true;
  }
  void onDisconnect(NimBLEServer *srv, NimBLEConnInfo &ci, int reason) override {
    s_connected = false; s_subscribed = false; s_pairing = false; s_auth = false;
    s_connHandle = BLE_HS_CONN_HANDLE_NONE;
    s_lastDc = (reason >= 0x200) ? (uint8_t)(reason & 0xFF) : (uint8_t)(0x80 | (reason & 0x7F));
    s_fDisconnected = true;
    // advertiseOnDisconnect (default true) re-anuncia; tick() garante o resto
  }
  void onMTUChange(uint16_t m, NimBLEConnInfo &ci) override { s_mtu = m; }
  uint32_t onPassKeyDisplay() override { s_pairing = true; return s_passkey; }
  void onAuthenticationComplete(NimBLEConnInfo &ci) override {
    s_pairing = false;
    if (!ci.isEncrypted()) {                // pareamento falhou/cancelado: derruba
      if (s_server) s_server->disconnect(ci);
      return;
    }
    s_auth = ci.isAuthenticated() || ci.isEncrypted();
    s_fBonded = true;
  }
};

class DataCB : public NimBLECharacteristicCallbacks {
  void onWrite(NimBLECharacteristic *c, NimBLEConnInfo &ci) override {
    uint8_t chr = (c == s_chrSessions) ? CHR_SESSIONS : (c == s_chrUsage) ? CHR_USAGE
                : (c == s_chrConfig) ? CHR_CONFIG : 0;
    if (!chr) return;
    NimBLEAttValue v = c->getValue();
    push_raw(chr, v.data(), v.size());
  }
};

class EventCB : public NimBLECharacteristicCallbacks {
  void onSubscribe(NimBLECharacteristic *c, NimBLEConnInfo &ci, uint16_t subValue) override {
    bool on = (subValue & 0x0001) != 0;
    s_subscribed = on;
    if (on) s_fHello = true;                // tick() manda DECK/HELLO do loop
  }
};

// ---------- remontagem (§3) ----------
static void reasm_feed(const RawFrame &f) {
  if (f.chr > 5 || f.len < 2) return;
  Reasm &r = s_re[f.chr];
  uint8_t hdr = f.data[0], total = hdr >> 4, idx = hdr & 0x0F;
  if (total == 0 || idx >= total) return;
  if (idx == 0) { r.total = total; r.next = 0; r.len = 0; r.ready = false; }
  else if (total != r.total || idx != r.next) { r.next = 0; r.len = 0; return; }
  size_t n = f.len - 1;
  if (r.len + n > MSG_MAX) n = MSG_MAX - r.len;
  memcpy(r.buf + r.len, f.data + 1, n);
  r.len += n; r.next++;
  if (r.next == r.total) r.ready = true;
}

static void start_adv() {
  NimBLEAdvertising *adv = NimBLEDevice::getAdvertising();
  if (adv->isAdvertising()) return;
  adv->start();
}

// ---------- API ----------
void BleLink::begin() {
  s_passkey = 100000 + (esp_random() % 900000);
  if (!NimBLEDevice::init(DECK_NAME)) { strlcpy(s_err, "NimBLE init falhou", sizeof(s_err)); return; }
  NimBLEDevice::setMTU(247);
  NimBLEDevice::setPower(9);                                  // dBm
#if DECK_BLE_SECURE
  NimBLEDevice::setSecurityAuth(true, true, true);          // bond + MITM + SC
  NimBLEDevice::setSecurityIOCap(BLE_HS_IO_DISPLAY_ONLY);   // passkey na tela
  // NAO chamar setSecurityPasskey(): o NimBLEServer so invoca onPassKeyDisplay() (que
  // liga o overlay na tela) quando o passkey estatico e o padrao 123456 — com um valor
  // fixo proprio o callback nunca roda e o usuario ve o dialogo do Mac sem codigo na
  // tela (visto na bancada). O callback devolve s_passkey (aleatorio por boot).
#endif
  strlcpy(s_mac, NimBLEDevice::getAddress().toString().c_str(), sizeof(s_mac));

  s_server = NimBLEDevice::createServer();
  s_server->setCallbacks(new ServerCB(), true);
  s_server->advertiseOnDisconnect(true);

  NimBLEService *svc = s_server->createService(UUID_SERVICE);
#if DECK_BLE_SECURE
  const uint32_t W = NIMBLE_PROPERTY::WRITE | NIMBLE_PROPERTY::WRITE_NR |
                     NIMBLE_PROPERTY::WRITE_ENC | NIMBLE_PROPERTY::WRITE_AUTHEN;
  // EVENT fica SEM READ_ENC/READ_AUTHEN de proposito: com essas flags o NimBLEServer
  // chama startSecurity() no subscribe se o link ainda nao esta cifrado; isso cria um
  // proc SM "Security Request" com timer de 30 s que o macOS (ja pareado, cifra por
  // conta propria) nunca responde -> BLE_HS_ETIMEOUT -> queda 0x16 aos ~30 s da conexao
  // (visto na bancada). O link continua protegido pelas escritas *_AUTHEN; o EVENT so
  // carrega indices de celula.
  const uint32_t N = NIMBLE_PROPERTY::NOTIFY | NIMBLE_PROPERTY::READ;
#else
  const uint32_t W = NIMBLE_PROPERTY::WRITE | NIMBLE_PROPERTY::WRITE_NR;
  const uint32_t N = NIMBLE_PROPERTY::NOTIFY | NIMBLE_PROPERTY::READ;
#endif
  s_chrInfo     = svc->createCharacteristic(UUID_INFO, NIMBLE_PROPERTY::READ, 8);
  s_chrSessions = svc->createCharacteristic(UUID_SESSIONS, W, RAW_MAX);
  s_chrEvent    = svc->createCharacteristic(UUID_EVENT, N, 8);
  s_chrUsage    = svc->createCharacteristic(UUID_USAGE, W, RAW_MAX);
  s_chrConfig   = svc->createCharacteristic(UUID_CONFIG, W, RAW_MAX);

  // INFO (§4.1)
  uint8_t info[8] = { PROTO_VERSION, FW_MAJOR, FW_MINOR, DECK_COLS, DECK_ROWS,
                      DECK_SESSION_CELLS, DECK_LABEL_LEN,
                      (uint8_t)((DECK_BLE_SECURE ? 1 : 0) | 2 | 4) };
  s_chrInfo->setValue(info, sizeof(info));
  uint8_t ev0[5] = { 0x10, PROTO_VERSION, 0, 0, 0 };
  s_chrEvent->setValue(ev0, sizeof(ev0));

  DataCB *dcb = new DataCB();
  s_chrSessions->setCallbacks(dcb);
  s_chrUsage->setCallbacks(dcb);
  s_chrConfig->setCallbacks(dcb);
  s_chrEvent->setCallbacks(new EventCB());
  svc->start();

  // Advertising: flags + UUID 128 bit no pacote principal (21 B); nome vai no
  // scan response porque nao cabe junto (31 B).
  NimBLEAdvertising *adv = NimBLEDevice::getAdvertising();
  NimBLEAdvertisementData ad;
  ad.setFlags(BLE_HS_ADV_F_DISC_GEN | BLE_HS_ADV_F_BREDR_UNSUP);
  ad.addServiceUUID(NimBLEUUID(UUID_SERVICE));
  NimBLEAdvertisementData sr;
  sr.setName(DECK_NAME);
  adv->enableScanResponse(true);
  adv->setAdvertisementData(ad);
  adv->setScanResponseData(sr);
  adv->setMinInterval(160);   // 100 ms
  adv->setMaxInterval(320);   // 200 ms
  if (!adv->start()) strlcpy(s_err, "advertising falhou", sizeof(s_err));
  s_heapAfter = ESP.getFreeHeap();
}

void BleLink::tick() {
  // frames crus -> remontagem (so aqui, na task do loop)
  for (;;) {
    RawFrame f;
    bool got = false;
    portENTER_CRITICAL(&s_mux);
    if (s_rawTail != s_rawHead) {
      f.chr = s_raw[s_rawTail].chr; f.len = s_raw[s_rawTail].len;
      memcpy(f.data, s_raw[s_rawTail].data, f.len);
      s_rawTail = (s_rawTail + 1) % RAW_SLOTS;
      got = true;
    }
    portEXIT_CRITICAL(&s_mux);
    if (!got) break;
    reasm_feed(f);
  }
  // HELLO logo que o agente assina EVENT (§5)
  if (s_fHello) { s_fHello = false; sendEvent(EV_DECK, DECK_HELLO, s_lastDc); }
  // DECK/BONDED e enviado pelo sketch (takeBonded), que tambem mostra o status
  // garante advertising quando ocioso
  uint32_t now = millis();
  if (!s_connected && now - s_advRetryMs > 1000) { s_advRetryMs = now; start_adv(); }
}

bool BleLink::isConnected()     { return s_connected; }
bool BleLink::isSubscribed()    { return s_subscribed; }
bool BleLink::isPairing()       { return s_pairing; }
bool BleLink::isAuthenticated() { return s_auth; }
uint32_t BleLink::passkey()     { return s_passkey; }
uint16_t BleLink::mtu()         { return s_mtu; }
uint8_t  BleLink::lastDisconnect() { return s_lastDc; }
int  BleLink::numBonds()        { return NimBLEDevice::getNumBonds(); }
const char *BleLink::mac()      { return s_mac; }
const char *BleLink::lastError(){ return s_err; }
uint32_t BleLink::heapAfterInit(){ return s_heapAfter; }

static bool take(volatile bool &f) { if (!f) return false; f = false; return true; }
bool BleLink::takeConnected()    { return take(s_fConnected); }
bool BleLink::takeDisconnected() { return take(s_fDisconnected); }
bool BleLink::takeBonded()       { return take(s_fBonded); }

bool BleLink::nextMessage(BleMsg &m) {
  static const uint8_t order[3] = { CHR_SESSIONS, CHR_USAGE, CHR_CONFIG };
  for (int i = 0; i < 3; i++) {
    Reasm &r = s_re[order[i]];
    if (!r.ready) continue;
    r.ready = false;
    m.chr = order[i]; m.len = r.len; m.data = r.buf;
    return true;
  }
  return false;
}

bool BleLink::sendEvent(uint8_t kind, uint8_t cell, uint8_t arg) {
  if (!s_chrEvent) return false;
  uint8_t buf[5] = { 0x10, PROTO_VERSION, kind, cell, arg };
  s_chrEvent->setValue(buf, sizeof(buf));
  // So exige conexao: o estado "assinado" restaurado do bond pelo NimBLE nem sempre
  // passa pelo onSubscribe (central pareada reconectando), e um notify() sem
  // assinante e inofensivo (retorna false).
  if (!s_connected) return false;
  return s_chrEvent->notify(buf, sizeof(buf));
}

void BleLink::forgetBonds() {
  NimBLEDevice::deleteAllBonds();
  if (s_connected && s_server && s_connHandle != BLE_HS_CONN_HANDLE_NONE)
    s_server->disconnect(s_connHandle);
  s_advRetryMs = 0;
}
