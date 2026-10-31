# 必读清单进度（任务书 §4）

> 状态补注（2026-10-28）：v0.2.0 已启用 Turing 路径，当前 proven boot 输入为 24576B，见 [Turing 前置与资产](faq.md#turing)；下文“仍门控”和 4096B boot 段是早期状态。现行装载使用 portal；host 描述符支持 V2/V3，V1 待实现，见[实现边界](faq.md#safety)。

状态截至 2026-10-26：M0–M4 已闭环（v0.1.3 已发布；28e0 1.65×、28a1 1.7~1.9× 实测）。
里程碑门控的待读分组随对应里程碑完成而失效，保留作为执行记录；Turing 考据与
上游对齐两节仍为现行参考。

## 已读（本会话）

1. **`gpu-unlock/docs/How Ampere was unlocked.md`**（193 行，全文）——引导链六步时序
   （PCI→固件取载→devinit→FWSEC-FRTS/WPR2→SEC2 Booter→GSP-RM/RPC）；攻击点=booter 无界
   DMA 栈溢出→ROP；debug 微码用公开测试密钥（key index 37）加密→可反汇编挖 gadget；
   V67 载荷=单次任意写（MEMORY[addr]=value）；PLM 模型与 SS0/SS1 语义。
2. **`cmp170hx/docs/unlock/overview.md`**（211 行，全文）——四写 PLM 表（WPR_CFG 0xfffff0ff
   注意 bits[11:8] 留空、FBPA/WPR/FEAT=0xffffffff）；SS0=0x88888888/SS1=8；成功判据
   FEAT_READOUT_1(0x823818)=0（锁定卡读 0x016db6ed）；显存几何 CFG1/LMR 编码按 SKU；
   为什么成立三事实（总熔丝未烧/GA100 载 Turing 固件/debug 与 prod 同明文码）；
   实测收益表（FP32 31x、BF16 tensor 6.4→164-192 TF/s）。
3. **`cmp170hx/docs/unlock/privilege-level-masks.md`**（§1–§4 精读）——PLM 位编码
   [3:0]读层/[7:4]写层/[23:8]源使能；**0x000000FF 不是"开"**（源使能=0 全堵）；PLM 拒绝
   越界值（暴力枚举受硬件编码域约束）；锁定基线 0xFFFFFF8F 在全系 Ampere 一致；开 PLM 循环
   逐项最多 2 次尝试、每次点火前重臂 WPR2 lo/hi、**成功=读回精确相等**（booter 恒报 0x31 无意义）。

## 待读（按里程碑归组）

### M1 前（少量）
- TOTORONG HF 模型卡 + `docs/INSTALL.en.md`/`ANALYSIS.en.md`（EFI 载具交互范式细节）

### M2 前（全部）
- `gpu-unlock/kernel/gpu_unlock/unlock_610.c` 逐段
- `gpu-unlock/tests/test_unlock_adapter.c`（mock 模式→Rust 移植蓝本）
- `cmp170hx/docs/unlock/{how-it-works,falcon-and-booter,rop-chain}.md` 剩余部分
- `gpu-unlock-ada/docs/ada-port.md` + `rtx4090-register-report.md` + `tools/*.py`
- `open-gpu-kernel-modules`: `kgspBootGspRm` / `kgspPopulateWprMeta_TU102` / `kernel_gsp_fwsec.c`
- `cmp170hx/docs/unlock/register-reference.md`（冷值表对照）

### M3 前
- `gpu-unlock-bar1/docs/bar1-64g-report.md`（若涉及 BAR1）
- gpu-unlock packaging/boot（initramfs 时序，理解"驱动前"窗口的 Linux 侧实现）

## 已有外部知识衔接

- NVOC 侧 TU106 VBIOS FWSEC 解码（`docs/reverse-engineering/vbios/tu106-fwsec-ucode-decode.md`）
  已给出 V2 描述符几何（IMEM 0x9600/DMEM 0x410/adjust 0x11600）——M2 资产管线直接复用。

## Turing 引导路径考据（2026-10-26，钉版 610.43.03 源码一手，evidence-as-commits）

TU106 资产完成度推进时把 Turing 专属源码逐文件读完（`assets/in/` 钉版包内，
文件行号引用见下）。结论与 GA10x 社区链的**四处代际差异**，全部已落进代码：

1. **BooterLoad = BOOT_DIRECT，不是 BOOT_FROM_HS**（`kernel_gsp_tu102.c:79`
   `falconConfig.bBootFromHs = NV_FALSE`；GA102 为 TRUE）。TU102 BooterLoad
   BINDATA：SIG_PROD 仅 **16 字节**（非 RSA-384 表）、NUM_SIGS=1、ucode_id=**0xd**、
   patch@0x8700、image 59136=0xE700。内核侧照常 16B 签名补丁（kernel_gsp_booter.c
   `s_patchBooterUcodeSignature`，sigSize=total/numSigs，无 384 硬编码）。
2. **点火 = 寄存器 portal 直写，无 HS bootstrap、无 Falcon DMA 队列**
   （`kernel_gsp_falcon_tu102.c` s_prepareHsFalconDirect）：IMEM NS 段
   （fields[0:2]）落在 0、**secure 段（fields[5:6]）落在 ALIGN_UP(ns,256)**、
   DMEM（fields[2:3]）落 0；BOOTVEC=0；邮箱0/1=WprMeta 物理地址 lo/hi；
   start→HALT→**mailbox0==0 即成功**。这正是我们 AD107 上实证可用的 portal
   路径（tx_portal.rs）——Turing 内核自己就是用 portal 装载 booter 的。
3. **GspRmBoot 描述符 = RM_RISCV_UCODE_DESC v4（76 字节）**（rmRiscvUcode.h；
   TU102 实测 d0=4、bootloader 0..0x488、param 0x488+0x10、monitor/data/code
   全 0）。WprMeta 的 bootloader Code/Data/Manifest 三个字段=desc d12/d10/d8
   （TU102 上即 0/0/0），verbatim 拷贝即可。GA10x 是 v5（84B）。
4. **FWSEC-FRTS = BOOT_WITH_LOADER**（kernel_gsp_frts_tu102.c:411 分支）：
   经 **generic BL**（ksec2GetBinArchiveBlUcode TU102 BINDATA）装载——
   RM_FLCN_BL_DMEM_DESC 放 DMEM 0，BL 镜像放 IMEM 顶，BOOTVEC=blStartTag<<8
   （s_setupLoader）。与 VBIOS V2 描述符 BOOT_WITH_LOADER 的 M1 结论互证。
   WPR2 建立判据=NV_PFB_PRI_MMU_WPR2_ADDR_HI.VAL≠0（同我们的 0x1fa828）。

**不变量（可复用 GA10x 机制）**：GspFwWprMeta 256B 同构同 magic
（gsp_fw_wpr_meta.h:211 `0xdc3aae21371a60b3`、revision 1）；布局算法
kgspPopulateWprMeta_TU102 与 layout_610.c 同族（vgaWorkspaceOffset 从
mmuLockLo 取 min——我们 MMU_LOCK 冷值可读正好喂它）；kgspResetHw_TU102
止于 RESET_FALSE（§9.2 已知坑一致）。

**资产管线（tu10x_610 pins 已冻结复验）**：booter_load_tu10x.bin 59224B
（BTL1，内含 59136 图像+16B 签名）、gsp_boot_tu10x.bin 4096B（RISC-V
boot 段）、gsp_boot_desc_tu10x.bin 76B、fwimage_tu10x.bin 29339080B +
fwsignature_tu10x.bin 4096B（gsp_tu10x.bin ELF 抽取）。

**TU106 事务仍门控**：main.rs tx_phase 对 family=='tu10x' 在 run_transaction
前拒绝（"Turing transaction track not enabled yet"）——GA10x HS 链
（falcon_hs/FWSEC HS 路径）在 Turing 上是错误序列；启用前需把 tx.rs 的
fwsec 段换成 WITH_LOADER/generic-BL 变体 + BSI SCRATCH_15 handoff 等待
（kernel 用 SCRATCH_14 bit BOOT_STAGE_3_HANDOFF 判 reload 完成；冷态
scrubber 状态在 SCRATCH_15 bits31:29）。booter_fire_direct 已就位待接线。

## gpu-unlock-ada 上游更新对齐（2026-10-26，仓库更新至 d28c485）

上游 AD107 实验轨全面展开：28e0（4060 Mobile）/28a0/2882（4060 桌面）/
2803/2805/28e1 等全部在 experimental_ids 表内（各自 opt-in + 精确地址），
且 **28e1 有完整实测报告**（四写链在 AD107M 真卡 2.00× 提速，含稳定性
验证与真实负载 roofline）。链代码（unlock_610.c）自我们移植基线仅设备
表路由变化，无链条逻辑改动。

**交叉验证（我们 M1 独立测量 vs 28e1 报告）**：SS0/SS1 引导漂移 ✓、
WPR2_HI=0 入口判据 ✓、FBPA 停 0xffffff0f ✓、FEAT_READOUT 字段 2/5/7=2
（0x00410080 逐位吻合）✓、VBIOS 1MiB aperture ✓、四写值
（WPR_CFG=0xfffff0ff/WPR/FEAT=0xffffffff）✓、post-FLR selector 存活 ✓、
615.78.08 驱动版本解耦 ✓。AD107M 家族可行性从"我们 M1 独家推断"升级为
"社区实测证实"——用户的 28e0 与验证的 28e1 同芯片同厂（ASUS）。

**对齐修正（f26362ee）**：
1. payload 模式 FBPA 判据：0xffffff0f（三卡实测停止值）与 0xffffffff
   均接受——原判据会在真卡上假失败；
2. scrubber 改非致命尝试：unlock_610.c 原文 "GA102 has no scrubber"，
   社区全部验证卡（含 28e1）无 scrubber 步骤即完成链——我们尝试后无论
   结果都继续（verdict 留日志），链条是否需要跨过门控态由实机裁决；
3. EALREADY 改记 skip（成功类，da96e3f 语义）且不 FLR——卡已解锁时
   FLR 会抹掉活的解锁。

## NvStrapsReBar 考据（2026-10-27，terminatorul/NvStrapsReBar @ master，已 clone 入 reference/）

ReBarUEFI（xCuri0，MIT）的 Turing 专用 fork，目标=RTX 20xx / GTX 16xx 的
ReBAR（issue #1 有社区验证卡列表）。**推翻我们此前"Turing BAR1 不可扩"的
判断**——Turing BAR1 可扩且社区量产验证，机制与 AD10x 的 CYA/CFG@SEC2 完全
不同代（再次印证"跨族移植=重抽重审"）。

**GPU 侧机制（PSTRAPS，envytools hw/io/pstraps.html 权威）**：
- 寄存器：BAR0+0x101000（PSTRAPS，nvidia 内部名 PEXTDEV）。SET0 primary
  @+0x0、SET1 primary @+0xc；bit31=override-enable（0=复位回 ROM straps；
  ROM 载荷在 VBIOS 0x58/0x5c/0x60/0x64）；写入**立即生效**（envytools 原文
  "starts being used by the card immediately"）。
- 位段（G80+，NvStrapsReBar SetupNvStraps.c 常量逐一吻合）：**BAR1 尺寸=
  SET0[15:14]+SET1[22:20] 和值**（0=64MiB，逐级倍增，10=64GiB）；
  SET1[19:17]=BAR0 尺寸（16MiB<<idx）；SET1[23]=BAR3 尺寸（0=2×BAR0）。
  出厂 2070 BAR1=256MiB 应解码为和值 2。
- 写法：改字段+置 bit31，读回确认（GpuStrapsConfirm/NoConfirm 状态变量
  ——与我们"读回是唯一判据"同款纪律）；BAR0 基址须 <4G（直接 MMIO 写）。

**平台侧机制（载具=主板 UEFI 镜像内的 DXE 驱动，需刷主板）**：hook
`EFI_PCI_HOST_BRIDGE_RESOURCE_ALLOCATION_PROTOCOL.PreprocessController`
（ReBarUEFI 机制），在**枚举期**改 BAR 尺寸掩码/ReBAR cap sizing，让枚举器
按新尺寸分窗口；无 ReBAR 主板需 Above 4G Decoding + 关 CSM；S3 有重放脚本
（S3ResumeScript.c）；改 BIOS setup 或换硬件后配置自动失效（依赖 BAR0 分配
地址），另有 CMOS 日期探保护。前提=GPU class 须为 VGA controller。

**对我们项目的意义**：
1. 只读半场立即成立：M1 读 0x101000/0x10100c 即可判断本卡 straps 可读性与
   出厂 BAR1 编码（sample.rs 已落地，2026-10-27）。
2. 写半场载具不同：他们在枚举**前**（DXE 驱动）改，PCI 窗口由枚举器分配；
   我们的 EFI app 在枚举**后**，改 straps 后需自重配 BAR1 config+root
   bridge aperture——风险/收益与解锁使命不匹配，记录不立项，用户要则单独立项。
3. Turing 工厂钳制问题重开（开放问题 #10）：Turing CMP 存在（30HX=TU116、
   40HX=TU106 马甲），任务书 §0"20 系无已知 segmentation"前提需按此修正；
   40HX 与本卡 1f07 同 die 族——M3 payload 模式的 FACTORY FEAT_READOUT_1
   采样点（tx_booter.rs 已落地）就是为此准备的窗口。

## Turing 软解锁谱系考据（2026-10-27 网络搜证，四个仓库已 clone 入 reference/）

**结论：Turing 软解锁先例存在且成谱系，TU106 上有实机证明。** 源头=
Jon Pry "A Canary in the Crypto Mine"（V67 canary 漏洞，DOI
10.5281/zenodo.20916112）→ CMP 90HX（GA102）→ **PZH1gdmu/CMP40HX-Unlock
v3.0.0（TU106，unlock40x_v70.c，MIT，仓库已删）** → 分四支：

| 仓库 | 芯片/形态 | 状态 |
|---|---|---|
| xrip/cmp50hx-unlock（含 efi-unlock/） | TU102（50HX 1e09），UEFI app + kmod | **实机闭环 2026-09-10**：FP32 0.427→13.512 TF/s（**31.7×**）、DP4A **28.4×**，stock 驱动零补丁零 Xid |
| haohervchb/Turing-unlock | TU102（ASUS 2080 Ti），GNU-EFI 载荷，v4 实测一次成功 | 结果 JSONL + VALIDATION/BOOT-STATUS 文档在库 |
| nlzy/turing-unlock | TU102（Dell 2080 Ti 22GB 1e07），Zagitta gpu-unlock 官方 Turing 改编，kmod | 驱动 575.51.03，FP16 张量 ~1.6×（白皮书峰值 88%→160%） |
| KohakuBlueleaf/KohakUnlock | Turing，boot-hook 形态 | offline tests + ROM 分析在库 |

**钳制谱按 SKU 不同**（关键认知，勿跨卡外推）：50HX=FP32-FMA+DP4A 深钳
（FP16 half2/INT32/带宽不钳）；GeForce TU102=FP16 张量半速；AD107M（我们
M1）=READOUT 字段 2/5/7=2 约 1/4；**TU106 consumer=未知，等 M3 FACTORY
READOUT 或 Windows 跑分旁证**。

**解锁协议（cmp50hx efi-unlock/unlock50x_v1.c 头注，"proved on hardware
by the 40HX project"）**：find card → BAR0 → GFW wait+PTIMER seed →
alloc >4GB → fake GSP fw image + radix3 + WPR meta（V67 载荷作签名）→
kill GFW → [FWSEC fallback] → SEC2 Booter load → canary ROP 开 FECS PLM
→ SS0=0x88888888/SS1=0x8 → SEC2 sanitize → chainload OS——与我们 tx.rs
的 claim→gfw→fwsec→booter→selectors→flr 逐段同构。

**TU106 相关硬事实**：
- 40HX/50HX 共用 **TU102 booter 归档**（kgspGetBinArchiveBooterLoadUcode_
  TU102，nv616，ucodeId=0xD，全 blob 0xE700/appCode 0x8400/osData
  0x8500/0x6200，SIG_PROD patch @0x8700）——Turing 家族单 booter 镜像，
  gadget 几何大概率族内通用（与 GA10x 的逐芯片 booter 不同）；
- FWSEC 按板自抽（V2 描述符几何 40HX/50HX 全同，仅 FRTS/WPR2 随显存尺寸
  变——印证 §7.3 V2 布局）；chipId0：TU102=0x162000A1/TU106=0x166000A1
  （与我们 M1 采样一致）；
- nlzy 文档给出 TU102 gadget/栈几何表（WprMeta@0x400、sigDMA@0x600、
  payload 0xfa00、fill 0x4a7、canary 0x6140、reg_write_indirect@0x10aa…）；
  decompil/ 还带 gsp_tu10x_610.43.03.elf.i64（TU10x GSP IDA 库，318MB LFS）。

**平台工艺可借鉴**：Windows 侧=BCD BootOrder 头插 + bootmgfw 链载（40HX
工具原样）；AGESA/AMD APU 主机要走 CF8-only 找卡路径（RootBridgeIo 会挂）；
PCIe Gen2 重训绝不在 pre-OS 做（X99/X299 挂死，OS 侧 BYOVD 做）；他们注
"OpenVolume from a loaded app hard-hangs whole firmware families"（我们
的 loader.log 用 SFS 在自己的机器上没出过事——平台相关，留档）；Windows
下装完设 EnableGpuFirmware=1 确保 GSP 路径接受 pre-OS 态。

**对我们的影响**：M3 的"无载荷链"技术不确定性大幅下降（TU106 全链已由
40HX 项目实机证明）；M2 资产清单更新——booter_load_tu10x=nv616 TU102
归档 blob（钉 610.43.03），fwsec=自 PROM 抽（M2 已计划），fwsignature
=TU10x 签名段；开放问题 #10 的关闭路径不变但风险大降。


## ReBarUEFI 考据（2026-11-01，xCuri0/ReBarUEFI @ master 9c80fdb，已 clone 入 reference/）

ReBarUEFI（0.1–0.3 tag）= NvStrapsReBar 的上游本体：同一个 DXE 载具
（hook `EFI_PCI_HOST_BRIDGE_RESOURCE_ALLOCATION_PROTOCOL.PreprocessController`，
EfiPciBeforeResourceCollection 相位里"EDK2 PciBusDxe setups Resizable BAR
twice so we will do same"），差别全部在**设备侧旋钮**：

**为什么分版本（ReBarUEFI vs NvStrapsReBar）= BAR1 尺寸旋钮随代际换位置**：

| 代际 | 旋钮 | 可达性 | 载具 |
|---|---|---|---|
| RTX 30 系+（Ampere+）/现代 AMD | PCIe ReBAR 扩展能力（cap ID 0x15）：`PCI_REBAR_CAP` 广播支持尺寸、`PCI_REBAR_CTRL` 写选定值 | 配置空间，PL0 可写，**设备自报尺寸域** | ReBarUEFI，328 行 C，完全 GPU 无关（仅一个 AMD cap dword quirk） |
| Turing（20/16 系） | PSTRAPS（PEXTDEV 0x101000，SET0[15:14]+SET1[22:20]）；**cap 0x15 存在但域小**（2026-11-01 TU106 实测修正，见下） | BAR0 MMIO + 配置空间，PL0 可写；广播域由 straps 决定、出厂顶到 256 MiB | NvStrapsReBar fork（strap 改写 + 枚举期载具不变） |
| Pascal- | straps 可写但 ReBAR cap 不存在 | 只能固定大 BAR，Windows 驱动 BSOD | 不支持 resize |
| AD10x 超广播域（64G） | CYA 0x88724 / CFG 0x88dcc | **PLM 锁死，SEC2 heavy-secure 内写** | gpu-unlock-bar1 谱系（本仓库 M5 线） |

即同一"改 BAR1"目标在四代硬件上是四种机制——旋钮从配置空间（自报域内）
→ straps（自报域外但 PL0 可达）→ PLM 锁死（必须让已签固件代写），与
PLM 模型预测严格同构，是"跨族移植=重抽重审"的又一实例。

**ReBAR 实测修正（2026-11-01，TU106 1f07 field round，ssdiag v0.5.0；完整日志
[docs/evidence/tu106-1f07/ssdiag-v050-rebar.log](evidence/tu106-1f07/ssdiag-v050-rebar.log)）**：
本机 2070 的 config space **确实有** ReBAR cap（@0x0bb0，3 个 resizable BAR）
——"Turing 无 cap"的先验错了，Turing 的限制是**广播域小**：BAR1 支持
{64M/128M/256M}（当前 256 MiB，与活窗 0xc0000000-0xcfffffff 精确吻合）、
BAR0 {16M}、BAR3 {32M}。 Ampere 同 cap 结构但域顶到 32G+——两代差异是
**域宽**，不是能力有无。随之 **PSTRAPS 之谜闭环**：straps 字段读数
（和值 0 = 64 MiB）是**上电默认尺寸**，活窗 256 MiB 来自 vBIOS/驱动在
枚举期经 cap 的 CTRL 写（size_idx 8）——"出厂 2070 = 和值 2"的旧预期
作废，straps 与活窗本就不必相等。这也解释了 NvStrapsReBar 为何必须
在枚举前改 straps：straps 决定 cap 广播域，域不放大，CTRL 写不进更
大值；枚举期是唯一窗口。解码单位两处笔误（字节数标 MiB）已修
（62c2a67，loader+ssdiag 同款）。

**对我们的增量信息**：
1. **枚举期是硬约束**：resize 必须发生在 `PreprocessController`
   （枚举相位内），PciBus 同轮分窗口；EFI **app**（我们）运行在枚举后，
   改 BAR 后需自重配 root bridge aperture——把 NvStrapsReBar 节"记录不
   立项"的裁决机制化。若未来真要做，唯一正路是 DXE 驱动载具，不是 app。
2. **M1 探测纠错（已落地，eb2bff5）**：对照 ReBarUEFI/Linux 布局发现
   M0 时代的 ext-cap ReBAR 解码（pci.rs，自初始 commit 未复核）读错了
   寄存器——BAR 数量取自 CAP bits[4:0]（那是支持尺寸位图）、尺寸取自
   下一对 CAP 低位，步进/字段全错位。已改为 {CAP,CTRL} 对链正确解码
   （首对 CTRL[7:5]=BAR 数、CTRL[2:0]=序号、CTRL[12:8]=当前尺寸
   1MiB<<n、CAP=支持位图 raw bit n=2^n MB），并新增支持域打印。**ssdiag
   v0.5.0 已跟进**（6e0521a）：pmxdrv 只有 map/unmap 物理，但 config space
   在现代 x64 本就是内存映射（MCFG/ECAM）——从注册表 LocationInformation
   取 bdf（取前三个整数 run，跨语言格式串参数序恒定）、GetSystemFirmwareTable
   读 MCFG、只映射目标函数自己的 4KB config 页、VID/DID 对 DeviceID 校验后
   再解码，不做任何总线扫描；停在 BAR0 walk 之前（停泊卡 config 仍可答）。
   diag-unlock.py（Linux sysfs config）仍未加，易做。
3. **GA10x + A4G/ReBAR 板的 BAR1 大孔径**对 `\tx-meta-in-vram` 的
   BAR1 批量拷贝是纯加速（84MB 少换窗）；与 booter 0x91 的 >4GB staging
   需求**正交**（booter 读 sysmem 走设备 DMA，不经 BAR1 孔径）——勿混。
4. ReBarState 的 NVRAM 变量 + CMOS 复位年检（before BUILD_YEAR 即静默
   禁用）是"防御性默认关"范式；我们用 U 盘 marker 文件（宪法不动
   NVRAM），两条路线互为对照。
5. UEFIPatch 补丁目录 = 平台固件大 BAR 限制分类学（<4G/<16G/<64G 上限、
   MMIO 39/36-bit 扩充、64 位 BAR 降级防护、X79 A4G fix）——#88 裁决
   "A4G 非硬前提"的用户侧补充文档：即使开了 A4G，固件处理大 BAR 的能
   力仍是独立变量。
6. Linux 免刷 BIOS 路径（4G decoding + `pci=realloc` 自动 resize）——
   RTD3/Linux 线（issue #9）的背景资料。

## 消费卡 PCIe P2P 考据（2026-11-01，四源交叉 + open-gpu-kernel-modules 一手源码）

**用户问题**：rtx-p2p（kacper-daftcode）/ forkProj open-gpu-kernel-modules-P2P /
club-3090 #70 / harrychen 5090 博文这类"消费卡 P2P 解锁"，能否在我们层级
（EFI pre-driver）打穿？EFI 是否天生优于系统内驱动？

**裁决：打不穿主体，且此处层级优势恰好反转。** 判别标准：我们的层级优势
只在"门=PLM 保护的**持久硅态**"上成立（FEAT/SM 速率：EFI 经签名固件 ROP
写入，驱动只是发现者）。P2P 的门是**运行期策略**，不是硅态：

### 门的三层解剖

1. **GSP-RM 固件 SKU 门（主体）**：GeForce feature gate + ebreak 崩溃守卫
   压住专业卡代码路径（P2P setup 是其中之一），全在闭源 `gsp_elf_*.bin` 内。
   rtx-p2p 用 Ghidra RISC-V RE 出固件内 ~850 键注册表解析器，其中
   `RMDisableFeatureDisablement=1` 一个键同时绕过 SKU 门与 ebreak 守卫——
   **该键在 open-gpu-kernel-modules 全树零命中（2026-11-01 实查）**，纯固件
   内键。kernel 侧 `p2pGetCapsStatus`（src/nvidia/src/kernel/platform/p2p/
   p2p_caps.c:804）按 C2C→NVLink→PCIe→PCIe BAR1 顺序查询、regkey 可覆盖，
   但 GSP 管理卡（Turing+ 全部）的实际 caps 出自固件。
2. **BAR1 孔径**：消费卡可行传输只有 `P2P_CONNECTIVITY_PCIE_BAR1`
   （p2p_caps.c:105/942）——peer VRAM 经 peer 的 BAR1 窗口访问。
   MAILBOXP2P/`GMMU_APERTURE_PEER` 是 pro 卡硬件（4090 缺；tinygrad fork
   550 用 `kbusEnableStaticBar1Mapping_GH100` 全 VRAM 静态 BAR1 + GMMU
   PEER→SYS_NONCOH 重写绕过；590 起固件侧一键搞定，无需补丁）。全 VRAM
   BAR1=ReBAR：4090/5090 官方支持；Turing/Ampere straps 域受限（TU106 顶
   256 MiB，见 ReBarUEFI 考据节）。
3. **平台层**：ACS/IOMMU 关闭（路由质量）+ NCCL_P2P_LEVEL。

### 为什么 EFI 打不穿

- 门在 GSP-RM **每次 bring-up 从 SKU 身份重算**，非持久寄存器。EFI 事例的
  GSP（M3 自 bring-up）与驱动事例无关——驱动 FLR 后自 bring-up（"驱动永不
  收养 EFI 阶段 GSP 镜像"已实证）。
- 旁路钥匙虽在固件手里，**递送通道是 OS 注册表→驱动→RPC→GSP**；EFI 阶段
  该通道不存在（无 OS、无驱动）。
- Windows 额外一层：WDDM 闭源前端 + CUDA runtime 自查
  （cudaDeviceCanAccessPeer）。四源全是 Linux；Windows 侧 `nvlddmkm`
  RegistryDwords 机制存在但 GSP 是否同样响应未验证（可测，非本项目杠杆）。

### EFI 侧真实杠杆

- **BAR1（唯一 EFI-native 件）**：我们的 PSTRAPS/CTRL 写技术正对路，但
  4090/5090 官方 ReBAR 使其无增益；Turing/Ampere 域小仍是瓶颈；且 3090
  实测 256 MiB BAR1 也能跑（窗口化）→ 只是带宽/覆盖因子。
- ACS config space 理论可写但 OS 重管（社区直接 BIOS 关）→ 顶多 ssdiag 加
  诊断字段，不值。
- **方法论收获（真价值）**：GSP 固件内存在 ~850 键注册表解析器 +
  "feature disablement 总闸"，与 OPT_FEATURE_FUSES 哲学同构。若 M3 自
  bring-up GSP-RM，可向**自己的 GSP 事例**喂任意 registry dwords 做侦察
  （枚举专业特性可翻性）——侦察仪器，非交付机制。

### 各源速记

| 源 | 卡/平台 | 手段 | 结果 |
|---|---|---|---|
| forkProj/open-gpu-kernel-modules-P2P（tinygrad 谱系，550.54.15） | 4090×6 Linux | 内核补丁：强制 GH100 BAR1P2P + GMMU PEER→SYS_NONCOH | 24 GB/s 单向，~50 双向，NCCL 24.5 busbw |
| aikitoria fork（harrychen 8×5090，590.48.01） | 5090×8 EPYC | ~100 行补丁去检查+vtable 换 datacenter 实现；关 ACS/IOMMU、ReBAR | 48.5/97.4 GB/s，NCCL 2.8×；P2P 进程退出后利用率泄漏坑 |
| rtx-p2p（kacper-daftcode，590.48.01） | 5090 | **零补丁**：modprobe 传 RMDisableFeatureDisablement=1 + EnableResizableBar=1（固件内键） | 未给基准数 |
| club-3090 #70（3090 双卡无 NVLink） | 3090×2 | aikitoria 补丁驱动 + NCCL_P2P_LEVEL=PHB | 纯 decode +2%/+9%，spec-decode +22%/+19%（NVLink 对照 +58%） |

**结论**：不立项。与使命（PLM 硅态解锁）正交；受益场景（单机多卡推理）
本 fleet 不具备；Linux 侧方案已三线铺满，Windows 侧闭源 WDDM 是硬天花板。
"运行期策略门 vs 持久硅态门"的判别框架收进裁决工具箱（§10 判例）：拿到
任何"NVIDIA 又锁了 X"的问题，第一步先定位门在哪层——硅态门才轮到 EFI
层级出手。
