# FAQ

[中文](faq.md) | [English](faq.en.md) · [README](../README.en.md) · [Compatibility](compatibility.en.md)

- [What are UEFI / DXE / chain loading? (background)](#uefi-primer)
- [What files and options belong on the USB stick?](#usb)
- [Why does Turing need GSP firmware mode?](#turing)
- [Which display and boot-entry modes work?](#display)
- [Why is there no gain on Linux despite a green log?](#linux)
- [The unlock worked but performance did not move - how do I tell whether it can?](#gain)
- [Why does an unlocked card get locked again once it reaches the OS?](#relock)
- [Does the unlock survive Windows sleep? (S3 / Modern Standby)](#sleep)
- [Does the unlock survive a Thunderbolt 3/4 / USB4 handoff?](#thunderbolt)
- [Does it work with Secure Boot on? (self-signing)](#secureboot)
- [A ghost key aborts the confirm countdown (stuck keys)](#ghost-key)
- [How do I check or repair the unlock in the OS?](#ssdiag)
- [The pmxdrv.sys security model and trade-off (required disclosure)](#pmxdrv)
- [An antivirus flags the download - what now?](#av-fp)
- [What does booter status 0x89 mean?](#bridge-bme)
- [The stick flashes and bounces back to the boot menu (or straight into the OS) - no loader.log?](#no-boot)
- [Why does the platform reset after a successful transaction?](#reconnect)
- [How do I recover from a black screen, STOP or driver failure?](#recovery)
- [What are the safety rules and implementation limits?](#safety)
- [How does the unlock work?](#mechanism)
- [What should I know about combining this tool with nvoc?](#nvoc-results)
- [How do I report an incompatibility or failure?](#reporting)
- [Where are the code and research records?](#layout)

<a id="uefi-primer"></a>

## What are UEFI / DXE / chain loading? (background)

**UEFI** is the successor to the BIOS: the firmware program a board runs after you press the power button and before any operating system starts. It initializes hardware, enumerates PCIe devices and decides which file on which drive to hand control to next; it is itself a small runtime environment - with memory allocation, filesystem drivers, and the ability to load and execute `.efi` applications.

**DXE** (Driver eXecution Environment) is the main phase of the UEFI boot: the firmware first brings up the most basic drivers, then loads each PCI device's driver in turn - the GPU's GOP (display output), the NVMe/USB storage controllers all come up here. **This tool is a UEFI application loaded by the firmware during DXE**: at that moment no NVIDIA driver exists yet, while the firmware's own initialization of the GPU is already complete - which is exactly the only programmable window "before the driver loads".

**Chain loading** means a UEFI application, once its own work is done, uses `LoadImage`/`StartImage` to load the next boot program (Windows' `bootmgfw.efi`, or GRUB on Linux) into memory and hands control over - a relay baton pass. This tool chains back into your drive's existing boot program automatically after unlocking, so "unlock from the stick" and "boot normally into the OS" are one and the same power-on.

**How does UEFI reach the operating system?** The firmware walks its boot order, finds a boot program in an EFI System Partition (ESP) and executes it; the Windows boot manager takes over, loads the kernel and the drivers, and the firmware then exits the stage entirely. The unlock this tool wrote lives on the GPU's always-on island (AON) and is inherited unchanged by the OS - that is what "volatile override, replayed every boot" means.

**Why does every operating system work?** The unlock happens **before** the OS starts and acts on the GPU's hardware rate selectors; the tool neither knows nor cares which system comes next - Windows, Linux, Hackintosh are all just "the next program to chain-load", so the flow is identical for all three. What does depend on the OS is persistence **after** boot (whether driver init re-derives, whether sleep drops power) - see the [relock](#relock), [sleep](#sleep) and [Thunderbolt](#thunderbolt) sections.

<a id="usb"></a>

## What files and options belong on the USB stick?

The maker emits the assets into a local `usbroot/` directory (the `*.bin` files flat, plus `EFI/BOOT/BOOTX64.EFI`). Copy the **contents** to the stick root (do not copy the directory itself as `U:\usbroot\`). The output now contains only the runtime set - extraction intermediates (the full GSP ELFs, the forensic `*_raw.c` dumps) no longer mix into it; set `ASSETS_DEBUG_C=1` to have the assets pipeline emit the forensic dumps again (debugging only).

**No-copy install (optional)**: after generating, the maker probes for EXISTING FAT32 partitions and offers a one-click install - an internal (non-USB) disk's **non-boot** (non-ESP, non-system/boot) data FAT32 partition is offered as a persistent install (boot that volume from the firmware's one-time boot menu afterwards; not every firmware lists internal FAT partitions - the USB stick stays the universal path), and a USB stick's FAT32 partition for the same copy. The install only ever **appends files**: nothing already on the target is overwritten or deleted, and differing same-name files refuse the whole target and get listed one by one (only a root `nvuu-stick.json` marking a previous nv-uefi-unlocker layout allows an in-place upgrade). No partition is ever created/resized/reformatted, the system ESP is never touched, and no NVRAM boot entry is written. When neither probe yields a target, the script prints how to prepare either route (carve a non-ESP data FAT32 partition on an internal disk, or use a stick with a large-enough FAT32 partition). For scripted runs use `-NoInstallPrompt` / `--no-install-prompt`, or `-InstallTo D:\` / `--install-to <mountpoint>` to name an existing target (same eligibility checks).

**Why does that path get recognized automatically?** UEFI firmware carries a fixed fallback boot path for **removable media** - `\EFI\BOOT\BOOTX64.EFI` on x86_64. A file at that path is listed and executed as a boot entry on its own, **without** writing or modifying any NVRAM boot entry - which is why the "no persistent boot entry" discipline ([safety](#safety)) holds: pull the stick and the machine is back to stock. Other architectures have their own default file names (aarch64 uses `BOOTAA64.EFI`); the tool, the assets and every field measurement so far are x86_64-only - if you need aarch64 (ARM laptops/servers), please open a feature request or PR describing the platform and firmware; the developer currently has no test hardware for it.

**`-Out G:\` fails on Windows PowerShell 5.1 with `New-Item : The path is not of a legal form`** (a bare drive root ends in `\`, which `New-Item -Path` rejects; [#195](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/195)). The script now strips the trailing backslash when creating the directories; on older packages pass **`-Out G:`** (no `\`).

`F` is `ad102` (40 series), `ga102` (30 series), `tu10x` (20 series) or `tu11x` (16 series, RE-derived experimental route); the family selector also takes a comma list (`30,40` - the mixed-generation rig form) or `all`:

| Location | Files | Purpose |
| --- | --- | --- |
| `EFI/BOOT/` | `BOOTX64.EFI` | Boot application; back up an existing file before replacement |
| Root | `booter_load_F.bin` | Booter bundle (BTL1) |
| Root | `gsp_boot_F.bin`, `gsp_boot_desc_F.bin` | GSP boot image and v5 descriptor |
| Root | `fwimage_F.bin`, `fwsignature_F.bin` | GSP-RM image and stock signature |
| Root | `v67-F-four-plm.bin` | Unlock payload |
| Root, Ada / Ampere | `scrubber_ad10x.bin` | SEC2 scrubber |
| Root, Turing | `sec2_bl_<tu10x|tu11x>.bin` | FWSEC bootloader |
| Root, Turing | `gsp-gate.txt` | Maker's GSP check record |
| Root, after execution | `loader.log` | Stage log; save before another run |

Removing `v67-*.bin` selects replay: no unlock payload, but drivers are still detached, signed firmware runs and the GPU may reset. **Replay is not read-only.** Build with `cargo efi` for read-only sampling.

Optional markers go in the stick root; normal use needs none:

| Marker | Behavior |
| --- | --- |
| `tx-require-d0.txt` | Abort before GPU writes/reset when entry is not D0 |
| `tx-select-bdf.txt` | Contents `bb:dd.f` or `dddd:bb:dd.f`; process only that card; invalid or absent target never selects another card |
| `input-reset.txt` | Optional per-device USB port reset before CONFIRM: one `usb VVVV:PPPP` per line (hex VID:PID, `#` starts a comment). Listed devices that are present get one UsbPortReset and a 1.5 s re-enumeration wait - for devices whose phantom key starts the moment the firmware USB stack comes up (USB mice/receivers exposing a keyboard interface); a port reset does not help every device (observed), in which case the INPUT-SETTLE ghost-key suppression is the fallback. Each USB interface also logs one `USB-INV` line to identify who is reporting keys |
| `tx-ignore-key.txt` | Phantom-key filter: one key per line, `#` comments, written exactly as an ABORT line prints it (`Special(LEFT)`, `Printable('z')`) or as `0x1A` / `scan 0x104`. Listed keys are treated as not pressed in every key wait (CONFIRM, armed countdown, chainload picker). **Esc can never be ignored.** Up to 8 entries; unreadable lines are logged and skipped. Zero-rebuild fix for a stuck key (EC-synthesized key, USB mouse/receiver with a keyboard interface): read the value from your own `loader.log`, write it here |
| `chain-target.txt` | First nonempty, non-`#` line specifies an EFI path tried on each volume before the default order; missing path is logged and falls back. **Check this first when the firmware BootOrder holds a stale cross-disk entry**: the default resolution follows BootOrder and can land on another disk's old loader ([#204](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/204) hit a leftover `\EFI\ubuntu\shimx64.efi` — green transaction, chain into Linux); put `\EFI\Microsoft\Boot\bootmgfw.efi` on the first line to pin the local chain target (the maker menu can write this for you: Windows / Ubuntu / systemd-boot / distro GRUB and shim / a custom path, default no pin) |
| `tx-preload.txt` | Enable the pre-transaction OS-loader preload (P2-7, TOTORONG craft): LoadImage bootmgfw before the transaction, StartImage it right after the handoff PE probe. **The default since 2026-11-09 is NO preload** - the post-transaction firmware BootOrder-resolved chain is the baseline: the [#198](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/198) nine-cell matrix matched "preload + dirty card" to every post-handoff logo freeze, while "preload + clean card" (readonly) and "no preload + dirty card" (v2.1.5 run-10, which predates the feature) both boot; the preload's original robustness motive (a post-FLR chain failing on a degraded allocation landscape) is covered by the shared firmware BootOrder resolution (#128/#142). Add the marker only to reproduce the old behavior for A/B runs; the retired `tx-no-preload.txt` marker no longer has any effect |
| `no-reconnect.txt` | Skip target GOP reconnect for a post-transaction reconnect hang/reset |
| `tx-reconnect-nonrecursive.txt` | Non-recursive reconnect only, keeps the vendor GOP binding (console stack left to bootmgfw to rebuild); gentler than `no-reconnect.txt`, for the "boots fine but hangs in reconnect" family (issue #52 X470, [Discussion #131](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/131) X99/C610) |
| `i_know_what_i_am_doing.txt` | Auto-run after an N-second countdown, any key aborts; the first marker line sets N (0-30, default 5). **0 = no wait**: PROCEED fires as soon as the read-only prep (enumerate/select/asset reads into RAM) finishes - the abort window is the prep itself, key polls still run every tick; legal since v2.1.12-rc1 (previously clamped to 1). The armed path fills the countdown window with read-only prep, so the auto-run starts from memory |
| `tx-flush-lines.txt` | First-line number (0-4096) overrides the count-based log-flush cadence (default every 16 lines; 0 = stage-marker lines only). Raising it reduces the occasional USB slow-write stalls (2026-11-01 TU106 run: four ~1.2 s stalls, all flush-aligned) at the cost of losing more of the log tail on a hang; stage-marker lines always flush |
| `tx-quiet.txt` | Quiet mode (issue #38): the console mirrors verdict lines only (banner/confirm/per-card target and result/HANDOFF/FINAL/CHAIN) while `loader.log` still records every line; if the log file cannot be opened the console stays complete. Turns minutes of echo into seconds on iGPUs that render ~1 line/s; combined with `i_know_what_i_am_doing.txt` it is a fully unattended fast run. A frozen screen during the transaction is normal, not a hang |
| `tx-gop-disconnect.txt` | Diagnostic switch: disconnect every GOP handle (including the iGPU) instead of only the target card. **Not a speed lever** - once the console is silent it buys nothing, and the other card's console is never restored by the loader (it depends on bootmgfw rebuilding it; a known Code 12 risk shape, see issue #35) |
| `tx-reconnect-legacy.txt` (tx builds) | **Opt-out**: restores the old reconnect path (recursive ConnectController + PR#13 non-recursive logic). The default has moved to native-GOP console rebuild (the card's own shadowed-ROM GOP); use only if the new default misbehaves on your board |
| `tx-native-gop.txt` (tx builds) | **Optional pin**: native-GOP is now the default, so this marker is no longer an enable switch; its first line may name a volume GOP module to use instead of the card's shadowed ROM (compatibility-package form); empty/absent = the shadowed ROM |
| GOP handle-order reorder (**default ON**, tx builds, issue #228) | After a native-GOP rebuild the console card's child is the NEWEST GOP handle (tail of the list), while grub gfxterm / the Linux efistub / bootmgr all take the FIRST GOP (sink-blind) - on multi-GPU + headless-BMC topologies the handle-order flip makes them draw into a headless output (#228 C621E: grub and plymouth fully dark, Windows only recovered at driver takeover). After a verified console rebuild the EDID-less competitor's controller drivers are stopped and restarted (fresh child appends to the tail) so the console child sorts first; competitors keep working. Four gates must ALL pass before any write: the verified child is enumerated + carries EDID (sink proof) + the flip is in place + competitors are EDID-less and not the console card itself; any failure = loud log + zero writes, with a post-surgery readback. Healthy-machine negative control: console already first = Noop, zero writes. **Scope = the in-session chain** (unlock -> handoff -> OS, the per-boot replay workflow); warm reboots rebuild the handle order firmware-side and are out of scope |
| `tx-gop-no-reorder.txt` (tx builds) | **Opt-out**: skips the default GOP handle-order reorder (the GOP-ORDER dump still lands in the log); only for boards where the default misbehaves |
| `tx-gop-reorder.txt` (tx builds) | Legacy enable marker: the reorder is now the default, so this marker is still accepted but is a **no-op** (kept for stick compatibility; safe to delete) |
| `tx-gop-hide.txt` (tx builds) | **Manual fallback**: one entry per line, a BDF (`bb:dd.f` or `dddd:bb:dd.f`) or vendor:device (`vvvv:dddd`); matching GOP children are disconnected without recreate. Refusals: a list entry hits the console card itself (sibling-head rule, **the whole operation is refused**), no parseable line (refusing to guess), a failed post-surgery readback is reported loudly. Mutually exclusive with the default reorder: hide runs first, reorder never runs |
| `tx-prom-sha256.txt` (tx builds) | **Opt-in identity gate**: 64 hex digits, the pinned SHA-256 of the fixed 1 MiB PROM window at `BAR0+0x300000`. Absent = not configured, the transaction proceeds as always. Present = required to match (two stable reads) before `run_transaction` is ever called; a missing/unparsable/unreadable/mismatched measurement refuses the card before any write, same as an unsupported asset set. Enrollment: when this file exists, every build (read-only included) logs the measured `IDENTITY:` SHA-256 - and PROM reads cost nothing on default boots. Boot once with a placeholder value, copy the logged value back into the file |
| `tx-stop-on-failure.txt` (tx builds) | **Opt-in STOP-on-hard-failure**: existence enables it, contents ignored. Default (absent) behavior is unchanged - a generic transaction failure runs `emergency_flr` and the multi-GPU loop continues to the next card. With this marker present, a generic failure instead halts immediately: no FLR, no bridge-BME release, no next card, and the loader does not chain-load Windows. The failure path's console re-enable and read-only verdict lines still run, but the screen may stay dark (the GOP is deliberately not reconnected on a card whose state is no longer trusted) - `loader.log` is the sole record. A stopped session also never StartImages the OS loader, so successive attempts measure an identical pre-OS chain into the TPM (relevant to BitLocker-style PCR-sealed profiles: recovery boots keep unsealing) |
| `tx-no-canary.txt` | Skip the whole reset canary group (A/B control; see [relock](#relock)) |
| `tx-relock.txt` (tx build) | **Escape hatch**: relock the FEAT PLM to `0xffffff8f` before handoff (all three families). The default since 2026-11-08 is to hand off with the gate OPEN everywhere (Turing joined the Ampere/Ada open-gate shape; a TU106 bench proved the open gate healthy end-to-end and leaves the in-OS write surface available, see `docs/evidence/tu106-1f07/keep-open-2026-11-08/`). The Turing arm's observed relock value is `0xffffffef`/`0xffffff8f` depending on the board; the Ampere/Ada arm (relock after the FLR verdict) has never been benched. **The relock itself has a cost**: some drivers answer a relocked state with an init-time PF FLR that wipes SS ([#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99) same-card A/B), so the default through v2.1.11 (Turing arm only; GA10x/Ada always handed off gate-open) loses the unlock on those drivers |
| `tx-ultrafast.txt` (tx build) | **Speed profile** (for machines with several verified rounds): skips every purely diagnostic pre/post probe - M1 cold-state sample, GOP probe, AUTH-SCAN, ROOT-DIR dump, opener/topology dumps, the post-payload FEAT sweep, HANDOFF-PCI / HANDOFF-VGA-FINAL dumps, the POST-GOP probe, CHAIN-MEMMAP - and folds in quiet + **transaction-anchor-only flush** (much tighter than the stage-marker set: ~80 flushes/run → ~15; a hang between anchors leaves a bigger log blind spot; an explicit `tx-flush-lines.txt` still wins). **None of the verdict readbacks are skipped**: the selector write/readback, the WPR2/POST-BOOT verdicts and the booter load audit all still run - readback is the only success criterion. **The reset canary is NOT skipped** (two writes + two reads, milliseconds; high reset-forensics value - since 2026-11-09 ultrafast arms it too and `HANDOFF-OUT` shows the real verdict; use `tx-no-canary.txt` to skip it explicitly). Every skipped stage leaves one `ultrafast - skipped` line in the log. Only recommended on machines with multiple clean recorded rounds (the maker menu can write this for you, default No) |
| `tx-no-log.txt` (tx build) | Never opens `\loader.log` (console-only output all run; quiet is structurally refused by its "console is the only record" guard, so the console stays complete). A hang means zero forensics - only for armed runs on machines with multiple verified rounds |
| `tx-no-console.txt` (tx build) | No console mirror at all (zero GOP text-output cost - the minutes-per-run slow-panel shape of issue #38 disappears). The confirm prompt is invisible - **combine with the armed marker** (keyboard semantics unchanged: any key during the armed countdown still aborts); together with `tx-no-log.txt` this is a fully silent run with no on-screen and no on-stick evidence if anything hangs |
| `tx-keep-assets.txt` | **Escape hatch**: do not hand the asset cache back before the chain (freed by default since v2.1.11, see [handoff memory](#handoff)). Only for A/B or for the "reads assets after the chain" shape; the older `tx-free-assets.txt` marker is still accepted and is now a no-op |
| `tx-low-staging.txt` / `tx-fwsec-high-staging.txt` | Manual staging-band pins: the former moves the whole booter set (meta/signature/radix/boot) below 4GiB, the latter moves the FWSEC staging back to the high region (FWSEC defaults LOW since v2.1.8). **Since v2.1.12-rc1 there is an automatic fallback (STAGING-FALLBACK)**: on Turing's HIGH default, a booter halt with `0x89` plus an intact host-side meta magic (= our write landed, SEC2's read did not deliver — the [#74](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/74)/[#156](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/156)/[#191](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/191) class) triggers exactly one LOW-band retry — no FLR, no loops; `0x91` (the opposite [#16](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/16) sub-4GB read-lock class) and a corrupt host page never trigger it, and a failed retry falls to the canonical path with a VT-d hint. The band requirement is machine-dependent in both directions; the case table lives in the [compatibility matrix](compatibility.md) |
| `tx-acpi-dump.txt` | Read-only: dump the firmware DSDT and every SSDT plus a keyword index to `\acpi\` on the stick (decompile offline with `iasl -d`; answers "who owns this card's power"). Runs before the confirm and works with no GPU present |
| `tx-exit-fw.txt` | **Escape hatch** ([PR #209](https://github.com/physicsdolphin/nv-uefi-unlocker/pull/209), issue #208 finding A): skips the chained `bootmgfw` start and hands back to the firmware boot manager after the transaction, so the firmware's own BootOrder boots Windows natively. For machines where a chain-loaded bootmgfw dies with `\BCD` 0xc0000034 (MSI GP66 11UG/AMI, field-proven: path, volume and image bytes were all exact, so the failure is a boot-context difference inside bootmgfw - only the firmware can start it correctly). **Precondition**: the persistent BootOrder must list Windows Boot Manager ahead of the stick, or the exit loops back into the loader; a one-time boot-menu entry falls through. The abort path (30 s timeout / wrong key) passes through the same check - with the marker present an abort also hands back to the firmware, which on a chain-broken machine is the better outcome (chaining would land on the Recovery screen anyway). The pre-transaction preload is skipped automatically when this marker is present |

Without the auto-run marker, only Space or Enter confirms; another key or a 30-second timeout aborts. Chainload order: Windows Boot Manager → supported Linux shim/grub/systemd-boot/Limine → generic `EFI/BOOT/BOOTX64.EFI`. The last tier offers a 5-second key-to-exit window. Slow-console speedup is the `tx-quiet.txt` console whitelist (issue #38), not async output - UEFI boot services are single-threaded and GOP text rendering is a synchronous call, so asynchrony cannot raise throughput; every log line carries a `[+ms]` stage-timing prefix so echo cost and transaction waits separate in `loader.log`.

Multi-GPU default: process all supported cards in firmware order; skip unsupported/already-unlocked cards, handle a card's failure without retrying it. Pin the BDF when only one card is intended.

<a id="auto-run"></a>

### How do I skip the 30-second key press and run fully automatically?

Put an `i_know_what_i_am_doing.txt` in the stick root - **whether that file exists is exactly the "decide at stick-preparation time whether a confirmation is needed" option**:

```
stick root\
  i_know_what_i_am_doing.txt   <- contents: 0
  tx-quiet.txt                 <- optional, quiet mode
```

The maker can also write this choice for you ([#259](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/259)): at the end of a `make-usbroot.ps1` / `make-usbroot.sh` build the script asks once for the confirm gate - **the default (plain Enter) keeps the 30-second key press**, answering `1` writes this marker (then asks for 0-30 seconds, Enter = 5). The same menu asks three more: `tx-quiet.txt` quiet mode (y/N), the `chain-target.txt` chain pin (`0` = Windows Boot Manager / `1` = GRUB, asks the distro / `2` = systemd-boot / `3` = Ubuntu / `4` = distro shim / `5` = a custom path, default no pin - the [issue #204](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/204) stale cross-disk BootOrder fix), and the `tx-ultrafast.txt` speed profile (y/N, **default No - only for machines already verified over multiple rounds**; first-run debugging depends on exactly the diagnostic stages it skips). Scripted runs pass `-AutoRun 0..30|off` (bash `--auto-run`), `-Quiet`/`--quiet`, `-Ultrafast`/`--ultrafast`, `-ChainTarget windows|ubuntu|systemd|path|off` (bash `--chain-target`) explicitly; `-NoMarkerPrompt` / `--no-marker-prompt` skips the ask. The `markers` field of `nvuu-stick.json` records what a build wrote. The menu admits only what a stick-maker can decide for the target WITHOUT symptom evidence (every default is the documented baseline); debug/diagnostic markers stay symptom-matched FAQ lookups.

- File present = no key wait: auto-run after an N-second countdown, and any key during the countdown still aborts.
- Empty contents default to 5 seconds; a number on the first line sets the seconds (valid range 0-30).
- **`0` = no wait at all**: PROCEED fires as soon as the read-only prep (enumerate / select / asset reads into RAM) finishes. `0` needs **v2.1.12-rc1 or newer**; v2.1.11 and earlier clamp it to 1 second.
- `CONFIRM: \i_know_what_i_am_doing.txt present - ARMED: Ns countdown auto-runs` in the log means the marker was picked up.

Why the key press is still the default: those 30 seconds are a **zero-write safety gate** - a timeout or a wrong key exits without touching a single register (the log says `CONFIRM: 30s timeout - ABORT`). The right way to go from "press a key every boot" to "confirm once, automatic forever" is to move the intent declaration to stick-preparation time (the marker), not to remove the gate. So **a 30-second timeout ABORT is not a failure, it is the gate working as designed** - and an in-OS read-back of `NOT_UNLOCKED` after one does not mean the tool is broken.

A countdown interrupted by itself (a `CONFIRM: key Printable(...)` line nobody typed) means the firmware is reporting a phantom key; filter it with `tx-ignore-key.txt` (see the table above and the [ghost-key](#ghost-key) section).

### Can it be installed to an internal partition for automatic boot?

Yes, but as an **expert, at-your-own-risk operation**: the shipped workflow stays USB / one-time boot menu, and the project neither provides nor recommends internal-partition boot changes for general users. Facts and boundary:

- `bcdedit /create /application osloader` produces a Windows Boot Loader entry whose `device` sits on a drive letter that dies on reboot - firmware never enumerates it; the `bootsector` flavor silently leaves `device` unset. **Neither reaches `{fwbootmgr}`** - don't spend hours there (field-tested in [Discussion #104](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/104), dual 4080 SUPER).
- The working shape (#104): shrink a data volume by ~2 GB, lay out a FAT32 partition exactly like the USB stick, then pick it from the BIOS boot menu (shows as `UEFI OS`) - manual reorder or one-time F12. Reversible from the BIOS menu back to Windows Boot Manager, nothing flashed.
- Internal system-ESP variant (field-tested in [D#249](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/249), GA102 / 300 MB ESP): `mountvol S: /S`, lay the assets flat at `S:\` with the loader at `\EFI\NVUNLOCK\BOOTX64.EFI`, write `S:\chain-target.txt` as `\EFI\Microsoft\Boot\bootmgfw.efi`, then `bcdedit /copy {bootmgr}` for a separate entry with `device partition=S:` (resolves to `\Device\HarddiskVolume1`, the volume bootmgr itself points at), `path` to the loader, `/addfirst` on top; leave the original `\EFI\Boot\bootx64.efi` alone. **Capacity**: the full ga102 package is ~162 MB and a 300 MB ESP still had 103 MB free (#29's 100 MB ESP could not hold the 80.7 MiB payload). **Two ops traps**: a firmware/Windows update reordering BootOrder silently disables it (re-run `displayorder`); without `i_know_what_i_am_doing.txt` an unattended boot is a 30 s zero-write ABORT and the in-OS read-back says `NOT_UNLOCKED` - do not misread that as "installed and useless".
- Why no automation script: for most users "my boot partition broke" is the tool's fault, and resize scenarios vary wildly (dual-boot, cross-disk entries, btrfs/zfs with no online shrink). **Policy: the recipe above is documented, automation is not provided, downstream forks may ship scripts and cross-link.**

For preparation on a different host, name the family explicitly (or `-Family all`); **when the target machine holds cards from several generations, comma-list them** (`-Family 30,40`) or just use `-Family all` - a single-generation stick skips every other generation's card at boot (with a dedicated `loader.log` line naming the missing family and the rebuild command). Use `-SkipGspCheck` / `--skip-gsp-check` when the host-side Turing GSP check does not apply; target GSP prerequisites still apply. Offline Turing archive `gsp_rm_boot_dbg.bin` selection uses `-ProvenBlob` / `--proven-blob`; length/hash validation remains mandatory. Do not substitute versions or bypass failed checks. For Windows PowerShell TLS download failures, the bash maker can run under WSL / Git Bash.

### Where do the pinned downloads come from now? (source group, 2026-11-08)

The `nvidia-utils` GSP-firmware container is fetched from a **source group with automatic fallback** ([#212](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/212): the original CachyOS archive keeps only a ~3-version rolling window per package and dropped the pinned `610.43.03` with an HTTP 404 for everyone):

1. **Arch Linux Archive** (`archive.archlinux.org`, the official rollback archive - published package versions are never purged);
2. **first-party NVIDIA `.run` installer** (`download.nvidia.com`, XFree86 tree is append-only - driver versions from 2008 remain fetchable; `us/tw/jp.download.nvidia.com` serve the same bytes);
3. the original **CachyOS v3 container** stays last as an exact-bytes resurrection channel (CF/local caches).

All three were cross-verified to carry **bit-identical GSP firmware** - the gate is content-addressed (pinned SHA-256 of the inner `gsp_ga10x.bin`/`gsp_tu10x.bin` ELFs), not container-addressed. Pin one route with `-UtilsSource arch|run|cachyos` / `--utils-source`; on total failure the report lists every source, its URL and pinned hash. Any pinned file obtained by other means (browser, another machine, proxy) can be dropped into `inputs/` unchanged - hash match accepts it offline. **Never substitute a version into the pin** (the pinned hashes are per-container bytes; 610.57/615.x simply will not match). The unlock itself, however, is GSP-version-agnostic **within your GPU's generation** (scoped: all measurements are Turing, `gsp_tu10x`): the whole 525-621 spectrum (16.7-37.1 MB fwimages) has been verified unlocking end-to-end on TU106 - GSP is just the badge that smashes the FEAT gate open. Each generation uses its own GSP lineage (40-series/AD10x consumes the `fwsignature_ad10x` section of `gsp_ga10x.bin`; its compatibility floor is the earliest Ada-launch GSP, not the Turing one). Mechanism, measured matrix and boundaries: [docs/gsp-compatibility.en.md](gsp-compatibility.en.md).

<a id="turing"></a>

## Why does Turing need GSP firmware mode?

Turing's Windows driver needs GSP-RM mode to accept the unlocked state. Legacy initialization can cause TDR / Code 43. In administrator PowerShell, run the release script:

```powershell
.\enable-turing-unlock.ps1
```

In a checkout, use `tools\windows\enable-turing-unlock.ps1`. It sets `EnableGpuFirmware=1` only for Turing display devices; `-Remove` reverses it. **Restart is required**; hibernate resume does not reinitialize the driver.

The maker checks both the registry switch and running GSP firmware in `nvidia-smi -q`; a failed check prints a loud warning but does not block the build - the TARGET machine's GSP mode is what matters, not the build host's. Host card detection reads the driver's bound-device list (`Services\nvlddmkm\Enum`) first, so display-class keys orphaned by previously removed cards (repair benches that swap many GPUs) are ignored instead of being mistaken for present adapters; each trimmed id is reported as a ghost warning. Linux Turing also needs GSP-RM. Proprietary drivers 545+ use it by default, but `NVreg_EnableGpuFirmware=0` can disable it; verify runtime state without the Windows registry script. The 30 / 40 series do not require this Windows registry prerequisite.

Turing's `gsp_boot_tu10x.bin` is the first `0x6000` bytes (24576 B) of the proven GspRmBoot archive. The maker verifies the source archive and output slice separately; do not substitute another boot image.

<a id="display"></a>

## Which display and boot-entry modes work?

Prefer dGPU-direct mode and enter USB boot through **Restart**. Disable Fast Startup to avoid a shutdown that actually follows a hibernate path.

| Entry state | Action |
| --- | --- |
| dGPU-direct, powered and initialized GPU | Run, then verify in the OS |
| Hybrid output, powered dGPU with readable registers | May run; compare before/after-handoff state |
| D3cold, BAR0 reads all `0xffffffff` | Use an entry path that powers and initializes the dGPU |
| Hibernate resume or D3-gated entry | Prefer Restart; EFI success alone does not prove OS persistence |

Firmware may initialize only the display-owning GPU. This tool cannot execute ACPI `_PS0` / PR3 to power an offline dGPU. PM preflight requests D0 and verifies it on a non-D0 entry; `tx-require-d0.txt` selects immediate abort instead.

**Mechanism and field workaround for the hybrid-output "no gain" family (#171 -> [#224](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/224), 2026-11-10)**: a Hasee TX9 3070M (`249d`, hybrid output / not dGPU-direct) previously showed "no unlock in the OS, no compute gain" ([#171](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/171), closed). **Mechanism**: in hybrid mode the vendor power management puts the dGPU into ACPI D3 after entering the OS (the family-A AON-reset/power-cut path), wiping the volatile unlock state — independently corroborated by another user ("hybrid mode wiped the unlock; switching to dGPU-direct fixed it", a bilibili comment, weak evidence, noted as corroboration only). **The field workaround = an HDMI dummy plug + the OS "duplicate displays" mode**: it makes the platform treat the dGPU as display-active so it never powers down; the reporter hypothesized this from the maintainer's D3 diagnosis and verified it on their own machine (armed chain fully green + in-OS ssdiag `UNLOCKED (override values live)`). **Hard prerequisite: at least one video port natively wired to the dGPU** — machines whose every output runs from the iGPU cannot use this; for those, switch to dGPU-direct if the BIOS/MUX offers it; with neither, this path is closed. The dummy plug itself is a one-time trivial cost (a generic accessory in the ~15-yuan class), entirely practical for hybrid-laptop owners. The record carries no performance A/B and no sleep-persistence data. Note: the 3070M (Ampere) needs no Turing GSP registry prerequisite; enabling it is harmless. **Firmware-side alternative (field-verified as of 2026-11-16)**: on machines whose firmware (hidden options included) offers an ACPI D3 switch, disabling it globally removes this power-cut path at the root — measured on a G614JV with the unlock retained 100% (see the [field record in the relock section](#relock-acpid3)); machines with no dGPU-wired output, where the dummy plug is unusable, should look for this switch first.

**Laptop internal panel and brightness (DDC-CI)**: on some laptops the old firmware reconnect left the internal link untrained after an unlock — the backlight dropped out and DDC-CI brightness control stopped responding (one confirmed case on a Legion R7000 2024; the legacy path reproduced it on the first attempt and v2.1.10 fixed the same machine). Since v2.1.10 the default native-GOP rebuild re-`StartImage`s the card's own stock GOP driver, which re-runs link training before Windows and clears the fault. On such a machine the legacy opt-out `tx-reconnect-legacy.txt` brings it back.

TU106 with GSP mode has verified successful hibernate-resume handoff, but that does not replace in-OS checks on other machines. See [OS verification](#ssdiag).

<a id="linux"></a>

## Why is there no gain on Linux despite a green log?

Check in-OS selectors and power state first. RTD3 can park an idle GPU in D3cold, clearing the volatile unlock; successful EFI read-backs do not prove continued OS persistence.

If your distribution enables `NVreg_DynamicPowerManagement=0x02` in `/usr/lib/modprobe.d/nvidia-runtimepm.conf`, override it with the same basename in `/etc/modprobe.d/`. Back up existing configuration first:

**One-shot script**: `sudo bash tools/disable-rtd3.sh` — performs every step of this section (the same-named override with backup, the device-level `power/control=on` that takes effect immediately, initramfs-tool detection, naming conflicting files, and a merged-config final check); `--check` is a read-only status report. Manual steps below:

```bash
echo 'options nvidia NVreg_DynamicPowerManagement=0x00' | \
    sudo tee /etc/modprobe.d/nvidia-runtimepm.conf
sudo update-initramfs -u
```

`update-initramfs` applies to Debian/Ubuntu; Fedora/RHEL uses `sudo dracut -f`. Follow other distributions' config/initramfs tooling as appropriate. Differently named files accumulate options, so remove conflicting entries.

Restart and check effective values rather than the saved file alone:

| Check | Command | Expected |
| --- | --- | --- |
| Driver parameter | `grep DynamicPowerManagement /proc/driver/nvidia/params` | 0 / `0x00` |
| Merged config | `modprobe -c \| grep -i DynamicPowerManagement` | One `0x00` entry |
| GPU power | `cat /sys/bus/pci/devices/<bdf>/power_state` | `D0` for the actual target BDF |
| Unlock state | `sudo python3 tools/diag-unlock.py` (checkout) | Effective SS overrides |

An unlock cleared by power loss needs USB replay; changing configuration alone does not restore it. Source: [RTD3 platform record](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/9). For the full mechanism inventory, triage and remedies see [Why does an unlocked card get locked again once it reaches the OS?](#relock).

<a id="gain"></a>

## The unlock worked but performance did not move - how do I tell whether it can?

Separate two questions first: **is the unlock still alive** (see the [in-OS read-back](#ssdiag); a closed gate or reverted selectors is a different problem), and **is the unlock alive but the workload cannot use it**. The second one has a rule, so you do not have to try workload after workload:

> **gain ≈ GEMM gain × the fraction of the workload that is rate-limited at stock clocks.**

The unlock raises the **issue rate**, not the power ceiling. So a workload that leaves the locked card well below its power limit, with SM utilisation unsaturated, converts the saved time into watts and finishes earlier; a workload that already sits on the power limit with SM pinned at 100 % only spends more watts for the same time.

One machine, one session, TB4 + AD102 (RTX 4090), `ssdiag` verified at both ends ([#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54), closed completed):

| Workload | Locked | Unlocked | Delta | Locked avg / peak power |
| --- | --- | --- | --- | --- |
| multi-reference image editing (double UNet pass) | 170.4 s | 139.0 s | -18.4% | 301 / 436 W |
| text-to-video, 15 s clip | 119 s | 89 s | -25.2% | 298 / 484 W |
| reference-to-video, 20 steps | 169 s | 169 s | 0% | 362 / 466 W |
| single-pass text-to-image | 24.8 s | 24.1 s | ~0% | - |
| quantised MoE LLM (prefill+decode) | - | - | 0% | - |

Controls for the three zero rows: the reference-to-video row already averaged 362 W with SM at 100 % while locked (three baseline runs at 169/169/169 s), and after the unlock it averaged 405 W at the same time; VRAM peaked at 22.1/24.6 GB, so paging to system memory is ruled out.

**How to use it:**

- Read the locked-state power draw and SM utilisation first, in the same session. At the ceiling with SM saturated means this workload can never show the unlock - no amount of tool tuning changes that.
- Clearly below the ceiling means the time should drop. If it does not, then check whether the gate was closed again ([locked again](#relock), [sleep and RTD3](#sleep)).
- On laptops, a shared TGP eats the extra issue rate; the measured shared-TGP case is in the [Linux no-gain section](#linux).

A third case is worth knowing beyond the rule: **the unlock is alive and the workload is compute-bound, yet the end-to-end GEMM figure still sits well below the raw-pipe convention** — CUDA libraries pick kernels through a fully static heuristic table calibrated for the stock half-speed hardware, which cannot sense the unlocked state and hands it an undersized, second-best kernel (measured on 8×RTX 4090: default vs best-in-library 1.2–1.9× apart, worst on FP8). This does not affect the [read-back verdict](#ssdiag) — silicon-state evidence and application-side numbers are two different things; to recover the difference on the application side, walk the heuristic candidates per shape and time them (autotune) or hand-write CUTLASS. Methods and per-precision recommended configurations: [Harry-Chen/unlocked-ada-gemm](https://github.com/Harry-Chen/unlocked-ada-gemm) and the [original blog post](https://harrychen.xyz/2026/11/17/unlocked-rtx-4090-gemm-perf/).

**Evidence level**: the table is one desktop machine (TB4 enclosure; setup in #54). The rule and its power criterion are what the numbers support; other architectures have different "full speed" baselines (`READOUT` semantics in the [read-back verdicts](#ssdiag)), so do not carry these multipliers to another card.

<a id="relock"></a>

## Why does an unlocked card get locked again once it reaches the OS?

Short answer: **NVIDIA is not detecting us**. The unlock is erased by three independent mechanisms stacking up - the reset semantics of the physical layer, the firmware's initialization contract, and the nature of the FEAT block itself. Nearly every "lost on the way into the OS" report involves **the platform handing the driver a reset or a power-removal event**, and is largely independent of driver version. Issues are a self-selected sample with no denominator, so any share quoted below is an order of magnitude, not a probability.

### First, what the FEAT block actually is

`FEAT_PLM` (`0x823804`) and `SS0/SS1` (`0x82381c` / `0x823820`) are named **fuse feature readout** in NVIDIA's own register headers - they are a **software shadow of fuses/straps**, not persistent configuration:

- `0xffffff8f` is not "some value", it is the PLM **power-on reset value**;
- for `SS0/SS1` it is the **override field returning to zero** that is the reset state; the other bits are strap-derived, so whether the whole register reads 0 varies per card (`#148` reads `0x57340065` / `0x7`). `#164` read `0x0`, but that sample was taken after the loss, so its baseline cannot be told apart from zeroing;
- what we write, `0xffffffff` / `0x88888888`, adds override bits to the shadow - neither fuses nor PLLs are touched.

So the unlock reverting is **factory restore**, not damage. **It also makes "override field back to zero with the gate closed" itself evidence**: a writer that only touches the gate would not bother rewriting the selectors to their baseline; only a full recompute or a whole-domain reset reverts both.

`FEAT_PLM` also lives on the AON (always-on) island - that is why it survives the in-transaction FLR (which the `POST-FLR` check verifies) and also the root of every problem here: **AON is reset only by power loss; no other reset source can reach it**.

### Triage: classify first, then look for a cause

| Observation | Meaning | Next step |
| --- | --- | --- |
| Already lost at `POST-FLR` | Erased inside the transaction | Treat as a transaction problem first |
| Lost at `POST-GOP`, intact at `POST-FLR` | Lost before bootmgfw | The card's own GOP/devinit or the platform firmware; the Windows driver is excluded |
| Intact at `POST-GOP`, gate closed in the OS with `SS` still at override values | Narrow write, gate only | Row 4 of the mechanism table |
| Intact at `POST-GOP`, gate closed and `SS` at neither baseline nor override values | Re-derivation (driver init programming actively) | Row 3; see the [#148 record](#relock-148) |
| Intact at `POST-GOP`, gate closed and `SS` back at the card's boot baseline (override field zero) | Whole-domain reset (power loss / SBR / full-chip reset) | Rows 1 and 2 |

Turing hands off with the gate OPEN by default since 2026-11-08 (`061aef8`, from v2.1.12-rc1 on; like Ampere/Ada); only `\\tx-relock.txt` produces a relocked value - do not read the marker arm's normal value as a loss; see the [Thunderbolt section](#thunderbolt) and [cold values](cold-values.md). **v2.1.11 and earlier relocked before handoff by default (Turing arm only - GA10x/Ada's payload four-write chain ends by opening the gate, so those always handed off open, and `061aef8` is a no-op on them) - at the time our largest self-inflicted relock source (mechanism row 7, [#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99)).**

**Baseline discipline:** before judging "SS wiped" you need a locked-state `ssdiag` sample **from the same boot, before the unlock**. A stock `SS0/SS1` is not always zero (#99 on the same card measured `0x00040000`/`0x6` on one stock boot and `0x0`/`0x0` on another, READOUT `0x00400080` both times); only a same-boot comparison of baseline -> handoff `POST-GOP` -> in-OS makes the SS change meaningful.

**Pairing method: use a reset canary as the witness.** FEAT alone only says the write is gone, not whether something **wrote** or the whole chip was **reset**. Since v2.1.11 the loader writes a pair of magics into the PBUS software-scratch words (`0x1480`/`0x1484`, indices `0x20`/`0x21` - earlier builds used the wrong `0x140020/24`; the old open driver only writes `0x1e`, these two are clear, and the BUS domain is cleared by FLR/SBR/rail-off) right after the FLR verify and reads them back; `POST-GOP`, `HANDOFF-OUT` and in-OS `ssdiag` (0.6.7+) each report the same state. Judge it only together with FEAT: **canary intact + FEAT gone = no reset, it was a write; canary cleared + FEAT intact = an FLR/SBR-class reset that the AON island survived; both gone = rail-off / full-chip reset.**

WPR2 is the corroborating signal: the only register the booter itself wrote this boot (the staging window base). `wpr2_lo` has travelled next to FEAT since v2.1.11 and can still be cross-checked against the canary.

Two methodological limits, both of which produce false verdicts - read them before running anything:

- **Safe-mode pairing fails two ways**: ① on MUXless machines the parked dGPU's BAR0 is unreadable, so `ssdiag` has nothing to read - safe mode can prove "the driver was not involved" but yields no sample (Catnip on [#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126)); ② even when BAR0 reads, the pmxdrv vehicle inside `ssdiag` hits `win32 1084` (`ERROR_NOT_SAFEBOOT_SERVICE`) on `StartService` - Windows refuses to start any kernel driver not listed under `SafeBoot\Minimal` (measured on #37, Victus; both the SCM and the kernel's own loader enforce that list). ② has a fix with **zero standing attack surface**: run `ssdiag --safeboot-arm` elevated in NORMAL mode - it stages only the two whitelist subkeys (the check is name-based; no .sys and no service are left behind) and proves the vehicle with a temp-layout round; in safe mode the driver is extracted and run exactly as in normal mode, removed on exit. `--safeboot-disarm` deletes the keys afterwards. ① has none - use the [Linux live read-back method](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/82) there.
- **`WPR2_HI` alone does not work**: the driver's own FWSEC rebuilds the window on the next boot, so the value changes anyway; it is only meaningful paired with `wpr2_lo` and compared bit-for-bit against this boot's handoff value.

### The mechanisms that can send it back to factory

| # | Mechanism | AON domain | Typical setting | Shape observed |
| --- | --- | --- | --- | --- |
| 1 | **Real power loss** (rail off) | reset | A root port with `_PR3` + D3cold/RTD3/Modern Standby (common on laptops, possible on desktops - all three PEG ports here), EC power gating, enclosure power-off, S4/fast startup | Gate locked + selectors at the boot baseline; `power_state` was D3cold |
| 2 | **Whole-domain reset** | reset | Platform SBR, link hot reset, full-chip SW_RESET; mostly old firmware / quirky boards | Same, and already broken at `POST-GOP` |
| 3 | **devinit replay** (driver init recomputes) | survives | Some GA10x laptops ([#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148); measured on Linux, no Windows sample of this shape yet) | Gate locked + `SS` set to derived values (neither baseline nor override) |
| 4 | **Narrow write** (gate only) | survives | A few platforms | Gate locked + `SS` intact ([#82](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/82) class) |
| 5 | **Our own tail**: the native-GOP rebuild `StartImage`s the card's ROM | - | Some platforms' ROM POST runs devinit | Broken at `POST-GOP` but fine at `POST-FLR` |
| 6 | **The OS-side connection manager (USB4 CM) establishing/re-establishing the USB4/Thunderbolt tunnel** | Reset | TB/USB4 eGPU ([#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54), closed, desktop TB4 + AD102 and laptop native USB4 + GA104 measured): Windows `Usb4DeviceRouter` and the Linux `thunderbolt` module tear the downstream bus down and re-enumerate it at bind/takeover. A firmware-CM discrete Thunderbolt host (the Intel-TB-panel kind) does not hit this row at boot, but a cable replug makes the managing stack re-establish the tunnel and triggers the same reset | Canary `cleared` (domain reset). Both OSes have a suppression: Linux `thunderbolt.host_reset=0`, Windows `Usb4DeviceRouter` `Start=4` (both measured, see the section) |
| 7 | **Our own handoff relock write** (the **Turing-arm** default through v2.1.11; GA10x/Ada always handed off gate-open) | Reset (PF FLR) | The gate is written back to `0xffffff8f` before handoff (reads back `0xffffffef`); the driver init then pushes it closed again and raises a PF FLR | SS intact at handoff, wiped in-OS; `nvlddmkm` id=14 spells out `None → PF FLR` ([#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99) same-card A/B 2026-11-12; open-gate default since `061aef8`, see `docs/evidence/issue99-1f07-x99-gate-open/`) |

Row 6 is special: the reset is performed by **whichever CM establishes/re-establishes the tunnel** — on a USB4 host that is the OS-side CM, before any driver binds, so re-arming after the takeover has no host-write path (`--reapply` is structurally dead, hot-add included); a firmware-CM host does not hit this row at boot (see the host taxonomy in the Thunderbolt section), only on replug. The only out is suppressing that teardown, which is field-verified on both OSes (command and cost in the section). Capture by chaining into Linux with `modprobe.blacklist=nouveau,thunderbolt` and `tools/diag-unlock.py` (rEFInd instead of the ISO's own GRUB, Secure Boot may stay on).

Rows 1-2 are physical events, row 3 is the firmware contract (below), rows 5 and 7 are the ones in our own code (row 7 has been off by default since 2026-11-08), and row 6 is an OS-side knob (not in this tool) - `\tx-reconnect-legacy.txt` makes the handoff tail reconnect the vendor stack instead of `StartImage`.

**Row 1 has a full field pairing caught by both axes** (2026-11-04, [#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37), HP Victus 16 `25a2`, hybrid-only platform, `docs/evidence/issue37-hp-victus-25a2/`): on the EFI side the canary was set and read back intact with the card in D0 at handoff; in the OS the canary is cleared + FEAT re-derived to factory + the root port `00:01.0` reading `D3` in PMCSR. FLR is excluded by the same log (the chain's own `POST-FLR` proves GA10x selectors survive it), so an event that exceeds FLR is the conventional-reset family, and the port in D3 tips the form to row 1. Contrast: our TU106 desktop read canary intact with both endpoints D0 - that is what a write-type loss looks like; #148 (GA107M 25a0, Linux) was the reset-free devinit rewrite (its canary would have been intact). Same die, different platform, different row. The canary table's fourth cell (`intact` + FEAT live) now has its first field instance too (2026-11-05, [#109](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/109#issuecomment-6650530730): same card, same VBIOS, same subsystem, B650 desktop - `intact` at the handoff and in the OS, both endpoints D0, `UNLOCKED`). "Nothing happened" is a reading as well: it eliminates card, VBIOS and driver from the discriminating variables and leaves only the platform.

### Who actually cuts the power on the platform

A dGPU has only two power sources: a desktop card is fed straight from the slot; a laptop / all-in-one dGPU is built into the platform and powered through a GPIO switched by the ACPI `_PR3` `_ON` / `_OFF` methods (the BOCO shape). **D3cold is defined as running `_OFF` on those power resources**, with the sequence: root port broadcasts PME_Turn_Off -> link drops to L2/3 -> PERST# asserted -> power removed. **"Desktop cards have no such path" is wrong**: measured on a Z170 desktop board (2026-11-04, `docs/evidence/tu106-1f07-acpi-dump/`) each of the three PEG root ports has a `_PR3` pointing at its own `PG0x`, reset and rail-enable are real GPIO pairs, and `_OFF` asserts PERST# and then cuts the slot rail - slot power does not mean the root port cannot cut it.

A sharper one (**hypothesis; the evidence is Linux, unverified on Windows**): at startup the OS synchronizes power resources nobody references, and ACPI permits `_OFF` on a resource that is already off; if the firmware AML is not idempotent, that redundant call really cuts power (coreboot's Brya board tracks dGPU power state specifically to dodge the open kernel's redundant calls). On this board `_ON` / `_OFF` **are** idempotent, but through hardware readback rather than `_STA`: both entry points run `CCHK` and return when the port already answers a vendor ID or the rail GPIO already sits in the target state. Repeats are therefore harmless - as long as the GPIO state is trustworthy. The hypothesis needs a board with non-idempotent AML; here the likelier path is Windows itself issuing D3cold.

This explains three things at once:

- [#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) reproduces **deterministically**, but its later rounds split the writer: with no root-port fix the canary cleared while the root port read `D3` (platform `_PR3/_OFF` rail-off); after the root-port opt-out held the rail on (`D0`) the canary was **still** cleared, pinning a second, driver-initiated reset at bind (runs 5-6); and `EnableRuntimePowerManagement=0` alone then kept the canary intact with `UNLOCKED` (run 7, see the [WinRTD3 field record](#relock-winrtd3)). The earlier "no driver bound, so the writer is not the driver" reading was too strong - once the rail is held on, the driver's runtime-D3 path is the reset initiator;
- why the `D3Cold` registry switch does nothing: it gates D3cold requests, not the startup synchronization of power resources;
- why the "entry mode" flips the outcome: entering from Windows Advanced Startup leaves the dGPU held by the previous driver, while a BIOS cold start has it parked and pulled up by the loader - unreferenced in the OS's accounting.

[#28](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/28) is a side observation: in hybrid mode the firmware enumerates only the dGPU's audio function, i.e. the whole card is parked unpowered. Modern Standby additionally requires the root port to apply a fundamental reset to downstream devices on D0->D3 (Microsoft firmware requirements), a third platform-side trigger.

Getting hit takes three things at once: the topology has the dGPU powered by ACPI/EC (or the whole link goes over USB4), nothing references it at handoff, and the machine's AML really cuts power or asserts PERST# - the third is vendor-specific, so the rate follows platform classes rather than chance. Why the desktop exceptions (Z390P, B450M-A, B350M) differ from B650 is unknown and nobody has reported CSM state. **Issues are a self-selected sample with no denominator**, so any percentage below is an order of magnitude, not a probability.

To look yourself: pull the DSDT / SSDT of the parent root port, check whether `_PR3` exists, whether `_ON` / `_OFF` are idempotent, and which GPIO carries PERST#. Half of these arguments are settled at that step. The AML says what the platform **may** do; the `POWER:` lines in `ssdiag` 0.6.8+ say **how far it got**: the card's and its upstream root port's `PMCSR` / D-state, and a port reading `D3` means `_OFF` already ran. It is a snapshot - `D0` does not prove the port never went to `D3` this boot (the canary owns that).

Still unsplit: **GC6 and GCOFF are different targets** (the former keeps self-refresh, the latter is a full power-off); the `_PR3` path and the GC6 wake path are currently discussed interchangeably and need one dedicated comparison. (An adjacent datapoint as of 2026-11-16: with ACPI D3 switched off in firmware, GCOFF no longer happens - mechanism note in the [field record](#relock-acpid3); the dedicated GC6/GCOFF comparison is still owed.)

### Why the recompute does not break the card

- **devinit is idempotent**: it is the sequence that takes a chip from "just powered" to "usable", run on every cold boot. Running it once more yields a working card.
- **Our overrides are not part of the functional contract**: PLM only changes who may write, SS only overrides a rate divisor; PLLs, geometry and link training all follow devinit.
- Conversely: **a card that still works is itself proof the full recompute ran** - an incomplete one is what breaks a card.
- The price is that failure is always **silent**: no error code, no WHEA, no degradation, only zero benefit. That is why [in-OS verification](#ssdiag) is not optional.

### Is this deliberate driver behaviour?

Three layers, three answers:

- **There is no detection.** `0x823804` / `0x82381c` / `0x823820` have **not a single definition** in any register header of NVIDIA's open kernel modules (the only relative is `NV_FUSE_FEATURE_READOUT 0x00823814`). The driver does not even have the symbols, so it cannot read, compare or decide to preserve them.
- **"Recompute after every reset" is deliberate.** It is an ASIC correctness contract: the `kbifPrepareForFullChipReset` comment reads "make sure the devinit scripts can be executed properly after the reset", the driver blocks after an FLR waiting for devinit to finish (timing out with "VBIOS boot failed!!"), and there is a dedicated timeout scale key `RMPcieFlrDevinitTimeout`. It cannot be skipped, because it establishes things far more fundamental than FEAT.
- **"No preservation of foreign overrides" is a structural omission, not a trade-off.** FEAT overrides were an internal debug facility, locked down from the factory; nobody ever needed to keep one external write across a reset. Add the AON power-loss path and **there is simply nowhere to store the fact that someone opened the gate**.

**So: not counter-measurement, but cheap to turn into one** - devinit already almost does it. The worst case always remains "the unlock silently stops working, never a dead card", which is the floor this design gives us.

### What we can do

What we cannot: **reopen the gate from inside the OS once it is closed**. Opening it requires running the signed booter inside SEC2, which belongs to the upstream kmod lineage and is out of scope here; replaying the USB transaction is the only recovery.

What we can:

0. **Rule out our own old tail first**: carriers up to v2.1.11 relocked the FEAT gate before handoff by default, and some drivers react to that with an init-time PF FLR that wipes the SS pair ([#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99), same-card A/B 2026-11-12). `061aef8` (from v2.1.12-rc1) hands off with the gate open and the SS pair as-is, so a carrier upgrade alone removes the trigger - do not retest this on v2.1.11.
1. **Block the reset sources - the firmware-free driver-level key first**: on Windows set `EnableRuntimePowerManagement=0` on the NVIDIA display-adapter registry key (one-shot script `tools/disable-rtd3.ps1`; no firmware poking, scriptable revert, verified on Intel + AMD, [#254](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/254)/[#266](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/266); **measured equivalent in-OS to the firmware switch on a G614JV**: hybrid unlock retained, GCOFF gone, gelper hot-switch to iGPU-only failing alike - 2026-11-20, see the [field record](#relock-winrtd3)); a third platform and first `2520` (MUXless 82K2) then showed the key as the only delta against a **driver-initiated reset with the rail held on** ([#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) run 7, FP16 +54% / BF16 +57%, same [field record](#relock-winrtd3)); on Linux `disable-rtd3.sh` ([Linux section](#linux)). **The firmware-side ACPI D3 switch (hidden options included) is the alternative** (field-verified 2026-11-16, see the [field record](#relock-acpid3)): global effect (Linux included, immune to driver upgrades), but it hides in a vendor menu and kills the iGPU-only mode. Then Fast Startup and Modern Standby, and PCIe idle power gating in the BIOS; never use "shutdown" as a restart.
2. **Change the entry state**: dGPU-direct first; in hybrid mode the MUX/panel ownership decides whether the device enumerates at all ([#148 record](#relock-148)).
3. **Repair afterwards**: with the gate still open, `ssdiag --reapply` / `diag-unlock.py --reapply` writes the selectors back; a locked gate means replay.
4. **Report with evidence**: the `POST-FLR` / `POST-GOP` / `HANDOFF-OUT` lines from `loader.log` (including `CANARY` and `wpr2_lo`), the in-OS read-back values (from `ssdiag` 0.6.5+, which prints a `CANARY:` verdict), `powercfg /a`, the device power state and the event log. **Plus a same-boot locked baseline** (run `ssdiag` once inside the very same boot, before the unlock): many cards' stock `SS0/SS1` already contain zeros (instance in diag protocol §4), so without a baseline "SS=0" cannot be told apart from factory. Without these it is guesswork.
5. **Turn it into a platform property**: if a platform is confirmed to have `_PR3` and to actually cut the rail, record it as a platform property rather than chasing it as a driver bug.
6. **Give the dGPU a reference**: attach an external display to a dGPU-wired port and cold start. The Linux community saying is that a monitor on the NVIDIA side blocks D3cold (anecdote); if the loss disappears, that supports the "unreferenced -> synchronized off" hypothesis. **A program-level reference is now field-verified on Windows ([D#229](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/229), 2026-11-11)**: a resident CUDA context with a real kernel submitted every 5 s (a SYSTEM scheduled task at boot) stops the RTD3 park — `nvidia-smi` still reads P8 / 1.5 W so the card is not pushed into a high-power state, the hybrid topology is untouched with no external display, and six same-boot samples over ~12 min all read `HELD` (versus a ~2 min idle wipe without the keep-alive); honest limits: no stop-the-keep-alive causal control, Modern Standby / lid close untested, and the "handoff -> driver takeover" window has no userland fix. The Linux equivalent is disabling RTD3 (the [one-shot script](#linux)).

<a id="relock-acpid3"></a>

### Field record: a hybrid laptop keeps the unlock 100% after disabling ACPI D3 in firmware (2026-11-16, maintainer's own machine)

Machine: ASUS ROG Strix G16 G614JV (RTX 4060 Laptop `10de:28e0`, hybrid output / Optimus - the standard host shape of mechanism row 1 "real power loss"). The change was a single item: **ACPI D3 support disabled globally in the machine's hidden BIOS options**. Result: **the unlock is retained 100%** — none of the OS-side compensations are needed any more (the #224 dummy plug, the D#229 CUDA keep-alive task, or switching to dGPU-direct).

**In-OS canary reading (same-day addendum)**: in the running OS, `ssdiag` (0.6.7+) read `VERDICT [ada]: UNLOCKED (override values live)` and `CANARY: cleared + SS live - a reset happened but the AON island survived it (FLR/SBR class, not a rail-off)`. This pins the conclusion on a **second independent axis**: canary cleared = an FLR/SBR-class reset did happen this boot, SS live = **the rail was never cut** - the opposite of row 1's rail-off shape (canary and FEAT vanish together). This is also the **first field instance** of the pairing table's "cleared + FEAT intact" cell (#37 Victus holds "both gone", #109 B650 holds "intact + intact", the TU106 desktop bench holds "intact + wiped" = write-type). The effect of the ACPI D3 switch is thus visible on two axes at once: the state axis (unlock retained) and the reset axis (only FLR/SBR-class resets remain, which the AON island survives by design - it resets only on power loss, every other reset source cannot reach it).

The value is directional: until now "hybrid machines lose the unlock in-OS = the platform cut the rail via ACPI D3" was an **inference** from `POWER:` / `_PR3` sampling and the [#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37)/[#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126)/[#224](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/224) field shapes; this is the first validation from the **firmware side** - remove the D3 power path at the firmware level and the loss disappears with it. Remedy order for row 1 gains a top entry: if the firmware offers an ACPI D3 switch (hidden options included), flip it off first - more fundamental than anything OS-side - and machines with no dGPU-wired output (where the dummy plug is unusable) gain a path too.
**Known cost (measured 2026-11-18, same G614JV)**: disabling ACPI D3 globally breaks the hybrid three-mode hot switch — the iGPU-only mode requires the dGPU to power down, and with D3 off that rail path no longer exists. Observable shapes: after switching to iGPU-only the dGPU never disappears from Device Manager; or it disappears but that amounts to a bare driver uninstall (driver gone, still drawing power). If you need iGPU-only (battery/runtime scenarios), flip the hidden option back or accept the mode being unavailable - self-consistent with mechanism row 1: the D3 switch suppresses the "real power-loss" reset source, blocking the unlock wipe and the power saving with the same stroke. **Swapping this switch for the driver-level key ([field record](#relock-winrtd3), AMD + Intel) does NOT buy the iGPU-only mode back** - measured on a G614JV (2026-11-20): with the switch reverted and only `EnableRuntimePowerManagement=0` set, hybrid retention and the dead GCOFF matched this switch item by item, and **the gelper hot-switch into iGPU-only failed just the same** (the switch's power-down needs the driver to initiate runtime D3, which the key blocks too; this section's earlier inference that the mode switch rides a firmware path the key does not touch is disproven by that round). The two knobs are equivalent in-OS (retention / GCOFF / hot-switch all match); the choice is purely operational: the key = firmware-free, per-adapter, script-revertible, Windows-only; this switch = global (Linux included), immune to driver upgrades, vendor-menu only.

If the firmware hides the switch: [gsetupmod](https://github.com/MikeWuPing/gsetupmod) (a third-party UEFI setup browser that parses the firmware's HII/IFR data, shows `SUPPRESS_IF`-hidden options as editable and writes back through UEFI variables) - **unrelated to this project**, but measured on the G614JV: disabling ACPI D3 there has the same effect as unlocking the hidden BIOS menu. Honest constraints: the image is unsigned (Secure Boot must be off); free for personal use, commercial use needs the author; it edits firmware setup variables - flip one item at a time and record what you changed.

**Mechanism note (2026-11-16, pinned against `reference/open-gpu-kernel-modules` 615.78.08)**:

- **Deep-state admission is negotiated between firmware and GSP, not hard-coded in the driver**: the RTD3-family GC6/GC8/GCOFF capability bits are reported by the GSP firmware via POWER_FEATURES key-values (`kernel_gsp.c`: `POWER_FEATURES_IS_{GC6,GC8,GCOFF}_RTD3_ALLOWED` → `bIsGcOffRtd3Allowed` etc.); RM turns them into `PDB_PROP_GPU_RTD3_GCOFF_SUPPORTED`, which `dynamic-power.c`'s `RmCheckRtd3GcxSupport()` consumes. GCOFF's action itself is "remove power from the whole chip" - root port PME_Turn_Off -> L2/3 -> PERST# (the driver even carries `DEFAULT_GCOFF_PEXRST_DELAY` for it) -> rail off; only platform firmware can walk that physical chain (see "who cuts the power on the platform" above), and the driver additionally evaluates the upstream port's D3cold `_DSM` (`nv_acpi_d3cold_dsm_for_upstream_port()`). With ACPI D3 switched off in firmware, the platform power resources (`_PR3`) vanish from enumeration and GCOFF loses admission - consistent with the observed "GCOFF no longer happens". (The hop "how `IS_GCOFF_RTD3_ALLOWED` learns about platform `_PR3`" is not pinned inside this tree; recorded as the observed fact plus the mapping above.)
- **NVPCF is an accelerator of this power-cut/wake cycle**: `\_SB.NPCF` is the platform ACPI device behind Dynamic Boost; the Linux driver registers an `NPCF` notifier callback and calls `\_SB.NPCF._DSM` directly (`nv-acpi.c`). Upstream reports that its notifications periodically wake a GPU sitting in D3cold (open-gpu-kernel-modules [#1300](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/1300); [#1299](https://github.com/NVIDIA/open-gpu-kernel-modules/pull/1299) tries deferring them to runtime resume) - on NVPCF machines the cycle is inherently more frequent, which matches how fast hybrid laptops lose the unlock; with ACPI D3 off in firmware the GPU never enters D3cold, so there is nothing left for NVPCF to wake.
- **The two levels of disabling are not the same**: firmware level (BIOS option; this record) is the clean one - the driver never sees a power-cut path during capability negotiation, and the whole family of deep states requiring a platform power cut loses admission at once; runtime kernel-level disabling (`pci_d3cold_disable()` / `d3cold_allowed=0`) only blocks the final D3hot->D3cold step and leaves the device in D3hot, where a driver unaware of it still walks the power-down flow (the eGPU-shaped failure described in upstream [#1228](https://github.com/NVIDIA/open-gpu-kernel-modules/pull/1228): the GPU-side flow completes, board power never drops, GSP errors and the device falls off the bus).

<a id="relock-winrtd3"></a>

### Field record: on a hybrid laptop, disabling only the driver's runtime power management kept the unlock (2026-11-18, MSI Vector 16 HX)

Machine: MSI Vector 16 HX A13VHG (MS-15M1, i9-13980HX, RTX 4080 Laptop `27a0`, BIOS `E15M1IMS.914`, driver 610.47 LEGACY, Win11 26H2). The same report ([#254](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/254) / [D#255](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/255)) tested both display modes: **dGPU-direct passed first try and was retained into Windows**; **hybrid (MSHybrid) was wiped during Windows driver init / runtime power management** - the EFI chain and POST-FLR were green, so the loss happened after the handoff.

The fix was a single item: set the DWORD `EnableRuntimePowerManagement = 0` on the **dynamically resolved NVIDIA display-adapter registry key** (one-shot script `tools/disable-rtd3.ps1`: default applies, `-Check` is a read-only report, `-Undo` restores the default; the key index changes whenever a driver is installed, so it is resolved at run time and never hardcoded; write modes need an elevated prompt), restart, replay the EFI transaction. **The maker scripts now warn proactively** (as of 2026-11-20): `make-usbroot.ps1`/`.sh` print a colored warning when "laptop chassis + runtime D3 enabled + no display output wired to the dGPU" all hold - exactly the precondition combination of this section's signature, and the unlock then survived into Windows in hybrid mode (same-boot ssdiag `UNLOCKED`, canary intact). This is a **driver-level knob** for row 1 of the mechanism table ("real power-off / RTD3") on the Windows side: it touches neither firmware nor the ACPI tables, it only stops the NVIDIA driver from initiating runtime D3 transitions for this card - and is therefore different from the G614JV "firmware ACPI D3 off" record above (which removes the platform-side `_PR3` path entirely, at the cost of losing the iGPU-only mode).

Honest boundaries: (1) the report read registers and the canary with ssdiag 0.7.9 but ran **no same-session causal A/B of the key on vs off** (the hybrid failure and success rounds are separated by a restart + replay); (2) `EnableRuntimePowerManagement` is a generic Windows display-driver power-policy key, and not every driver branch was checked to honour it; (3) use dGPU-direct when available - hybrid retention is not a guarantee on every platform. Keep the usual diagnostic order: confirm `POST-GOP` and `HANDOFF-OUT` are intact first (rule out the transaction and the handoff), then check the `POWER:` D-state lines and the **RTD3 policy line** (ssdiag 0.8.3+ reads this key's current value directly; `--inventory` also prints `GPU n RTD3:`), and only then touch the key - probing its current value does not require changing it.

**A second platform + the causal A/B added (2026-11-20, [#266](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/266), AMD Acer Swift SFX14-41G / Ryzen 7 5800U)**: a `25a2` (3050 Laptop 4GB, GA107) on a **pure-MUXless thin-and-light with zero dGPU outputs** (a dummy plug is physically impossible there, and the BIOS has no ACPI D3 switch either) ran v2.1.12-rc4 on two independent boots to `UNLOCKED` with the canary intact and a tensor-issue-rate of **2.00x** (register-level WMMA microbenchmark, FP32 FFMA control flat at 1.00x). The report also fills boundary (1) with a **same-machine causal pair**: same machine, same stick - the round without the key read canary cleared + SS factory (matching the [#262](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/262) signature item by item), and the round with only `EnableRuntimePowerManagement=0` added (+ASPM AC/DC=0) read intact + `UNLOCKED` - the key was the only variable between the rounds, so row 1 of the mechanism table (rail-off) is now confirmed on an AMD platform. **It is also the first workable path for the [#262](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/262) "hybrid + no MUX + no dGPU output" form**: on machines where the firmware switch and a dummy plug are both impossible, try this driver-level key first. Boundary update: (1) is now covered by this report (same machine, key as the only delta); one new boundary - the key lives under the driver key, which a **driver upgrade/reinstall may re-create, so re-check and re-set it after upgrades** (an upgrade round is untested); boundaries (2) and (3) stand.

**In-OS equivalence with the firmware switch (2026-11-20, G614JV 4060L measured)**: the same machine already carries the firmware ACPI-D3-off 100% retention record ([previous section](#relock-acpid3)); with the switch reverted and only this key set, all three behaviours matched item by item - hybrid unlock retained, GCOFF gone, and the gelper hot-switch into iGPU-only failing (the switch's power-down needs the driver to initiate runtime D3, so the key blocks it too). The two knobs are equivalent in-OS; they differ only in operational properties: the key is firmware-free, per-adapter, Windows-only and script-revertible (`tools/disable-rtd3.ps1 -Undo`); the firmware switch is global (Linux included), immune to driver upgrades, and hides in a vendor menu. **Offered as the preferred firmware-free fix.**

**Third platform, first native-laptop `2520`, and the first key-only save on a machine whose relock had already been attributed to a *driver-initiated* reset (2026-11-20, [#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126))**: a Lenovo IdeaPad Gaming 3 82K2 (RTX 3060 Laptop `10de:2520`, GA106M, AMD platform, root port `00:01.1` = `1022:1633`, **MUXless with zero dGPU outputs**, driver 621.01) had already produced six negative rounds - NVCP "prefer maximum performance", ASPM AC=0, endpoint D3Cold opt-out, root-port `PnPCapabilities=24` + root-port D3Cold opt-out, a benchmark raced within 60 s of boot, and a driver update 591.86→621.01. What made run 7 more than another success is what those rounds had already split: (a) **platform `_PR3/_OFF` rail-off** - run 4 read canary `cleared` with the root port in `D3`; (b) **a driver-initiated reset at bind** - run 5 held the rail on (root port `D0`) with the root-port opt-out, yet the canary was **still** `cleared`; run 6 showed the same signature on the newer driver. Run 7 added only `EnableRuntimePowerManagement=0` (on top of the run-6 registry stack): the EFI log stayed green, and in-OS ssdiag 0.7.0 read card `PMCSR=D0`, root port `D0`, **canary `intact`**, `SS0=0x88888888` / `SS1=0x8` live, `VERDICT [ampere]: UNLOCKED`. GEMM (8192³, FP32 accumulation, median of 15) went FP16 26.3-26.7 → **40.83 TFLOPS (+54%)**, BF16 26.6-26.8 → **42.20 (+57%)**, FP32 control flat (8.58-8.62 → 8.60); a fixed-seed SDXL run (Forge, 832×1216, 25 steps) went 25.3 s → **17.2-18.4 s (about -30%)** at 73 °C / ~50 W / 1770 MHz mid-load. **This is the first case where the key was the only delta against a documented driver-initiated reset with the rail held on** - it covers both layers of the two-killer shape, not only row 1's rail-off. It is also the first native-laptop `2520` and the third Windows platform (Intel MSI, AMD Acer, AMD Lenovo). Boundary: run 7 did not record reverting the earlier registry switches, so read it as run 6 + the key (single-variable against run 6, not a from-stock A/B); sleep/S4 still clear the unlock as designed, and the key lives under the driver key and must be re-checked after a driver upgrade/reinstall. The Linux equivalent remains `NVreg_DynamicPowerManagement=0x00` ([#230](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/230)). Full run matrix and readbacks: [`docs/evidence/issue126-82k2-2520/`](evidence/issue126-82k2-2520/README.md).

<a id="relock-148"></a>

### Field record: a GA107M laptop re-derived by driver init on Linux ([#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148))

This is the only record so far that separates all three suspects - RTD3, GSP-RM and FLR - in one pass, and it is worth keeping.

Setup: RTX 3050 Ti Laptop (GA107M `10de:25a0`), Linux Mint 22.3 / 595.71.05, RTD3 off per the FAQ down to `0x00` verified in `/proc/driver/nvidia/params`, `supergfxctl -m AsusMuxDgpu`. The transaction was green (including `POST-FLR: verified`); in-OS read-only BAR0 mmap:

| Register | Boot | Written by payload | In-OS, GSP on | In-OS, GSP off |
| --- | --- | --- | --- | --- |
| `FEAT_PLM 0x823804` | `0xffffff8f` | `0xffffffff` | `0xffffff8f` | `0xffffff8f` |
| `FEAT_SS0 0x82381c` | `0x57340065` | `0x88888888` | `0x57740165` | `0x57740065` |
| `FEAT_SS1 0x823820` | `0x00000007` | `0x00000008` | `0x00000007` | `0x00000007` |
| `FEAT+0x08 0x823808` | `0x00000082` | - | `0x00000082` | `0x00000083` |

Three inferences:

- **Not RTD3**: RTD3 was off and the state is lost at `modprobe nvidia`, not at idle power-down.
- **Not the FLR**: the in-transaction `POST-FLR` check passed.
- **The init path programs it actively**: `SS0`'s delta from its boot value is `0x00400100` with GSP and `0x00400000` without - the two init paths write **different bits**, and `FEAT+0x08` moves between `0x82` and `0x83`. That is a write, not a failure to preserve.

Two operational details: `resource0` **only supports mmap**, and `dd` silently returns nothing, which looks like a false "gate already closed"; on another machine the same stick found no device at all in Hybrid mode (`vendor 0xffff`) and only ran once the panel was handed to the dGPU - **panel ownership is an entry axis, not cold/warm**.

There was no gain and a slight drop: the internal panel shares the `80 W` TGP, so FP16/BF16 land at ~`0.92-0.93x`, and even FP32, which never uses the tensor path, is `0.92x` - a power-budget effect, not a tensor-rate one. **Not generalized**: `25a0` is the only Linux re-derivation case, the same family's `25ad` persists on Windows, and the OS / driver-version / MUX-EC platform-state variables are still unsplit.

### What disabling GSP on Linux buys, and what it costs

Disabling GSP is a **diagnostic lever**, not a fix: it removes GSP-RM's late devinit from the kernel init path. [#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148) concluded that re-derivation happens anyway, so it will not stop the loss - but precisely because the two modes write different bits, it is still a useful tool for identifying the writer.

How (Linux only, temporarily):

```bash
echo 'options nvidia NVreg_EnableGpuFirmware=0' | \
    sudo tee /etc/modprobe.d/nvidia-no-gsp.conf
sudo update-initramfs -u     # Fedora/RHEL: sudo dracut -f
```

After the restart verify `GSP Firmware Version: N/A` in `nvidia-smi -q` and `EnableGpuFirmware: 0` in `/proc/driver/nvidia/params`, otherwise the test is void.

**Side effect (measured, [#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148))**: dynamic power management is lost - the card idles at P0 / 1927 MHz / **26.9 W** instead of P5 / about 8 W. Throughput is unchanged, idle power and temperature are not, so keep it for a controlled A/B only. It is also a proprietary Linux-driver knob: **Windows Turing needs `EnableGpuFirmware=1`** ([Turing section](#turing)) - do not mix the two up.

<a id="sleep"></a>

## Does the unlock survive Windows sleep? (S3 / Modern Standby)

Usually not - "sleep" is an umbrella over several power paths, and most of them let the GPU re-run its power-on initialization (devinit) on wake, clearing the volatile unlock. First identify which one your machine uses:

| State | Typical on | GPU power during sleep | After wake | Remedy |
| --- | --- | --- | --- | --- |
| S0 working (display off / locked) | All machines | Retained | Unlock unaffected | Nothing to do |
| S0ix / Modern Standby | Recent laptops | Driver runtime PM may park the dGPU in D3cold (rails off) | Mostly lost; selectors back to factory (a canary-evidenced retention counter-example exists, see the mechanism notes below) | Read-only check first; often repairable with `--reapply` |
| S3 traditional sleep | Most desktop BIOS "Sleep" | Whole platform off except RAM | Always lost | Same, depending on the check |
| S4 hibernate / fast-startup "shutdown" | Laptop hibernate, Win10/11 fast startup default | Full power off | Always lost; equivalent to a cold start | Same, depending on the check |

Check which states your machine supports: `powercfg /a` in an admin prompt - "Standby (S0 Low Power Idle)" is S0ix, "Standby (S3)" is traditional sleep; machines listing both decide at runtime via firmware/driver.

Mechanism notes:

- **The discriminator is whether the rails dropped, not whether you "slept"**. S0ix nominally stays in S0, but the dGPU can be powered down independently when idle (same mechanism as Linux RTD3, [previous section](#linux)); S3 cuts the whole platform. Either way the wake-up result can look identical: selectors re-derived to factory by devinit.
- **S0ix is not a guaranteed loss - first canary-evidenced retention record (2026-11-20, [#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99#issuecomment-7041057712))**: on a GA104M `249D` (Hasee NH5xEPER, hybrid no-MUX) with v2.1.12-rc3, four Start → Power → Sleep rounds all survived with the canary `intact` - one of them **after deleting the `EnableRuntimePowerManagement` key**. An intact canary means the whole sleep→wake cycle contained no FLR/SBR, no rail-off and no writes: this machine's S0ix path keeps the dGPU powered, and "usually lost" does not happen here - sleep loss distributes by platform, it is not a law. The same machine on v2.1.6 lost on the first wake with a **different shape**: gate closed + SS at non-override values (`0x03000141`/`0x7`) - mechanism rows 1/2 (back to the boot baseline) or row 3 (re-derivation); `--reapply` answers `REFUSED: gate closed` and only a USB replay recovers. That round's attribution is open (v2.1.6 on GA10x already handed off gate-open - the relock was Turing-arm only, mechanism row 7), one negative sample cannot establish a version cause, and the reading still goes through the same-boot locked baseline + canary + `POWER:`. Note that vehicles older than v2.1.11-diag1 have no canary writer: on their boots a `0x0/0x0` canary read is the never-armed default, and ssdiag's canned reset verdict is a false positive in that scenario.
- **FLR survives, power loss does not**: the unlock lives on the AON (always-on) island - selector survival across the in-transaction FLR is a design check - but AON does not survive losing power. "Lost after reboot" and "lost after sleep-wake" can be the same event.
- **Judge the gate and the selectors separately**: the common post-wake shape on Ampere/Ada is the FEAT gate still open with SS values back at factory - run the [in-OS check](#ssdiag), then `ssdiag --reapply` writes the selectors back; no USB replay needed. If the gate is locked or the device unreadable, the rails truly dropped and the USB transaction must be re-run. **Whole-machine power-removal paths (S4 hibernate / fast startup) always lock the gate**: the AON island does not survive power loss, so `--reapply` answers `REFUSED: gate closed` — that is the correct verdict, not a tool failure, and the only recovery is re-running the USB transaction (user-side confirmation in D#129). Turing ssdiag stays read-only (its SS overrides persist across driver load; devinit re-derivation behaves differently).
- **Read the `POST-GOP` line before blaming the driver**: from v2.1.11 the handoff tail samples the FEAT triple once more (`POST-GOP: FEAT_PLM=… SS0=… SS1=…`). It sits **after** the console rebuild, which on the default native-GOP path `StartImage`s the card's own GOP driver (`\tx-reconnect-legacy.txt` reconnects the vendor stack instead, no StartImage), while the `POST-FLR` verification sits before it — only both lines together split the writer: already locked/lost at `POST-GOP` means it was lost before bootmgfw (the card's own GOP/devinit or the firmware; the Windows driver is excluded), while intact at `POST-GOP` and wiped in the OS puts the driver pass on the hook. This is the triage entry for [#164](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/164).
- **Keeping it**: on desktops, avoid idle power-down via BIOS/driver settings; on S0ix laptops, disable Modern Standby if the firmware offers it, or accept a reapply after each wake. The Linux-side knob is in the [RTD3 section](#linux).
- **The desktop pattern that gets both convenience and a durable unlock** ([#137](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/137), `1e89`, measured): disable automatic **S3 sleep** (`Sleep = Never` - an idle S3 silently wipes the unlock, canary cleared, selectors to factory), use **hibernate (S4)** instead - and keep the unlock loader in the boot path (USB-first BootOrder or the internal-partition recipe). Hibernate's next power-on is a normal firmware boot, so the loader re-applies the unlock before bootmgfw touches the hiberfile: you resume the session with the card already unlocked. Every boot replays the unlock; hibernate turns that design into the feature.

<a id="thunderbolt"></a>

## Does the unlock survive a Thunderbolt 3/4 / USB4 handoff?

First, the keyword this section turns on: the **connection manager (CM)** — in the Thunderbolt/USB4 architecture, the entity that enumerates routers and establishes and manages tunnels. **It lives either in firmware or in the operating system.** A Thunderbolt host chip carries its own Firmware CM (Windows is not involved; on the machine this shows up as the Intel Thunderbolt control panel and no USB4 section in settings). The OS-side CM is the USB4 CM — Windows' `Usb4HostRouter`/`Usb4DeviceRouter` service stack, or the Linux kernel `thunderbolt` module.

**Whether the unlock survives depends on which side establishes or re-establishes the tunnel after boot**: whoever tears the downstream bus down and rebuilds it performs a whole-domain reset that wipes the gate and the selectors out of the AON island (canary `cleared`).

| host shape | how to tell | who establishes/re-establishes the tunnel after boot | result |
|---|---|---|---|
| **Discrete Thunderbolt host chip + firmware/legacy CM** (Intel JHL8xxx-class TB host silicon, e.g. JHL8540; BIOS settings like `USB4 CM Mode = Firmware CM`) | The **Intel Thunderbolt control panel** is installed and Windows settings have **no USB4 section** (measured on the G614JV); Device Manager shows a "Thunderbolt Controller", no `USB4\ROOT_*_ROUTER` nodes | **At boot: nobody tears anything down** — the firmware-POST-built topology is inherited as-is | **Retained from the first boot.** But a cable replug makes the managing stack re-establish the tunnel → domain reset (measured on the JHL8540 machine: gate relocked, rates at factory, **canary dead `cleared`**) — so retention is **per-boot**, not hot-plug-safe |
| **USB4 host router** (integrated Intel/AMD, or discrete silicon bound by MS `usb4hostrouter.inf` such as ASMedia; includes a discrete TB chip set to OS-CM mode in BIOS) | `PCI\…&USB4_MS_CM` present, the `Usb4HostRouter`/`Usb4DeviceRouter` services exist | **At boot: the OS-side CM tears down and rebuilds as soon as it binds** (Windows `Usb4DeviceRouter`, the Linux `thunderbolt` module) | **Lost at boot.** [#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54), desktop TB4 + AD102: after the handoff, after POST-to-OS, and even inside a booted driver-less Linux the canary stayed `intact`, the gate `0xffffffff` — the loss happens only in the seconds of the CM's teardown/rebuild |
| A TB4 device on a USB4 host (backward-compat mode) | same as the USB4 row | same | same |

Evidence (the table is every known TB/USB4 experiment in this repo): the maintainer's G614JV (JHL8540 = a Thunderbolt 4 host chip + TB3 dock, forced ATX, first-boot retained / replug domain-reset); [#74](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/74)'s PHN16-72 (BIOS `Firmware CM Only` + TB4 dock on forced power, in-OS `UNLOCKED`); Ambolio (ASMedia silicon bound by the MS CM, lost at boot, retained with the suppression); GWASDW123 (Intel integrated USB4 + TB3 dock, same, and additionally the **timing route excluded** — moving both routers into early kernel boot compressed the tunnel window from 8.06 s to 4.31 s and the unlock was lost anyway; what matters is whether the teardown happens, not when); the three Linux rounds (`thunderbolt` module teardown / `host_reset=0` suppression). **#54 and #74 are both closed completed (2026-11-07)** — new host shapes or new loss shapes go into a fresh issue with a `USB4:` block and canary readings attached.

**Maintainer's conclusion (2026-11-06)**: with a discrete Thunderbolt host chip (e.g. JHL8xxx) plus a dock on forced ATX power, both TB3 and TB4 retain the unlock; for USB4 - and for TB4 running in USB4 backward-compat mode - the unlock is lost at boot, use the suppression below. Three qualifiers that must travel with it: (1) the discriminating variable is **which CM stack binds**, not "a discrete chip exists" - the ASMedia chip is discrete yet bound by MS `usb4hostrouter.inf` and tears the bus down; a discrete TB chip can also be set to OS-CM mode in BIOS, so judge by "Intel Thunderbolt control panel vs USB4 router nodes / the `USB4:` block". (2) The PHN16-72 suppression state is unrecorded; if it is also `Firmware CM Only` no suppression was needed in the first place. (3) On Linux the `thunderbolt` kernel CM takes over on every host shape (Run A) - `host_reset=0` remains the only lever there.

**`--reapply` does not save this path.** The gate is closed by that reset, before any driver (re)binds; after the teardown there is no host-write path, so an in-OS read just shows a closed gate you cannot write back. The only fix is to stop the OS-side CM from doing that teardown, and both OSes now have a measured suppression:

- **Linux**: kernel command line `thunderbolt.host_reset=0`. Measured (#54 Run C): the module still loads and the enclosure is still discovered on the Thunderbolt side, but `pci_bus … is released` never appears, the hierarchy is not torn down, and the canary stays `intact`, the gate `0xffffffff`, the verdict `UNLOCKED`.
- **Windows**: set the `Usb4DeviceRouter` service `Start` to `4` (Disabled) and leave `Usb4HostRouter` at `3` (Manual). **Disable the device side only, keep the host side** - disabling both leaves no tunnel at all. On three different host stacks (ASMedia TB4 + AD102 desktop / Intel native USB4 + GA104 laptop / Dell WD22TB4 dock + Sonnet eGPU enclosure + GA102 3090 Ti on a laptop, [D#260](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/260)) the card enumerated normally and bound `nvlddmkm`, the canary stayed `intact`, `ssdiag` returned `UNLOCKED`, and the AD102 host reached 95% of the full-rate tensor figure in an 8192³ FP16 GEMM plus a locked-clock sweep with `nvidia-smi -lgc`; the D#260 3090 Ti gained FP16 +76.9% / TF32 +81.5% with the FP32 control flat (also the **first chained dock -> enclosure topology** - the dock's USB4 router nodes showed Unknown during the unlocked session, matching the cost column). The cost, stated plainly: the enclosure loses its device-router services, its USB ports die and both `USB4\ROOT_DEVICE_ROUTER` nodes show `Problem 32` (Code 32 = service disabled, not a failure); GPU, card audio and `nvidia-smi` are all fine. `DevPnPCapabilities=0x100` is disproven; the `Usb4HostRouter` node's `Device Parameters` subkey holds only WDF/DMA/interrupt keys, no reset-policy surface.

```bat
:: Windows: suppress (admin) - takes effect on the next boot, this machine only
reg add HKLM\SYSTEM\CurrentControlSet\Services\Usb4DeviceRouter /v Start /t REG_DWORD /d 4 /f
:: restore the default
reg add HKLM\SYSTEM\CurrentControlSet\Services\Usb4DeviceRouter /v Start /t REG_DWORD /d 3 /f
```

**The suppression's causality is now single-variable closed** (#54): `Usb4DeviceRouter\Start` back to `3` with everything else untouched → canary `cleared`, `NOT_UNLOCKED`; back to `4` → `intact`, `UNLOCKED`, both directions on one machine on one day. `Start=4` persists across reboots (days on end; the only thing that ever changed it was `sc config` itself); in the suppressed state the card is **fully usable** - `nvlddmkm` `Problem 0`, CUDA/display/card-audio in daily use for days (including a monitor on the card as primary).

**Undock/replug is measured (the TB4 machine, suppressed)**: pulling and re-inserting the cable mid-session does not come back - the card sits at `Code 45` Unknown, `pnputil /scan-devices` is a no-op, a dock power cycle does not raise the tunnel either; the only recovery is a reboot through the stick chain, **and the unlock is still there after that reboot** (canary `intact`). "Plug in before boot, do not treat it as hot-plug" has moved from a reservation to a measured fact. Sleep/resume is still untested.

**A third host shape (maintainer's own round, 2026-11-06/07)**: Thunderbolt 4 host chip JHL8540 + a TB3 dock (JHL6540) + a GA102 3090 (crossflashed VBIOS; the host laptop's internal 4060 disabled to avoid the driver conflict, driver 610.74): **retained from the very first boot** - after the UEFI unlock Windows picks the card up directly, gate open, SS green; but **unplugging/replugging the TB3 cable (dock kept powered) → unlock lost, gate relocked, rates at factory, and the canary dead (`cleared`)**. Both open points are now closed: (1) stack identity - the JHL8540 is a Thunderbolt 4 chip, not USB4; the machine has the Intel Thunderbolt control panel and no USB4 section in settings (legacy TB stack, no MS USB4 CM service, no `USB4:` sampling needed); (2) the replug reset is a **domain reset** (canary `cleared`), not a narrow write. Unified reading: **the reset rides tunnel (re-)establishment** - a firmware-CM host inherits the POST-built topology at boot (no reset) and only resets when the stack re-establishes the tunnel on replug; a USB4-CM host tears down and rebuilds at bind time (reset). That is where the two stacks' "retained at boot or not" difference comes from.

From ssdiag 0.6.13 a `USB4:` block prints both service `Start` values and the USB4 nodes, so a report carries its own configuration.

**The timing route is excluded** (#54, 2026-10-30 round, measured by `GWASDW123`): moving both router services into early kernel boot — `Start=0` plus `Group=Boot Bus Extender` (`Usb4DeviceRouter` originally sits in the `Base` group, position 43 in `ServiceGroupOrder`; changing `Start` alone does not change the load order) — compressed the tunnel-establishment window from +8.06 s after boot to +4.31 s and moved the GPU bind 3.8 s earlier; both services ran, the eGPU enumerated normally, and **the unlock was still lost** (gate back at `0xffffff8f`, SS zeroed). So the loss does not depend on *how late* the takeover happens, only on *whether it happens*: compressing the unprotected window has no escape path, and the only out is stopping the device router from performing the takeover (the suppression above). Same root as "`--reapply` is structurally dead" — an early takeover and a late one are the same reset.

An [in-OS read-only check](#ssdiag) is still the first step, but the reading changes:

- Canary `intact`, gate still open: no takeover yet, or it was suppressed - the unlock is live.
- Canary `cleared`, gate locked: the takeover has reset; nothing recovers it within this boot - replay the USB, or apply the suppression first.
- SS values correct but throughput low: check power, clocks and the test method.

**Capture recipe (eGPU reporters can copy it verbatim)**: chain into a Linux live USB with `modprobe.blacklist=nouveau,thunderbolt` and read back driver-free with `tools/diag-unlock.py`; rEFInd instead of the ISO's own GRUB dodges the boot-media trap, and Secure Boot may stay on throughout. Full readings and the negative control: `docs/evidence/issue54-tb4-4090-linux-router-reset/`.

Turing hands off with the gate OPEN by default since 2026-11-08 (like Ampere/Ada; a TU106 bench proved it end-to-end: neither the card's own GOP rebuild nor the Windows driver init re-closes it, in-OS PLM stays `0xffffffff` with the canary intact, see `docs/evidence/tu106-1f07/keep-open-2026-11-08/`; builds up to v2.1.11 relocked by default and `\\tx-relock.txt` restores that; the old relock provokes an init-time PF FLR and wipes SS on some drivers, concluded by the [#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99) same-card A/B). The open gate also frees the in-OS write surface: the ssdiag Turing read-only restriction on `--reapply` can now be lifted (tool-side follow-up), and the SS0 4-bit rate-field encodings become explorable from within the OS. **The marker arm's relock value itself is not a failure**: with `\\tx-relock.txt` present the observed value is usually `0xffffffef` (bits 5:6 set) while the cold default is `0xffffff8f`, and it varies with board and driver version - the `1e04` 22G mod ([#74](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/74)) has been measured reading `fully closed 0xffffff8f` with the SS pair still holding the unlock values, so `8f` alone is not a failure verdict; read the `ss0/ss1` on the same line. `tu10x: plm=0xffffffff` (default) or `tu10x: plm=0xffffffef` (marker arm) in `HANDOFF-OUT`, either together with `tu10x_unlock=true` and SS0/SS1 still reading `0x88888888`/`0x8`, is the normal handoff shape ([cold values](cold-values.md), [#102](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/102), [#55](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/55); #54/#74/#102 are closed completed). Source: [Thunderbolt platform record](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54).

<a id="secureboot"></a>

## Does it work with Secure Boot on?

**Scope**: starting the USB tool without turning Secure Boot off. It does not cover anti-cheat clients that demand Secure Boot be *disabled* (for example the ACE-family clients that attest the boot chain); handle those per the vendor requirement, this section gives no procedure for them.

First, what this is not: it is not a way around Secure Boot. The boot chain is still verified end to end; the trust root grows from "a certificate Microsoft issued" to "that certificate plus yours". The only window this tool can work in - the firmware stage before any driver loads - is inside Secure Boot's jurisdiction, and this project holds no Microsoft or UEFI CA signing identity.

Three limits; without them the steps below set the wrong expectation:

- **Self-signing does not fix `ssdiag`.** `pmxdrv.sys` is an Intel/COMODO cross-signed driver from 2018 with no Microsoft root; `tools/ssdiag/README.md` says the load is **expected** to be refused, and its three carve-outs (OS upgraded from Windows <= 1607, Secure Boot off, signing certificate issued before 2015-07-29) cannot apply to a leaf certificate issued 2018-10-24. So with Secure Boot on, in-OS read-back comes from `HANDOFF-OUT` in `loader.log`. `tools/diag-unlock.py` on Linux is a zero-driver path, but **nobody has verified it with Secure Boot on**. `ssdiag --reapply` still needs Secure Boot off for one boot.

  **One counter-datapoint (2026-11-09, [bailu974's record](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot))**: on an Insyde laptop (COLORFIRE MEOW R15 24) with Secure Boot on, `ssdiag` printed this pre-written warning **but the driver actually loaded** - the tail shows `driver service removed` and the `0x823xxx` MMIO readings are real (without the driver those values cannot be read). The mechanism is unknown (that machine's DSE / vulnerable-driver-blocklist state was not captured); **the default expectation is still refusal** - but treat the warning as a hint, not a verdict: check for the trailing `driver service removed` and complete register readings before concluding anything, and when reporting, say whether your SB-on machine loaded or refused. Either answer moves this row.
- **Anti-cheat was not guaranteed; the first Vanguard measurement now passes.** Self-signing leaves the Secure Boot state bit on; the maintainer states that Vanguard (LoL) checks that bit. **First field measurement, 2026-11-14**: on the MOK+shim route the Riot Vanguard kernel driver loaded clean with zero VAN9001/VAN9003 ([D#243](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/243), N=1 machine, one game) — the ACE family is still unmeasured; do not extrapolate a single point into a guarantee. ACE-family clients validate the TPM/PCR[7] chain, and a self-signed chain measures differently from the stock boot (see [what each route costs in measurements](#secureboot-vs-ffs) below).
- **Re-sign every build**: a signature dies if one byte changes afterwards; the `*.bin` assets are not PE images and need no signing.

### Before you touch anything: BitLocker

Changing `db`/PK changes what PCR[7] measures, so a recovery prompt is possible; changing the boot path **may** do the same, depending on the BitLocker profile. Suspend protection once before either route, and have the recovery key in hand:

```powershell
Suspend-BitLocker -RebootCount 1
```

One report used this and saw no prompt; that is one machine, not a guarantee for other configurations.

### Choosing between the three routes

| | Route A: sign into `db` | Route B: MOK + shim | Route C: firmware per-file trust |
| --- | --- | --- | --- |
| What changes | one more certificate in the firmware `db` | firmware key database untouched; the key lives in the stick's MOK | one file entry in a firmware-private trust list (PK/KEK/db untouched; on that machine not even `db` is exposed) |
| Worst case | PK cleared and the vendor PK cannot be restored on this board -> Secure Boot cannot be turned on again | boot fails; delete the files from the stick and you are back to stock | boot fails; delete the files from the stick and you are back to stock; the trust entry remains but trusts only your file |
| After a firmware update / CMOS reset | `db` may be reset, re-append needed | unaffected (the MOK is on the stick) | untested (if it is hash-bound, a firmware update probably means re-adding) |
| Availability | the board's Key Management must be editable | a Microsoft-signed shim (`shim-signed` package of any distribution) | the firmware page has `Select a UEFI file as trusted for execution` (some Insyde machines; most do not) |
| Certificates / keys | required (step 0) | required (step 0) | **none at all** |

**A board whose Key Management is not editable: first look for route C's menu entry - it is the cheapest path when present; otherwise route B is the only option.**

### Step 0: keys (routes A/B; route C needs no keys at all)

Linux or WSL. Packages: Debian/Ubuntu `sbsigntool binutils mokutil`, Fedora `sbsigntools binutils mokutil`, Arch `sbsigntools binutils mokutil`; route A also needs `efitools` (or `sbctl`), route B also needs `shim-signed` / `shim-x64`. On Windows do this inside WSL; signing can also use `signtool` (route A step 1 only).

```bash
openssl req -new -x509 -newkey rsa:4096 -nodes -days 3650 \
  -subj "/CN=NVUEFI Self-Signed/" \
  -addext "keyUsage=digitalSignature" \
  -addext "extendedKeyUsage=codeSigning" \
  -keyout NVUEFI.key -out NVUEFI.crt
openssl x509 -in NVUEFI.crt -outform DER -out NVUEFI.cer
```

Do not drop `extendedKeyUsage=codeSigning`: Windows `signtool` checks it (`sbsign` does not). Keep the private key on this machine. For route B use `MOK.key` / `MOK.crt` / `MOK.der` as the file names; the commands are identical.

**This block is the only command set in this section that this project has executed** (OpenSSL 4.0.3; output carried `Key Usage: Digital Signature`, `Extended Key Usage: Code Signing`, and the DER export). Every step below that uses `sbsign`, `objcopy`, `sbverify`, `efi-updatevar` or `sbctl`, and every firmware step, **has not been executed** and is written from the tools' documented behaviour.

### Route A: sign the certificate into the firmware `db` (second-hand, not reproduced here)

The only source is a second-hand user statement quoted in [#145](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/145), with no logs and no board model, and none of the commands below has been run by this project. Decide with the "worst case" row above in view.

1. **Sign** (run `python3 check_efi.py BOOTX64.EFI` first and confirm the banner is the build you meant):

   ```bash
   sbsign --key NVUEFI.key --cert NVUEFI.crt --output BOOTX64.signed.efi BOOTX64.EFI
   sbverify --cert NVUEFI.crt BOOTX64.signed.efi
   ```

   Windows equivalents: `New-SelfSignedCertificate -Type CodeSigningCert`, then `signtool sign /fd SHA256 /a /f NVUEFI.pfx /p <password> BOOTX64.EFI`, then `signtool verify /pa BOOTX64.EFI`.

2. **Hand it to the maker** (current `make-usbroot.ps1` has `-BootEfi`, which writes the signed copy to the root and to `EFI/BOOT/`):

   ```powershell
   .\make-usbroot.ps1 -Family ad102 -Out G:\ -BootEfi .\BOOTX64.signed.efi
   ```

3. **Read these three before you touch the firmware** (step 4 is the destructive one, so the warning has to come first):

   - **Append, never replace**, the Microsoft certificates in `db`: `bootmgfw.efi` is signed by Microsoft Windows Production PCA 2011, and removing it stops Windows from booting.
   - **Clearing PK is the last resort, not the first step.** Rule out the non-destructive causes first: an unset BIOS/supervisor password leaves Key Management greyed out; Secure Boot in Standard mode (not switched to Custom) refuses `db` writes; some OEM boards expose no Key Management at all. Only when none of those apply **and this board can re-enrol its vendor PK** is clearing PK on the table - a board that cannot restore the vendor PK cannot turn Secure Boot on again.
   - **Export PK/KEK/db** to a stick first.

4. **Append the certificate** (`NVUEFI.cer`, DER) - the two append-only ways:

   - Firmware UI: Security / Secure Boot -> Key Management -> `db` -> **Append / Append from file**.
   - Linux with the KEK private key (`efitools`): `efi-updatevar -a -c NVUEFI.crt -k KEK.key db`

   **This is not an append**: `sbctl enroll-keys --microsoft` installs *your* keys as PK/KEK/db (with the Microsoft certificates alongside), requires Setup Mode, and **replaces** the vendor trust root; `sbctl sign` then signs with the keys `sbctl create-keys` generated, not the `NVUEFI.key` from step 0. To stay on that path, import the step-0 keys into sbctl first (see its documentation), otherwise the signature the firmware checks uses a different key from the one you enrolled.

5. **Verify**: `mokutil --sb-state` should report `SecureBoot enabled`; the firmware-level proof is a fresh `loader.log` with the TX banner on the stick.

### Route B: MOK + shim (three machines reported, two full desktop runs)

The firmware verifies only the Microsoft-signed shim; your key takes effect past it through MOK, and the firmware key database is not modified. Basis: three field reports — [#162](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/162) (Secure Boot on, v2.1.10, RTX 30 laptop, `ga102`, PK/KEK/db untouched), [#210](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/210) (2026-11-08, RTX 3080 20GB on an ASUS ROG Z790, official v2.1.10 package plus an added `.sbat` and a self-signed MOK, full chain green through handoff, in-OS `Confirm-SecureBootUEFI=True`, certificate present in `MokListRT`, PK/KEK/db unchanged), and the [#241 follow-up](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/241#issuecomment-6894516749) (2026-11-15, RTX 3090 desktop, using the **hash-enrollment variant** below with three protected boots under active BitLocker). The scripts from #162 live at [J0s3f/nv-unlock-secureboot-esp](https://github.com/J0s3f/nv-unlock-secureboot-esp) as a reference implementation and are not vendored here. What #210 adds over #162 is the reproducible section parameters (step 3) and the countdown-keypress step (step 7), both from #210 only; what the #241 follow-up adds is the **hash-enrollment variant plus the active-BitLocker / TPM-measurement experience** (right after step 7).

1. Generate `MOK.key` / `MOK.crt` / `MOK.der` with step 0.

2. **Check whether the release already has a `.sbat`**:

   ```bash
   objdump -h BOOTX64.EFI | grep -i sbat
   ```

   Output means the section is there - skip to step 4. The v2.1.10 artifact has none (checked here: `.text` / `.rdata` / `.data` / `.eh_fram` / `.reloc`, five sections, no `.sbat`). #210 confirmed the consequence on real hardware: with the section missing, shim answers only `Verification failed: (0x1A) Security Violation`, never reaches MokManager, and explains nothing.

3. **If it is missing, add it.** shim >= 15.3 refuses a second stage without a valid `.sbat` ([Arch Wiki, Secure Boot](https://wiki.archlinux.org/title/Unified_Extensible_Firmware_Interface/Secure_Boot), [Gentoo bug 925902](https://bugs.gentoo.org/show_bug.cgi?id=925902)); expect that refusal to be printed by shim rather than by us, and not to name the cause.

   Write `sbat.csv` first: the first line is the SBAT metadata line and must be present; the second line describes this component (pick a name, keep the generation at 1):

   ```text
   sbat,1,SBAT Version,sbat,1,https://github.com/rhboot/shim/blob/main/SBAT.md
   nvuefi,1,nv-uefi-unlocker,nv-uefi-unlocker,<build version>,https://github.com/physicsdolphin/nv-uefi-unlocker
   ```

   Then derive the new section's VMA: `objdump -h BOOTX64.EFI` shows each section's `VMA` and `Size`; take the **last** section's `VMA + Size` and round up to the `SectionAlignment` in the PE header (commonly `0x1000`). For v2.1.10 that is `.reloc` `VMA 0x67000` + `Size 0x4fc`, giving **`0x68000`** (equal to the original `SizeOfImage`).

   ```bash
   objcopy --add-section .sbat=sbat.csv --change-section-vma .sbat=0x68000 BOOTX64.EFI BOOTX64.grub.efi
   objdump -h BOOTX64.grub.efi | grep -i sbat       # .sbat must be listed; SizeOfImage must grow
   ```

   Order matters: add the section to the **unsigned** `BOOTX64.EFI`. If the file you have is already signed, you must re-sign after adding the section. What #210 actually produced on v2.1.10 matches this algorithm (`.sbat` 177 bytes, sections 5 -> 6, `SizeOfImage` `0x68000` -> `0x69000`); any of the three differing means the file you are working on is not that release artifact.

4. **Sign and verify** (name the result `grubx64.efi`; shim looks for that name next to itself):

   ```bash
   sbsign --key MOK.key --cert MOK.crt --output grubx64.efi BOOTX64.grub.efi
   sbverify --cert MOK.crt grubx64.efi
   ```

   Windows equivalent (what #210 did): `signtool sign -fd SHA256 -f MOK.pfx grubx64.efi`. Afterwards **recompute the Authenticode digest independently** and compare it against the embedded `SpcIndirectDataContent` digest; `signtool verify` alone does not prove the right file went in.

5. **Get shim and MokManager**: `shimx64.efi.signed` and `mmx64.efi` from a distribution's `shim-signed` package. The reporter used shim-signed 15.8; **shim < 15.3 does not do the SBAT check**, and its behaviour is not verified in this repository.

6. **Build the stick, then replace**: run the maker normally first (it produces the seven `ad102` assets and `EFI/BOOT/BOOTX64.EFI`), then replace four files in the stick's `\EFI\BOOT\`: `BOOTX64.EFI` <- shim, `grubx64.efi` <- step 4 output, `mmx64.efi` <- MokManager, `MOK.der` <- your certificate. **Leave the assets at the volume root** (about 85 MB for `ga102`).

7. **First boot**: shim reports `Verification failed: (0x1A)` (expected, the MOK is not enrolled yet) and then shows `Press any key to perform MOK management` with a **10-second countdown — you must press a key inside it**. If you do not, shim silently continues and the loader never runs, which looks like nothing happened (#210 measured this). In MokManager: Enroll key from disk -> `MOK.der` -> one-time password -> confirm Enroll after the reboot. Normal boots then go: shim (Microsoft-signed) -> your signature verified against MOK -> loader -> unlock -> chain back to `bootmgfw`, with Secure Boot on throughout.

**Hash-enrollment variant (no private key at all; measured in the [#241 follow-up](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/241#issuecomment-6894516749), RTX 3090 desktop)**: skip the key pair entirely and have MokManager enroll the loader image's own hash — the menu path is **`Enroll hash from disk`** (not `Enroll key from disk`, which expects a DER certificate; mixing them reports `Unsupported Format`). The TX image in that record took the equivalent other path: a Windows-side helper staged a `MokNew`/`MokAuth` request, and the pending hash appeared under `Enroll MOK` in MokManager on the next shim boot. Four measured lessons (all from that record): ① MOK enrolls the **PE/Authenticode image hash**, whereas `Get-FileHash` computes the whole-file hash — they differ (fields like the PE checksum do not enter the image hash), so **never enter a file hash as if it were the MOK hash**; and since the EFI embeds the build time, even the same source rebuilds to new hashes — **an old enrollment never authorizes new bytes**, every rebuild needs a fresh enrollment. ② An unenrolled image produces the same `0x1A` on first boot, with the same countdown keypress requirement. ③ The same machine also ran **active BitLocker protection throughout** (XTS-AES-128, PCR 0/2/4/11 profile): three protected boots, two recovery-free, elevated checks confirming SB/protection/PK-KEK-db-dbx untouched; the first setup did include one explicitly authorized suspension (RebootCount=3, resumed afterwards with a separate protected confirmation boot) — a suspension is not part of ordinary operation and must not be extended automatically. ④ **TPM measurement side**: that machine's TCG2 logs show Chain-only vs TX-skip differing in PCR 4, and actual TX vs skip (same file) differing in PCR 2/7 — **different modes are different measured boots**; skip can also trigger a BitLocker recovery prompt (**recovery ≠ TX failure**), so stick to one mode; changing shim/MokManager versions also changes the measurements and both boot paths need re-acceptance. That record ships with a machine-bound hardened build (GPU/ROM/assets/ESP all pinned, markers synthesized from the signed image and never read from the stick); see the #241 row in [community-reports](community-reports.md).

Internal-ESP variant (**not recommended by this project; two field records exist, both self-declared risk-taking deviations**): put the four files on the internal ESP and skip the stick on every boot. Shape 1 (#162's reporter): files in `\EFI\BOOT\`, first in BootOrder - CHAIN-BOOT skips its own `Boot####` and the Windows entry on the same ESP, so it does not loop. Shape 2 ([bailu974's record](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot), 2026-11-09): that Insyde firmware **refuses to create new `BootXXXX` variables** (`SetFirmwareEnvironmentVariableEx` returns `err=87`, while rewriting existing ones works), so the reporter rewrote `Boot0000` (the Windows Boot Manager entry) to point at `\EFI\nvunlock\BOOTX64.EFI`, keeping a copy of the original entry in `Boot2001` as the fallback; the assets sit at the ESP root (the loader reads assets only from the root of its own volume, never subdirectories; the 84 MB needs ESP headroom). Shared costs: the README advice is explicit - "[do not copy it to the system ESP](../README.md), and the permanent boot order does not need changing" - and rewriting `Boot0000` or the ESP can leave the machine unbootable; back up first and confirm you can reach BIOS recovery. Both reporters shipped backup/rollback scripts with their records.

### Route C: the firmware's per-file trust (one machine reported, not reproduced here)

Some Insyde firmwares offer **`Select a UEFI file as trusted for execution`** on the `Administrator Secure Boot` page: the firmware adds the named `.efi` file to its own trust list, **touching no PK/KEK/db and needing no certificate at all** (all of step 0 drops out). Basis: [bailu974's record](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot) (COLORFIRE MEOW R15 24, Insyde `1.07.11COLO1`) - add the trust entry, set `Enforce Secure Boot` back to Enabled, the stick's loader is admitted, the whole chain runs, and `Confirm-SecureBootUEFI` reports on.

Practical points and pitfalls (all from that record):

1. **First check your firmware even has the entry** - without it this route does not exist (do not clear PK to make it exist; that is route A's risk territory). Note that `Erase all Secure Boot Settings` / `Restore Secure Boot to Factory Settings` on those pages are **immediately executed actions, not toggles**; do not select them.
2. The trust is **very likely bound per file hash**: on that machine the firmware does not expose `db`/`dbx` through the standard variables at all (reads return `ERROR_ENVVAR_NOT_FOUND`), so the list lives in private NVRAM. **After replacing `BOOTX64.EFI` you must re-add the trust entry.**
3. BitLocker precondition: that machine's C: is unencrypted, so no recovery prompt appeared; if your OS volume is protected, suspend BitLocker before changing Secure Boot state (see [before you start](#before-you-start-bitlocker)).
4. `bootmgfw.efi` is Microsoft-signed, so Windows still boots with enforcement on; the worst case is "our loader is refused", not "the system cannot boot".
5. Evidence tier: one machine, self-reported, not reproduced by this project; and the binary that record trusted was a **locally patched build** (see [ghost keys](#ghost-key)), not a release artifact. The trust UI should admit an unmodified release binary the same way, but nobody has tested that.

<a id="secureboot-vs-ffs"></a>

### How is that different from flashing the loader into the firmware?

Start with the half that is easier to misread: **why self-signing still does not guarantee passing ACE**.

ACE-family clients (Tencent) run a kernel driver plus server-side policy, and they can look at several inputs at once; there is no public, complete list of what they check:

1. **The Secure Boot state bit** - reportedly the only thing Vanguard-class clients read; self-signing does not change it, which is why this route can work at all.
2. **TPM measurements and the event log of the boot chain**: the loaded boot application lands in PCR[4], and in profiles that also measure the Secure Boot policy, PK/KEK/db/dbx land in PCR[7]. A self-signed chain changes both - **even route A's "just append one certificate to `db`" is not a no-op from the attestation side**.
3. **Kernel driver policy**: who signed the driver, the vulnerable-driver blocklist, VBS/HVCI state.

So "Secure Boot is on" is neither sufficient nor the only input, and **nobody has measured ACE on either route**. The table below shows that the two routes differ in *which* measurement they disturb, not that one of them passes reliably.

| | Self-signed (route A / B) | Firmware-resident FFS |
| --- | --- | --- |
| What changes | the OS boot chain (the PE under `\EFI\BOOT\`) | a DXE driver inside a firmware volume |
| Secure Boot | the certificate must be added to the trust root (A: `db`; B: MOK) | not part of OS boot-chain verification (an FV is part of the trusted image); Secure Boot can stay on [design inference, not verified on hardware] |
| Flashing | not needed | **required**: insert with UEFITool/FPT, back up the full image first, keep an external programmer (CH341A) at hand; power loss or a bad image bricks the board; Boot Guard platforms may refuse to boot an IBB modification (board-dependent) |
| Who should do it | anyone who can enter firmware setup | advanced users only, at their own risk; the project ships no installer and does not flash for you (the [#73](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/73) precedent) |
| When it runs | the user boots the stick and runs the transaction | a `ReadyToBoot` callback, same window (before `bootmgfw`), replayed on every boot |
| TPM / BitLocker | a `db` change moves PCR[7] (if the profile measures it); a self-signed chain changes the measurement of the loaded image | changing the firmware moves **PCR[1]**; with a BitLocker profile that includes [1] the volume cannot unseal and the recovery key is demanded; suspend before flashing |
| Verification without flashing | - | `load fs0:\NvUnlockDxe.efi` from a UEFI Shell proves the form before you flash anything |
| Evidence | route B: three machines, two full desktop runs with the `.sbat` parameters and the countdown keypress actually executed (#241's follow-up being the hash-enrollment variant + active BitLocker); route A: second-hand | [feasibility document](dxe-driver-ffs-feasibility.md) only, **not verified on hardware** |

In one line: **both routes disturb a measurement** - self-signing moves the boot chain (PCR[4]/[7] side), FFS moves the firmware (PCR[1] side). Against a client that only reads the Secure Boot state bit, either can work; against a client that requires the stock boot chain or the stock firmware measurements, neither is guaranteed. Anti-cheat rules are vendor policy and change without notice; this document promises nothing either way.

### Evidence status

| Item | Evidence | Status |
| --- | --- | --- |
| Route B (MOK + shim) | #162 laptop report (unlock survives into Windows); **#210 full desktop run**: Secure Boot on, v2.1.10 + `.sbat` + self-signed MOK, whole chain green + `Confirm-SecureBootUEFI=True` + certificate in `MokListRT` + PK/KEK/db untouched; **#241 follow-up full desktop run (hash-enrollment variant, no private key)**: Secure Boot on + active BitLocker (PCR 0/2/4/11), shim 15.8 pinned SHA, three protected boots (two recovery-free), elevated checks all green, TCG2 measurement analysis (TX vs skip differ in PCR 2/7, Chain-only vs skip in PCR 4) | three machines (two full desktop runs); steps 3/4/7 hard-code #210's measured parameters, not re-run on this project's own hardware |
| Release lacks `.sbat` | this project checked the v2.1.10 artifact's section table; #210 confirmed on hardware that a missing section yields only `0x1A` from shim | verified (artifact level + one machine) |
| The `openssl` block in step 0 | executed here (OpenSSL 4.0.3) | verified |
| Route A (db self-sign) | second-hand statement, no logs, no board model | not reproduced, not executed |
| Secure Boot on + `ssdiag` | `tools/ssdiag/README.md`: expected to be refused; #210 declined to load it on that basis | not measured (#210 marks it not verified) |
| Secure Boot on + `diag-unlock.py` | none | not verified |
| Anti-cheat (Vanguard / ACE) | one Vanguard-side field measurement exists: on the MOK+shim route `vgk.sys`/`vgc` loaded clean with zero VAN errors ([D#243](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/243), 2026-11-14); the ACE family is still unmeasured | single datapoint, N=1 |
| Firmware-resident FFS route | `docs/dxe-driver-ffs-feasibility.md` (design, risk, PCR[1]/BitLocker effects) | not verified on hardware |
| Route C (per-file trust) | [bailu974's record](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot): Insyde file trust + Enforce, whole chain ran | one machine; not reproduced here |
| `ssdiag` loading successfully with Secure Boot on | same record: the pre-set warning prints but the driver loads, MMIO readings complete | one-machine counterexample; mechanism unknown, refusal remains the default expectation |
| Internal ESP + rewriting `Boot0000` (firmware refuses new `BootXXXX`, `err=87`) | same record: `Boot2001` fallback + backup scripts | one machine; deviates from the README discipline, self-declared risk |

If you run either route, reporting the **board model**, whether `ssdiag` was refused, whether BitLocker asked for the recovery key, and whether `diag-unlock.py` can read the block with Secure Boot on moves one row forward each.

<a id="ssdiag"></a>

<a id="ghost-key"></a>

### A ghost key aborts the confirm countdown (stuck keys)

Symptom: nobody touches the keyboard, but the loader prints `CONFIRM: key Printable('\u{3}') ... ABORT` (or any other key value) and aborts; the `\i_know_what_i_am_doing.txt` armed countdown gets interrupted the same way. Something in the firmware stage is **repeatedly reporting a phantom key**. Three confirmed instances:

| Key | Source device | Machine |
| --- | --- | --- |
| `0x1A` | a physically stuck Z key (EC) | Alienware m18 R1 ([#110](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/110)) |
| `0x03` (Ctrl+C) | the EC synthesising the key during firmware boot | COLORFIRE MEOW R15 24 ([bailu974's record](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot), with a v2.1.10 patch diff) |
| `Special(LEFT)` | a **USB mouse**'s boot-keyboard interface reporting continuously ([PR #220](https://github.com/physicsdolphin/nv-uefi-unlocker/pull/220), Legion 5 Pro 16ACH6H) |

Two self-help routes, both bound by one discipline - **ignore only the exact key value you observed; do not touch Space/Enter/timeout semantics** (these windows are the gatekeepers of the zero-write guarantee):

1. **A local patch ignoring that key value**: `patches/nv-uefi-unlocker-v2.1.10-stuck-key.patch` in bailu974's repo is the template (the abort windows in `wait_confirm` / `wait_confirm_armed` / tx_prep become immune to the observed byte, everything else unchanged). #110 took the same approach.
2. **The "settle + ESC-only" route from [PR #220](https://github.com/physicsdolphin/nv-uefi-unlocker/pull/220)** (under review, not merged): drain the input and wait for a quiet window before CONFIRM opens; if the input never goes quiet, the countdown degrades to ESC-only-abort - a safe direction, because ESC can never proceed, so a phantom key can at most be ignored and can never "press once to unlock".

How to identify it: unplug the other USB devices one at a time and bisect (#220's phantom came from the mouse; unplugging silenced it); if the key value in the ABORT line matches a physical key, replace that keyboard first.

**A BitLocker finding that rides along (#220, important)**: on a machine with BitLocker TPM-bound to PCR 0,2,4,11, **an aborted boot and an unlocked boot differ in boot-chain behaviour** (the aborted run skips the second native-GOP driver load) -> PCR[2] measures differently -> Windows asks for the recovery key (event 24707, `MismatchedPcr=2`); warm boots are unaffected. This is the first field record of a **behavioural** boot-chain difference (not a Secure Boot state change) tripping a PCR mismatch: on machines whose TPM profile includes PCR[2], either keep every boot on the same path (all unlocked or all aborted) or suspend protection first. This FAQ previously covered only PCR[7] (Secure Boot state) and PCR[1] (FFS flashing).

## Can a 50-series (RTX 50 / Blackwell) card be unlocked?

**Not today.** The cap and the gate are the same shape as on 40-series, and harder: GB206 fuses read `FMLA32/IMLA1/2/4 = 3` and `FEAT_OVR_PLM = 0xfffffb8f` admits only PL3, the level inside signed boot firmware. All three host writes are dropped silently. An RTX PRO 6000 on the same die runs full rate, so it is product segmentation, not silicon.

The real break is the boot chain: Blackwell **has no SEC2 stage** (FSP → GSP BootROM → encrypted FMC → GSP-RM), so there is no signed-booter stack overflow to turn into a ROP write of the PLM. No GB20x booter is public; the debug FMC is byte-identical ciphertext to production with only a different signature and no slot table. Running the USB stick therefore cannot open the gate either, and `ssdiag --reapply` correctly refuses.

What still works: ssdiag / diag-unlock / the USB SAMPLE record the FEAT block and fuses (read-only). Read them carefully — `READOUT_1` is a fuse mirror with non-contiguous fields, Blackwell `3` is ~half rate (not the Ada 1/8), and a nonzero SS0/SS1 is only shadow state: every nibble's bit 3 being clear is what says the fused selectors are in force. Mechanism, exploration status and the surfaces still open: [blackwell-gb20x-status](blackwell-gb20x-status.md) ([#168](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/168)).

## How do I check or repair the unlock in the OS?

On Windows, use the `ssdiag-x86_64-windows.exe` published as a separate release asset (the zip does not carry it - see [antivirus false positives](#av-fp)), abbreviated below as `ssdiag`:

```text
ssdiag --inventory
ssdiag
ssdiag --select 2504
ssdiag --reapply
```

`--inventory` (since 0.7.0, widened in 0.7.1) prints the **no-admin, read-only** hardware snapshot: OS/CPU (with the CPUID topology and F/M/S decode - an ES/QS junk brand string still yields family/model/cores/threads)/board/BIOS/chassis form/RAM slot layout (capacity + speed, SMBIOS Type 17, no serials)/disk models and sizes/partition layout (offset, filesystem, letters, free space)/the other PCI devices on the bus, plus per-card device ID / subsystem ID / VBIOS / driver version / GSP mode (resolved through the driver's bound-instance list, ghost-free), the family admission verdict (IN-SCOPE/OUT-OF-SCOPE) and pre-flight advisories (Secure Boot on, Turing without GSP firmware mode, laptop relock exposure). This is the block the issue templates ask you to paste - review it and redact anything you would rather not post; it reads no hostname, username or serials. The full run (register read-backs) still needs an administrator; `--select` filters by DeviceID. `--reapply` writes and verifies SS0/SS1 only behind an open gate; it does not write PLM. Turing remains read-only. On Linux, run `sudo python3 tools/diag-unlock.py` from the checkout root; `--bdf` selects an actual target address, and `--reapply` is also available; the no-root snapshot twin is `python3 tools/diag-unlock.py --inventory` (sysfs/DMI/nvidia-smi reads only, same output format as ssdiag).

If you hit `MAP_PHYS win32 error 5` (mapping denied): 0.6.3+ prints a one-shot precondition breakdown (HVCI runtime / kernel-mode AV filter driver / driver refused load). **Pausing protection or exiting user-mode AV processes does NOT unload the kernel hooks** - two field closures (Kaspersky, cured after uninstall); see [platform gates and denied-map triage](ssdiag-platform-gates.md).

**Running ssdiag (or any tool that reads PCI config directly) instantly bluescreens the whole machine**: the bug check is `0x1EA SECURE_PCI_CONFIG_SPACE_ACCESS_VIOLATION` ([#244](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/244) is the first in-the-wild sample). Mechanism ([Microsoft's page](https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/bug-check-0x1ea--secure-pci-config-space-access-violation) is explicit): when VBS "secure PCI" is on, VTL0 is barred from directly mapping the MCFG/ECAM range to read config space, and the enforcement sits in the hypervisor/VTL1 EPT - **the map itself succeeds and the first read bluescreens the machine** (not a recoverable `MAP_PHYS error 5`); the four WinDbg arguments are the faulting instruction's opcode (`0x8b` = MOV r32,r/m32 in #244) / device RID / config offset / faulting RIP.

Two tiers - **do not blame memory integrity (HVCI)**: HVCI/VBS running on its own does not trigger 0x1EA - #188 (VBS+HVCI on, SB off) and #204 (HVCI ON, SB off) ran the **full probe** (pmxdrv load + ECAM reads) fine; #218's SB+HVCI+blocklist-all-on datapoint is `--inventory` only (the driver-free tier) and is not full-probe evidence. The documented trigger pair is an **SDEV (Secure Devices) ACPI table present + VBS running**: since Windows 10 2004 (19041.423), the [MS troubleshoot page](https://learn.microsoft.com/en-us/troubleshoot/windows-client/performance/stop-error-lenovo-thinkpad-kb4568831-uefi) states unsupported config-space access is blocked, with **"parsing the MCFG table and mapping configuration space to virtual memory" named explicitly**; the [ESS doc](https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/windows-hello-enhanced-sign-in-security) names the enforcer - the **SDEV table is parsed by the Secure Kernel**, and unsupported reads/writes "result in bug checks"; the [CHIPSEC doc](https://chipsec.github.io/installation/InstallWindows.html) carries the same condition formula. Public cases are ordinary machines, not OEM-image-exclusive: Framework+Speccy, LG Gram+Speccy (cpuz149_x64.sys), AMD Ryzen Master, Vanguard vgk; our own observation: on some 2024+ ASUS OEM images (TUF Gaming 天选6, field-tested) Intel's FPTW64 bluescreens the same way just reading SPI-flash MMIO (whether the SDEV restriction covers MMIO beyond MCFG is not publicly documented).

**Why it disappears after a reinstall (corrected: the SDEV table lives in firmware, and a Windows reinstall does not touch it)** - the loss must come from the VBS side or the firmware option side, ordered by how cheap they are to check: (1) VBS is actually not running after the reinstall (go by **WMI `VirtualizationBasedSecurityStatus == 2` or msinfo32** - the registry value is absent on some VBS-running machines and must not be read as "off", #244 field measurement); (2) the BIOS ESS-class option got reset or changed by a BIOS update - **"Enhanced Windows Biometric Security" (Lenovo, Security > Virtualization) / "Enhanced Sign-in Security" / "Enhanced Hello"-class options**; disabling it removes the SDEV restriction (the troubleshoot page's documented workaround); (3) an undisclosed OS-side precondition (ESS provisioning, biometric trustlets - pure hypothesis); (4) the OEM image and the reinstall differ in build. The earlier "OEM images provision the SystemGuard registry key" speculation is **retracted** (no public document ties that key to secure PCI; #244's `SecurityServicesRunning=[2,3,4,5]` is treated as the usual four-piece co-occurrence on secured-stack machines).

Since ssdiag 0.8.2: when the `GetSystemFirmwareTable('ACPI','SDEV')` probe is present, the VBS read decides the branch - `== 2` (or the SystemGuard key provisioned) -> the probe refuses before deploying anything; **an absent registry VBS value (unverified) also refuses, fail-closed** - #244's T14s measured WMI reporting 2 while that registry value does not exist, so the 0.8.0/0.8.1 `== 2` positive predicate silently disarmed there (caught by the reporter's ASSESSMENT and fixed); only a definitive "VBS off / not running" re-enables the probe. No override flag (the cost is a machine-wide bugcheck, not a denied read); the SystemGuard key stays a conservative secondary signal; `--inventory` is unaffected. Collection recipe (one 天选6 sample in both states pins it):

```powershell
Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard |
  Select-Object VirtualizationBasedSecurityStatus, SecurityServicesRunning
# SDEV table: the "SDEV table (ACPI)" line in the ssdiag platform gates (read-only
# firmware-table query, cannot trigger the bugcheck itself);
# in BIOS, compare the state of Enhanced Windows Biometric Security / Enhanced
# Sign-in Security / Enhanced Hello-class options
# crash sample: the minidump's Arg1 (opcode) / Arg2 (RID) / Arg3 (offset)
```

For SS/PLM read-back use the EFI-side read-only sampler (#244's reporter arranged exactly that: PnP/CUDA on the Windows side, read-back in UEFI). See [platform gates and denied-map triage](ssdiag-platform-gates.md) §2a.

| Read-back | Verdict / action |
| --- | --- |
| SS0=`0x88888888`, SS1 low nibble=`0x8` | Overrides effective; verify performance with a matched workload |
| Ampere/Ada gate open, SS values wrong | Selector repair is possible; read back after repair |
| Turing gate open (`0xffffffff`), SS values correct | Default handoff shape (since 2026-11-08, same as Ampere/Ada); the in-OS write surface comes with it |
| Turing gate relocked (`0xffffffef`/`0xffffff8f`), SS values correct | Normal shape with `\\tx-relock.txt` present (value varies by board, bits 5:6 set, not the `0xffffff8f` cold default), or a platform/driver relock event - tell them apart with the canary, see [cold values](cold-values.md) |
| Turing gate relocked at handoff, SS wiped in-OS | The old carrier's (<= v2.1.11) default relock provoked an init-time PF FLR ([#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99)); replay on v2.1.12-rc3+ and stop retesting on the old build |
| Gate closed and SS invalid, or GPU powered off | Repair cannot reopen the gate; follow recovery and replay |

Compare the OS result with `HANDOFF-OUT` in `loader.log`. For driver initialization failures, also inspect Device Manager, nvlddmkm/Xid and power state. EFI success does not replace OS verification.

For no gain after handoff, use these read-back patterns to guide diagnosis. Confirm the cause with power and driver logs:

| In-OS observation | Investigate |
| --- | --- |
| Ampere/Ada gate and selectors return to locked defaults | Power loss or full reset |
| Gate remains open, selectors return to stock | devinit rewriting overrides (see [relock mechanisms](#relock)) |
| Rewritten selectors with Code 43 or driver resets | Failed driver initialization and recovery |
| Effective selectors but low performance | Clocks, power wall and workload |
| Card shows Code 12 (`CM_PROB_NORMAL_CONFLICT`) in Device Manager while loader/`HANDOFF-OUT` are all green | A boot-time Windows resource-arbitration artifact ([#165](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/165), closed after twelve rounds: the condition is multiple cards present + the first-position card being the unlocked one; A4G / devnode residue / card identity / footprint change all excluded). **Self-remedy: disable -> re-enable the conflicted card in Device Manager** — the arbitration re-trigger fixes it, and the PnP reload takes no FLR so the unlock survives intact ([#196](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/196), the same operation's second use); or preventively unlock the second-position card in multi-card setups |

ssdiag discovers BAR0 from the OS and accepts no manually supplied physical base; it wakes parked GPUs as needed. Its embedded, verified `pmxdrv.sys` uses a temporary `SSDIAG_PMXDRV` service with cleanup on exit, or borrows an existing device such as nvoc's. Driver deployment normally needs administrator privileges. **If a direct run reports `OpenSCManager win32 5`** (the machine's security policy denying the service manager; [D#248](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/248) sample): confirm elevation first; if it still fails, install the bundled `pmxdrv.sys` as a service and start it manually, then run ssdiag through the borrow path (the read-back works normally and is unaffected by the policy). Its security model and trade-off have their own section: [The pmxdrv.sys security model and trade-off](#pmxdrv). See the [ssdiag reference](../tools/ssdiag/README.md) for the register surface and driver details.

<a id="vgpu-vf-guest"></a>

## Can I trust ssdiag readings inside a vGPU / SR-IOV VF guest (e.g. geforce-sriov)?

**No — they are not evidence of the host card's unlock state.** vGPU comes in two forms: in SR-IOV VF passthrough ([geforce-sriov](https://github.com/noon-at-cgn/geforce-sriov)-class stacks) the guest gets a **virtualized register window** — the MMIO is real hardware access, but routed through per-VF filtering; in mdev / GPU-PV forms (vgpu_unlock lineage, Hyper-V GPU-PV) the guest has no real MMIO at all — BAR accesses are trapped by the hypervisor and forwarded to the host mediator. The first field sample (host RTX 3090 Linux + upstream gpu-unlock + geforce-sriov; the guest sees `10de:2230` "RTXA6000-8Q") established the VF window shape ([evidence and decode](evidence/sriov-vf-guest-2230/findings.md)):

- **Extended config space is all-FF** (from 0x0100): the ReBAR/link probes skip automatically (builds <0.7.2 crashed silently here; fixed in 0.7.2);
- **Every register except `PMC_BOOT_0` reads hard-wired zeros**: not badf — the badf encoder lives on the PF's PRI and the VF window has none. FECS/PLM are PF-side raw PRI state with no readable projection in the guest; `closed (non-standard)` labels are fill artifacts, not gate state. 0.7.3 prints a `NOTE: ... virtualized / SR-IOV VF window` line for this shape;
- **BOOT0 and the device ID are firmware-authored profile identity**: `0x1fa52a02` (die segment 0x1fa, no known die segment); the guest's "RTX A6000" enumeration is not the host 3090 silicon — the disguise is two-layer: the host identity hook offers the A6000 profile, then the PF firmware projects that identity into the guest;
- **The only valid VM datapoint class is full-device VFIO/DDA passthrough** ([#135](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/135), where the guest gets real PF MMIO).

To verify the host card's unlock state, probe on the **host**: the upstream gpu-unlock verification path or `tools/diag-unlock.py` on a Linux host; ssdiag on bare-metal Windows. Nothing read inside the guest constitutes host evidence.

<a id="pmxdrv"></a>

## The pmxdrv.sys security model and trade-off (required disclosure)

`ssdiag` needs to read PCI BAR0 registers, and **Windows offers no sanctioned user-mode path** to do that — reading MMIO registers requires a ring-0 physical-memory transport. For contrast: on Linux, root is everything — one `mmap` of the device's sysfs resource puts the registers in front of you (`tools/diag-unlock.py` works exactly that way, with zero third-party kernel modules); Windows has no equivalent.

**Why pmxdrv.sys**: the classic hobbyist transport, WinRing0, is on Microsoft's vulnerable-driver blocklist — HVCI seats refuse it before it loads. Among the transports that still load on the vast majority of real-world installs under current security models, the Intel-signed `pmxdrv.sys` (PCILeech lineage) is the one we could narrow: SHA-256 pinned inside the binary, extracted on demand to `%TEMP%`, run under our own `SSDIAG_PMXDRV` service name, stopped and deleted on exit, with a preflight notice (blocklist/HVCI/EDR state) since 0.4.9. Read-only by default; `--reapply` is the single explicit write path.

**This choice is industry routine, not an outlier — the author's experience**: in-OS UEFI BIOS flashing is a mainstream, vendor-supported maintenance activity, and its entire toolchain depends on exactly this class of mechanism — AMI's AFUWIN, Insyde's InsydeFlash and Intel's CSME flash tool all rely on kernel drivers that open the PCI MMIO mappings of the UEFI firmware for read/write. The author has used all three for years for necessary BIOS maintenance; the mechanism is old habit. Within that class: AFUWIN's driver is already on Microsoft's blocklist, while Intel's own CSME-signed transport is the one with the most ordinary security model of the three — vendor-signed, shipped with a maintenance tool, no special ceremony. `pmxdrv.sys` (Intel-signed) is a surviving member of that everyday class: choosing it is not reaching for an exotic weapon, but using the vendor-signed representative of the industry's standard in-OS firmware-maintenance mechanism that still loads.

**What mechanism of the driver we use, stated openly**: `ssdiag` invokes exactly two interfaces of the driver — `MAP_PHYS` (0x222AB8) and `UNMAP_PHYS` (0x222ABC), physical-memory map/unmap — and no other capability. Each run creates at most two mapping windows: one page of ECAM configuration space (ReBAR/link probes) and the full BAR0 window (4096 pages = 16 MiB, base auto-discovered from the OS/config space, never user-supplied); **every register access afterwards is plain memory read/write on that mapped window** — reads are MMIO reads, the `--reapply` SS0/SS1 writes are MMIO writes through the same window, and the mapping is torn down on exit. To be equally plain about the rest: the driver's underlying capability is **unbounded arbitrary physical memory read/write** — the scoping above (only these two windows, addresses derived solely from the device's own configuration space) lives in our code, **not in the driver**; the driver itself cannot be narrowed, and anyone with administrator privileges could use the same driver for other things. That is exactly why the right to refuse is yours to exercise.

**The defective state, stated plainly**: the lineage has known-vulnerable historical builds; Microsoft's blocklist has **no** pmxdrv entry — nothing external stops the load, the only gate is your own judgment; loading leaves a permanent SCM event-7045 audit record; EDR may alert. It is a capability, not a behavior: the design narrows it, the use decides it. The full triage lives in [#40](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/40).

**You hold two rights of equal weight**:

1. **The right to witness** — run `ssdiag` and read FEAT PLM/SS0/SS1/READOUT yourself, confirming the unlock state with your own eyes rather than anyone's word;
2. **The right to refuse** — you may decline this kernel driver on any or all of your physical machines and still get verification: the loader's own EFI read-backs (pre-boot, driver-free), GEMM output hashes against a stock baseline, `nvidia-smi` telemetry; Linux users additionally have the driver-free `diag-unlock.py`. Reports that skip `ssdiag` (e.g. [#135](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/135)) are recorded in the compatibility matrix as fully verified all the same.

Not installing, not running, not understanding it does not affect the unlock itself — `pmxdrv.sys` exists only for zero-dependency, zero-residue in-OS testing and first-hand evidence.

<a id="av-fp"></a>

### An antivirus flags the download - what now?

**`ssdiag` was never part of the main package, and the zip no longer carries it** (changed in place for v2.1.10): it is an optional download on the release page, and it is the only thing that gets flagged.

The shape of the detection is itself the evidence. `ssdiag` is an unsigned Rust host with a whole kernel-driver PE image embedded in `.rdata` (file offset `0x2C7C9`) - the combination static-envelope heuristics like least. Both 2026-11-05 reports point at the same zip, and the two Microsoft names they quote do not even agree (`Trojan:Script/Sabsik.EN.A!ml` vs `Trojan:Script/Wacatac.H!ml`); the third-party hits are one and the same generic name from BitDefender and its engine OEMs (ALYac / Arcabit / eScan / Emsisoft / GData / VIPRE `Trojan.GenericKD.82039403`, Bkav `W32.Malware.*`, CTX's container heuristic `Zip.trojan.generic`, Elastic's `Malicious (moderate)`) - **a cluster of engines agreeing on a generic name while Redmond's whole catalogue stays silent is the textbook false-positive shape**. Re-tested here: Defender signatures 1.463.142.0 (updated 2026-11-05) report `found no threats` on the official zip.

**One residual hit remains after dropping `ssdiag`**: Bkav alone (`W32.Malware.AEFAF8C4`), and it names the Linux-side `assets-x86_64-linux` - another unsigned Rust binary, and ELF has no Authenticode path at all (the `W32.` prefix on an ELF is Bkav habit, not a verdict). Windows users never load that file and can ignore it; Linux users do not run this engine family anyway.

How to judge and what to do:

1. **Check the hashes first** - **verification is two-layer**: the zip itself is not in `SHA256SUMS.txt` (the sums cover the extracted member files); verify the zip against GitHub's asset digest on the release page, then match each extracted member against `SHA256SUMS.txt`; `ssdiag`'s hash is additionally pinned inside the binary and self-verified at run time. Matching hashes rule out a swapped download. (The two-layer scope was clarified in the field by [D#223](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/223); the earlier wording "the zip is in the sums" here was wrong and has been fixed.)
2. **Keep using the main package** - the unlock itself (`BOOTX64.EFI` + generator scripts + `assets`) is unaffected by this false positive; you can skip `ssdiag` and still unlock and verify (EFI read-back, GEMM hashes, `nvidia-smi`).
3. **If you want `ssdiag`, allow it** - it is a portable tool with no installation, no leftover service (stopped and deleted on exit); add the file or its folder to the Defender exclusions. Treat `Generic` / `!ml` names as false positives; only a concrete trojan-family name is worth a second look.

<a id="bridge-bme"></a>

## What does booter status 0x89 mean?

`0x89` is a Booter WPR-descriptor validation error. If a GPU's upstream PCI bridge has Bus Master disabled, device DMA cannot read WprMeta correctly and can produce this error.

The loader enables Bus Master where needed on the upstream bridge path, verifies it and restores it on completion. Logs contain `DMA-PATH` / `BRIDGE-BME`. Preserve the complete log and inspect those lines and GPU state; do not diagnose from the status alone or change BIOS settings blindly.

SVM, VT-x and VT-d need not be disabled on verified platforms. Task Manager's virtualization status is not the PCI Command Bus Master bit. See the [bridge BME report](bridge-bme-root-cause.md) for mechanism and evidence.

<a id="no-boot"></a>

## The stick flashes and bounces back to the boot menu (or straight into the OS) - no loader.log?

Short answer: **the EFI never executed, so nothing was written.** The loader starts `loader.log` about 110 ms in, before it reads any asset; no `loader.log` and no `tx-log.txt` on the stick means not a single instruction ran. The unlock is volatile anyway - even a real run is gone after power-off.

Check in this order and stop at the first hit:

1. **No `EFI\BOOT\BOOTX64.EFI` on the stick.** That is the only default path removable media is auto-booted from; a `BOOTX64.EFI` at the stick root, or the whole folder landing as `U:\usbroot\`, is invisible to auto-boot. Re-run the maker and copy the **contents** (the layout already carries this path), or move `BOOTX64.EFI` under `EFI\BOOT\`.
2. **Secure Boot is still on.** Toggling "Other OS" is not enough on most boards: go to Key Management, delete all PK/KEK/db keys, reboot, and verify `Confirm-SecureBootUEFI` returns `False`.
3. **You entered through a non-`UEFI:` entry (CSM).** Pick the `UEFI:`-prefixed entry and turn Fast Boot off.
4. Still no `loader.log` with all of the above correct: try another USB port (prefer a rear onboard one) and another stick; if it still fails, report the board model and BIOS version.

### There is a `loader.log`, but it is 0 bytes

**Do not infer the death point from the byte count.** On a number of firmwares (measured repeatedly on AMI/Insyde boards) `Flush()` does **not** commit the FAT directory entry - only `Close` does. So on those boards "file created, 0 bytes" means **this log was never Closed**, not that execution stopped at some line: dying at the confirmation prompt, dying in the preparation stage, or even finishing the transaction without reaching the teardown all look identical.

- [#170](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/170): a full disk image parsed by BPB, scanning the FAT cluster chain, recovered two complete 261-line logs from a file that read as 0 bytes - the transaction had actually passed detach, AUTH-SCAN, the scrubber and the WPR2 layout while the screen had already frozen. That is what retired the "hangs at DETACH" reading.
- [#198](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/198): a newer build that commits the directory entry at creation had its **reopen fail** on such firmware, so the logger degraded to console-only for the whole run and the file stayed empty, while the older build on the same machine (single handle, `Flush`, one `Close` at handoff) wrote 47-56 KB normally.

So the order of work is:

1. **Do not try to fix it with `\tx-flush-lines.txt`** - that knob changes flush cadence, not who commits the directory entry.
2. Run a build that `Close`s at every commit point (`2.1.11-diag3` onwards; `diag4/diag5` instrument the ignition stages too), and the log will stop at the real last line.
3. If you already have a 0-byte file, recover the body from a full disk image with the cluster scan in #170; when the body only ever existed on the console, a screen photo or serial capture is evidence too.
4. When reporting, include the build version and SHA - the same "0 bytes" means different things on old and new builds.

Failures that happen after the stick boots and logs are transaction problems - see [recovery](#recovery).

### Two ESPs, and the chain lands on the dead one (`0xc000000f` / `0xc0000225`)

A machine that has been reinstalled can carry **two ESPs**: the live one, and an old partition whose ESP still holds a stale `bootmgfw` plus a BCD store pointing somewhere useless (`{default}`/`osdevice` on a data volume with no `\Windows`). A chain-started `bootmgfw` adopts the BCD store of the volume it was loaded from and Windows then holds that store open, so chaining the wrong ESP looks like a broken Windows install rather than a chain bug — two field machines, [#128](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/128) and [#142](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/142).

**v2.1.10 and later resolve this by themselves**: the loader walks the firmware's own `BootOrder`, matches short-form HD options by partition identity (the shape real firmware writes), and chains the option Windows is actually booted from. The old enumeration-order scan remains only as the last fallback.

How to tell it worked, from the log:

```
CHAIN-BOOT: firmware resolve: [0] Boot0001 short-form HD identity match on Handle(...) - not by path alignment
CHAIN-BOOT: firmware resolve: [0] Boot0001 -> '\EFI\MICROSOFT\BOOT\BOOTMGFW.EFI' on the firmware's volume - chaining its choice
```

`identity match` plus no `volume scan fallback` anywhere in the log means the resolver took the firmware's choice. Hand-editing the BCD is only needed on older builds; on v2.1.10+ a workaround can be reverted. The `CHAIN-BOOT:` dump prints the whole `BootOrder` with decoded device paths and an `hd(partN,<guid>)` tag per volume, so a wrong pick is visible in one boot.

<a id="handoff"></a>

## The unlock log is green but Windows will not start (`0xc000009a` / Recovery)?

`0xc000009a` is `STATUS_INSUFFICIENT_RESOURCES` at the winload stage (the Recovery screen with "press F8 / F1 / Esc" means it got that far). winload has two independent ceilings, both verified by static analysis of `winload.efi` (2026-11-12, [issue #146](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146)):

1. **Descriptor count — the binding one.** winload ingests every descriptor of the memory map it inherits into a **static pool of exactly 1024 entries** (`winload.efi!BlpMmInitialize`: `MmGlobalMemoryDescriptorCount = 1024`, 48-byte descriptors, no dynamic overflow); pool exhaustion kills the boot with `0xc000009a`. Measured line: 998 descriptors boot, 1005 fail. Fragments cost 1:1 regardless of size - a 4 KiB hole spends the same slot as an 80 MiB one. Affected firmware builds (Gigabyte B550 family in the field) either sit stable-high near 998 or swing 448↔1005 entries between identical zero-write boots; the map is the firmware's own artifact and cannot be merged or re-typed from an EFI app, so a full power-off **retry** is the legitimate workaround and the BIOS version is the real fix. The log prints a hard WARNING above 950 descriptors.
2. **Largest single run — the original #146 mechanism.** winload's boot reservation comes out of a single descriptor without stitching; ExitBootServices converts nothing, so a resident asset pool can split the only large run below 4 GiB even with gigabytes free in total.

Read the discriminator straight off the `CHAIN-MEMMAP` summary line - the `(... N descriptors ...)` total decides which ceiling you are hitting: **> ~1000** = count ceiling (retry / BIOS update); **≤ ~1000 with a halved largest block** = size ceiling (the fix below). And when the count is high, **first ask which path you booted through**: on the affected B550 firmware, the **F12 / BBS-popup boot menu** leaves ~370–475 more map descriptors than **DEL → BIOS → Save/Exit → Boot Override** (the popup re-initializes the console at native resolution and its allocations shred the map), enough to push a board that boots fine via BIOS-override over the pool. Workarounds in order: boot via the BIOS override flow; upgrade the BIOS (field-proven on the first board); or permanently install the layout to an internal FAT32 partition booted by fixed BootOrder so the menu path never runs (see [handoff-lowmem §0c](handoff-lowmem.en.md)).

**Since v2.1.11 the asset pool is handed back to the firmware allocator before the chain, by default**, at zero cost (nothing reads an asset after the transaction). Read it off the log:

```
CHAIN-MEMMAP: ... largest block 2604 MiB (...)     <- after the fix; halved in the failing state
TX-ASSET-CACHE: cleared 9 entries, 84388324 bytes handed back to the firmware allocator
```

- On an older build with this symptom: put an empty `tx-free-assets.txt` at the stick root and run again; on v2.1.11+ that marker is a no-op.
- To get the "not freed" control back, use `tx-keep-assets.txt` (empty); the log then says `cache left resident across the chain`.
- Mechanism, the three-run maps and the spec/EDK2 basis: [low memory at handoff](handoff-lowmem.en.md).

<a id="reconnect"></a>

## Why does the platform reset after a successful transaction?

Some AMI firmware hangs or resets during GOP reconnect after a successful GPU transaction. The loader uses nonrecursive vendor reconnect when a pathless GOP survives detach, skips NVIDIA display functions during pre-chainload reconnect, and writes logs during execution.

If the log stops at `RECONNECT: ConnectController ... begin`, or the platform resets there, fully power off and recover before adding `no-reconnect.txt` to the stick root to skip target GOP reconnect. Windows Boot Manager can establish its own console; skipping reconnect does not waive OS verification.

For Turing `0x91` or a GSP-handoff bug-check after EFI success, see the [compatibility matrix](compatibility.en.md#device-16). Not every black screen is a GOP problem. Evidence: [AMI report](ad106-asus-z790-windows-unlock.md), [X470 record](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70).

**A failed transaction can leave the screen black while the machine is alive.** The `native-GOP` rebuild runs on the success path only; a failure path just unwinds (stock signature -> FLR -> chainload) and does not rebuild the console it disconnected. Field case ([#191](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/191)): after the failing HIGH-staging round the system still booted, SSH worked, the driver and desktop were fine — only the internal panel stayed dark. On a single-display laptop treat that picture as "black but alive": give it time, confirm over SSH or an external screen, or power off and start over; do not pull the stick mid-boot just because the screen is black.

**`NATIVE-GOP: no gop child on target` in the log is usually not a display fault — it means the console was never on this card** (first recorded shape in [#261](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/261)). Three lines settle it: (1) `GOP-PROBE` lists exactly one GOP handle, on the iGPU (`PciRoot(0)/Pci(0x2,0x0)`) with an EDID; (2) the target's `DETACH: ... is not on the target GPU (multi-GPU) - left untouched`; (3) the target's pre-claim VGA subset is `0x0`. The firmware console belongs to the iGPU, `DETACH` never tore down a display path on the target, so there is nothing to rebuild. rc4 and earlier logs still ran the rebuild — we load the card's own stock GOP driver from its shadowed ROM and `LoadImage` / `StartImage` / binding `Supported`+`Start` all return SUCCESS, but in this state the driver publishes no GOP child (it sees no display path it owns), the three probes honestly report none, and the log ends with `native-GOP recovery failed`; **from main (unreleased)**, detach records that the target had no GOP child this boot, and the default path skips the rebuild outright with `TX: native-GOP rebuild skipped - the target had no GOP child at detach ...` instead of the `failed` line (`\tx-gop-disconnect.txt`, the tear-down-every-GOP mode, still rebuilds and keeps its original diagnostic meaning). The console stays however firmware left it, bootmgfw self-establishes, and the `POST-GOP` readback proves the unlock is unaffected — an **informational** form of structured degradation, distinct from #236's `console rebind failed NOT_FOUND` (child created, console re-attach failed) and #188's `no rom image` (no shadowed ROM at all). For the rebuild to actually happen, the firmware itself must boot with the dGPU as the boot display (BIOS-level dGPU-direct / iGPU disabled); the EFI stage cannot change display ownership.


### Legacy/MBR system disk: nothing to chain -> the MBR2GPT migration recipe ([#122](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/122) P3, measured)

The chain target must be an **ESP on a UEFI+GPT system disk**; a Legacy/MBR-installed system disk has no ESP and the chain phase ends `NOT_FOUND` (the loader transaction itself can be fully green - #122 rc1 was exactly that shape: `unlock=true` but `chainload NOT_FOUND`). This is not a loader limitation: chainloading is, by construction, finding `bootmgfw.efi` on an ESP.

**Migration recipe (measured end-to-end by the #122 reporter)**:

1. Dry-run first from an admin prompt: `mbr2gpt /validate /allowFullOS` - only then `/convert` (hard prerequisites: <=3 primary partitions on the system disk, BitLocker suspended, Windows 1703+; the conversion repurposes one primary partition as the ESP and stages `bootmgfw`);
2. Switch the firmware from Legacy/CSM to pure UEFI boot (CSM off); Secure Boot off (OS Type=Other OS - required by the unsigned loader);
3. Clear the now-dead Legacy HD entries from the firmware BootOrder so nothing dead ranks above the stick/Windows;
4. Re-run with a v2.1.11+ stick - the chain phase should find the new ESP via Pass 0/0.5.

**Two boundary facts from the same run**: (1) the loader boot media does NOT need GPT - rc2 booted from a FAT32 partition on an MBR data disk via the firmware's automatic "UEFI OS" entry; only the chain **target** (the Windows system disk) needs GPT+ESP. (2) With pure UEFI the oldest AMI board in the matrix (Z97-K) ran the full chain - firmware capability is verifiable locally beforehand (if it can run the EFI loader from a FAT32 volume, it can UEFI-boot).

### Unlocked, then a CUDA error as soon as load hits? Drop the overclock and the undervolt first

The unlock doubles the Tensor issue rate, so current spikes, VRM load and thermal stress inside the **same power envelope** go up. An overclock or undervolt that used to be just stable stops being stable under the new current profile. Hotter silicon leaks more and needs *more* voltage for the same clock - "it was always fine before" is not evidence.

**Real case** ([#147](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/147), 3060 12 GB): pointer casting for NVFP4/AWQ weights threw `CUDA error: an illegal memory access` and `HostBuffer.truncate failed`; `--disable-cuda-malloc --disable-pinned-memory --lowvram` changed nothing. Removing a 0.875 V UV curve and returning the core to stock fixed all of it. **The tool is not at fault** - the unlock only reaches the existing margin sooner.

Order of work: (1) turn the unlock off and stress-test at stock clock and stock voltage - a card that fails *there* needs a downclock of its own; (2) still unstable while unlocked: drop core **and** memory by 200 MHz and re-run; (3) add margin back step by step. Post-unlock gains are settled by the power wall: a core undervolt usually costs more stability than the clock it buys.

### Compute load pinned at P8? Check for a modded VBIOS first

**Confirmed precedent** ([#90](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/90), 2080 Ti): the EFI transaction was confirmed at source level (booter exit code 0x0, PLM open, selector read-back), yet in Windows graphics ran at P0 while **creating a CUDA context pinned the card to P8 645 MHz** - utilization 70-100 % while power *fell* from 71 W to 44 W. Cause: the card had been flashed with a leaked kingpin-class extreme-test VBIOS (the EVGA 3090/4090 1000 W, XOC 890 W family that NVIDIA hands to OEM overclocking labs for extreme-cooling runs; it carries a separate P-state policy for compute loads - the tell is a `nvidia-smi` power reading far above the retail design, e.g. 565 W against a 300 W wall). **A retail VBIOS recovered it immediately.** The mechanism is filed under [open question #14](open-questions.md); for modded or overclocked cards, parse the current VBIOS power table with `nvoc-cli get-vbios --verbose` before unlocking.

<a id="recovery"></a>

## How do I recover from a black screen, STOP or driver failure?

**Do not retry in the same session.** Firmware may still be running and DMA buffers may remain in use.

1. Fully power off; follow the laptop vendor's EC-discharge procedure where applicable.
2. Remove the USB stick and boot the original OS entry. The tool does not require ESP writes or permanent boot-order changes.
3. Check Device Manager errors and system event logs. Preserve `loader.log`, then diagnose the specific symptom.
4. If the driver still misbehaves, check again after another full power cycle; use DDU and reinstall the driver if necessary.

Power loss clears the unlock. A black screen, transaction failure or driver error alone neither establishes permanent hardware damage nor replaces checking the recovered device.

<a id="safety"></a>

## What are the safety rules and implementation limits?

- Verify writes by read-back; status codes or completion messages are not independent execution evidence.
- No retry after a fault in the same session. Use USB / the one-time boot menu; do not overwrite the system ESP or install persistent BootOrder entries.
- Back up existing USB boot files. Disable Secure Boot or use a trusted self-signed EFI.
- Regenerate NVIDIA firmware from pinned inputs locally; do not distribute or commit it. Stop on verification failure.
- The loader routes by family and records PROM CRC32; **per-card ROM/PROM SHA-256 enforcement is not implemented**.
- [WPR2 guards](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/4) depend on pre-FLR and family behavior. Not every refusal is zero-write: confirmation abort and strict-D0 entry abort have their own zero-write semantics; `EBUSY` may follow PCI configuration changes and console detach.
- [Host descriptor parsing](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/25) supports V2/V3; V1 is unimplemented, and the encrypted flag is not retained or enforced. [Runtime loading](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/20) uses portal; Falcon DMA remains experimental.
- Some [simulator](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/22) verdicts come from fault switches and simulator-side markers. Hardware acceptance still depends on read-back.

For Turing `0x91`, check actual DMA addresses and usable VRAM, not just HIGH labels or nominal capacity. [Diagnostic record #14](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/14), [#19](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/19) and the [forensics matrix](tu10x-booter-0x91-matrix.md) retain the relevant conditions. `tu10x-fb-gb.txt` is a manual GiB override when register decoding fails; use it only as directed for diagnosis, not normal installation.

Boundary tracking: [diagnostic offsets/masks](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/18), [ROM bounds checks](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/21), [bash multi-GPU GSP checks](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/23), [LIC descriptors](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/24).

See [AGENTS.md](../AGENTS.md) for development gates and experiments, and [AGENTS.deploy.md](../AGENTS.deploy.md) for deployment. BAR1 expansion is separate from this tool's compute unlock. License: GPL-2.0-only.

<a id="mechanism"></a>

## How does the unlock work?

PLM controls register write permissions; the host cannot directly change protected overrides. Before the NVIDIA driver loads, this tool executes a signed-firmware boot chain, opens permissions through the Booter's V67/ROP path, writes rate overrides, verifies read-backs and hands control to the OS. Loading uses register portals; signature DMA is part of the payload trigger.

| Family | Execution and handoff checks |
| --- | --- |
| Ada | Marker, HALT, target PLMs and selector read-backs; AON state after FLR |
| Ampere GA10x | PLM flip verifies execution, followed by selectors and handoff state |
| Turing | FEAT at `0x4096xx`; hands off gate-open (default since 2026-11-08; `\\tx-relock.txt` restores the legacy relock), skip FLR on the unlock path |

Ada FBPA read-back `0xffffff0f` is an accepted observation, not a standalone failure. A scrubber handoff/timeout warning is not the final execution verdict either: check subsequent FWSEC, family-specific execution, selectors and handoff results.

The stock signature is restored before handoff. The unlock does not flash the VBIOS or persist through power loss. Mechanism and evidence: [Ada report](ada28e0-uefi-report.md), [Turing report](evidence/m3-tu106/host-and-final-result.md).

<a id="nvoc-results"></a>

## What should I know about combining this tool with nvoc?

This tool removes Tensor issue-rate limits; nvoc supplies in-OS power, voltage and overclocking controls. The kmd power-wall unlock works on mobile GPUs; 10/20-series desktop cards have driver read-back evidence but unverified performance effects. `set-volt-rail-limit` voltage-wall control applies to 10-series and newer GPUs.

Verify this tool independently, then select controls and tests using the [nvoc documentation](https://github.com/Skyworks-Neo/nvoc). After issue-rate unlocking, gains may be limited by power, clocks, cooling, memory access and workload precision. A microbenchmark multiplier does not apply to every application.

Combined operation can exceed factory power and thermal envelopes. Monitor Board Power, Hot Spot, throttle reasons and power delivery, and check cooling/supply headroom. Extreme test voltages and power are not recommended settings for other cards; the user assumes physical-damage risk from excess power, voltage or temperature. Measurements and conditions: [combined test report](ada28e0-uefi-report.md). Throughput tool: [cli-stressor-cuda-rs](https://github.com/Skyworks-Neo/nvoc/tree/main/cli-stressor-cuda-rs).

<a id="reporting"></a>

## How do I report an incompatibility or failure?

Submit the full `loader.log`, GPU model/PCI ID, platform, driver, display mode, entry path and in-OS diagnosis to [Issues](https://github.com/physicsdolphin/nv-uefi-unlocker/issues). **No need to look the hardware up by hand** - run `ssdiag --inventory` on Windows or `python3 tools/diag-unlock.py --inventory` on Linux and paste the block straight into the form (both read-only, no admin / no root respectively; VBIOS, driver, subsystem ID and chassis form are all in the block). Modded cards need the original model, VBIOS and VRAM capacity. Redact machine-specific details as needed. For performance issues, include matched dimensions, precision, power and before/after results.

Use [Discussions](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions) for success reports. For no in-OS gain, check [RTD3](#linux) and [read-back verdicts](#ssdiag) first.

<a id="layout"></a>

## Where are the code and research records?

| Directory | Contents |
| --- | --- |
| `loader/` | UEFI application |
| `assets/` | Download, verify, extract and generate payloads |
| `sim/` | Host simulator and transaction tests |
| `tools/` | In-OS diagnostic tools |
| `docs/evidence/` | Raw experiments and milestone evidence |

`cargo build --release` builds host assets/sim. Use `cargo efi-tx` for the transaction loader, or `cargo efi` for read-only sampling. `cargo xt` runs host tests. Do not run `cargo build --workspace` for the host target.

Use [compatibility](compatibility.en.md) for device status and exceptions. [Community hardware records](community-reports.md), the [bridge BME report](bridge-bme-root-cause.md) and [TU10x 0x91 investigation](tu10x-booter-0x91-matrix.md) retain independent evidence and reasoning. Use [AGENTS.md](../AGENTS.md) for development and [AGENTS.deploy.md](../AGENTS.deploy.md) for deployment.
