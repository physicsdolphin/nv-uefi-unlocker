# HP Victus 16, GA107M 25a2 — first full row-1 (power-event) confirmation (2026-11-04)

Reporter `hartienyu`, HP Victus 16 (hybrid mode only — no MUX/direct option in
BIOS or Omen Gaming Hub), RTX 3050 Laptop `10de:25a2`. Two boots of
`v2.1.11-diag1` + in-OS `ssdiag` 0.6.8.

## What the two axes caught

| axis | EFI (loader) | in-OS (ssdiag) |
|---|---|---|
| canary | `CANARY-PRE: 0/0` → `CANARY-SET: 0x0164c0de/0x0165cafe readback ok`, `POST-GOP … CANARY=intact`, `HANDOFF-OUT … canary=intact pmcsr=0x0008 dstate=D0` | `CANARY_A/B … = 0x00000000/0x00000000 -> cleared` |
| FEAT | unlock live at handoff (ga/ada `ss0_unlock=true ss1_unlock=true plm_open=true`) | `PLM=0xffffff8f (closed (stock))`, `SS0=0x14645165` `SS1=0x3` (devinit-derived; the per-boot baseline in these two logs is `0x14645164`/`0x14645165` — bit 0 varies by boot) |
| POWER | — | `root port 00:01.0 VEN_8086&DEV_460D PMCSR = D3 (0x0003)`; card config space via ECAM reads all-ones (see below) |

Interpretation: FLR is excluded by the loader's own run in these very logs —
`POST-FLR: verified (selectors survive AON)` proves a GA10x FLR leaves SS
untouched, so canary-cleared **plus** SS re-derived means the event exceeded
FLR: the conventional-reset family (SBR or rail-off). On a hybrid-only Victus
with the port parked in D3, the D3cold/RTD3 path is the prime form. Either
way it is the platform's power management — FAQ relock mechanism row 1, first
field run where canary and `POWER:` agree with each other.

Contrast pair: our TU106 desktop (`docs/evidence/tu106-1f07-acpi-dump/`) read
canary intact + card and port both D0 — a write-type loss would look like
that. #148 (GA107M 25a0, Linux) was the reset-free devinit-rewrite form
(canary would have been intact); same die family, different platform,
different mechanism row — the platform-reset-density framing again.

## The reporter's two puzzles, resolved

- **"BAR0 still reads, though"**: true at probe time — the card had already
  been woken and is powered *now* (`NVAPI GC6 wake: all GPUs already D0`,
  full core block readable, driver rebuilt WPR2 `0x1fffe00`). `POWER:` is a
  snapshot; the destructive event happened earlier in the boot and the canary
  is its witness. Card config space all-ones via MMCONFIG while MMIO is alive
  has two candidate explanations (firmware MMCONFIG gap for the PEG secondary
  bus, or D3-gated forwarding) — not verdict-relevant; a cheap discriminator
  would be one ECAM read of the same page in the EFI phase (port still D0).
- **`native-GOP recovery failed: unexpected EFI image count`**: the dGPU is
  the non-console card (iGPU owns the display, hybrid mode); its shadowed ROM
  carries no EFI image (`found 0`), so the rebuild correctly declines and
  leaves the console to bootmgfw. Same shape as #165's non-console card.
  `POST-GOP` readback proves it cost nothing.

## Safe-mode leg: structurally unavailable for ssdiag

`ssdiag-safe.txt`: `StartService failed: win32 1084` =
`ERROR_NOT_SAFEBOOT_SERVICE` — Windows refuses to start a kernel driver that
is not listed under `HKLM\SYSTEM\CurrentControlSet\Control\SafeBoot\Minimal`.
This is our tool's limitation (the pmxdrv vehicle), not the reporter's
machine; the safe-mode pairing leg needs either a pre-staged SafeBoot key
(unimplemented) or the zero-driver Linux live route (protocol §1b round 2).
This round did not need it: the normal-mode leg answered the question.

## Closure (2026-11-05)

The reporter confirmed the machine has no direct-connect option anywhere (not
in BIOS, not in OMEN Gaming Hub) — hybrid only, permanently. The three axes
thus settle it: **platform power management, mechanism row 1**. Closed as
completed. Reply: issuecomment-6648724585.

Two sub-questions were answered in the closure rather than left standing:

- **"BAR0 still reads, though"** — `POWER:` is a snapshot, not history. By
  probe time the driver had already woken the card (`NVAPI GC6 wake: all GPUs
  already D0`) and rebuilt `WPR2=0x1fffe00`; the destructive event happened
  earlier in the boot, where only the canary was watching. Both observations
  are true at once.
- **`native-GOP … unexpected EFI image count`** — the non-console card's
  shadowed ROM carries no EFI image, so the rebuild correctly declines. Zero
  cost, proven by the intact `POST-GOP` read.

Low-cost remainders offered, none expected to be decisive: enter from a
Windows advanced-startup **restart** rather than a cold boot (the previous
driver held a reference), disable Fast Startup and any BIOS RTD3 item, and
dump the AML to check `_PR3` idempotency and the PERST# GPIO on `00:01.0`.
Reopen on any contrary reading.

## Attachments

- `loader-normal-diag1.log` / `loader-safe-diag1.log` — the two EFI runs,
  identical except the per-boot baseline SS0 bit 0 (`0x14645164` vs
  `0x14645165`).
- `ssdiag-normal.txt` (UTF-16) — the full in-OS readback quoted above.
- `ssdiag-safe.txt` (UTF-16) — the 1084 record.
