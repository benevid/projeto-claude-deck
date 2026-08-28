# Plano — Clow Deck (stream deck físico para Claude Code)

> Transformar a tela touch 3,5" (ESP32-S3, JC4832W535 — mesma placa do Usage Stick) num
> **deck físico dedicado ao Claude Code**: cada sessão aberta vira um botão com estado ao
> vivo (trabalhando / esperando você / ociosa), uma fileira de ações envia comandos e voz
> para a sessão escolhida, e uma grade impressa em 3D divide os botões fisicamente.
> A peça central NÃO é a tela: é o **agente no computador** — a tela é o painel remoto dele.

---

## 0. Decisões já tomadas

| Decisão | Escolha |
|---|---|
| Sistemas (v1) | **macOS + Windows** (Linux fora do escopo; arquitetura não impede depois) |
| Transporte | **Bluetooth LE** (ESP32-S3 não tem BT clássico — é BLE 5, e basta) |
| Papel do hardware | Periférico "burro": exibe o que o agente manda, reporta toques. **Zero segredo no dispositivo** |
| Injeção de comandos | **Foco de janela + teclas sintéticas** pelo agente (não existe IPC oficial p/ sessão interativa) |
| `--remote-control` | **Não é requisito da v1** (hoje não expõe API local p/ terceiros). Fica como trilho futuro — ver §1.3 |
| Voz | **Microfone do host** + transcrição local (whisper.cpp, PT/EN). O deck é só o gatilho push-to-talk |
| Layout | Grade **4×3** em 480×320 (células ~120×106 px): 2 fileiras de sessões (8) + 1 fileira de ações |
| Físico | Grade divisória **impressa em 3D** sobreposta à tela (como no case do Usage Stick) |
| Nome | Provisório **"Clow Deck"** (casa com o mascote Clow do app iOS); decidir na v1 |

### Por que BLE faz sentido (validação do estudo)

**A favor (ganha da proposta Wi-Fi original):**
- **Pareamento = UX de fone de ouvido**: sem portal de configuração, sem SSID/senha, sem
  mDNS, sem firewall. Funciona em rede corporativa com isolamento de clientes e quando o
  notebook troca de rede/VPN — dores reais do modelo Wi-Fi.
- **Zero segredo no firmware**: sem token da Anthropic, sem credencial de Wi-Fi, sem TLS.
  Quem fala com a API é o agente. Vaza/perde o deck? Não vaza nada.
- **Segurança nativa**: bonding BLE com passkey exibido NA TELA do deck (temos display+touch
  → capability "DisplayYesNo") = canal criptografado com proteção MITM. No Wi-Fi teríamos
  que inventar pareamento por token.
- **Firmware mais leve**: sem pilha TLS/certificados/crypto sobra RAM para LVGL + NimBLE.
- Alimentação continua por USB (qualquer carregador) — BLE não depende do USB de dados.

**Contras aceitos:**
- Alcance de mesa (~metros) — irrelevante, é um dispositivo de mesa.
- Um host por vez — ok para o caso de uso.
- Banda baixa (~20–90 KB/s útil): **não enviar bitmaps** pelo ar; ícones/mascote ficam
  gravados no firmware e o agente manda só texto + cor + glyph id + estado.
- Atualização de firmware: por USB (script `flash.sh` como no Usage Stick); OTA-BLE é lento
  e fica para depois, se um dia precisar.

---

## 1. Fundamentos técnicos verificados (não é achismo)

### 1.1 Descobrir sessões e estados — caminho oficial e robusto
- **Processos**: cada sessão interativa é um processo `claude` com cwd legível de fora
  (`lsof -a -p PID -d cwd` no macOS; no Windows/WSL via ponte `wsl.exe`). Verificado
  empiricamente: 5 sessões enxergadas na máquina de desenvolvimento.
- **Hooks do Claude Code** (docs oficiais): todo hook recebe em stdin JSON com
  `session_id`, `cwd`, `permission_mode`, `transcript_path`. Os que importam:
  - `SessionStart` / `SessionEnd` → botão aparece/some.
  - `Stop` → Claude terminou de responder → **verde**.
  - `PermissionRequest` → esperando permissão → **laranja piscando** (o recurso matador).
  - `UserPromptSubmit` → voltou a trabalhar → **neutro pulsando**.
  - O agente instala/remove os hooks na config do usuário (comando `curl` para
    `http://127.0.0.1:<porta-do-agente>` com o JSON do stdin — push, sem polling).
- **Statusline** (opcional, M2+): recebe modelo, % de contexto e custo por sessão; o agente
  pode encadear no comando de statusline existente do usuário para enriquecer os botões.

### 1.2 Enviar comandos — a única porta é o teclado
Verificado na doc oficial: **não há IPC/CLI para injetar texto ou trocar modo numa sessão
interativa já aberta** (`/clear`, `/compact` e o ciclo de modos são só do TUI). Logo:
- **Focar a janela certa**: o processo `claude` tem um TTY; Terminal.app e iTerm2 expõem por
  AppleScript qual aba usa qual TTY → mapeamento processo→aba **exato**. Windows: janela via
  Win32; Windows Terminal precisa de spike (ver riscos).
- **Sintetizar teclas**: macOS CGEvent (permissão de Acessibilidade), Windows `SendInput`.
- **Modos**: `Shift+Tab` cicla `default → acceptEdits → plan → …` na sessão focada; o modo
  corrente chega de graça nos hooks (`permission_mode`) → o botão mostra o modo real.
- Tudo isso atrás de uma interface `InjectionProvider` trocável (ver §1.3).

### 1.3 Remote Control — por que fica de fora da v1
`--remote-control` conecta a sessão ao claude.ai/code e ao app móvel, **mas não expõe API
local documentada** para um terceiro (nosso agente) mandar mensagens. Usar os endpoints
privados seria engenharia reversa de API da Anthropic — fundação errada para um produto.
Decisão: injeção por teclado é o mecanismo da v1; `InjectionProvider` permite plugar um
transporte oficial se/quando existir. Bônus não-técnico: usuários que já usam remote-control
continuam com o celular como segundo controle — não conflita com o deck.

---

## 2. Produto

### 2.1 Proposta
"Suas sessões do Claude Code viram botões físicos." Olhou, viu quem precisa de você
(laranja piscando); tocou, a janela certa veio pra frente; segurou o botão de voz, falou,
o texto entrou na sessão ativa; um toque cicla o modo de permissão. Sem alt-tab, sem caçar
janela, sem perder o momento em que o Claude parou esperando resposta.

### 2.2 Layout da tela (480×320, grade 4×3)

```
┌─────────┬─────────┬─────────┬─────────┐
│ usage-  │ projeto-│ GameLED │ n8n-    │   ← fileira 1: sessões 1–4
│ stick ⏳│ escopo ●│ -PCB  ✔ │ ffmpeg ⚠│      (nome da pasta + estado)
├─────────┼─────────┼─────────┼─────────┤
│ ios-app │   ---   │   ---   │   ---   │   ← fileira 2: sessões 5–8
├─────────┼─────────┼─────────┼─────────┤
│ 🎤 VOZ  │ ⇄ MODO  │ ⚡ CMD  │ ☰ 5h:72%│   ← fileira 3: ações
└─────────┴─────────┴─────────┴─────────┘
```

- **Botão de sessão** — cor/estado: neutro pulsando (trabalhando), **laranja piscando**
  (esperando permissão/atenção), verde (terminou/ocioso), vermelho (erro), cinza (morta).
  Rótulo = basename do cwd (ASCII; transliterar acentos — limite da fonte LVGL atual).
  - *Tap*: foca a janela no host **e** marca como "sessão ativa" do deck (borda coral).
  - *Hold*: página da sessão (comandos `/compact`, `/clear` com confirmação, modo, info).
- **VOZ**: push-to-talk — segurar grava (mic do host), soltar transcreve e digita na sessão
  ativa + Enter. Feedback na célula (nível/contador). Tap curto = dica de uso.
- **MODO**: mostra o `permission_mode` real da sessão ativa; tap envia Shift+Tab.
- **CMD**: abre página 2 com grade de comandos configuráveis (`/compact`, `/clear`,
  prompts prontos do usuário, etc.).
- **☰/usage**: strip com a janela 5h (o agente reaproveita o probe do Usage Stick e manda
  o % pelo BLE — o deck não fala com a Anthropic). Hold = brilho/pareamento/config.

### 2.3 Requisitos de setup (onboarding do agente)
- macOS: permissões de **Acessibilidade** (teclas), **Bluetooth**, **Microfone** e
  **Automação** (AppleScript no Terminal/iTerm). O agente guia cada uma com telas.
- Windows: Bluetooth + microfone; sem equivalente de Acessibilidade para `SendInput`.
- Instalação dos hooks: 1 clique no agente (escreve na settings do usuário, reversível).

---

## 3. Arquitetura

### 3.1 Três peças

1. **Firmware** (`firmware/`) — ESP32-S3 + LVGL 9.2 + **NimBLE** (peripheral):
   reaproveita do Usage Stick o bring-up de display/touch, estrutura de telas e o case como
   base da grade 3D. Remove: Wi-Fi manager, TLS/certs, crypto, token. UI = grade 4×3
   dirigida por estado recebido; toques viram eventos BLE.
2. **Agente** (`agent/`) — app de bandeja **Tauri v2 (Rust)**, um código para macOS+Windows:
   - BLE central: `btleplug` (CoreBluetooth/WinRT por trás).
   - Descoberta: scan de processos + TTY; ponte WSL no Windows.
   - Estado: servidor HTTP localhost recebendo os hooks.
   - Injeção: `enigo`/CGEvent/SendInput + AppleScript (via osascript) para foco por aba.
   - Voz: `whisper-rs` (modelo base/small multilíngue, ~150–500 MB, download no 1º uso).
   - Config: webview do próprio Tauri (mapear botões, comandos custom, terminal preferido).
   - *Alternativa avaliada*: Go (systray + tinygo-bluetooth + robotgo). Decidir no spike M0
     — critério: qualidade do BLE central e da síntese de teclas nos DOIS SOs.
3. **Protocolo** (`protocol/PROTOCOL.md`) — GATT service próprio, characteristics:
   - `Sessions` (notify, agente→deck): lista compacta [id curto, rótulo, estado, modo, ativa].
   - `ButtonEvent` (write, deck→agente): [célula, tap|hold|release].
   - `Usage` (notify): % 5h/7d + reset epoch.
   - `Config` (write): layout/página/brilho.
   - MTU 247, framing com sequência p/ payloads > MTU, versionamento no handshake.
   - Bonding com passkey no display (MITM). Agente só aceita deck pareado.

### 3.2 Fluxos principais

- **Estado**: hook dispara → POST localhost → agente atualiza modelo → notify BLE → LVGL
  redesenha a célula. Latência esperada < 300 ms do evento ao pixel.
- **Toque em sessão**: ButtonEvent → agente resolve PID→TTY→aba → foca janela → confirma
  → deck marca ativa.
- **Voz**: hold VOZ → agente grava → release → whisper transcreve → foca sessão ativa →
  digita texto + Enter → hook `UserPromptSubmit` confirma que entrou (célula pulsa).
- **Queda de conexão**: deck volta a anunciar; agente reconecta sozinho; tela mostra
  "procurando host" com o Clow procurando (reuso do mascote).

---

## 4. Riscos críticos — atacar ANTES de codar o resto

| # | Risco | Mitigação / spike |
|---|---|---|
| 1 | **Foco de aba no Windows Terminal** (sem API pública p/ focar aba existente) | Spike no M0 junto com BLE: UIA (UI Automation) p/ trocar aba; fallback: focar a janela e orientar 1 sessão por janela. Se feio demais, documentar limitação |
| 2 | **VS Code terminal integrado** (não dá p/ focar um terminal específico de fora) | Fallback: focar a janela do VS Code do projeto (match por pasta no título). Documentar |
| 3 | RAM: LVGL + NimBLE + PSRAM | Sem TLS sobra folga; NimBLE é leve; validar no bring-up M0 |
| 4 | Atalhos/comandos do Claude Code mudam entre versões | Mapeamentos em config (não hardcoded); teste por versão; hooks são API estável documentada |
| 5 | Segurança: agente digita teclas por comando remoto | Bonding BLE obrigatório + passkey na tela; whitelist de comandos; `/clear` e afins pedem confirmação no deck |
| 6 | macOS TCC/notarização (Acessibilidade some a cada build não assinado) | Assinar o agente desde M1 com o Developer ID (conta já existe) |
| 7 | WSL: hooks dentro do WSL precisam alcançar o agente no Windows | `localhost` com mirrored networking (Win11) ou IP do host; testar no M4 |
| 8 | Fonte LVGL só ASCII | Transliterar rótulos (já resolvido no Usage Stick com TRS) |

---

## 5. Marcos

### M0 — Fundações + spikes matadores (decisões com código)
- Repo `projeto-claude-deck` com `firmware/`, `agent/`, `protocol/`, `case-3d/`.
- **Spike A (transporte)**: ESP32 anunciando GATT + Tauri/btleplug conectando, notify de
  contador e write de toque, nos DOIS SOs. Mede latência e estabilidade de reconexão.
- **Spike B (injeção)**: focar aba por TTY (macOS) e janela (Windows) + Shift+Tab sintético
  chegando numa sessão real. Inclui o teste de Windows Terminal (risco #1).
- **Spike C (hooks)**: hook `Stop`/`PermissionRequest` → POST localhost → log.
- Gate: fechar stack do agente (Tauri vs Go) com base nos spikes.

### M1 — Fatia vertical macOS (demo completa)
- Firmware: grade 4×3 com células de sessão dirigidas por BLE (estados + cores + rótulos).
- Agente: descoberta ps+TTY, hooks instalados, modelo de sessões, foco por tap,
  MODO com Shift+Tab, bonding com passkey.
- Critério: 3 sessões reais na tela, laranja piscando quando uma pede permissão,
  tap trazendo a janela certa pra frente.

### M2 — Ações + voz
- Sessão ativa (borda coral), push-to-talk com whisper.cpp (PT/EN), digitação do transcript.
- Página 2 de comandos (grade configurável) + confirmação no deck p/ comandos destrutivos.
- Strip de uso 5h via probe no agente (porta a lógica `unified-*` do Usage Stick p/ Rust).

### M3 — App de configuração
- UI (webview Tauri): mapear células, comandos custom, terminal preferido, idioma PT/EN,
  gerenciar pareamento, instalar/remover hooks com um clique.
- Persistência + migração de config; onboarding guiado de permissões (TCC).

### M4 — Windows ✅ (2026-08-28)
- BLE WinRT, `SendInput`, foco de janela e descoberta (`sysinfo`) implementados e validados
  numa máquina Win11; deck controlando sessões reais de Codex e opencode.
- Pareamento exige **sessão de desktop** e um adaptador com papel de **central BLE + LE Secure
  Connections** (TP-Link UB500 sim, Realtek RTL8821CU não) — ver `docs/CODEX-INTEGRATION.md`.
- Instaladores NSIS + MSI gerados (`dist/windows/`).
- **Pendente**: ponte WSL (descoberta + hooks alcançando o agente).

### M5 — Polimento e identidade
- Mascote Clow no deck (glyphs no firmware): procurando host, celebrando sessão concluída,
  dormindo quando tudo ocioso. Animações de estado (pulso/piscar) procedurais no loop.
- Grade 3D: modelar célula 4×3 encaixando no case existente; STL em `case-3d/`.
- Assinatura/notarização macOS, docs de instalação, `flash.sh` do firmware.

---

## 6. Estrutura de repositório

```
projeto-claude-deck/
├── PLANO-CLAUDE-DECK.md        # este documento
├── protocol/PROTOCOL.md        # GATT, framing, versionamento (fonte da verdade)
├── firmware/                   # ESP32-S3: LVGL + NimBLE (base: Usage Stick sem Wi-Fi/TLS)
├── agent/                      # Tauri v2 (Rust): tray, BLE central, hooks, injeção, voz
└── case-3d/                    # grade divisória + case (base: 3D Case do Usage Stick)
```

---

## 7. Fora do escopo da v1 (anotado para não esquecer)
- Linux (X11 seria fácil; Wayland é hostil a injeção — decidir depois).
- OTA de firmware por BLE (atualização fica por USB).
- Plugin para Stream Deck da Elgato usando o MESMO agente (o agente é o produto; a Elgato
  vira só outro "display" — caminho natural de expansão).
- Integração com o app iOS/Clow (ex.: deck e iPhone mostrando o mesmo estado) — depois que
  os dois existirem.
- Áudio pelo deck (a placa não tem microfone/alto-falante).
