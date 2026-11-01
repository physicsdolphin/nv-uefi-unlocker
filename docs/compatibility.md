# 兼容性矩阵

[中文](compatibility.md) | [English](compatibility.en.md) · [FAQ](faq.md) · [README](../README.md)

本工具面向 GTX 16 至 RTX 40 系列显卡。矩阵分别列出系列支持、逐卡验证情况与已知平台限制。

[系列支持](#series) · [路由与证据等级](#routing) · [设备矩阵](#devices) · [验证结论与例外](#reports)

<a id="series"></a>

## 系列支持

| 系列 | 支持情况 | 前置条件 |
| --- | --- | --- |
| GTX 16 | 理论兼容，待实机验证 | GSP-RM |
| RTX 20 | 全系兼容；部分型号有已知问题 | GSP-RM，启用后必须重启 |
| RTX 30 GA10x | 全系兼容 | 无需 Turing 的 Windows 注册表前置 |
| RTX 40 | 全系兼容，支持笔记本与台式 | 入口时 GPU 有电且完成初始化 |
| Turing 之前 / GA100 / RTX 50 | 不支持 | — |

<a id="routing"></a>

## 路由与证据等级

本工具按芯片家族路由，同族设备共用 Booter/payload 资产。设备 ID 可进入事务路径，不代表该型号已经单独实测。

| 等级 | 含义 |
| --- | --- |
| ✅ | 本工具有实机证据；仅 EFI 成功或仅系统侧验证的情况单独注明 |
| 🟢 | 上游或社区有同链硅验证，不等于本工具逐卡验证 |
| 🟡 | 代码路径存在，完整实机验证待补充 |
| ❌ | 不支持 |

| 家族 | PCI device ID 范围 | 资产选择 |
| --- | --- | --- |
| Turing | `0x1e02..=0x1fff`、`0x2182..=0x21d1` | `tu10x` |
| Ampere GA10x | `0x2200..=0x22ff`、`0x2400..=0x25ff` | `ga102` |
| Ada | `0x2600..=0x28ff` | `ad102` |

支持家族中的移动、工作站、CMP 与魔改卡使用所属家族的路径。GA100（A100 `0x20b0` / `0x20c2`、A30 `0x20f1`）、Hopper 和 Blackwell 不在支持路径中。未单列的同族 ID 继承家族证据等级。

<a id="devices"></a>

## 设备矩阵

| 家族 | 型号 / device ID | 证据与例外 |
| --- | --- | --- |
| AD107 | 4060 Laptop **28e0** | ✅ 28e0 · [详情](#device-1) |
| AD107 | 4060 2882 / 4060 Laptop 28a0 / 4050M 28e1 | ✅ 28a0；🟢 2882/28e1 · [详情](#device-2) |
| AD107 | 4050 Laptop **28a1** | ✅ 28a1 · [详情](#device-3) |
| AD106 | 4060Ti 2803 / 4060Ti 16G 2805 | ✅ 2803/2805 · [详情](#device-4) |
| AD106 | 4070 Laptop **2820**（魔改卡同 ID） | 🟡 待验证 · [详情](#device-5) |
| AD104 | 4070Ti 2782 / 4070S 2783 / 4070 2786 | ✅ 2783；其余保留上游层级 · [详情](#device-6) |
| AD103 | 4080S 2702 / 4080 2704 / 4070TiS 2705 / **4070 2709** / **4090M 2757** | ✅ 2704/2705/2709/2757；🟢 2702 · [详情](#device-7) |
| AD102 | 4090 2684 / 4090D 2685 / 4070TiS **2689** | ✅ 2684/2689；🟢 2685 · [详情](#device-8) |
| GA102 | 3090 2204 / **3080 2206** / **3080 LHR 2216** / 3080 Ti 2208 / 3070 Ti 2205 等（0x22xx 全段放行） | ✅ 2206/2216；其余全段放行 · [详情](#device-9) |
| GA103M | 3080 Ti Laptop **2420** | ✅ 2420 · [详情](#device-10) |
| GA104 | 3070 2484–248c / 3060 Ti **2489** 等（0x24xx 全段放行） | ✅ 2489；2484 仅 EFI、Windows 不保留 · [详情](#device-11) |
| GA104M | 3070 Laptop **249d** | ✅ 249d；混合输出有例外 · [详情](#device-12) |
| GA106（含 M） | 3060 2501–2508 / 3060 Laptop 2560 等（0x25xx 全段放行） | ✅ 2504/2520；🟢 其余；2560 待复测 · [详情](#device-13) |
| GA107 | 3050 2507 / 3050 Laptop 25a2 等 | 🟡 待验证 · [详情](#device-14) |
| **TU106** | **RTX 2070 1f07 / 2060 Super 1f06** | ✅ 1f07/1f06 · [详情](#device-15) |
| TU102/TU104/TU116（20 系其余/16 系） | 1e02–1fff、2182–21d1 等 | 🟢 社区同链；EFI 交棒仍有问题 · [详情](#device-16) |
| Turing 之前（Pascal…） | — | ❌ 无事务轨 · [详情](#device-17) |
| Blackwell (50 系) / GA100 | — | ❌ 链路不同 · [详情](#device-18) |

<a id="reports"></a>

## 验证结论与例外

<a id="device-1"></a>

### AD107 · 4060 Laptop 28e0

本工具全链解锁与 Windows 验证通过。独立平台复现同样确认系统侧状态和 Tensor 性能提升。来源：[实测报告](ada28e0-uefi-report.md)、[#63](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/63)。

<a id="device-2"></a>

### AD107 · 4060 2882 / 4060 Laptop 28a0 / 4050M 28e1

`28a0` 全链与 Windows 性能验证通过，满载收益受 TGP 限制；`2882` / `28e1` 保留上游硅验证层级。来源：[#50](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/50)。

<a id="device-3"></a>

### AD107 · 4050 Laptop 28a1

全链、选择器读回、FLR 后存活及计算完整性检查通过。来源：[实测记录](evidence/m1-ad107-laptop/log.md)。

<a id="device-4"></a>

### AD106 · 4060Ti 2803 / 4060Ti 16G 2805

`2803` / `2805` 均通过全链与系统侧性能验证，覆盖 Intel、AMD、无内显单卡及双卡环境。部分 AMI 平台需要 [跳过 GOP 重连](faq.md#reconnect)。来源：[2803 报告](evidence/ad106-4060ti-desktop/report.md)、[2805 报告](evidence/ad106-2805-nuc13/report.md)、[#51](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/51)、[#33](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/33)、[#56](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/56)。

<a id="device-5"></a>

### AD106 · 4070 Laptop 2820（魔改卡同 ID）

家族路由和资产可用，EFI 实机结果待验证。魔改卡反馈需包含原卡型号、VBIOS 与完整 `loader.log`。来源：[#10](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/10)。

<a id="device-6"></a>

### AD104 · 4070Ti 2782 / 4070S 2783 / 4070 2786

`2783` 通过回放、解锁读回、FLR 存活和 Windows 性能验证；`2782` / `2786` 保留上游硅验证层级。来源：[#3](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/3)。

<a id="device-7"></a>

### AD103 · 4080S 2702 / 4080 2704 / 4070TiS 2705 / 4070 2709 / 4090M 2757

`2704` / `2705` / `2709` / `2757` 通过全链及系统侧性能验证；`2702` 为上游硅验证。`2757` 的 Alienware m18 R1 实测（Windows 11；FP16/BF16/FP8 约 2 倍，控制组持平）见 [evidence/uefi-2757-m18r1/report.md](evidence/uefi-2757-m18r1/report.md)。来源：[#53](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/53)、[#36](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/36)、[Discussion #72](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/72)、[#8](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/8)、[#110](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/110)、[gpu-unlock-ada #132](https://github.com/JustAResearcher/gpu-unlock-ada/issues/132)。

<a id="device-8"></a>

### AD102 · 4090 2684 / 4090D 2685 / 4070TiS 2689

`2684` 全链、Windows 读回与性能验证通过；同频归一化结果支持发射率解锁，boost 频率收益受功耗墙限制，部分累加路径的差距仍待解释。`2689` 有系统侧读回及性能证据，EFI 日志未归档；`2685` 为上游硅验证。来源：[#69](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/69)、[社区记录](community-reports.md)。

<a id="device-9"></a>

### GA102 · 3090 2204 / 3080 2206 / 3080 LHR 2216 / 3080 Ti 2208 / 3070 Ti 2205 等（0x22xx 全段放行）

`2206` / `2216` 通过固件读回与 Windows 性能验证；其余同族设备全段放行。实际收益受功耗和工作负载影响。来源：[#29](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/29)、[#15](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/15)。

<a id="device-10"></a>

### GA103M · 3080 Ti Laptop 2420

独显直连下通过 PLM 翻转、选择器读回、FLR 存活和 Windows 性能验证。来源：[#27](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/27)、[平台注意事项](community-reports.md)。

<a id="device-11"></a>

### GA104 · 3070 2484–248c / 3060 Ti 2489 等（0x24xx 全段放行）

`2489` 在 AMD 平台通过全链及 Windows 验证，跨厂 VBIOS 环境也可用。`2484` 的雷电 eGPU 环境通过 EFI 事务，但 Windows 不保留解锁；雷电不作为保证生效的环境。其余同族设备全段放行。来源：[#11](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/11)、[#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)、[雷电限制](faq.md#thunderbolt)。

<a id="device-12"></a>

### GA104M · 3070 Laptop 249d

独显直连下通过全链及 Windows 性能验证；无 MUX 的混合输出平台存在进入系统后无收益的反馈，需核对系统侧读回。来源：[#28](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/28)、[#6](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/6)。

<a id="device-13"></a>

### GA106（含 M） · 3060 2501–2508 / 3060 Laptop 2560 等（0x25xx 全段放行）

`2504` / `2520` 通过解锁和系统侧性能验证；`2504` 的 AMI X470 平台需用 `no-reconnect.txt` 跳过重连。其他同族设备保留上游证据层级；`2560` 的系统侧成功结果待复测。来源：[#70](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70)、[#71](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/71)、[#6](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/6)。

<a id="device-14"></a>

### GA107 · 3050 2507 / 3050 Laptop 25a2 等

家族路由存在，完整实机成功证据待补充。来源：[#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37)。

<a id="device-15"></a>

### TU106 · RTX 2070 1f07 / 2060 Super 1f06

`1f07` / `1f06` 通过全链与 Windows 验证。Windows 必须启用 GSP-RM 并重启；交棒时 FEAT PLM 重锁、解锁值保留，解锁路径跳过 FLR。非标准显存容量应核对日志中的可用显存解码结果。来源：[TU106 报告](evidence/m3-tu106/host-and-final-result.md)、[#12](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/12)、[GSP 前置](faq.md#turing)。

<a id="device-16"></a>

### TU102/TU104/TU116（20 系其余/16 系） · 1e02–1fff、2182–21d1 等

同族资产可用，有社区同链硅验证，但本工具尚无完整 EFI 至 Windows 成功交棒证据。已知问题：2080 Ti `1e07`、2080 `1e87` 返回 booter `0x91`；2080 Super `1e81` 在 EFI 解锁后 GSP 交棒蓝屏。来源：[#17](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/17)、[#62](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/62)、[#55](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/55)。GTX 16 系仍待本工具实机验证。

<a id="device-17"></a>

### Turing 之前（Pascal…） · —

没有对应事务路径，不支持。

<a id="device-18"></a>

### Blackwell (50 系) / GA100 · —

引导链与本工具的事务家族不同，不支持。
