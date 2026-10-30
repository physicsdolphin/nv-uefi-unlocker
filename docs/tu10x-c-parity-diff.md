# TU10x 成功实现 × main 差分清单（Phase B，2026-10-31）

> 方法：travis-mcdevitt fork `2654419e` 的 `docs/evidence/msi-tu102-guarded/source-overlay/`（19 文件）逐文件 diff 其声明基线 `3bb069b`（CRLF 差异除外）；PR #49（`fix/tu102-1e07-c-compatible`，base `a99fe4b`）完整 diff；nlzy/turing-unlock（Linux TU104 成功链）源码通读。配套成败矩阵见 [tu10x-booter-0x91-matrix.md](tu10x-booter-0x91-matrix.md)。

## B0. 决定性交集差分：FWSEC 完成判据与 staging 释放时机

| 实现 | FWSEC 完成判据 | FWSEC staging 释放时机 | 现场结果 |
|---|---|---|---|
| **main GA10x/Ada**（`fwsec_ignite`，tx_fwsec.rs:607-633） | WPR2 窗口校验 + **`poll_us(GSP+0x100, HALT, 5s)`，`r?` 失败即中止** | HALT 之后 | 全绿（Ada/Ampere 场验证） |
| **main tu10x**（`fwsec_ignite_tu10x`，tx_fwsec.rs:790-820） | **仅 WPR2 lo 匹配 + hi≠0**（实测 0–3ms 即中） | **WPR2 一中立即 `dma.release`（FWSEC staging≈40KB/10 页），且从不读 FWSEC 结果寄存器；后续分配可复用这 10 页** | 硬核四例 0x91 |
| nlzy/turing-unlock（Linux，TU104 1e84 成功） | `poll(GSP+0x100, HALT, 5s)` + scratch0e hi16=0 + FRTS lo 精确匹配（unlock.c:346-364） | **模块退出才 `dma_free_coherent`**（unlock.c:925-927） | 成功 |
| PR #49（#17 同卡成功） | 精确 WPR2(lo+hi) + **GSP HALT 5s**，文档明言"避免把 WPR2 刚出现当作固件完成或**提前复用缓冲区**" | 成功/双 HALT 后 | 成功（+51%） |
| travis guarded（MSI TU102 成功） | `wait_fwsec`：**GSP stopped + WPR2 精确 lo==frts>>8 且 hi==(frts+0xe0000)>>8**，双条件缺一不可（guard/src/lib.rs:471-495） | stopped 后 | 成功 |

**结论**：三个成功实现（含 main 自己已验证的 Ada 路径形态）全部要求 FWSEC 后 GSP 进入 HALT 再继续；main 的 tu10x 路径是唯一例外。这是三方交集里唯一的强机制性差分，候选子机制：

- **(a) 页复用竞态（窗口已收窄）**：WPR2 一中即 free FWSEC staging（`fw.image_size`≈40KB/10 页，**非** radix 的 28MiB 块——早先记录有误，2026-10-31 更正）；后续 meta/stock/boot/booter 五次分配在确定性 EFI 分配器下可能复用这 10 页。窗口真实但窄。
- **(b) 提前点火 SEC2（主轴）**：GSP 侧 FWSEC 未收尾（WPR2/MMU 锁/安全世界状态仍在编程）且 **FWSEC 结果寄存器从未被读**（见 B0.1——内核在停机后必查 mailbox/scratch0e）时 booter 已点火——NS 桩能跑（明文、无依赖），安全段/验证阶段对账失败 → 0x91；
- (a)+(b) 都以固件收尾时长 vs 主机准备时长为变量 → **同机确定性、跨机分裂**（与全部现场形态吻合，含 #14 的 v0.2.0→v2.1.1 翻转：分配区域改变→复用模式改变）。

### B0.1 内核端复核（open-gpu-kernel-modules 610.43.03，2026-10-31）

一手源码逐行核对，**内核中不存在任何"不等 GSP 停机就继续"的路径**：

- `kgspExecuteHsFalcon_TU102`（`kernel_gsp_falcon_tu102.c:382-437`）：`prepare(WITH_LOADER/DIRECT)` → 写 mailbox → `kflcnStartCpu_HAL` → **`kflcnWaitForHalt_HAL(GPU_TIMEOUT_DEFAULT)`** → 回读 mailbox。注释原文顺序即"Start CPU now / Wait for completion"。
- `kgspExecuteFwsec_TU102`（`kernel_gsp_frts_tu102.c:463-533`）：在 HALT 之后依次校验 **`scratch0e[31:16] FRTS_ERR_CODE==0`**（`NV_VBIOS_FWSECLIC_SCRATCH_INDEX_0E`）、**WPR2_HI≠0**、**WPR2_LO==frtsOffset>>ALIGNMENT（精确）**——错误即 `NV_ERR_GENERIC` 拒绝。**main 从未读过 scratch0e**。
- `kern_gpu_tu102.c:424-432` `gpuWaitForGfwBootComplete_TU102`：注释原文"**we want to check that FWSEC has lowered its PLM first. If not then obviously it has not completed**"——NVIDIA 自己把"PLM 未降=FWSEC 显然没完成"写成了显式判据；超时预算 50ms 启动+2s 完成。
- `kgspExecuteCoreResume_TU102`（同 falcon 文件 444-489 行）：SEC2 侧同样纪律——START → `gpuTimeoutCondWait(BSI_14 handoff DONE)` → **查 SEC2 mailbox0==0**。
- nlzy/turing-unlock `unlock.c:346-364`：`poll(GSP+0x100, HALT, 5s)` + scratch0e hi16==0 + WPR2 lo 精确——内核形状的逐行移植；全部 staging `dma_alloc_coherent` 挂在 `w->` 上直到模块退出（`unlock.c:912-929`）才释放。

→ `\tx-fwsec-gsp-halt.txt` marker 已按此对齐：HALT（10s）+ 精确 lo/hi + **scratch0e FRTS 错误码门控** + mailbox0/1 回读记录。

## B1. travis overlay vs base `3bb069b`（guard 之外的全部行为差分）

tx.rs +78/-29、tx_fwsec.rs +32/-2、pci.rs +17、main.rs +29/-2、chainload.rs +21/-2、keywait.rs +4、Cargo.toml +4。`tx_booter.rs`/`tx_portal.rs` **不在 overlay 内**——booter 点火代码就是基线原版（marker `\tx-sec2-c-load.txt` 驱动）。

| # | 差分 | 内容 | 判定 |
|---|---|---|---|
| T1 | **FWSEC 完成判据**（tx_fwsec.rs） | `wait_fwsec`：GSP stopped + WPR2 精确 lo/hi；stopped 前不释放 DMA | **头号嫌疑（见 B0）** |
| T2 | **跳过 `pm_preflight`**（tx.rs） | guarded 下不执行：少一次 PMCSR 读+force-D0 写。冷启动 D0 时 force-D0 写多为 no-op | 弱嫌疑（一个配置写） |
| T3 | **跳过 `BridgeMaster::enable`**（tx.rs/pci.rs） | 不写上游桥 BME（他的板本就使能）。仅当 main 在该机上实际写过桥时才构成差分 | 弱嫌疑（查日志 before/after-bridge-bme 行可判） |
| T4 | **FWSEC 镜像改用宿主侧钉死副本**（tx_fwsec.rs） | 不在运行时经 BAR0 读 1MiB PROM；用 hash 钉死的 host blob + 预解析字段 | 中弱嫌疑（少 1MiB BAR0 PROM 读） |
| T5 | `post_flr=false` 进 claim_guards | 冷启动（WPR2=0）时与 tu10x 无条件接受等价 | 等价 |
| T6 | layout 身份断言（fb==22GiB && LMR==0x2c9） | 只读检查 | 无行为差 |
| T7 | 结尾 guard/释放校验、reconnect 跳过、no-ss 禁用、read32 改 try 读取 | 全部在成功点之后或纯读 | 无前置行为差 |
| T8 | `dma_alloc` 恒走 `dma_alloc_high`（无 PciIo 降级） | 与 main 的 tu10x 默认（高staging 开）等价 | 等价 |

**→ travis 成功相对 main marker-A 轮的最小必要差集 = {T1, T2, T3, T4}，其中 T1 独占机制性重量。**

## B2. PR #49（TU102-C-FIX1）相对 main 的差分

在 T1 之外另有（多变量打包、未消融，作者明示不声称唯一根因）：

| # | 差分 | 内容 |
|---|---|---|
| P1 | Reserved-memtype 高位分槽 staging | `dma_alloc_tu102_compat`：`AllocateAddress`+`MemoryType::RESERVED`，`0x110000000` 起 +16MiB×12 槽，不降级 |
| P2 | 全零 dummy RM | fwimage 用全零 `0x1bfadc8` 镜像替代真实 GSP-RM |
| P3 | bootBin/gspFw 指向 VRAM | meta `gspFw=0x57e1f0000`、`bootBin=0x57fdfa000` |
| P4 | PTIMER seed + host-probe | fix1 配置含（travis 证明非必要） |
| P5 | 身份/PROM/资产钉死 + 一次性授权 | preflight |
| P6 | DMA 全程持有，双 HALT 后才释放 | 与 T1 同族 |

**travis（T1-T4，无 P1-P5）与 PR #49（P1-P6+T1）都能成功 → 公共子集 {T1, T2, T3, T4}；其中 T1 是唯一带明确机制的成员。**

## B3. nlzy/turing-unlock（Linux TU104 成功）对照

- FWSEC：`flcn_reset` → FBIF CTL bit7 + DMACTL=0 + TRANSCFG idx4=phys → BL 装载 → START → **poll HALT 5s** → scratch/FRTS 校验（与 main Ada 路径同构，与 main tu10x 路径不同）。
- staging：全部 `dma_alloc_coherent` 存入 `w->`，**模块退出才统一释放**——不存在"FWSEC 后早释放"。
- 另证：其 meta=`0x34c11000`（sub-4GB sysmem）成功——sub-4GB 读锁并非全机器成立（对照 #16）。

## B4. 消融 marker 优先级（下一轮现场实验顺序）

1. **`\tx-fwsec-gsp-halt.txt`** —— tu10x FWSEC 完成判据改为：WPR2 精确 lo+hi（hi==frts+0xe0000）**且** GSP CPUCTL(0x110100) HALT（10s 预算），HALT 前不释放 staging、不进入 booter 阶段。**预期：若硬核四例中任一转绿，根因闭环。**
2. `\tx-fwsec-hold-dma.txt` —— 只把 staging 持有到事务结束（不释放），不加 HALT 等待。用于把 B0 的 (a)（页复用）与 (b)（提前点火）分解（若 #1 转绿而 #2 不转绿 → (b) 为主）。
3. `\tx-no-pm-preflight.txt` —— 跳过 pm_preflight（T2）。若 #1/#2 阴性再试。
4. `\tx-no-bridge-master.txt` —— 跳过 BridgeMaster 写（T3；先查日志确认该机是否实际写过桥）。
5. `\tx-fwsec-preparsed.txt` —— 跳过运行时 PROM 读（T4；配合宿主侧钉死资产，仅诊断）。
6. （#1 转绿后的收敛验证）PR #49 独有项 P1/P2/P3 单独复刻的必要性检验——预期全部"非必要"。

## 对 #14 历史翻转的覆盖

v0.2.0（PciIo 任意页）→ v2.1.1（伪高位=cap 下最高空闲段）改变了 FWSEC staging 与后续 radix 分配的**页布局关系**；在页复用竞态假设 (a) 下，同 meta 地址 `0x42b66000` 一次碰撞一次不碰撞完全成立。#14 复跑 v0.2.0 复现 0x91 = 确定性页布局的体现。（不作为独立证据，仅记录假设覆盖面。）
