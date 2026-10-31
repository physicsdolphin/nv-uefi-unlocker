# ssdiag — in-OS NVIDIA register diagnosis and selector repair (Windows)

Reads the NVIDIA registers touched by the EFI unlock stage from Windows. It can restore SS overrides without rebooting on Ampere/Ada when the FEAT PLM gate remains open.

For read-back verdicts, Turing behavior, Linux checks and handoff diagnosis, use the [FAQ](../../docs/faq.en.md#ssdiag). This page covers the tool's register surface, driver lifecycle and build.

## What it dumps

- multi-GPU: every NVIDIA Display-class function is dumped (companion
  audio/USB-C functions excluded), `--select` narrows.
- core/debug block: PMC_BOOT_0, PSTRAPS (decoded BAR1/BAR0/BAR3 sizes),
  FB_SIZE_MIB, GFW PLM/scratch, BSI_SCRATCH15, VGA_WORKSPACE, WPR2 lo/hi,
  MMU PLM + lock window, GSP/SEC2 CPUCTL + mailboxes + EXCI, fuse
  rows — with `badf` bucketing (PRIV_VIOLATION / UNDECODED / FLOORSWEPT).
- FEAT clusters of BOTH families on every chip: Turing gains READOUT
  (0x409668, per nlzy); plus the Ada four-write targets, the AD10x
  64G-BAR1 line (CYA/CFG/XVE/BAR1 cap mirror), the cmp170hx 26-register
  PLM survey, and the Ampere/Ada FEAT sweep — the union of the register
  surfaces the reference unlockers touch.

Architecture identification covers Maxwell through Rubin; the FEAT unlock verdict is available only for the mapped Turing/Ampere/Ada families. Output uses colored addresses and verdicts on a console; pipes or `NO_COLOR` select plain text. Parked GPUs can be woken through NVAPI GC6Control.

## Driver: embedded, strict remove-when-done

`pmxdrv.sys` (Intel-signed, hash-pinned against the nvoc tree at build time
and re-verified at startup, fail-closed) is **bundled in the exe**. On run:

1. If a PMXDRV device is already up (e.g. deployed by nvoc), it is borrowed —
   no service changes.
2. Otherwise the driver is extracted to `%TEMP%\ssdiag-pmxdrv.sys`, loaded
   under **its own service name `SSDIAG_PMXDRV`** (kernel, demand-start), and
   on exit — every exit path — the service is stopped, deleted and the temp
   file removed. Nothing is left in the user's service list. Cleanup never
   touches services owned by other tools (the name is ours alone).

Elevation is required for the deploy path (`OpenSCManager`); borrowing an
already-running driver works unelevated only if the device ACL allows it.

## Platform gates (preflight)

Before any deploy, the tool prints three platform gates (also via
`ssdiag --dry-run`, which loads nothing):

- **Secure Boot** — the gate that actually refuses this driver. The pinned
  `pmxdrv.sys` is Intel/COMODO cross-signed (leaf cert issued 2018-10-24, no
  Microsoft root), so on a Secure Boot-ON machine it fits none of the
  [DSE cross-signing carve-outs](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/kernel-mode-code-signing-policy--windows-vista-and-later-)
  (exemptions: OS upgraded from Windows <= 1607, Secure Boot off, or a
  signing cert issued before 2015-07-29) and the load is expected to be
  refused.
- **Vulnerable-driver blocklist / core isolation (HVCI)** — informational:
  the pinned hash is **not** on the current Microsoft blocklist (verified
  2026-11 against both the OS-delivered `driversipolicy.p7b` and the fuller
  downloadable policy — no `pmxdrv` entry in any hash form, while in-list
  control hashes do match). These gates refuse nothing for this driver until
  Microsoft adds the hash; see issue #40 for the full evidence trail.

The historical "pmxdrv blocked" experience comes from a different binary: the
2010-era file circulating as `pmxdrv.sys` was WinRing0 1.2.2, which the
blocklist denies under its WinRing0 rules. Note also that 1.0.0.1003 is the
pre-fix build of the Eclypsium-disclosed flaw (CVE-2019-11147, compiled
2019-07-16, fix shipped 2019-11-12) — "not blocked" is not "patched".

## Usage

```text
ssdiag                              # dump every NVIDIA display GPU (auto-discovers BAR0 per card)
ssdiag --dry-run                    # print platform gates + discovered cards, load nothing
ssdiag --reapply                    # + SS0=0x88888888 SS1=0x8 per card where the gate is open
ssdiag --select 1F07                # narrow to functions whose DeviceID matches
```

BAR0 is discovered from the OS for each GPU. Manually supplied physical bases are not accepted; the mapping can differ between EFI, Windows and subsequent boots.

## Build

```text
cargo build --release        # run from tools/ssdiag on a Windows MSVC host
```

## Caveats

- Writes are limited to SS0/SS1 behind an open gate and verified by
  read-back; the tool never writes the PLM register (door-opening stays the
  EFI stage's job).
- BYOVD caveat applies (cmp50hx precedent): some anti-cheats frown on kernel
  driver use — this one lives for the duration of the probe only, but a
  simultaneous anti-cheat session may still complain.
- The transport protocol is copied byte-for-byte from nvoc
  `core/src/kmd/pmxdrv.rs` (reverse-engineered + live-verified there); the
  transport definitions must match the driver protocol.

## Example output

<img width="750" height="780" alt="ssdiag register dump and unlock verdict" src="https://github.com/user-attachments/assets/d475654e-bc0a-4def-b48a-05ff6095b118" />
