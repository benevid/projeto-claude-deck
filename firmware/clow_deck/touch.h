#ifndef TOUCH_H
#define TOUCH_H

#include <Arduino.h>
#include <Wire.h>

// Driver de toque AXS15231B (versão validada do bring-up/projeto anterior).
#define AXS_GET_POINT_X(buf) (((uint16_t)(buf[2] & 0x0F) << 8) + (uint16_t)buf[3])
#define AXS_GET_POINT_Y(buf) (((uint16_t)(buf[4] & 0x0F) << 8) + (uint16_t)buf[5])

class AXS15231B_Touch {
public:
    AXS15231B_Touch(uint8_t scl, uint8_t sda, uint8_t int_pin, uint8_t addr, uint8_t rotation)
        : _scl(scl), _sda(sda), _int_pin(int_pin), _addr(addr), _rotation(rotation) {}

    bool begin() {
        _instance = this;
        pinMode(_int_pin, INPUT_PULLUP);
        attachInterrupt(digitalPinToInterrupt(_int_pin), _isr, FALLING);
        return Wire.begin(_sda, _scl, 400000);
    }

    bool touched() { return _update(); }

    void readData(uint16_t *x, uint16_t *y) {
        *x = _point_x;
        *y = _point_y;
    }

    void setRotation(uint8_t r) { _rotation = r; }

private:
    uint8_t _scl, _sda, _int_pin, _addr, _rotation;
    volatile bool _touch_int = false;
    bool _pressed = false;            // ultimo estado lido do controlador
    uint32_t _pressedSinceMs = 0;
    uint16_t _point_x = 0, _point_y = 0;

    static AXS15231B_Touch *_instance;

    static void ARDUINO_ISR_ATTR _isr() {
        if (_instance) _instance->_touch_int = true;
    }

    // Le o controlador quando: houve borda de INT, OU o INT esta em nivel baixo
    // (dedo na tela), OU a ultima leitura dizia "pressionado". Sem isso um dedo
    // PARADO (sem novos relatorios → sem bordas de INT) virava "soltou" no LVGL e o
    // LONG_PRESSED de 400 ms nunca fechava — era preciso tocar varias vezes.
    // "Pressionado" = numero de pontos (buf[1]) > 0 e coordenada != (0,0).
    bool _update() {
        bool edge = _touch_int;
        if (!edge && !_pressed && digitalRead(_int_pin) != LOW) return false;
        _touch_int = false;

        static const uint8_t read_cmd[8] = {0xB5, 0xAB, 0xA5, 0x5A, 0x00, 0x00, 0x00, 0x08};
        uint8_t buf[8] = {0};

        Wire.beginTransmission(_addr);
        Wire.write(read_cmd, sizeof(read_cmd));
        Wire.endTransmission();

        Wire.requestFrom(_addr, (uint8_t)sizeof(buf));
        for (int i = 0; i < (int)sizeof(buf) && Wire.available(); i++)
            buf[i] = Wire.read();

        uint8_t  npts  = buf[1];
        uint16_t raw_x = AXS_GET_POINT_X(buf);
        uint16_t raw_y = AXS_GET_POINT_Y(buf);

        bool down = (npts > 0 && npts < 6) && !(raw_x == 0 && raw_y == 0);
        // rede de seguranca: 15 s "pressionado" sem nenhuma borda de INT = solta
        if (down && _pressed && !edge && millis() - _pressedSinceMs > 15000) down = false;
        if (down && !_pressed) _pressedSinceMs = millis();
        _pressed = down;
        if (!down) return false;

        switch (_rotation) {
            case 0: _point_x = raw_x; _point_y = raw_y; break;
            case 1: _point_x = raw_y; _point_y = 319 - raw_x; break;
            case 2: _point_x = 319 - raw_x; _point_y = 479 - raw_y; break;
            case 3: _point_x = 479 - raw_y; _point_y = raw_x; break;
        }
        return true;
    }
};

// `inline` obrigatorio: isto e uma DEFINICAO em escopo de arquivo dentro de um
// header. Enquanto o projeto era um .ino unico, so havia uma unidade de traducao
// e passava despercebido. Com o codigo dividido em varios .cpp, o segundo include
// gera "multiple definition of AXS15231B_Touch::_instance" — erro de LINKAGEM,
// que aparece no fim do build e aponta para o header, nao para quem incluiu.
inline AXS15231B_Touch *AXS15231B_Touch::_instance = nullptr;

#endif // TOUCH_H
