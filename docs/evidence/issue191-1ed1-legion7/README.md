# Lenovo Legion 7, TU104 `1ed1` — second cross-platform replay of the `0x89` chain, first `\tx-low-staging.txt` green (2026-11-05)

Reporter's issue: [#191](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/191).
RTX 2070 SUPER Max-Q, TU104 `10de:1ed1` (Lenovo Legion 7 15IMHg05 81YU001UUK,
SUBSYS `17AA:3FCE`, 8 GB), Ubuntu 24.04 / `6.8.0-147-generic`, open kernel
module `595.99.02`, internal display, 80 W GPU limit untouched.
`v2.1.10` + assets generated on the fly; F12 one-time USB UEFI boot chaining
`\EFI\ubuntu\shimx64.efi`; Secure Boot off (Setup Mode).

Two rounds on the same stick, identical except one empty marker file:

| round | knob | meta address | mailbox0 | result |
|---|---|---|---|---|
| 1 | default (HIGH staging) | `0x10001f000` | `0x89` | fail, PLM stays closed, `unlock=false` |
| 2 | `\tx-low-staging.txt` added, power-off, retry | `0x6afa1000` `[LOW]` | `0x0` | `execution verified`, `SS0=0x88888888 SS1=0x8`, `unlock=true` |

The marker is the whole difference: FWSEC and everything before the booter fire
are identical between the runs.

## What round 1 establishes (the #156 criterion chain, verbatim)

```
TX: >4GB staging gate: true (family tu10x)
BOOTER-DIRECT: fire meta dev=0x10001f000 mbox0=0x0001f000 mbox1=0x00000001
BOOTER-DIRECT: mid-run sampler verdict: dma_landed=false sanitized_only=true
BOOTER-DIRECT: halted, mailbox0=0x89 mailbox1=0x1 exci=0x00900100
BOOTER-DIRECT: DMEM[0x0600/0x6140/0xff38/0xff4c/0xff50/0x0200] = 0xdead5ec2   (all)
POST-BOOT: SEC2 MAILBOX0=0x00000089 WPR2_HI=0x1ffee00
POST-BOOT: PLM still locked - the payload's writes did not happen
POST-BOOT: CPU readback of the meta page: magic intact (0xdc3aae21371a60b3) -
           our write landed in DRAM, the booter's DMA read is what came back garbage
POST-BOOT: status 0x89-0x90 = WPR descriptor (WprMeta) rejected on DMA readback
```

Three groups are intact simultaneously — host-side meta magic at `+0x00` of the
published page, the full meta field walk (`+0x48` sig addr, `+0xac`, `+0xf8 =
0xa0a0a0a0`), clean failure-path unwind (stock signature restored → FLR →
chainload) — so this is not a construction bug and not a missing bridge bus
master (`HANDOFF-PCI bridge … cmd=0x0007` on all five path bridges, BME on at
claim). What failed is exactly one thing: SEC2's DMA read of a page above 4 GiB.

FWSEC-side geometry matches #156's TU104M (`dst=0xfe00`, `BOOTVEC=0xfd00`),
i.e. the same die-family booter layout on both machines.

## What round 2 establishes

```
TX: >4GB staging gate: false (family tu10x)
DMA alloc: 16 pages memtype=EfiBootServicesData dev=0x6afa1000 [LOW]
BOOTER-DIRECT: fire meta dev=0x6afa1000 mbox0=0x6afa1000 mbox1=0x00000000
BOOTER-DIRECT: halted, mailbox0=0x0 mailbox1=0x0 exci=0x00900100
TX: execution verified
TX: TU10X PLM relock: plm=0xffffffef known-good (bits 5:6 sticky-open)
TX-tu10x: FLR skipped (Turing FEAT PLM does not survive FLR ...)
HANDOFF-OUT: ... tu10x:unlock=true ...
```

In-OS `ssdiag` after the benchmarks: `PLM=0xffffffef (closed (relock+drift))`,
`READOUT_1=0`, `VERDICT: UNLOCKED`, zero Xid / card-loss / GPU-fault hits.
`0xffffffef` is the same relock value TU102/TU106 produced — the known-good
criterion reproduces on **TU104** for the first time (input to open question
#12, not a defect of this run).

Both rounds' `dma_landed=false sanitized_only=true` is expected and documented
in the log itself: the fill pattern checked is V67-specific, so a replay run
that reaches sanitize first trips it — the verdict rests on POST-BOOT and the
selectors, which round 2 satisfies.

## Two operator-relevant observations

- **The failed round left the screen black while the machine was alive** (SSH
  worked; the reporter power-cycled). `native-GOP::recover` is called on the
  success path only, so a failed transaction hands off without rebuilding the
  console. Do not pull the stick on a dark frame; wait for the OS. Round 2's
  recovery rebuilt the console from the card's own shadowed ROM
  (`RomImage`/`RomSize=131072`) and verified a 1920x1080 GOP child.
- Round 2's handler still ended `console rebind failed NOT_FOUND (bootmgfw will
  self-establish)` and `STRUCTURAL: display recovery NOT verified` — display
  came back via the OS, not via us. Treat black-until-OS as the expected shape,
  not as a hung transaction.

## Limits

VT-d, ReBAR and CSM state were not recorded, and clocks/power were not pinned,
so the VT-d×staging 2×2 from open question #17 is not answered by this pair —
what it does show is that "HIGH always 0x89, LOW always passes" now holds on a
second, unrelated platform (Lenovo laptop vs the Alienware DGFF of #156).

## Attachments

- `loader-1ed1.log` — round 1 (default HIGH staging), fail at booter fire.
- `loader-1ed1-low.log` — round 2 (`\tx-low-staging.txt`), full green,
  including the TU10X FEAT sweep (`0x4096a4..0x4096fc` all `badf5040` again
  after PLM open — the Turing FEAT block's PL0 lock reconfirmed) and the
  native-GOP rebuild.
- `unlock-diag-low.txt` — in-OS `ssdiag` after the benchmarks (VERDICT:
  UNLOCKED; `FB_SIZE_MIB`/`MMU_PLM`/`FUSE_*` badf as usual at PL0).
- `stock-steady-tensor.json` / `unlocked-low-steady-tensor.json` — torch
  steady-state: FP16 26.63 → 35.15 TFLOPS (+32.0%), FP32 control 5.60 flat.
- `stock-v2-ollama.json` / `unlocked-low-ollama.json` — quantised LLM traffic
  (qwen3.5:4b): 2132 → 2126 tok/s, no gain (bandwidth/shape-bound, as expected).
- `SHA256SUMS` — all seven files.
