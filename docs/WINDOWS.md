# Clow Deck no Windows (M4)

Estado em 2026-08-28: **validado numa maquina Win11 (AMD64)** — build, 15/15 testes,
descoberta de sessoes (cwd/rotulo/terminal), hooks, `doctor`, inicio no login (chave `Run`),
servidor HTTP + deck virtual, foco de janela, teclas sinteticas, instaladores e, com o
adaptador certo, o **caminho BLE inteiro**: `scan`, `pair`, `run` e o deck operando sessoes
reais de **Codex** e **opencode**.

Pendente: ponte **WSL** (hooks de dentro do WSL alcancando o agente no Windows).

## Instalacao

Os instaladores sao gerados pelo build do `app/` (Tauri) e ficam em
`app/target/release/bundle/{nsis,msi}/`; copias para teste manual ficam em `dist/windows/`
(nao versionado). Nenhum e assinado ainda: o SmartScreen mostra "Windows protegeu o
computador" na primeira execucao → **Mais informacoes → Executar assim mesmo**.

1. Instale com o `Clow Deck_<versao>_x64-setup.exe` (NSIS; ha tambem um `.msi` para
   implantacao/GPO).
2. Abra o **Clow Deck** (icone do invader na bandeja) → **Abrir deck virtual**. As sessoes de
   Claude Code / Codex / opencode aparecem sozinhas.
3. Hooks do Claude Code: menu da bandeja → **Instalar hooks**.

CLI (rode numa janela do PowerShell **na sessao de desktop**, nao por SSH):

```powershell
.\clowdeck-agent.exe doctor          # curl, Bluetooth, porta, descoberta, config
.\clowdeck-agent.exe sessions        # sessoes vistas agora
.\clowdeck-agent.exe ble scan        # o deck aparece?
.\clowdeck-agent.exe ble pair        # pareia; pergunta o codigo que a placa mostra
.\clowdeck-agent.exe ble info        # conecta e le INFO
```

## Bluetooth: o adaptador decide

O agente precisa que o radio faca o papel de **central BLE com LE Secure Connections** — o
firmware exige bond + MITM + SC (`DECK_BLE_SECURE`). Validado com um **TP-Link UB500**
(`USB\VID_2357&PID_0604`, RTL8761B, driver TP-Link 1.9.1051.3016).

Um dongle Realtek **RTL8821CU** *escaneia* o deck mas nao completa o pareamento: falha em
`AuthenticationFailure` (19) mesmo com o passkey correto. Reproduzido com duas bibliotecas
(btleplug e bleak) e dois firmwares (NimBLE e Bluedroid), com e sem seguranca, enquanto o Mac
conectava na mesma sala — historico completo em [`CODEX-INTEGRATION.md`](CODEX-INTEGRATION.md).

Confira o seu adaptador antes de comprar/testar (o `doctor` so verifica se o servico
`bthserv` esta de pe, nao as capacidades do radio):

```powershell
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$m = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
  $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
  $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
$null = [Windows.Devices.Bluetooth.BluetoothAdapter,Windows.System.Devices,ContentType=WindowsRuntime]
$t = $m.MakeGenericMethod([Windows.Devices.Bluetooth.BluetoothAdapter]).Invoke(
       $null, @([Windows.Devices.Bluetooth.BluetoothAdapter]::GetDefaultAsync()))
$t.Wait(-1) | Out-Null
$t.Result | Format-List IsLowEnergySupported, IsCentralRoleSupported,
                        AreLowEnergySecureConnectionsSupported
```

Exija **`IsCentralRoleSupported = True`** e **`AreLowEnergySecureConnectionsSupported = True`**.

## Armadilhas do pareamento

- **`ble pair` so funciona numa sessao de desktop.** Por SSH ou tarefa nao interativa o WinRT
  devolve `AccessDenied` (status 12) *sem nem iniciar a cerimonia* — a placa, portanto, nao
  chega a mostrar o passkey. Uma sessao RDP conta como desktop e funciona.
- **A placa fica pareada-e-conectada a um host por vez.** Feche o app do Mac
  (`pkill -f clow-deck-app`) antes de parear no Windows, senao o agente do Mac segura o link e
  o PC nao enxerga a placa.
- **O passkey e sorteado a cada boot da placa** (`s_passkey` em `ble_link.cpp`) e nunca vai
  para a serial: so da para ler na tela, e so depois que a cerimonia comeca. Nao reinicie a
  placa no meio do pareamento.
- Dois avisos do `BTHUSB` no Visualizador de Eventos **nao sao erro**: id=6 "Only one active
  Bluetooth adapter is supported at a time" (dois radios plugados ao mesmo tempo) e id=18
  "cannot store link keys on the local adapter" (afeta so teclado BT dentro da BIOS).
