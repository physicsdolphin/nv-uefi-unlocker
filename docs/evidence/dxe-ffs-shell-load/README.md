# DXE driver form — QEMU/OVMF shell-load smoke (V0/V1-lite)

> 2026-11-01, branch dxe-driver-ffs. Zero-flash validation of the
> firmware-resident DXE driver form (docs/dxe-driver-ffs-feasibility.md §8
> V0/V1). Firmware: QEMU 11.1 bundled `edk2-x86_64-code.fd` (contains the
> EFI Internal Shell, Boot0002). Driver: `nvunlock-dxe` bin (tx OFF),
> subsystem patched 0x0A→0x0B by ffspack, FFS SHA256
> a7c176dd…8c6bda (tx build, packaged for header verification only).
> Repro: `mkimages.sh` (WSL mtools FAT images) + the three qemu command
> lines below; fresh `edk2-i386-vars.fd` copy per run.

## Run 1 — armed driver (run-armed.log, loader-armed.log)

`startup.nsh`: `load fs0:\NvUnlockDxe-readonly.efi`; arm marker
`\tx-dxe-arm.txt` = `0` (0s countdown).

- Shell `load` → BDS-up path → immediate run (BDS arch protocol present).
- `volumes` override verified: log file `\loader.log` opened on the ARMED
  volume (the driver has no SFS self-volume; without the override every
  marker/asset would read absent).
- CONFIRM armed 0s → PROCEED → ENUMERATE 6 PciIo handles → GOP-PROBE
  (OVMF VGA) → SAMPLE: no NVIDIA GPU (negative control) → FINAL:
  "driver flow complete; BDS continues to the boot option".
- Control returns to the `Shell>` prompt (clean return = BDS continues).
- `\loader.log` extracted from the FAT image afterwards: identical content.

## Run 2 — negative control, unarmed (run-noarm.log)

Same image minus `\tx-dxe-arm.txt`.

- 0 driver log lines on console, no `\loader.log` in the image: the
  unarmed driver is a total no-op (volume scan only, reads only).

## Run 3 — app regression after the lib split (run-app.log)

`loader.efi` as `\EFI\BOOT\BOOTX64.EFI` (tx OFF).

- Boot0001 → banner → `CONFIRM: no armed marker - Space/Enter/30s` →
  30s timeout → ENUMERATE → SAMPLE negative control → `CHAIN: failed:
  NOT_FOUND` (expected: no bootmgfw in QEMU) → FINAL. The M0 semantics
  are unchanged by the lib split (`main.rs` is a thin entry now).

## Coverage and what this does NOT cover

Covered: driver entry, BDS-up immediate path, volume selection + reader
override, arm/quiet/flush marker semantics, countdown knob (0), failsafe
counter code paths compile (not exercised here), app parity after the
lib split, ffspack PE patch + FFS structure (self-verify + this smoke).

Not covered here (feasibility doc §8): FV dispatch of the FFS file with a
TRUE depex (needs firmware-image insertion — OVMF FVMAIN_COMPACT has no
room; first validated on a real board at V3), the ReadyToBoot event path
(entry registers and returns; the shell-load path bypasses it because
BDS is already up), and any tx build on real hardware (V2, gated).
