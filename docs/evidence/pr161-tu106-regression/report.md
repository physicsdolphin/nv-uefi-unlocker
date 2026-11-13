# PR #161 takeover round TU106 (10de:1f07) default-path regression — 2026-11-14

Booted EFI: v2.1.12-rc3 TX build (banner built Sat 2026-11-14 02:08:57 +08:00,
522240 B, sha256 49b78c1b642d91ab54a1bc26b24d9ee9aae9dd316c556fc3f89f5826118d27bd;
deployed as \EFI\BOOT\BOOTX64.EFI, previous stick binary kept as
BOOTX64.EFI.bak-5dc37f75). Markers: i_know_what_i_am_doing (armed 0s),
tx-ultrafast, tx-quiet, tx-no-console - the established armed fast bench
profile. **Both new PR #161 knobs ABSENT** (`tx-prom-sha256.txt`,
`tx-stop-on-failure.txt`) = the default field-proven behavior this round was
meant to regression-check.

Verdict: **full chain green, and the PR #161 success-tail release path
executed on hardware for the first time** - `TX: bridge BME restored (clean
success tail)` + the FINAL suffix "bridge BME restored" are in the log
(215-216). Selectors SS0=0x88888888 SS1=0x8 (shape B: PLM open, loader
writes, readback confirms), HANDOFF-OUT `tu10x: plm=0xffffffff
ss0=0x88888888 ss1=0x00000008 readout=0x00000000 | tu10x:unlock=true`,
canary intact (0x0164c0de/0x0165cafe), dstate=D0, chain resolved the
firmware's Boot0000 bootmgfw. In-OS confirmation: **ssdiag shows the unlock
OK** (operator report, 2026-11-14) - EFI readback + in-OS verdict, evidence
chain complete.

Board nuance measured: all six bridges above the GPU (8086:1901 / a167 /
a110 / a112 / a115 / a118) already carry bus master in the before-bridge-bme
dump (command=0x0007, firmware-set), so `BridgeMaster::enable` skipped all
six ("already has bus master") and the guard armed EMPTY - the success-tail
release is therefore a no-op on this board, which is exactly the
"only restore what we set" semantics (firmware-owned bits are never touched).

Not exercised this round (honest scope): the failure-path post-FLR bridge
release (no failure occurred; semantics pinned by sim/tests/tx_policy.rs),
the identity gate (marker absent -> NotConfigured, zero-cost default), and
STOP-on-failure (marker absent). The IDENTITY not-measured line is absent
because ultrafast skips the SAMPLE phase entirely (pre-existing skip, the
enrollment surface lives in sample::run).

Timing: ~2.24 s to CHAIN LoadImage (ultrafast band); log complete through
CHAIN (237 lines) - the two-rounds log-discipline (wait before pulling the
stick) held. Companion QEMU negative control for the same code:
docs/evidence/pr161-qemu-negctl/ (c5c129b).
