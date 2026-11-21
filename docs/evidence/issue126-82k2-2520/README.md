# #126 闭环（2026-11-20）：82K2 / `2520` MUXless 的双层回锁归因与驱动级 RTD3 键

## 实验问题

[#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126)：联想 IdeaPad Gaming 3 82K2（RTX 3060 Laptop `10de:2520`，GA106M）在 EFI 事务全绿后，进 Windows 仍回锁，且出现在驱动 bind 前/绑定期。首报时只能确认「EFI 写进去了、系统内没了」，无法区分是平台断电、驱动主动复位，还是两者叠加。本轮要回答四件事：

1. 抹掉状态的是哪一层（平台轨 / 驱动 reset / 写入）？
2. Windows 侧有没有任何用户态/策略级杠杆能留住解锁？
3. 驱动级 RTD3 键（`EnableRuntimePowerManagement=0`，[#266](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/266) 的处方）在这台「已定性为驱动 bind 期复位」的机器上是否仍然有效？
4. 若有效，真实工作负载是否同向？

## 配置

- 卡：RTX 3060 Laptop 12 GB，GA106M `10DE:2520`（联想 IdeaPad Gaming 3 82K2）
- 平台：AMD（根口 `00:01.1` = `1022:1633`）；**无 MUX，内屏硬接 iGPU，dGPU 永久无显示输出**；未见 ACPI D3 固件开关，欺骗器物理不可行
- 驱动：591.86 → **621.01**（Run 6 起）
- 载具：v2.1.11-diag1（canary 轮）及后续同链；`ssdiag` 0.6.8 / 0.7.0
- 进入方式：冷启 → F12 → U 盘事务 → 链回 bootmgfw；每轮基准在进桌面约 60 s 内跑完

## 运行矩阵

| 轮 | 变量（累积） | 结果 |
|---|---|---|
| 首报（两次独立引导） | 无 | EFI 全绿（`SS0=0x88888888`/`SS1=0x8`、POST-FLR 存活、交棒正常）；系统内 `PLM=0xffffff8f`、`SS0=0x26302042`、`--reapply` REFUSED；GPU-awake 探针与 D3 泊车探针均 `NOT_UNLOCKED` |
| Run 3 | + 端点 `D3ColdSupported=0`/`D3ColdEnabled=0` + NVCP 最高性能 + ASPM AC=0 | 仍回锁；ssdiag 仍 `device D3 / MEM decode off / driver unbound`；60 s 内基准仍锁定 |
| Run 4 | + v2.1.11-diag1 canary | **金丝雀 `cleared`**；ssdiag 0.6.8：根口 `00:01.1` `PMCSR=D3` → 平台 `_PR3/_OFF` 路径已执行、PERST# + 槽电撤除 |
| Run 5 | + 根口类实例 `PnPCapabilities=24` + 根口 D3Cold 退出项 | **根口 `D0`、轨保持**，金丝雀**仍 `cleared`** → 第二条凶手：**驱动 bind 期主动复位**（GC6-exit / 内部 FLR）；卡停泊在端点 D3(hot) |
| Run 6 | + 驱动 591.86 → **621.01** | 签名不变：根口 `D0`、轨完整、金丝雀 `cleared`、`NOT_UNLOCKED` → 驱动代际不是判别变量 |
| **Run 7** | + `EnableRuntimePowerManagement=0`（`tools/disable-rtd3.ps1`，驱动键 `{4d36e968...}\0001`） | **卡 `PMCSR=D0`、根口 `D0`、金丝雀 `intact`、`SS0=0x88888888`/`SS1=0x8` 活、`UNLOCKED`**；GEMM FP16 +54% / BF16 +57% / FP32 对照平；SDXL −30% |

## Run 7 原始读数（ssdiag 0.7.0）

```text
POWER: card PMCSR = D0 (0x0008)                          <- was D3 in runs 1-6
POWER: root port 00:01.1 VEN_1022&DEV_1633 PMCSR = D0 -> rail intact
CANARY_A/B (loader-written) = 0x0164c0de/0x0165cafe -> intact
SS0 = 0x88888888 / SS1 = 0x00000008 (live)
VERDICT [ampere]: UNLOCKED (override values live)
CANARY: intact + SS live - no reset, nothing rewrote the state after handoff
```

EFI 侧 Run 7 日志同前：canary armed、`POST-GOP ... CANARY=intact`、`HANDOFF-OUT ... ss0_unlock=true ss1_unlock=true plm_open=true`。

## 性能

GEMM（8192³ dense，FP32 累加，关闭 reduced-precision reduction，15 次取中位；同机 591.86/621.01 锁定态为对照）：

| 精度 | 锁定态（runs 1–6） | Run 7 | 变化 |
|---|---:|---:|---:|
| FP16 | 26.3–26.7 TFLOPS | **40.83** | **+54%** |
| BF16 | 26.6–26.8 | **42.20** | **+57%** |
| FP32（对照） | 8.58–8.62 | 8.60 | 平 |

SDXL（Forge f2.0.1，obsessionIllustrious_vPredV11，832×1216，25 步，Euler a，CFG 1.8，seed 12345，暖态，API 计时）：

| 状态 | 单张 |
|---|---:|
| 锁定基线（驱动 591.86） | 25.3 s |
| **RTD3 键 + 解锁（621.01）** | **17.2–18.4 s（3 次）** |

满载中点 73 °C / ~50 W / 1770 MHz（节流点 87 °C）。

## 归因

82K2 有**两层叠加的凶手**：

1. **平台 `_PR3/_OFF` 断轨**——Run 4 金丝雀 `cleared` + 根口 `D3`。根口类实例 `PnPCapabilities=24` + 根口 D3Cold 退出项可把这条路径压住（Run 5 根口 `D0`）。
2. **驱动 bind 期主动复位**——轨保持住之后金丝雀仍 `cleared`（Run 5），且跨驱动代际不变（Run 6）。在电源未断的前提下，唯一能清掉金丝雀的候选是驱动在 bind/devinit 发起的复位（GC6-exit / 内部 FLR）。

Run 7 只加 `EnableRuntimePowerManagement=0`（在 Run 6 注册表栈之上）：驱动不再发起运行时 D3 转换 → 第二层的 park/rewind 不发生 → 第一层的 `_OFF` 也不再被走到，金丝雀与选择器全部保留。**早期「驱动未绑定，因此写入方不在驱动」的读法被本轮更正**：探测时驱动未绑定，不等于 bind 窗口里驱动没参与；Run 5 的「轨在、态仍丢」正是驱动参与的判据。

## 边界

- Run 7 未记录撤掉前几轮的注册表开关，应按 **Run 6 + 键**读：键是 Run 6→7 的唯一变量，但不是从原厂态起的单变量 A/B。
- 睡眠/S4 仍按设计清解锁；本机接受「卡不再进入停泊、待机功耗上升」的代价。
- 键挂在驱动键下，驱动升级/重装可能重建键，升级后须复查重设（本机 Run 6 升级后已重设一次，但未做「升级后不设键」的对照）。
- 单机 N=1；「首个 `2520`」指该键的首个 `2520` 数据点，不表示同族普遍可复现。
- 无同会话（同 boot）键开/关键对照；Run 6/7 之间隔着一次重启 + 重放。

## 判读规则（可复用）

- **金丝雀 + 根口 PMCSR 是分层的两把尺**：`cleared` + 根口 `D3` = 断轨；`cleared` + 根口 `D0` = 复位型（驱动 bind / FLR 类）；`intact` + SS 活 = 什么都没发生。
- **「系统内看到驱动未绑定」是事后快照**，不能据此排除驱动在 bind 窗口的行为；要判驱动是否参与，看「轨保持后金丝雀是否仍被清」。
- 回锁后 `--reapply` REFUSED 是预期形态，门关后主机写无路；只能重放 EFI 事务。
- 混合/MUXless 机器先试驱动级 RTD3 键（免固件、可脚本撤销），再考虑固件 ACPI D3 开关或独显直连。

## 来源

- 首报与证据包：[#126 comment 6576715319](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126#issuecomment-6576715319)
- D3Cold 退出项轮（Run 3）：[#126 comment 6587801805](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126#issuecomment-6587801805)
- Canary / 断轨归因（Run 4）：[#126 comment 6628437674](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126#issuecomment-6628437674)
- 根口保持 / 驱动复位归因（Run 5）：[#126 comment 6628644467](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126#issuecomment-6628644467)
- 驱动代际反证（Run 6）：[#126 comment 6629135806](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126#issuecomment-6629135806)
- 处方发布：[#126 comment 7019531329](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126#issuecomment-7019531329)（#266 的键）
- **Run 7 闭环**：[#126 comment 7029535572](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126#issuecomment-7029535572)
- SDXL 跟进：[#126 comment 7029591947](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126#issuecomment-7029591947)
- 机制与文档：FAQ [`#relock-winrtd3`](../../faq.md#relock-winrtd3)、[`#relock`](../../faq.md#relock)；兼容性 [GA106 节](../../compatibility.md#device-13)；社区记录 [community-reports](../../community-reports.md)
