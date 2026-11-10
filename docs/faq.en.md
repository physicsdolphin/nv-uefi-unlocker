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

The maker emits the assets into a local `usbroot/` directory (the `*.bin` files flat, plus `EFI/BOOT/BOOTX64.EFI`). Copy the **contents** to the stick root (do not copy the directory itself as `U:\usbroot\`). Do not copy extraction intermediates as runtime assets.

**Why does that path get recognized automatically?** UEFI firmware carries a fixed fallback boot path for **removable media** - `\EFI\BOOT\BOOTX64.EFI` on x86_64. A file at that path is listed and executed as a boot entry on its own, **without** writing or modifying any NVRAM boot entry - which is why the "no persistent boot entry" discipline ([safety](#safety)) holds: pull the stick and the machine is back to stock. Other architectures have their own default file names (aarch64 uses `BOOTAA64.EFI`); the tool, the assets and every field measurement so far are x86_64-only - if you need aarch64 (ARM laptops/servers), please open a feature request or PR describing the platform and firmware; the developer currently has no test hardware for it.

**`-Out G:\` fails on Windows PowerShell 5.1 with `New-Item : The path is not of a legal form`** (a bare drive root ends in `\`, which `New-Item -Path` rejects; [#195](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/195)). The script now strips the trailing backslash when creating the directories; on older packages pass **`-Out G:`** (no `\`).

`F` is `ad102` (40 series), `ga102` (30 series) or `tu10x` (20 / 16 series):

| Location | Files | Purpose |
| --- | --- | --- |
| `EFI/BOOT/` | `BOOTX64.EFI` | Boot application; back up an existing file before replacement |
| Root | `booter_load_F.bin` | Booter bundle (BTL1) |
| Root | `gsp_boot_F.bin`, `gsp_boot_desc_F.bin` | GSP boot image and v5 descriptor |
| Root | `fwimage_F.bin`, `fwsignature_F.bin` | GSP-RM image and stock signature |
| Root | `v67-F-four-plm.bin` | Unlock payload |
| Root, Ada / Ampere | `scrubber_ad10x.bin` | SEC2 scrubber |
| Root, Turing | `sec2_bl_tu10x.bin` | FWSEC bootloader |
| Root, Turing | `gsp-gate.txt` | Maker's GSP check record |
| Root, after execution | `loader.log` | Stage log; save before another run |

Removing `v67-*.bin` selects replay: no unlock payload, but drivers are still detached, signed firmware runs and the GPU may reset. **Replay is not read-only.** Build with `cargo efi` for read-only sampling.

Optional markers go in the stick root; normal use needs none:

| Marker | Behavior |
| --- | --- |
| `tx-require-d0.txt` | Abort before GPU writes/reset when entry is not D0 |
| `tx-select-bdf.txt` | Contents `bb:dd.f` or `dddd:bb:dd.f`; process only that card; invalid or absent target never selects another card |
| `input-reset.txt` | Optional per-device USB port reset before CONFIRM: one `usb VVVV:PPPP` per line (hex VID:PID, `#` starts a comment). Listed devices that are present get one UsbPortReset and a 1.5 s re-enumeration wait - for devices whose phantom key starts the moment the firmware USB stack comes up (USB mice/receivers exposing a keyboard interface); a port reset does not help every device (observed), in which case the INPUT-SETTLE ghost-key suppression is the fallback. Each USB interface also logs one `USB-INV` line to identify who is reporting keys |
| `chain-target.txt` | First nonempty, non-`#` line specifies an EFI path tried on each volume before the default order; missing path is logged and falls back. **Check this first when the firmware BootOrder holds a stale cross-disk entry**: the default resolution follows BootOrder and can land on another disk's old loader ([#204](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/204) hit a leftover `\EFI\ubuntu\shimx64.efi` — green transaction, chain into Linux); put `\EFI\Microsoft\Boot\bootmgfw.efi` on the first line to pin the local chain target |
| `tx-preload.txt` | Enable the pre-transaction OS-loader preload (P2-7, TOTORONG craft): LoadImage bootmgfw before the transaction, StartImage it right after the handoff PE probe. **The default since 2026-11-09 is NO preload** - the post-transaction firmware BootOrder-resolved chain is the baseline: the [#198](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/198) nine-cell matrix matched "preload + dirty card" to every post-handoff logo freeze, while "preload + clean card" (readonly) and "no preload + dirty card" (v2.1.5 run-10, which predates the feature) both boot; the preload's original robustness motive (a post-FLR chain failing on a degraded allocation landscape) is covered by the shared firmware BootOrder resolution (#128/#142). Add the marker only to reproduce the old behavior for A/B runs; the retired `tx-no-preload.txt` marker no longer has any effect |
| `no-reconnect.txt` | Skip target GOP reconnect for a post-transaction reconnect hang/reset |
| `tx-reconnect-nonrecursive.txt` | Non-recursive reconnect only, keeps the vendor GOP binding (console stack left to bootmgfw to rebuild); gentler than `no-reconnect.txt`, for the "boots fine but hangs in reconnect" family (issue #52 X470, [Discussion #131](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/131) X99/C610) |
| `i_know_what_i_am_doing.txt` | Auto-run after an N-second countdown, any key aborts; the first marker line sets N (0-30, default 5). **0 = no wait**: PROCEED fires as soon as the read-only prep (enumerate/select/asset reads into RAM) finishes - the abort window is the prep itself, key polls still run every tick; legal since v2.1.12-rc1 (previously clamped to 1). The armed path fills the countdown window with read-only prep, so the auto-run starts from memory |
| `tx-flush-lines.txt` | First-line number (0-4096) overrides the count-based log-flush cadence (default every 16 lines; 0 = stage-marker lines only). Raising it reduces the occasional USB slow-write stalls (2026-11-01 TU106 run: four ~1.2 s stalls, all flush-aligned) at the cost of losing more of the log tail on a hang; stage-marker lines always flush |
| `tx-quiet.txt` | Quiet mode (issue #38): the console mirrors verdict lines only (banner/confirm/per-card target and result/HANDOFF/FINAL/CHAIN) while `loader.log` still records every line; if the log file cannot be opened the console stays complete. Turns minutes of echo into seconds on iGPUs that render ~1 line/s; combined with `i_know_what_i_am_doing.txt` it is a fully unattended fast run. A frozen screen during the transaction is normal, not a hang |
| `tx-gop-disconnect.txt` | Diagnostic switch: disconnect every GOP handle (including the iGPU) instead of only the target card. **Not a speed lever** - once the console is silent it buys nothing, and the other card's console is never restored by the loader (it depends on bootmgfw rebuilding it; a known Code 12 risk shape, see issue #35) |
| `tx-reconnect-legacy.txt` (tx builds) | **Opt-out**: restores the old reconnect path (recursive ConnectController + PR#13 non-recursive logic). The default has moved to native-GOP console rebuild (the card's own shadowed-ROM GOP); use only if the new default misbehaves on your board |
| `tx-native-gop.txt` (tx builds) | **Optional pin**: native-GOP is now the default, so this marker is no longer an enable switch; its first line may name a volume GOP module to use instead of the card's shadowed ROM (compatibility-package form); empty/absent = the shadowed ROM |
| `i_know_what_i_am_doing.txt` | Auto-run after 5 seconds, any key aborts; existence enables it, contents ignored |
| `tx-no-canary.txt` | Skip the whole reset canary group (A/B control; see [relock](#relock)) |
| `tx-relock.txt` (tx build) | **Escape hatch**: relock the FEAT PLM to `0xffffff8f` before handoff (all three families). The default since 2026-11-08 is to hand off with the gate OPEN everywhere (Turing joined the Ampere/Ada open-gate shape; a TU106 bench proved the open gate healthy end-to-end and leaves the in-OS write surface available, see `docs/evidence/tu106-1f07/keep-open-2026-11-08/`). The Turing arm's observed relock value is `0xffffffef`/`0xffffff8f` depending on the board; the Ampere/Ada arm (relock after the FLR verdict) has never been benched |
| `tx-ultrafast.txt` (tx build) | **Speed profile** (for machines with several verified rounds): skips every purely diagnostic pre/post probe - M1 cold-state sample, GOP probe, AUTH-SCAN, ROOT-DIR dump, opener/topology dumps, the post-payload FEAT sweep, HANDOFF-PCI / HANDOFF-VGA-FINAL dumps, the POST-GOP probe, CHAIN-MEMMAP - and folds in quiet + **transaction-anchor-only flush** (much tighter than the stage-marker set: ~80 flushes/run → ~15; a hang between anchors leaves a bigger log blind spot; an explicit `tx-flush-lines.txt` still wins). **None of the verdict readbacks are skipped**: the selector write/readback, the WPR2/POST-BOOT verdicts and the booter load audit all still run - readback is the only success criterion. **The reset canary is NOT skipped** (two writes + two reads, milliseconds; high reset-forensics value - since 2026-11-09 ultrafast arms it too and `HANDOFF-OUT` shows the real verdict; use `tx-no-canary.txt` to skip it explicitly). Every skipped stage leaves one `ultrafast - skipped` line in the log. Only recommended on machines with multiple clean recorded rounds |
| `tx-no-log.txt` (tx build) | Never opens `\loader.log` (console-only output all run; quiet is structurally refused by its "console is the only record" guard, so the console stays complete). A hang means zero forensics - only for armed runs on machines with multiple verified rounds |
| `tx-no-console.txt` (tx build) | No console mirror at all (zero GOP text-output cost - the minutes-per-run slow-panel shape of issue #38 disappears). The confirm prompt is invisible - **combine with the armed marker** (keyboard semantics unchanged: any key during the armed countdown still aborts); together with `tx-no-log.txt` this is a fully silent run with no on-screen and no on-stick evidence if anything hangs |
| `tx-keep-assets.txt` | **Escape hatch**: do not hand the asset cache back before the chain (freed by default since v2.1.11, see [handoff memory](#handoff)). Only for A/B or for the "reads assets after the chain" shape; the older `tx-free-assets.txt` marker is still accepted and is now a no-op |
| `tx-low-staging.txt` / `tx-fwsec-high-staging.txt` | Manual staging-band pins: the former moves the whole booter set (meta/signature/radix/boot) below 4GiB, the latter moves the FWSEC staging back to the high region (FWSEC defaults LOW since v2.1.8). **Since v2.1.12-rc1 there is an automatic fallback (STAGING-FALLBACK)**: on Turing's HIGH default, a booter halt with `0x89` plus an intact host-side meta magic (= our write landed, SEC2's read did not deliver — the [#74](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/74)/[#156](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/156)/[#191](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/191) class) triggers exactly one LOW-band retry — no FLR, no loops; `0x91` (the opposite [#16](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/16) sub-4GB read-lock class) and a corrupt host page never trigger it, and a failed retry falls to the canonical path with a VT-d hint. The band requirement is machine-dependent in both directions; the case table lives in the [compatibility matrix](compatibility.md) |
| `tx-acpi-dump.txt` | Read-only: dump the firmware DSDT and every SSDT plus a keyword index to `\acpi\` on the stick (decompile offline with `iasl -d`; answers "who owns this card's power"). Runs before the confirm and works with no GPU present |
| `tx-exit-fw.txt` | **Escape hatch** ([PR #209](https://github.com/physicsdolphin/nv-uefi-unlocker/pull/209), issue #208 finding A): skips the chained `bootmgfw` start and hands back to the firmware boot manager after the transaction, so the firmware's own BootOrder boots Windows natively. For machines where a chain-loaded bootmgfw dies with `\BCD` 0xc0000034 (MSI GP66 11UG/AMI, field-proven: path, volume and image bytes were all exact, so the failure is a boot-context difference inside bootmgfw - only the firmware can start it correctly). **Precondition**: the persistent BootOrder must list Windows Boot Manager ahead of the stick, or the exit loops back into the loader; a one-time boot-menu entry falls through. The abort path (30 s timeout / wrong key) passes through the same check - with the marker present an abort also hands back to the firmware, which on a chain-broken machine is the better outcome (chaining would land on the Recovery screen anyway). The pre-transaction preload is skipped automatically when this marker is present |

Without the auto-run marker, only Space or Enter confirms; another key or a 30-second timeout aborts. Chainload order: Windows Boot Manager → supported Linux shim/grub/systemd-boot/Limine → generic `EFI/BOOT/BOOTX64.EFI`. The last tier offers a 5-second key-to-exit window. Slow-console speedup is the `tx-quiet.txt` console whitelist (issue #38), not async output - UEFI boot services are single-threaded and GOP text rendering is a synchronous call, so asynchrony cannot raise throughput; every log line carries a `[+ms]` stage-timing prefix so echo cost and transaction waits separate in `loader.log`.

Multi-GPU default: process all supported cards in firmware order; skip unsupported/already-unlocked cards, handle a card's failure without retrying it. Pin the BDF when only one card is intended.

### Can it be installed to an internal partition for automatic boot?

Yes, but as an **expert, at-your-own-risk operation**: the shipped workflow stays USB / one-time boot menu, and the project neither provides nor recommends internal-partition boot changes for general users. Facts and boundary:

- `bcdedit /create /application osloader` produces a Windows Boot Loader entry whose `device` sits on a drive letter that dies on reboot - firmware never enumerates it; the `bootsector` flavor silently leaves `device` unset. **Neither reaches `{fwbootmgr}`** - don't spend hours there (field-tested in [Discussion #104](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/104), dual 4080 SUPER).
- The working shape (#104): shrink a data volume by ~2 GB, lay out a FAT32 partition exactly like the USB stick, then pick it from the BIOS boot menu (shows as `UEFI OS`) - manual reorder or one-time F12. Reversible from the BIOS menu back to Windows Boot Manager, nothing flashed.
- Why no automation script: for most users "my boot partition broke" is the tool's fault, and resize scenarios vary wildly (dual-boot, cross-disk entries, btrfs/zfs with no online shrink). **Policy: the recipe above is documented, automation is not provided, downstream forks may ship scripts and cross-link.**

For preparation on a different host, use `-SkipGspCheck` / `--skip-gsp-check`; target GSP prerequisites still apply. Offline Turing archive `gsp_rm_boot_dbg.bin` selection uses `-ProvenBlob` / `--proven-blob`; length/hash validation remains mandatory. Do not substitute versions or bypass failed checks. For Windows PowerShell TLS download failures, the bash maker can run under WSL / Git Bash.

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

**Field workaround for the hybrid-output "no gain" family (#171 -> [#224](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/224), 2026-11-10)**: a Hasee TX9 3070M (`249d`, hybrid output / not dGPU-direct) previously showed "no unlock in the OS, no compute gain" ([#171](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/171), closed); after **adding an HDMI dummy plug + the OS "duplicate displays" mode**, the same machine ran an armed auto-run with the chain fully green and in-OS ssdiag read `UNLOCKED (override values live)` (PLM OPEN / SS pair / READOUT full speed / canary intact) — the dummy plug makes the hybrid platform treat the dGPU as display-active, giving the "hybrid output" row above a verified workaround. The record carries no performance A/B and no sleep-persistence data. Note: the 3070M (Ampere) needs no Turing GSP registry prerequisite; enabling it is harmless.

**Laptop internal panel and brightness (DDC-CI)**: on some laptops the old firmware reconnect left the internal link untrained after an unlock — the backlight dropped out and DDC-CI brightness control stopped responding (one confirmed case on a Legion R7000 2024; the legacy path reproduced it on the first attempt and v2.1.10 fixed the same machine). Since v2.1.10 the default native-GOP rebuild re-`StartImage`s the card's own stock GOP driver, which re-runs link training before Windows and clears the fault. On such a machine the legacy opt-out `tx-reconnect-legacy.txt` brings it back.

TU106 with GSP mode has verified successful hibernate-resume handoff, but that does not replace in-OS checks on other machines. See [OS verification](#ssdiag).

<a id="linux"></a>

## Why is there no gain on Linux despite a green log?

Check in-OS selectors and power state first. RTD3 can park an idle GPU in D3cold, clearing the volatile unlock; successful EFI read-backs do not prove continued OS persistence.

If your distribution enables `NVreg_DynamicPowerManagement=0x02` in `/usr/lib/modprobe.d/nvidia-runtimepm.conf`, override it with the same basename in `/etc/modprobe.d/`. Back up existing configuration first:

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

Turing hands off with the gate OPEN by default since 2026-11-08 (like Ampere/Ada); only `\\tx-relock.txt` produces a relocked value - do not read the marker arm's normal value as a loss; see the [Thunderbolt section](#thunderbolt) and [cold values](cold-values.md).

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

Row 6 is special: the reset is performed by **whichever CM establishes/re-establishes the tunnel** — on a USB4 host that is the OS-side CM, before any driver binds, so re-arming after the takeover has no host-write path (`--reapply` is structurally dead, hot-add included); a firmware-CM host does not hit this row at boot (see the host taxonomy in the Thunderbolt section), only on replug. The only out is suppressing that teardown, which is field-verified on both OSes (command and cost in the section). Capture by chaining into Linux with `modprobe.blacklist=nouveau,thunderbolt` and `tools/diag-unlock.py` (rEFInd instead of the ISO's own GRUB, Secure Boot may stay on).

Rows 1-2 are physical events, row 3 is the firmware contract (below), row 5 is the only one we can still change in our own code, and row 6 is an OS-side knob (not in this tool) - `\tx-reconnect-legacy.txt` makes the handoff tail reconnect the vendor stack instead of `StartImage`.

**Row 1 has a full field pairing caught by both axes** (2026-11-04, [#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37), HP Victus 16 `25a2`, hybrid-only platform, `docs/evidence/issue37-hp-victus-25a2/`): on the EFI side the canary was set and read back intact with the card in D0 at handoff; in the OS the canary is cleared + FEAT re-derived to factory + the root port `00:01.0` reading `D3` in PMCSR. FLR is excluded by the same log (the chain's own `POST-FLR` proves GA10x selectors survive it), so an event that exceeds FLR is the conventional-reset family, and the port in D3 tips the form to row 1. Contrast: our TU106 desktop read canary intact with both endpoints D0 - that is what a write-type loss looks like; #148 (GA107M 25a0, Linux) was the reset-free devinit rewrite (its canary would have been intact). Same die, different platform, different row. The canary table's fourth cell (`intact` + FEAT live) now has its first field instance too (2026-11-05, [#109](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/109#issuecomment-6650530730): same card, same VBIOS, same subsystem, B650 desktop - `intact` at the handoff and in the OS, both endpoints D0, `UNLOCKED`). "Nothing happened" is a reading as well: it eliminates card, VBIOS and driver from the discriminating variables and leaves only the platform.

### Who actually cuts the power on the platform

A dGPU has only two power sources: a desktop card is fed straight from the slot; a laptop / all-in-one dGPU is built into the platform and powered through a GPIO switched by the ACPI `_PR3` `_ON` / `_OFF` methods (the BOCO shape). **D3cold is defined as running `_OFF` on those power resources**, with the sequence: root port broadcasts PME_Turn_Off -> link drops to L2/3 -> PERST# asserted -> power removed. **"Desktop cards have no such path" is wrong**: measured on a Z170 desktop board (2026-11-04, `docs/evidence/tu106-1f07-acpi-dump/`) each of the three PEG root ports has a `_PR3` pointing at its own `PG0x`, reset and rail-enable are real GPIO pairs, and `_OFF` asserts PERST# and then cuts the slot rail - slot power does not mean the root port cannot cut it.

A sharper one (**hypothesis; the evidence is Linux, unverified on Windows**): at startup the OS synchronizes power resources nobody references, and ACPI permits `_OFF` on a resource that is already off; if the firmware AML is not idempotent, that redundant call really cuts power (coreboot's Brya board tracks dGPU power state specifically to dodge the open kernel's redundant calls). On this board `_ON` / `_OFF` **are** idempotent, but through hardware readback rather than `_STA`: both entry points run `CCHK` and return when the port already answers a vendor ID or the rail GPIO already sits in the target state. Repeats are therefore harmless - as long as the GPIO state is trustworthy. The hypothesis needs a board with non-idempotent AML; here the likelier path is Windows itself issuing D3cold.

This explains three things at once:

- [#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) reproduces **deterministically**: dGPU has no display, no driver bound, `D3` in-OS with MEM decode off - the driver never participates and it still loses, so the writer is not in the driver;
- why the `D3Cold` registry switch does nothing: it gates D3cold requests, not the startup synchronization of power resources;
- why the "entry mode" flips the outcome: entering from Windows Advanced Startup leaves the dGPU held by the previous driver, while a BIOS cold start has it parked and pulled up by the loader - unreferenced in the OS's accounting.

[#28](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/28) is a side observation: in hybrid mode the firmware enumerates only the dGPU's audio function, i.e. the whole card is parked unpowered. Modern Standby additionally requires the root port to apply a fundamental reset to downstream devices on D0->D3 (Microsoft firmware requirements), a third platform-side trigger.

Getting hit takes three things at once: the topology has the dGPU powered by ACPI/EC (or the whole link goes over USB4), nothing references it at handoff, and the machine's AML really cuts power or asserts PERST# - the third is vendor-specific, so the rate follows platform classes rather than chance. Why the desktop exceptions (Z390P, B450M-A, B350M) differ from B650 is unknown and nobody has reported CSM state. **Issues are a self-selected sample with no denominator**, so any percentage below is an order of magnitude, not a probability.

To look yourself: pull the DSDT / SSDT of the parent root port, check whether `_PR3` exists, whether `_ON` / `_OFF` are idempotent, and which GPIO carries PERST#. Half of these arguments are settled at that step. The AML says what the platform **may** do; the `POWER:` lines in `ssdiag` 0.6.8+ say **how far it got**: the card's and its upstream root port's `PMCSR` / D-state, and a port reading `D3` means `_OFF` already ran. It is a snapshot - `D0` does not prove the port never went to `D3` this boot (the canary owns that).

Still unsplit: **GC6 and GCOFF are different targets** (the former keeps self-refresh, the latter is a full power-off); the `_PR3` path and the GC6 wake path are currently discussed interchangeably and need one dedicated comparison.

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

1. **Block the reset sources**: turn off RTD3 ([Linux](#linux) / the equivalent Windows power setting on the driver), Fast Startup and Modern Standby, and PCIe idle power gating in the BIOS; never use "shutdown" as a restart.
2. **Change the entry state**: dGPU-direct first; in hybrid mode the MUX/panel ownership decides whether the device enumerates at all ([#148 record](#relock-148)).
3. **Repair afterwards**: with the gate still open, `ssdiag --reapply` / `diag-unlock.py --reapply` writes the selectors back; a locked gate means replay.
4. **Report with evidence**: the `POST-FLR` / `POST-GOP` / `HANDOFF-OUT` lines from `loader.log` (including `CANARY` and `wpr2_lo`), the in-OS read-back values (from `ssdiag` 0.6.5+, which prints a `CANARY:` verdict), `powercfg /a`, the device power state and the event log. Without these it is guesswork.
5. **Turn it into a platform property**: if a platform is confirmed to have `_PR3` and to actually cut the rail, record it as a platform property rather than chasing it as a driver bug.
6. **Give the dGPU a reference**: attach an external display to a dGPU-wired port and cold start. The Linux community saying is that a monitor on the NVIDIA side blocks D3cold (anecdote); if the loss disappears, that supports the "unreferenced -> synchronized off" hypothesis.

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
| S0ix / Modern Standby | Recent laptops | Driver runtime PM may park the dGPU in D3cold (rails off) | Usually lost; selectors back to factory | Read-only check first; often repairable with `--reapply` |
| S3 traditional sleep | Most desktop BIOS "Sleep" | Whole platform off except RAM | Always lost | Same, depending on the check |
| S4 hibernate / fast-startup "shutdown" | Laptop hibernate, Win10/11 fast startup default | Full power off | Always lost; equivalent to a cold start | Same, depending on the check |

Check which states your machine supports: `powercfg /a` in an admin prompt - "Standby (S0 Low Power Idle)" is S0ix, "Standby (S3)" is traditional sleep; machines listing both decide at runtime via firmware/driver.

Mechanism notes:

- **The discriminator is whether the rails dropped, not whether you "slept"**. S0ix nominally stays in S0, but the dGPU can be powered down independently when idle (same mechanism as Linux RTD3, [previous section](#linux)); S3 cuts the whole platform. Either way the wake-up result can look identical: selectors re-derived to factory by devinit.
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
- **Windows**: set the `Usb4DeviceRouter` service `Start` to `4` (Disabled) and leave `Usb4HostRouter` at `3` (Manual). **Disable the device side only, keep the host side** - disabling both leaves no tunnel at all. On two different host stacks (ASMedia TB4 + AD102 desktop / Intel native USB4 + GA104 laptop) the card enumerated normally and bound `nvlddmkm`, the canary stayed `intact`, `ssdiag` returned `UNLOCKED`, and the AD102 host reached 95% of the full-rate tensor figure in an 8192³ FP16 GEMM plus a locked-clock sweep with `nvidia-smi -lgc`. The cost, stated plainly: the enclosure loses its device-router services, its USB ports die and both `USB4\ROOT_DEVICE_ROUTER` nodes show `Problem 32` (Code 32 = service disabled, not a failure); GPU, card audio and `nvidia-smi` are all fine. `DevPnPCapabilities=0x100` is disproven; the `Usb4HostRouter` node's `Device Parameters` subkey holds only WDF/DMA/interrupt keys, no reset-policy surface.

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

Turing hands off with the gate OPEN by default since 2026-11-08 (like Ampere/Ada; a TU106 bench proved it end-to-end: neither the card's own GOP rebuild nor the Windows driver init re-closes it, in-OS PLM stays `0xffffffff` with the canary intact, see `docs/evidence/tu106-1f07/keep-open-2026-11-08/`; older builds relocked by default and `\\tx-relock.txt` restores that). The open gate also frees the in-OS write surface: the ssdiag Turing read-only restriction on `--reapply` can now be lifted (tool-side follow-up), and the SS0 4-bit rate-field encodings become explorable from within the OS. **The marker arm's relock value itself is not a failure**: with `\\tx-relock.txt` present the observed value is usually `0xffffffef` (bits 5:6 set) while the cold default is `0xffffff8f`, and it varies with board and driver version - the `1e04` 22G mod ([#74](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/74)) has been measured reading `fully closed 0xffffff8f` with the SS pair still holding the unlock values, so `8f` alone is not a failure verdict; read the `ss0/ss1` on the same line. `tu10x: plm=0xffffffff` (default) or `tu10x: plm=0xffffffef` (marker arm) in `HANDOFF-OUT`, either together with `tu10x_unlock=true` and SS0/SS1 still reading `0x88888888`/`0x8`, is the normal handoff shape ([cold values](cold-values.md), [#102](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/102), [#55](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/55); #54/#74/#102 are closed completed). Source: [Thunderbolt platform record](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54).

<a id="secureboot"></a>

## Does it work with Secure Boot on?

**Scope**: starting the USB tool without turning Secure Boot off. It does not cover anti-cheat clients that demand Secure Boot be *disabled* (for example the ACE-family clients that attest the boot chain); handle those per the vendor requirement, this section gives no procedure for them.

First, what this is not: it is not a way around Secure Boot. The boot chain is still verified end to end; the trust root grows from "a certificate Microsoft issued" to "that certificate plus yours". The only window this tool can work in - the firmware stage before any driver loads - is inside Secure Boot's jurisdiction, and this project holds no Microsoft or UEFI CA signing identity.

Three limits; without them the steps below set the wrong expectation:

- **Self-signing does not fix `ssdiag`.** `pmxdrv.sys` is an Intel/COMODO cross-signed driver from 2018 with no Microsoft root; `tools/ssdiag/README.md` says the load is **expected** to be refused, and its three carve-outs (OS upgraded from Windows <= 1607, Secure Boot off, signing certificate issued before 2015-07-29) cannot apply to a leaf certificate issued 2018-10-24. So with Secure Boot on, in-OS read-back comes from `HANDOFF-OUT` in `loader.log`. `tools/diag-unlock.py` on Linux is a zero-driver path, but **nobody has verified it with Secure Boot on**. `ssdiag --reapply` still needs Secure Boot off for one boot.

  **One counter-datapoint (2026-11-09, [bailu974's record](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot))**: on an Insyde laptop (COLORFIRE MEOW R15 24) with Secure Boot on, `ssdiag` printed this pre-written warning **but the driver actually loaded** - the tail shows `driver service removed` and the `0x823xxx` MMIO readings are real (without the driver those values cannot be read). The mechanism is unknown (that machine's DSE / vulnerable-driver-blocklist state was not captured); **the default expectation is still refusal** - but treat the warning as a hint, not a verdict: check for the trailing `driver service removed` and complete register readings before concluding anything, and when reporting, say whether your SB-on machine loaded or refused. Either answer moves this row.
- **Anti-cheat is not guaranteed.** Self-signing leaves the Secure Boot state bit on; the maintainer states that Vanguard (LoL) checks that bit, and **anti-cheat itself was not tested by anyone** (the reporter this section leans on says so explicitly). ACE-family clients validate the TPM/PCR[7] chain, and a self-signed chain measures differently from the stock boot (see [what each route costs in measurements](#secureboot-vs-ffs) below).
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

### Route B: MOK + shim (two machines reported, one a full desktop run)

The firmware verifies only the Microsoft-signed shim; your key takes effect past it through MOK, and the firmware key database is not modified. Basis: two field reports — [#162](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/162) (Secure Boot on, v2.1.10, RTX 30 laptop, `ga102`, PK/KEK/db untouched) and [#210](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/210) (2026-11-08, RTX 3080 20GB on an ASUS ROG Z790, official v2.1.10 package plus an added `.sbat` and a self-signed MOK, full chain green through handoff, in-OS `Confirm-SecureBootUEFI=True`, certificate present in `MokListRT`, PK/KEK/db unchanged). The scripts from #162 live at [J0s3f/nv-unlock-secureboot-esp](https://github.com/J0s3f/nv-unlock-secureboot-esp) as a reference implementation and are not vendored here. What #210 adds over #162 is the reproducible section parameters (step 3) and the countdown-keypress step (step 7); both come from #210 only.

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
| Evidence | route B: two machines, one a full desktop run with the `.sbat` parameters and the countdown keypress actually executed; route A: second-hand | [feasibility document](dxe-driver-ffs-feasibility.md) only, **not verified on hardware** |

In one line: **both routes disturb a measurement** - self-signing moves the boot chain (PCR[4]/[7] side), FFS moves the firmware (PCR[1] side). Against a client that only reads the Secure Boot state bit, either can work; against a client that requires the stock boot chain or the stock firmware measurements, neither is guaranteed. Anti-cheat rules are vendor policy and change without notice; this document promises nothing either way.

### Evidence status

| Item | Evidence | Status |
| --- | --- | --- |
| Route B (MOK + shim) | #162 laptop report (unlock survives into Windows); **#210 full desktop run**: Secure Boot on, v2.1.10 + `.sbat` + self-signed MOK, whole chain green + `Confirm-SecureBootUEFI=True` + certificate in `MokListRT` + PK/KEK/db untouched | two machines; steps 3/4/7 hard-code #210's measured parameters, not re-run on this project's own hardware |
| Release lacks `.sbat` | this project checked the v2.1.10 artifact's section table; #210 confirmed on hardware that a missing section yields only `0x1A` from shim | verified (artifact level + one machine) |
| The `openssl` block in step 0 | executed here (OpenSSL 4.0.3) | verified |
| Route A (db self-sign) | second-hand statement, no logs, no board model | not reproduced, not executed |
| Secure Boot on + `ssdiag` | `tools/ssdiag/README.md`: expected to be refused; #210 declined to load it on that basis | not measured (#210 marks it not verified) |
| Secure Boot on + `diag-unlock.py` | none | not verified |
| Anti-cheat (Vanguard / ACE) | maintainer statement | not measured |
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

| Read-back | Verdict / action |
| --- | --- |
| SS0=`0x88888888`, SS1 low nibble=`0x8` | Overrides effective; verify performance with a matched workload |
| Ampere/Ada gate open, SS values wrong | Selector repair is possible; read back after repair |
| Turing gate open (`0xffffffff`), SS values correct | Default handoff shape (since 2026-11-08, same as Ampere/Ada); the in-OS write surface comes with it |
| Turing gate relocked (`0xffffffef`/`0xffffff8f`), SS values correct | Normal shape with `\\tx-relock.txt` present (value varies by board, bits 5:6 set, not the `0xffffff8f` cold default), or a platform/driver relock event - tell them apart with the canary, see [cold values](cold-values.md) |
| Gate closed and SS invalid, or GPU powered off | Repair cannot reopen the gate; follow recovery and replay |

Compare the OS result with `HANDOFF-OUT` in `loader.log`. For driver initialization failures, also inspect Device Manager, nvlddmkm/Xid and power state. EFI success does not replace OS verification.

For no gain after handoff, use these read-back patterns to guide diagnosis. Confirm the cause with power and driver logs:

| In-OS observation | Investigate |
| --- | --- |
| Ampere/Ada gate and selectors return to locked defaults | Power loss or full reset |
| Gate remains open, selectors return to stock | devinit rewriting overrides (see [relock mechanisms](#relock)) |
| Rewritten selectors with Code 43 or driver resets | Failed driver initialization and recovery |
| Effective selectors but low performance | Clocks, power wall and workload |

ssdiag discovers BAR0 from the OS and accepts no manually supplied physical base; it wakes parked GPUs as needed. Its embedded, verified `pmxdrv.sys` uses a temporary `SSDIAG_PMXDRV` service with cleanup on exit, or borrows an existing device such as nvoc's. Driver deployment normally needs administrator privileges. Its security model and trade-off have their own section: [The pmxdrv.sys security model and trade-off](#pmxdrv). See the [ssdiag reference](../tools/ssdiag/README.md) for the register surface and driver details.

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

`0xc000009a` is `STATUS_INSUFFICIENT_RESOURCES` at the winload stage (the Recovery screen with "press F8 / F1 / Esc" means it got that far). **It is not a shortage of memory, it is a shortage of the largest usable run**: ExitBootServices converts nothing to conventional, so our own asset cache crosses over as a `LOADER_DATA` descriptor and splits the only large run below 4 GiB - and winload takes its boot reservation out of a single descriptor without stitching. A 32 GB machine with 2.4 GiB still free fails all the same ([#146](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146), the only case so far).

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
