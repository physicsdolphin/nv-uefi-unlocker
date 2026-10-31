# nv-uefi-unlocker

[中文](README.md) | [English](README.en.md)

**A UEFI pre-driver NVIDIA GPU unlocker for the GTX 16 and RTX 20, 30, and 40 series.**

Unlock your NVIDIA GPU’s Tensor compute performance. Boot from a USB stick, unlock, and continue into Windows or Linux. No VBIOS flashing is needed; powering off clears the unlock.

[Watch the project trailer ↗](https://www.bilibili.com/video/BV1d6xN66ECM/) (Simplified Chinese)

## Compatibility at a glance

| Series | Support |
| --- | --- |
| GTX 16 · Turing | Theoretically compatible; field tests welcome |
| RTX 20 · Turing | Fully compatible; some models have known issues listed in the full matrix |
| RTX 30 · Ampere GA10x | Fully compatible |
| RTX 40 · Ada | Fully compatible on laptops and desktops |
| Pascal and older / GA100 / RTX 50 | Unsupported |

[Full compatibility matrix and field results](docs/compatibility.en.md) · [FAQ](docs/faq.en.md)

## Get started

### 1. Get the tools and check prerequisites

Download and extract a package from [Releases](https://github.com/physicsdolphin/nv-uefi-unlocker/releases). NVIDIA firmware is not bundled; the setup script downloads the specified versions, verifies them and generates the required files.

Prepare a FAT32 USB stick and back up its files. The target must boot in UEFI mode, with Secure Boot disabled or a self-signed EFI certificate trusted. On laptops, prefer dGPU-only display mode and choose **Restart** in the OS before entering the one-time boot menu.

**Turing users: run this once in an administrator PowerShell on the target Windows machine, then restart:**

```powershell
.\enable-turing-unlock.ps1
```

After restarting, confirm that GSP firmware is running before making the stick. Linux Turing targets also need GSP-RM; see the [GSP prerequisite](docs/faq.en.md#turing). Linux users should also check [RTD3](docs/faq.en.md#linux) to prevent power loss from clearing the unlock after handoff.

### 2. Prepare the bootable USB stick

Run from the extracted package directory. Select `40` for the 40 series, `30` for the 30 series, or `20` for the 20 / 16 series.

Windows PowerShell:

```powershell
$gpuSeries = "40"
.\make-usbroot.ps1 -Family $gpuSeries
```

Linux / WSL / Git Bash:

```bash
gpu_series=40
bash make-usbroot.sh --family "$gpu_series"
```

Windows requires PowerShell 5+; the Linux binary requires glibc ≥ 2.34 (for example, Ubuntu 22.04+ / Debian 12+). Initial generation needs internet access. Resolve download or verification failures without substituting another firmware version.

Files are generated in the local `usbroot/` directory. Copy the firmware files listed at the end of the script to the stick root, then put `BOOTX64.EFI` in `EFI/BOOT/`. The other extraction intermediates do not need to be copied.

```text
USB stick (FAT32)
├── EFI/BOOT/BOOTX64.EFI
└── Firmware files (.bin files listed by the script)
```

Keep the generated `v67-*.bin` unlock file. [Full file list and purposes](docs/faq.en.md#usb).

Back up an existing `EFI/BOOT/BOOTX64.EFI` before replacing it. Do not copy into the system ESP or change the permanent boot order. See [USB files and options](docs/faq.en.md#usb) for file details, building on another host, GPU selection and advanced options.

### 3. Boot and verify

1. Select the USB stick in the firmware’s one-time boot menu.
2. Press **Space or Enter** at the version screen. Any other key or 30 seconds without input aborts the transaction and returns to the OS bootloader. A temporary black screen or frozen frame on the target GPU is expected.
3. Wait for the automatic OS handoff. The log is `loader.log` at the stick root.
4. On Windows, run the release’s `ssdiag-x86_64-windows.exe` to check the in-OS state, then compare performance using the same workload. On Linux, run `sudo python3 tools/diag-unlock.py` from a source checkout. See [verification and repair](docs/faq.en.md#ssdiag). Replay after each cold boot.

On STOP, a fault or a stuck black screen, **do not retry in the same session**. Fully power off, remove the stick and boot normally; follow the [recovery instructions](docs/faq.en.md#recovery).

For agent-assisted setup, use the [deployment guide](AGENTS.deploy.md). Copy this request:

> Follow https://github.com/physicsdolphin/nv-uefi-unlocker/blob/main/AGENTS.deploy.md to install nv-uefi-unlocker and set it up to unlock my GPU.

## Build from source (optional)

Skip this section if you use a release package. Install Rust; choose the MSVC toolchain on Windows. From the repository root:

```text
rustup target add x86_64-unknown-uefi
cargo build --release
cargo efi-tx
```

The EFI program is generated at `target/x86_64-unknown-uefi/release/loader.efi`. Prepare the boot files with the following commands (40-series example; use `-Family 30` for the 30 series or `-Family 20` for the 20 / 16 series):

```powershell
.\release\make-usbroot.ps1 -Family 40 -AssetsExe .\target\release\assets.exe -BootEfi .\target\x86_64-unknown-uefi\release\loader.efi
```

```bash
bash release/make-usbroot.sh --family 40 \
  --assets ./target/release/assets \
  --boot-efi ./target/x86_64-unknown-uefi/release/loader.efi
```

Then copy to the USB stick using the layout above. [Read-only diagnosis, tests and development notes](docs/faq.en.md#layout).

## Combine with nvoc

This tool unlocks Tensor compute performance. [nvoc](https://github.com/Skyworks-Neo/nvoc) provides in-OS power, voltage and overclocking controls; the two can be combined.

1. Unlock with this tool, verify the in-OS state and record a baseline.
2. Follow the [nvoc documentation](https://github.com/Skyworks-Neo/nvoc#readme) for controls supported by your GPU. The kmd power-wall unlock works on mobile GPUs; its performance effects on 10/20-series desktop cards remain unverified. `set-volt-rail-limit` voltage-wall control applies to 10-series and newer GPUs.
3. Compare the same workload with [cli-stressor-cuda-rs](https://github.com/Skyworks-Neo/nvoc/tree/main/cli-stressor-cuda-rs), monitoring power, Hot Spot and throttle reasons.

Combined operation can substantially exceed stock power and thermal envelopes; check cooling and power-delivery headroom. [Combined test results and risks](docs/faq.en.md#nvoc-results).

## References and repositories

This project is a direct beneficiary and derivative of community work;
full credits in the same-named section of
[docs/ada28e0-uefi-report.md](docs/ada28e0-uefi-report.md). Core:

- [gpu-unlock](https://github.com/Zagitta/gpu-unlock) (GPL-2.0) — the
  transaction-chain porting source
- [gpu-unlock-ada](https://github.com/JustAResearcher/gpu-unlock-ada)
  (GPL-2.0) — the AD10x four-write chain origin; the 28e1 report
  (Windroid234) is this project's direct basis
- [cmp170hx](https://github.com/Consensus-Protocol/cmp170hx) — PLM
  semantics and verification methodology
- [cmp90hx-unlock](https://github.com/WildFlash1st/cmp90hx-unlock) — the
  V67 chain origin
- [AD103_win_unlocker](https://huggingface.co/TOTORONG/AD103_win_unlocker)
  (TOTORONG) — the Windows EFI carrier-craft precedent
- [open-gpu-kernel-modules](https://github.com/NVIDIA/open-gpu-kernel-modules)
  (MIT) — first-hand authority on the boot path

License: **GPL-2.0-only** (`LICENSE`). NVIDIA firmware and VBIOS remain
NVIDIA property; regenerate them locally from pinned public packages
under their own license terms — they are not distributed in this
repository.

- How it works (Simplified Chinese): [power-user edition](https://www.bilibili.com/opus/1264436370549506065) · [beginner edition](https://www.bilibili.com/opus/1264493051219279872)
- Community discussions: [Reddit](https://www.reddit.com/r/overclocking/comments/1xj5jp3/new_uefi_gpu_unlocking_tool_revealed_for_windows/) · [X](https://x.com/RoundtableSpace/status/2115749232980140458)

## Star History

<a href="https://www.star-history.com/?repos=physicsdolphin%2Fnv-uefi-unlocker&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&legend=top-left" />
 </picture>
</a>
