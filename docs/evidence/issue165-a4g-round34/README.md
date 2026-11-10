# Issue #165 rounds 3/4 — Above-4G A/B（2026-11-09，报告者 junkjunker）

## 配置

- Dell Precision T5810（X99/C612，双 RTX 3060 GA106 `2504`：03:00.0 = Gigabyte SUBSYS_40721458，04:00.0 = ASUS SUBSYS_88181043）
- 构建：v2.1.11-diag2（2026-11-04）；两轮均 `\tx-select-bdf.txt` 钉单卡；FLR 路径两轮都执行
- **Above 4G Decoding：round3/4 均已开启**（round3 loader ENUMERATE：03 的 BAR1 = `0x33fe0000000` ≈207 GB Above-4G 放置，A4G ON 实证）

## 结果

| 轮 | pin | 解锁（ssdiag BAR 映射） | pnp | nvidia-smi |
|---|---|---|---|---|
| 3 | 03:00.0（Gigabyte） | 03=UNLOCKED（BAR f6000000）、04=NOT_UNLOCKED | **03（SUBSYS_4072）= CM_PROB_NORMAL_CONFLICT**、04=OK | 仅 04:00.0 可见 |
| 4 | 04:00.0（ASUS） | 04=UNLOCKED（BAR f4000000）、03=NOT_UNLOCKED | **两卡 CM_PROB_NONE** | 03+04 双卡可见 |

## 关键取证：BAR 足印逐位一致（推翻足印改变假设）

round3 目标（03）的 HANDOFF-PCI 转储 vs 固件预分配（ENUMERATE）：

| 寄存器 | 预（ENUMERATE） | 后（HANDOFF-PCI cfg dump） |
|---|---|---|
| BAR0 @0x10 | f6000000 | f6000000 |
| BAR1 @0x14/18 | 0x33fe0000000（flag 0xc=64-bit prefetchable） | e000000c / 0000033f = 同一值含标志位 |
| BAR3 @0x1c/20 | 0x33ff0000000 | f000000c / 0000033f = 同 |
| IO @0x24 | e000 | e001（io 标志位） |

事务恢复的 config 与固件分配**逐位相同**（含 64 位 BAR 标志位）→「解锁改变资源足印」在 config-space 层被证伪；冲突是 Windows 侧仲裁与槽位/devnode 交互。

## 四轮模式汇总

**被解锁的卡输掉 Windows 仲裁、未触碰的卡存活**——A4G 开关不变量（round1-2 A4G off、round3-4 A4G on 同形）；冲突只在「解锁 03」出现，「解锁 04」干净。03 槽位 devnode（instance `4&1c1c94c&0&0010`）与 phantom Quadro M2000（CM_PROB_PHANTOM）同路径——槽位残留 devnode 历史是头号嫌疑。

## 对照：维护者 3647 双路 4×3090 台架（2026-11-09，v2.1.12-rc1）

四张 2204 全链解锁全绿（4× `VERDICT [ampere]: UNLOCKED`、PLM OPEN、canary intact+SS live、任务管理器 GPU0-3 全 3090 可见）。与 #165 的差异向量：①四卡同型号同 SUBSYS vs 混装 OEM 卡；②服务器固件（Lewisburg 8086:2030 桥+C620 根口，MMIO 预算巨大、无残留显示 devnode）vs T5810 工作站（03 槽带 phantom M2000）；③对称 BAR 足印 vs 混合足印。**多卡事务链本体在 4 卡平台上无此症状 → #165 的冲突是单根复合体工作站+槽位 devnode 的仲裁交互，非工具链路。**

## events.txt

两轮附件 events.txt 均为 0 字节——下一轮需 `wevtutil qe System "/q:*[System[(EventID=411 or EventID=412)]]" /c:20 /rd:true /f:text`（Kernel-PnP 资源仲裁事件）或导出 System.evtx。
