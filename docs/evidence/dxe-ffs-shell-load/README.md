# DXE driver form — QEMU/OVMF shell-load smoke, v2 (post-audit)

> 2026-11-01, branch dxe-driver-ffs. Supersedes the pre-audit three-run set
> (its `ffspack-tx-build.log` recorded a FAILED packaging invocation - audit
> F5e; those runs validated the pre-audit arm model and are replaced by the
> four-run protocol below, which exercises the NVRAM-arm authority).
>
> Firmware: QEMU 11.1 bundled `edk2-x86_64-code.fd` (EFI Internal Shell,
> Boot0002). Images: PARTITIONED disks (one FAT32 LBA-2048 partition) so the
> volumes carry a partition identity (audit F1 volume binding). Driver:
> `nvunlock-dxe` tx-OFF, subsystem 0x0A→0x0B by ffspack.
> Repro: `mkimages3.sh` (WSL mtools, MTOOLSRC partition=1) + the four qemu
> command lines; fresh `edk2-i386-vars.fd` copy per run except run B.

## Run A — installer app writes the NVRAM arm variable (run-install.log)

`\EFI\BOOT\BOOTX64.EFI` = app, `\tx-dxe-arm-install.txt` = `1`.

```
DXE-ARM: NVRAM variable set -> volume partition GUID 00000000000800000000000000000000
         - firmware-resident driver ARMED for this volume only
```

The identity is the MBR hard-drive node: 4-byte disk signature ++ start LBA
(0x800 = 2048). App continues its normal read-only flow (30 s confirm →
FINAL). The variable lands in `vars-a.fd` (pflash write-back).

## Run B — armed driver, same vars (run-armed2.log, loader-armed2.log)

`\startup.nsh`: `load fs0:\NvUnlockDxe-readonly.efi`; `\tx-dxe-arm.txt` = `7`.

- `FORM:` line names the build identity and `tx=false` (audit M2).
- `ARM:` line ties the run to the NVRAM variable and the matching volume
  identity (audit F1: authority = variable, not a filename).
- 7 s armed countdown (marker content, floor 1..=30 - audit N1) → PROCEED.
- NO `WATCHDOG:` line - the driver never touches the platform watchdog
  (audit M3); the BDS 5-minute timer stays armed.
- ENUMERATE 6 PciIo handles → GOP-PROBE → SAMPLE no-NVIDIA negative control
  → `FINAL: driver flow complete; BDS continues to the boot option`; the
  shell prompt returns (clean hand-back).
- `\loader.log` persisted on the armed volume (extracted below); it opens
  with the corrected build-class line `nv-uefi-unlocker v2.1.9 (read-only
  build; no GPU writes by design)` (audit M2).

## Run C — marker present, NO NVRAM authority (run-noarm2.log)

Same image, fresh vars (variable absent). **0 driver log lines**, no
`\loader.log` written: the filename alone no longer arms anything - the
core F1 fix, exercised end to end.

## Run D — app regression after the remediation (run-appreg.log)

`loader.efi` as `\EFI\BOOT\BOOTX64.EFI`, no install marker: 30 s confirm →
enumerate → no-NVIDIA → `CHAIN: failed: NOT_FOUND` (expected in QEMU) →
FINAL. Application semantics unchanged.

## Packaging transcripts (audit F5e / N8-6)

- `ffspack-build-ro.log` / `ffspack-build-tx.log`: successful `ffspack
  build` for the read-only AND the tx driver build (a tx-build FFS now
  exists in-tree), both pass the strict self-verification.
- `ffspack-verify-tx.log`: independent `verify` - `type=0x07 state=0xf8
  data_checksum_ok=true`, depex + identity tag + subsystem 0x0B checked.
  FFS data checksum (FFS_ATTRIB_CHECKSUM) is enabled, so payload corruption
  is detectable (audit F5); build refuses inputs without the DXE identity
  tag (F4); outputs refuse overwrite without --force (M5).

## Coverage and what this does NOT cover

Covered: NVRAM-arm authority end to end (install → match → run / refuse),
regular-file marker predicate, countdown floor semantics, provenance +
FORM/ARM evidence lines, log persistence on the armed volume, no-watchdog
behavior, boot-quiet unarmed path, ffspack identity/anti-corruption gates,
app parity.

Not covered here: FV dispatch of the FFS with a TRUE depex and the
ReadyToBoot event path (needs firmware-image insertion; OVMF FVMAIN_COMPACT
has no room - first validated on a real board at V3); tx builds on real
hardware (V2, gated); multi-boot-option ReadyToBoot re-entry; the NVRAM
fail-counter skip path after two failures (needs two injected failures).
