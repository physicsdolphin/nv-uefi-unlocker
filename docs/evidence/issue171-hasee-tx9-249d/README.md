# 神舟 TX9, GA104M 25a2-class hybrid — 249d second muxless report (2026-11-05)

Reporter `Zen2077-lab`, 神舟 TX9, RTX 3070 Laptop `10de:249d` (GA104M), Intel
Comet Lake host (`8086:9bc5` iGPU owns the console), **no MUX / hybrid output
only**, `v2.1.11-diag1`. Reported symptom: `ssdiag` in the OS reads no unlock,
no throughput gain.

This is the second report on the `249d` ID and the mirror image of
[#28](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/28) (same die,
MUX present, direct mode, verified gains). It is the second muxless case after
[#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37) (`25a2`,
HP Victus 16), which resolved to the platform power path.

## The EFI half is clean — and effective

`loader-v2.1.11-diag1.log`:

| step | line |
|---|---|
| gate opened | `POST-BOOT: FEAT_PLM=0xffffffff`, `wait_plm: PLM already open at the first verdict read` |
| selectors **took effect**, not merely written | `FEAT_READOUT_1` `0x00400080` (factory half rate) → `POST-GOP … READOUT_1=0x00000000` (full speed, direct read) |
| survived FLR | `POST-FLR selectors: ss0=0x88888888 ss1=0x8` |
| still intact at the handoff tail | `POST-GOP: … PLM=0xffffffff … CANARY=intact` |
| handoff state | `HANDOFF-OUT … canary=intact(0x0164c0de/0x0165cafe) pmcsr=0x0008 dstate=D0 \| ga/ada: ss0_unlock=true ss1_unlock=true plm_open=true` |

So the loss is entirely after `bootmgfw`; the transaction chain is not a
suspect. Note the `tu10x: … 0xbadf5040 / tu10x:unlock=false` tail on the same
line is noise — this die's FEAT block is the GA one (`0x8238xx`), the Turing
alias (`0x4096xx`) does not exist on GA104.

## Platform shape, read straight out of the log

- console belongs to the iGPU: both GOP handles report
  `not on the target GPU - left untouched`, and the console handle's device
  path (`02010c00d041030a…0101060000020203…`) decodes to `00:02.0`;
- the dGPU has no display ownership at all, so `native-GOP … no GOP child on
  the target` is the expected outcome, not a fault;
- `SAMPLE: PMC_BOOT_0 = 0xffffffff` (whole probe plane all-ones) ⇒ the card was
  **parked/undecoded** when the loader claimed it, and had to be woken;
- root port under `01:00.0` is `00:01.0` (`8086:1901`).

Parked-at-entry + no display ownership is the same shape that tips toward
row 1 (platform rail-off) rather than a driver write, but the prior is not
the verdict — the canary is.

## Round handed to the reporter

`v2.1.11-diag2` (`BOOTX64.EFI.2.1.11-diag1` transaction is byte-equivalent;
diag2 only makes the #146 asset-pool release the default) + `ssdiag.exe.0.6.10`,
`\tx-acpi-dump.txt` on the volume root, default console path (no
`tx-reconnect-legacy.txt`), readback **within the same boot, no reboot**,
admin shell, no `--select`. Collection set: full `ssdiag` output
(`CANARY_A/B`, `CANARY:`, both `POWER:` lines, `VERDICT`), `\acpi\index.txt`
plus decompiled AML, `Get-PnpDevice -Class Display` dump, and whether the
screen blanked or the card misbehaved in Device Manager.

Verdict table given: `cleared` + FEAT factory + root port `D3` = platform
rail-off (row 1, same as #37); `intact` + SS rewritten to third values = driver
init re-derivation (row 3, see #148); `unreadable` = parked function, fall
back to the AML plus the zero-driver Linux live readback (protocol §1b).

## Attachments

- `loader-v2.1.11-diag1.log` — the reporter's run (both uploaded copies were
  byte-identical, so one is kept). Reply: issuecomment-6648723598.

## Closure (2026-11-07)

The canary round came back and the issue resolved to **row 1 — platform power
management** (the #37 family). The arc, loader and probe from the same boot
(reporter comment 2026-11-05T15:20, ssdiag asset `34432490`):

| station | reading |
|---|---|
| loader arms | `CANARY-SET … readback ok` |
| handoff tail | `HANDOFF-OUT … canary=intact(0x0164c0de/0x0165cafe) pmcsr=0x0008 dstate=D0` |
| in-OS | `CANARY_A/B = 0/0 -> cleared`; `PLM = 0xffffff8f (closed (stock))`, SS factory (`0x02217012`), `READOUT_1 = 0x00400080` (locked signature), `VERDICT [ampere]: NOT_UNLOCKED` |
| power | **root port `00:01.0` PMCSR = D3** — the `_PR3/_OFF` path ENGAGED |

Canary armed → intact through the handoff (card D0) → cleared in-OS with the
FEAT block back at factory and the root port parked in D3 = a rail-off reset
between the handoff and the probe. Not driver re-derivation (that row keeps the
canary intact with SS rewritten), not the #164 write shape (canary intact, gate
closed), no tunnel involved. The reporter's own observation — no black screen,
no flicker, Device Manager clean — fits the quiet-RTD3-parking shape exactly:
the card parks, the driver wakes it transparently, only the unlock dies with
the rail.

One reading discipline recorded with it: `cleared` is only meaningful when the
same boot's loader armed the canary (diag1 does; v2.1.10 does not — a 0/0
probe after a v2.1.10 boot reads "never armed", not "cleared").

The decompiled DSDT the reporter delivered carries `VOL0/VOL1/VOL2` devices
with `_PR3` and `V0PR/V1PR/V2PR` PowerResources (`_ON`/`_OFF`) — this Insyde
firmware exercises `_OFF`-style gating; the dGPU's own RTD3 methods typically
live in an SSDT on these boards. `powercfg` shows S3 available (S3 clears too).

Closed as completed 2026-11-07 (reply: issuecomment-6691807314). Remedy per
the FAQ sleep/RTD3 section; `--reapply` correctly refuses after a rail drop.
