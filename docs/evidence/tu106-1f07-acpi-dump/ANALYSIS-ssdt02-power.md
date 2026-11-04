# SSDT-02 analysis — who owns the dGPU's power on this board (Z170, TU106 in PEG0)

Answer to the question `\tx-acpi-dump` exists for. The table is
`PegSsd/PegSsdt`; all three root ports of `\_SB.PCI0` carry a full
power-resource trio, and the card sits on PEG0 (`00:01.0` → `01:00.0`).

## The layout

Per port (PEG0/PEG1/PEG2), `ssdt-02.dsl:131`, `:376`, `:621`:

```
PowerResource (PG0x, 0, 0)
{
    Name (_STA, One)
    Method (_ON)  { if (OSYS > 0x7D9 && PEGS()) { PGON(x); _STA = One } }
    Method (_OFF) { if (OSYS > 0x7D9 && PEGS()) { PGOF(x); _STA = Zero } }
}
Name (_PR0, Package { PG0x })      // D0
Name (_PR2, Package { PG0x })      // D2
Name (_PR3, Package { PG0x })      // D3hot  <-- this is what enables D3cold
Method (_PRW) { Return (GPRW (0x69, 0x04)) }
```

So Windows is *allowed* to put the root port into D3cold, and doing so runs
`_OFF`, which is a **full slot power cycle, not a link state change**
(`PGOF` → `GPPR(port, 0)` → `ssdt-02.dsl:2343`):

```
SGPO (port, HRE, HRG, HRA, One)    // PERST# asserted (Arg3=1 -> no invert)
SGPO (port, PWE, PWG, PWA, Zero)   // rail enable driven off (Arg3=0 -> inverted)
LKDS (port)                        // link down
DIWK (port)                        // hand wake to the EC
```

`_ON` is the mirror image and drives the reset line explicitly
(`ssdt-02.dsl:1176`, `:2369`):

```
SGPO (reset, 1)  SGPO (power_enable, 1)  Sleep (DLPW)
SGPO (reset, 0)  Sleep (DLHR)            // PERST# released -> link train
```

The pins (`HRE0/HRG0/HRA0`, `PWE0/PWG0/PWA0`, group `SGGP`) come from the
Setup NVRAM block described in `ssdt-01.dsl:95`; `SGPO`/`SGPI` go through the
`\SGOV`/`\GGOV` GPIO opcodes, so these are real platform GPIOs, not EC
mailboxed.

## The two guards

Both methods start with `OSYS > 0x07D9 && PEGS()`.

- `OSYS` is the OS-version word; `0x7D9` is Windows 8.1. Any modern Windows
  passes.
- `PEGS()` (`ssdt-02.dsl:1118`) returns 1 only if
  `(CPEX & 0x0FFF0FF0) == 0x000506E0` — CPU model **Skylake-S/H** — and
  `(MODI & 0x06) != 0x04`. On a different CPU or a different PCIe mode the
  whole resource silently becomes a no-op while `_STA` still reports whatever
  it last held. This machine is Skylake-S, so the path is live.

**Idempotency**: both are safe to re-enter, but not because of `_STA`. They
re-read the hardware through `CCHK` and bail out on the real state —
`_ON` returns when the port already answers a vendor ID or the rail-enable
GPIO reads on (`:2242`, Arg1=1 leg); `_OFF` returns when the rail GPIO reads
off (Arg1=0 leg). `IVID = 0xFFFF` (`:871`) is the "device absent" sentinel.

## What this means for the unlock

- The FEAT unlock is volatile. `_OFF` cuts the rail, so **any D3cold of
  PEG0 takes the unlock with it** — no register write involved, and the
  ARM/AON argument does not matter because the power is gone.
- The reverse direction is equally relevant: `_ON` asserts PERST# and
  retrains. A D3cold→D0 cycle therefore looks exactly like a cold boot to the
  card.
- A driver that enters D3 (not D3cold) is harmless; the `_PR3` entry is what
  permits the cold variant, and the board advertises it for all three ports.

Practical consequence: whether the unlock survives a boot is not decided in
the transaction alone; it is decided by whether Windows/some agent drives
`PG00._OFF`. That is read out by `ssdiag`'s canary and by the `_STA`/rail
state, and it is the mechanism a mitigation has to cover (hold the port in
D0, or re-apply after every D3cold).

Table index of the other dumps: `ssdt-00` SATA, `ssdt-01` Setup/GPIO provider,
`ssdt-02` **this one**, `ssdt-03` empty, `ssdt-04` xHCI topology,
`ssdt-05` CPU, `ssdt-06` sensor hub, `ssdt-07` PTID.
