# TU106 1f07 — first `\tx-acpi-dump` field run (2026-11-04)

One hibernation-resume boot from the stick with `\tx-acpi-dump.txt` present.
Two loader runs exist from that boot:

- `loader.log` — the full transaction (payload mode, `tu10x:unlock=true`,
  handoff reached, `StartImage bootmgfw` at the tail). The dump ran before the
  confirm.
- `loader-abort.log` — the next boot's run, aborted with `Space` during the
  armed countdown ("no GPU writes this boot").

## What the dump produced

`index.txt` indexes 8 SSDTs and the DSDT. **The DSDT's write succeeded; every
SSDT write failed** with `WRITE FAILED (create failed)` — the name carried a
`\acpi\` prefix although the directory handle is already `\acpi`. The index
lines were also concatenated without newlines. Both are fixed in `dca02d1`
and re-verified in QEMU/OVMF with an injected SSDT (`ssdt-00.aml` written,
one line per table, overwrite idempotent). The SSDT *contents* of this run are
therefore lost; only their keyword offsets survive in `index.txt`.

That index is still informative about where the power logic lives:

| table | len | signals |
|---|---|---|
| SSDT-02 | 9534 | `_PR3=3@0x612 _ON_=3@0x5b9 _OFF=3@0x5da _DSM=3@0x421 _STA=9@0x5b2` |
| DSDT | 173979 | `_ON_=20 _OFF=40 GPIO=2 _ADR=134 _PS0=28 _PS3=28` |

The DSDT's own root-port nodes (`PEG0/PEG1/PEG2`, `_ADR` `0x00010000..2`,
each with an empty `PEGP` child) carry **no `_PR3`**. The three power-resource
triples are added by SSDT-02 — i.e. the "who cuts the card's power" answer is
in an SSDT on this board, exactly the table class that failed to write.

`dsdt.aml` + `dsdt.dsl` (iasl 20251212) are kept for the record; the `.dsl` is
1.3 MB, so only the DSDT is here.

## Diagnostics state of this run

- `CANARY-PRE a=0xbadf5040 b=0xbadf5040`, `CANARY-SET … readback FAILED` —
  the offsets were `0x140020/0x140024`, which are not the scratch block
  (`NV_PBUS_SW_SCRATCH(i) = 0x1400 + i*4`; indices `0x20/0x21` are
  `0x1480/0x1484`). Fixed in `dca02d1`; ssdiag bumped to 0.6.7 with the same
  fix.
- `POST-GOP: … unlock NOT intact` — a false positive. On Turing the chain
  re-locks the gate on purpose; `0xffffffef` is the proven handoff value
  (`TX: TU10X PLM relock: … known-good`), and `POST-GOP` read exactly the same
  `0xffffffef` with `SS0=0x88888888 SS1=0x8` unchanged. The probe compared
  against `0xffffffff`; family-aware expectation since `dca02d1`.

**Substantive result:** the console rebuild did not touch the gate on this
machine (identical PLM and selectors at `POST-FLR` and 3.4 s later at
`POST-GOP`), which keeps the Windows-side pass on the hook for #164.
