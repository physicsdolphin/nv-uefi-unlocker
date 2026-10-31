# TU10x booter 0x91 成败矩阵与缺口清单（2026-10-31）

> 调查对象：Turing（tu10x）链 booter 阶段确定性失败族——halt 后 `mailbox0=0x91/0x11/0x0B/0x31`、`EXCI=0x00900100`、PLM 不开、`signature DMA never landed`。
>
> **定性更正（重要）**：这不是"时成时败"的偶发问题。证据形态是**同一人同一卡跨所有版本必败（或必成）**；同型号卡在不同人手里确定性分裂（2080S 1e81：#14 必成 / ProxVirtualZone 四轮必败；2080Ti 1e07：Reddit 双卡必成 / #17 必败）。分裂变量跟随**机器/点火环境**，不跟随卡型号、显存容量（8G TU104-400 也败；22G TU102 在 C 链下能成）。
>
> 数据来源：issue #12/#14/#16/#17/#55/#57/#62/#74/#82/#84/#89/#90/#91，PR #49/#64/#65/#67，travis-mcdevitt fork 证据包 `2654419e`（docs/evidence/msi-tu102-guarded），Reddit r/overclocking 1xj5jp3。引用日志行保留原文。

---

## A1. 谁成功 / 谁失败（逐数据点）

### A1-F 失败侧（"硬核"四例：当代版本+全消融仍败）

| # | 报告人 | GPU / device ID / VBIOS | 显存 | 平台（CPU/主板/固件） | OS / 驱动 / GSP | 工具版本/构建 | 路径 | 结果与关键读数 | 来源 |
|---|---|---|---|---|---|---|---|---|---|
| F1 | OBC100（#17） | 2080 Ti 魔改，TU102，`10de:1e07`，`90.02.17.00.e9`，chip `0x162000a1` | 22528 MiB | R7 7700 + 七彩虹 CVN B650M GAMING FROZEN（BIOS 1018） | Win11 工作站版；616.56 / GSP 615.65.06（EnableGpuFirmware=1）；SB off | v0.2.0→v2.1.6 全部官方版 + v2.1.7-tu102-diag（min-tu102-22g@309d949 源码自编） | 默认 + marker 矩阵 A/B 轮 | 全败：`halted, mailbox0=0x91`、`exci=0x00900100`、`PLM=0xffffff8f`。v2.1.6 起几何逐位正确（`LOCAL_MEMORY_RANGE 0x2c9 → 22528 MiB`、heap `0x6b00000`）仍败。**同卡同机：chenxluo 原生 C 与 PR #49 全 C 链成功**（见 S5）——机器不受诅咒，路径是分野 | #17 |
| F2 | hackerwxsk（#57） | TITAN RTX ×2，TU102，`10de:1e02`，`90.02.42.00.0f` | 24576 MiB | R9 5950X + ASUS ROG STRIX X570-E（AMI 5013），94GB DDR4 | 纯 Linux Debian 12 + 615.71.09 open（GSP-RM 常开）；链 `\EFI\debian\shimx64.efi`；SB off | v2.1.6 + 自制 4 字节 heap 补丁（24G→107MiB，`.text` RVA 0x1abe3）+ `tu10x-fb-gb.txt=24` | 默认 / low-staging / pre-flr | R1 默认（staging `0x100010000`）→ `mailbox0=0x11`；R2 low-staging（`0xc639f000`）→ `0x91`（精确复现 #17）；R3 pre-flr → `GFW did not re-complete within 10s`（更早失败）。`exci=0x00900100` 全程。前半段全绿（WPR2 3ms 起、heap 公式对） | #57 |
| F3 | chenc222（#62） | RTX 2080，TU104-400，`10de:1e87`，`90.04.0B.00.A5`（原厂 PROM 落盘证实） | 8192 MiB | B760 Intel UEFI（非 CSM）；无核显独显直连；Above4G on（BAR1@`0x400000000`）、ReBAR off、VT-d off | Win + 621.01 / GSP 620.67.01 | v2.1.1/2.1.5/2.1.6 + 8 轮单变量；A–F 轮用 v2.1.7-tu102-diag 与 PR #67 head `cdc634076`（自编 `1aa5651153e82fcc…`） | 默认 + C-load + #67 开关 | 8 轮全 `0x91`（含断电冷启、staging 强制 768MiB `0x3000b000`）；A/B/C 轮（C-load、+PTIMER+host-probe、v67-direct-ss）仍 `0x91`/`exci=0x00900100`。**D/E 轮（搬安全段基址 0x100→0x200 / BOOTVEC 直入）首次改变异常类**：`exci=0x00a00100`、`mbox0=0x31`；F 轮（secure=false）→ `exci=0x50aa2863`、DMEM 完全未动。布局与成功 2080S 逐位相同 | #62 |
| F4 | ProxVirtualZone（#62 尾） | RTX 2080 SUPER，TU104-450，**`10de:1e81`（与 #14 成功卡同 device ID）**，subsystem `13a1:10de` | 8192 MiB | R9 3900X + MSI MPG X570 | Win11 | 自编译 `c52e8ea`（v2.1.6 谱系） | 默认 + C-load（有效/无效 marker）+ 全套诊断，共 4 轮 | 一致失败：`mailbox0=0x0B`、`exci=0x00900100`、`tu10x_unlock=false`。**同 ID 跨机器分裂的直接实证** | #62 |

另类（非 0x91 族，单列）：**#55**（维护者自试）Z370+10700K（核显开）+2080S 独显单屏、Win10 GSP 610——解锁成功但交棒黑屏 + Windows 蓝屏 `APC_INDEX_MISMATCH`；TU10X PLM relock 只回 `0xffffffef`（差 bit4）。归"交棒完整性"问题（#35/#61 谱系），与本矩阵的 booter 拒收族不同病。

### A1-S 成功侧

| # | 报告人 | GPU / device ID / VBIOS | 显存 | 平台 | OS / 驱动 | 工具版本/构建 | 路径 | 结果 | 来源 |
|---|---|---|---|---|---|---|---|---|---|
| S1 | vadash（#12） | 2060 Super，TU106，`10de:1f06` | 8192 MiB | AMD（root `1022:1450`，B450/X470 系） | Windows | **v0.2.0**（built 2026-10-27） | 默认（sub-4GB staging 时代） | 全链成功：FWSEC→WPR2→BOOTER→PLM 全开→SS0/SS1 `0x88888888/0x8`→relock 持久。首个 AMD host TU106 证据 | #12 |
| S2 | wxw663（#14） | 2080 Super，TU104，`10de:1e81` | 8192 MiB | Intel Alder/Raptor（`8086:a704`/`a70d`）；Above4G on | Windows | **v0.2.0 必败 → v2.1.1 起必成**（v2.1.3 复测 `tu10x_unlock=true`；v0.2.0 复跑复现 0x91） | 默认 | 46→68 TFLOPS。**翻转发生在同卡同 staging 地址（`0x42b66000`，均 sub-4GB）**——v0.2.0→v2.1.1 的构建差异至今未识别（GOP 断开与地址带均已排除） | #14 |
| S3 | wuzhong081-byte（#16） | RTX 2070，TU106，`10de:1f14`，`90.06.59.00.5a`，chip `0x166` | 8192 MiB | 炫龙 CNH5L / Clevo NH5Ax 准系统（桌面 R7 3700X，无核显独显直连，Insyde `1.07.07THZX2`） | Win；591.86 / GSP 590.57 | **v0.2.0、v2.1.1 均 0x91 → v2.1.3 成功** | 默认 | 20.76→37.38–37.97 TFLOPS（+80%）。**决定变量=真 >4GB staging**（v2.1.2/v2.1.3 的 GetMemoryMap+AllocateAddress；v2.1.1 伪高位仍落 `0xd8ad1000` 时必败） | #16 |
| S4 | travis-mcdevitt | MSI 2080 Ti 22G，TU102，`10de:1e07`（`1462:371d`），`90.02.42.00.35`（与 #17 同 ID 同容量、**不同 VBIOS/FWSEC profile**） | 22528 MiB | Windows（未记录板卡）；610.88 / GSP 610.57.01 | Windows | base `3bb069b` + guarded source-overlay build 004（EFI SHA `73cbe014…`，一次性 `guarded-auto-once.txt`） | **C-load**（无 reset、TRANSCFG 不动、3s 静默；**无 PTIMER、无 host-probe**；强制 >4GB） | attempt-003 一次成功：`mailbox0=0x0 mailbox1=0x1`、`failure_mask=0x00`、`FEAT_PLM=0xffffffef`、SS `0x88888888/0x8`、六项 DMA 全释放；1021.36 FLOP/SM/clock（FP32acc=FP16 全速），GEMM 78.04/79.97 TF。**注意 `exci=0x00900100` 在成功链中同样出现（三次 attempt 全有）** | fork `2654419e` 证据包 |
| S5 | OBC100（PR #49） | **= F1 同卡同机**（#17 的 22G 魔改） | 22528 MiB | = F1（B650M+7700） | = F1 | PR #49 分支（base `a99fe4b`=v2.1.5）feature `tu102-c-compat`（TU102-C-FIX1） | **全 C 链**：Reserved 分槽 staging + dummy RM + FWSEC 等 GSP HALT + PTIMER + host-probe + fire_direct 静默窗 | 成功：`booter Ok(()) mailbox0=0x0`、`PLM=0xffffff8f SS0=0x88888888 SS1=0x8`、`tu10x_unlock=true`；64.353→97.388 / 67.210→100.962 TFLOPS（+51.33%/+50.22%）。多变量移植、一次成功、未逐项消融 | PR #49 / #17 |
| S6 | u/Additional-Avocado33（Reddit） | 2080 Ti ×2，`10de:1e07`（17:00.0 + 65:00.0） | （22G 魔改概率大，未确认） | 未记录 | Windows（LLM 推理负载） | **未知（待问）** | 未知（ssdiag 读回验证） | 双卡 `VERDICT [turing]: UNLOCKED`：READOUT=0 全速、SS0=`0x88888888`、SS1=`0x8`；tensor 56→203 TF；q4 14B 118.7/391W→121.9/442W | Reddit 1xj5jp3 |
| S7 | nlzy/turing-unlock（Linux） | 2070 SUPER，TU104，`10de:1e84` rev a1（HP OEM `103c:8701`），`90.04.76.40.2A` | 8192 MiB | Linux 侧 | EndeavourOS + 615.78.08 open | nlzy/turing-unlock 2.0.0-2（kmod） | Linux direct-via-dma | 成功：`meta=0x34c11000`（**sysmem 低址**！）、`dmem sig readback match=1`、`halted=1 plm=0xffffffff`、FLR 后干净绑定。**sub-4GB meta 在 Linux 上成功——对照 S3 的 sub-4GB 被拒** | #55 pascalblanchet 长评 |
| S8 | 维护者自机 | RTX 2070，TU106，`10de:1f07` | 8192 MiB | 主力实验机 | Windows GSP 模式 | 各版本回归基准；**2026-10-31 诊断件回归通过**（BOOTX64.EFI.tu10x-forensics-diag sha256 `ae14421d…c27d44f`，无 marker 默认路径=main v2.1.6+只读诊断行，解锁正常落地——维护者实测报告，loader.log 待归档） | 默认 | 长期全绿（回归门） | 本仓证据链 |

谱系对照（非 20 系，不展开）：#52 X99+GA103 2420、Z790+GA102、#11 3060Ti LHR 2489 等 GA10x/Ada 成功族。

---

## A2. 版本 × 结果交叉（确定性证据）

修复级联与各机器的"离场点"：

| 版本 | 关键变化 | #12 (TU106) | #14 (TU104-1e81) | #16 (TU106-1f14) | #17 (TU102-22G) | #57 (TU102-24G) | #62 (TU104-1e87) | ProxVZ (TU104-1e81) |
|---|---|---|---|---|---|---|---|---|
| v0.2.0 | 恒 sub-4GB staging；无多卡/GOP 归属检查 | ✅ | ❌ 0x91（必败，复跑复现） | ❌ 0x91 | ❌ 0x91 | — | — | — |
| v2.1.1 | "伪 >4GB"（MaxAddress=u64::MAX 实落 1–3GiB）+ 多卡 GOP 归属检查（单卡机不再断 console） | — | ✅（同址 `0x42b66000`，翻转变量未识别） | ❌ 0x91（`0xd8ad1000`） | ❌ 0x91 | — | — | — |
| v2.1.2/2.1.3 | **真 >4GB**（GetMemoryMap+AllocateAddress）；MEMMAP 采集；GOP A/B marker | — | ✅（含 GOP 断开 marker 亦成） | ✅ **（决定变量=真 >4GB，+80%）** | ❌ 0x91（真 `0x100027000`） | — | — | — |
| v2.1.4 | `tu10x-fb-gb.txt` 手工 fb 覆盖 | — | — | — | ❌ 10/22/11 GiB 三轮全 0x91 | — | — | — |
| v2.1.5/2.1.6 | heap 档→**驱动公式**（钳位 [88,280]MiB）+ `LOCAL_MEMORY_RANGE` 寄存器解码；heap tiers；非递归 reconnect | — | — | — | ❌（几何逐位正确仍 0x91） | ❌（0x11/>4GB、0x91/<4GB；自制 24G heap 补丁） | ❌（8 轮单变量全 0x91） | ❌（0x0B，四轮一致） |
| v2.1.7-tu102-diag（marker 矩阵） | A：C-load 三点；B：+PTIMER+host-probe；C：v67-direct-ss | — | — | — | ❌ A/B 轮 | — | ❌ A/B/C 轮 | — |
| PR #67 head（D/E/F） | 搬安全段基址 / BOOTVEC 直入 / secure=false | — | — | — | — | — | ❌ 但 **EXCI/异常类首次改变**（`0x00900100`→`0x00a00100`、`0x91`→`0x31`；secure=false → `0x50aa2863` 且 DMEM 未动） | — |
| 全 C 链（chenxluo / PR #49 / travis guarded） | 见 A3 | — | — | — | ✅（PR #49，本机本卡） | — | — | — |

**结论**：每一台失败机的失败是跨版本确定性的（#17 七个版本、#62 八轮+六轮、ProxVZ 四轮、#57 三轮），排除"某版回归"；两台早期失败机（#14/#16）分别在 v2.1.1、v2.1.3 处确定性离场，且离场变量互不相同——**同一个 0x91 收口码覆盖了至少两种已被修复的机器条件 + 一种至今未破的硬核条件**。

---

## A3. C code reference 消融表（排除了什么 / 还差哪几项）

C 参考谱系：chenxluo/tu102-22g-uefi-unlock（#17 卡原生成功）→ PR #49（Rust 移植，同卡成功）→ travis guarded（MSI 卡成功）。main 已逐项复刻并单变量验证的成分：

| C 链成分 | main 复刻处 | 现场结果 | 判定 |
|---|---|---|---|
| 22G heap 档 → 驱动公式 `8+align1MiB(96K×GiB)+96`（钳位 [88,280]MiB） | v2.1.6（`afd6b7c`/`86700e9`），`wpr_meta_pack` tu10x 分支 | #17：`LOCAL_MEMORY_RANGE 0x2c9→22528MiB`、heap `0x6b00000`、meta 11 字段与 C 成功值逐位一致——仍 0x91 | **非充分**（几何层整体排除） |
| C-load 点火三点：不 reset / TRANSCFG 不动 / 3s 零 MMIO 静默窗 | `\tx-sec2-c-load.txt`（`3bb069b`，`booter_fire_direct_c`） | A 轮：#17 与 #62 均 `0x91`/`exci=0x00900100` | **非充分** |
| + PTIMER seed（月初+时分秒，保留"漏日"怪癖） | `\tx-ptimer-seed.txt`（`3bb069b`） | B 轮：两卡均 0x91 | **非充分** |
| + host-probe 写序列（PLM/SS0/SS1 探测写+settle） | `\tx-host-probe.txt`（`3bb069b`） | B 轮：0x91（#62 上 settle 写后 PLM 读回 `0xfffffe8e`，位翻转行为与 40HX 不同） | **非充分** |
| v67-direct-ss 载荷（CMP 血统 `de7288bc…` 替换 four-plm `b0ec9029…`） | `\v67-direct-ss.bin`（`309d949`） | C 轮：0x91；#67 的可证伪预测应验（载荷是 booter 起来后自己 DMA 的，崩溃在入口处够不着） | **非充分（载荷线整体划掉）** |
| wbinvd + portal 读回校验 + 单次 START 无 alias 分支 | C 轮路径已含 | 同上 | 已在 C 轮组合内 |

**尚未在 main 复刻的 C 链成分（在册嫌疑，PR #49 / travis overlay 独有）**：

| 嫌疑项 | PR #49 实现 | 为何仍可疑 |
|---|---|---|
| **Reserved-memtype 高位分槽 staging** | `dma_alloc_tu102_compat`：`AllocateType::Address` + `MemoryType::RESERVED`，候选从 `0x110000000` 起 +16MiB 步进×12 槽，不降级低址 | main 的高位分配用 `EfiBootServicesData` 且落在描述符起始；#49 成功轮 meta 在 `0x11882b000`（第 7 槽），main A 轮在 `0x10001f000`——地址与 memtype 双双不同 |
| **全零 dummy RM 镜像**（`0x1bfadc8`） | 内存生成全零 fwimage | main 用真实 `fwimage_tu10x.bin`（29MB）；booter 对 GSP-RM 镜像有无预读/校验未知 |
| **FWSEC 完成判据 = 精确 WPR2 + GSP 进入 HALT** | preflight 断言双条件，"WPR2 出现不能充当完成信号" | **已定位为头号嫌疑（2026-10-31，见 [tu10x-c-parity-diff.md](tu10x-c-parity-diff.md) B0/B0.1）：三个成功实现 + main 自己的 GA10x/Ada 路径 + **内核本体**（`kgspExecuteHsFalcon_TU102`/`kgspExecuteFwsec_TU102`：HALT 后必查 mailbox/scratch0e/WPR2 精确 lo）全部等停机并读结果；唯独 main tu10x 路径 WPR2 一中（0-3ms）即释放 ~40KB staging 并继续、从不读 FWSEC 结果——机制候选：提前点火（主）+ 页复用（窄窗）** |
| **bootBin/gspFw 指向 VRAM** | meta：`gspFw=0x57e1f0000`、`bootBin=0x57fdfa000`、`wprEnd=0x57ff00000` | main 的 boot/gsp staging 在 sysmem DMA（`\tx-meta-in-vram.txt` 基建存在但未用于此） |
| 身份/PROM CRC/资产 SHA 钉死 + 一次性授权 | PR #49 preflight / travis guard crate | 弱嫌疑（不改硬件行为，但排除法上未做） |
| DMA 生命周期：固件执行期不释放、双 HALT 后才释放 | PR #49 `release_after_handoff` | 弱嫌疑（main 的 release 时序不同） |
| **travis overlay 的非-guard 差分** | fork `2654419e` source-overlay（tx.rs 76KB 等 20 文件） | **未枚举**——它是"base `3bb069b`+C-load"在另一张卡成功的直接证据，与 main A 轮失败的差分就在其中 → Phase B1 |

travis 成功的额外信息：证明 C-load 成功**不需要** PTIMER 与 host-probe（组合层面）；以及 `exci=0x00900100` 出现在成功链中（非判据）。

---

## A4. 还差什么（缺口清单，按优先级）

1. ~~成功实现与 main 的未枚举差分~~ **已坐实（2026-10-31）**：见 [tu10x-c-parity-diff.md](tu10x-c-parity-diff.md)。三方成功实现的公共差集 = {FWSEC GSP-HALT 完成判据 + staging 延迟释放（头号）、跳过 pm_preflight、跳过 BridgeMaster 写、跳过运行时 PROM 读}；消融 marker 优先级清单在彼文 B4。nlzy 的 sub-4GB sysmem staging 成功与 #16 的 sub-4GB 被拒互为对照（sub-4GB 读锁非全机器成立）。
2. ~~mailbox0 残留码产生者未定~~ **已解码（2026-10-31，见 [tu10x-booter-nsstub-disasm.md](tu10x-booter-nsstub-disasm.md)）**：NS 桩在 `lcall 0x100` 前写 **0x31**（进入安全段标记）；0x91/0x11/0x0B 为**安全段 booter 自己的状态码**（0x91=wpr_desc_validate 拒收）→ 硬核四例安全段确实运行过；PR #67 D/E 轮终值 0x31=入口被破坏安全段未跑。EXCI=INV_INS@0x100 是拒收后的自毁残影。
3. ~~EXCI 位域待权威核对~~ **已定（open-gpu-kernel-modules dev_soe_ip.h）**：EXCAUSE[24:20]，9=**INV_INS**、0x0a=MISS_INS、0x10=DMEM_MISS_INS。`0x00900100`=EXPC 0x100+INV_INS（旧"0x10"解读作废）；成功链亦出现该值 → 非独立判据。
4. **安全段放置 / IMEM 写入完整性未检验**：#67 D/E 轮是唯一让异常类动过的轴（EXCAUSE 9→0x0a、0x91→0x31）；main 的 portal 校验只读 2 个字，缺**全段 per-256B 块读回 CRC**（→ Phase D）。
5. **入口状态记录缺失**：WPR2-at-entry（FWSEC ran/skipped）、冷/热启动、staging band/memtype 摘要——成败两组的分布无从统计（→ Phase D 常开日志）。
6. **#14 的 v0.2.0→v2.1.1 确定性翻转变量未识别**（同卡同址 `0x42b66000`，GOP 与地址带已排除，v0.2.0 复跑复现）——历史遗留的未解构建差分，可能是同一机制的另一条线索（v0.2.0..v2.1.1 的行为级 diff 值得回头做）。
7. **成功侧构建身份缺口**：Reddit 双 1e07 用哪个版本/构建未知（若为 v2.1.6 默认路径，则"硬核"分野更尖锐）；#12 仅早期 v0.2.0 形态。
8. **sub-4GB 读锁的机器相关性**：#16（sub-4GB 败→>4GB 成）vs #14（sub-4GB 成）vs nlzy Linux（sub-4GB 成）——cmp50hx v2.63 字典"sub-4GB 读锁→0x91"只在部分机器成立，机制未明。

---

## 机理问答（维护者四问，2026-10-31）

**Q1：之前探索中的修复哪些真有效、哪些陪跑？**

真有效（各救一类机器，缺一不可）：
- **真 >4GB staging**（v2.1.2/2.1.3）：cmp50hx v2.63 字典"sub-4GB sysmem 读被锁→0x91"是真实存在的按机器/卡属性；#16（TU106 1f14 Clevo）被它确定性修好（伪高位必败→真高位必成）。
- **heap 几何正确性**（22G 档→驱动公式 + LOCAL_MEMORY_RANGE 解码）：WprMeta heap 字段必须逐位等于驱动算法，错则 booter 在签名处理前 0x91——这是 C 参考作者实测记录的事实，不是推断；22G/24G 卡没有它永远失败。容量识别与活寄存器读取的判断正确，且 LMR 顺带覆盖 ECC ×15/16。

陪跑（测了但与硬核 0x91 无因果）：
- **多卡 GOP 归属检查（v2.1.1）**：修的是真实 console 误断 bug，但 GOP A/B 双轮全阴；它与 #14 翻转只是同框相关。
- **PTIMER seed / host-probe 写**：B 轮单变量阴，travis 成功链明确不用——退役。
- **v67-direct-ss 载荷谱系**：#67 可证伪预测应验（NS 桩反汇编给出机制：安全段跑不到载荷 DMA）。
- **C-load 三点单变量**：A 轮阴——单独不充分，但阴性正是把嫌疑逼到 GSP-HALT 的必要消融。

结论：修复级联每级修的都是真 bug（各自救一类机器）；硬核四例是全部已知 bug 修完后的剩余集合——同一个 0x91 收口码覆盖至少三种独立成因（sub-4GB 锁、几何错、FWSEC 未收尾）。

**Q2：解不开的卡本质是什么？刻意防护还是意外不兼容？**

判断：**意外，非防护**（置信度较高）。①同一张"解不开"的卡同机换 C 链就解开（#17 卡跑 PR #49、MSI 卡跑 travis 构建）——刻意反制会识别事务内容（未签名 ROP），不可能以毫秒为单位选择性放行；②NVIDIA 在 Turing 上的真实防线（PLM/secure-boot）在所有机器行为一致，而随机器变化的变量（FWSEC 耗时、内存图/页复用、sub-4GB 执法）全是工程事故形状；③真正被利用的"签名 DMA 无界"漏洞在 30/40 系根本没修——连真正的攻击面都没堵，不会单独为 20 系做时序陷阱。为什么只有 20 系：Turing 是这套协议的第一代（BOOT_DIRECT+16B AES 签名、WITH_LOADER FWSEC），隐含前置条件最多；且 30/40 系的链从内核驱动源码移植了正确时序形状——我们自己代码里 GA10x/Ada 的 `fwsec_ignite` 本来就等 HALT，30/40 系从未参赛。

**Q3：C 方案凭什么"知道得比我们多"？为什么遇不到我们的问题？**

它不比我们知道得多，它**结构上免疫**，免疫来自血统：①它转录了内核驱动的时序形状——内核对 FWSEC 的协议就是"START→WaitForHalt→读结果"（B0.1 一手源码），写 Falcon loader 的自然写法就是等它停；"WPR2 寄存器刚出现=完成"这个捷径是我们发明的（动机合理：guard 只需 WPR2，等 HALT 像是浪费时间）。②缓冲生命周期由环境决定：Linux kmod 的 dma_alloc_coherent 活到模块卸载（想提前释放都做不到）；EFI C 版是一次性程序心智（全部分配、跑完、重启），没有清理纪律反而没有早释放 hazard；我们的 Rust 分阶段清理纪律平时是美德，在这里把"镜像已传输"（WPR2 up）误当"固件与 sysmem 窗口两清"。③它也没在别处更聪明：几何我们已逐位 driver-exact、身份校验我们更严；travis 成功构建=我们的代码+4 个小差分——差距是 4 个差分宽，不是另一套科学。

**Q4："没等 FWSEC 收尾"凭什么不是简单时序问题？为什么只有这些卡？Falcon 性能烂？**

它确实是时序问题——但这是**按机器确定性的时序**（竞态在固定平台上稳定复现=嵌入式固件 bug 经典形状），所以看起来像诅咒。不是"Falcon 慢"：同一硅片族性能一致；变量是**固件收尾尾巴 vs 主机准备时长**的相对长度，两头都是主板/配置的函数：①尾巴长度∝显存容量（22G/24G 清理/初始化的内存比 8G 大得多——直接解释大显存卡在失败名单超频出现）+POST 留下的 GSP 时钟+ECC；②主机侧 WPR2-up→SEC2 START 之间隔着 29MB 镜像拷贝+页表构建+~15k 次 MMIO portal 写（数十 ms，快机器更短=窗口更宽）；③对照组不参赛：30/40 系路径本来就等 HALT，跑 C 链的机器根本没有早点火。还有一个此前遗漏的维度：**内核自己给 FWSEC 收尾留的预算是 50ms 启动+2s 完成**（kern_gpu_tu102.c:454-457）——NVIDIA 按"秒级"设计这段等待，我们在 3ms 处前进了。两个子机制（R1/R2 分解）：(b) 提前点火+结果未查（主轴）；(a) 40KB staging 页复用（真实但窄窗）。

## 时序冲突全表（FWSEC → WPR2 → booter，对错两版逐步对照）

> 时间量级：实测值标 [实测]（来源日志）；推断值标 [推断]。固件侧工作清单为推断——FWSEC 镜像与 booter 安全段均为 AES 密文（熵 7.99/7.955），不可离线反汇编；清单依据内核源码注释、寄存器行为与 NS 桩反汇编交叉。

### 正确时序（内核 `kgspExecuteHsFalcon_TU102` + `kgspExecuteFwsec_TU102`；nlzy/C 链/travis/PR#49 逐行同形）

| 步 | 主机侧动作 | 固件侧（GSP Falcon）并行状态 | 这一步在干什么/为什么这个顺序 |
|---|---|---|---|
| 1 | 复位 GSP、装载 BL（0x200B PIO）、BL DESC 写入（codeDma/dataDma 指向 sysmem staging≈40KB）、BOOTVEC=0xfd00 | 复位态、HALT | WITH_LOADER 引导：DESC 告诉 BL 去哪取真镜像 |
| 2 | 写 mailbox（如有参数）→ `CPUCTL=2` START | BL 执行：经 FBIF DMA 把 FWSEC 镜像从 sysmem 搬进 GSP IMEM/DMEM，然后跳进 FWSEC 入口 | BL 是搬运工；搬运完成后固件才存在 |
| 3 | **`kflcnWaitForHalt(GPU_TIMEOUT_DEFAULT)`——原地等停机** | FWSEC 在 heavy-secure 里执行全清单：编程 FRTS → **WPR2 lo/hi 寄存器出现（此时刻≠完成）** → 清 WPR2 窗（0xe0000=896KiB）→ 编程 MMU lockdown 窗 → 设置 GFW/handoff scratch（SB 命令时**降 PLM**）→ 写 scratch0e 结果码 → **HALT** | **核心纪律**：Falcon 协议是"请求→完成"，完成信号=HALT，不是任何中间寄存器。内核给这段留 50ms+2s 预算 [内核源码常量] |
| 4 | HALT 后回读 mailbox0/1 | 已停机，状态冻结 | 读结果通道 |
| 5 | 校验 **scratch0e[31:16] FRTS_ERR_CODE==0**；**WPR2_HI≠0**；**WPR2_LO==frts>>ALIGNMENT（精确）**；错即 NV_ERR_GENERIC 中止 | — | 固件可能停机但带着错误码——不查=盲飞。main 从未执行此步 |
| 6 | （FRTS 命令完成）进入下一阶段（内核：装 GSP-RM；社区链：booter 点火）；**staging 缓冲保持持有到流程结束**（内核/kmod 生命周期；nlzy 到模块退出） | — | 缓冲不早释放：免页复用、免 Unmap 副作用 |
| 7 | SEC2 侧同样纪律（`kgspExecuteCoreResume_TU102`：START→等 BSI_14 handoff DONE→**查 SEC2 mailbox0==0**） | — | 每个引擎一步一确认 |

### 错误时序（main `fwsec_ignite_tu10x` 现状，分歧点标 ★）

| 步 | 主机侧动作 | 固件侧并行状态 | 问题 |
|---|---|---|---|
| 1-2 | 同上（BL/DESC/START，几何与内核一致） | 同上 | — |
| 3' | **1ms 步进轮询 WPR2 lo/hi；lo 匹配+hi≠0 即 break**——实测 0ms（#62/ProxVZ）～3ms（#17 v2.1.6、travis MSI）[实测] | FWSEC 刚完成清单前几项（FRTS/WPR2 寄存器刚写），MMU 锁/scrub/handoff/scratch0e **尚未做**，未 HALT | **★分歧点①（主轴）**：把"WPR2 寄存器出现"当"固件完成"。对照：内核在此处的预算是秒级，我们毫秒级前进 |
| 4' | **立即 `dma.release`（≈40KB staging 回收进 free pool）+ 打印 "WPR2 verified"** | 固件仍在跑 | **★分歧点②**：早释放。后续 5 次 dma_alloc（meta/stock/boot/booter…）在确定性 EFI 分配器下可能拿到同一批页——若固件尾巴仍读 sysmem 则被覆写（子机制 a，窄窗） |
| 5' | （不读 mailbox0/1、不读 scratch0e——**结果通道完全未查**） | 固件可能已把 FRTS 错误码写进 scratch0e | **★分歧点③**：盲飞。内核必查的三件事一件没做 |
| 6' | boot_desc/booter 解析 + 5×dma_alloc + **radix_build：29MB 镜像 memcpy+2 万条页表项** + meta 打包 + portal 装载 **~15k 次 MMIO 写**——WPR2-up→SEC2 START 总计数十 ms（机器速度函数）[推断，依据各段规模] | 固件尾巴继续（长度∝显存容量/时钟/ECC） | 主机侧时长与固件尾巴长度的**相对顺序**=成败裁决 |
| 7' | **SEC2 START（mailbox=meta 地址）** | 尾巴已完→WPR2/MMU 状态定型→booter `wpr_desc_validate` 对账通过→0x0；尾巴未完→对账失败→**0x91**+自毁 INV_INS@0x100（终态 EXCI 残影） | **裁决点**：T(START) vs T(固件 HALT)——同机确定、跨机分裂的骰子 |

### 为什么"同机必败/必成、同型号分裂"

- 每台机器的（固件尾巴时长，主机准备时长）二元组逐 boot 稳定（内存图固定→分配器行为固定；固件工作量固定）→ 竞态结果确定 → "这台机器的 2080S 永远解不开"。
- 二元组随主板固件/内存图/CPU/显存容量变化 → 同型号卡跨机器分裂（1e81、1e07 两对实证）。
- 22G/24G 大显存卡尾巴更长 → 在失败名单超频出现（但 8G #62 证明主机侧足够快也能输）。
- 对照组不参赛：30/40 系路径（我们代码里）本来就等 HALT；跑 C 链的机器无早点火。

## 性能特征指纹（2026-10-31，逐机实测值汇总）

> 数据来源：各 issue 附件日志离线提取（entry SAMPLE/PMCSR、BSI_SCRATCH15、`WPR2 UP after Nms`、DMA alloc 序列、halt 后 mailbox0）。"—"=未取得。

| 组 | 报告人 | 卡（ID/显存/VBIOS） | 平台（CPU/芯片组/内存代） | 核显 | 入口状态 | WPR2-up | FWSEC staging 页复用 | 终值 mailbox0 | BSI_SCRATCH15 |
|---|---|---|---|---|---|---|---|---|---|
| 败 | #17 | 2080Ti 22G / 1e07 / .e9 | R7 7700(Zen4) + B650M / AM5 / DDR5 | 7700 有 iGPU（显示路由未知） | ran（冷） | **3ms** | （v2.1.6 轮 meta 0x10001f000） | 0x91 | handoff=1（值未取） |
| 败 | #57 | TITAN RTX 24G×2 / 1e02 / .0f | R9 5950X(Zen3 16C) + X570-E / AM4 / DDR4 94G | **无核显**（dGPU 直连，双卡） | ran | 3ms | — | 0x11(>4G)/0x91(<4G) | — |
| 败 | #62 | 2080 8G / TU104-400 1e87 / .A5 | B760（Intel 12-14 代平台，CPU 未报） | **无核显**（自述独显直连） | ran | — | — | 0x91 | — |
| 败 | #62尾 ProxVZ | 2080S 8G / 1e81 | R9 3900X(Zen2 12C) + MSI MPG X570 / AM4 | **无核显** | ran（冷） | **0ms** | **10p@0x100000000 → 立即被 15p 复用** | 0x0B | 0x202001bf |
| 成 | #12 | 2060S 8G / 1f06 | AMD B450/X470 系（CPU 未报） | ? | ran | — | — | 0x0 | — |
| 成 | #14 | 2080S 8G / **1e81（与 ProxVZ 同 ID）** | **Intel Alder/Raptor（8086:a704）** | 可有核显 | **ran（冷）** | **3ms** | **10p@0x100000000 → 立即被 15p 复用（与败方逐位同）** | **0x0** | 0x202001bf |
| 成 | #16 | 2070 8G / TU106 1f14 | R7 3700X(Zen2) + Clevo NH5Ax 准系统 / Insyde | **无核显**（dGPU 直连） | ran | — | — | 0.0（v2.1.3，>4GB 修复后） | 0x202001bf |
| 成(事务) | #55 | 2080S 8G / TU104 | i7-10700K + Z370 | 有核显 | ? | ? | ? | 解锁成功（交棒蓝屏另病） | — |
| 成 | travis | 2080Ti 22G / 1e07 / **.35** | Windows 未记录 | ? | ran | 3ms | guarded **不释放**（等 HALT） | 0x0 | **0x202039bf** |
| 成 | Reddit×2 | 2080Ti 1e07 | 未记录 | ? | ? | ? | ? | UNLOCKED | — |

**指纹排除法结论（日志直证，2026-10-31）**：

1. **页复用轴基本出局**：#14 与 ProxVZ 的 DMA 序列逐位相同（staging 10p@0x100000000 释放→15p 分配原地复用→后续布局一致），一成一败——复用本身不构成判别（与"镜像在 WPR2-up 前已进 GSP IMEM"的推理一致）。R2 marker 降级为对照保留。
2. **内存图轴出局**（这两机之间）：高位描述符同（0x100000000+0x40000000）、分配布局同。
3. **主机速度轴反向**：#14（最快主机类）赢、ProxVZ（2019 平台）输——纯"主机快→窗口宽"模型不成立。
4. **BSI handoff 不判别**：三机同 0x202001bf（travis 0x202039bf 是另一张卡/BIOS）。
5. **在册剩余变量=固件侧行为本身**：WPR2-up 0ms（ProxVZ）vs 3ms（#14/#17/travis）是"每台 FWSEC 行为不同"的直接可观测证据（同为 fwsec=ran、同为 8G、同为 1e81 族）；VBIOS/FWSEC profile（#17 .e9 vs travis .35 同 ID 不同 profile）+ POST 留下的引擎状态（3/4 败方无核显、POST 在 dGPU 上跑过 GOP）是当前唯二未排除的载体。22G/24G 的尾巴∝容量效应仍与超频失败一致，但 8G 败例说明存在不依赖容量的第二条入径。
6. **含义**：R1（等 GSP HALT+读结果）仍是头号实验且现在更聚焦——它同时覆盖"尾巴未完"与"结果带错未查"两个剩余机制；R2 仅作对照。

## 第一轮消融结果（R1/R2，2026-10-31 深夜回收）

| 机器 | R1（gsp-halt） | 决定性读数 | 结论 |
|---|---|---|---|
| #62 TU104-400 1e87 | **转绿** | `WPR2 UP after 3ms` → `GSP-HALT gate halted after 156ms`——旧路径在 FWSEC 尾巴中段点火；booter `mailbox0=0x0`、`tu10x_unlock=true`、34.9→69 TFLOPS | **竞态实锤并修复**（v2.1.8 转正默认） |
| #57 TITAN RTX 24G×2 | **双阴性** | 门控 87ms 生效（halt+scratch0e 全过），booter 仍 0x0B/0x11；R2 同形 | 竞态非本机病因——B 类独立存在 |
| #82 2060M 6G 1f15 | **推进未治愈** | 0x91 → **0x0B**；FWSEC 全绿；`BOOTER-AUDIT: ns MATCH / dmem MATCH / sec 首字 0xdead5ec1`；R2 无变化 | 同 B 类 |
| #74 TU102 22G+TB3 | 用户 28 轮自查（第 15-16 轮单变量实锤；**第 17 轮重要修正**） | staging `dev=0x4b782000 [LOW]` ⇒ WPR2 0ms 起；`0x100000000`/`0x110000000` ⇒ `WPR2 never came up`（零残影）；**全低位轮是 V67 模式**：`mailbox0=0x0`=secure-stage 成功但 `PLM still locked - the payload's writes did not happen` → **UEFI 链在该机从未完整跑通**；第 28 轮：签名/radix ≥4G → `BOOTER-C: poll timeout reg=0x8400100`（SEC2 永不停机挂死，非报错）；用户已撤回"邮箱 32 位截断"误读（GA 明文反汇编独立证伪：meta 全 64 位寻址） | **FWSEC/签名/radix staging ≥4GiB 在 TB 链全段不可达**（三种死形：WPR2 不建立/0x89/挂死）；剩余阻塞=Windows 侧 USB4 接管（D 类） |
| #89 2060 12G 1f03 | **v2.1.8 后用户跑通**（11-01） | v2.1.8 低位默认仍死（EXCI 0x009003ff）→ 用户自改 **FWSEC 走 VRAM staging**（BAR1 写+`TRANSCFG4=(t4&!7)|4` LOCAL_FB+BL DESC codeDma/dataDma 指 VRAM）→ `WPR2 UP after 3ms` 全链绿，20.6→33.7 TFLOPS；**meta 必须留 sysmem**（放 VRAM 则 0x89） | A 类第三形态：低位不够、需 VRAM 载具——"sysmem 自测失败→VRAM 兜底"提案待落码 |
| #90 2080Ti 11G X79 | 判读纠偏 | UEFI 侧实为成功（PLM 开+selector 写入+unlock_persist=true）；Windows 侧病态疑硬件混杂（630 条 WHEA；断电未跑工具仍病态） | E 类：先排硬件 |

**三个跨机定案**：
1. `BOOTER-AUDIT: sec readback MISMATCH(0xdead5ec1/fnv 0056bc76c5)` 在**成功机**（#62 R1）上同样出现且 fnv 逐位一致——Turing 安全段经 NS 读口回读一律 scrub 毒值；**sec 行无判别力，ns/dmem 行才是判据**（v2.1.8 文案已改）。
2. diag 分支 `parse_num_ascii` 吃不掉 `0x` 前缀（"0x200" 无声失效、"512" 可用）——坑了 #57 的 E 轮；v2.1.8 修复+回归测试。
3. `signature DMA never landed` verdict 是 V67 专用检查的恒假阴性（#14 成功轮、#90 实际成功皆有此行）——v2.1.8 文案已改。

**残留类汇总（2026-11-01 更新）**：A=FWSEC 死（staging 带：#74 TB 型已由 v2.1.8 低位默认覆盖；#89 VRAM 型待 FWSEC-VRAM 载具落地）；B=签名/验证域码 0x0B/0x11（#57/#82；**TU 明文 RE 已定案**——key 族 #6 破解后 TU102 全量反汇编：0x0B=终判 32B 哈希比较失败（PSS 尾，0x37dd）、0x11=期望值表 id、0x91=WPR 窗口 1MiB 几何校验（0x2995），见 keychain §7）；C=0x0+sanitize 误读（文案已修）；D=Windows 侧重编程（#82 实测 GSP 615.78/驱动 617.42 init 重编程 FEAT；成功例均 ≥621/620）；E=Windows 病态疑硬件（#90）。另：#91（Catnip202X）提议 DMEM 内 WprMeta 全字段 dump 与成功机 A/B——可由 v2.1.8 的 BOOTER-AUDIT 框架扩展实现。

## 维护结论（2026-11-01 定案版）

- **0x91 竞态根因已源码级闭环并修复**（keychain §6.5：booter 将描述符与活 WPR2 寄存器重建比对、64 位差值须恰 1 MiB；v2.1.8 默认等 FWSEC HALT+scratch0e 门控）。#62 已实证转绿。
- "NS 桩取指 miss"旧工作假设作废：EXCI=0x00900100（INV_INS@0x100）是 booter 拒收后的自毁残影（GA 系干净 exit 对照证实 TU 自毁为 TU 代际行为），成功链亦有该值。
- 剩余 OPEN：B 类 TU 精确映射（等 TU debug key）；A 类 VRAM 载具；D 类驱动版本敏感；E 类硬件隔离；#35 Code 12；#55 交棒蓝屏。
- 复现工具链：GA 系明文钥匙公开（keychain §2 指针）；TU key 猜解进行中（round-3 报告）。
