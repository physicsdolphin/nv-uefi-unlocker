# TU10x 取证诊断协议 v2（v2.1.8，2026-10-31 第二轮）

> **v2.1.8 变更**：① GSP-HALT 判据（精确 WPR2 lo+hi + GSP HALT 10s + mailbox 回读 + scratch0e FRTS 错误码门控）**转正为 tu10x 默认**——#62 R1 场证明（`WPR2 UP 3ms` vs `HALT 156ms`，34.9→69 TFLOPS），`\tx-no-gsp-halt-gate.txt` 为逃生开关（恢复旧行为，仅消融用）；② **FWSEC staging 默认改低位**——#74 第 15-28 轮单变量实锤 ≥4GiB 在 TB 平台全链不可达（`WPR2 never came up` 零残影），而 sub-4GB FWSEC 在 #16/#12/#14/#74 全绿零反例；meta/sig/radix 维持 >4GB 门控，`\tx-fwsec-high-staging.txt` 恢复旧高位（消融用）；③ `\tx-fwsec-gsp-halt.txt`/`\tx-fwsec-hold-dma.txt` 退役（已转正/页复用轴证伪）；④ `\tx-sec-base.txt` 的 `0x` 前缀解析修复（曾无声失效，坑了 #57 E 轮——`0x200` 与 `512` 现在等价）；⑤ 0x89 提示扩为三成因（桥 BME/地址带不可达/固件 IOMMU——#74 实证 BME 全开仍 0x89）。
>
> 配套：[tu10x-booter-0x91-matrix.md](tu10x-booter-0x91-matrix.md)（成败矩阵+第一轮结果+指纹）、[tu10x-booter-nsstub-disasm.md](tu10x-booter-nsstub-disasm.md)（mailbox/EXCI 字典）。

## 每次运行自动记录（回收清单不变，另加）

- `GSP-HALT gate: … halted=true after Nms` + `FWSEC result (kernel shape): … frts_err=…`——**默认开启**；Nms 是该机 FWSEC 尾巴时长的直接测量。
- `BOOTER-AUDIT: ns/dmem readback MATCH` 为有效判据；**sec 行在 Turing 上一律返回 scrub 毒值（成功机实证），无判别力**。
- `signature DMA never landed` verdict 仅 V67 模式有意义；replay/快速 sanitize 下是已知假阴性。

## 按残留类的第二轮消融（每轮一次冷启动、一个变量）

**A 类（FWSEC 死：`WPR2 never came up`）——#89**：
1. **v2.1.8 默认已修**（FWSEC staging 低位化）——直接跑无 marker 默认路径，预期 WPR2 起来。
2. 若仍死 → `\tx-low-staging.txt`（全局低位含 meta/radix）排除其余缓冲的地址带问题；若 booter 阶段变 0x91/0x89（#16 型 sub-4GB meta 锁），回报即可（混合带机器的存在性本身是重要数据）。

**B 类（FWSEC 全绿但终值 0x0B/0x11）——#57、#82**：
1. `\tx-bootvec-sec.txt`（跳 NS 桩直入安全段）——区分"桩的环境块被拒"vs"安全码自身拒"。
2. `\tx-sec-base.txt` 搬址轮重做（`0x200` 前缀 bug 已修，#57 的 E 轮作废重跑）。
3. Windows 侧（#82）：升级 GSP≥620/驱动≥621 后复测（D 类预防——615.78/617.42 已实证 init 重编程 FEAT）。

**E 类（Windows 病态）——#90**：先跑 stock 基线三件套（不插 U 盘正常启动 + ssdiag 读数 + VRAM/内存测试排 WHEA）再谈软件。

**纪律**：彻底断电冷启动；同会话失败不重试；回收完整 loader.log。
