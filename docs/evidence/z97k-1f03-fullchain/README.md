# Z97-K / 1f03 full-chain closure (issue #122, P3, 2026-11-10/11)

The three-act closure of the oldest AMI bench in the matrix (ASUS Z97-K
R2.0 + Xeon E3-1231 v3, no iGPU, dGPU-direct):

| Act | Shape |
|---|---|
| P1 (v2.1.9, 2026-11-09) | transaction hangs pre-log, `loader.log` created then 0 bytes — the issue that incubated the diag1-7 ladder and the cluster-carve forensics |
| P2 (v2.1.12-rc1, 2026-11-09) | transaction green (`unlock=true`), chain `NOT_FOUND` — system disk was Legacy/MBR, nothing to chain |
| P3 (v2.1.12-rc2, 2026-11-10/11) | **full chain**: MBR2GPT + pure UEFI migration, unlock -> chain Pass 0 -> Windows driver init -> **ssdiag `UNLOCKED (override values live)`, CANARY intact** |

Files:
- `rc2-p3-chain-success-loader.log` — 549 lines, rc2 built 2026-11-10
  15:40:33 +08:00, booted from a 2 GB FAT32 partition on the 870 EVO (an
  **MBR data disk**, via the firmware auto "UEFI OS" entry) — proof the
  boot media needs no GPT; only the chain target does.
- `rc2-p3-ssdiag-output.txt` — in-OS verdict: PLM OPEN, SS0
  0x88888888 / SS1 0x8, READOUT full speed, canary intact after driver
  init on 591.74 (GSP mode) — another non-621.01/630.20 surviving
  generation in the #99 family.

Platform facts recorded in the FAQ no-boot section (MBR2GPT recipe) and
the compatibility matrix TU106 section.
