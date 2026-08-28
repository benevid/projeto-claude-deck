# Release macOS — app de bandeja (M3), assinatura e notarizacao

O app de bandeja (`app/`, Tauri 2) embute o agente (`agent/` como lib) e substitui o
servico CLI: ao abrir, ele derruba o LaunchAgent `my.autom.clowdeck` (mesma porta) e
assume o agente. "Iniciar no login" cria `my.autom.clowdeck.app.plist`
(RunAtLoad + KeepAlive só em saida com erro — "Sair" respeitado).

## Build local

```bash
cd app
PATH="$HOME/.cargo/bin:$PATH" cargo tauri build --bundles app   # .app em app/target/release/bundle/macos/
../tools/make_dmg.sh                                            # DMG em app/target/release/bundle/dmg/
APPLE_SIGNING_IDENTITY="Developer ID Application: ..."          # (env) identidade p/ .app e DMG
```

O DMG e gerado por `tools/make_dmg.sh` (hdiutil puro): o bundler do Tauri arruma a
janela do DMG dirigindo o Finder por AppleScript, o que exige permissao de Automacao
e falha com "AppleEvent timed out (-1712)" em shells nao interativos/CI.
O script embute o icone do app no **volume** (.VolumeIcon.icns + bit custom, via
UDRW→UDZO) e no **arquivo .dmg** (resource fork via Rez, apos assinar — o resource
fork fica fora do data fork, entao assinatura e staple continuam validos). Qualquer
mudanca de CONTEUDO do DMG (ex.: icone do volume) exige re-submeter a notarizacao.

Sem `APPLE_SIGNING_IDENTITY` o bundle sai com assinatura ad-hoc (roda só nesta maquina
e o TCC esquece as permissoes a cada build). Com "Apple Development: ..." roda nas
maquinas do time de dev. Para DISTRIBUIR fora da App Store precisa de
**Developer ID Application** + notarizacao (abaixo).

## Como isto esta configurado aqui

- Certificado usado nas releases: um **Developer ID Application** da conta paga
  (`security find-identity -v -p codesigning` lista o que existe no keychain).
- Perfil de credenciais no keychain: `clowdeck`
  (`xcrun notarytool ... --keychain-profile clowdeck`).
- **A senha de app precisa ser gerada na MESMA conta Apple dona do certificado**, em
  appleid.apple.com — uma senha de app de outro Apple ID devolve **401** na notarizacao.
  Foi exatamente esse o erro que custou uma tarde: havia dois Apple IDs em uso na maquina.
- Um certificado "Apple Development" de outro time serve so para build local: assina, mas
  nao distribui.

## Criar o certificado "Developer ID Application" (uma vez)

So o **Account Holder** da conta paga pode criar certificados Developer ID.

Caminho A — Xcode (mais simples):
1. Xcode → Settings → Accounts → selecionar o Apple ID → **Manage Certificates…**
2. botao **+** → **Developer ID Application**. (Se quiser distribuir `.pkg` tambem:
   **Developer ID Installer**.)
3. O certificado + chave privada caem direto no Keychain. Conferir:
   `security find-identity -v -p codesigning` deve listar
   `Developer ID Application: <nome> (<TEAMID>)`.

Caminho B — portal web:
1. https://developer.apple.com/account/resources/certificates → **+**
2. Tipo **Developer ID Application** → continuar.
3. Gerar um CSR no Mac: Acesso as Chaves → menu Acesso as Chaves →
   Assistente de Certificado → **Solicitar um Certificado de uma Autoridade…**
   (e-mail da conta, "Salva no disco").
4. Subir o CSR, baixar o `.cer`, dar dois cliques para instalar no Keychain.

## Notarizacao (uma vez a configuracao, depois automatica)

1. Criar uma **chave de API do App Store Connect** (Users and Access → Integrations →
   App Store Connect API → Team Key, papel Developer) e baixar o `.p8`;
   ou usar Apple ID + senha de app (appleid.apple.com → App-Specific Passwords).
2. Guardar credenciais no keychain:
   ```bash
   xcrun notarytool store-credentials clowdeck \
     --key AuthKey_XXXX.p8 --key-id XXXX --issuer <issuer-uuid>
   # ou: --apple-id you@x.com --team-id TEAMID --password <senha-de-app>
   ```
3. Build assinado com Developer ID e notarizar o DMG:
   ```bash
   APPLE_SIGNING_IDENTITY="Developer ID Application: <nome> (<TEAMID>)" cargo tauri build
   xcrun notarytool submit "app/target/release/bundle/dmg/Clow Deck_*.dmg" \
     --keychain-profile clowdeck --wait
   xcrun stapler staple "app/target/release/bundle/dmg/Clow Deck_*.dmg"
   ```
   (O Tauri tambem sabe notarizar sozinho com as envs `APPLE_API_ISSUER`/`APPLE_API_KEY`/
   `APPLE_API_KEY_PATH` — ai o `cargo tauri build` ja entrega o DMG carimbado.)

## Permissoes (TCC) do app

Sao pedidas na primeira execucao, em nome de "Clow Deck": **Bluetooth** (BLE com o
deck), **Acessibilidade** (teclas sinteticas — dialogo ~25 s depois de abrir) e
**Automacao** (AppleScript ao focar Terminal/iTerm). As permissoes ficam presas ao
certificado: builds assinados com a MESMA identidade nao perdem os grants.

## Firmware (referencia p/ o gravador web do usuario)

Binario mesclado para Web Serial/esptool-js:
```bash
esptool --chip esp32s3 merge-bin -o clow_deck_merged.bin \
  0x0 bootloader.bin 0x8000 partitions.bin 0xe000 boot_app0.bin 0x10000 app.bin
```
(offsets do esquema de particao custom em `firmware/clow_deck/partitions.csv`; a placa
usa `CDCOnBoot=cdc` e aparece como porta USB-CDC.)
