# RTX 3080 20GB on B650 — the canary's positive-control arm (2026-11-05)

Reporter `zLermontz`, same card/VBIOS/subsystem (`10de:2206`, `SUBSYS_146D10DE`,
VBIOS 94.02.27.00.14) as the failing Z390P row in
[#109](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/109), but on an
ASRock B650 PG Lightning, CSM disabled.

This is the **first field instance of the fourth arm**: nothing reset, nothing
wrote.

## The measurement

| axis | EFI | in-OS, same boot |
|---|---|---|
| canary | `CANARY-SET … readback ok` → `POST-GOP … CANARY=intact` → `HANDOFF-OUT … canary=intact pmcsr=0x0008 dstate=D0` | `CANARY_A/B = 0x0164c0de/0x0165cafe -> intact` |
| FEAT | unlock live at handoff | `PLM=0xffffffff` (OPEN), `SS0=0x88888888`, `SS1=0x8`, `READOUT_1=0x00000000`, `UNLOCKED` |
| POWER | — | card `D0`, root port `00:01.1` `VEN_1022&DEV_14DB` `D0` |

Nothing reset (canary intact), nothing rewrote the block (FEAT fully live), and
the power path never engaged (both endpoints D0). The driver on this platform
simply never re-derives.

Together with the earlier Z390P row (gate closed, SS zeroed) and #148 (GA107M
Linux, SS rewritten to derived values) that is three outcomes on one card
identity — card, VBIOS, subsystem and driver family are eliminated as the
discriminator, which leaves platform/firmware.

## The mask correction (real, and accepted)

The reporter's `DETACH: pre-claim attributes raw=0x300` line decodes, against
EDK2 `PciIo.h`, as `IO`=`0x100` | `MEMORY`=`0x200` — **no VGA bit at all**
(`VGA_PALETTE_IO`=`0x04`, `VGA_MEMORY`=`0x08`, `VGA_IO`=`0x10`; the `0x001C`
subset masks only those three). So the chain-2 capture is empty by
construction and cannot engage on this board — the #128 shape, on a second
board. It also matches the Code 12 history: the conflict tracked legacy-VGA
*ownership* (iGPU), and disappeared when Internal Graphics was disabled, not
because any attribute was restored.

Consequence taken: the empty-capture log line now states that chain-2 cannot
engage, instead of "nothing to restore", which invited the reading "absence of
conflict".

## Involuntary power event (inference, labelled)

2026-11-04 20:04, house power loss (Kernel-Power 41). The following boot read
selectors back to stock and FP16 at the locked line (465.9 F/clk/SM vs 856–873
unlocked). No canary was installed, so this cannot discriminate by itself —
row-1 consistent, not measured.

## Window semantics confirmed in the reply

The canary only exists within one boot; a power-off → next-boot pair can never
carry it (rail-off clears it). A loss during an OS session must be captured
after resume and **before** any restart. After a reboot only the FEAT shape is
available. Since this board has no in-session loss channel observed, the
remaining run (armed → idle/sleep → resume → read in-session) would close it
out, and "replay per boot" is the correct expectation either way.

## Side data

`verdict: 8 TPC(s) fused off in 3 GPC(s), no defect bit on them - override
candidates` — first field row for the 0.6.10 floorsweep block, cross-posted to
[#169](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/169). With 7
GPCs on GA102, 3 affected GPCs carrying 8 compensated TPCs is the clean-but-
disabled pattern.

Replies: [#109 issuecomment-6650530730](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/109#issuecomment-6650530730),
[#169 issuecomment-6650531164](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/169#issuecomment-6650531164).
