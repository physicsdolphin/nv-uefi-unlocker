# Compatibility

[中文](compatibility.md) | [English](compatibility.en.md) · [FAQ](faq.en.md) · [README](../README.en.md)

This tool covers GTX 16 through RTX 40 series. The matrix separates family support from device-level evidence and lists known platform limitations.

[Series support](#series) · [Routing and evidence](#routing) · [Device matrix](#devices) · [Verified status and exceptions](#reports)

<a id="series"></a>

## Series support

| Series | Support | Prerequisite |
| --- | --- | --- |
| GTX 16 | Theoretically compatible; hardware verification pending | GSP-RM |
| RTX 20 | Fully compatible; the 0x91 family is root-caused and fixed, recent-driver reset under investigation | GSP-RM; restart after enabling it |
| RTX 30 GA10x | Fully compatible | No Turing Windows registry prerequisite |
| RTX 40 | Fully compatible on laptops and desktops | GPU powered and initialized at entry |
| Pre-Turing / GA100 / RTX 50 | Unsupported | — |

<a id="routing"></a>

## Routing and evidence

Routing uses chip families. Same-family IDs share Booter/payload assets; a routed ID does not imply an individual hardware test.

| Tier | Meaning |
| --- | --- |
| ✅ | This tool has hardware evidence; EFI-only or OS-side-only results are marked explicitly |
| 🟢 | Upstream or community silicon evidence for the same chain; not individual EFI verification |
| 🟡 | Code path exists; complete hardware verification pending |
| ❌ | Unsupported |

| Family | PCI device-ID ranges | Assets |
| --- | --- | --- |
| Turing | `0x1e02..=0x1fff`, `0x2182..=0x21d1` | `tu10x` |
| Ampere GA10x | `0x2200..=0x22ff`, `0x2400..=0x25ff` | `ga102` |
| Ada | `0x2600..=0x28ff` | `ad102` |

Mobile, workstation, CMP and modded devices within a supported family use that family’s route. GA100 (A100 `0x20b0` / `0x20c2`, A30 `0x20f1`), Hopper and Blackwell have no supported route. Unlisted same-family IDs inherit the family evidence tier.

<a id="devices"></a>

## Device matrix

| Family | Cards / device IDs | Evidence and exceptions |
| --- | --- | --- |
| AD107 | 4060 Laptop **28e0** | ✅ 28e0 · [Details](#device-1) |
| AD107 | 4060 2882 / 4060 Laptop 28a0 / 4050M 28e1 | ✅ 28a0; 🟢 2882/28e1 · [Details](#device-2) |
| AD107 | 4050 Laptop **28a1** | ✅ 28a1 · [Details](#device-3) |
| AD106 | 4060Ti 2803 / 4060Ti 16G 2805 / 4060 **2808** | ✅ 2803/2805; 🟢 2808 · [Details](#device-4) |
| AD106 | 4070 Laptop **2820** (same ID on modded cards) / **2860** | 🟡 2820; 🟢 2860 · [Details](#device-5) |
| AD104 | 4070Ti 2782 / 4070S 2783 / 4070 2786 / 4060Ti **2788** (AD104 refresh) / 4080 Laptop **27a0 / 27e0**; ES: AC AD104 20GB **2785** | ✅ 2783; others retain upstream tier · [Details](#device-6) |
| AD103 | 4080S 2702 / 4080 2704 / 4070TiS 2705 / **4070 2709** / 4090M **2757 / 2717** | ✅ 2702/2704/2705/2709/2757; 🟢 2717 · [Details](#device-7) |
| AD102 | 4090 2684 / 4090D 2685 / 4070TiS **2689**; ES: TITAN Ada **2681** / PG137 **26af** | ✅ 2684/2689; 🟢 2685/2681/26af · [Details](#device-8) |
| GA102 | 3090 Ti 2203 / 3090 2204 / **3080 2206** / 3080 12GB **220a** / **3080 LHR 2216** / 3080 Ti 2208 / CMP 90HX **220d**; ES: 3090 **222b** / 3080 **222f** / 3080 Ti 20GB **2205** / 3070 Ti **2207** (0x2200..=0x223f routed) | ✅ 2206/2216; others fully routed · [Details](#device-9) |
| GA103M | 3080 Ti Laptop **2420** | ✅ 2420 · [Details](#device-10) |
| GA104 | 3070 Ti **2482** / 3070 2484–248c / 3060 Ti **2489** / 3060 **24c7** etc. (whole 0x24xx range routed) | ✅ 2489; 2484 EFI-only, lost in Windows · [Details](#device-11) |
| GA104M | 3070 Laptop **249d** | ✅ 249d; hybrid exceptions · [Details](#device-12) |
| GA106 (incl. M) | 3060 2501–2509 / 3050 8GB **2507** / 3060 Laptop 2560 etc. (whole 0x25xx range routed) | ✅ 2504/2520; 🟢 others; 2560 retest pending · [Details](#device-13) |
| GA107 | 3050 8GB **2582** / 3050 4GB 2583 / 3050 6GB 2584 / RTX 2050 **25a9** / MX570 25a6 / 3050 Laptop **25a2** etc. | 🟡 Unverified · [Details](#device-14) |
| **TU106** | **RTX 2070 1f07 / 2060 Super 1f06 / 2060 12GB 1f03 / 2070 Laptop 1f14** | ✅ 1f07/1f06/1f03/1f14 · [Details](#device-15) |
| TU102/TU104/TU116 (rest of the 20/16 series, incl. TITAN RTX 1e02) | 2080 Ti 1e07 / 2080 Super 1e81 / 2080 1e87 etc. | ✅ 1e07 (factory 11GB)/1e81/1e87; TU116/GTX 16 still 🟢 · [Details](#device-16) |
| Older than Turing (Pascal…) | — | ❌ No transaction track · [Details](#device-17) |
| Blackwell (50 series) / GA100 | — | ❌ Different chain · [Details](#device-18) |

<a id="reports"></a>

## Verified status and exceptions

<a id="device-1"></a>

### AD107 · 4060 Laptop 28e0

Full-chain unlock and Windows verification pass, including an independent platform reproduction of in-OS state and Tensor gains. Sources: [report](ada28e0-uefi-report.md), [#63](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/63).

<a id="device-2"></a>

### AD107 · 4060 2882 / 4060 Laptop 28a0 / 4050M 28e1

`28a0` passes full-chain and Windows performance verification; sustained gains are TGP-limited. `2882` / `28e1` retain upstream silicon evidence. Source: [#50](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/50).

<a id="device-3"></a>

### AD107 · 4050 Laptop 28a1

Full chain, selector read-backs, post-FLR survival and computation integrity checks pass. Source: [field record](evidence/m1-ad107-laptop/log.md).

<a id="device-4"></a>

### AD106 · 4060Ti 2803 / 4060Ti 16G 2805 / 4060 2808

`2803` / `2805` pass full-chain and in-OS performance verification on Intel, AMD, single-GPU/no-iGPU and dual-GPU systems. Some AMI platforms require [skipping GOP reconnect](faq.en.md#reconnect). `2808` is the original AD106-die 4060 variant (the AD107-die version is `2882`); upstream silicon tier. Sources: [2803 report](evidence/ad106-4060ti-desktop/report.md), [2805 report](evidence/ad106-2805-nuc13/report.md), [#51](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/51), [#33](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/33), [#56](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/56).

<a id="device-5"></a>

### AD106 · 4070 Laptop 2820 / 2860

`2820` is the official RTX 4070 Laptop ID (reused by modded cards): family routing and assets are available; EFI hardware verification is pending. Modded-card reports need the original model, VBIOS and full `loader.log`. Source: [#10](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/10).

`2860` is the other official ID of the same card, double-sourced against the open-gpu-kernel-modules compatible-GPUs table and pci.ids (2026-11-02); the runtime whitelist span `0x2600..=0x28ff` already covers it, no change needed to run. The reporter, [JerryHelper](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/101), volunteered a first-run `loader.log` - once it lands the row upgrades to the measured tier.

<a id="device-6"></a>

### AD104 · 4070Ti 2782 / 4070S 2783 / 4070 2786 / 4060Ti 2788 / 4080 Laptop 27a0 / 27e0

`2783` passes replay, unlock read-backs, FLR survival and Windows performance verification. `2782` (4070 Ti, PNY, H670) has a first real-machine log: the v2.1.8 transaction path is present, but replay mode hits a Windows-side black screen ([#105](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/105), triple-monitor dGPU-direct, under triage) — tier pending re-test. The others retain upstream silicon evidence. `2788` is the AD104-die refresh variant of the 4060 Ti; `27a0` / `27e0` are the two RTX 4080 Laptop IDs (triple-sourced, 2026-11-02). Source: [#3](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/3).

<a id="device-7"></a>

### AD103 · 4080S 2702 / 4080 2704 / 4070TiS 2705 / 4070 2709 / 4090M 2757 / 2717

`2702` / `2704` / `2705` / `2709` / `2757` pass full-chain and in-OS performance verification; `2717` has upstream silicon evidence. `2702` (4080 SUPER) was verified in [#107](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/107): official v2.1.8, CSM off — GOP recovery, Windows-side verification and tensor throughput all pass (the reporter also contributed a CSM-on display-recovery method that volatile-injects the unmodified native GOP from the card's own Option ROM, now tracked in issue #107). `2717` is the other RTX 4090 Laptop ID (same card as `2757`; triple-sourced, 2026-11-02). The 2757 Alienware m18 R1 result (Windows 11; FP16/BF16/FP8 ~2x, controls flat) is in [evidence/uefi-2757-m18r1/report.md](evidence/uefi-2757-m18r1/report.md). Sources: [#53](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/53), [#36](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/36), [Discussion #72](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/72), [#8](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/8), [#110](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/110), [gpu-unlock-ada #132](https://github.com/JustAResearcher/gpu-unlock-ada/issues/132).

<a id="device-8"></a>

### AD102 · 4090 2684 / 4090D 2685 / 4070TiS 2689

`2684` passes full-chain, Windows read-back and performance verification. Fixed-clock normalization supports the issue-rate unlock; boost-clock gains are power-limited, with a residual gap on some accumulation paths unresolved. `2689` has in-OS read-back and performance evidence, without an archived EFI log. `2685` has upstream silicon evidence. ES/prototypes: `2681` (RTX TITAN Ada) and `26af` (PG137 board) are pci.ids-listed AD102 silicon inside the runtime span; tier 🟢. Sources: [#69](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/69), [community records](community-reports.md).

<a id="device-9"></a>

### GA102 · 3090 Ti 2203 / 3090 2204 / 3080 2206 / 3080 12GB 220a / 3080 LHR 2216 / 3080 Ti 2208 / CMP 90HX 220d etc. (0x2200..=0x223f routed)

`2206` / `2216` pass firmware read-backs and Windows performance verification; other same-family devices are fully routed. Gains depend on power and workload.

Platform notes (2026-11-03): `2204` (3090) on ASUS X570 hangs at ConnectController after a successful transaction ([#106](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/106); `tx-reconnect-nonrecursive.txt` does not help, the `no-reconnect.txt` path awaits a follow-up log — same shape as the #52/#35/#61 reconnect-hang family; the CSM-compat remedy is tracked in #107). A 3080 20GB modded card on Z390P shows GSP devinit resetting SS0/SS1 back to factory ([#109](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/109); 30-series cards must **not** enable the Turing GSP registry switch — retest with it off; the report also adds Z390P/AMI to the platform list).

2026-11-02 pci.ids correction: the old "3070 Ti 2205" entry was wrong - the shipping 3070 Ti is GA104 `2482`, while `2205` / `2207` are cancelled GA102 ES SKUs (RTX 3080 Ti 20GB / a GA102 3070 Ti), now listed explicitly as ES above. The runtime span is narrowed to `0x2200..=0x223f` accordingly (the old 0x22..ff tail holds no GA102 cores - only HD-audio functions, Tegra/THOR/GB10 ARM blocks and H100 NVSwitch). ES/cancelled SKUs (`222b` 3090 ES, `222f` 3080 11/12GB ES, `2205`, `2207`) are GA102 cores inside the span and run as-is for anyone holding one; tier 🟢 (upstream silicon) until a first hardware report. Sources: [#29](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/29), [#15](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/15).

<a id="device-10"></a>

### GA103M · 3080 Ti Laptop 2420

dGPU-direct mode passes PLM flip, selector read-backs, FLR survival and Windows performance verification. Sources: [#27](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/27), [platform notes](community-reports.md).

<a id="device-11"></a>

### GA104 · 3070 2484–248c / 3060 Ti 2489 etc. (whole 0x24xx range routed)

`2489` passes full-chain and Windows verification on AMD, including a cross-vendor VBIOS. A Thunderbolt eGPU `2484` passes EFI execution but loses the unlock in Windows; Thunderbolt persistence is not guaranteed. Other same-family devices are fully routed. Sources: [#11](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/11), [#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54), [Thunderbolt limits](faq.en.md#thunderbolt).

<a id="device-12"></a>

### GA104M · 3070 Laptop 249d

dGPU-direct mode passes full-chain and Windows performance verification. MUX-less hybrid platforms have reports of no in-OS gain; check OS-side read-backs. Sources: [#28](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/28), [#6](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/6).

<a id="device-13"></a>

### GA106 (incl. M) · 3060 2501–2509 / 3050 8GB 2507 / 3060 Laptop 2560 etc. (whole 0x25xx range routed)

`2504` / `2520` pass unlock and in-OS performance verification. The AMI X470 `2504` platform needs `no-reconnect.txt`. Other same-family devices retain upstream evidence; an in-OS success result for `2560` awaits retesting. Sources: [#70](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70), [#71](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/71), [#6](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/6).

<a id="device-14"></a>

### GA107 · 3050 2582 / 2583 / 2584 / RTX 2050 25a9 / MX570 25a6 / 3050 Laptop 25a2 etc.

The old "3050 2507" entry belonged to the wrong die: `2507` is the GA106-150-die 3050 8GB (moved to the GA106 row); the GA107-die desktop 3050s are `2582` / `2583` / `2584`, and RTX 2050 is `25a9` (also 25ad/25ed) - pci.ids + DeviceHunt verified (2026-11-02).

The family route exists. `25a2` (3050 Laptop 4GB, HP Victus) has a first real-machine log: v2.1.0 reported a wait_plm timeout, but log analysis concluded the **payload most likely executed and the FEAT PLM gate did open** — a known entry-race issue (the payload flips the gate faster than the entry read, [#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37); 4GB VRAM is unrelated). The reporter can re-tier by doing the Windows-side read-back/write-up steps in the issue; the fix is scheduled there.

Family routing is available; complete hardware success evidence is pending. Source: [#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37).

<a id="device-15"></a>

### TU106 · RTX 2070 1f07 / 2060 Super 1f06 / 2060 12GB 1f03 / 2070 Laptop 1f14

`1f07` / `1f06` / `1f03` / `1f14` pass full-chain and in-OS verification. `1f03` (2060 12GB) failed the FWSEC stage on v2.1.6 with an EXCI error; six controlled experiment rounds in [issue #89](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/89) pinned it to a 4 GiB-boundary staging DMA reachability problem (same shape as #74), and it now runs with the **FWSEC VRAM staging + BridgeMaster guard fixes** (2026-11-01). `1f14` (2070 Laptop, barebone CNH5L, dGPU-direct) is a 0x91-family fix verification (v2.1.3+, [#16](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/16)). Windows requires GSP-RM and a restart. Handoff relocks FEAT PLM while preserving the override values; the unlock path skips FLR. For nonstandard VRAM capacities, check the usable-memory decode in the log. **Open**: with drivers 621.01 (GSP 620.67.01) and 630.20 (GSP 630.03), Windows-side `nvlddmkm` PF FLR resets wipe the unlock back to factory ([#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99); two datapoints, 1f06 and 1f07; under investigation). Sources: [TU106 report](evidence/m3-tu106/host-and-final-result.md), [#12](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/12), [#89](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/89), [#16](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/16), [GSP prerequisite](faq.en.md#turing).

<a id="device-16"></a>

### TU102/TU104/TU116 (rest of the 20/16 series) · 1e02–1fff, 2182–21d1 etc.

**The 0x91 family is root-caused and fixed**: the Turing booter enforces a sub-4GB read lock on meta/WPR staging (v61 booter behavior; independently corroborated by cmp50hx v2.63 comments, "из <4ГБ чтение блокировано (exit 0x91)"). Deterministic >4G allocation landed in v2.1.2, and v2.1.8 made the GSP-HALT gate + low staging defaults converge it. Verified: `1e81` (2080 Super, v2.1.3, 46→68 TFLOPS, [#14](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/14)), `1e87` (2080, v2.1.8, [#62](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/62)), `1e07` (**factory 11GB** 2080 Ti, AMD platform, unlock survives into Linux with the proprietary driver, 60.31→89.57 TFLOPS, [#103](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/103) — the first full-chain TU102 record on a non-22GB-modded card). Still on file: `1e04` 22G HANDOFF-OUT state parsing ([#102](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/102)); dual 1e07 NVLink — transactions succeed on both cards but Windows reports Code 12 on both ([#116](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/116)); TITAN RTX 0x0B/0x11 on Linux ([#57](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/57)); pre-v2.1.8 TB4 laptops failing FWSEC/Booter ([#74](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/74), passes with a separate C implementation's low staging). GTX 16 hardware verification with this tool remains pending.

No-marketing-name bare-die / internal boards (pci.ids-listed, inside the runtime span, tier 🟢 until a first report): TU102GL `1e3c`/`1e3d`/`1e3e`; TU104M `1eab`, TU104GL `1eb9`/`1eba` (PG189 SKU600)/`1ebe`; TU106 `1f04`, TU106M `1f2e`; TU117 `1f81`, TU117M `1fa1`, TU117GL `1fae`/`1fbf`; TU116GL `21ae`/`21bf`, bare TU116 `21c2`/`21c3`. The Turing range has no GA102-style "Engineering Sample" named entries (pci.ids full-span scan, 2026-11-02).

<a id="device-17"></a>

### Older than Turing (Pascal…) · —

Unsupported: no corresponding transaction path.

<a id="device-18"></a>

### Blackwell (50 series) / GA100 · —

Unsupported: a different boot chain from this tool’s transaction families.
