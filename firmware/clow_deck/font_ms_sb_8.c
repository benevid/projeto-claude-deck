/*******************************************************************************
 * Size: 8 px
 * Bpp: 4
 * Opts: --font assets/fonts/Montserrat-SemiBold.ttf --size 8 --bpp 4 --format lvgl --range 0x20-0x7E,0xB0,0x2022 --no-compress -o firmware/clow_deck/font_ms_sb_8.c
 ******************************************************************************/

#ifdef LV_LVGL_H_INCLUDE_SIMPLE
#include "lvgl.h"
#else
#include "lvgl/lvgl.h"
#endif

#ifndef FONT_MS_SB_8
#define FONT_MS_SB_8 1
#endif

#if FONT_MS_SB_8

/*-----------------
 *    BITMAPS
 *----------------*/

/*Store the image of the glyphs*/
static LV_ATTRIBUTE_LARGE_CONST const uint8_t glyph_bitmap[] = {
    /* U+0020 " " */

    /* U+0021 "!" */
    0x7b, 0x6a, 0x59, 0x24, 0x69,

    /* U+0022 "\"" */
    0x84, 0xd8, 0x4c, 0x0, 0x0,

    /* U+0023 "#" */
    0x4, 0x63, 0x80, 0x6d, 0xcd, 0xc5, 0x9, 0x27,
    0x30, 0x8e, 0xad, 0xb2, 0xb, 0xb, 0x0,

    /* U+0024 "$" */
    0x0, 0xa0, 0x2, 0xce, 0xc4, 0x89, 0xa0, 0x1,
    0xaf, 0xa2, 0x10, 0xa7, 0xa6, 0xce, 0xc4, 0x0,
    0xa0, 0x0,

    /* U+0025 "%" */
    0x69, 0x80, 0x65, 0xa, 0xa, 0x39, 0x0, 0x37,
    0x6a, 0x67, 0x20, 0xa, 0x2a, 0x18, 0x7, 0x40,
    0x9a, 0x40,

    /* U+0026 "&" */
    0xa, 0xbb, 0x0, 0xd, 0x3c, 0x0, 0x1b, 0xe5,
    0x41, 0x96, 0x1b, 0xe0, 0x3c, 0xcb, 0xa3,

    /* U+0027 "'" */
    0x84, 0x84, 0x0,

    /* U+0028 "(" */
    0x9, 0x60, 0xe0, 0x3c, 0x4, 0xa0, 0x3c, 0x0,
    0xe0, 0x9, 0x60,

    /* U+0029 ")" */
    0x95, 0x2, 0xc0, 0xf, 0x0, 0xe1, 0xf, 0x2,
    0xc0, 0x95, 0x0,

    /* U+002A "*" */
    0x5a, 0x50, 0x7f, 0xb0, 0x28, 0x20,

    /* U+002B "+" */
    0x0, 0x40, 0x0, 0x1c, 0x0, 0x5c, 0xfc, 0x20,
    0x1c, 0x0,

    /* U+002C "," */
    0x11, 0x88, 0x73,

    /* U+002D "-" */
    0x7c, 0x80,

    /* U+002E "." */
    0x11, 0x87,

    /* U+002F "/" */
    0x0, 0xc, 0x10, 0x2, 0xb0, 0x0, 0x85, 0x0,
    0xd, 0x0, 0x4, 0x90, 0x0, 0xa3, 0x0, 0x1d,
    0x0, 0x0,

    /* U+0030 "0" */
    0xa, 0xdd, 0x30, 0x7a, 0x4, 0xd0, 0x96, 0x0,
    0xf0, 0x7a, 0x4, 0xd0, 0xa, 0xdd, 0x30,

    /* U+0031 "1" */
    0xbf, 0x40, 0xc4, 0xc, 0x40, 0xc4, 0xc, 0x40,

    /* U+0032 "2" */
    0x7c, 0xda, 0x2, 0x0, 0xe2, 0x0, 0x7a, 0x0,
    0xa9, 0x0, 0xaf, 0xdd, 0x50,

    /* U+0033 "3" */
    0x9c, 0xdf, 0x10, 0x1c, 0x40, 0x5, 0xcb, 0x2,
    0x0, 0xd4, 0x9d, 0xea, 0x0,

    /* U+0034 "4" */
    0x0, 0x6b, 0x0, 0x4, 0xc0, 0x0, 0x2d, 0x29,
    0x50, 0xad, 0xde, 0xe4, 0x0, 0xa, 0x50,

    /* U+0035 "5" */
    0x3e, 0xcc, 0x15, 0x90, 0x0, 0x6d, 0xda, 0x11,
    0x0, 0xc5, 0x7d, 0xeb, 0x10,

    /* U+0036 "6" */
    0x9, 0xdd, 0x56, 0xb0, 0x0, 0x9b, 0xcc, 0x37,
    0xa0, 0x4c, 0xa, 0xcc, 0x40,

    /* U+0037 "7" */
    0xcd, 0xde, 0x88, 0x20, 0xe2, 0x0, 0x7a, 0x0,
    0xe, 0x20, 0x7, 0xa0, 0x0,

    /* U+0038 "8" */
    0x2b, 0xcc, 0x36, 0xa0, 0x6a, 0x2f, 0xcf, 0x59,
    0x70, 0x4d, 0x3c, 0xcc, 0x50,

    /* U+0039 "9" */
    0x4c, 0xba, 0xb, 0x40, 0xa7, 0x3c, 0xbb, 0x90,
    0x0, 0xb6, 0x5d, 0xd9, 0x0,

    /* U+003A ":" */
    0x87, 0x10, 0x11, 0x87,

    /* U+003B ";" */
    0x87, 0x10, 0x11, 0x88, 0x73,

    /* U+003C "<" */
    0x0, 0x1, 0x0, 0x5b, 0xa1, 0x6e, 0x40, 0x0,
    0x28, 0xb2, 0x0, 0x0, 0x0,

    /* U+003D "=" */
    0x5b, 0xbb, 0x20, 0x0, 0x0, 0x5b, 0xbb, 0x20,

    /* U+003E ">" */
    0x10, 0x0, 0x4, 0xb9, 0x40, 0x0, 0x7f, 0x25,
    0xb6, 0x10, 0x0, 0x0, 0x0,

    /* U+003F "?" */
    0x7c, 0xda, 0x2, 0x0, 0xf1, 0x0, 0xc5, 0x0,
    0x4, 0x0, 0x3, 0xc0, 0x0,

    /* U+0040 "@" */
    0x3, 0x99, 0x99, 0x50, 0x2a, 0x4b, 0xab, 0x95,
    0x92, 0xd1, 0xd, 0x2b, 0x92, 0xd0, 0xd, 0x2a,
    0x2a, 0x5c, 0xb9, 0xc6, 0x3, 0xa9, 0x96, 0x0,

    /* U+0041 "A" */
    0x0, 0xc, 0xc0, 0x0, 0x0, 0x4b, 0xb4, 0x0,
    0x0, 0xc3, 0x4c, 0x0, 0x4, 0xfc, 0xcf, 0x40,
    0xc, 0x40, 0x4, 0xc0,

    /* U+0042 "B" */
    0x4f, 0xcc, 0xc1, 0x4c, 0x0, 0xb5, 0x4f, 0xcc,
    0xe2, 0x4c, 0x0, 0x6a, 0x4f, 0xcc, 0xd4,

    /* U+0043 "C" */
    0x7, 0xdd, 0xc2, 0x5c, 0x10, 0x20, 0x97, 0x0,
    0x0, 0x5c, 0x10, 0x20, 0x7, 0xdd, 0xc2,

    /* U+0044 "D" */
    0x4f, 0xdd, 0xc3, 0x4, 0xc0, 0x4, 0xe0, 0x4c,
    0x0, 0xd, 0x34, 0xc0, 0x4, 0xe0, 0x4f, 0xdd,
    0xc3, 0x0,

    /* U+0045 "E" */
    0x4f, 0xcc, 0xa4, 0xc0, 0x0, 0x4f, 0xcc, 0x64,
    0xc0, 0x0, 0x4f, 0xdd, 0xc0,

    /* U+0046 "F" */
    0x4f, 0xcc, 0xa4, 0xc0, 0x0, 0x4f, 0xdd, 0x64,
    0xc0, 0x0, 0x4c, 0x0, 0x0,

    /* U+0047 "G" */
    0x7, 0xdd, 0xc3, 0x5c, 0x10, 0x21, 0x97, 0x0,
    0x35, 0x5c, 0x10, 0x69, 0x7, 0xdd, 0xc4,

    /* U+0048 "H" */
    0x4c, 0x0, 0x5b, 0x4c, 0x0, 0x5b, 0x4f, 0xdd,
    0xdb, 0x4c, 0x0, 0x5b, 0x4c, 0x0, 0x5b,

    /* U+0049 "I" */
    0x4c, 0x4c, 0x4c, 0x4c, 0x4c,

    /* U+004A "J" */
    0x6, 0xce, 0x80, 0x0, 0x88, 0x0, 0x8, 0x80,
    0x20, 0x97, 0xa, 0xdd, 0x20,

    /* U+004B "K" */
    0x4c, 0x1, 0xc4, 0x4c, 0x2d, 0x30, 0x4e, 0xea,
    0x0, 0x4e, 0x2b, 0x80, 0x4c, 0x0, 0xc6,

    /* U+004C "L" */
    0x4c, 0x0, 0x4, 0xc0, 0x0, 0x4c, 0x0, 0x4,
    0xc0, 0x0, 0x4f, 0xdd, 0x90,

    /* U+004D "M" */
    0x4e, 0x0, 0x5, 0xd4, 0xf9, 0x1, 0xde, 0x4c,
    0xb4, 0xa7, 0xe4, 0xc1, 0xea, 0x1e, 0x4c, 0x3,
    0x11, 0xe0,

    /* U+004E "N" */
    0x4e, 0x20, 0x4b, 0x4f, 0xd1, 0x4b, 0x4c, 0x6d,
    0x5b, 0x4c, 0x7, 0xeb, 0x4c, 0x0, 0x9b,

    /* U+004F "O" */
    0x7, 0xdd, 0xc4, 0x5, 0xc1, 0x3, 0xe1, 0x97,
    0x0, 0xb, 0x55, 0xc1, 0x3, 0xe1, 0x7, 0xdd,
    0xc4, 0x0,

    /* U+0050 "P" */
    0x4f, 0xdc, 0x90, 0x4c, 0x0, 0xb5, 0x4c, 0x0,
    0xc5, 0x4f, 0xdc, 0x70, 0x4c, 0x0, 0x0,

    /* U+0051 "Q" */
    0x5, 0xcd, 0xc3, 0x4, 0xd1, 0x4, 0xe1, 0x97,
    0x0, 0xc, 0x48, 0x90, 0x0, 0xd4, 0x2e, 0x63,
    0x9c, 0x0, 0x29, 0xfb, 0x11, 0x0, 0x2, 0xbb,
    0x40,

    /* U+0052 "R" */
    0x4f, 0xdd, 0x90, 0x4c, 0x0, 0xc5, 0x4c, 0x0,
    0xc5, 0x4f, 0xce, 0x90, 0x4c, 0x2, 0xc2,

    /* U+0053 "S" */
    0x2c, 0xcc, 0x48, 0x90, 0x0, 0x19, 0xca, 0x21,
    0x0, 0x6a, 0x5c, 0xdd, 0x40,

    /* U+0054 "T" */
    0xcd, 0xfd, 0xa0, 0x1e, 0x0, 0x1, 0xe0, 0x0,
    0x1e, 0x0, 0x1, 0xe0, 0x0,

    /* U+0055 "U" */
    0x4c, 0x0, 0x69, 0x4c, 0x0, 0x69, 0x4c, 0x0,
    0x69, 0x2e, 0x0, 0xa7, 0x7, 0xdd, 0xa0,

    /* U+0056 "V" */
    0xc, 0x50, 0x6, 0xa0, 0x5d, 0x0, 0xd2, 0x0,
    0xd4, 0x6a, 0x0, 0x5, 0xcd, 0x20, 0x0, 0xd,
    0xa0, 0x0,

    /* U+0057 "W" */
    0x97, 0x1, 0xf4, 0x4, 0xb3, 0xd0, 0x6d, 0xa0,
    0x96, 0xd, 0x3d, 0x2e, 0x1e, 0x0, 0x7c, 0xc0,
    0x9b, 0xa0, 0x1, 0xf6, 0x3, 0xf4, 0x0,

    /* U+0058 "X" */
    0x6b, 0x3, 0xd1, 0x9, 0xad, 0x20, 0x1, 0xf9,
    0x0, 0xb, 0x7d, 0x40, 0x8a, 0x2, 0xe2,

    /* U+0059 "Y" */
    0xb, 0x60, 0x1d, 0x10, 0x1e, 0x1a, 0x50, 0x0,
    0x6d, 0xa0, 0x0, 0x0, 0xe3, 0x0, 0x0, 0xe,
    0x20, 0x0,

    /* U+005A "Z" */
    0x7d, 0xde, 0xf0, 0x0, 0x1d, 0x30, 0x1, 0xd4,
    0x0, 0xc, 0x50, 0x0, 0x9f, 0xdd, 0xd1,

    /* U+005B "[" */
    0x4e, 0x74, 0xc0, 0x4c, 0x4, 0xc0, 0x4c, 0x4,
    0xc0, 0x4e, 0x70,

    /* U+005C "\\" */
    0x2c, 0x0, 0x0, 0xc2, 0x0, 0x6, 0x80, 0x0,
    0xd, 0x0, 0x0, 0xa4, 0x0, 0x4, 0xa0, 0x0,
    0xd, 0x10,

    /* U+005D "]" */
    0xaf, 0x0, 0xf0, 0xf, 0x0, 0xf0, 0xf, 0x0,
    0xf0, 0xaf, 0x0,

    /* U+005E "^" */
    0x4, 0xe1, 0x0, 0xb5, 0x70, 0x38, 0xb, 0x0,

    /* U+005F "_" */
    0x99, 0x99,

    /* U+0060 "`" */
    0x7, 0x80,

    /* U+0061 "a" */
    0x3b, 0xca, 0x2, 0xa9, 0xe3, 0x96, 0xd, 0x34,
    0xdb, 0xe3,

    /* U+0062 "b" */
    0x5a, 0x0, 0x0, 0x5a, 0x0, 0x0, 0x5d, 0xcd,
    0x60, 0x5c, 0x1, 0xf1, 0x5c, 0x1, 0xf1, 0x5d,
    0xcd, 0x60,

    /* U+0063 "c" */
    0x1b, 0xcc, 0x19, 0x70, 0x20, 0x97, 0x2, 0x1,
    0xbc, 0xc1,

    /* U+0064 "d" */
    0x0, 0x2, 0xd0, 0x0, 0x2d, 0x1b, 0xcc, 0xd9,
    0x70, 0x5d, 0x97, 0x5, 0xd1, 0xbc, 0xbd,

    /* U+0065 "e" */
    0x1a, 0xbb, 0x29, 0xb9, 0xa8, 0x99, 0x2, 0x1,
    0xbc, 0xc2,

    /* U+0066 "f" */
    0xb, 0xc1, 0x2d, 0x0, 0xbf, 0xb0, 0x3c, 0x0,
    0x3c, 0x0, 0x3c, 0x0,

    /* U+0067 "g" */
    0x1b, 0xcb, 0xe9, 0x70, 0x4e, 0x97, 0x4, 0xe1,
    0xbc, 0xcd, 0x2a, 0xcc, 0x40,

    /* U+0068 "h" */
    0x5a, 0x0, 0x5, 0xa0, 0x0, 0x5d, 0xcd, 0x55,
    0xc0, 0x3c, 0x5a, 0x2, 0xd5, 0xa0, 0x2d,

    /* U+0069 "i" */
    0x5a, 0x0, 0x5a, 0x5a, 0x5a, 0x5a,

    /* U+006A "j" */
    0x4, 0xa0, 0x0, 0x4, 0xb0, 0x4b, 0x4, 0xb0,
    0x5b, 0x7d, 0x50,

    /* U+006B "k" */
    0x5a, 0x0, 0x0, 0x5a, 0x0, 0x0, 0x5a, 0x1c,
    0x60, 0x5c, 0xd6, 0x0, 0x5e, 0x8c, 0x0, 0x5a,
    0x8, 0xa0,

    /* U+006C "l" */
    0x5a, 0x5a, 0x5a, 0x5a, 0x5a, 0x5a,

    /* U+006D "m" */
    0x5d, 0xbd, 0x8c, 0xd5, 0x5c, 0x5, 0xd0, 0x4c,
    0x5a, 0x4, 0xb0, 0x3c, 0x5a, 0x4, 0xb0, 0x3c,

    /* U+006E "n" */
    0x5d, 0xbd, 0x55, 0xc0, 0x3c, 0x5a, 0x2, 0xd5,
    0xa0, 0x2d,

    /* U+006F "o" */
    0x1b, 0xcc, 0x29, 0x70, 0x5c, 0x97, 0x5, 0xc1,
    0xbc, 0xc2,

    /* U+0070 "p" */
    0x5d, 0xcc, 0x60, 0x5c, 0x0, 0xe1, 0x5c, 0x1,
    0xf1, 0x5d, 0xcd, 0x60, 0x5a, 0x0, 0x0,

    /* U+0071 "q" */
    0x1b, 0xcb, 0xd9, 0x70, 0x5d, 0x97, 0x5, 0xd1,
    0xbc, 0xcd, 0x0, 0x2, 0xd0,

    /* U+0072 "r" */
    0x5d, 0xc1, 0x5c, 0x0, 0x5a, 0x0, 0x5a, 0x0,

    /* U+0073 "s" */
    0x5c, 0xc7, 0x9a, 0x40, 0x4, 0x8c, 0x8c, 0xc8,

    /* U+0074 "t" */
    0x3c, 0x0, 0xbf, 0xb0, 0x3c, 0x0, 0x3d, 0x0,
    0xc, 0xc1,

    /* U+0075 "u" */
    0x6a, 0x3, 0xc6, 0xa0, 0x3c, 0x5b, 0x5, 0xc0,
    0xcd, 0xbc,

    /* U+0076 "v" */
    0xd, 0x30, 0x86, 0x5, 0xa0, 0xd0, 0x0, 0xd9,
    0x80, 0x0, 0x6f, 0x10,

    /* U+0077 "w" */
    0xc2, 0xe, 0x50, 0xc1, 0x68, 0x5c, 0xb2, 0xb0,
    0xd, 0xb2, 0xca, 0x50, 0x9, 0xc0, 0x6e, 0x0,

    /* U+0078 "x" */
    0x7a, 0x2d, 0x10, 0xae, 0x30, 0xa, 0xe4, 0x8,
    0x91, 0xd2,

    /* U+0079 "y" */
    0xc, 0x40, 0x86, 0x4, 0xc1, 0xd0, 0x0, 0xbc,
    0x60, 0x0, 0x5d, 0x0, 0xc, 0xc4, 0x0,

    /* U+007A "z" */
    0x7c, 0xdd, 0x1, 0xd2, 0xc, 0x40, 0x9e, 0xbb,

    /* U+007B "{" */
    0xa, 0xa0, 0xe1, 0xf, 0x7, 0xd0, 0xf, 0x0,
    0xe1, 0xa, 0xa0,

    /* U+007C "|" */
    0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a,

    /* U+007D "}" */
    0xb9, 0x1, 0xe0, 0x1e, 0x0, 0xe6, 0x1e, 0x1,
    0xe0, 0xb9, 0x0,

    /* U+007E "~" */
    0x2b, 0x77, 0x36, 0x28, 0xb0,

    /* U+00B0 "°" */
    0x28, 0x50, 0x90, 0x90, 0x28, 0x50,

    /* U+2022 "•" */
    0x27, 0x5, 0xe1
};


/*---------------------
 *  GLYPH DESCRIPTION
 *--------------------*/

static const lv_font_fmt_txt_glyph_dsc_t glyph_dsc[] = {
    {.bitmap_index = 0, .adv_w = 0, .box_w = 0, .box_h = 0, .ofs_x = 0, .ofs_y = 0} /* id = 0 reserved */,
    {.bitmap_index = 0, .adv_w = 35, .box_w = 0, .box_h = 0, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 0, .adv_w = 36, .box_w = 2, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 5, .adv_w = 53, .box_w = 3, .box_h = 3, .ofs_x = 0, .ofs_y = 2},
    {.bitmap_index = 10, .adv_w = 91, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 25, .adv_w = 81, .box_w = 5, .box_h = 7, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 43, .adv_w = 110, .box_w = 7, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 61, .adv_w = 90, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 76, .adv_w = 28, .box_w = 2, .box_h = 3, .ofs_x = 0, .ofs_y = 2},
    {.bitmap_index = 79, .adv_w = 44, .box_w = 3, .box_h = 7, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 90, .adv_w = 45, .box_w = 3, .box_h = 7, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 101, .adv_w = 53, .box_w = 4, .box_h = 3, .ofs_x = 0, .ofs_y = 3},
    {.bitmap_index = 107, .adv_w = 76, .box_w = 5, .box_h = 4, .ofs_x = 0, .ofs_y = 1},
    {.bitmap_index = 117, .adv_w = 31, .box_w = 2, .box_h = 3, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 120, .adv_w = 49, .box_w = 3, .box_h = 1, .ofs_x = 0, .ofs_y = 2},
    {.bitmap_index = 122, .adv_w = 31, .box_w = 2, .box_h = 2, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 124, .adv_w = 47, .box_w = 5, .box_h = 7, .ofs_x = -1, .ofs_y = -1},
    {.bitmap_index = 142, .adv_w = 86, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 157, .adv_w = 49, .box_w = 3, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 165, .adv_w = 74, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 178, .adv_w = 74, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 191, .adv_w = 87, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 206, .adv_w = 75, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 219, .adv_w = 80, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 232, .adv_w = 78, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 245, .adv_w = 83, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 258, .adv_w = 80, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 271, .adv_w = 31, .box_w = 2, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 275, .adv_w = 31, .box_w = 2, .box_h = 5, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 280, .adv_w = 76, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 293, .adv_w = 76, .box_w = 5, .box_h = 3, .ofs_x = 0, .ofs_y = 1},
    {.bitmap_index = 301, .adv_w = 76, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 314, .adv_w = 74, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 327, .adv_w = 132, .box_w = 8, .box_h = 6, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 351, .adv_w = 96, .box_w = 8, .box_h = 5, .ofs_x = -1, .ofs_y = 0},
    {.bitmap_index = 371, .adv_w = 97, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 386, .adv_w = 92, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 401, .adv_w = 106, .box_w = 7, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 419, .adv_w = 86, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 432, .adv_w = 82, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 445, .adv_w = 99, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 460, .adv_w = 104, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 475, .adv_w = 41, .box_w = 2, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 480, .adv_w = 67, .box_w = 5, .box_h = 5, .ofs_x = -1, .ofs_y = 0},
    {.bitmap_index = 493, .adv_w = 93, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 508, .adv_w = 77, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 521, .adv_w = 122, .box_w = 7, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 539, .adv_w = 104, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 554, .adv_w = 108, .box_w = 7, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 572, .adv_w = 93, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 587, .adv_w = 108, .box_w = 7, .box_h = 7, .ofs_x = 0, .ofs_y = -2},
    {.bitmap_index = 612, .adv_w = 94, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 627, .adv_w = 81, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 640, .adv_w = 77, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 653, .adv_w = 101, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 668, .adv_w = 93, .box_w = 7, .box_h = 5, .ofs_x = -1, .ofs_y = 0},
    {.bitmap_index = 686, .adv_w = 146, .box_w = 9, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 709, .adv_w = 89, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 724, .adv_w = 85, .box_w = 7, .box_h = 5, .ofs_x = -1, .ofs_y = 0},
    {.bitmap_index = 742, .adv_w = 85, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 757, .adv_w = 45, .box_w = 3, .box_h = 7, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 768, .adv_w = 47, .box_w = 5, .box_h = 7, .ofs_x = -1, .ofs_y = -1},
    {.bitmap_index = 786, .adv_w = 45, .box_w = 3, .box_h = 7, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 797, .adv_w = 76, .box_w = 5, .box_h = 3, .ofs_x = 0, .ofs_y = 1},
    {.bitmap_index = 805, .adv_w = 64, .box_w = 4, .box_h = 1, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 807, .adv_w = 77, .box_w = 4, .box_h = 1, .ofs_x = 0, .ofs_y = 5},
    {.bitmap_index = 809, .adv_w = 78, .box_w = 5, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 819, .adv_w = 88, .box_w = 6, .box_h = 6, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 837, .adv_w = 74, .box_w = 5, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 847, .adv_w = 88, .box_w = 5, .box_h = 6, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 862, .adv_w = 80, .box_w = 5, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 872, .adv_w = 47, .box_w = 4, .box_h = 6, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 884, .adv_w = 89, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 897, .adv_w = 88, .box_w = 5, .box_h = 6, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 912, .adv_w = 37, .box_w = 2, .box_h = 6, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 918, .adv_w = 38, .box_w = 3, .box_h = 7, .ofs_x = -1, .ofs_y = -1},
    {.bitmap_index = 929, .adv_w = 81, .box_w = 6, .box_h = 6, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 947, .adv_w = 37, .box_w = 2, .box_h = 6, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 953, .adv_w = 135, .box_w = 8, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 969, .adv_w = 88, .box_w = 5, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 979, .adv_w = 83, .box_w = 5, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 989, .adv_w = 88, .box_w = 6, .box_h = 5, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 1004, .adv_w = 88, .box_w = 5, .box_h = 5, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 1017, .adv_w = 54, .box_w = 4, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 1025, .adv_w = 66, .box_w = 4, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 1033, .adv_w = 54, .box_w = 4, .box_h = 5, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 1043, .adv_w = 87, .box_w = 5, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 1053, .adv_w = 74, .box_w = 6, .box_h = 4, .ofs_x = -1, .ofs_y = 0},
    {.bitmap_index = 1065, .adv_w = 118, .box_w = 8, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 1081, .adv_w = 73, .box_w = 5, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 1091, .adv_w = 74, .box_w = 6, .box_h = 5, .ofs_x = -1, .ofs_y = -1},
    {.bitmap_index = 1106, .adv_w = 68, .box_w = 4, .box_h = 4, .ofs_x = 0, .ofs_y = 0},
    {.bitmap_index = 1114, .adv_w = 47, .box_w = 3, .box_h = 7, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 1125, .adv_w = 39, .box_w = 2, .box_h = 7, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 1132, .adv_w = 47, .box_w = 3, .box_h = 7, .ofs_x = 0, .ofs_y = -1},
    {.bitmap_index = 1143, .adv_w = 76, .box_w = 5, .box_h = 2, .ofs_x = 0, .ofs_y = 2},
    {.bitmap_index = 1148, .adv_w = 54, .box_w = 4, .box_h = 3, .ofs_x = 0, .ofs_y = 3},
    {.bitmap_index = 1154, .adv_w = 43, .box_w = 3, .box_h = 2, .ofs_x = 0, .ofs_y = 1}
};

/*---------------------
 *  CHARACTER MAPPING
 *--------------------*/

static const uint16_t unicode_list_1[] = {
    0x0, 0x1f72
};

/*Collect the unicode lists and glyph_id offsets*/
static const lv_font_fmt_txt_cmap_t cmaps[] =
{
    {
        .range_start = 32, .range_length = 95, .glyph_id_start = 1,
        .unicode_list = NULL, .glyph_id_ofs_list = NULL, .list_length = 0, .type = LV_FONT_FMT_TXT_CMAP_FORMAT0_TINY
    },
    {
        .range_start = 176, .range_length = 8051, .glyph_id_start = 96,
        .unicode_list = unicode_list_1, .glyph_id_ofs_list = NULL, .list_length = 2, .type = LV_FONT_FMT_TXT_CMAP_SPARSE_TINY
    }
};

/*-----------------
 *    KERNING
 *----------------*/


/*Map glyph_ids to kern left classes*/
static const uint8_t kern_left_class_mapping[] =
{
    0, 0, 1, 2, 0, 3, 4, 5,
    2, 6, 0, 7, 8, 9, 8, 9,
    10, 11, 0, 12, 13, 14, 15, 16,
    17, 18, 11, 19, 19, 0, 0, 0,
    20, 21, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 22, 23, 0, 0,
    24, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 22, 0, 0, 8,
    25, 8
};

/*Map glyph_ids to kern right classes*/
static const uint8_t kern_right_class_mapping[] =
{
    0, 0, 1, 2, 0, 3, 4, 5,
    2, 0, 6, 7, 8, 9, 8, 9,
    10, 11, 12, 13, 14, 15, 16, 11,
    17, 18, 19, 20, 20, 0, 0, 0,
    21, 22, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 23, 24, 25, 0,
    26, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 25, 8,
    27, 8
};

/*Kern values between classes*/
static const int8_t kern_class_values[] =
{
    0, 0, 0, 0, 0, 0, 1, 0,
    0, 0, 0, 1, 0, 0, 0, 0,
    1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 6, 0, 3, -3,
    0, 3, 0, -7, -8, 1, 6, 3,
    2, -5, 1, 6, 0, 5, 1, 4,
    -2, 0, 8, 1, -1, 3, 0, -4,
    0, 0, 0, 0, -3, 2, 3, 0,
    0, -1, 0, -1, 1, 0, -1, 0,
    -1, -1, -3, 0, 0, -1, 0, -3,
    -2, 0, -3, 0, -16, 0, -3, -6,
    3, 4, 0, 0, -3, 1, 1, 4,
    3, -2, 3, 0, 0, -7, 0, 0,
    -4, 0, 0, -3, -2, -6, 0, -5,
    -1, 0, -4, 0, 0, 4, 0, -4,
    -1, 0, 0, 0, -2, 0, 0, -1,
    -9, 0, 0, -10, -1, 4, -6, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    4, 0, 1, 0, 0, -3, 0, 0,
    0, 0, 0, 0, 0, 0, 5, 1,
    0, 0, 1, 3, 1, 4, -1, 0,
    3, -1, -4, -17, 1, 3, 3, 0,
    -2, 0, 4, 0, 4, 0, 4, 0,
    0, 0, 0, -1, 4, 0, 0, -2,
    -4, 0, 0, -1, 0, -1, 0, 1,
    -3, -2, -3, 1, 0, -1, 0, 0,
    0, -5, 1, 0, -8, 0, 0, 0,
    0, -7, 1, -8, 0, 0, -4, -1,
    0, 12, -2, -2, 1, 1, -1, 0,
    -2, 1, 0, 0, -7, -3, 0, -12,
    0, 1, -8, 0, 8, -3, 0, -4,
    4, 0, -9, -12, -9, -3, 4, 0,
    0, -8, 0, 1, -3, 0, -2, 0,
    -3, 0, 3, 4, -15, 6, 0, 1,
    0, 0, 0, 0, 1, 1, -2, -3,
    0, 0, 0, -1, 0, 0, -1, 0,
    0, 0, -3, 0, 0, -3, 0, -3,
    0, 0, 0, 0, 1, -1, 0, 0,
    -1, 1, 1, 0, 0, 0, 0, -2,
    0, 0, 0, 0, 0, 0, 0, 0,
    -1, 0, 4, 0, 0, -1, 0, -1,
    0, 0, 0, 0, 0, 0, 0, 0,
    -1, -1, 0, -1, -1, 0, 0, 0,
    0, 0, 0, 0, 0, 0, -2, 0,
    -4, -1, -4, 3, 0, -3, 1, 3,
    3, 0, -3, 0, -1, 0, 0, -6,
    1, -1, 1, -7, 1, 0, -6, 0,
    3, -4, 0, 0, 0, -1, 0, 0,
    -1, 0, 0, 0, 0, 0, -1, -1,
    0, -1, -2, 0, 0, 0, 0, 0,
    0, -1, 0, 0, -1, 0, -1, 0,
    -3, 1, 0, -1, 1, 1, 1, 0,
    0, 0, 0, 0, 0, -1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, -2,
    0, 4, -1, 0, -4, 0, 3, -6,
    -6, -5, -3, 1, 0, -1, -8, -2,
    0, -2, 0, -3, 2, -2, 0, 1,
    0, -4, 1, 0, 0, 0, -1, 0,
    0, 1, 0, 1, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, -1,
    0, 0, -4, 0, 0, 0, 0, 1,
    0, 0, 0, 0, 0, 0, 0, 6,
    0, 0, 0, 0, 0, 0, 1, 0,
    0, 0, -1, 0, 0, -3, 0, 1,
    0, -1, 0, 0, 0, -2, 0, 1,
    0, -6, -4, 0, 0, 0, -2, -6,
    0, 0, -1, 1, 0, -5, 0, -2,
    0, 0, -2, 1, 0, -2, 0, 0,
    0, 1, 0, 1, -3, -3, 0, -1,
    -1, -2, 0, 0, 0, 0, 0, 0,
    -4, 0, 0, -3, 1, -4, 1, 0,
    1, 0, 0, 0, 1, 0, -1, 0,
    5, 0, 3, 0, 0, -2, 0, 3,
    0, 0, 0, 1, 0, 0, 4, 0,
    4, 0, 0, -8, 0, -1, 2, 4,
    -17, 0, 12, 1, -3, -3, 1, 1,
    -1, 0, -6, 0, 0, 6, -8, -3,
    0, -9, 5, 18, -8, 0, -1, 3,
    -3, 0, 0, -1, 0, 1, 16, -3,
    -1, 4, 3, -3, 1, 0, 0, 1,
    1, -2, -4, 0, -17, 4, 0, 0,
    0, 3, 3, 3, 0, 0, 4, 0,
    -8, -7, 0, 6, 4, 2, -5, 1,
    5, 0, 4, 0, 3, 1, 0, 7,
    0, 0, 0
};


/*Collect the kern class' data in one place*/
static const lv_font_fmt_txt_kern_classes_t kern_classes =
{
    .class_pair_values   = kern_class_values,
    .left_class_mapping  = kern_left_class_mapping,
    .right_class_mapping = kern_right_class_mapping,
    .left_class_cnt      = 25,
    .right_class_cnt     = 27,
};

/*--------------------
 *  ALL CUSTOM DATA
 *--------------------*/

#if LVGL_VERSION_MAJOR == 8
/*Store all the custom data of the font*/
static  lv_font_fmt_txt_glyph_cache_t cache;
#endif

#if LVGL_VERSION_MAJOR >= 8
static const lv_font_fmt_txt_dsc_t font_dsc = {
#else
static lv_font_fmt_txt_dsc_t font_dsc = {
#endif
    .glyph_bitmap = glyph_bitmap,
    .glyph_dsc = glyph_dsc,
    .cmaps = cmaps,
    .kern_dsc = &kern_classes,
    .kern_scale = 16,
    .cmap_num = 2,
    .bpp = 4,
    .kern_classes = 1,
    .bitmap_format = 0,
#if LVGL_VERSION_MAJOR == 8
    .cache = &cache
#endif
};



/*-----------------
 *  PUBLIC FONT
 *----------------*/

/*Initialize a public general font descriptor*/
#if LVGL_VERSION_MAJOR >= 8
const lv_font_t font_ms_sb_8 = {
#else
lv_font_t font_ms_sb_8 = {
#endif
    .get_glyph_dsc = lv_font_get_glyph_dsc_fmt_txt,    /*Function pointer to get glyph's data*/
    .get_glyph_bitmap = lv_font_get_bitmap_fmt_txt,    /*Function pointer to get glyph's bitmap*/
    .line_height = 8,          /*The maximum line height required by the font*/
    .base_line = 2,             /*Baseline measured from the bottom of the line*/
#if !(LVGL_VERSION_MAJOR == 6 && LVGL_VERSION_MINOR == 0)
    .subpx = LV_FONT_SUBPX_NONE,
#endif
#if LV_VERSION_CHECK(7, 4, 0) || LVGL_VERSION_MAJOR >= 8
    .underline_position = -1,
    .underline_thickness = 0,
#endif
    .dsc = &font_dsc,          /*The custom font data. Will be accessed by `get_glyph_bitmap/dsc` */
#if LV_VERSION_CHECK(8, 2, 0) || LVGL_VERSION_MAJOR >= 9
    .fallback = NULL,
#endif
    .user_data = NULL,
};



#endif /*#if FONT_MS_SB_8*/

