# Spec vetorial dos icones outline do Clow Deck.
# Grade de design 24x24 (y para baixo), traco padrao 2.1 unidades, pontas e
# juncoes redondas. Angulos em graus no sentido horario a partir de 3h (PIL).
# Primitivas:
#   ("line", x1,y1,x2,y2)
#   ("pline", [(x,y), ...])            # polilinha, juncoes redondas
#   ("circle", cx,cy,r)                # circulo em traco
#   ("ellipse", cx,cy,rx,ry)           # elipse em traco
#   ("arc", cx,cy,r,a0,a1)             # arco em traco, pontas redondas
#   ("rrect", x,y,w,h,r)               # retangulo arredondado em traco
#   ("dot", cx,cy,r)                   # circulo preenchido
#   ("arrowhead", tipx,tipy,dir,len)   # chevron de seta apontando para dir
# Consumido por tools/gen_icons.py (rasteriza com Pillow, supersample + LANCZOS).

ICONS = {
    # Focar — alvo: anel, ponto central, 4 tiques
    "focus": {"prims": [
        ("circle", 12, 12, 6.8),
        ("dot", 12, 12, 2.0),
        ("line", 12, 1.8, 12, 4.6), ("line", 12, 19.4, 12, 22.2),
        ("line", 1.8, 12, 4.6, 12), ("line", 19.4, 12, 22.2, 12),
    ]},
    # Modo (Shift+Tab) — duas setas em ciclo
    "mode": {"prims": [
        ("arc", 12, 12, 7.2, 190, 335),
        ("arrowhead", 18.52, 8.96, 65, 3.2),
        ("arc", 12, 12, 7.2, 10, 155),
        ("arrowhead", 5.48, 15.04, 245, 3.2),
    ]},
    # Esc — x
    "esc": {"prims": [
        ("line", 6.5, 6.5, 17.5, 17.5),
        ("line", 17.5, 6.5, 6.5, 17.5),
    ]},
    # Enter — seta de retorno
    "enter": {"prims": [
        ("pline", [(18.5, 7), (18.5, 14), (6.2, 14)]),
        ("arrowhead", 6.2, 14, 180, 3.4),
    ]},
    # /compact — setas convergindo na linha central
    "compact": {"prims": [
        ("line", 12, 3, 12, 9.2), ("arrowhead", 12, 9.2, 90, 3.2),
        ("line", 12, 21, 12, 14.8), ("arrowhead", 12, 14.8, 270, 3.2),
        ("line", 3.8, 12, 6.6, 12), ("line", 10.6, 12, 13.4, 12),
        ("line", 17.4, 12, 20.2, 12),
    ]},
    # /clear — lixeira
    "clear": {"prims": [
        ("line", 4.8, 7, 19.2, 7),
        ("pline", [(9.3, 6.8), (9.3, 4.2), (14.7, 4.2), (14.7, 6.8)]),
        ("pline", [(6.6, 7.2), (7.6, 20), (16.4, 20), (17.4, 7.2)]),
        ("line", 10.6, 10.8, 10.6, 16.2), ("line", 13.4, 10.8, 13.4, 16.2),
    ]},
    # comandos — terminal emoldurado (ref. Material 'terminal' do tema Stitch)
    "cmd": {"prims": [
        ("rrect", 2.6, 4.6, 18.8, 14.8, 3.4),
        ("pline", [(6.4, 9.2), (9.6, 12), (6.4, 14.8)]),
        ("line", 12.4, 14.8, 16.6, 14.8),
    ]},
    # Voz (push-to-talk) — microfone
    "voice": {"prims": [
        ("rrect", 9.2, 3.2, 5.6, 11.0, 2.8),
        ("arc", 12, 11.2, 5.6, 0, 180),
        ("line", 12, 16.8, 12, 20.3),
        ("line", 8.6, 20.8, 15.4, 20.8),
    ]},
    # voltar — chevron esquerda
    "back": {"prims": [
        ("pline", [(14.8, 5.2), (8.2, 12), (14.8, 18.8)]),
    ]},
    # proxima pagina — chevron direita
    "next": {"prims": [
        ("pline", [(9.2, 5.2), (15.8, 12), (9.2, 18.8)]),
    ]},
    # Lida (ACK) — check
    "ack": {"prims": [
        ("pline", [(4.6, 12.6), (9.6, 17.6), (19.4, 7.2)]),
    ]},
    # Tab — aceitar a sugestao do terminal (seta ate a barra)
    "tab": {"prims": [
        ("line", 3.8, 12, 15.6, 12),
        ("arrowhead", 15.6, 12, 0, 3.4),
        ("line", 20.2, 6.6, 20.2, 17.4),
    ]},
    # atencao — triangulo com !
    "warn": {"prims": [
        ("pline", [(12, 3.6), (21.2, 19.6), (2.8, 19.6), (12, 3.6)]),
        ("line", 12, 9.4, 12, 13.6),
        ("dot", 12, 16.9, 1.5),
    ]},
    # brilho — sol
    "bright": {"prims": [
        ("circle", 12, 12, 3.8),
        ("line", 12, 2.4, 12, 5.6), ("line", 12, 18.4, 12, 21.6),
        ("line", 2.4, 12, 5.6, 12), ("line", 18.4, 12, 21.6, 12),
        ("line", 16.53, 7.47, 18.79, 5.21), ("line", 16.53, 16.53, 18.79, 18.79),
        ("line", 7.47, 16.53, 5.21, 18.79), ("line", 7.47, 7.47, 5.21, 5.21),
    ]},
    # idioma — globo
    "lang": {"prims": [
        ("circle", 12, 12, 8.6),
        ("line", 3.4, 12, 20.6, 12),
        ("ellipse", 12, 12, 4.1, 8.6),
    ]},
    # ajustes — engrenagem (anel + 8 dentes + furo)
    "gear": {"prims": [
        ("circle", 12, 12, 3.1),
        ("circle", 12, 12, 7.0),
        ("line", 19.0, 12, 22.2, 12), ("line", 1.8, 12, 5.0, 12),
        ("line", 12, 19.0, 12, 22.2), ("line", 12, 1.8, 12, 5.0),
        ("line", 16.95, 16.95, 19.21, 19.21), ("line", 7.05, 7.05, 4.79, 4.79),
        ("line", 16.95, 7.05, 19.21, 4.79), ("line", 7.05, 16.95, 4.79, 19.21),
    ]},
    # bluetooth — runa BT
    "bt": {"prims": [
        ("pline", [(6.8, 7.2), (16.6, 16.6), (12, 20.8), (12, 3.2),
                   (16.6, 7.4), (6.8, 16.8)]),
    ]},
    # agente parado — ampulheta
    "stale": {"prims": [
        ("line", 6.6, 3.6, 17.4, 3.6),
        ("line", 6.6, 20.4, 17.4, 20.4),
        ("pline", [(8.2, 3.6), (8.2, 7.6), (12, 12), (8.2, 16.4), (8.2, 20.4)]),
        ("pline", [(15.8, 3.6), (15.8, 7.6), (12, 12), (15.8, 16.4), (15.8, 20.4)]),
        ("dot", 12, 17.8, 1.3),
    ]},
}
