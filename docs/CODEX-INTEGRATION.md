# Estudo: integrar o Codex CLI ao Clow Deck (proposta M6)

> Estudo de viabilidade (2026-08-24), baseado na instalacao real da bancada:
> `codex-cli 0.149.0` (Homebrew). Conclusao: **viavel, e por um caminho melhor que o
> do Claude Code** — o Codex expoe um daemon local ("app-server") com protocolo
> JSON-RPC tipado que cobre monitoramento E acao programatica, sem teclas sinteticas.

## O que o Codex 0.149 oferece (verificado)

- **`codex app-server`**: daemon local de sessoes, endpoint `ws://host:port` ou
  `unix://PATH`; `codex app-server generate-json-schema --out <dir>` emite o protocolo
  completo (291 tipos; copia gerada no scratchpad da sessao de estudo).
- **Eventos que mapeiam 1:1 no nosso modelo de estados** (`agent/src/model.rs`):
  | Protocolo Codex (v2) | Estado do deck |
  |---|---|
  | `TurnStartedNotification` | WORKING |
  | `TurnCompletedNotification` | DONE |
  | `ThreadStatusChangedNotification` | (fonte direta de estado) |
  | `CommandExecutionRequestApproval` / `FileChangeRequestApproval` / `PermissionsRequestApproval` (ServerRequests) | ATTENTION |
  | `ThreadClosedNotification` / `ThreadArchivedNotification` | DEAD |
  | `ErrorNotification` | ERROR |
- **Acoes programaticas** (sem foco de janela, sem enigo):
  | Acao do deck | Codex |
  |---|---|
  | ENTER / prompt | `TurnStartParams` ou `codex queue --thread <id> --message <txt>` |
  | ESC (interromper) | `TurnInterruptParams` |
  | /compact | `ThreadCompactStartParams` |
  | aprovar pedido (novo!) | responder o ServerRequest de approval |
  | steer (novo!) | `TurnSteerParams` |
- **Descoberta**: `ThreadListParams`/`ThreadLoadedListParams` pelo protocolo; e o processo
  TUI (`codex` sob node, tty + cwd via lsof) e visto pela nossa descoberta atual com
  uma linha a mais no match de nomes (`discovery/macos.rs`).
- **Voz**: o TUI nao tem `/voice`, mas o protocolo tem `ThreadRealtime*`
  (SDP/audio/transcript) — da para, no futuro, um push-to-talk do deck falando com o
  Realtime do Codex. Curto prazo: botao de voz desabilitado em sessoes Codex.

## Resultados da sonda ao vivo (M6.a, 2026-08-24)

- **Embutir funciona**: `codex app-server` como filho, JSON-RPC NDJSON via stdio;
  `initialize` com `capabilities.experimentalApi=true`; notificacoes fluem.
- **Pegadinha**: `thread/list` sem `sourceKinds` explicito filtra fora `exec` (e
  retornava vazio) — passe `["cli","vscode","exec","appServer","unknown"]`.
- Itens de thread trazem `cwd`, `updatedAt`, `status` — `cwd` casa com a descoberta.
- **Limite encontrado**: threads carregados em OUTRO processo (o TUI do usuario)
  aparecem como `status: notLoaded` para o nosso app-server; o M6.a usa o avanco de
  `updatedAt` entre polls como sinal de WORKING. Estados ricos ao vivo virao quando
  as acoes rodarem pela NOSSA instancia (M6.b: `TurnStart`/queue tornam o agente o
  host do turno e os eventos chegam completos) ou via daemon compartilhado
  (`codex app-server daemon start` pelo brew pede o instalador oficial — irrelevante
  para o modo embutido).

## Questao aberta original (respondida acima)

O TUI interativo registra a sessao no daemon compartilhado? (`codex agents` sugere que
sim — "all agent sessions on the shared local app-server daemon" — e `~/.codex/
thread-writer-locks` reforca; confirmar com `codex agents`/`ThreadList` com um TUI
aberto.) Se NAO registrar: fallback = teclas sinteticas (funcionam hoje: foco por
cwd + Esc/Enter/Tab/texto sao iguais) + estados via `notify`/heuristica, igual ao
plano B abaixo.

## Arquitetura proposta

1. **`agent/src/engines/`**: trait `Engine` (descobrir, estados, acoes) com duas
   implementacoes: `claude` (hooks HTTP atuais) e `codex` (cliente do app-server:
   conectar `unix://`, `ThreadList` + subscribe, traduzir notificacoes para o
   `SessionTable`).
2. **`SessionTable`**: campo `engine` por sessao; celulas do deck mostram um badge
   `CDX` (chip ja existe). PROTOCOL.md: bit livre em `flags` (bit2 = engine != claude)
   — aditivo, sem quebrar formato.
3. **`dispatch.rs`**: por acao, se `engine == codex` usar o cliente do protocolo;
   senao, o caminho atual (foco + teclas). MODE_CYCLE vira `approval policy` toggle
   (config do Codex) ou fica desabilitado na v1; /clear vira `/new` (via queue) ou
   `ThreadStart` novo.
4. **Config**: `[engines.codex]` no config.toml do agente (endpoint, habilitar).

## Fases sugeridas

- **M6.a (1 sessao de trabalho)**: probe ao vivo do daemon + descoberta do processo
  TUI + celulas mostrando sessoes Codex com estados via protocolo (read-only).
- **M6.b (IMPLEMENTADO 2026-08-24)**: acoes por engine no dispatch — sessao Codex:
  `/compact` e `/clear`(→`/new`) e comandos custom vao por `codex queue --thread <id>`
  (sem focar janela; thread vinculado por cwd pelo poll) com fallback de teclado;
  Esc/Enter/Tab continuam teclado (agnosticos); MODO e VOZ bloqueados com mensagem
  clara (sao recursos do Claude Code). Approvals direto do deck ficam para M6.c
  (exigem hospedar o turno na nossa instancia).
- **M6.c (IMPLEMENTADO 2026-08-24)**: estados AO VIVO por **tail dos rollouts**
  (`~/.codex/sessions/AAAA/MM/DD/rollout-*.jsonl`, que o TUI escreve em tempo real:
  `task_started`->WORKING, `task_complete`->DONE, eventos de aprovacao->ATTENTION;
  offset comeca no fim = so eventos novos, zero conflito de escrita) + **APPROVE
  (0x17)**: na pagina de sessao Codex, a celula de voz vira APROVAR (tecla 'y' no
  TUI apos focar; claude usaria '1') e a de modo vira NEGAR (Esc). Voz via
  Realtime segue exploratorio (futuro).

## Riscos

- Superficie marcada **[experimental]** e versionada (v2): pin de versao minima do
  codex + checagem no `doctor`; o schema gerado versionado no repo para diff.
- Sessoes TUI fora do daemon (ver questao aberta) → fallback por teclas ja validado.
- `state_5.sqlite` bloqueado para leitura externa — nao depender de sqlite, so do
  protocolo.


# Windows (M4) — BLE: limite do radio, nao do codigo (2026-08-27)

Bancada: PC Win11 com dongle **Realtek RTL8821CU** (`VID_0BDA&PID_C821`) e driver
generico da Microsoft. Resultado dos testes remotos:
- `ble scan` **acha** o deck (RSSI -62) — o radio escaneia normalmente;
- `ble pair` (WinRT `DeviceInformationCustomPairing`) chega a negociar: o Windows pede
  `ProvidePin` (a placa exibe o passkey), mas o resultado final e `Failed` (19) —
  intermitente: a cerimonia so disparou em 2 de ~8 tentativas;
- com firmware **`DECK_BLE_SECURE=0`** (sem pareamento nenhum) o `connect()` continua
  falhando ("Not connected" / sem resposta em 20 s);
- **controle**: o Mac conecta e le INFO da MESMA placa, mesmo firmware, na mesma sala.

Conclusao: o dongle escaneia mas nao sustenta o papel de **central** BLE com o driver
inbox. O caminho BLE do agente no Windows so pode ser validado com um adaptador que
funcione (driver Realtek oficial ou dongle BT 5.0 Intel). Correcoes de codigo que estes
testes renderam (validas para qualquer maquina):
- `find_deck` sem `ScanFilter` de servico: no WinRT o anuncio chega com `services`
  vazio e o filtro descartava o proprio deck;
- `connect_tolerant()`: no Windows o `connect()` do btleplug pode falhar porque o WinRT
  so conecta ao acessar o GATT — seguimos para `discover_services()`;
- novo `clowdeck-agent ble pair [passkey]` (WinRT), interativo (pergunta o codigo que a
  placa mostra) e com erros claros: `AccessDenied` = rodar numa **sessao de desktop**
  (SSH/servico e Sessao 0 e o Windows recusa), `AuthenticationFailure` = passkey errado.

# opencode (M7) — implementado 2026-08-25

Mesmo padrao do M6, com fonte de eventos ainda melhor: o opencode faz **event-sourcing
em sqlite** (`~/.local/share/opencode/opencode.db`): tabela `event` (type/data JSON com
`sessionID`), `session` (com `directory` = cwd) e `permission` (aprovacoes). Leitura
`-readonly` no banco VIVO funciona sem conflito (WAL).

- `agent/src/opencode.rs`: poll de 1,5 s por rowid via o `sqlite3 -json` do sistema
  (zero dependencias novas); `message.part.updated`->WORKING, `message.updated`
  (assistant + time.completed)->DONE, eventos/linhas de `permission`->ATTENTION
  — CORRECAO 2026-08-25 apos teste real: a tabela `permission` guarda regras
  PERSISTIDAS ("allow always"), nao pedidos pendentes (heuristica removida); o pedido
  pendente aparece como tool part com `state.status == "pending"` no evento
  `message.part.updated` — e essa a fonte de ATTENTION (o ultimo estado de cada lote
  de poll vence, entao pending->running de tools auto-aprovados nao pisca).
- Descoberta: binario `opencode` com TTY, excluindo os subcomandos de servico
  (serve/acp/run/db/...). Flag de entrada **bit3 = EF_OPENCODE**; chip "OC"; logo
  oficial (lobehub) como marca d'agua, no firmware e no deck virtual.
- Acoes por engine: MODO -> tecla **Tab** (alterna build/plan no opencode);
  /clear -> `/new`; `/init` -> AGENTS.md; APROVAR -> **Enter** (confirma a opcao
  padrao do dialogo de permissao — VALIDAR na 1a aprovacao real); NEGAR -> Esc;
  voz bloqueada (recurso do Claude Code).
