# TU106 1f07 — `\tx-acpi-dump` field runs (2026-11-04)

Two boots from the stick with `\tx-acpi-dump.txt` present.

**Run 1** (`loader.log` from the first boot, `loader-abort.log` = the retry
aborted with `Space`): full transaction, but the dump path had three defects —
the DSDT wrote, **all eight SSDTs failed** (`WRITE FAILED (create failed)`,
name carried a `\acpi\` prefix although the handle already is `\acpi`), and the
index lines were concatenated without newlines. Fixed in `dca02d1`, re-verified
in QEMU/OVMF with an injected SSDT.

**Run 2** (`loader.log` here, the fixed build): all nine tables written,
`index.txt` one line per table, and the transaction fully green —

```
CANARY-PRE: a=0x00000000 b=0x00000000 …                       (clean, not poison)
CANARY-SET: a=0x0164c0de b=0x0165cafe readback ok
TU10X PLM relock: plm=0xffffffef known-good … unlock_persist=true
POST-GOP: FEAT_PLM=0xffffffef SS0=0x88888888 SS1=0x8 … CANARY=intact(0x0164c0de/0x0165cafe)
POST-GOP: unlock intact after the console rebuild (… tu10x handoff value …)
HANDOFF-OUT: … canary=intact … | tu10x:unlock=true
```

`CANARY-PRE = 0/0` also retires the first run's worry: the block is zero as
found, so the magics are the only writers. And the same value pair survives
the console rebuild, so the rebuild is not the writer on this machine — which
keeps the Windows-side pass on the hook for #164.

## In-OS readback (same boot, after run 2) — `in-os-ssdiag-power-0.6.8.log`

`ssdiag` 0.6.8 from the running OS: `CANARY_A/B … -> intact`,
`CANARY: intact + SS live - no reset, nothing rewrote the state after
handoff`, `VERDICT [turing]: UNLOCKED`. It marks that the driver rebuilt WPR2
(`0x01fce000`/`0x01ffee00`) and left `SEC2_EXCI=0x00003911`, i.e. the normal
started state.

Its new `POWER:` lines are the baseline for the rail hypothesis: the card
(`01:00.0`) and its root port (`00:01.0`, `VEN_8086&DEV_1901`) both read
**D0** (`PMCSR=0x0008`, No_Soft_Reset), so at probe time the platform
`_PR3`/`_OFF` path has not run and the unlock loss on this machine was a
write, not a power event. Both endpoints in D0 is also what a healthy
reference machine should show; a `D3` on the root port line is the state that
runs PERST# + rail-off.

## What the dump produced

`index.txt` indexes 8 SSDTs and the DSDT, all written. The DSDT's own root
ports (`PEG0/PEG1/PEG2`, `_ADR 0x00010000..2`, each with an empty `PEGP`
child) carry **no `_PR3`**; the three power-resource triples are added by
SSDT-02.

**[ANALYSIS-ssdt02-power.md](ANALYSIS-ssdt02-power.md)** reads that table:
`_PR3` on all three ports, `_ON`/`_OFF` guarded on a Skylake-S CPU check, both
idempotent by hardware readback, and `_OFF` = assert PERST# + kill the slot
rail. Any D3cold of PEG0 therefore costs the unlock without a single register
write.

Table index: `ssdt-00` SATA, `ssdt-01` Setup/GPIO provider, `ssdt-02` **the
power table**, `ssdt-03` empty, `ssdt-04` xHCI, `ssdt-05` CPU, `ssdt-06`
sensor hub, `ssdt-07` PTID. `dsdt.dsl`/`ssdt-*.dsl` are iasl 20251212 output.
