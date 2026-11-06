# #195 — GA102 `2203` (MSI 3090 Ti) on Gigabyte X570 GAMING X, v2.1.10

Source: [issue #195](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/195) (reported 2026-11-06, English/Chinese bilingual).

## Context

| Item | Value |
|---|---|
| GPU | MSI RTX 3090 Ti, `10de:2203`, SUBSYS `1462:5090`, 24 GB, no known modifications |
| VBIOS | `94.02.a0.00.0d`, PROM first 64 KiB CRC32 `0x51a96118` |
| Board | Gigabyte X570 GAMING X, AMI BIOS F38f |
| CPU / RAM | Ryzen 9 5950X (no iGPU, display on the 3090 Ti), 64 GB |
| OS / driver | Windows 11 Pro build 26200, NVIDIA 616.92 |
| Firmware settings | Secure Boot off, CSM disabled, Above 4G/ReBAR enabled (BAR1 256 -> 32768 MiB), fast startup off |
| Tool | v2.1.10 official zip, zip SHA-256 `2d8093e64b308710683fc020142b567fb437510e2d3dcee09156b7824701beb1`, `BOOTX64.EFI` SHA-256 `fd8cc71648e45337c57ebaa652ac14fc92efb8335c283aef67736796b77ce755` |

Two runs on the same stick, same settings:

- **Run 1** (2026-11-05 ~23:25): no key pressed, `CONFIRM: 30s timeout - ABORT to Windows`, zero GPU writes, clean handoff.
- **Run 2** (~23:32): confirmed, unlock mode. The attached `loader.log` is this run.

`-Out G:\` failed on Windows PowerShell 5.1 at the `New-Item` line with "The path is not of a legal form"; the reporter built to the default `.\usbroot` and copied it with `xcopy`. Fixed in `release/make-usbroot.ps1` and FAQ.

## Chain (run 2, quoted)

```
CONFIRM: Enter pressed
payload_mode=true
[  20957ms] INFO : loader::tx_fwsec: SCRUB: SEC2 fired but handoff stuck at 1
[  20957ms] INFO : loader::tx: TX: scrubber did not complete (ETimeout { tolerated: false }) - proceeding without it (community chain shape)
SELECTOR RESULT: SS0=0x88888888 SS1=0x8
POST-FLR selectors: ss0=0x88888888 ss1=0x8
POST-FLR: verified (selectors survive AON)
[  25331ms] INFO : loader::gop_native: NATIVE-GOP: verified GOP child Handle(0xb698d498) (1920x1080) after 0 probe(s)
NATIVE-GOP: complete (display path verified)
FINAL: transaction complete, all DMA released
HANDOFF-OUT: ss0=0x88888888 ss1=0x8 readout1=0x00000000 feat_plm=0xffffffff wpr2_hi=0x0 pmcsr=0x0008 dstate=D0
             ga/ada:ss0_unlock=true ss1_unlock=true plm_open=true
```

Entry `FEAT_READOUT_1=0x00400080`. No marker files. No `0x89` on this platform (the `<4GiB` gate does not fire on `2203`).

Windows booted normally: last log write 23:32:00, boot time 23:32:02; no Device Manager problem code, `nvidia-smi` working, display normal.

## A/B (before.json / after.json)

PyTorch 2.1.2+cu118 `torch.matmul`, dense 8192^3, FP32 accumulation; 30 s per mode, batches of 10 timed with CUDA events, 90th percentile; power/clock/temp are 1 s `nvidia-smi` medians. Same driver, VBIOS, 480 W limit, ReBAR state and script; before = locked boot, after = unlocked boot.

| Mode | Before | After | Ratio | Power W | SM MHz | Output SHA-256 |
|---|---:|---:|---:|---|---|---|
| FP16 | 71.5 | 99.5 | 1.39x | 467 > 446 | 1950 > 1800 | identical |
| BF16 | 72.6 | 120.4 | 1.66x | 470 > 457 | 1980 > 1755 | identical |
| TF32 | 36.1 | 64.1 | 1.77x | 409 > 446 | 1995 > 1845 | identical |
| FP32 (control) | 25.4 | 25.4 | 1.00x | 462 > 462 | 1830 > 1830 | identical |

Medians, same order: before 71.1 / 72.0 / 35.8 / 25.3, after 93.9 / 119.3 / 63.7 / 25.4.

Gains are power-limited: clocks fall while power stays in the same band. FP32 (no tensor cores) is flat. The identical per-mode output hashes are the strongest part of the method — environment noise cannot explain the deltas.

## What this establishes

- First issue report for `2203` with a full EFI log and a same-script A/B; first 3090 Ti shape. The `2203` row moves from ESM/unverified to ✅ (EFI chain + self-evidenced Windows result).
- **TF32 doubles on GA102 too** (1.77x), same direction as #194's 2.20x on GA104 and #159's +58% on `2204`. The docs' "GA10x TF32 is full rate, only FMLA32/IMLA4 are clamped" line is per-die at best.
- v2.1.10 native-GOP passes on an X570 GAMING X, the board family behind several reconnect-hang reports (#61, #106).

## Limits

- `ssdiag` not run in-OS; the unlocked state rests on the A/B, the output hashes and `HANDOFF-OUT`, not an in-OS FEAT read-back.
- One benchmark run per state, not a controlled round; desktop apps were open (GPU utilization 1% before / 12% after). Earlier stock runs before ReBAR was enabled varied ~70-83 TFLOPS FP16/BF16 with other GPU users.
- Sleep/resume, long-run stability and application workloads untested.
- CSM state is the reporter's own setting, not read back.

## Files

`loader.log` (run 2, complete), `before.json`, `after.json`, `SHA256SUMS`. Hashes: `loader.log` `f3203b69…`, `before.json` `78fec3ac…`, `after.json` `2619064d…`.
