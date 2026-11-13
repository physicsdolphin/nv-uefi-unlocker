[Source: issue #237 attachments by JeusChrit, 2026-11-12 (GPL-2.0-only, same as repo).
 AD103-300 / RTX 4080 10de:2704, stock non-unlocker boot, passive BAR0 capture
 0x00820000..0x00825ffc (6 pages, 6144 words) + PTOP 0x22430-0x2243c; and static
 disassembly tables from 595.44.15 GSP (150 OBJFUSE+118 query descriptors,
 64 VBIOS app08 override pairs, 50 InfoROM RPR v2 targets, FWSEC 5-chip bitmask tables).
 Byte-dump coverage: page 0 (0x820000-0x820dFF) + pages 3-5 (0x823000-0x825fff);
 pages 1-2 (0x821000-0x822fff, i.e. 0x8213a0 trims / 0x822148-0x822444 override
 pairs / 0x822880 ROP-per-GPC) are NOT in the capture - those addresses are
 disassembly-derived claims only.]
# AD102 / AD103 Static Disassembly Tables (595.44.15 GSP & VBIOS Falcon)

# All 150 OBJFUSE +118 query entries

Expressions are static arithmetic on saved words, not observed API returns; software filters and runtime path selection still apply.

| Query | Target | Kind | Offset | Shift/mask | Saved word | Saved expression |
|---|---|---|---|---|---|---|
| 0xff000000 | 0x55724a | simple descriptor | 0x204 | 0/0x1 | 0x0 | 0x0 |
| 0xff000001 | 0x557232 | simple descriptor | 0x20c | 0/0x1 | 0x0 | 0x0 |
| 0xff000002 | 0x55718c | simple descriptor | 0x260 | 0/0x3f | 0x1e | 0x1e |
| 0xff000003 | 0x557172 | simple descriptor | 0x26c | 0/0x3f | 0x8 | 0x8 |
| 0xff000004 | 0x55727c | simple descriptor | 0x270 | 0/0x1ff | 0x1fe | 0x1fe |
| 0xff000005 | 0x557262 | simple descriptor | 0x274 | 0/0x1ff | 0x7 | 0x7 |
| 0xff000006 | 0x5572ae | simple descriptor | 0x330 | 0/0x7 | 0x0 | 0x0 |
| 0xff000007 | 0x557296 | simple descriptor | 0x2fc | 0/0x1 | 0x1 | 0x1 |
| 0xff000008 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000009 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff00000a | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff00000b | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff00000c | 0x5572e2 | simple descriptor | 0x264 | 0/0xffffffff | 0x1409247 | 0x1409247 |
| 0xff00000d | 0x5572c6 | simple descriptor | 0x268 | 0/0xfffffff | 0x0 | 0x0 |
| 0xff00000e | 0x55751e | simple descriptor | 0x2c0 | 0/0x3 | 0x0 | 0x0 |
| 0xff00000f | 0x557508 | simple descriptor | 0xc24 | 0/0xf | 0xe | 0xe |
| 0xff000010 | 0x5574f2 | simple descriptor | 0xc0c | 0/0xf | 0xf | 0xf |
| 0xff000011 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000012 | 0x5574da | simple descriptor | 0x25c | 0/0xf | 0x1 | 0x1 |
| 0xff000013 | 0x5574c2 | simple descriptor | 0x228 | 0/0x1 | 0x0 | 0x0 |
| 0xff000014 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000015 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000016 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000017 | 0x5574aa | simple descriptor | 0x4f8 | 0/0x1 | 0x0 | 0x0 |
| 0xff000018 | 0x557492 | simple descriptor | 0x250 | 0/0x1 | 0x0 | 0x0 |
| 0xff000019 | 0x55747a | simple descriptor | 0x434 | 0/0x1 | 0x1 | 0x1 |
| 0xff00001a | 0x557462 | simple descriptor | 0x510 | 0/0x1 | 0x0 | 0x0 |
| 0xff00001b | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff00001c | 0x55744a | simple descriptor | 0x57c | 0/0x1 | 0x0 | 0x0 |
| 0xff00001d | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff00001e | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff00001f | 0x557432 | simple descriptor | 0x610 | 0/0x1 | 0x0 | 0x0 |
| 0xff000020 | 0x55741a | simple descriptor | 0x584 | 0/0x1 | 0x1 | 0x1 |
| 0xff000021 | 0x557402 | simple descriptor | 0x500 | 0/0x1 | 0x0 | 0x0 |
| 0xff000022 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000023 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000024 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000025 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000026 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000027 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000028 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000029 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff00002a | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff00002b | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff00002c | 0x5573da | descriptor delegated to selected +110 | 0xc14 | 0/0xffffffff | 0x30 | 0x30 |
| 0xff00002d | 0x5573c4 | simple descriptor | 0xc1c | 0/0xffffffff | 0xf80 | 0xf80 |
| 0xff00002e | 0x5573ac | simple descriptor | 0xd38 | 0/0xffff | 0x30 | 0x30 |
| 0xff00002f | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000030 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000031 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000032 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000033 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000034 | 0x557396 | simple descriptor | 0xc04 | 0/0x1 | 0x0 | 0x0 |
| 0xff000035 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000036 | 0x55737e | simple descriptor | 0x67c | 0/0x1 | 0x1 | 0x1 |
| 0xff000037 | 0x557368 | simple descriptor | 0x1304 | 0/0x7 | 0x1 | 0x1 |
| 0xff000038 | 0x557352 | simple descriptor | 0x1350 | 0/0x7 | 0x2 | 0x2 |
| 0xff000039 | 0x55733c | simple descriptor | 0x1398 | 0/0x7 | 0x2 | 0x2 |
| 0xff00003a | 0x557326 | simple descriptor | 0x15ac | 0/0x7 | 0x1 | 0x1 |
| 0xff00003b | 0x557310 | simple descriptor | 0x15b4 | 0/0x7 | 0x2 | 0x2 |
| 0xff00003c | 0x5572fa | simple descriptor | 0x15b8 | 0/0x7 | 0x2 | 0x2 |
| 0xff00003d | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff00003e | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff00003f | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000040 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000041 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000042 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000043 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000044 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000045 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000046 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000047 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000048 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000049 | 0x556fc0 | simple descriptor | 0x1310 | 0/0x3ffff | 0xa03 | 0xa03 |
| 0xff00004a | 0x556fb8 | simple descriptor | 0x1314 | 0/0x3ffff | 0xacc | 0xacc |
| 0xff00004b | 0x556fb0 | simple descriptor | 0x1318 | 0/0x3ffff | 0xad8 | 0xad8 |
| 0xff00004c | 0x556fa8 | simple descriptor | 0x131c | 0/0x3ffff | 0xc4d | 0xc4d |
| 0xff00004d | 0x556fa0 | simple descriptor | 0x1320 | 0/0x3ffff | 0x906 | 0x906 |
| 0xff00004e | 0x556f98 | simple descriptor | 0x1324 | 0/0x3ffff | 0xa49 | 0xa49 |
| 0xff00004f | 0x556f90 | simple descriptor | 0x1328 | 0/0x3ffff | 0xb54 | 0xb54 |
| 0xff000050 | 0x556f88 | simple descriptor | 0x132c | 0/0x3ffff | 0x0 | 0x0 |
| 0xff000051 | 0x556f80 | simple descriptor | 0x1330 | 0/0x3ffff | 0x0 | 0x0 |
| 0xff000052 | 0x556f78 | simple descriptor | 0x1334 | 0/0x3ffff | 0x0 | 0x0 |
| 0xff000053 | 0x556f70 | simple descriptor | 0x1338 | 0/0x3ffff | 0x0 | 0x0 |
| 0xff000054 | 0x556f68 | simple descriptor | 0x133c | 0/0x3ffff | 0x0 | 0x0 |
| 0xff000055 | 0x556f60 | simple descriptor | 0x130c | 0/0x3ffff | 0xa46 | 0xa46 |
| 0xff000056 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000057 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000058 | 0x556f46 | simple descriptor | 0x15a0 | 0/0x3ffff | 0xac2 | 0xac2 |
| 0xff000059 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff00005a | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff00005b | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff00005c | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff00005d | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff00005e | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff00005f | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000060 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000061 | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff000062 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000063 | 0x556f2e | simple descriptor | 0x1358 | 0/0x3ff | 0x1 | 0x1 |
| 0xff000064 | 0x556f16 | simple descriptor | 0x135c | 0/0x3ff | 0x1 | 0x1 |
| 0xff000065 | 0x556efe | simple descriptor | 0x1360 | 0/0x3ff | 0x0 | 0x0 |
| 0xff000066 | 0x556ee6 | simple descriptor | 0x1364 | 0/0x3ff | 0x1 | 0x1 |
| 0xff000067 | 0x556ece | simple descriptor | 0x1368 | 0/0x3ff | 0x0 | 0x0 |
| 0xff000068 | 0x556eb6 | simple descriptor | 0x136c | 0/0x3ff | 0x0 | 0x0 |
| 0xff000069 | 0x556e9e | simple descriptor | 0x1370 | 0/0x3ff | 0x0 | 0x0 |
| 0xff00006a | 0x556e86 | simple descriptor | 0x1374 | 0/0x3ff | 0x0 | 0x0 |
| 0xff00006b | 0x556e6e | simple descriptor | 0x1378 | 0/0x3ff | 0x0 | 0x0 |
| 0xff00006c | 0x556e56 | simple descriptor | 0x137c | 0/0x3ff | 0x0 | 0x0 |
| 0xff00006d | 0x556e3e | simple descriptor | 0x1380 | 0/0x3ff | 0x0 | 0x0 |
| 0xff00006e | 0x556e26 | simple descriptor | 0x1384 | 0/0x3ff | 0x0 | 0x0 |
| 0xff00006f | 0x556e0e | simple descriptor | 0x1354 | 0/0x3ff | 0x0 | 0x0 |
| 0xff000070 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000071 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000072 | 0x556df6 | simple descriptor | 0x15a4 | 0/0x3ff | 0x40 | 0x40 |
| 0xff000073 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000074 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000075 | 0x556dde | simple descriptor | 0x13a0 | 0/0x3ff | 0x40 | 0x40 |
| 0xff000076 | 0x556dc6 | simple descriptor | 0x13a4 | 0/0x3ff | 0x3f | 0x3f |
| 0xff000077 | 0x556dae | simple descriptor | 0x13a8 | 0/0x3ff | 0x3d | 0x3d |
| 0xff000078 | 0x556d96 | simple descriptor | 0x13ac | 0/0x3ff | 0x7e | 0x7e |
| 0xff000079 | 0x55715a | simple descriptor | 0x13b0 | 0/0x3ff | 0x0 | 0x0 |
| 0xff00007a | 0x557142 | simple descriptor | 0x13b4 | 0/0x3ff | 0x3f | 0x3f |
| 0xff00007b | 0x55712a | simple descriptor | 0x13b8 | 0/0x3ff | 0x3e | 0x3e |
| 0xff00007c | 0x557112 | simple descriptor | 0x13bc | 0/0x3ff | 0x0 | 0x0 |
| 0xff00007d | 0x5570fa | simple descriptor | 0x13c0 | 0/0x3ff | 0x0 | 0x0 |
| 0xff00007e | 0x5570e2 | simple descriptor | 0x13c4 | 0/0x3ff | 0x0 | 0x0 |
| 0xff00007f | 0x556fe0 | simple descriptor | 0x13c8 | 0/0x3ff | 0x0 | 0x0 |
| 0xff000080 | 0x556fc8 | simple descriptor | 0x13cc | 0/0x3ff | 0x0 | 0x0 |
| 0xff000081 | 0x55721a | simple descriptor | 0x139c | 0/0x3ff | 0x0 | 0x0 |
| 0xff000082 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000083 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000084 | 0x557202 | simple descriptor | 0x15a8 | 0/0x3ff | 0x40 | 0x40 |
| 0xff000085 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000086 | 0x556d3c | returns 0xffff | — | —/— | not in capture | — |
| 0xff000087 | 0x5571ea | simple descriptor | 0x344 | 0/0xffffffff | 0x671337a8 | 0x671337a8 |
| 0xff000088 | 0x5571d2 | simple descriptor | 0x348 | 0/0xffffffff | 0x934c0e64 | 0x934c0e64 |
| 0xff000089 | 0x556d5a | simple descriptor | 0x1618 | 0/0x1 | 0x0 | 0x0 |
| 0xff00008a | 0x5571bc | simple descriptor | 0xe00 | 0/0x1 | 0x0 | 0x0 |
| 0xff00008b | 0x556d34 | zero-output success | — | —/— | not in capture | — |
| 0xff00008c | 0x5570d6 | delegated to selected +170 | 0x39c | 0/0x1 | 0x0 | 0x0 |
| 0xff00008d | 0x557084 | two reads; common block overwrites first output | 0x520 | 25/0x1 | 0xa0000c | 0x0 |
| 0xff00008e | 0x55706c | simple descriptor | 0x168 | 0/0x1 | 0x1 | 0x1 |
| 0xff00008f | 0x557054 | simple descriptor | 0x1060 | 0/0xff | 0x12 | 0x12 |
| 0xff000090 | 0x55703c | simple descriptor | 0x105c | 0/0xff | 0x1a | 0x1a |
| 0xff000091 | 0x557024 | simple descriptor | 0x580 | 0/0x1 | 0x0 | 0x0 |
| 0xff000092 | 0x556d5a | simple descriptor | 0x1618 | 0/0x1 | 0x0 | 0x0 |
| 0xff000093 | 0x55700e | simple descriptor | 0x1614 | 0/0x1 | 0x0 | 0x0 |
| 0xff000094 | 0x556ff8 | simple descriptor | 0x1620 | 0/0x1 | 0x0 | 0x0 |
| 0xff000095 | 0x5571a6 | simple descriptor | 0x161c | 0/0x1 | 0x0 | 0x0 |

# Gated app08 programming pairs

All addresses are Falcon D-space. ROM defaults are not runtime DMEM. Host columns compare low address bits only. Not a RIR format/capacity claim.

| DMEM source | ROM default | Data store PC | Data destination | Control store PC | Control destination | Saved host data/control |
|---|---|---|---|---|---|---|
| 0x650 | 0x00000080 | 0x6d8d | 0x14822148 | 0x6f96 | 0x1482214c | 0x0/0x0 |
| 0x654 | 0x00000080 | 0x6d95 | 0x14822150 | 0x6faa | 0x14822154 | 0x0/0x0 |
| 0x658 | 0x00000080 | 0x6d9d | 0x14822158 | 0x7015 | 0x1482215c | 0x0/0x0 |
| 0x65c | 0x00000080 | 0x6da5 | 0x14822160 | 0x6ffa | 0x14822164 | 0x0/0x0 |
| 0x660 | 0x00000080 | 0x6dad | 0x14822168 | 0x6fe6 | 0x1482216c | 0x0/0x0 |
| 0x664 | 0x00000080 | 0x6db5 | 0x14822170 | 0x6fbe | 0x14822174 | 0x0/0x0 |
| 0x668 | 0x00000080 | 0x6dbd | 0x14822178 | 0x700b | 0x1482217c | 0x0/0x0 |
| 0x66c | 0x00000080 | 0x6dc5 | 0x14822180 | 0x6fd2 | 0x14822184 | 0x0/0x0 |
| 0x670 | 0x00000840 | 0x6dcd | 0x14822188 | 0x7021 | 0x1482218c | 0x0/0x0 |
| 0x674 | 0x00000840 | 0x6dd5 | 0x14822190 | 0x702d | 0x14822194 | 0x0/0x0 |
| 0x678 | 0x00000840 | 0x6ddd | 0x14822198 | 0x7075 | 0x1482219c | 0x0/0x0 |
| 0x67c | 0x00000840 | 0x6de5 | 0x148221a0 | 0x705d | 0x148221a4 | 0x0/0x0 |
| 0x680 | 0x00000840 | 0x6dea | 0x148221a8 | 0x7051 | 0x148221ac | 0x0/0x0 |
| 0x684 | 0x00000840 | 0x6df2 | 0x148221b0 | 0x7039 | 0x148221b4 | 0x0/0x0 |
| 0x688 | 0x00000840 | 0x6df7 | 0x148221b8 | 0x7069 | 0x148221bc | 0x0/0x0 |
| 0x68c | 0x00000840 | 0x6dfc | 0x148221c0 | 0x7045 | 0x148221c4 | 0x0/0x0 |
| 0x690 | 0x00000044 | 0x6e01 | 0x148221c8 | 0x7081 | 0x148221cc | 0x0/0x0 |
| 0x694 | 0x00000044 | 0x6e15 | 0x148221d0 | 0x708d | 0x148221d4 | 0x0/0x0 |
| 0x698 | 0x00000044 | 0x6e1d | 0x148221d8 | 0x70d1 | 0x148221dc | 0x0/0x0 |
| 0x69c | 0x00000044 | 0x6e25 | 0x148221e0 | 0x70bb | 0x148221e4 | 0x0/0x0 |
| 0x6a0 | 0x00000044 | 0x6e2d | 0x148221e8 | 0x70b1 | 0x148221ec | 0x0/0x0 |
| 0x6a4 | 0x00000044 | 0x6e35 | 0x148221f0 | 0x7099 | 0x148221f4 | 0x0/0x0 |
| 0x6a8 | 0x00000044 | 0x6e3d | 0x148221f8 | 0x70c7 | 0x148221fc | 0x0/0x0 |
| 0x6ac | 0x00000044 | 0x6e47 | 0x14822300 | 0x70a5 | 0x14822304 | 0x0/0x0 |
| 0x6b0 | 0x00000840 | 0x6e4f | 0x14822308 | 0x70dd | 0x1482230c | 0x0/0x0 |
| 0x6b4 | 0x00000840 | 0x6e57 | 0x14822310 | 0x70e7 | 0x14822314 | 0x0/0x0 |
| 0x6b8 | 0x00000840 | 0x6e5f | 0x14822318 | 0x7120 | 0x1482231c | 0x0/0x0 |
| 0x6bc | 0x00000840 | 0x6e67 | 0x14822320 | 0x710c | 0x14822324 | 0x0/0x0 |
| 0x6c0 | 0x00000840 | 0x6e6f | 0x14822328 | 0x7105 | 0x1482232c | 0x0/0x0 |
| 0x6c4 | 0x00000840 | 0x6e74 | 0x14822330 | 0x70f1 | 0x14822334 | 0x0/0x0 |
| 0x6c8 | 0x00000840 | 0x6e7c | 0x14822338 | 0x7116 | 0x1482233c | 0x0/0x0 |
| 0x6cc | 0x00000840 | 0x6e81 | 0x14822340 | 0x70fb | 0x14822344 | 0x0/0x0 |
| 0x6d0 | 0x00000840 | 0x6e86 | 0x14822348 | 0x7127 | 0x1482234c | 0x0/0x0 |
| 0x6d4 | 0x00000840 | 0x6e8b | 0x14822350 | 0x7131 | 0x14822354 | 0x0/0x0 |
| 0x6d8 | 0x00000840 | 0x6e9f | 0x14822358 | 0x7167 | 0x1482235c | 0x0/0x0 |
| 0x6dc | 0x00000840 | 0x6ea7 | 0x14822360 | 0x7156 | 0x14822364 | 0x0/0x0 |
| 0x6e0 | 0x00000840 | 0x6eaf | 0x14822368 | 0x714c | 0x1482236c | 0x0/0x0 |
| 0x6e4 | 0x00000840 | 0x6eb7 | 0x14822370 | 0x713b | 0x14822374 | 0x0/0x0 |
| 0x6e8 | 0x00000840 | 0x6ebf | 0x14822378 | 0x715d | 0x1482237c | 0x0/0x0 |
| 0x6ec | 0x00000840 | 0x6ec7 | 0x14822380 | 0x7142 | 0x14822384 | 0x0/0x0 |
| 0x6f0 | 0x00000840 | 0x6ecf | 0x14822388 | 0x7171 | 0x1482238c | 0x0/0x0 |
| 0x6f4 | 0x00000840 | 0x6ed7 | 0x14822390 | 0x7178 | 0x14822394 | 0x0/0x0 |
| 0x6f8 | 0x00000840 | 0x6edf | 0x14822398 | 0x71ae | 0x1482239c | 0x0/0x0 |
| 0x6fc | 0x00000840 | 0x6ee7 | 0x148223a0 | 0x719d | 0x148223a4 | 0x0/0x0 |
| 0x700 | 0x00000840 | 0x6eef | 0x148223a8 | 0x7193 | 0x148223ac | 0x0/0x0 |
| 0x704 | 0x00000840 | 0x6ef7 | 0x148223b0 | 0x7182 | 0x148223b4 | 0x0/0x0 |
| 0x708 | 0x00000840 | 0x6efc | 0x148223b8 | 0x71a7 | 0x148223bc | 0x0/0x0 |
| 0x70c | 0x00000840 | 0x6f04 | 0x148223c0 | 0x718c | 0x148223c4 | 0x0/0x0 |
| 0x710 | 0x00000840 | 0x6f09 | 0x148223c8 | 0x71b8 | 0x148223cc | 0x0/0x0 |
| 0x714 | 0x00000840 | 0x6f0e | 0x148223d0 | 0x71c2 | 0x148223d4 | 0x0/0x0 |
| 0x718 | 0x00000840 | 0x6f13 | 0x148223d8 | 0x71fe | 0x148223dc | 0x0/0x0 |
| 0x71c | 0x00000840 | 0x6f27 | 0x148223e0 | 0x71ea | 0x148223e4 | 0x0/0x0 |
| 0x720 | 0x00000840 | 0x6f2f | 0x148223e8 | 0x71e0 | 0x148223ec | 0x0/0x0 |
| 0x724 | 0x00000840 | 0x6f37 | 0x148223f0 | 0x71cc | 0x148223f4 | 0x0/0x0 |
| 0x728 | 0x00000840 | 0x6f3f | 0x148223f8 | 0x71f4 | 0x148223fc | 0x0/0x0 |
| 0x72c | 0x00000840 | 0x6f47 | 0x14822400 | 0x71d6 | 0x14822404 | 0x0/0x0 |
| 0x730 | 0x00000044 | 0x6f4f | 0x14822408 | 0x7205 | 0x1482240c | 0x0/0x0 |
| 0x734 | 0x00000044 | 0x6f57 | 0x14822410 | 0x720f | 0x14822414 | 0x0/0x0 |
| 0x738 | 0x00000044 | 0x6f5f | 0x14822418 | 0x7239 | 0x1482241c | 0x0/0x0 |
| 0x73c | 0x00000044 | 0x6f67 | 0x14822420 | 0x722b | 0x14822424 | 0x0/0x0 |
| 0x740 | 0x00000044 | 0x6f6c | 0x14822428 | 0x7224 | 0x1482242c | 0x0/0x0 |
| 0x744 | 0x00000044 | 0x6f71 | 0x14822430 | 0x7216 | 0x14822434 | 0x0/0x0 |
| 0x748 | 0x00000044 | 0x6f76 | 0x14822438 | 0x7232 | 0x1482243c | 0x0/0x0 |
| 0x74c | 0x00000044 | 0x6f80 | 0x14822440 | 0x721d | 0x14822444 | 0x0/0x0 |

## Appendix D — Independently modeled unpacked RPR

This is a host-side layout calculation using the saved 448-byte object, **not captured runtime DMEM or an executed firmware call**. First 12 DWORDs below are followed by all 54 address/value pairs. The selector does not use the declared count to shorten the format loop.

| Unpacked offset | Modeled DMEM | DWORD |
|---:|---:|---:|
| 000 | 0d64 | 00000052 |
| 004 | 0d68 | 00000050 |
| 008 | 0d6c | 00000052 |
| 00c | 0d70 | 00000002 |
| 010 | 0d74 | 00000010 |
| 014 | 0d78 | 000001c0 |
| 018 | 0d7c | 00000001 |
| 01c | 0d80 | 00000032 |
| 020 | 0d84 | 00000000 |
| 024 | 0d88 | 00000000 |
| 028 | 0d8c | 00000000 |
| 02c | 0d90 | 00000000 |

| Pair | Packed offset | Unpacked offset | Modeled DMEM address/value fields | Address word | Value word | Within declared count |
|---:|---:|---:|---|---:|---:|---|
| 0 | 010 | 030 | 0d94/0d98 | 0082081c | 00000000 | True |
| 1 | 018 | 038 | 0d9c/0da0 | 00820838 | 00000000 | True |
| 2 | 020 | 040 | 0da4/0da8 | 0082083c | 00000000 | True |
| 3 | 028 | 048 | 0dac/0db0 | 00820840 | 00000000 | True |
| 4 | 030 | 050 | 0db4/0db8 | 00820844 | 00000000 | True |
| 5 | 038 | 058 | 0dbc/0dc0 | 00820848 | 00000000 | True |
| 6 | 040 | 060 | 0dc4/0dc8 | 0082084c | 00000000 | True |
| 7 | 048 | 068 | 0dcc/0dd0 | 00820850 | 00000000 | True |
| 8 | 050 | 070 | 0dd4/0dd8 | 008209bc | 00000000 | True |
| 9 | 058 | 078 | 0ddc/0de0 | 008209c0 | 00000000 | True |
| 10 | 060 | 080 | 0de4/0de8 | 008209c4 | 00000000 | True |
| 11 | 068 | 088 | 0dec/0df0 | 008209c8 | 00000000 | True |
| 12 | 070 | 090 | 0df4/0df8 | 008209cc | 00000000 | True |
| 13 | 078 | 098 | 0dfc/0e00 | 008209d0 | 00000000 | True |
| 14 | 080 | 0a0 | 0e04/0e08 | 008209d4 | 00000000 | True |
| 15 | 088 | 0a8 | 0e0c/0e10 | 00820938 | 00000000 | True |
| 16 | 090 | 0b0 | 0e14/0e18 | 00820814 | 00000000 | True |
| 17 | 098 | 0b8 | 0e1c/0e20 | 00820970 | 00000000 | True |
| 18 | 0a0 | 0c0 | 0e24/0e28 | 00820974 | 00000000 | True |
| 19 | 0a8 | 0c8 | 0e2c/0e30 | 00820978 | 00000000 | True |
| 20 | 0b0 | 0d0 | 0e34/0e38 | 0082097c | 00000000 | True |
| 21 | 0b8 | 0d8 | 0e3c/0e40 | 00820980 | 00000000 | True |
| 22 | 0c0 | 0e0 | 0e44/0e48 | 00820984 | 00000000 | True |
| 23 | 0c8 | 0e8 | 0e4c/0e50 | 00822000 | 00000000 | True |
| 24 | 0d0 | 0f0 | 0e54/0e58 | 00822004 | 00000000 | True |
| 25 | 0d8 | 0f8 | 0e5c/0e60 | 00822008 | 00000000 | True |
| 26 | 0e0 | 100 | 0e64/0e68 | 0082200c | 00000000 | True |
| 27 | 0e8 | 108 | 0e6c/0e70 | 00822010 | 00000000 | True |
| 28 | 0f0 | 110 | 0e74/0e78 | 00822014 | 00000000 | True |
| 29 | 0f8 | 118 | 0e7c/0e80 | 00822018 | 00000000 | True |
| 30 | 100 | 120 | 0e84/0e88 | 00820800 | 00000000 | True |
| 31 | 108 | 128 | 0e8c/0e90 | 00820810 | 00000000 | True |
| 32 | 110 | 130 | 0e94/0e98 | 00820968 | 00000000 | True |
| 33 | 118 | 138 | 0e9c/0ea0 | 00820824 | 00000000 | True |
| 34 | 120 | 140 | 0ea4/0ea8 | 008209b8 | 00000000 | True |
| 35 | 128 | 148 | 0eac/0eb0 | 0082080c | 00000000 | True |
| 36 | 130 | 150 | 0eb4/0eb8 | 00820bc0 | 00000000 | True |
| 37 | 138 | 158 | 0ebc/0ec0 | 00822080 | 00000000 | True |
| 38 | 140 | 160 | 0ec4/0ec8 | 00822084 | 00000000 | True |
| 39 | 148 | 168 | 0ecc/0ed0 | 00822088 | 00000000 | True |
| 40 | 150 | 170 | 0ed4/0ed8 | 0082208c | 00000000 | True |
| 41 | 158 | 178 | 0edc/0ee0 | 00822090 | 00000000 | True |
| 42 | 160 | 180 | 0ee4/0ee8 | 00822094 | 00000000 | True |
| 43 | 168 | 188 | 0eec/0ef0 | 00822800 | 00000000 | True |
| 44 | 170 | 190 | 0ef4/0ef8 | 00822804 | 00000000 | True |
| 45 | 178 | 198 | 0efc/0f00 | 00822808 | 00000000 | True |
| 46 | 180 | 1a0 | 0f04/0f08 | 0082280c | 00000000 | True |
| 47 | 188 | 1a8 | 0f0c/0f10 | 00822810 | 00000000 | True |
| 48 | 190 | 1b0 | 0f14/0f18 | 00822814 | 00000000 | True |
| 49 | 198 | 1b8 | 0f1c/0f20 | 00822818 | 00000000 | True |
| 50 | 1a0 | 1c0 | 0f24/0f28 | 00000000 | 00000000 | False |
| 51 | 1a8 | 1c8 | 0f2c/0f30 | 00000000 | 00000000 | False |
| 52 | 1b0 | 1d0 | 0f34/0f38 | 00000000 | 00000000 | False |
| 53 | 1b8 | 1d8 | 0f3c/0f40 | 00000000 | 00000000 | False |



# Appendix B — Proposed sparse/dense five-group decomposition

**Grouping is a structural interpretation, not a recovered FWSEC type/consumer.** Bit positions below are within each little-endian concatenated row, not GPU unit numbers.

## Candidate group 192 / AD102
Range `370..3dc`, stride `c`; 16 disjoint adjacent-bit pairs.

| Row offset | Stored DWORDs | Set-bit positions in row |
|---|---|---|
| `370` | `00003000 00000000 00000003` | [12, 13, 64, 65] |
| `37c` | `00300000 00000000 00000300` | [20, 21, 72, 73] |
| `388` | `30000000 00000000 00030000` | [28, 29, 80, 81] |
| `394` | `00000000 00000030 03000000` | [36, 37, 88, 89] |
| `3a0` | `00000000 00003000 00000000` | [44, 45] |
| `3ac` | `00000003 00300000 00000000` | [0, 1, 52, 53] |
| `3b8` | `00000300 30000000 00000000` | [8, 9, 60, 61] |
| `3c4` | `00030000 00000000 00000030` | [16, 17, 68, 69] |
| `3d0` | `03000000 00000000 00000000` | [24, 25] |

Dense record `524..534`: `04100000 41041041 10410410 00004104`.
Set bits: `[20, 26, 32, 38, 44, 50, 56, 62, 68, 74, 80, 86, 92, 98, 104, 110]`.

## Candidate group 194 / AD104
Range `3dc..448`, stride `c`; 16 disjoint adjacent-bit pairs.

| Row offset | Stored DWORDs | Set-bit positions in row |
|---|---|---|
| `3dc` | `06000000 00000000 00006000` | [25, 26, 77, 78] |
| `3e8` | `00000000 00000006 00600000` | [33, 34, 85, 86] |
| `3f4` | `00000000 00000600 60000000` | [41, 42, 93, 94] |
| `400` | `00000000 00060000 00000000` | [49, 50] |
| `40c` | `00000060 06000000 00000000` | [5, 6, 57, 58] |
| `418` | `00006000 00000000 00000006` | [13, 14, 65, 66] |
| `424` | `00600000 00000000 00000600` | [21, 22, 73, 74] |
| `430` | `60000000 00000000 00060000` | [29, 30, 81, 82] |
| `43c` | `00000000 00000060 00000000` | [37, 38] |

Dense record `534..544`: `82082082 20820820 08208208 00000000`.
Set bits: `[1, 7, 13, 19, 25, 31, 37, 43, 49, 55, 61, 67, 73, 79, 85, 91]`.

## Candidate group 193 / AD103
Range `448..4b4`, stride `c`; 16 disjoint adjacent-bit pairs.

| Row offset | Stored DWORDs | Set-bit positions in row |
|---|---|---|
| `448` | `c0000000 00000000 000c0000` | [30, 31, 82, 83] |
| `454` | `00000000 000000c0 0c000000` | [38, 39, 90, 91] |
| `460` | `00000000 0000c000 00000000` | [46, 47] |
| `46c` | `0000000c 00c00000 00000000` | [2, 3, 54, 55] |
| `478` | `00000c00 c0000000 00000000` | [10, 11, 62, 63] |
| `484` | `000c0000 00000000 000000c0` | [18, 19, 70, 71] |
| `490` | `0c000000 00000000 0000c000` | [26, 27, 78, 79] |
| `49c` | `00000000 0000000c 00c00000` | [34, 35, 86, 87] |
| `4a8` | `00000000 00000c00 00000000` | [42, 43] |

Dense record `544..554`: `41041040 10410410 04104104 00000001`.
Set bits: `[6, 12, 18, 24, 30, 36, 42, 48, 54, 60, 66, 72, 78, 84, 90, 96]`.

## Candidate group 196 / AD106
Range `4b4..4ec`, stride `8`; 8 disjoint adjacent-bit pairs.

| Row offset | Stored DWORDs | Set-bit positions in row |
|---|---|---|
| `4b4` | `000c0000 00000000` | [18, 19] |
| `4bc` | `000000c0 0c000000` | [6, 7, 58, 59] |
| `4c4` | `00000000 0000c000` | [46, 47] |
| `4cc` | `00000000 0000000c` | [34, 35] |
| `4d4` | `00c00000 00000000` | [22, 23] |
| `4dc` | `00000c00 c0000000` | [10, 11, 62, 63] |
| `4e4` | `00000000 00000000` | [] |

Dense record `554..560`: `04000000 41041041 00000010`.
Set bits: `[26, 32, 38, 44, 50, 56, 62, 68]`.

## Candidate group 197 / AD107
Range `4ec..524`, stride `8`; 8 disjoint adjacent-bit pairs.

| Row offset | Stored DWORDs | Set-bit positions in row |
|---|---|---|
| `4ec` | `00060000 00000000` | [17, 18] |
| `4f4` | `00000060 06000000` | [5, 6, 57, 58] |
| `4fc` | `00000000 00006000` | [45, 46] |
| `504` | `00000000 00000006` | [33, 34] |
| `50c` | `00600000 00000000` | [21, 22] |
| `514` | `00000600 60000000` | [9, 10, 61, 62] |
| `51c` | `00000000 00000000` | [] |

Dense record `560..56c`: `82000000 20820820 00000008`.
Set bits: `[25, 31, 37, 43, 49, 55, 61, 67]`.


