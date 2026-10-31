# TU106 regression — v2.1.8 chain + WPR2-up claim triage (b2a4584)

- **Date**: 2026-11-01 (late night), operator-run, armed mode (`\i_know_what_i_am_doing.txt`, 5 s auto-run)
- **Binary**: `loader.efi` built from b2a4584 (post-#93 main), SHA256 `bb1647da…8ce50`, TX banner verified
- **Target**: RTX 2070 TU106 `10de:1f07` @ 0000:01:00.0, Z170-class Intel platform (8086:1901 bridge)
- **Raw log**: `loader.log` (this directory)

## Verdict: full chain green

- FWSEC fired by the loader (cold entry): **WPR2 up in 0 ms** (`lo=0x1ffe000 hi=0x1ffee00`), **GSP-HALT gate satisfied in 0 ms** (v2.1.8 default path working)
- Booter: mailbox0 = 0x0 (secure-stage success shape), EXCI INV_INS(9) as expected non-verdict; DMEM sampler "signature DMA never landed" — known non-rejection (#14/#90 field-proven false negative)
- POST-BOOT: FEAT_PLM = 0xffffffff; selectors **SS0=0x88888888 SS1=0x8 written and read back**
- Turing relock 0xffffffef (bits 5:6 sticky), FLR skipped, `tu10x_unlock=true` in HANDOFF-OUT; stock signature restored; all DMA released; chain-load proceeded

## Findings that changed the code

1. **This entry was COLD**: `SAMPLE WPR2_HI = 0` / `entry WPR2 lo=0x1ffffe00 hi=0x0 (cold)`.
   The #4.2 triage branch (`hi != 0` at claim) was therefore **not exercised** — this run
   regresses the v2.1.8 chain, not the triage. The WPR2-up entries of the sixteen-fire era
   were warm-reset shapes; a warm-reset run (reboot from Windows → boot menu → stick) is
   still owed to exercise the accept/refuse paths on hardware.
2. **Entry BSI handoff = 1, not DONE**: `BSI_SCRATCH15=0x202001bf handoff=1 (DONE=3)` —
   the handoff is the scrubber FSM (OpenRM `kgspExecuteScrubberIfNeeded_AD102`, DONE=3),
   and the firmware-window entry measures 1. The first cut of the triage anchored its
   accept on `handoff == 1` — wrong anchor (would mis-refuse entries reading 2/3, and the
   field direction may even be inverted). Corrected in the follow-up commit: the accept is
   **GFW_BOOT completed + GSP not started**; the handoff is logged but never gated. The
   measured stuck-0x101 GFW shape is now the refuse-side evidence anchor.

## Follow-up owed

- Warm-reset entry run on TU106 with the corrected binary to (a) capture the triage
  evidence line on hardware, (b) validate accept on the real warm shape.
