/**
 * lv_conf.h — config do LVGL 9.2 para o Clow Deck (JC4832W535).
 *
 * Compile com -DLV_CONF_INCLUDE_SIMPLE -I<sketch> nos TRES recipes
 * (c, cpp e S — ver build.sh). Arquivo parcial: o que nao estiver aqui usa o
 * default do lv_conf_internal.h.
 */
#ifndef LV_CONF_H
#define LV_CONF_H

/* Guarda obrigatoria: lv_conf_internal.h inclui este arquivo tambem durante a
   montagem dos .S do LVGL (lv_blend_helium.S / lv_blend_neon.S). Sem a guarda,
   o assembler recebe os typedef de stdint.h e falha com
   "unknown opcode or format name 'typedef'". */
#ifndef __ASSEMBLY__
#include <stdint.h>
#endif

/*====================
   COLOR
 *====================*/
#define LV_COLOR_DEPTH 16
#define LV_COLOR_16_SWAP 0

/*=========================
   MEMORIA — pool interno (objetos/estilos). O buffer de render full-screen
   (480x320x2) e alocado a parte na PSRAM, dentro do sketch.
 *=========================*/
#define LV_USE_STDLIB_MALLOC   LV_STDLIB_BUILTIN
#define LV_USE_STDLIB_STRING   LV_STDLIB_BUILTIN
#define LV_USE_STDLIB_SPRINTF  LV_STDLIB_BUILTIN
#define LV_MEM_SIZE            (96 * 1024U)

/*====================
   HAL / SISTEMA
 *====================*/
#define LV_USE_OS LV_OS_NONE
/* tick vem de lv_tick_set_cb(millis) no sketch */

/*====================
   RENDER
 *====================*/
#define LV_USE_DRAW_SW 1

/*====================
   FONTES (Montserrat: ASCII + simbolos FontAwesome embutidos; sem acentos)
 *====================*/
#define LV_FONT_MONTSERRAT_12 1
#define LV_FONT_MONTSERRAT_14 1
#define LV_FONT_MONTSERRAT_16 1
#define LV_FONT_MONTSERRAT_18 1
#define LV_FONT_MONTSERRAT_20 1
#define LV_FONT_MONTSERRAT_24 1
#define LV_FONT_MONTSERRAT_28 1
#define LV_FONT_MONTSERRAT_40 1
#define LV_FONT_DEFAULT &lv_font_montserrat_14

/*====================
   WIDGETS usados
 *====================*/
#define LV_USE_LABEL        1
#define LV_USE_BUTTON       1
#define LV_USE_BAR          1
#define LV_USE_IMAGE        1
#define LV_USE_LINE         1
#define LV_USE_SPINNER      1
#define LV_USE_ARC          1

#endif /*LV_CONF_H*/
