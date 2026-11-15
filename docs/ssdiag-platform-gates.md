# ssdiag 平台门与 MAP 被拒排查（issue #118 / B 站双例闭环）

> 2026-11-02。ssdiag 0.6.3+ 的平台门与 denied-map 前置分类清单经过两例实机闭环（#118 卡巴斯基/B650M、B 站 5060/Win10），本文是该机制的权威说明。
> 2026-11-15 增补（#244 轮）：HVCI 措辞勘误（§2）+ tier-2「secure PCI」0x1EA 整机蓝屏分层（§2a）+ 平台门两新行（§3）+ **§3 Secure Boot 行勘误**（旧行"ON=无法加载+工具先行拒绝"两处皆错，见该行）。

## 1. 两个实机案例（同签名、同根因、不同杀软形态）

| | #118（GitHub） | B 站第二例 |
|---|---|---|
| 卡 / 平台 | 3080 Ti 2208 / B650M AM5 | 5060 28E0 / Lenovo，Win10 |
| ssdiag 版本 | 0.4.8（旧发现层） | 0.6.4 |
| 症状 | `MAP_PHYS win32 error 5`，"17 MiB 跨区间"假候选 | 发现层干净（0.6.x CfgMgr32 修复生效），MAP 仍全拒 |
| 杀软 | Kaspersky（用户态暂停无效） | Kaspersky（用户态退出无效） |
| 判决手段 | （当时无） | **0.6.3 前置分类清单**：HVCI clear → 排除①；无 CodeIntegrity 3077（驱动未被拒）+ 7045（驱动已载）→ 排除③；卡巴 14 个内核驱动 Running → **定位第 2 层：内核态杀软钩子** |
| 痊愈动作 | **卸载**卡巴斯基 | **卸载**卡巴斯基 |
| 痊愈后真读 | — | `PLM OPEN / SS0=0x88888888 / READOUT_1=0x00000000`（全速），VERDICT UNLOCKED |

**两例共同教训（对用户最重要的一句）**：杀软的**暂停保护/退出用户态进程不卸载内核态过滤驱动**——AV 的内核钩子常驻，`MAP_PHYS` 照样被拒。判定内核层是否涉案，卸载（或临时移除其内核组件）是唯一充分测试。

## 2. MAP 被拒前置分类（0.6.3 起自动输出）

首个 `MAP_PHYS win32 error 5` 时输出一次，按可能性排序：

1. **内核态 AV/EDR 过滤驱动在 veto**——用户态杀软进程关闭**不**卸载它；验证 = 卸载杀软内核组件后重跑（#118/B 站双例均落在此层）；
2. **驱动实际未加载**——SCM 7045 只证明 SCM 层安装；查 System 日志 CodeIntegrity 3077/3089 是否拒载（Microsoft Vulnerable Driver Blocklist 名单核查：本 pinned pmxdrv 的 hash 经核对**不在** OS 内置与可下载名单中，2026-11）。

> **勘误（2026-11-15，#244 轮）**：原第 1 条「HVCI RUNNING 时 `MmMapIoSpace` 按设计拒绝」系 #118 时代的推断——当时两例 HVCI 均 clear，真凶是上层的杀软钩子，该说法从未被 RUNNING 态实证过。在役反证：**#188（VBS+HVCI on，SB off，26H2 26300.9818）/#204（HVCI ON，SB off）全量 probe（pmxdrv 载入 + ECAM 读）跑通**；#218（SB+HVCI+阻止列表全开）只跑过 `--inventory`（免驱动层，不构成 SB 下全量 probe 的证据——其解锁引导走路线 A 临时关 SB），勿当全量证据引用。HVCI 运行态保留在平台门里作背景位，不再当作拒因；真正的硬门在上一级 tier-2（§2a）。

## 2a. tier-2：VTL1 保护 MMIO（"secure PCI"）——0x1EA 整机蓝屏

形态：**不做 MAP（映射能成功），首次 ECAM 读直接整机蓝屏** `0x1EA SECURE_PCI_CONFIG_SPACE_ACCESS_VIOLATION`——[微软文档](https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/bug-check-0x1ea--secure-pci-config-space-access-violation)（2022-08 起）明文：secure PCI 启用时禁止 VTL0 直接映射 PCI MCFG 区间；强制点在 hypervisor/VTL1 的 EPT 层，比 error 5 严重一级（无恢复机会）。四参数：Arg1=故障指令 opcode（#244 实例 `0x8b`=MOV r32,r/m32）、Arg2=设备 RID、Arg3=config 偏移、Arg4=故障 RIP。首个在役样本 [#244](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/244)：ssdiag 0.6.8 POWER PMCSR 路径，pmxdrv 把 MCFG 映进用户态，用户态 `mov eax,[rax]` 触发（RID 0x5200=52:00.0、偏移 0）。

分层判据（**HVCI ≠ tier-2**）：

- HVCI/VBS 运行**本身不触发** 0x1EA——在役反证 #188/#204/#218（见 §2 勘误）；
- #244 机器 `Win32_DeviceGuard.SecurityServicesRunning=[2,3,4,5]`（HVCI + **System Guard Secure Launch**（DRTM）+ **SMM Firmware Measurement** + VBS Enclaves）→ 判别位 = **[3]/[4]**（WMI 或 msinfo32「基于虚拟化的安全性服务正在运行」）；
- 保护范围不止 MCFG：同族生态先例 = Speccy（Framework 社区）、NI-VISA nipcibrd.sys（Surface 5）、AMD Ryzen Master driver；我方自有观察：**2024+ 部分华硕 OEM 镜像（天选6 实测）上 Intel FPTW64 读 SPI-flash MMIO 同样直接蓝屏**（该例 bugcheck 码未验证，公开检索无独立报告）。

ssdiag 0.8.0 起的行为：`DeviceGuard\Scenarios\SystemGuard`（或 `SecureLaunch` 别名）`Enabled=1` **且** `DeviceGuard\VirtualizationBasedSecurityStatus==2`（内核开机回写）→ 部署前拒绝（含借用路径，无覆盖旗标——借用路径同走一条 ECAM 读）；`--inventory` 与 `--dry-run` 不受影响（不碰驱动）。检测是**注册表启发**：WMI [3]/[4] 才是权威运行态——若机器 tier-2 在跑而两个键都不在，属漏检样本，请按拒绝文案里的命令确认后反馈。

使能旋钮（公开，见 [System Guard 文档](https://learn.microsoft.com/en-us/windows/security/hardware-security/system-guard-secure-launch-and-smm-protection)）：`Scenarios\SystemGuard\Enabled=1` / GPO「Device Guard → 打开 VBS → Secure Launch Configuration」/ MDM `ConfigureSystemGuardLaunch` / 安全中心「内核隔离 → 固件保护」。Windows 侧**从不自动启用**这层（2026-10 微软公告的 HVCI 扩张波不含它）；2024+ 部分 OEM 在出厂镜像 image time provision（天选6 实测带、重装干净最新 Win11 后消失）。**推测待判**：OEM 具体写了哪个键、0x1EA 使能位是否恰为 SystemGuard 键，公开文档未写明——现有证据只有 #244 的 [2,3,4,5] 相关性，待一个天选6 在役样本采集钉死（配方见 FAQ 蓝屏节）。

## 3. 平台门行语义

| 行 | 来源 | 含义 |
|---|---|---|
| `vulnerable-driver blocklist` | registry `CI\Config\VulnerableDriverBlocklistEnable` | 配置态；本工具 hash 不在名单（已核对） |
| `core isolation (HVCI)` | registry `HVCI\Scenario\Enabled` | **配置态**——Win11 默认启用的机器常缺省（打印 unknown） |
| `HVCI runtime (CodeIntegrity)` | `NtQuerySystemInformation(103)` 0x10 位 | **运行态权威**；RUNNING=代码完整性门已升——HVCI 本身不拒 probe（在役反证见 §2 勘误），硬门看下两行 / CLEAR=无此层 / unknown=查询被拒（非判决） |
| `VBS status (kernel readback)` | registry `DeviceGuard\VirtualizationBasedSecurityStatus` | 内核开机回写（2=RUNNING）；=0 时 tier-2 无强制面 |
| `Secure Launch (SystemGuard)` | registry `Scenarios\SystemGuard`（±`SecureLaunch` 别名）`Enabled` | **配置态启发**；与 VBS RUNNING 同时在场 = tier-2 armed，probe 部署前拒绝（0x1EA，#244） |
| `Secure Boot` | registry `SecureBoot\State` | **配置态软门**（0.7.1 起措辞，2026-11-08 依在役数据软化）——ON 时 pinned pmxdrv **可能**拒载（#40 分析背景：Intel/COMODO 交叉签名、无 Microsoft 根、叶证书晚于 2015-07-29 切线，不符 DSE 豁免），但 SB-ON 全量 probe 载入运行已有在役实证（bailu974 记录：`28a0`/MEOW R15，警告照打、驱动实载、完整 MMIO 读数 + `VERDICT UNLOCKED`；机理未知，拒载仍是默认预期）——工具**只打黄字 "may not load, just try"，从不先行拒绝**；真拒载时查 CodeIntegrity 3077/3089，免驱动路径 = EFI 只读采样；0.8.0 起唯一的部署前硬拒是 tier-2（上行） |

## 4. 解锁判读的旁证增量（B 站例首证）

`READOUT_1` 在未解锁时 `0x00401080`（有 compute throttle 位段）、解锁后归零 `[full speed]`——这是"选择器覆盖实际生效到全速"的**直接读出**，与 VERDICT 的 SS 值比对互为独立证据。两行同时出现 = 判定完备。

## 5. 遗留边界

- `nvapi.dll not loadable` 在 0.6.4 前是 x64 加载名 bug（裸名只在 SysWOW64），GC6 wake 从不生效——0.6.4 起 `nvapi64.dll` 优先，parked 态卡才能真正拉出 D3 诊断；
- 非 0.6.4 用户遇到 MAP error 5 时，升级本身可能就是修复（0.4.8 发现层的 17 MiB 合并候选是另一类假阳性，#118）。
- 0.8.0 的 tier-2 检测 = 注册表键启发（配置态）+ VBS 内核回读（运行态）；WMI `SecurityServicesRunning` [3]/[4] 是权威运行态但需 COM，未进工具——「tier 在跑、键不在」的漏检形态待样本（拒绝文案带确认命令）。
