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

## 速度阶梯（同机速度栈全开，2026-11-09 收敛于 ~1.9-2.0s 地板）

| 级 | 构建/配置 | loader 起跑 → CHAIN LoadImage | 证据 |
|---|---|---|---|
| v2.1.8 代（诊断全开） | 22.1 s | issue #38 基线 | — |
| v2.1.9（quiet+armed 5s） | 17.2 s | issue #38 | — |
| rc1（ultrafast+no-console，armed 1s） | 3.2 s | 113→3205ms | `loader-high-staging.log`（当时构建） |
| rc1（armed `0`） | 2.835 s | 113→2835ms | `loader-armed0-2835ms.log` |
| + 两处固定 stall 摘除（06f3a14） | 2.033 s | 用户实测 | — |
| + CHAIN pass skip + 金丝雀回归 ultrafast（2e01833/4f90e2a） | **1.821 s** | 119→1940ms，`CHAIN: reconnect pass skipped` 触发、`canary=intact` 回归 HANDOFF | `loader-allspeeds-1821ms.log` |
| （尝试后撤销）targeted PROM read | 1.920 s | TU106 walk 超 128KB → 稀疏解析 None → 全量回退，净负 ~45ms | `loader-promfallback-1920ms.log` |

**地板定谳（用户实测 1.9–2.0s 波动）**：剩余大头 = fwimage USB2 读 ~310ms（硬件）、PROM 全量 ~225ms（需 per-die walk-region 逆向立项才可省，撤）、GSP-HALT ~151ms（FRTS scrub 硬件时间）、native-GOP 绑定+console 挂接 ~650ms（显示路径证据本体）。`no-reconnect.txt` 可再省 ~650ms 但以显示路径验证为代价（DDC-CI 一类会碎），不作为速度手段。

## 关联

- release: v2.1.12-rc1（pre-release）；实现 e2217b2；sim t34–t39。
- 带要求双向判例表：docs/compatibility.md 0x91 全景段；#74/#156/#191。
- 前一轮门开证据：`docs/evidence/tu106-1f07/keep-open-2026-11-08/`。
