# TOTORONG AD103_win_unlocker 对照（2026-10-26，仓库 51df9c1）

> 状态补注（2026-10-28）：当前 loader 使用 portal 装载，Falcon DMA 代码保留作实验。逐卡 ROM/PROM SHA-256 校验待实现，见[实现边界](faq.md#safety)。下表保留 2026-10-26 的实现对照。

对象：`reference/AD103_win_unlocker`（Windows UEFI 预驱动解锁的唯一先例，
RTX 4080 AD103 32GB 单卡实验）。结论：**链形状与我们同源且一致**（他们的
交易体就是社区 unlock_610.c 经 compat 垫片直接编译进 EFI）；载具工艺多处
差距已采纳（见"本轮已采纳"）；GOP 重连初判不采纳、后已转入采纳
（58b258f）；停机纪律与逐板钉死有意保持分歧。

## 逐点对照

| 点 | TOTORONG | 我们 | 裁决 |
|---|---|---|---|
| 交易体 | vendor unlock_610.c（社区原版）+ compat.c 内核 API 垫片 | tx.rs 1:1 移植同一源 | **一致**（同源） |
| 链形状 | claim→gfw→layout→fwsec→booter→selectors→FLR，**无 scrubber** | 同形 + 差异项：我们加跑**内核同款 scrubber**（BSI handoff 1→3，SEC2/GSP 域解锁——booter 能点火的前提） | 同源；scrubber 是我们独有的第 7 步 |
| 入口门 | chip==钉值、WPR2 hi>>4、PLM==0xffffff8f、SS0&0x88888888==0、SS1==7（逐板钉死）、VRAM 钉值 | family 表 + EALREADY/EBUSY 语义 | 一致方向；他们逐板钉死（单卡实验），我们按族（多卡工具）——各有立场 |
| FLR | GuFlr：等 Transactions Pending 清零→FLR 位**保留**其余 devctl→**100ms 全静默**→等 config 身份复原→恢复（解码关、高 BAR 先、ROM 不开解码、BME 永不还原） | 旧实现：写 0x8000 **清掉整个 devctl**、无 pending 等待、无静默即读 BAR、升序恢复 | **他们正确——已修**（flr_once + flr_phase 两处） |
| PCI 恢复 | 高 BAR dword 先于低、command 最后且 &~4（永不还原 BME）、逐值读回验证 | 升序全还原 | **他们正确——已修** |
| BME | 事务中 set_master（GuEnableMaster），结束后 GuClearMaster 收容 | claim 时 attributes_enable(0x7)，FLR 前清 | 一致 |
| GOP | 断开 vendor 驱动→事务→**ConnectController 恢复**→DisplayCheck→config/BAR 复核→POST-GOP selectors | 断开 → 事务 → FLR → **`reconnect_all` 递归 ConnectController**（58b258f 采纳，原判"不采纳"已反转）→ LoadImage bootmgfw | **已采纳**：FLR 后卡回冷态，重连让固件 BDS 重新发现设备、恢复 console；失败非致命（bootmgfw 自建 GOP 兜底） |
| 链回 | 预载 bootmgfw（事务前 LoadImage）→成功后 StartImage | 事务后 LoadImage/StartImage | 一致语义；他们预载避免事务后分配。可选优化 |
| Secure Boot | 读 SecureBoot 变量，非 0 拒绝 | 未检查（宪法要求实验机关闭） | 候选：加变量检查作显式报错（API 待接） |
| 卡身份 | PCI RomImage SHA-256 钉死 + PROM BAR 视图 SHA-256 双重钉板 | vendor:device + NV_PROM crc32 记录 | 他们更强（单卡可钉死）。候选：loader 加 no_std sha256，按机钉 PROM hash |
| 验证判据 | marker+HALT+三 PLM 读回，FBPA mismatch **非致命**（0xffffff0f）| 同（本轮修完 FBPA 判据） | **一致** |
| SS 读取防毒 | VerifySelectors 拒绝 0xffffffff 与 **0xbadf5620** | inaccessible() 桶（含 badf5620） | **一致**——旁证：badf5620 在 AD103 上也被 TOTORONG 视为已知毒值（开放问题 #3 收敛：跨族"域门控/锁定"族） |
| 停机纪律 | 清算不可证明→Halt() 死循环（物理断电） | 失败→emergency FLR→链回 | 分歧但更保守：我们选 FLR 交还冷卡（已实机验证 Windows 恢复）；他们的场景是单卡实验机不交还。保留 |
| 日志即门 | 每阶段 Step() 要求 GuLogReady，否则 DEVICE_ERROR | 日志尽力而为 | 候选：关键阶段（事务后）加 log-ready 断言 |

## 本轮已采纳（commit 见 git log）

1. `flr_once`/`flr_phase`：devctl 保存+保留式置位、Transactions Pending
   等待、FLR 后 100ms 静默、config 身份复原等待、恢复顺序
   （解码关→BAR 高到低→command 去 BME 最后）。
2. FBPA 判据 0xffffff0f/0xffffffff 双接受（上一轮，28e1 对照）。
3. scrubber 非致命 + EALREADY=skip（上一轮，unlock_610 对照）。
4. **GOP 重连** `reconnect_all`（递归 ConnectController，失败非致命）+
   多 bootloader 候选回退（58b258f）——初判"不采纳"已反转，见上表 GOP 行。

## 有意不采纳

- STOP 即 Halt 死循环（我们的 FLR 交还已验证可恢复；两者满足同一宪法
  条款"STOP 后不重试"，恢复手段不同）。
- 逐板钉死入口值（SS1==7 等）：他们是单卡实验，我们是多卡工具；族级
  判据 + 实测冷值归档（docs/cold-values.md）替代。

## 后续候选（按价值排序）

1. loader 内 no_std SHA-256：PROM 全镜像 hash 记录 + 按机钉死（宪法 §1.5
   的完整形态）。
2. SecureBoot 变量检查（显式拒绝+清晰日志）。
3. PciIo.Flush 接入 DMA 缓冲填充后（TOTORONG dma_wmb 组合拳；DMA 路径
   复活时一并做）。
4. 事务后 log-ready 断言（证据完整性）。
