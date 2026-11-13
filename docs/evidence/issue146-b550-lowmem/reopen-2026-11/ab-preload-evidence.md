# A/B — descriptor-count drivers (rc3): preload, boot type, boot path

## System (once)

| Item | Value |
|---|---|
| GPU | NVIDIA RTX 3080 10GB — `10de:2206` (GA102), VBIOS `94.02.26.40.9b` |
| Board | Gigabyte B550 AORUS MASTER, BIOS F18i |
| CPU | AMD Ryzen 9 5900X |
| RAM | 64 GiB |
| OS | Windows 11 25H2, build 26200.9168 |
| Driver | 610.47 |
| Secure Boot | ON — shim 15.8 + self-signed MOK (`.sbat` added); db/KEK/PK untouched |
| Loader | nv-uefi-unlocker **v2.1.12-rc3** (TX), MOK-signed |

## Legends

- **boot:** cold = full power-off ≥30 s before run; warm = restart without power cycle.
- **path:** F12 = one-time boot menu; **BIOS-override** = DELETE → (exit & save) → Boot Override → USB.

## Results

| Run | boot | path | tx-preload | N descriptors | Outcome |
|---|---|---|---|---|---|
| R0 | unrecorded | F12* | OFF | **668** | boots |
| R1 | warm | F12* | OFF | 1040 | `0xc000009a` |
| R2 | warm | F12* | ON | 1048 | `0xc000009a` |
| R3 | cold | F12* | ON | 1039 | `0xc000009a` |
| R4 | cold | F12* | OFF | 1024 | `0xc000009a` |
| R5 | warm | **BIOS-override** | ON | **665** | boots |
| R6 | warm | **BIOS-override** | ON | **549** | boots |

\* = assumed F12 (not explicitly recorded at the time).

## Per-run evidence

### R5 — warm · BIOS-override · preload ON
```
HANDOFF-OUT: ss0=0x88888888 ss1=0x8 readout1=0x00000000 feat_plm=0xffffffff
             ga/ada:ss0_unlock=true ss1_unlock=true plm_open=true
CHAIN-MEMMAP: 2809 MiB conventional / 137 MiB resident, largest block 2551 MiB (665 descriptors)
CHAIN-MEMMAP-TYPES: counted=665/665 reserved=6 loader_code=2 loader_data=17 bs_code=139
  bs_data=316 rt_code=1 rt_data=1 conv=167 ... | tiny_conv(<=64KiB): 145 blocks, 0 MiB
```

### R6 — warm · BIOS-override · preload ON
```
HANDOFF-OUT: ss0=0x88888888 ss1=0x8 readout1=0x00000000 feat_plm=0xffffffff
             ga/ada:ss0_unlock=true ss1_unlock=true plm_open=true
CHAIN-PRE: '\EFI\MICROSOFT\BOOT\BOOTMGFW.EFI' resident at 0x10000000 (0x33a000 bytes)
CHAIN-MEMMAP: 2809 MiB conventional / 138 MiB resident, largest block 2551 MiB (549 descriptors, 64 shown)
CHAIN-MEMMAP-TYPES: counted=549/549 reserved=6 loader_code=2 loader_data=6 bs_code=139
  bs_data=257 rt_code=1 rt_data=1 conv=121 unusable=0 acpi_reclaim=1 acpi_nv=2 mmio=13
  | tiny_conv(<=64KiB): 98 blocks, 1 MiB
```

## Summary

- **Boot path is the lever.** BIOS-override draws a clean map every time so far: **665, 549** (2/2 boots). F12 draws high **1024–1048** in 4 of 5, with one 668 outlier.
- **Map cleanliness is the mechanism:** BIOS-override `tiny_conv` 145→98 blocks, `bs_data` 316→257 — a compact map; F12 runs are shredded (`tiny_conv` 300–305, `bs_data` 494–503).
- **Preload and cold/warm are negligible** (+8–15 descriptors), lost in the F12 swing.
- **Unlock green in every run** — only winload's 1024-entry pool decides the boot, and BIOS-override keeps `N` comfortably under it.
