# Deep Space Glass — tema visual do Clow Deck

> Fonte da verdade visual do firmware (`firmware/clow_deck/`). Tema custom criado com a
> skill `theme-factory` (ponto de partida: "Midnight Galaxy") + regras de `ui-ux-pro-max`
> (glassmorphism em dark mode, alvos de toque >= 44 px, contraste >= 4.5:1 para texto).
> Ideia: o "liquid glass" da Apple em tom escuro — fundo profundo quase preto com leve
> gradiente, superficies de vidro escuro translucido, bordas finas claras de baixa
> opacidade, um fio de luz no topo de cada superficie, cantos generosos.

## 1. Paleta (revisao 2026-08-23 — cores da marca Claude sobre vidro escuro)

Base: `brand-guidelines-anthropic` (dark `#141413`, light `#faf9f5`, mid gray `#b0aea5`,
laranja `#d97757`, azul `#6a9bcc`, verde `#788c5d`). O vidro escuro e os estados seguem
essa familia; a atencao usa um ambar proprio para nao se confundir com o acento laranja.

| Token | Hex | Uso |
|---|---|---|
| `C_BG_TOP` / `C_BG_BOTTOM` | `#0C0D0C` → `#141513` | fundo quase preto (OLED) + ceu estrelado |
| `C_GLASS` / `C_GLASS_HI` / `C_GLASS_PRESS` | `#262624` / `#2F2E2B` / `#3B3A36` | superficies de vidro (opa ~215), highlight, pressionado |
| `C_GLASS_GLOW` | `#48342B` | glow quente (reservado; celulas usam a almofada abaixo) |
| `C_CELL` / `C_CELL_LINE` / `C_CELL_DIM` | `#131412` / `#3A3B39` / `#242523` | celula quase preta; contorno ocupado/vazio (variante B) |
| `C_STRIP` / `C_ACCENT_HI` | `#10110F` / `#E2957F` | fundo da faixa; coral claro p/ icones utilitarios |
| `C_EDGE` | `#FAF9F5` @30 borda · @40 fio de luz | contorno do vidro |
| `C_TEXT` / `C_MUTED` / `C_FAINT` | `#FAF9F5` / `#B0AEA5` / `#6B6960` | texto |
| `C_ACCENT` / `C_ACCENT_DEEP` | `#D97757` / `#B35F42` | acento (ativa, botoes primarios) |
| `C_WORK` | `#A3E635` | WORKING (verde limao — padrao do usuario; pulso + borda/glow) |
| `C_ATTN` (+ `C_ON_ATTN` `#141413`) | `#F0B35B` | ATTENTION (pisca 500 ms) |
| `C_DONE` / `C_IDLE` | `#788C5D` / `#4E5A40` | DONE (pisca lento < 60 s) / IDLE |
| `C_ERR` | `#C8524A` | ERROR |
| DEAD | `C_FAINT` | rotulo apagado |

O mascote ao fundo de cada celula e tingido com a cor do estado (opa 40–90/255).

## 2. Forma

| token | valor |
|---|---|
| raio celula / faixa | 18 px |
| raio botao | 16 px |
| raio chip | 10 px |
| raio caixa de overlay | 20 px |
| superficie | **variante B (contorno luminoso, escolhida em mockup 2026-08-24)**: celula flat `C_CELL`, contorno 2 px (`C_CELL_LINE` ocupada / `C_CELL_DIM` vazia / cor do estado quando ha estado) |
| glow de estado | sombra colorida (shadow 14 px spread 2, cor do estado, opa 110) aplicada pelo `grid_anim`; ativa = coral |
| press | eleva por claridade (`#1E1F1D`) + afunda 1 px — sem sombras pretas |
| badge de estado | chip no topo-direito: bg estado opa 55, borda 1 px estado, texto estado (12 SemiBold) |
| borda de vidro | 1 px EDGE |
| fio de luz | 2 px SHINE, largura = celula - 2*raio, no topo interno |
| padding da celula | 10 px |
| gap da grade | 4 px (fixo pelo hardware/case) |
| alvo de toque minimo | 44 px (celulas 101x115; botoes da faixa 96x40 + ext_click_area) |

## 3. Tipografia (Montserrat; ASCII + ° + •) — hierarquia por PESO (tema Stitch)

Fontes reais geradas por `npx lv_font_conv` de `assets/fonts/Montserrat-{SemiBold,Bold}.ttf`
(OFL) em `firmware/clow_deck/font_ms_*.c` (`fonts_theme.h`): SemiBold 12/14/18, Bold 40.
Rotulos de botao em MAIUSCULAS com letter-spacing 1 (`upcase_lbl`); passkey com spacing 3.

| papel | fonte |
|---|---|
| rotulo da celula-botao | **SemiBold 8** maiusculo (sessao: **SemiBold 10**, embaixo, centralizado) |
| chip / idade / rodape | montserrat 12 |
| titulo de faixa / overlays | **SemiBold 18** |
| titulo de overlay | montserrat 20 |
| valor grande (brilho, modo) | montserrat 24 |
| passkey | **Bold 40** + letter-spacing 3 |

## 4. Regra da grade (todas as telas — compatibilidade com o case 3D)

Fundo: gradiente `C_BG_TOP`->`C_BG_BOTTOM` + "ceu estrelado" (`ic_bgtile_1` ladrilhado,
recolorido no acento, opa 22) em todas as telas.

Tela 320x480 (retrato) em 4 linhas com gap 4 px:
- **linhas 0-2 = 9 celulas 101x115** em `x = 4 + col*105`, `y = 4 + row*119` (col 0..2,
  row 0..2). Linhas 0-1 = sessoes, linha 2 = utilitarios. Posicoes e tamanhos
  identicos em TODAS as telas.
- **linha 3 = faixa livre 312x115** em `(4, 361)`: na HOME e a marca (mascote @4 +
  wordmark CLOW DECK; alertas so pelos icones bt/stale do canto); nas demais telas,
  botoes em layout livre ou duas sub-linhas de informacao.
- O case 3D so tem paredes nos gaps das linhas 0-2; a faixa fica aberta.

## 5. Mascote e icones

Assets A8 (`icons.h`, gerado por `tools/gen_icons.py`), coloridos em runtime
(`lv_obj_set_style_image_recolor` + `recolor_opa 255`) e esmaecidos com `image_opa`.
- Fundo das celulas de sessao: logo do ENGINE (spark Claude Code / no Codex), tingido
  pelo estado — o Clow e a marca do app (SEARCH, faixa da home, ABOUT, icones do app).
- Mascote "Clow" (pixel-art, `assets/pixel`): invasor classico 24x19 decodificado do SVG
  de referencia do usuario (2026-08-24; cores da marca), 2 frames (pes), olhos = furos 3x3
  que as palpebras cobrem ao piscar. Escalas: @4 = 96x76 (SEARCH, ABOUT), @2 = 48x38
  (celulas de sessao, faixas).
- Icones de botao: **outline com anti-aliasing** (`assets/icons_vec.py`, grade 24x24,
  traco 2.1, pontas redondas; ref. icon packs de Stream Deck), emitidos a 40 px
  (`ic_*_4`, celulas) e 16 px (`ic_*_2`, faixa/status). Sufixo = tag de tamanho.
  Catalogo em `ICONS.md`.

## 6. Deck virtual (web.rs) espelha este tema

A pagina em `http://127.0.0.1:47831/` usa os MESMOS tokens (fundo OLED + estrelas,
celulas flat com contorno/glow por estado, badges, rotulos maiusculos Montserrat,
faixa = marca CLOW DECK). As marcas d'agua dos engines sao os proprios A8 da placa
convertidos em PNG branco e aplicados via CSS `mask` com `background` na cor do
estado — mesmo mecanismo de recolor do firmware. Ao mudar tokens aqui, atualizar o
`<style>` do `INDEX_HTML` junto.

## 7. Movimento (procedural no `loop()`, nunca `lv_anim`)

- tick de animacao: 100 ms (so escreve estilo quando a cor muda)
- mascote SEARCH: alterna frames a cada 600 ms, flutua +-3 px (seno, 600 ms), pisca
  (150 ms fechado a cada ~3,2 s)
- celulas: pulso WORKING, pisca ATTENTION, pisca lento DONE (< 60 s)
