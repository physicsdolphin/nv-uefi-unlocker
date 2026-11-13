# Issue #146 reopen (2026-11-04 → 2026-11-12): the descriptor-count ceiling

Three further machines hit `0xc000009a` on builds where the v2.1.11 asset free worked
exactly as designed. Root cause re-identified by static analysis of `winload.efi`
(Windows 11 26200-era build, SHA-256 `7002fe01ac5153e06bdfeacecafc5e5c929172149e4731b7259bf90b6611f44f`):
winload ingests every descriptor of the handoff map into a **static pool of exactly
1024 entries** (`BlpMmInitialize` / `MmGlobalMemoryDescriptorCount`, 48-byte stride,
no dynamic overflow); per-entry `MmMdInitDescriptor` NULL ⇒ `0xC000009A`. Full
walkthrough: `docs/handoff-lowmem*.md` §0b and the 2026-11-12 comments on the issue
([RE reply](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146#issuecomment-6815041042),
[BIOS-test protocol](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146#issuecomment-6815166830)).

## Field data — CHAIN-MEMMAP whole-map descriptor totals

| Total | Machine / board | Path | Outcome | Log here |
|---|---|---|---|---|
| 448 | otacoo / B550I AORUS PRO AX | ESC abort (zero GPU writes) | **boots** | run5-…esc-abort-repeat.log |
| 998 | ivandrozdov1505-coder / B550M AORUS ELITE (FHb) | ESC abort | **boots** | ivan/loader-escape-control.log |
| 1005 | otacoo / B550I | **ESC abort** (zero GPU writes) | fail, no Recovery screen | run4-…esc-abort.log |
| 1007 | otacoo / B550I | dry-run + preload (zero writes, no detach) | 0xc000009a | run7-…dry-run.log |
| 1012 | TheHarbit / (B550, model unreported) | tx + preload | 0xc000009a | loader_v2.1.11-diag2.log |
| 1013 | ivandrozdov / B550M | tx, no-preload | 0xc000009a | ivan/loader-no-preload-attempt.log |
| 1014 | ivandrozdov / B550M | tx, no-preload, no-reconnect | fail (frozen frame) | ivan/loader-skip-gop-attempt.log |
| 1021 | otacoo / B550I | tx + preload | 0xc000009a | run3-…transaction.log |
| 1022 | otacoo / B550I | tx, no-preload (and no-cache variant, identical map) | 0xc000009a | run6-…, run8-… |
| — | ivandrozdov / B550M | tx on driver 621.14, map identical to 1013 run | fail | ivan/loader-driver-62114.log |

Threshold ∈ (998, 1005] ≈ 1024 minus winload's ~20–25 descriptors of concurrent
working set. Zero exceptions in ten data points. GPU state exonerated (the failing
1005 and passing 448 runs are both zero-write aborts); largest-block theory demoted
to secondary (same-size pass/fail pairs on two machines).

## Machine B (ivandrozdov) map anatomy

The tx path's visible delta over the passing abort is a ~7.9 MiB BOOT_SERVICES_DATA
island (the loader heap arena's placement) that splits a 93 MiB conventional run —
`0xb0294000-0xb0a7f000` with reconnect, `0xb0a7f000-0xb1269000` without. Position
moves boot-to-boot; the island is a descriptor-count contributor (+15 total), not a
size problem.
