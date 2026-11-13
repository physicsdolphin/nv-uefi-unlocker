# Recorded runs — descriptor-count A/B (rc3, B550 AORUS MASTER F18i, `10de:2206`, 64 GiB)

Legend: **boot** = cold (full power-off ≥30 s) / warm (restart). **path** = F12 / BIOS-override (DELETE → exit/save → Boot Override → USB). `*` = assumed (not recorded at the time).

| Run | boot | path | preload | N descriptors | largest (MiB) | resident (MiB) | tiny_conv | bs_data | Outcome |
|---|---|---|---|---|---|---|---|---|---|
| R0 | unrecorded | F12* | OFF | 668 | 2634 | 138 | 139 | 318 | boots |
| R1 | warm | F12* | OFF | 1040 | 2660 | 92 | 301 | 501 | 0xc000009a |
| R2 | warm | F12* | ON | 1048 | 2577 | 92 | 305 | 503 | 0xc000009a |
| R3 | cold | F12* | ON | 1039 | 2577 | 92 | 300 | 500 | 0xc000009a |
| R4 | cold | F12* | OFF | 1024 | 2662 | 80 | 301 | 494 | 0xc000009a |
| R5 | warm | BIOS-override | ON | 665 | 2551 | 137 | 145 | 316 | boots |
| R6 | warm | BIOS-override | ON | 549 | 2551 | 138 | 98 | 257 | boots |

Notes: every run unlock-green (`SS0=0x88888888 SS1=0x8 READOUT_1=0x0`, `plm_open=true`). `N` = the `(... descriptors ...)` total on the `CHAIN-MEMMAP` summary line; `tiny_conv` / `bs_data` from `CHAIN-MEMMAP-TYPES`.
