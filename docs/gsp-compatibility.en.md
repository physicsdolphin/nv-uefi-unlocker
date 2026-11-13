# GSP compatibility: the badge that smashes the gate - versions do not matter (measured, 2026-11-08)

> TL;DR: **the unlock is insensitive to the GSP firmware's version, size and
> provenance** - in this pipeline the GSP image is just "the badge that
> smashes the FEAT gate open" (in payload mode the booter's own verification
> of the image never runs before the ROP fires). The full 525→621 spectrum -
> five same-format GSPs - unlocked end-to-end on the TU106/Z170-A bench. The
> only two hard boundaries are **container-format generation** (pre-52x) and
> the **fwimage↔fwsignature pairing**.

## Default behavior since v2.1.11

`make-usbroot` now prefers the **locally installed NVIDIA driver's own GSP**
("badge mode") and falls back to downloads only when no local store exists:
auto = local DriverStore/firmware → Arch Linux Archive → first-party `.run` →
CachyOS v3 → CachyOS v4. Windows: `DriverStore\FileRepository\nv*inf_amd64_*\gsp_*.bin`
(newest `DriverVer` wins); Linux: `/usr/lib|/lib|/run/current-system` firmware,
`sort -V`. Badge mode **waives the version pin** - the waiver is exactly the
measured matrix below - and the local driver is the handoff target's own
firmware, which is the most correct GSP to map. Pin a single route with
`-UtilsSource/--utils-source local|arch|run|cachyos|cachyos4`; the hash gate
inside the download routes is unchanged, and the pinned 610.43.03 remains the
reproducibility anchor for everything that is downloaded (constitution 6).
A stick built in badge mode carries the host driver's own GSP - the
`fwimage_*.bin loaded N bytes` line in `loader.log` is its size fingerprint;
bit-exact stick reconstruction (evidence-as-commits) should take a download
route and keep the pinned file in `inputs/`.

There is deliberately **no "GSP version" download selector**: full
compatibility means the option has no functional payoff and would imply a
support matrix that does not need to exist. Non-pinned GSP goes through
badge mode or the offline drop-in (both sanctioned paths).

## Why the GSP does not care about versions

The transaction's entire dependency surface on the image is three pure
functions (sim `gsp_version_swap.rs`, 125 tests green):

1. `layout_from_registers` - consumes the **image size**;
2. `radix::build` - content-agnostic page-table mapping;
3. the WprMeta `image_bytes` field - again, just the size.

In payload mode the WprMeta signature DMA aims at the V67 blob, and the
booter's signature copy overflows into the ROP - **any verification of the
GSP image itself only comes after that**, by which time the gate is open.
Inside the unlock window the image's entire role is radix payload + WPR2
placeholder.

The three pieces that genuinely bind to versions do not come from the
nvidia-utils package: the **booter** (kernel-source BINDATA), the **GspRmBoot
BL** (same source) and **FWSEC** (built into the VBIOS).

## Measured matrix (TU106 / Z170-A, 2026-11-08)

**Scope rule: every measurement is the Turing lineage (`gsp_tu10x`).** GSP is
distributed per GPU generation (Turing = `gsp_tu10x.bin`; Ampere/Ada =
`gsp_ga10x.bin`, with Ada consuming its `.fwsignature_ad10x` signature
section) - **cross-generation use is out of scope and should not be
attempted**: a 40-series (AD10x) card's compatibility floor is the earliest
Ada-launch GSP (Windows 522.27 / Linux 525.60.11 era - verify its
`fwsignature_ad10x` is present), and the Ada lineage's version freedom needs
its own bench rounds (same mechanism, unmeasured - do not extrapolate).

| GSP version | fwimage_tu10x | size | loader | result |
|---|---|---|---|---|
| 525.60.11 | 0x2359FE0 | 37,068,768 B | main (post-fix) | ✅ full unlock |
| 560.35.03 | - | 26,250,912 B | v2.1.10 | ✅ full unlock |
| 610.43.03 (pinned stock) | - | 29,339,080 B | v2.1.10 | ✅ full unlock |
| 615.78.08 | 0x15D3DC8 | 22,887,880 B | v2.1.10 | ✅ full unlock |
| 621.01 (Windows driver store) | 0xFE6DD8 | 16,674,264 B | main (post-fix) | ✅ full unlock |

A 2.2x size span within the Turing lineage, all absorbed by the layout, with
byte-exact radix mapping, `HANDOFF-OUT tu10x:unlock=true`, selectors
0x88888888/0x8 landed and READOUT_1 = 0x0 (full speed). Evidence:
`docs/evidence/z170a-tu106-logcommit/` (complete unlock logs for 525 and
621). Both loaders (v2.1.10 and post-fix main) cross versions green - the
loader does not couple to the GSP version either.

## Boundary one: container-format generation (510-520 cannot be packaged)

- The `gsp.bin` of **510.39.01 / 515.x / 520.56.06** is a single
  multi-generation RISC-V ELF: `.tu10x_resid` / `.ga10x_resid` /
  `.ga100_resid` resident sections + `.acl` + `.manifest` - **no `.fwimage`,
  no `.fwsignature_*` sections**. The split per-family signed container
  starts at **525.60.11**. This is a structural cliff, not a version
  distance: the pipeline (and the booter's WprMeta/radix consumer surface)
  takes an `.fwimage` span as input, which the 510-520 containers cannot
  produce.
- The makeself shell changed with it: 510-535 payloads are xz (magic
  `fd377a58`), 55x+ are zstd (`28b52ffd`) - firmware extraction needs both.

## Boundary two: fwimage↔fwsignature must be paired

The signature section and the image come from the same container (the
signature covers that image's build). When swapping versions, swap BOTH;
stock/diagnostic mode (no V67) and the payload path's pre-FLR signature
restore both depend on the pairing. Rolling containers are paired by
construction; hand-assembling is not allowed.

## What this means for the pipeline and users

- The download routes keep the pinned 610.43.03 (reproducibility,
  constitution 6); the compatibility finding is knowledge, not a config knob.
- If the download sources die again: **any 525-621 `gsp_tu10x.bin` works as
  an offline substitute** - extract the paired `.fwimage` +
  `.fwsignature_tu10x` into `inputs/` or the stick root (no runtime hash
  pin; only the size window is checked). The Windows driver store
  (`DriverStore\FileRepository\nv_dispi.inf_*\gsp_tu10x.bin`) is a sanctioned
  source - this machine's 621.01 was extracted exactly that way.
- No "GSP version" download selector: full compatibility means the option
  has no functional payoff; non-pinned GSP goes through badge mode or the
  offline drop-in (both sanctioned).

## Evidence and reproduction

- sim: `sim/src/gsp_version_swap.rs` (125 tests); real-machine logs:
  `docs/evidence/z170a-tu106-logcommit/` (complete unlock logs for 621 and
  525).
- Extraction recipe: the `gsp-swap` kit README (stick root); dual-format
  .run shell parsing (skip= line number + xz/zstd magic discrimination).
