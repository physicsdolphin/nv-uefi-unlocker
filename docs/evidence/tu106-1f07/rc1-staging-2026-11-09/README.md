# TU106 台架：v2.1.12-rc1 STAGING-FALLBACK 双臂验证（2026-11-09）

构建 = v2.1.12-rc1 tx（sha `bece7118…`，460288B）。U 盘常驻 marker：`i_know_what_i_am_doing.txt`（=1，1s armed）、`tx-quiet.txt`、`tx-ultrafast.txt`、`tx-no-console.txt`。

## Arm 1（默认 HIGH，负控）— `loader-high-staging.log`

- `TX: >4GB staging gate: true`（1614ms）；booter 组全部 HIGH：meta **0x10001f000**、签名 0x10000f000、radix+RM image 0x100027000（2052-2067ms）——meta 地址与 #156 毒机完全同段；
- `halted, mailbox0=0x0`（2150ms）= **TU106 对 ≥4GiB staging 读取全绿**；
- **全程零 `STAGING-FALLBACK` 行**（grep 计数 0）= 回退门在健康机不误触（sim t39 的实机对应）；
- shape B 交棒：`SELECTOR RESULT: SS0=0x88888888 SS1=0x8`、`tu10x:unlock=true`（209 行）。

## Arm 2（`\tx-low-staging.txt`，回退臂落点预演）— `loader-low-staging.log`

- `>4GB staging gate: false`（46 行）；全部 `DMA alloc … [LOW]`（meta 0x3a0de000）；FWSEC 照旧默认 LOW（88 行）；
- `CHAIN-PRE: no \tx-preload.txt - preload is opt-in since #198`（rc1 预载新默认首次上机）；
- `mailbox0=0x0` + `tu10x:unlock=true`，**零 `STAGING-FALLBACK` 行**（LOW 自选压低不进门 = sim t35 的实机对应）；
- **系统内 ssdiag**（`ssdiag-in-os-excerpt.txt`）：turing 0x4096xx PLM=0xffffffff（OPEN）、SS0=0x88888888、READOUT=0x00000000（full speed）、SS1=0x8——**rc1 门开交棒在 OS 侧活体确认**（0x823xxx 跨族块 badf = Turing 卡预期）。

## 判读结论

1. **负控通过**：健康机 HIGH 一次过、回退门零误触；两臂 `mailbox0=0x0` + unlock=true。
2. **机器相关性再添一证**：同一 meta 地址 0x10001f000——TU106 交付（mailbox0=0x0），#156 Area-51m R2 0x89；STAGING-FALLBACK 的门控（0x89 + 宿主 magic 完好）在健康机无假阳性面。
3. **尾部 `0xbadf5040` 四字段**（两臂 209 行均在，native-GOP binding Start=SUCCESS 之后）= 绑定期读窗 artifact，与 11-08 keep-open 轮逐字同形；权威判读 = `tu10x:` 元组 + 系统内 ssdiag（本轮 Arm 2 系统内 PLM OPEN 活体证实）。
4. 台架工程记录：首轮 HIGH 臂日志因拔盘过快丢失（FAT dirent/USB 盘内缓存；loader init 会删上一轮日志），重跑即复现——**日志须在两轮之间拷走**；不构成构建层回归（同构建 LOW 臂日志完整）。

## 关联

- release: v2.1.12-rc1（pre-release）；实现 e2217b2；sim t34–t39。
- 带要求双向判例表：docs/compatibility.md 0x91 全景段；#74/#156/#191。
- 前一轮门开证据：`docs/evidence/tu106-1f07/keep-open-2026-11-08/`。
