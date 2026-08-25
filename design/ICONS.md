# Catalogo de icones — outline (estilo icon-pack) + mascote pixel-art

Duas familias, ambas A8 recoloriveis em runtime (`lv_obj_set_style_image_recolor`):

1. **Icones de botao — outline**: spec vetorial em `assets/icons_vec.py` (grade de
   design 24x24, y para baixo), rasterizada por `tools/gen_icons.py` com Pillow
   (supersample 8x + LANCZOS => anti-aliasing). Emitidos em dois tamanhos:
   `ic_<nome>_4` = **40 px** (celulas) e `ic_<nome>_2` = **16 px** (faixa/status).
   O sufixo eh **tag de tamanho** (herdada dos call sites), nao fator de escala.
2. **Mascote — pixel-art**: `assets/pixel/clow_{a,b}.txt` (grade 24x19, decodificada
   do SVG de referencia do usuario; alpha binario, nearest-neighbor @2/@4) — identidade da marca, **nao migra** para
   outline.

Regras do tema outline (referencia: icon packs de Stream Deck):
- grade 24x24 com margem ~2 px; traco 2.1 unidades, pontas e juncoes **redondas**;
- monocromatico — a cor vem sempre do recolor (estado/acento), nunca do asset;
- formas fechadas e silhueta legivel a 16 px (testar na folha de contato);
- primitivas disponiveis: `line`, `pline`, `circle`, `ellipse`, `arc`, `rrect`,
  `dot`, `arrowhead` (ver cabecalho de `assets/icons_vec.py`).

O gerador tambem emite `ic_bgtile_1`: tile de fundo 160x160 (pontos + brilhos de 4
pontas, sem emenda, seed fixa em `sparkle_tile()`), ladrilhado atras de todas as telas
e recolorido no acento a baixa opacidade.

Regenerar (da raiz do repo): `python3 tools/gen_icons.py`

| nome | uso | desenho |
|---|---|---|
| `clow_a`, `clow_b` | mascote/marca do app (SEARCH, faixa da home, ABOUT) | invasor classico 24x19 exato do SVG do usuario (2026-08-24), olhos-furo 3x3 |
| `claude` | fundo das celulas de sessao Claude | logo oficial Claude Code (raster `assets/logos/claudecode.webp`, mascara por distancia do fundo) |
| `codexlogo` | fundo das celulas de sessao Codex | logo oficial Codex (raster `assets/logos/codex.png`, mascara = alpha) |
| `back` | voltar (`<`) | chevron esquerda |
| `focus` | Focar | alvo: anel + ponto + 4 tiques |
| `voice` | Voz (push-to-talk) | microfone com arco e base |
| `mode` | Modo (Shift+Tab) | duas setas em ciclo |
| `esc` | Esc | x |
| `enter` | Enter | seta de retorno |
| `compact` | /compact | setas convergindo na linha central |
| `clear` | /clear | lixeira |
| `ack` | (sem botao dedicado; tap na celula DONE ja faz ACK) | check |
| `tab` | Tab (aceita a sugestao do terminal) | seta ate a barra |
| `exit` | encerrar a sessao (pagina de comandos, com confirmacao) | porta aberta com seta saindo |
| `plus` | nova sessao (CLEAR mapeado por engine) | sinal de mais |
| `cmd` | comandos | terminal emoldurado (moldura + chevron + underscore, ref. Material/Stitch) |
| `lang` | idioma | globo (equador + meridiano) |
| `bright` | brilho | sol (8 raios) |
| `gear` | ajustes | engrenagem (anel + 8 dentes + furo) |
| `bt` | bluetooth | runa BT |
| `warn` | atencao | triangulo com `!` |
| `stale` | agente parado | ampulheta |
| `next` | proxima pagina (`>`) | chevron direita |
