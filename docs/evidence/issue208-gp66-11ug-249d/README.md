# #208 — MSI GP66 Leopard 11UG (GA104M, 10de:249d): safe-mode cut = boot-path rail-off

Reporter: albertik322-sudo, 2026-11-07. Evidence gist:
https://gist.github.com/albertik322-sudo/df8362b78cf83dc197f14de9bd82a4cc

Build: v2.1.11-diag7 + the reporter's own `\tx-exit-fw.txt` patch (upstreamed as PR #209).
Machine: GP66 Leopard 11UG / MS-1543, BIOS E1543IMS.323, i7-11800H, Win11 26H2
build 26300, MUX-less, driver 621.01 (legacy mode confirmed unset), ru-RU locale.

Second `249d` canary datapoint (sibling: #171, TX9 3070M) and the first with a
completed **safe-mode cut** (nvlddmkm absent, ssdiag 0.7.0 via `--safeboot-arm`):

- `ssdiag-safemode.txt`: FEAT cluster readable (card at D0, not parked),
  canary `cleared`, `0x823xxx PLM = 0xffffff8f (closed stock)`,
  `SS0 = 0x60103120`, `SS1 = 0x00000007` (factory).
- `ssdiag-run6-normal.txt`: identical readings in normal Windows.
- Interpretation (the #37 canary table, row 2): the wipe happened **before any
  driver loaded** — boot-path rail-off directly evidenced, no longer inferred.
  The driver-exoneration is now direct evidence, not argument: a driver-init
  re-derivation would read canary *intact* (the #148 GA107M Linux shape).
- `loader.safemode-run.log`: green transaction (PLM flip, SELECTOR RESULT,
  POST-FLR verified selectors survive AON, CANARY-SET readback ok, POST-GOP
  intact, HANDOFF-OUT plm_open=true) then the `tx-exit-fw` exit; chain phase
  shows the firmware-resolve path chaining Boot0001's own choice and the
  successful preload — i.e. path/volume/image-byte resolution was exact, and
  the `\BCD 0xc0000034` on StartImage is a boot-context difference, not a
  resolution failure.
- `powercfg-a.txt` (ru-RU): S3 and hibernate available; S1/S2/S0-low-power-idle
  all unsupported by the firmware (no modern standby).
- `ssdiag-inventory.txt`: the 0.7.0 inventory block (the platform context the
  issue templates ask for, self-contained).

Verdict: platform power-management class (ACPI `_PR3`/`_OFF` D3cold rail-off,
the #37 family) — `platform restrict` / `not a bug`. Windows-side unfixable;
the only lever aimed at the gate itself is a Linux `acpi_override` table patch.
Finding A (chain-loaded bootmgfw mis-resolving `\BCD` on this AMI) is fixed by
the `\tx-exit-fw.txt` marker, PR #209.
