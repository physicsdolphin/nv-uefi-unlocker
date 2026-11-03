# TU106 native-GOP bench round — Z170 + RTX 2070 (1f07), CSM on (2026-11-04)

First real-machine native-GOP run on Turing, and the round that pinned the
TU1xx GOP driver's display-topology dependence. Binary: diag.5 lineage with
the probe-diagnostics verify (loader.efi 408,576 B; banner timestamp stale
per the known build.rs cache trap — feature strings and size are the
identity). Analysis lives in [docs/csm-compat.md §3.2](../../csm-compat.md).

## Runs

| File | Boot | What it shows |
|---|---|---|
| `loader-abort-zerowrite.log` | ABORT (Space during the 1 s armed countdown) | Zero-write abort path: read-only ENUMERATE/GOP-PROBE/SAMPLE, short-form resolution hits, chain proceeds; `SAMPLE` all-ones = the dGPU entered from D3cold (monitor on the iGPU). |
| `loader-unlock-probe-diag.log` | Full unlock | Transaction green (`unlock_persist=true`, FLR skipped per design); native-GOP: shadowed ROM 131072 B → TU1xx GOP 70727→135824 B → firmware decompress → PE ok → driver started → `Supported=SUCCESS`/`Start=SUCCESS` → **probe 0/1/2: only the iGPU's GOP child exists** (hex decodes to `Acpi(PNP0A03,0)/Pci(0x2,0)/AcpiAdr`), no child on the target after 750 ms — recovery fails through to bootmgfw gracefully. In Windows: no Code 12 (hibernate-resumed session skips resource re-arbitration), dGPU directly usable, ssdiag `UNLOCKED`. |

## Verdicts recorded (details in csm-compat.md §3.2)

1. The TU1xx GOP driver (binding version `0x50009`) returns Supported/Start
   SUCCESS yet installs **no** GOP child when no display is attached to the
   GPU — driver behavior, not a loader bug; the sibling run with the monitor
   on the dGPU verifies fine.
2. native-GOP's target form = a CSM board with the display on the dGPU
   (#107/#133 lineage); the iGPU-display mixed form failing through to
   bootmgfw is the designed graceful fallback.
3. Short-form HD+FILE boot-option resolution: three field boots, all hits
   (the Z170 Windows entry is the short-form + FILE hybrid shape).
4. Hibernate-resumed Windows sessions skip resource re-arbitration — no
   Code 12; the earlier cold-boot run produced Code 12, recovered by a
   Device Manager disable/re-enable cycle which does not drop the unlock.

## Integrity

- `loader-abort-zerowrite.log` sha256 `101e43484b70f7c5c1e99e21d529d6d6b11120b610a5466ed3ec8983b0cc2`
- `loader-unlock-probe-diag.log` sha256 `6922bc2e25a50f887192824b02304f86dd23e5c85c3adc13f08578fd7d0f94b5`
