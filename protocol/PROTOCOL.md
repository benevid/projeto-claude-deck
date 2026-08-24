# Clow Deck — Protocolo BLE (v1)

> **Fonte da verdade** para firmware (`firmware/clow_deck/`) e agente (`agent/`).
> Qualquer mudanca de byte aqui exige bump de `PROTO_VERSION` nos dois lados.

## 0. Papeis

| Peca | Papel GATT | Faz |
|---|---|---|
| Deck (ESP32-S3) | **Peripheral / GATT server** | anuncia, desenha o que recebe, notifica toques |
| Agente (Mac/Win) | **Central / GATT client** | escaneia, conecta, escreve estado, assina eventos |

Consequencia (corrige a tabela do plano): **agente → deck = WRITE** numa characteristic;
**deck → agente = NOTIFY**. O deck nunca "escreve" no agente — ele notifica.

Zero segredo no deck. O deck nao sabe o que e uma sessao do Claude: recebe celulas
(rotulo + estado + modo) e devolve indices de celula + gesto.

## 1. Anuncio e identificacao

- Nome: `Clow Deck`
- Advertising: flags + **UUID do servico** (128 bit, completo) + nome. O agente filtra pelo UUID.
- MTU: o deck pede 247. O agente nunca assume mais que **MTU-3 = 182 bytes** por frame
  (valor que o macOS costuma negociar) — por isso o framing da §3 e obrigatorio.
- Conexao: 1 central por vez. Enquanto conectado o deck **para de anunciar**; ao
  desconectar volta a anunciar em ate 1 s.

## 2. Servico GATT

Base: `c1a0deXX-0dec-4a11-8000-c10dec000001` (XX = id da characteristic).

| XX | Nome | Props | Direcao | Conteudo |
|---|---|---|---|---|
| `00` | **Service** | — | — | — |
| `01` | `INFO` | READ | deck → agente | versao do protocolo, fw, geometria (§4.1) — **sem framing** |
| `02` | `SESSIONS` | WRITE, WRITE_NR | agente → deck | estado das 8 celulas de sessao (§4.2) |
| `03` | `EVENT` | NOTIFY, READ (**sem** exigir cifra) | deck → agente | toque/gesto/acao (§4.3) |
| `04` | `USAGE` | WRITE, WRITE_NR | agente → deck | janela 5h/7d (§4.4) |
| `05` | `CONFIG` | WRITE, WRITE_NR | agente → deck | brilho, idioma, comandos (§4.5) |

`WRITE_NR` = write without response (o agente usa p/ frames <= MTU-3; para
payloads maiores usa WRITE com resposta — o SO faz o *long write*).

### 2.1 Seguranca

- Bonding + MITM + Secure Connections. IO capability do deck: **DisplayOnly**
  (passkey de 6 digitos mostrado na tela; o usuario digita no computador).
- As characteristics de escrita (`02`, `04`, `05`) exigem link **criptografado e
  autenticado** (`WRITE_ENC|WRITE_AUTHEN`). `INFO` e aberta (serve p/ o agente validar a
  versao antes de parear). `EVENT` e **deliberadamente aberta**: com `READ_ENC/AUTHEN`
  o NimBLEServer manda um *Slave Security Request* quando o central assina antes de
  cifrar; o macOS ignora esse pedido, o procedimento SM expira em 30 s e a conexao cai
  (`0x16`). O EVENT so carrega indices de celula — o link continua protegido porque
  nada entra no deck sem escrita autenticada.
- **A primeira operacao autenticada do agente e um WRITE COM RESPOSTA** (`CONFIG`).
  So um erro ATT "insufficient authentication" faz o CoreBluetooth parear/cifrar; um
  write-without-response num link nao cifrado e descartado pelo deck **sem erro** e o
  Mac nunca fica sabendo (visto na bancada: centenas de escritas "ok" e o deck em
  "aguardando sessoes"). Por isso o agente usa write-with-response por padrao.
- O passkey e **aleatorio por boot**; o deck o mostra so enquanto ha pareamento em curso.
- "Esquecer pareamento" fica nos ajustes do deck (hold na celula ☰) → apaga todos os bonds.
- Flag de compilacao `DECK_BLE_SECURE` (config.h): `0` desliga a exigencia de
  autenticacao (somente p/ debug de bancada).

## 3. Framing (todas as characteristics exceto `INFO`)

Toda mensagem e enviada como 1..15 frames. Primeiro byte de cada frame:

```
bit 7..4 : total de frames (1..15)
bit 3..0 : indice deste frame (0-based)
```

- Frame unico: `0x10` + payload.
- O receptor zera o buffer de remontagem ao receber indice 0; descarta a mensagem se
  chegar indice fora de ordem ou total diferente do primeiro frame.
- Payload maximo por frame: `MTU-3-1`. Mensagem maxima: 15 × 181 ≈ 2,7 KB.
- Inteiros multi-byte: **little-endian**.

## 4. Payloads (apos remontagem)

### 4.1 `INFO` (READ, 8 bytes, sem framing)

| off | tipo | campo |
|---|---|---|
| 0 | u8 | `PROTO_VERSION` = **1** |
| 1 | u8 | fw major |
| 2 | u8 | fw minor |
| 3 | u8 | colunas da grade (deck vertical atual: 3) |
| 4 | u8 | linhas da grade (deck vertical atual: 4) |
| 5 | u8 | celulas de sessao `session_cells` (deck vertical atual: **6**; maximo 8) |
| 6 | u8 | tamanho do rotulo (12) |
| 7 | u8 | caps: bit0 = link autenticado exigido, bit1 = tem USAGE, bit2 = tem CONFIG |

O agente recusa deck com `PROTO_VERSION` diferente (mostra erro, nao tenta adivinhar).

### 4.2 `SESSIONS` (agente → deck)

Cabecalho 4 bytes + **8 entradas fixas** de 18 bytes = 148 bytes (cabe num frame).
Entradas sao **posicionais**: entrada `i` = celula `i` (0..7, preenchendo a grade de
sessoes linha a linha). O agente e o dono da atribuicao de celulas (uma sessao mantem a
celula ate morrer; nova sessao ocupa a primeira livre). O deck nao reordena nada.

O deck anuncia em `INFO.session_cells` quantas celulas de sessao tem (6 no deck vertical
3x4). **O agente nunca ocupa celulas >= `session_cells`** — sessoes a mais esperam na
fila dele ate vagar uma celula. O payload continua com 8 entradas (PROTO_VERSION 1); o
deck ignora as entradas que nao tem.

| off | tipo | campo |
|---|---|---|
| 0 | u8 | `PROTO_VERSION` (1) |
| 1 | u8 | flags: bit0 agente pronto (sempre 1) · bit1 voz disponivel · bit2 usage disponivel |
| 2 | u8 | numero de entradas (sempre 8 na v1) |
| 3 | u8 | celula ativa (0..7) ou `0xFF` (nenhuma) |
| 4 + 18·i | entrada | ver abaixo |

Entrada (18 bytes):

| off | tipo | campo |
|---|---|---|
| 0 | u8 | `sid` — id curto da sessao (1..255). **0 = celula vazia** |
| 1 | u8 | estado (tabela abaixo) |
| 2 | u8 | modo de permissao (tabela abaixo) |
| 3 | u8 | flags: bit0 ativa · bit1 sem hooks (so descoberta por processo) · bit2 engine Codex CLI (M6) |
| 4 | u16 | `age_s` — segundos no estado atual (satura em 65535) |
| 6 | char[12] | rotulo ASCII, zero-padded, **sem** terminador garantido (basename do cwd, transliterado) |

Estados:

| valor | nome | significado | visual no deck |
|---|---|---|---|
| 0 | `EMPTY` | celula vazia | `---` apagado |
| 1 | `UNKNOWN` | processo visto, nenhum hook ainda | neutro, sem animacao |
| 2 | `WORKING` | Claude trabalhando | neutro **pulsando** |
| 3 | `ATTENTION` | pediu permissao / pergunta / notificacao | **laranja piscando** |
| 4 | `DONE` | Claude terminou de responder, espera voce | verde (pisca lento enquanto `age_s` < 60) |
| 5 | `IDLE` | esperando prompt (reconhecido) | verde apagado, fixo |
| 6 | `ERROR` | erro reportado pelo agente | vermelho |
| 7 | `DEAD` | processo terminou (some em ~5 s) | cinza riscado |

Modos (`permission_mode` dos hooks):

| valor | nome | rotulo curto no deck |
|---|---|---|
| 0 | desconhecido | `--` |
| 1 | `default` | `ask` |
| 2 | `acceptEdits` | `edits` |
| 3 | `plan` | `plan` |
| 4 | `bypassPermissions` | `bypass` |
| 5 | `dontAsk` | `auto` |

Cadencia: o agente escreve `SESSIONS` **a cada mudanca** e como *heartbeat* a cada
**3 s**. Se o deck ficar **10 s** sem receber `SESSIONS` estando conectado, mostra
"agente parado" na celula ☰ (nao troca de tela).

### 4.3 `EVENT` (deck → agente, 4 bytes)

| off | tipo | campo |
|---|---|---|
| 0 | u8 | `PROTO_VERSION` (1) |
| 1 | u8 | `kind` |
| 2 | u8 | `cell` (0..`session_cells`-1) ou alvo; `0xFF` = sessao ativa |
| 3 | u8 | `arg` |

`kind`:

| valor | nome | `cell` | `arg` |
|---|---|---|---|
| 1 | `CELL_TAP` | celula de sessao tocada (0..`session_cells`-1) | 0 |
| 2 | `CELL_HOLD` | celula de sessao segurada (0..`session_cells`-1) | 0 |
| 3 | `CELL_RELEASE` | celula de sessao solta apos hold (0..`session_cells`-1) | 0 |
| 4 | `ACTION` | celula-alvo (0..7) ou `0xFF` = ativa | id da acao (tabela) |
| 5 | `DECK` | codigo de sistema | 0 |

Acoes (`kind = ACTION`):

| id | nome | o agente faz | exige confirmacao no deck |
|---|---|---|---|
| `0x01` | `FOCUS` | traz a janela da sessao pra frente e a marca ativa | nao |
| `0x10` | `MODE_CYCLE` | `Shift+Tab` na sessao | nao |
| `0x11` | `COMPACT` | digita `/compact` + Enter | nao |
| `0x12` | `CLEAR` | digita `/clear` + Enter | **sim** (o deck so envia depois do "Confirmar") |
| `0x13` | `ESC` | tecla `Esc` (interrompe) | nao |
| `0x14` | `ENTER` | tecla `Enter` | nao |
| `0x15` | `ACK` | marca `DONE` → `IDLE` sem focar | nao |
| `0x16` | `TAB` | tecla `Tab` (aceita a sugestao/auto-complete do terminal) | nao |
| `0x17` | `APPROVE` | aprova o pedido pendente (codex: tecla `y`; claude: tecla `1`) | nao |
| `0x20` | `VOICE_START` | foca a sessao-alvo (`cell` = 0..`session_cells`-1; `0xFF` = ativa) e **pressiona a barra de espaco** — com `/voice` ligado na sessao, o Claude Code grava enquanto o espaco estiver segurado (demora 2–3 s p/ comecar) | nao |
| `0x21` | `VOICE_STOP` | solta o espaco: o Claude transcreve e deixa o texto no prompt esperando `ENTER` (nao envia Enter sozinho) | nao |
| `0x22` | `VOICE_CANCEL` | solta o espaco + `Esc` (descarta) | nao |
| `0x30..0x3F` | `CUSTOM_n` | comando customizado `n` (lista vem por `CONFIG`) | conforme `CONFIG` |

Codigos `DECK`:

| `cell` | nome | quando |
|---|---|---|
| 1 | `HELLO` | logo apos o agente assinar `EVENT` — "manda o estado". `arg` = motivo da **ultima** desconexao: codigo HCI (ex. `0x13` remote user terminated, `0x08` supervision timeout, `0x3D` MIC failure) ou `0x80\|erro` do host NimBLE; `0` = nenhuma desde o boot |
| 2 | `BONDED` | pareamento concluido |
| 3 | `PAGE` | `arg` = pagina agora visivel (0 grade, 1 sessao, 2 comandos, 3 ajustes) |
| 4 | `STATS` | a cada 10 s: `arg` = media de ms por iteracao do `loop()` do deck (saude/latencia; so log no agente) |

Semantica de toque nas celulas de sessao (0..7) fica **no agente**: `CELL_TAP` numa
celula ocupada = `FOCUS` daquela sessao (o deck nao precisa mandar `ACTION FOCUS`).
O deck mostra a borda coral so quando o proximo `SESSIONS` vier com `ativa` — nunca
por conta propria (confirmacao pelo agente).

O deck so emite `CELL_*` para as celulas de sessao (0..`session_cells`-1). As demais
areas da tela — no deck vertical 3x4: a linha de 3 celulas utilitarias (idioma, brilho,
status/ajustes) e a faixa livre de baixo — sao **locais ao deck** e nao geram EVENT
(idioma/brilho tambem podem ser escritos pelo agente via `CONFIG`).

As acoes de uma sessao (Voz, Modo, Esc, Enter, /compact, /clear, Lida) ficam na **pagina
da sessao** do deck (hold numa celula) e saem como `ACTION` com `cell` = aquela sessao.
Voz e push-to-talk: `VOICE_START` no press do botao, `VOICE_STOP` no release; o agente
solta o espaco sozinho se o deck cair ou apos 60 s de hold.

### 4.4 `USAGE` (agente → deck, 15 bytes)

| off | tipo | campo |
|---|---|---|
| 0 | u8 | `PROTO_VERSION` |
| 1 | u8 | % 5h (0..100; `255` = n/d) |
| 2 | u8 | % 7d (0..100; `255` = n/d) |
| 3 | u32 | epoch do reset 5h (0 = n/d) |
| 7 | u32 | epoch do reset 7d (0 = n/d) |
| 11 | u32 | epoch "agora" no host (o deck nao tem relogio — usa p/ contar regressivo) |

### 4.5 `CONFIG` (agente → deck, TLV)

`[0] = PROTO_VERSION`, depois 0..n entradas `[tipo u8][len u8][dados]`:

| tipo | len | dados |
|---|---|---|
| 1 | 1 | brilho 0..255 (o deck salva em NVS) |
| 2 | 1 | idioma: 0 pt, 1 en (salva em NVS) |
| 3 | 1 + 12·n | lista de comandos customizados da pagina CMD: `[n]` + n rotulos de 12 bytes. Rotulo `i` ↔ `CUSTOM_i`. Flag de confirmacao: rotulo comecando com `!` exige confirmar no deck (o `!` nao e exibido) |

## 5. Sequencia de conexao

```
deck: anuncia (UUID do servico)
agente: scan → conecta → le INFO → confere PROTO_VERSION
agente: escreve CONFIG COM resposta (dispara pareamento se ainda nao ha bond; deck mostra passkey;
        o agente espera ate 75 s pelo usuario digitar)
agente: assina EVENT
deck:   EVENT DECK/HELLO
agente: escreve SESSIONS (e USAGE, se houver); depois SESSIONS a cada mudanca + 3 s
deck:   EVENT a cada gesto
(queda) deck volta a anunciar; agente re-escaneia (backoff 1 s → 5 s) e repete tudo
```

## 6. Vetores de teste

`SESSIONS` com uma sessao `deck` (sid 7) WORKING modo plan ativa na celula 0, resto vazio:

```
10                                   ; frame unico
01 01 08 00                          ; ver, flags(pronto), 8 entradas, ativa=0
07 02 03 01 2A 00 64 65 63 6B 00 00 00 00 00 00 00 00   ; sid 7, WORKING, plan, ativa, age 42, "deck"
00 00 00 00 00 00 00 .. (×7 entradas vazias de 18 bytes)
```

`EVENT` tap na celula 2: `10 01 01 02 00`.
`EVENT` acao CLEAR confirmada na sessao ativa: `10 01 04 FF 12`.
