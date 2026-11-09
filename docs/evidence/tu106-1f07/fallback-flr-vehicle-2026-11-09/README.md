# TU106 FLR 回退载具构建零误触轮（2026-11-09）

## 目的

验证 1792a30（STAGING-FALLBACK 重试载具换 FLR + FWSEC 重新点火）在**健康机默认路径上零漂移**：门条件未变，TU106（HIGH 可达）必须零 `STAGING-FALLBACK` 行、全链全绿。

## 配置

- 构建版本串 v2.1.12-rc1（built Mon 2026-11-09 16:28:44 +08:00），部署 EFI sha256 `4c515afd23b56fa11264…`（462336 B），H 盘（内置 SATA ESP）
- marker：`i_know_what_i_am_doing.txt`（armed 0s）、`tx-quiet.txt`、`tx-ultrafast.txt`、`tx-no-console.txt`；无 `\tx-preload.txt`（#198 opt-in）、无 `\tx-low-staging.txt`（默认 HIGH）、无 `\tx-relock.txt`（keep-open 默认）
- 本轮超集确认（并行线已在 main）：金丝雀恢复 ultrafast 运行（4f90e2a，HANDOFF-OUT 带真值而非 `off(ultrafast)`）；CHAIN-PRE 预载 opt-in（280b60e）；CHAIN reconnect pass ultrafast 跳过

## 结果（全绿）

- `STAGING-FALLBACK` 行 **0 次**——健康机零误触（sim t39 实机孪生）
- FWSEC WPR2 UP 974ms（3ms 建窗）→ GSP-HALT gate 151ms → booter `mailbox0=0x0`
- selectors SS0/SS1 写读回 `0x88888888`/`0x8`；READOUT `0x00000000` full speed
- `HANDOFF-OUT`：`tu10x:unlock=true` + `canary=intact(0x0164c0de/0x0165cafe)` + 尾部四字段 badf（绑定窗 artifact，既定口径）
- Turing FLR skipped；native-GOP 重建绑定成功（`binding Start=SUCCESS`）

## 计时

`121ms`（log open）→ `673ms` PROCEED（armed 0）→ `1979ms` CHAIN LoadImage；**事务段 detach(712ms)→FINAL(1935ms) ≈ 1.22s，全程 ≈ 1.9s**。对照阶梯：22.1s（v2.1.8 代）→ 17.2s（quiet）→ 3.2s（armed1+ultrafast，USB）→ 2.835s（armed0）→ 2.03s（stall 删+canary 回归）→ **1.9s（+预载 opt-in+chain skip，本构建）**。FLR 载具改动默认路径零成本（回退代码只在 0x89 死角执行）。

## 边界

- 本机 HIGH 可达，**FLR 载具本体未在本轮执行**——回退臂全链真值待 #156 报告者（1e93）裸跑回收；TU106 只承担"门条件未变 + 零误触"验证。
- `\usbroot\v67-direct-ss.bin fs open failed ACCESS_DENIED` 为既定探测序（主路径 `\v67-direct-ss.bin` absent 后落 `\v67-tu10x-four-plm.bin`）。
