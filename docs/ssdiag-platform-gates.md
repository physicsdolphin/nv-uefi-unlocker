# ssdiag 平台门与 MAP 被拒排查（issue #118 / B 站双例闭环）

> 2026-11-02。ssdiag 0.6.3+ 的平台门与 denied-map 前置分类清单经过两例实机闭环（#118 卡巴斯基/B650M、B 站 5060/Win10），本文是该机制的权威说明。
> 2026-11-15 增补（#244 轮）：HVCI 措辞勘误（§2）+ tier-2「secure PCI」0x1EA 整机蓝屏分层（§2a）+ 平台门两新行（§3）+ **§3 Secure Boot 行勘误**（旧行"ON=无法加载+工具先行拒绝"两处皆错，见该行）。
> 2026-11-15 第二轮查证更正：tier-2 的**文档化触发 = SDEV ACPI 表 + VBS running**（§2a 重写）；此前"SystemGuard 键 = 使能旋钮"无公开文档支持，**降级为保守二级信号**；ssdiag 0.8.1 同步改判据。
> 2026-11-15 第三轮（#244 ASSESSMENT 在役闭环）：`DeviceGuard\VirtualizationBasedSecurityStatus` 注册表值在 VBS-running 机器上**可能缺席**（T14s 实测），0.8.0/0.8.1 的 `Some(2)` 正判据在该机上静默失效——0.8.2 改 **fail-closed**（SDEV 在场 + VBS 不可验证 → 拒）。

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

形态：**不做 MAP（映射能成功），首次 ECAM 读直接整机蓝屏** `0x1EA SECURE_PCI_CONFIG_SPACE_ACCESS_VIOLATION`——[微软文档](https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/bug-check-0x1ea--secure-pci-config-space-access-violation)（2022-08 起）明文：secure PCI 启用时禁止 VTL0 直接映射 PCI MCFG 区间；强制点在 hypervisor/VTL1 层，比 error 5 严重一级（无恢复机会）。四参数：Arg1=故障指令 opcode（#244 实例 `0x8b`=MOV r32,r/m32）、Arg2=设备 RID、Arg3=config 偏移、Arg4=故障 RIP。首个在役样本 [#244](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/244)：ssdiag 0.6.8 POWER PMCSR 路径，pmxdrv 把 MCFG 映进用户态，用户态 `mov eax,[rax]` 触发（RID 0x5200=52:00.0、偏移 0）。

**文档化触发条件（2026-11-15 第二轮查证更正，此前 SystemGuard 键说作废）**：SDEV 本身是 **ACPI 规范 6.2（2017-05，UEFI Forum）引入的标准表**（§5.2.26 "Secure Devices (SDEV) Table"；微软为 VBS/secured-core/ESS 安全 I/O 路线推动入册，IBV 按 OEM 配置实现，非 BIOS 厂商私有）；[MS KB4568831 ThinkPad 排障页](https://learn.microsoft.com/en-us/troubleshoot/windows-client/performance/stop-error-lenovo-thinkpad-kb4568831-uefi)明文——**自 Windows 10 2004（19041.423，2020-07-31）起，SDEV 表存在且 VBS running 时**，非受支持的 config space 访问被封堵并产生 Stop error，**"parsing MCFG table and mapping configuration space to virtual memory"被点名**；[ESS 文档](https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/windows-hello-enhanced-sign-in-security)给出执行方：**SDEV 表由 Secure Kernel（VTL1）解析并强制**，不支持方式的读写"result in bug checks"；[CHIPSEC Windows 安装文档](https://chipsec.github.io/installation/InstallWindows.html)同一条件公式（并因此随附 PCI filter driver 走受支持路径）。两条文档合读还推出强制形态：**MCFG 映射路径被整体封**——SDEV 列的是安全设备（ESS 摄像头等），而 #244 读的是 GPU config，照样炸。SystemGuard/Secure Launch 与此**无公开文档关联**（[System Guard 文档](https://learn.microsoft.com/en-us/windows/security/hardware-security/system-guard-secure-launch-and-smm-protection)只讲 Secure Launch 配置/验证，不提 PCI config space）；#244 的 [2,3,4,5] 更像 secured 风格机器四件套**共现**（推断，非文档结论），[3]/[4] 降为背景判别位。

分层判据（**HVCI ≠ tier-2**）：

- HVCI/VBS 运行**本身不触发** 0x1EA——在役反证 #188/#204（全量 probe，均 SB off；#218 只跑过 `--inventory` 免驱动层，不作全量证据，见 §2 勘误）；
- 触发对 = **SDEV 表在场 + VBS running**；公开案例都是普通装机、非 OEM 镜像专属：Framework+Speccy、LG Gram+Speccy（cpuz149_x64.sys，CCleaner 论坛）、AMD Ryzen Master driver、Vanguard vgk——0x1EA 发生 ⇒ 按排障页条件反推，这些机器固件必带 SDEV；
- 我方自有观察：**2024+ 部分华硕 OEM 镜像（天选6 实测）上 Intel FPTW64 读 SPI-flash MMIO 同样直接蓝屏**——SDEV 限制是否覆盖 MCFG 之外的 MMIO（SPI 区），公开文档未写明（该例 bugcheck 码未验证）。

ssdiag 0.8.2 起的行为（0.8.1 的注册表正判据被 #244 在役闭环推翻后的 fail-closed 重写）：SDEV 表在场时按 VBS 读取结果分支——`==2`（RUNNING）或 SystemGuard 键在场 → 拒绝；**注册表值缺席（unverified）→ 也拒绝（fail-closed）**：#244 的 T14s 实测 WMI 报 2 而该注册表值**根本不存在**，0.8.0/0.8.1 都要求 `Some(2)` 正判据，在那台机器上闸门静默失效——"值缺席"不能当"VBS 关"读；仅 `==0`（off）/`==1`（configured-not-running）放行（无 hypervisor 即无强制面）。拒绝含借用路径、无覆盖旗标；`--inventory` 与 `--dry-run` 不受影响。SDEV 探测是只读固件表查询，本身不会触发蓝屏。**在役发现三条（#244 ASSESSMENT，2026-11-15）**：① 该注册表值在 26300 VBS-running 机器上缺席（见上）；② `GetSystemFirmwareTable` sizing 调用（buffer=NULL）返回所需大小的同时 LastError 可能残留 122——是 sizing 语义不是读取失败；③ 该机 `SystemCodeIntegrityInformation` 报 HVCI clear 而 WMI `SecurityServicesRunning` 含 2（HVCI running）——两接口结论相反，机理待查（fail-closed 门不依赖 HVCI 位，不阻塞）。

**为什么"重装后消失"（更正：SDEV 在固件里，重装不动它）**：消失必然出自 VBS 侧或固件选项侧，按可查性排序：① 重装后 VBS 实际没跑（**WMI `VirtualizationBasedSecurityStatus==2` 或 msinfo32 为准——注册表值在部分 VBS-running 机器上缺席，不能当反证**）；② BIOS ESS 类选项被重置/刷 BIOS 改动（关闭该选项即撤 SDEV 限制，排障页明示 workaround）；③ OS 侧未公开前置（ESS provisioning/生物识别 trustlet 等——纯假设）；④ OEM 镜像与重装 build 不同。待天选6 双态采样判别（配方见 FAQ 蓝屏节）。

## 3. 平台门行语义

| 行 | 来源 | 含义 |
|---|---|---|
| `vulnerable-driver blocklist` | registry `CI\Config\VulnerableDriverBlocklistEnable` | 配置态；本工具 hash 不在名单（已核对） |
| `core isolation (HVCI)` | registry `HVCI\Scenario\Enabled` | **配置态**——Win11 默认启用的机器常缺省（打印 unknown） |
| `HVCI runtime (CodeIntegrity)` | `NtQuerySystemInformation(103)` 0x10 位 | **运行态权威**；RUNNING=代码完整性门已升——HVCI 本身不拒 probe（在役反证见 §2 勘误），硬门看下两行 / CLEAR=无此层 / unknown=查询被拒（非判决） |
| `VBS status (kernel readback)` | registry `DeviceGuard\VirtualizationBasedSecurityStatus` | 2=RUNNING / 1=configured-not-running / 0=off；**缺席≠off**（#244 T14s 实测 WMI 报 2 而值不存在）——unverified + SDEV 在场 = fail-closed 拒绝 |
| `Secure Launch (SystemGuard)` | registry `Scenarios\SystemGuard`（±`SecureLaunch` 别名）`Enabled` | **配置态二级信号（保守保留）**——与 secure PCI 无公开文档关联，同栈机器常齐；仅 VBS==2 正读时参与 armed（SDEV 缺席+VBS 未知不拒，否则误伤普通机器） |
| `SDEV table (ACPI)` | `GetSystemFirmwareTable('ACPI','SDEV')` | **文档化触发的固件半**（secure kernel 解析强制，0x1EA）；在场 + VBS==2（或 VBS 不可验证）= tier-2 armed，probe 部署前拒绝（#244）；关闭 = BIOS ESS 类选项 |
| `Secure Boot` | registry `SecureBoot\State` | **配置态软门**（0.7.1 起措辞，2026-11-08 依在役数据软化）——ON 时 pinned pmxdrv **可能**拒载（#40 分析背景：Intel/COMODO 交叉签名、无 Microsoft 根、叶证书晚于 2015-07-29 切线，不符 DSE 豁免），但 SB-ON 全量 probe 载入运行已有在役实证（bailu974 记录：`28a0`/MEOW R15，警告照打、驱动实载、完整 MMIO 读数 + `VERDICT UNLOCKED`；机理未知，拒载仍是默认预期）——工具**只打黄字 "may not load, just try"，从不先行拒绝**；真拒载时查 CodeIntegrity 3077/3089，免驱动路径 = EFI 只读采样；0.8.0 起唯一的部署前硬拒是 tier-2（上行） |

## 4. 解锁判读的旁证增量（B 站例首证）

`READOUT_1` 在未解锁时 `0x00401080`（有 compute throttle 位段）、解锁后归零 `[full speed]`——这是"选择器覆盖实际生效到全速"的**直接读出**，与 VERDICT 的 SS 值比对互为独立证据。两行同时出现 = 判定完备。

## 5. 遗留边界

- `nvapi.dll not loadable` 在 0.6.4 前是 x64 加载名 bug（裸名只在 SysWOW64），GC6 wake 从不生效——0.6.4 起 `nvapi64.dll` 优先，parked 态卡才能真正拉出 D3 诊断；
- 非 0.6.4 用户遇到 MAP error 5 时，升级本身可能就是修复（0.4.8 发现层的 17 MiB 合并候选是另一类假阳性，#118）。
- 0.8.2 的 tier-2 判据 = **SDEV 表**（文档化触发的固件半，`GetSystemFirmwareTable` 只读探测）按 VBS 读取结果分支：`==2` 或 SystemGuard 键在场 → 拒；**unverified（注册表值缺席）+ SDEV 在场 → 拒（fail-closed，#244 ASSESSMENT 闭环）**；`==0/==1` → 放行。WMI `SecurityServicesRunning` [3]/[4] 是背景判别位；[2]（HVCI running）与 `SystemCodeIntegrityInformation` 0x10 位在该机结论相反，两接口差异待查。残余漏检形态 = 「tier 在跑 + SDEV 探测阴性」——SDEV 探测签名校验失败会报 unknown 而非阴性，只有 `need==0` 才判 absent；待样本（拒绝文案带确认命令）。
- **tier-2 判据目前无故障注入测试**（`secure_pci_armed()` 直连 registry/固件表调用，`#[cfg(test)]` 无覆盖）：0.8.1 的静默失效正属"否定路径从未被观察"一类，矩阵（SDEV 在场/注册表缺席/SystemGuard 组合）+ 解析分支 + 拒绝文案断言已立 [#265](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/265) 跟踪，落地前本门的行为证据仍以在役样本为准。
