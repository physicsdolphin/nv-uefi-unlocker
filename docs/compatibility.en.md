# Compatibility

[中文](compatibility.md) | [English](compatibility.en.md) · [FAQ](faq.en.md) · [README](../README.en.md)

This tool covers GTX 16 through RTX 40 series. The matrix separates family support from device-level evidence and lists known platform limitations.

[Series support](#series) · [Routing and evidence](#routing) · [Device matrix](#devices) · [Verified status and exceptions](#reports)

<a id="series"></a>

## Series support

| Series | Support | Prerequisite |
| --- | --- | --- |
| GTX 16 | Theoretically compatible; hardware verification pending | GSP-RM |
| RTX 20 | Fully compatible; some models have known issues | GSP-RM; restart after enabling it |
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
| AD106 | 4060Ti 2803 / 4060Ti 16G 2805 | ✅ 2803/2805 · [Details](#device-4) |
| AD106 | 4070 Laptop **2820** (same ID on modded cards) | 🟡 Unverified · [Details](#device-5) |
| AD104 | 4070Ti 2782 / 4070S 2783 / 4070 2786 | ✅ 2783; others retain upstream tier · [Details](#device-6) |
| AD103 | 4080S 2702 / 4080 2704 / 4070TiS 2705 / **4070 2709** / 4090M 2757 | ✅ 2704/2705/2709; 🟢 2702/2757 · [Details](#device-7) |
| AD102 | 4090 2684 / 4090D 2685 / 4070TiS **2689** | ✅ 2684/2689; 🟢 2685 · [Details](#device-8) |
| GA102 | 3090 2204 / **3080 2206** / **3080 LHR 2216** / 3080 Ti 2208 / 3070 Ti 2205 etc. (whole 0x22xx range routed) | ✅ 2206/2216; others fully routed · [Details](#device-9) |
| GA103M | 3080 Ti Laptop **2420** | ✅ 2420 · [Details](#device-10) |
| GA104 | 3070 2484–248c / 3060 Ti **2489** etc. (whole 0x24xx range routed) | ✅ 2489; 2484 EFI-only, lost in Windows · [Details](#device-11) |
| GA104M | 3070 Laptop **249d** | ✅ 249d; hybrid exceptions · [Details](#device-12) |
| GA106 (incl. M) | 3060 2501–2508 / 3060 Laptop 2560 etc. (whole 0x25xx range routed) | ✅ 2504/2520; 🟢 others; 2560 retest pending · [Details](#device-13) |
| GA107 | 3050 2507 / 3050 Laptop 25a2 etc. | 🟡 Unverified · [Details](#device-14) |
| **TU106** | **RTX 2070 1f07 / 2060 Super 1f06** | ✅ 1f07/1f06 · [Details](#device-15) |
| TU102/TU104/TU116 (rest of the 20/16 series) | 1e02–1fff, 2182–21d1 etc. | 🟢 Community chain; EFI handoff issues remain · [Details](#device-16) |
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

### AD106 · 4060Ti 2803 / 4060Ti 16G 2805

`2803` / `2805` pass full-chain and in-OS performance verification on Intel, AMD, single-GPU/no-iGPU and dual-GPU systems. Some AMI platforms require [skipping GOP reconnect](faq.en.md#reconnect). Sources: [2803 report](evidence/ad106-4060ti-desktop/report.md), [2805 report](evidence/ad106-2805-nuc13/report.md), [#51](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/51), [#33](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/33), [#56](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/56).

<a id="device-5"></a>

### AD106 · 4070 Laptop 2820 (same ID on modded cards)

Family routing and assets are available; EFI hardware verification is pending. Modded-card reports need the original model, VBIOS and full `loader.log`. Source: [#10](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/10).

<a id="device-6"></a>

### AD104 · 4070Ti 2782 / 4070S 2783 / 4070 2786

`2783` passes replay, unlock read-backs, FLR survival and Windows performance verification. `2782` / `2786` retain upstream silicon evidence. Source: [#3](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/3).

<a id="device-7"></a>

### AD103 · 4080S 2702 / 4080 2704 / 4070TiS 2705 / 4070 2709 / 4090M 2757

`2704` / `2705` / `2709` pass full-chain and in-OS performance verification. `2702` / `2757` have upstream silicon evidence. Sources: [#53](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/53), [#36](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/36), [#8](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/8).

<a id="device-8"></a>

### AD102 · 4090 2684 / 4090D 2685 / 4070TiS 2689

`2684` passes full-chain, Windows read-back and performance verification. Fixed-clock normalization supports the issue-rate unlock; boost-clock gains are power-limited, with a residual gap on some accumulation paths unresolved. `2689` has in-OS read-back and performance evidence, without an archived EFI log. `2685` has upstream silicon evidence. Sources: [#69](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/69), [community records](community-reports.md).

<a id="device-9"></a>

### GA102 · 3090 2204 / 3080 2206 / 3080 LHR 2216 / 3080 Ti 2208 / 3070 Ti 2205 etc. (whole 0x22xx range routed)

`2206` / `2216` pass firmware read-backs and Windows performance verification; other same-family devices are fully routed. Gains depend on power and workload. Sources: [#29](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/29), [#15](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/15).

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

### GA106 (incl. M) · 3060 2501–2508 / 3060 Laptop 2560 etc. (whole 0x25xx range routed)

`2504` / `2520` pass unlock and in-OS performance verification. The AMI X470 `2504` platform needs `no-reconnect.txt`. Other same-family devices retain upstream evidence; an in-OS success result for `2560` awaits retesting. Sources: [#70](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70), [#71](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/71), [#6](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/6).

<a id="device-14"></a>

### GA107 · 3050 2507 / 3050 Laptop 25a2 etc.

Family routing is available; complete hardware success evidence is pending. Source: [#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37).

<a id="device-15"></a>

### TU106 · RTX 2070 1f07 / 2060 Super 1f06

`1f07` / `1f06` pass full-chain and Windows verification. Windows requires GSP-RM and a restart. Handoff relocks FEAT PLM while preserving the override values; the unlock path skips FLR. For nonstandard VRAM capacities, check the usable-memory decode in the log. Sources: [TU106 report](evidence/m3-tu106/host-and-final-result.md), [#12](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/12), [GSP prerequisite](faq.en.md#turing).

<a id="device-16"></a>

### TU102/TU104/TU116 (rest of the 20/16 series) · 1e02–1fff, 2182–21d1 etc.

Same-family assets and community silicon evidence are available, but this tool has no complete successful EFI-to-Windows handoff evidence for these models. Known issues: 2080 Ti `1e07` and 2080 `1e87` return booter `0x91`; 2080 Super `1e81` unlocks in EFI but bug-checks during GSP handoff. Sources: [#17](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/17), [#62](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/62), [#55](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/55). GTX 16 hardware verification with this tool remains pending.

<a id="device-17"></a>

### Older than Turing (Pascal…) · —

Unsupported: no corresponding transaction path.

<a id="device-18"></a>

### Blackwell (50 series) / GA100 · —

Unsupported: a different boot chain from this tool’s transaction families.
