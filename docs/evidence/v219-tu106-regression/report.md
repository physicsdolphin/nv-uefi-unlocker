# v2.1.9 TU106 (10de:1f07) release-bits regression — 2026-11-02

Booted EFI: v2.1.9 release asset (sha256 9d2c41f9..., 335872 B; stick root
loader.efi size matches). Markers: i_know_what_i_am_doing, tx-quiet,
tx-flush-lines=0 (markers-only flush).

Verdict: full chain green - preload path taken (CHAIN-PRE start probe ok),
selectors 0x88888888/0x8, readout 0, relock 0xffffffef known-good, handoff
tu10x three-values correct. Total ~16.7 s (issue#38 optimized band); the
~1.2 s gaps are the known USB BOT class - no new stalls (P1-6 ablation pass).
New data: first EFI-cold XP3G row (PLM0=0xffffffff NO_DECODE, PLMC=badf5040,
OVR0/VAL0=0), FACTORY FEAT_READOUT_1=0x00400080 capture, TU106 relock
=0xffffffef (cross-die with TU102/2654419).
