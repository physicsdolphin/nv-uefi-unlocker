# nv-uefi-unlocker

[中文](README.md) | [English](README.en.md)

**Your GeForce runs AI at half speed — this fixes it before the driver even loads.**

[![version](https://img.shields.io/badge/version-2.1.8-brightgreen)](https://github.com/physicsdolphin/nv-uefi-unlocker/releases/latest)
[![GeForce](https://img.shields.io/badge/GeForce-GTX%2016%20%7C%20RTX%2020%2F30%2F40-green)](docs/compatibility.en.md)
[![platform](https://img.shields.io/badge/platform-UEFI%20pre%20driver%20%7C%20Win%20%2F%20Linux%20%2F%20Hackintosh-blue)](docs/faq.en.md)
[![license](https://img.shields.io/badge/license-GPL--2.0-blue)](LICENSE)
[![stars](https://img.shields.io/github/stars/physicsdolphin/nv-uefi-unlocker?label=stars&color=yellow)](https://github.com/physicsdolphin/nv-uefi-unlocker/stargazers)

**Measured up to ~2× FP16 / BF16 / TF32 tensor throughput · +30–75% image / video generation · no VBIOS flashing, cleared by power-off** (per-board data in the [compatibility matrix](docs/compatibility.en.md); gains are bounded by power limits and precision paths)

A UEFI pre-driver NVIDIA GPU unlocker for the GTX 16 and RTX 20, 30, and 40 series.

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
full credits in [docs/credits.md](docs/credits.md) (verified against
each item's actual role in the repo history). The
repositories actually studied and cited during development are cloned
into a local, untracked `reference/` directory, grouped by role:

**Transaction-chain sources and first-hand authority**

- [gpu-unlock](https://github.com/Zagitta/gpu-unlock) (GPL-2.0) — the
  transaction-chain porting source; `unlock_610.c` read section by
  section, and the blueprint for this repo's mock tests
- [gpu-unlock-ada](https://github.com/JustAResearcher/gpu-unlock-ada)
  (GPL-2.0) — the AD10x four-write chain origin; the 28e1 report
  (Windroid234) is this project's direct basis and drove three verdict
  corrections here (FBPA `0xffffff0f` acceptance, non-fatal scrubber,
  EALREADY without FLR)
- [gpu-unlock-bar1](https://github.com/84634E1A607A/gpu-unlock-4090-48g-64g-bar1) —
  AD10x large-BAR1 reference line (not merged into the mainline)
- [cmp90hx-unlock](https://github.com/WildFlash1st/cmp90hx-unlock) — the
  V67 payload-chain origin
- [open-gpu-kernel-modules](https://github.com/NVIDIA/open-gpu-kernel-modules)
  (MIT) — first-hand authority on the boot path: all four Turing
  generational differences (BOOT_DIRECT, register-portal loading,
  RM_RISCV_UCODE_DESC v4, FWSEC generic-BL) were verified here and
  landed in the code
- [AD103_win_unlocker](https://huggingface.co/TOTORONG/AD103_win_unlocker)
  (TOTORONG) — the Windows EFI carrier-craft precedent (GOP
  disconnect/chain-load handoff/transaction-log conventions)

**Turing lineage and addressing (the 0x4096xx FEAT block)**

- [cmp170hx](https://github.com/Consensus-Protocol/cmp170hx) — PLM
  semantics, the booter 0x89/0x91 status-code dictionary and
  verification methodology
- [cmp50hx-unlock](https://github.com/xrip/cmp50hx-unlock) — the
  hardware-proven origin of the Turing lineage: 31.7× measured on 50HX
  (TU102); 40HX/50HX share the TU102 booter archive, which is the
  field evidence behind TU106 full-chain feasibility; also the source
  of the FEAT block `0x409650/64/6c` addressing
- [turing-unlock](https://github.com/nlzy/turing-unlock) (nlzy) — the
  official Turing adaptation of the Zagitta kmod (Dell 2080 Ti 22GB,
  ~1.6× FP16); source of the TU102 gadget/stack geometry tables and
  the GSP IDA database; the `TU_FEAT_READOUT 0x409668` citation source
  (ssdiag and the loader share this constant)
- [travis-mcdevitt/nv-uefi-unlocker](https://github.com/travis-mcdevitt/nv-uefi-unlocker) —
  a downstream fork of this project; its successful Turing chain logs
  feed the 0x91 forensics cross-check
- [tu102-22g-uefi-unlock](https://github.com/chenxluo/tu102-22g-uefi-unlock) —
  UEFI approach for TU102 22G modded cards (peer reference for the 22G
  support line)

**Prior Windows/UEFI work of the same kind**

- [rtx30-unlock](https://github.com/craftingmod/rtx30-unlock) — the
  Windows/UEFI port of the Zagitta kmod (GA10x predecessor)
- [rtx30-unlock-windows-toolkit](https://github.com/Siselea/rtx30-unlock-windows-toolkit) —
  companion toolkit for the above (single-GPU, single-display)
- [rtx3080-unlock-public](https://github.com/rough2515-hub/rtx3080-unlock-public) —
  earlier RTX 3080 10GB UEFI experiment (Korean)
- [KohakUnlock](https://github.com/KohakuBlueleaf/KohakUnlock) —
  source-first research covering the Turing lineage (boot-hook form)
  and a 4090 Windows/UEFI unlock
- [Turing-unlock](https://github.com/haohervchb/Turing-unlock) (haohervchb) —
  volatile unlock on a 2080 Ti (TU102), one successful physical run
  2026-10-22
- [nv_unlock_win](https://github.com/mirabarukaso/nv_unlock_win) —
  Titan RTX in-kernel Windows attempt (a different route to the same
  goal)
- [gpuunlock](https://github.com/miounet11/gpuunlock) — merged
  distribution for 4090 / 4070 Ti SUPER

**Register/fuse side lines**

- [ReBarUEFI](https://github.com/xCuri0/ReBarUEFI) (xCuri0, MIT) — the
  platform-side authority for ReBAR on RTX 30+ (enumeration-time hook +
  PCIe ReBAR extended capability) and the catalog of platform large-BAR
  limitation patches; [NvStrapsReBar](https://github.com/terminatorul/NvStrapsReBar)
  is its Turing fork — overturned this project's earlier "Turing BAR1 is
  not expandable" assumption; direct basis of the M1 sample PSTRAPS
  decoding
- [ga100-fuse-re](https://github.com/ahnguyen17/ga100-fuse-re) ·
  [gpcprobe](https://github.com/matstatman/gpcprobe) ·
  [170hx-rir-tool](https://github.com/lixiangwuxian/170hx-rir-tool) —
  GA100 fuse/RIR architecture (fuse-semantics reference line)
- [p104-gen2-unlock](https://github.com/skbskhud/p104-gen2-unlock) —
  P104 PCIe Gen2 link tool (PCI config-space operation reference,
  orthogonal to the compute unlock)

**Reverse-engineering toolchain**

- [envytools](https://github.com/envytools/envytools) — envydis (fuc5
  Falcon disassembler; the production booter secure segment is
  ciphertext and only decodes with the key in hand) + nvbios
- [faucon](https://github.com/vbe0201/faucon) — alternative fuc5
  disassembler (could not decode the production booter in practice)

License: **GPL-2.0-only** (`LICENSE`). NVIDIA firmware and VBIOS remain
NVIDIA property; regenerate them locally from pinned public packages
under their own license terms — they are not distributed in this
repository.

- How it works (Simplified Chinese): [power-user edition](https://www.bilibili.com/opus/1264436370549506065) · [beginner edition](https://www.bilibili.com/opus/1264493051219279872)
- Community discussions and coverage (language tagged per item):
  [Reddit](https://www.reddit.com/r/overclocking/comments/1xj5jp3/new_uefi_gpu_unlocking_tool_revealed_for_windows/) (EN) ·
  [X/RoundtableSpace](https://x.com/RoundtableSpace/status/2115749232980140458) (EN) ·
  [X/Pirat_Nation](https://x.com/Pirat_Nation/status/2116303886604739008) (EN) ·
  [TechPowerUp](https://www.techpowerup.com/354316/new-uefi-tool-lifts-nvidias-tensor-cap-on-rtx-20-to-40-series-cards-in-windows?type=Tools) (EN) ·
  [Wccftech](https://wccftech.com/new-nvidia-uefi-gpu-unlocker-brings-up-to-75-higher-ai-compute-performance-to-rtx-20-30-40-series-gpus-on-windows/) (EN) ·
  [NotebookCheck](https://www.notebookcheck.net/UEFI-level-pre-driver-Nvidia-GPU-unlocker-now-available-for-RTX-2xxx-3xxx-and-4xxx-GPUs-on-Windows-and-Linux.1438153.0.html) (EN) ·
  [Mezha Media](https://mezha.net/eng/news/mezha-2ffa1478104cdc175bc0e096_uefi_pre-driver_nvidia/) (EN) ·
  [Digg](https://digg.com/tech/vfyuouiy) (EN) ·
  [XenoSpectrum](https://xenospectrum.com/en/nvidia-uefi-windows-tensor-unlock/) (EN) ·
  [HotRepo](https://hotrepo.ai/physicsdolphin/nv-uefi-unlocker) (EN) ·
  [HardwareCooking](https://www.hardwarecooking.fr/nv-uefi-unlocker-tensor-cores-rtx-20-30-40-windows/) (FR) ·
  [DNS Club](https://club.dns-shop.ru/amp/digest/187732-instrument-nv-uefi-unlocker-povyishaet-ii-proizvoditelnost-kart/) (RU) ·
  [Playground.ru](https://www.playground.ru/misc/news/novaya_utilita_pozvolyaet_uvelichit_proizvoditelnost_videokart_nvidia_rtx_do_75_v_zadachah_ii-1886614) (RU) ·
  [FanaticosDelHardware](https://fanaticosdelhardware.com/nv-uefi-unlocker-multiplica-hasta-265-veces-el-rendimiento-tensor-de-las-geforce-rtx-en-windows/) (ES) ·
  [OMGPU](https://www.omgpu.com/nvidia-luefi-gpu-unlocker-dope-de-75-lia-des-rtx-20-a-40-sur-windows/) (FR) ·
  [PauseHardware](https://pausehardware.com/nvidia-tensor-rtx-debridees-sous-windows/) (FR) ·
  [HardwarePremium](https://www.hardwarepremium.com/noticias/78275/uefi-nvidia-desbloqueador-rtx-2000-3000-4000/) (ES) ·
  [Gaming-ST](https://gaming-st.com/gpu/nv-uefi-unlocker-windows-rtx20-30-40-tensor-unlock/) (JA) ·
  [Shahrsakhtafzar](https://www.shahrsakhtafzar.com/fa/news/graphic-card/62809-nvidia-geforce-rtx-tensor-unlock-windows-tool) (FA) ·
  [Quasar Zone](https://quasarzone.com/bbs/qn_hardware/views/2085634) (KO) ·
  [GiggleHD](https://gigglehd.com/gg/hard/19358883) (KO) ·
  [GECID](https://ua.gecid.com/news/novij_nstrument_podvoyu_produktivn_st_tenzornih_yader_dlya_v_deokart_rtx_20_40_u_windows/) (UK) ·
  [DicasPC](https://dicaspc.com.br/noticias/nv-uefi-unlocker-libere-ate-75-mais-ia-na-sua-rtx/) (PT-BR) ·
  [Diario da Midia](https://diariodamidia.com.br/autor/thiago-nogueira) (PT-BR) ·
  [GPU-Unlock.net](https://gpu-unlock.net/) (EN) ·
  [IT-SVR](https://it-svr.com/svrforum-news-3234984/) (KO)

## Star History

<a href="https://www.star-history.com/?repos=physicsdolphin%2Fnv-uefi-unlocker&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&legend=top-left" />
 </picture>
</a>
