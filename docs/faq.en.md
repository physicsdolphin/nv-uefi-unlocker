# FAQ

[中文](faq.md) | [English](faq.en.md) · [README](../README.en.md) · [Compatibility](compatibility.en.md)

- [What files and options belong on the USB stick?](#usb)
- [Why does Turing need GSP firmware mode?](#turing)
- [Which display and boot-entry modes work?](#display)
- [Why is there no gain on Linux despite a green log?](#linux)
- [Does the unlock survive a Thunderbolt handoff?](#thunderbolt)
- [How do I check or repair the unlock in the OS?](#ssdiag)
- [What does booter status 0x89 mean?](#bridge-bme)
- [Why does the platform reset after a successful transaction?](#reconnect)
- [How do I recover from a black screen, STOP or driver failure?](#recovery)
- [What are the safety rules and implementation limits?](#safety)
- [How does the unlock work?](#mechanism)
- [What should I know about combining this tool with nvoc?](#nvoc-results)
- [How do I report an incompatibility or failure?](#reporting)
- [Where are the code and research records?](#layout)

<a id="usb"></a>

## What files and options belong on the USB stick?

The maker emits a flat local `usbroot/` directory. Copy the selected family's files listed at completion, and create `EFI/BOOT/` for the executable. Do not copy extraction intermediates as runtime assets.

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
| `chain-target.txt` | First nonempty, non-`#` line specifies an EFI path tried on each volume before the default order; missing path is logged and falls back |
| `no-reconnect.txt` | Skip target GOP reconnect for a post-transaction reconnect hang/reset |
| `tx-native-gop.txt` (tx builds) | CSM machines stuck at `RECONNECT: ConnectController begin` (#107 class): starts the native GOP explicitly before the reconnect instead of the hanging generic one; falls back to bootmgfw on any failure. See docs/csm-compat.md |
| `i_know_what_i_am_doing.txt` | Auto-run after 5 seconds, any key aborts; existence enables it, contents ignored |

Without the auto-run marker, only Space or Enter confirms; another key or a 30-second timeout aborts. Chainload order: Windows Boot Manager → supported Linux shim/grub/systemd-boot/Limine → generic `EFI/BOOT/BOOTX64.EFI`. The last tier offers a 5-second key-to-exit window.

Multi-GPU default: process all supported cards in firmware order; skip unsupported/already-unlocked cards, handle a card's failure without retrying it. Pin the BDF when only one card is intended.

For preparation on a different host, use `-SkipGspCheck` / `--skip-gsp-check`; target GSP prerequisites still apply. Offline Turing archive `gsp_rm_boot_dbg.bin` selection uses `-ProvenBlob` / `--proven-blob`; length/hash validation remains mandatory. Do not substitute versions or bypass failed checks. For Windows PowerShell TLS download failures, the bash maker can run under WSL / Git Bash.

<a id="turing"></a>

## Why does Turing need GSP firmware mode?

Turing's Windows driver needs GSP-RM mode to accept the unlocked state. Legacy initialization can cause TDR / Code 43. In administrator PowerShell, run the release script:

```powershell
.\enable-turing-unlock.ps1
```

In a checkout, use `tools\windows\enable-turing-unlock.ps1`. It sets `EnableGpuFirmware=1` only for Turing display devices; `-Remove` reverses it. **Restart is required**; hibernate resume does not reinitialize the driver.

The maker checks both the registry switch and running GSP firmware in `nvidia-smi -q`; the latter must not be N/A. Linux Turing also needs GSP-RM. Proprietary drivers 545+ use it by default, but `NVreg_EnableGpuFirmware=0` can disable it; verify runtime state without the Windows registry script. The 30 / 40 series do not require this Windows registry prerequisite.

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

An unlock cleared by power loss needs USB replay; changing configuration alone does not restore it. Source: [RTD3 platform record](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/9).

<a id="thunderbolt"></a>

## Does the unlock survive a Thunderbolt handoff?

Persistence is not guaranteed. Native PCIe slots and dGPU-direct configurations are the supported baseline. Thunderbolt tunnel rebuilding, hot reset / PERST# or enclosure power management can reset or power-cycle the GPU after handoff, clearing the unlock. Pure link retraining does not itself reset the endpoint.

Start with an [in-OS read-only check](#ssdiag):

- Ampere/Ada FEAT gate open, SS values reverted: `--reapply` can restore the overrides and verify them.
- Gate locked, device powered off or registers inaccessible: `--reapply` cannot open the gate or recover power-loss state; follow recovery and USB replay.
- SS values correct but throughput low: check power, clocks and the test method.

Turing intentionally relocks PLM at handoff; do not apply the Ampere/Ada closed-gate interpretation to it. Source: [Thunderbolt platform record](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54).

<a id="ssdiag"></a>

## How do I check or repair the unlock in the OS?

On Windows, use the release's `ssdiag-x86_64-windows.exe`, abbreviated below as `ssdiag`:

```text
ssdiag
ssdiag --select 2504
ssdiag --reapply
```

Default mode is read-only; `--select` filters by DeviceID. `--reapply` writes and verifies SS0/SS1 only behind an open gate; it does not write PLM. Turing remains read-only. On Linux, run `sudo python3 tools/diag-unlock.py` from the checkout root; `--bdf` selects an actual target address, and `--reapply` is also available.

| Read-back | Verdict / action |
| --- | --- |
| SS0=`0x88888888`, SS1 low nibble=`0x8` | Overrides effective; verify performance with a matched workload |
| Ampere/Ada gate open, SS values wrong | Selector repair is possible; read back after repair |
| Turing gate relocked, SS values correct | Normal handoff state |
| Gate closed and SS invalid, or GPU powered off | Repair cannot reopen the gate; follow recovery and replay |

Compare the OS result with `HANDOFF-OUT` in `loader.log`. For driver initialization failures, also inspect Device Manager, nvlddmkm/Xid and power state. EFI success does not replace OS verification.

For no gain after handoff, use these read-back patterns to guide diagnosis. Confirm the cause with power and driver logs:

| In-OS observation | Investigate |
| --- | --- |
| Ampere/Ada gate and selectors return to locked defaults | Power loss or full reset |
| Gate remains open, selectors return to stock | devinit rewriting overrides |
| Rewritten selectors with Code 43 or driver resets | Failed driver initialization and recovery |
| Effective selectors but low performance | Clocks, power wall and workload |

ssdiag discovers BAR0 from the OS and accepts no manually supplied physical base; it wakes parked GPUs as needed. Its embedded, verified `pmxdrv.sys` uses a temporary `SSDIAG_PMXDRV` service with cleanup on exit, or borrows an existing device such as nvoc's. Driver deployment normally needs administrator privileges. See the [ssdiag reference](../tools/ssdiag/README.md) for the register surface and driver details.

<a id="bridge-bme"></a>

## What does booter status 0x89 mean?

`0x89` is a Booter WPR-descriptor validation error. If a GPU's upstream PCI bridge has Bus Master disabled, device DMA cannot read WprMeta correctly and can produce this error.

The loader enables Bus Master where needed on the upstream bridge path, verifies it and restores it on completion. Logs contain `DMA-PATH` / `BRIDGE-BME`. Preserve the complete log and inspect those lines and GPU state; do not diagnose from the status alone or change BIOS settings blindly.

SVM, VT-x and VT-d need not be disabled on verified platforms. Task Manager's virtualization status is not the PCI Command Bus Master bit. See the [bridge BME report](bridge-bme-root-cause.md) for mechanism and evidence.

<a id="reconnect"></a>

## Why does the platform reset after a successful transaction?

Some AMI firmware hangs or resets during GOP reconnect after a successful GPU transaction. The loader uses nonrecursive vendor reconnect when a pathless GOP survives detach, skips NVIDIA display functions during pre-chainload reconnect, and writes logs during execution.

If the log stops at `RECONNECT: ConnectController ... begin`, or the platform resets there, fully power off and recover before adding `no-reconnect.txt` to the stick root to skip target GOP reconnect. Windows Boot Manager can establish its own console; skipping reconnect does not waive OS verification.

For Turing `0x91` or a GSP-handoff bug-check after EFI success, see the [compatibility matrix](compatibility.en.md#device-16). Not every black screen is a GOP problem. Evidence: [AMI report](ad106-asus-z790-windows-unlock.md), [X470 record](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70).

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
| Turing | FEAT at `0x4096xx`; relock PLM after selector writes, skip FLR on the unlock path |

Ada FBPA read-back `0xffffff0f` is an accepted observation, not a standalone failure. A scrubber handoff/timeout warning is not the final execution verdict either: check subsequent FWSEC, family-specific execution, selectors and handoff results.

The stock signature is restored before handoff. The unlock does not flash the VBIOS or persist through power loss. Mechanism and evidence: [Ada report](ada28e0-uefi-report.md), [Turing report](evidence/m3-tu106/host-and-final-result.md).

<a id="nvoc-results"></a>

## What should I know about combining this tool with nvoc?

This tool removes Tensor issue-rate limits; nvoc supplies in-OS power, voltage and overclocking controls. The kmd power-wall unlock works on mobile GPUs; 10/20-series desktop cards have driver read-back evidence but unverified performance effects. `set-volt-rail-limit` voltage-wall control applies to 10-series and newer GPUs.

Verify this tool independently, then select controls and tests using the [nvoc documentation](https://github.com/Skyworks-Neo/nvoc). After issue-rate unlocking, gains may be limited by power, clocks, cooling, memory access and workload precision. A microbenchmark multiplier does not apply to every application.

Combined operation can exceed factory power and thermal envelopes. Monitor Board Power, Hot Spot, throttle reasons and power delivery, and check cooling/supply headroom. Extreme test voltages and power are not recommended settings for other cards; the user assumes physical-damage risk from excess power, voltage or temperature. Measurements and conditions: [combined test report](ada28e0-uefi-report.md). Throughput tool: [cli-stressor-cuda-rs](https://github.com/Skyworks-Neo/nvoc/tree/main/cli-stressor-cuda-rs).

<a id="reporting"></a>

## How do I report an incompatibility or failure?

Submit the full `loader.log`, GPU model/PCI ID, platform, driver, display mode, entry path and in-OS diagnosis to [Issues](https://github.com/physicsdolphin/nv-uefi-unlocker/issues). Modded cards need the original model, VBIOS and VRAM capacity. Redact machine-specific details as needed. For performance issues, include matched dimensions, precision, power and before/after results.

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
