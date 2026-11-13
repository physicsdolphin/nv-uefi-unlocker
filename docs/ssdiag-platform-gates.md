# ssdiag 平台门与 MAP 被拒排查（issue #118 / B 站双例闭环）

> 2026-11-02。ssdiag 0.6.3+ 的平台门与 denied-map 前置分类清单经过两例实机闭环（#118 卡巴斯基/B650M、B 站 5060/Win10），本文是该机制的权威说明。

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

1. **HVCI 运行中**——运行态经 `SystemCodeIntegrityInformation`（0x10 位）判定；RUNNING 时 `MmMapIoSpace` 按设计拒绝（关闭路径：Windows 安全中心 → 设备安全性 → 内核隔离）；
2. **内核态 AV/EDR 过滤驱动在 veto**——用户态杀软进程关闭**不**卸载它；验证 = 卸载杀软内核组件后重跑（#118/B 站双例均落在此层）；
3. **驱动实际未加载**——SCM 7045 只证明 SCM 层安装；查 System 日志 CodeIntegrity 3077/3089 是否拒载（Microsoft Vulnerable Driver Blocklist 名单核查：本 pinned pmxdrv 的 hash 经核对**不在** OS 内置与可下载名单中，2026-11）。

## 3. 平台门行语义

| 行 | 来源 | 含义 |
|---|---|---|
| `vulnerable-driver blocklist` | registry `CI\Config\VulnerableDriverBlocklistEnable` | 配置态；本工具 hash 不在名单（已核对） |
| `core isolation (HVCI)` | registry `HVCI\Scenario\Enabled` | **配置态**——Win11 默认启用的机器常缺省（打印 unknown） |
| `HVCI runtime (CodeIntegrity)` | `NtQuerySystemInformation(103)` 0x10 位 | **运行态权威**；CLEAR=映射不拦 / RUNNING=MmMapIoSpace 拒 / unknown=查询被拒（非判决） |
| `Secure Boot` | registry `SecureBoot\State` | ON 时 pinned pmxdrv 无法加载（DSE 交叉签名无豁免），工具会先行拒绝 |

## 4. 解锁判读的旁证增量（B 站例首证）

`READOUT_1` 在未解锁时 `0x00401080`（有 compute throttle 位段）、解锁后归零 `[full speed]`——这是"选择器覆盖实际生效到全速"的**直接读出**，与 VERDICT 的 SS 值比对互为独立证据。两行同时出现 = 判定完备。

## 5. 遗留边界

- `nvapi.dll not loadable` 在 0.6.4 前是 x64 加载名 bug（裸名只在 SysWOW64），GC6 wake 从不生效——0.6.4 起 `nvapi64.dll` 优先，parked 态卡才能真正拉出 D3 诊断；
- 非 0.6.4 用户遇到 MAP error 5 时，升级本身可能就是修复（0.4.8 发现层的 17 MiB 合并候选是另一类假阳性，#118）。
