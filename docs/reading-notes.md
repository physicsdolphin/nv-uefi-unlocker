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

### Windows 侧假想攻面的工具选择（防误读，2026-11-01 追问后补）

主武器是**用户态注册表写**，非内核写原语：门是配置门不是代码门，旁路钥匙
（RMDisableFeatureDisablement）的递送通道 OS 无关——OS 注册表→RM 注册表
DB→GSP bring-up RPC→固件 ~850 键解析器（osapi.c rm_parse_option_string /
kernel_gsp.c 一手核对）。Windows 等价物=nvlddmkm 服务键 RegistryDwords
（REG_SZ）。决策树：①注册表写+重启→`nvidia-smi topo -p2p` /
cudaDeviceCanAccessPeer（Linux 590 一键已实证，Windows 未验证；3090 旧
GSP ELF 是否同吃该键未证实——club-3090 那批仍在用代码补丁 fork）；
②若 WDDM 闭源前端另有硬检查（未证实：550 时代 gate 在内核代码，590 移进
固件后 Linux 已免补丁，前端更像 caps 转发者）→才轮到 BYOVD 内核写做内存
补丁——RE 闭源驱动+per-version 偏移+每次驱动更新重做，且 pmxdrv 物理
map/unmap 原语打 VA 须先 CR3 页表翻译，工具本就不匹配；③CUDA 用户态
（nvcuda.dll）若有 gate 仍是用户态补丁。本项目 pmxdrv 纪律=只读诊断+用完
即毁，不拿去做第三方签名驱动镜像补丁。

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

## GA100 电压故障注入（VFI）考据（2026-11-11，kate.land 博客 + kate-land/ga100vfi）

来源：[Using voltage glitching to unleash the A100 in my mining
card](https://blog.kate.land/posts/using-voltage-glitching-to-unlock-the-a100/)
（2026-11-10 发布，发布前已报 NVIDIA PSIRT）+ [ga100vfi](https://github.com/kate-land/ga100vfi)
（MIT，单 commit 资源转储：KiCad P8/P9 硬件、STM32G431 固件、`driver/pwmvidctl`
主机工具、`gpu0-registers` Rust 有界采集引擎、`glitch-loop` 循环契约）。谱系：
Jon Pry《Canary in the Crypto Mine》（ROP 开 PLM——与我们同源）+ Amogh
Munikote cmpunlocker（软件侧 GA100 解锁）。本文完成度判据：博客全文通读 +
仓库 README/ALGORITHM/EXAMPLES + 672 词选项字典逐条提取（原料在本地
`.ga100vfi-notes/`，不入 git）。

### 1. 攻击链速记

- **战果**：CMP 170HX 8GB（GA100 马甲，实带 80GB HBM）→ 等效 A100-80G：
  SM 70→120-128（FP16 非张量 85.9 TF = 零售 A100 的 1.18×，因全功能 SM 多于
  零售的 108）、PCIe Gen1→Gen4、NVLink（另需 U11 封装桥接小改）、ECC/MIG 理论可用。
- **熔丝宏结构**（16,384 eFuse = 512 行×32，四区）：regular（行 0-221，含
  安全敏感控制行 row0/1）、**FF fuseless fuse（行 222-487）**、IFF、RIR（厂修）。
  FF 记录 32 位：`[4:0]`=chain、bit5=FF/IFF、`[15:6]`=block、`[31:16]`=data，
  **后写胜出**——追加 FF 行即可覆盖早前记录的有效选项字，物理熔丝不动。
  设计缺陷：吹更多熔丝可以*放松*限制（分段全在 FF 区）。
- **语义门 SBFE**：`NV_FUSE_OPT_SECURED_BURN_FEATURE_ENABLE` 量产恒 1，
  此时熔丝控制器 FSM 对非 FPF 区（行 96-221 之外）的烧写请求**行为完全相同
  但不落熔丝**（静默丢弃——与 PLM 丢弃主机写同构）。ROP 开 PLM 只解决
  "寄存器可达"，SBFE 挡的是"块愿意执行"；HS 上下文也绕不过，纯软件至今无解。
- **Goldilocks 故障态**：三脉冲（900mV 背景 + 三次 2ms 低脉冲 @ ~980ms）精确
  打断 row0/1 控制行*锁存*（原始行 `0x53557c3d` 完好，17 个派生安全别名全零）。
  此时 EFP=0 疑似 NV 的 glitch canary（控制行损坏 → 烧熔丝自动禁用）。突破口
  =`SENSE_CTRL` 重锁存命令的不对称：所有控制行派生字回熔丝值，**唯独
  DISABLE_SW_OVERRIDE 保持 0**（重初始化命令的状态保留漏洞）。DSOV=0 →
  `EN_SW_OVERRIDE` 软件影子机制（CTRL_OPT_*→STATUS_OPT_*）可用 → SBFE
  影子成 0、EFP 回 1、清 `OPT_SECURE_FUSE_WDATA_WR_SECURE` 的 PLM →
  任意 FF 烧写开闸 → 按 die 定制 `_DISABLE` 掩码（对齐 `_DEFECTIVE`）。
- **硬件载体**：STM32G4 板拦截 PWMVID（NV 私有 VRM 数字协议），拔 0402
  0Ω 电阻隔离 die 原始 PWM，让卡自己的 MP2888B VRM 执行攻击波形；NVVDD
  过 225mV 触发 DMA 紧循环；S0→S3→S0 当电源循环（RTC ~3s 唤醒），SBC
  保姆机 GPIO 顶电源键救挂死宿主。战役时长 1-48 小时/卡。
- **UEFI 判定**：VFI 写入面在 PWMVID/NVVDD 物理层、判定面在熔丝控制器上电
  锁存 FSM，两者先于且独立于任何 CPU 代码——**UEFI 既不能发起也防不了**。
  EFI 预驱动窗在 VFI 战役里的角色只有"冷态读出臂"（无驱动竞争的选项字判定），
  现实分工 = 系统内 ssdiag 当每循环读出器 + EFI 冷读做终确认。

### 2. 分层门槛阶梯（本节的核心架构学产出）

| 层 | 门 | 破法 | 例 |
|---|---|---|---|
| L0 | 运行期策略（GSP-RM 固件内键） | 系统内工具 | RMDisableFeatureDisablement（P2P 考据） |
| L1 | 接口门 PLM + 运行期覆盖字 | **EFI 预驱动窗 + ROP（本项目）** | FEAT_OVR_SM_SPD / PLM |
| L2 | 语义门（FSM 拒绝执行） | 仅 VFI | SBFE（本次 GA100VFI） |
| L3 | regular fuse / RIR | 物理层 + 不可逆 | 签名槽选择 |

NV 把消费卡钳制放 L1（运行期字），数据中心卡放 L2/L3（熔丝）——两个市场
反作弊威胁模型的分野。推论：**我们的"易失+每引导重放"不是权宜，而是软件
路线的必然终态**（SBFE 未破前持久化在软件域是死路）；FF 追加语义是唯一
"逻辑可逆的持久"存储（行预算 265 条），修正了"烧熔丝必不可逆"的直觉。

### 3. 跨族寄存器块：GA100 词典即罗塞塔石碑

决定性发现（§11 修订后仍成立但加一层）：`NV_FUSE_OPT_FEATURE_FUSES_OVERRIDE_DISABLE`
在 GA100 词典里 = **0x008203f0**，与任务书为 TU106 记载的
`OPT_FEATURE_FUSES_OVERRIDE_DISABLE` 同址（**文档性同址**——TU106 在该址
从未读出过值，恒 badf5040；Turing 真值面在 −0x7FF000，见 §11）。SM_SPEED_SELECT
九词偏移与 ssdiag BLACKWELL_ROWS 的 FUSE_SS_* 逐位一致（GB20x 消费 die 用
同一布局）。结论（修订）：**熔丝控制器/选项字块是跨族共享 IP，但 bank 基址
分家——GA10x/AD10x/GB20x=0x820xxx、TU10x=0x021xxx（−0x7FF000）**，0x820xxx
在 TU106 上另有解码面（控制词一致、选项词空槽）；FEAT_OVR 运行期块照样
逐族换位（TU106=0x4096xx、GA10x/AD10x=0x823xxx）。词典里另有两族词
有直接解释力：`SM_SPEED_SELECT_{FFMA,FMLA16,FMLA32,DP,IMLA0-4}` 是我们
SS0/SS1 的熔丝级兄弟（同一分段语义两层实现）；130 个 `_WR_SECURE` 词 =
用熔丝定义其它寄存器 PLM 的元配置（FEAT_OVR_PLM 的熔丝版祖先）。

注意反面先例：AD10x 的熔丝 PLM 在 0x8200d0..fc 而 GA100 社区图在
0x820050/54（ssdiag ADA_FUSE_ROWS 注释）——块内布局允许逐族微调，所以
消费 die 上某个词读 badf ≠ 块不存在，可能只是挪了位。

### 4. 实验纪律同构（物理注入与软件 ROP 收敛到同一认识论）

`glitch-loop/ALGORITHM.md` vs 我们的安全宪法，逐条对应：

| 他们的规矩 | 我们的宪法 |
|---|---|
| never replay the uncertain physical issue | #2 STOP 后同会话绝不重试 |
| two matching, independent read-only baselines | #5 每卡钉死身份 |
| never infers a fuse write route from alias values alone | #1 读回是唯一成功判据 |
| classify from validated evidence, preserving partial observations | badf 三桶，垃圾分桶不报"不可读" |
| a reset / new specimen begins a new decision context | 断电→正常引导→再来 |

`gpu0-registers` 的 contract crate 把纪律做进了类型系统：
`AttemptState::permits(next)` 重放门禁、`Eligibility` 枚举**只有 `Unproven`
一个变体**（分析层结构性禁止自动宣布"可烧"）、全链 content-addressed
Digest 收据 + pinned map digest 校验、`NegativeRebootIntent` 负控类型、
capture 收尾恢复熔丝地址与 PCI command（devres 契约的用户态投影）。
Goldilocks 成功判据是纯读回合取（FUSECTRL=IDLE ∧ CRC_STATUS=0x55 ∧
17 别名全零 ∧ DSOV=0 ∧ SBFE=0 ∧ EN_SW_OVERRIDE=1 ∧ 三 PLM 全开 ∧
WDATA_WR_SECURE=0）——我们 FINAL 报告"多寄存器合取、单值永不构成证据"
的范本。SENSE_CTRL 不刷新 DSOV 的重初始化不对称，进 booter/FWSEC 审计
checklist：任何"重新加载/复位"命令逐字段核对哪些状态真被刷新。

### 5. 借鉴与不借鉴

- **借鉴（已落地 ssdiag v0.7.4）**：GA100 词典驱动的跨族只读探测（§6 表），
  一轮回答 TU106/GA106 上这些词的存在性。
- **借鉴（方法论）**：跨 SKU 差分比对 FF 值 vs 有效选项字反推映射（无官方
  映射时的唯一可行建法）；badf20xx 双掩码意识——GA100 实证 floorsweep 分
  `_DISABLE`（分段）与 `_DEFECTIVE`（良率），我们的 badf20xx 桶可能混着
  两者，同 SKU 多卡 A/B 可切分。
- **不借鉴**：物理攻击本身——出圈（宪法 #7 写操作分级）、不可逆风险面、
  与软件交付物定位不符。硬件战役若单独立项，工程参考 = PWMVID 拦截式
  glitcher + S3 循环 + SBC 保姆 + 有界采集引擎。

### 6. 进 ssdiag sweep 的寄存器全表（v0.7.4 实验面）

GA100 名 → 偏移（BAR0）；`NV_FUSE_OPT_` 前缀统一省略。判读法见 §7。

**熔丝控制器核心（GA100 core map）**

| 词 | 偏移 | 语义 |
|---|---|---|
| FUSECTRL | 0x00820000 | FSM 状态（GA100 判 IDLE=0xe0040000） |
| FUSEADDR | 0x00820004 | 行寻址 |
| EN_SW_OVERRIDE | 0x00820040 | 选项字影子总开关（漏洞链支点；GB20x 同位） |
| FUSEDEBUG | 0x00820060 | bit1 切 RIR 宏视图（AD10x 注释同位） |
| CRC_STATUS | 0x00820070 | FF 链解析状态（GA100 判 0x55=双半无误） |
| ENABLE_FUSE_PROGRAM_STATUS | 0x00820078 | EFP 状态（疑似 glitch canary） |
| DISABLE_FUSE_PROGRAM_STATUS | 0x0082007C | （AD10x 表已有 DISABLE_FUSE_PROG 同位） |
| DISABLE_SW_OVERRIDE_STATUS | 0x00820084 | DSOV 状态（SENSE_CTRL 不对称的观察点） |
| FUSEWDATA_PLM / FUSECTRL_PLM / FUSEOPT_PLM | 0x008200F4/F8/FC | 熔丝域自己的 PLM 三件套 |
| RING_GLOBAL_CTL / RING_RESET_PLM | 0x00120060 / 0x001200A0 | PRI 环（GA100 判 RING_GLOBAL_CTL=0） |

**17 个 row0/1 安全别名（goldilocks 清单）+ 2 关键词**

| 词 | 偏移 |
|---|---|
| DEBUG_FEATURE_SW_OVERRIDE_ENABLE | 0x00820148 |
| IST_DEBUG_FA | 0x00820158 |
| PRIV_SEC_EN | 0x00820434 |
| SECURE_PRI_SOURCE_ISOLATION_EN | 0x00820410 |
| PRIVKEY_ACC_DIS | 0x00820324 |
| SFK_ACC_DIS | 0x008205A8 |
| SECURE_PMU_DEBUG_DIS | 0x00820640 |
| SECURE_SECENGINE_DEBUG_DIS | 0x0082063C |
| SECURE_NVDEC_DEBUG_DIS | 0x00820644 |
| SECURE_MINION_DEBUG_DIS | 0x008206FC |
| SECURE_GSP_DEBUG_DIS | 0x0082074C |
| HDCP_RAM_ACCESS_DIS | 0x008207F4 |
| DEBUG_SW_OVERRIDE_ENABLE | 0x00820E20 |
| XV2CLK_RESET_RETIME_DISABLE | 0x00821064 |
| SECURE_FUSE_RESENSE_DISABLE | 0x00821058 |
| SECURED_BURN_FEATURE_ENABLE | 0x00821108 |
| SECURE_XUSB_DEBUG_DIS | 0x00820B98 |
| SECURE_FUSE_WDATA_WR_SECURE | 0x0082048C |
| DEVID_SW_OVERRIDE_DIS | 0x00820584 |

（`FEATURE_FUSES_OVERRIDE_DISABLE` 0x008203F0 已在 PRIV_ROWS，不重复。）

**SM_SPEED_SELECT 熔丝族（SS0/SS1 的熔丝级兄弟；= BLACKWELL_ROWS 同偏移）**

| 词 | 偏移 |
|---|---|
| SM_SPEED_SELECT_DP | 0x00820224 |
| SM_SPEED_SELECT_FFMA | 0x0082059C |
| SM_SPEED_SELECT_FMLA16 | 0x008207D4 |
| SM_SPEED_SELECT_FMLA32 | 0x008207D8 |
| SM_SPEED_SELECT_IMLA0..4 | 0x008207DC/E0/E4/E8/EC |

**STATUS_OPT_\* 有效值读出（20 标量 + 3 数组）**

| 词 | 偏移 | | 词 | 偏移 |
|---|---|---|---|---|
| HALF_FBPA | 0x00820C00 | | FB_CONFIG | 0x00820C34 |
| DISPLAY | 0x00820C04 | | FBP | 0x00820D38 |
| DISP_HEAD | 0x00820C08 | | NVENC | 0x00820D68 |
| NVJPG | 0x00820C0C | | GC6_ISLAND | 0x00820D6C |
| HALF_LTC | 0x00820C10 | | NVLINK | 0x00820DB8 |
| FBIO | 0x00820C14 | | SYS_PIPE | 0x00820E30 |
| FBPA | 0x00820C18 | | OFA | 0x00820E34 |
| GPC | 0x00820C1C | | PES_GPC[0..15] | 0x00820DBC+4n |
| PERLINK | 0x00820C20 | | ROP_L2_FBP[0..15] | 0x00820D70+4n |
| NVDEC | 0x00820C24 | | TPC_GPC[0..31] | 0x00820C38+4n |
| SEC / PCIE_LANE / SPARE_FS | 0x00820C28/2C/30 | | | |

（TPC_GPC[0..7] 与 dump_tpc 的 STATUS 基址同位——那边只对 ampere/ada 打，
本表全族打印即是补齐。）

### 6b. 词典覆盖核算（2026-11-12 全仓复核）

ga100vfi 仓库的寄存器数据 = 恰好三张图（data/README 自证）：options 672 +
status 84 + core 15；baseline/linux/maps/waveform/leaf-api 五个源文件 grep
无额外偏移。覆盖核算：**status 84/84 ✓、core 15/15 ✓、options 首轮仅 29/672**
——缺口是本节 §6 之外的全部选项词。同日补齐（同钉 0.7.4，同名重建）：

- **`_DISABLE/_DEFECTIVE` 对**（34 词）：FBIO/FBPA/FBP/PES/ROP_L2/NVDEC/
  NVENC/NVJPG/OFA/NVLINK/SYS_PIPE/DISPLAY/PCIE_LANE 各自的熔丝侧禁用与
  缺陷掩码（STATUS_OPT_* 的投影源、GA100VFI FF 烧写目标族）+ DISP_HEAD/
  PERLINK/SEC/GC6/HALF_FBPA/HALF_LTC(_ENABLE 极性)/FB_CONFIG/
  GEN3_SPEED_DISABLE。
- **PCIE_DEVIDA/DEVIDB + MAGIC_A-D**（6 词）：设备 ID 与 Gen4 魔术位。
- **TPC 逐 GPC 三族**：DISABLE 连续 0x820760+4g、RECONFIG 连续 0x820788+4g
  （各 8）、DEFECTIVE 散布 8 词。
- **_CP 影子族全量 24 词**：EN_SW_OVERRIDE 后面的软禁用副本——GA100
  170hx-rir-tool 先例里**可易失改写回启用切断单元**的层（OQ#13 Ada 臂的
  消费卡候选；本轮只读）。

补后 options 覆盖 ≈120/672；**其余 ~550 词有意排除**（XUSB/AZA 音频/ADCS/
时钟树/Tegra 遗产/FPF 备用熔丝 0x8242xx——GA100 专属块，跨 die 信息价值
低），排除理由已注进两张表的源码注释。另记：capture crate 的 512 行
CMD_READ 原始行读出是**写驱动**的操作（FUSEADDR/FUSECTRL 命令），不入
只读 sweep；TU106 解锁态 FUSECTRL_PLM/FUSEOPT_PLM=OPEN 使其成为候选
实验（单独立项）。

### 6c. 覆盖表 TU106 首轮（2026-11-12，ssdiag-v074-ga100vfi-cover.log）

新 72 读在两个面上各跑一遍，五个硬结果：

1. **PCIE_DEVIDA（真 bank 0x214D8）= 0x00001f07——本卡自己的设备 ID**。
   熔丝 bank 自报身份，shifted bank=熔丝身份的**最终确证**（DEVIDB=0x1f47
   =同 die 姊妹 SKU 槽）。
2. **NVLINK_DISABLE（0x21684）= 1 ↔ STAT_NVLINK=1**——熔丝词与状态词跨层
   互证（NVLink 关是分段不是缺陷：DEFECTIVE=0）。
3. **除 NVLink 外全部 _DISABLE 掩码=0**：FBIO/FBPA/FBP/PES/ROP_L2/NVDEC/
   NVENC/NVJPG/DISPLAY/PCIE_LANE/SEC/GC6 全 0——**2070 的 GPC 3/4/5 切割
   不在 _DISABLE 掩码里承载**（OPT_GPC_DISABLE=0 早有此象，现在整族确认）；
   floorsweep 状态只经 STATUS 线表达，TPC_DISABLE/RECONFIG 真bank 上
   badf5040（PL0 不可读）。
4. **_CP 影子层在 Turing 存在且全零**：16 个主 CP 词明文可读、全 0（OFA_CP/
   SYS_PIPE_CP/TPC6-7_CP 不可读=对应块缺位——又一 GPC 数佐证）。OQ#13
   消费臂的数据点：影子层在、无活动覆盖。
5. **平铺面选项词对本 die 非权威**：GEN3_SPEED_DISABLE 平铺=3 vs 真 bank=0
   vs 卡实际训练 Gen3（LnkCap max Gen3）——真 bank 与硬件一致，平铺面
   的非零值是另一套解码的巧合内容；PCIE_LANE_CP=3/DISPLAY_CP=1 同理。
   平铺面新可读词（ROP_L2_DIS/OFA_DIS+DEF/FB_CONFIG=0）与真 bank 同值，
   但以真 bank 为准。附注：PCIE_MAGIC_D=0x16480000 与 XP3G_STATUS3 逐位
   同值——同源寄存器双孔径嫌疑，记录不裁决。

### 6d. GA102（2204，3090 级）词典轮（2026-11-12，diag-unlock-v074-ga102-dict.log）

Linux 侧 diag-unlock.py（0.7.4 面）首张 Ampere 词典，UNLOCKED 态（熔丝
boot-static=出厂值）。0x820xxx = Ampere 真 bank（AD10x/GA10x 惯例）：
**全别名集明文、零 badf1100 缺槽**——Ampere bank 覆盖比 Turing 全。

1. **PCIE_DEVIDA=0x2204=3090 自报身份**（DEVIDB=0x2244 姊妹槽）——bank
   自识别 2/2 跨族（TU106=0x1f07）。
2. **SECURED_BURN_FEAT_EN=0 + EN_SW_OVERRIDE=1 + ENABLE_FUSE_PROG_STAT=1**
   （对照组：GA100 量产 SBFE=1；TU106 槽缺）。两读：①消费 Ampere 出厂
   SBFE=0（L2 语义门在 GA102 **默认开**）；②读数被 EN_SW_OVERRIDE=1 的
   影子污染（但 DSOV_STATUS=1 应挡影子，且其余安全词全是量产 1 姿态——
   SBFE=0 夹在 1 里反常突出）。无论哪种，**本卡剩余的熔丝写阻断只剩
   WDATA 端口**：FUSEWDATA_PLM=0xffffff8f 关 + WDATA_WR_SECURE=1 +
   DSOV=1（挡 kate 影子路线）。判别器=任意 stock GA10x 样本（我方链
   不写 0x820xxx，stock 机同样带驱动，一词即可定）。
3. **熔丝速度选择签名与 TU106 完全相同**（FMLA32=1、IMLA4=1、余 0），而
   3090 的 FP32=市场规格=满公式——**selector 1 ≠ 钳制**。OQ#10 的
   "2070 stock FP32 半速"推测**作废**；selector 是档位码非钳制深度。
   30 系 TF32 钳（#80/#194 ×0.5）不经这九词承载。
4. **跨层互证再添两对**：NVDEC_DISABLE=2 ↔ STAT_NVDEC=2（NVDEC1 关）；
   TPC_DIS[0]=1 ↔ TPC_GPC STATUS[0]=1 ↔ GA100_SM_STAT[0]=1 三方一致，
   84 SM − 2 = 82 ✓ 3090。GA102=7 GPC（TPC_DIS[7]/TPC7_CP/TPC7_DEF
   badf、余可读）。
5. **_CP 影子层 Ampere 活跃**：SYS_PIPE_DISABLE_CP=1——首个非零 CP 覆盖
   （消费卡无 MIG，经 CP 层表达）。CP 层不只是存在，是**活的覆盖状态**，
   OQ#13 易失回启假设加权。
6. **镜像三列改写**：−0x800000 列在 GA102 读出与 TU106 **逐位相同**的值
   （0x22582245、0xff000000）——该列 die 无关，根本不是 per-die 选项
   bank（共块/未解码读），假说③"两库并存"对 −0x800000 死刑。−0x7FF000
   列在 GA102 **解码**（预期 badf 落空）——第三张面：PLM 形值
   0xffffffcf、新毒值码 **badf1002**（#3 词典未载）、WDATA=0x39、
   SBFE=badf1100。三面各表。
7. **Ampere FEAT 簇扩展词首次活读**（TU106 永远 badf 的 0x8238xx）：
   0x823808=0x380、0x82380c=0x00888888、0x823810=0x002aaaaa、
   0x823814=0x233、0x823b00=0x4b8f——0x823xxx 块布局新窗口（0x888888/
   0x2aaaaa 的字段模式疑与逐类速率掩码有关，下一轮分析）。
8. 杂项：FUSECTRL=0xe0040000=GA100 IDLE 同码（TU106 0xc0040000 异码）；
   CRC_STATUS=0x4555（GA100 判 0x55+bit14）；FUSE_VER=3 可读；
   WPR/WPR_CFG_PLM=0xcb8f 族（=OQ#8 AD107 原厂重锁模式跨族确认）；
   GSP_CPUCTL=badf5720（0x56 码族）；MAGIC_D=0x10a80000≡XP3G_STATUS3
   ≡OPT_MAGIC（0x820520=MAGIC_D 同址——三词一源，2/2 跨族）；
   LnkCap2=0x1e 含 bit4=Gen4 能力、LnkCtl2 TLS=Gen3（当前目标）；ReBAR
   BAR1 cap 支持至 1G 档（0x0460）。

**四层阶梯按族修订**：L2（SBFE）GA100=1 锁、GA102=0 开（待 stock 样本
钉死）、TU106=槽缺——门槛阶梯是**逐族结构**，不是统一深度。

### 6e. GA106 stock 对照轮（2026-11-12，2504/3060 12GB，ssdiag-v074-ga106-stock-dict.log）

**对照判别闭合**：stock（PLM=8f 关、SS0=厂值、canary cleared、从未过链）
下 `SECURED_BURN_FEAT_EN=0` 与 `EN_SW_OVERRIDE=1` **原样复现**——§6d 的
"解锁/我方驱动侧产物"假说死刑，**stock Ampere 姿态=SBFE 0 + ENSWO 1**
（GA102=GA106 两 die 一致）。L2 门 GA102/GA106 消费线默认开升格为定论
（判别器一词闭合）；剩余熔丝写阻断=WDATA 端口（8f+WR_SECURE=1+DSOV=1）
不变。附带价值：这是**首个 legacy 驱动模式（EnableGpuFirmware 未设）的
Ampere 词典样本**（3090 那张是 GSP 模式），WPR2_LO/HI=0x1ffffe00/0 =
OQ#7 冷窗口默认态 ✓（无 GSP-RM 在跑）。

**stock GA106 FEAT 簇首读**（#70/#71 那批解锁时没有厂值）：FEAT_SS0=
**0x10210040**（[. r4 . . r1 r2 . r1]，无 override 位）、SS1=0x3（DP=r3）、
READOUT_1=0x00400080（gapped 解码：imla0=0 fmla16=0 fmla32=1 ffma=4
imla1..3=0 imla4=1 DP=0）。熔丝 speed-sel 与 READOUT 互证（fmla32=1↔1、
imla4=1↔1）。**#70/#71 的 FP16 1.80× 算术候选闭合**：SS0 nibble1=r4
（fmla16 位域，视字段序）→ 解锁清 0，1/1.80≈0.556=code-4 速率——厂值
钳制签名与实测增益首次对上（需 theoretical-uplift 交叉核对）。SS0 字段序
存疑：nibble1=4 而 READOUT ffma=4/fmla16=0——nibble 序或与 READOUT 错位
（fmla16/ffma 互换），待 GA10x 解码口径核。

**TPC 双掩码首次同卡齐现**：TPC_GPC0_DEFECTIVE=**0x08**（本轮全套数据
唯一非零 defect 词）——GPC0 的 TPC3=真缺陷；STATUS=0x28（TPC3+TPC5）；
TPC_GPC0_DISABLE=0x08（熔丝禁用词承载缺陷位）、TPC_GPC0_DISABLE_CP=
0x08（CP 影子同步镜像）。结构：**TPC3/GPC0=缺陷（disable 词+defect 位
双载），TPC5/GPC0-2 与整 GPC3-6=纯分段（无 disable 词载体，STATUS 独载）**
——kate 博客的 _DISABLE/_DEFECTIVE 双掩码结构在消费硅首次实证，且分段
切法（同 TPC5 跨 GPC+整 GPC）与 TU104 KO 的"配置切割"形态同族。

**几何极性疑问（新开）**：28 TPC 活 × bit=off 读法 ≠ 28 SM（3060），
bit=on 恰=28——但 GA102 的 82 SM 强制 bit=off（41 TPC×2）；同族两 die
极性矛盾，或 GA106 的 TPC:SM 映射/缺席编码不同（GA106 GPC6=0x3f/GPC7=
0x00 vs GA102 GPC7=badf）。判别器=GR 拓扑锚（0x409604 现为 turing 门，
下轮放全族）+第二张 GA106。显存侧同疑：STAT_FBIO=STAT_FBP=**0x38**、
ROP_L2_FBP[3..5]=3——FBP 掩码↔ROP 数组内部自洽（3/4/5），192-bit 的
3060 需要 FBP 字段宽/恒置位假设再核（GA106 若 3-FBP die：bit0-2 字段+
bit3-5 恒置=TU104 模式，但恒置段只有 3 位与 TU106 的 4 位不同宽）。

**新活值**：L2_CANDIDATE(0x821200)=**0x81**（GA102=0、TU106=badf）——
aajax L2 禁用掩码层在 GA106 承载真实切片切割（L2_EFF=0x81、SW=0）。
FEAT 扩展词跨 die 分类：0x82380c=0x888888/0x823810=0x2aaaaa/0x823814=
0x233 三 die 恒同=常量非速率掩码；**0x823808（GA106=0x281/GA102=0x380）
与 0x823824（1/0）=逐 die 字段**——真正的 SKU 状态候选。镜像 −0x800000
列的"die 无关"要弱化：SM_SPEED_DP=0x22582245 3/3 恒同，但 CRC（0x14710
vs 0x4001ad50）与 DEBUG_FEAT（GA106=badf5720!）逐 die 异——混合面。
−0x7FF000 第三面：WDATA=0x39、0xffffffcf 与 GA102 同值，SBFE=badf1100
三 die 恒同。杂项：XP3G_PLM0=0xffffff8f（TU106 OPEN 仍是孤例✓）；
0x9a0168=0x82006bcf（含 0x8200 的怪值，GA102/TU106=0xffffffcf，记录）；
GFW_PLM=0x8b8f=Ampere 形（Turing=0xffffff8f）。

### 7. 判读指南（TU106/GA106 实验用）

- `badf5040 [PRIV_VIOLATION]` → 词在**已证实解码的块内**时=存在、PLM 挡读
  （**§9 勘误**：TU106 实测 0x8238xx 已知缺席块全域也回 badf5040——特权
  过滤器先于解码，跨块缺席同码；只有块内明文/badf 交错模式才能当存在性
  证据）。
- 正常数据 → 直接按 GA100 语义判读（如 STATUS_OPT_GPC 位掩码=GPC floorsweep
  有效值；TU106 应与已知 GPC 数自洽）。
- `0xFFFFFFFF` / `badf1100 [UNDECODED]` / 全同良性值 → 该偏移在本 die 不
  解码或挪位（AD10x PLM 挪位先例）——**null 结果 ≠ 块缺席**，如实记录。
- `badf20xx [FLOORSWEPT]` → 物理不存在；对照 `_DEFECTIVE` 语义时记得 GA100
  的双掩码教训（分段 vs 良率未切分）。
- FFDO(0x8203F0)：TU106 系统内 PL0 实测 badf5040（§9，块内=存在但不可读，
  值未知；"读 0"是 AD107/GB20x 的实测绘景——总开关未烧是解锁域前提，但
  TU106 上这是从"事务能落地"反推的，不是读出来的）。
- 预期管理：TU106 为 Turing，GSP/SEC2 布局与 GA100 不同属常态；本轮的
  产出是"词典命中率图"，逐词存在/挪位/缺席三态，喂给开放问题 #SBFE。

### 8. 开放问题（新开三条）

1. 消费 die 的 SBFE/EN_SW_OVERRIDE/SENSE_CTRL 面：§6 表在 TU106/GA106 上
   的命中率 + 有多少词可读。
2. "SBFE=1 下存在纯软件 secured burn 路径"（作者原话，语义未明的
   "secured"）——若破，持久软件解锁翻盘，挂外部突变点监控。
3. badf20xx 双掩码切分（同 SKU 多卡 A/B）——影响 22G 类几何研究的边界预期。

### 9. TU106 首轮实测（2026-11-11，ssdiag v0.7.4；Z170-A 台、解锁态、驱动 621.01/GSP 模式）

证据：`docs/evidence/tu106-1f07/ssdiag-v074-ga100vfi-dict.log`（254 行完整
log；同轮 FEAT 块 OPEN+SS 活=解锁态，canary intact）。

**命中图谱：125 读 = 13 明文 + 112 badf5040 + 0 badf1100/0xffffffff——词典
100% 落在解码空间，零"缺席"码。** 0x820000–0x821108 全段明文/badf 逐词交错
（0x820040/60 可读、0x820070-84 badf、0x8200f4/f8/fc 可读、0x820148/158
可读、0x820224 可读……）——逐词解码粒度，0x820xxx 面在 TU106 实存
（§11：控制/PLM 词真、选项词空槽——真值面在 −0x7FF000）。

明文 13 词（存在性铁证 + 值判读）：

| 词 | 值 | 判读 |
|---|---|---|
| FUSECTRL | 0x40040000 | Turing 控制器 FSM 实值（GA100 IDLE=0xe0040000，同族异代编码，记录在案） |
| FUSEADDR | 0x00000000 | 空闲 |
| EN_SW_OVERRIDE | 0x00000000 | **GA100 goldilocks 支点寄存器在 TU106 明文可读**，读 0=影子关 |
| FUSEDEBUG | 0x00000000 | 正常视图（bit1 未切 RIR 宏） |
| FUSEWDATA_PLM | 0xffffff8f | 关（stock）——与 GA100 量产行为同向（WR_SECURE 词烧 1 的投影） |
| FUSECTRL_PLM | 0xffffffff | **OPEN**（caveat：解锁态单样本，冷/锁定态基线待采） |
| FUSEOPT_PLM | 0xffffffff | 同上 |
| RING_GLOBAL_CTL | 0x00000000 | PRI 环未持复位（GA100 controller_valid 同判 0） |
| RING_RESET_PLM | 0xffffff8f | 关（stock） |
| DEBUG_FEAT_SW_OVR_EN | 0x00000000 | §11 修订：真值在 0x21148=1（GA100 量产语义）；本址读数降级为布局空槽 |
| IST_DEBUG_FA | 0x00000000 | 同上族（真值面 −0x7FF000） |
| PRIVKEY_ACC_DIS | 0x00000000 | §11 修订：真值 0x21324=4；本址空槽 |
| SM_SPEED_SELECT_DP | 0x00000000 | DP 速率选择熔丝词明文 0；FFMA/FMLA16/32/IMLA0-4 全 badf——逐词 PLM 粒度的又一证 |

badf5040 组（112）：CRC_STATUS、EFP/DFP/DSOV 三状态、SBFE(0x821108)、
SECURE_* 调试禁用族、HDCP、WDATA_WR_SECURE、DEVID_SW_OVR_DIS、8/9
speed-sel、全部 STATUS_OPT 标量+三数组。

**§7 勘误实据**：同 log 里 TU106 已知缺席的 0x8238xx 全域（FEAT_AMPADA
簇+AMPADA_SWEEP 9 词+PLM survey 3 词）无一例外回 badf5040——跨代块不可能
存在于 Turing die，故 badf5040="PL0 不可读"（过滤器先于解码），跨块缺席
同码；存在性推断仅块内有效。

**与 open-questions #13 Turing 臂的交叉（待判别）**：#13 记 TU104(1e89)
实测选项状态词落 0x021xxx（−0x7FF000：GPC status 0x21c1c、TPC 0x21c38+4g、
FBIO/FBP/ROP 0x21c14/21d38/21d70"全数命中"），推导控制器词在 0x020xxx
（−0x800000）。TU106 今在 0x820xxx 读到实值——三假说：①bit23 别名（两址
同块，PRI 解码忽略 bit23）②TU104≠TU106 布局（同族共享 IP 下可能性低）
③两库并存。**镜像探测判别**（§10，随 v0.7.4 一并交付）。

### 10. 镜像探测（0x820xxx vs −0x800000 vs −0x7FF000；v0.7.4 内置）

16 代表词（控制器核心/明文三别名/DP/GPC 掩码对/状态词/SBFE）× 三址并读。
判读：镜像列与原址**同值** → bit23 别名坐实（TU104 与 TU106 数据合流，
两址皆有效）；镜像列 badf 而原址明文 → 0x820xxx 独占（TU104 结论需修）；
镜像列出现**不同实值** → 两库并存（最有意思的形态）。GA10x/Ada 上镜像列
应 badf（对照组）。本轮同时执行 open-questions #13 Turing 臂的 ①
（0x020350/0x0205c4/0x020040 三点只读验证）。

### 11. 镜像判定（2026-11-11 TU106 实测）

证据：`docs/evidence/tu106-1f07/ssdiag-v074-ga100vfi-mirror.log`（轮内
interim 构建采集，本轮全部功能最终钉在 v0.7.4 一个版本交付）。三假说的归宿：

**①−0x7FF000（0x021xxx）= TU106 真选项库（#187/TU104 结论在本 die 复现）**。
控制面与选项面全部实值且与 GA100 语义相容：`EN_SW_OVERRIDE=0`、PLM 三件套
`8f/ff/ff`（与 0x820xxx 面逐位同）、`FUSECTRL=0xc0040000`（与 0x820xxx 的
0x40040000 低 24 位同、顶字节异——同族 FSM 的双窗差异）、
`DEBUG_FEAT_SW_OVR_EN=1`（**GA100 量产语义**：row0/1 安全词吹 1）、
`PRIVKEY_ACC_DIS=4`、`SM_SPEED_SELECT_DP=0`、`OPT_GPC_DISABLE=0`、
`OPT_GPC_DEFECTIVE=0`（全 die 2070 自洽）、`WDATA_WR_SECURE=1`
（**与 FUSEWDATA_PLM=0xffffff8f 语义锁死成对**——烧 1 → WDATA 关，跨两个
面孔径互证）、`STAT_GPC=0x38`、`STAT_TPC_GPC0=0`。SBFE 的 sub-0x1108 落在
0x22108=badf1100——0x021xxx 页不含该槽（≠词缺席，见 §11 待决）。

**②−0x800000（0x020xxx）= 非 GA10x 镜像**。多为其它解码区数据
（0x71058113、0x22582245 等），但非纯垃圾：`0x205c4=badf5108`
（**安全 PLM 主机读**，#3 词典码）= 该址实词被 secure 级挡住——
cold-values §TU104 列的三个"应重试"候选（0x020040/0x020084/0x0205c4）
今日得其二：0x20040=0x20000000（值、语义未知）、0x205c4=badf5108。
0x020xxx 区需要自己的地图，不是本轮能收的。

**③0x820xxx = 解码面但非选项真值面**。控制/PLM 词与 0x021xxx 库一致
（同值），**选项词不一致**：DEBUG_FEAT 0 vs 1、PRIVKEY 0 vs 4、OPT_GPC/
WDATA_WR badf vs 0/1。v0.7.4 的"13 明文"里，控制器核与 PLM 词的语义归属
维持，三个明文 0 的选项词（§9 表）**归属降级为布局空槽/零锁存**——真值
在 −0x7FF000。故 §3 的"0x8203f0 同位"实为文档性同址（TU106 从未在该址
读出过值），Turing 的真 FFDO 候选=0x213f0（v0.7.4 起入表）。

**遗留三问**（v0.7.4 已工具化；2026-11-11 同日事故与加固见 §12，三问全文
在本节末尾重列）：

### 12. 工具事故与加固（2026-11-11，同日）

v0.7.4 首跑 turing 移位段在 FUSEOPT_PLM 后**进程静默退出**（用户实测：exe
死掉，机器无恙，数秒后返回；重跑修复版完整走通）。根因 = 我的算术 bug：
GA100_FUSECTL_ROWS 里的两个 PRI 环词（RING_GLOBAL_CTL 0x120060 /
RING_RESET_PLM 0x1200A0）**不是 0x82xxxx bank 行**，移位减法
`0x120060 − 0x7FF000` 下溢 u32，release 构建静默回绕，读取落在 16 MiB 用户
映射之外 ~4 GiB → 未映射用户 VA → access violation → 进程终止。（初判
"master abort 冻结总线/硬关机恢复"是错的——实际是进程级 AV，机器存活；
pmxdrv 把物理页映射成用户 VA，越界读撞的是 VA 边界不是总线。）全程只读、
零写操作。教训三层，全部当日修入 v0.7.4（同名重建）：

1. **根因**：移位只对 bank 内行有定义——移位段跳过 `<0x800000` 的行（两
   RING 词已在平铺段读过）。
2. **回归网**：blackwell 测试能拦上一轮 0x800_0000 常数错（镜像段无家族
   门），但 turing 移位段有 `fam=="turing"` 门且无 turing 用例——新增
   `turing_dump_renders_without_panicking`（debug 构建对溢出 panic，正是
   拦截网）。**规矩：每个 fam 门控段必须有对应家族的 render 测试。**
3. **护栏**：主 BAR0 rd/wr 闭包加界检——偏移 ≥16 MiB（0x01000000）读返回
   全 1、写直接忽略。今后任何上游算术 bug 的最坏后果从"进程死亡"降级
   为"一个数据点"（Linux 侧 diag-unlock.py 同步加护栏）。

冻结轮的部分产出仍有效（[shifted-partial 证据](evidence/tu106-1f07/ssdiag-v074-ga100vfi-shifted-partial.log)）：
移位面 FUSECTRL=0xc0040000 / PLM 三件套 8f/ff/ff 复确认；**FUSEADDR=0xff
（平铺面读 0）**——移位面首个独有的非零内容词，语义开放（fuse 行地址寄存
器持 0xff？），挂入 §11 遗留观察。修复版重跑完整走通（[full 证据](evidence/tu106-1f07/ssdiag-v074-ga100vfi-full.log)），
完整移位面分析见 §13。

**遗留三问**（接上，完整重列）：a) `STAT_GPC=0x38` 与全 die 2070 矛盾
（TU104 同址=0x8 即"GPC3 关"；0x38=bit3/4/5）——需 GR 拓扑 0x409604 锚
交叉（TU104 KO 5 活 GPC 读 0x06030005；若 TU106 读 6 活 GPC 而 STAT_GPC
仍 0x38，则 TU106 的 0x21c1c 归属或编码另议）；b) 0x820xxx 面的本体
（第二孔径？过滤器自身寄存器？）——只读手段到此为止，记录为开放细化；
c) GA10x/Ada 上 −0x7FF000 列的行为=对照组（预期 badf，若同值则别样故事）。

### 13. 完整移位面（2026-11-11 修复版重跑，= §11 三问的答案）

证据：`docs/evidence/tu106-1f07/ssdiag-v074-ga100vfi-full.log`（284 行完整
走通；解锁态样本，但熔丝 bank 是 boot-static，读数=出厂值——OQ#10 想要的
"FACTORY 窗口"从此可以系统内侧读，不依赖 payload 的 POST-BOOT 点）。

**a) STAT_GPC=0x38 谜底：GPC 3/4/5 整体 floorsweep——五源自洽，不是矛盾。**
①STAT_GPC=0x38（bit3/4/5）②TPC_GPC[3,4,5]=0x3f（6 位宽全置=该 GPC 全部
6 TPC 关；GPC 0/1/2=0x00 全开——TU106 6 TPC/GPC 字段满宽，**无 TU104 的
恒置高位**，KO 的 bit4/5 恒置=字段上方空位读 1 的同一模式，两 die 统一）
③PES_GPC[3,4,5]=7 ④GR_TOPO=0x08040003 byte0=3 活 GPC（KO 锚 {6,3,0,5}=
5 活 GPC 的同位解读）⑤36 SM = 3 GPC × 6 TPC × 2 SM ✓ 2070。同轮修正：
OPT_GPC_DISABLE(0x21350)=0 而 3 个 GPC 确实关着——**GA10x 的"0x820350=
GPC disable"映射在 Turing 不承载 GPC floorsweep**（整 GPC 关由 TPC 掩码
0x3f 表达）；GR_TOPO byte3（TU104=6 vs TU106=8）编码仍未解，唯一悬挂位。

**b) STAT_FBP=0xf0 = 4 位字段（FBP 0-3 全开，256-bit 满配）+ 字段上方恒置
高半字节**——与 TU104 TPC 恒置位同模式；ROP_L2_FBP[4..7]=3 为同一恒置
模式的数组投影。**2070 显存侧零切割**（呼应 §9 FFDO 行的教训：读数先过
字段宽度解释再谈语义）。

**c) FFDO=0（0x213f0）——TU106 覆盖域总开关第一次是"读出来的"**，不是
"事务能落地"的反推。与 AD107（0x8203f0=0）、GB206（0x8203f0=0）三族同值：
消费线覆盖域全开。PRIV_ROWS 的 0x8203f0 平铺读 badf5040 保持文档性同址。

**d) SBFE 在 Turing 缺槽（0x21108=badf1100）**：三个最高偏移别名
（XV2CLK 0x21064、RESENSE 0x21058、SBFE 0x21108）全部 badf1100，最后解码
的别名=SECURE_XUSB_DEBUG_DIS（0x20b98）——Turing bank 解码边界落在
0x20b98 与 0x21064 之间，GA100 的 SBFE 槽位在 TU106 无载体。语义门（L2）
在消费 die 是否另有地址=开放；但 GA100 地址上确定没有。

**e) 熔丝级速度选择 = OQ#10 的出厂钳制签名**：**FMLA32=1、IMLA4=1，
DP/FFMA/FMLA16/IMLA0-3=0**。对照 AD107M stock READOUT 字段 2/5/7=2
（fmla32/imla2/imla4 钳 selector 2）——同一轴、逐 SKU 深度阶梯：桌面
2070 两类指令钳 selector 1，移动 AD107 三类钳 selector 2。selector 的
Turing 速率法则未证（运行期解码口径 1=half 不能直接平移到熔丝层——
若 1=half 且 stock 生效，2070 市场 FP32=钳后值，解锁=2×；但 50HX 的
31.7× 说明谱系里存在深得多的钳，需 50HX 词典样本来标定）。关闭路径：
①stock 态 READOUT 采样（重锁后系统内读）②FP32 microbench 前/后 A/B
（OQ#10 ③ 原案）。

**f) 特性切割清单（消费分割的熔丝证据）**：NVJPG=1（关，GeForce 无
NVJPG）、NVENC=2（bit1 关→1 个 NVENC ✓ 2070）、NVLINK=1（关 ✓）；
DISPLAY/DISP_HEAD/PCIE_LANE/SEC/FB_CONFIG 全 0（开）。安全别名姿态=
GA100 量产同向（PRIV_SEC_EN=1、PRIVKEY_ACC_DIS=4、DEBUG_DIS 族=1、
WDATA_WR_SECURE=1↔FUSEWDATA_PLM=8f 跨孔径锁死复确认）：**消费 vs
数据中心的分野不在安全熔丝，而在 FFDO=0 + 钳制走运行期层 + 速度选择值**
——§2 四层阶梯的硅证据版。

**g) 镜像三列复确认**（第三跑，逐位稳定）；FUSEADDR=0xff（真 bank）维持
开放。GA10x 对照（GA102 词典轮）随 diag-unlock.py 0.7.4 面执行。

## maker 免 U 盘一键安装 + 资产瘦身（2026-11-12，make-usbroot.ps1/.sh + assets）

**动机**：#73 裁定拒绝的是"替用户动分区表的 installer"；把布局**追加复制进用户已有的
FAT32 分区**不触碰该裁定——分区不被创建/调整/格式化，已有文件不被覆盖，一次引导菜单
选择即可用，U 盘不再每轮必插。

### 1. 安装探测（两个 maker 同构）

生成完成后枚举**现有** FAT32 卷：Windows 侧 `Get-Volume`(FAT32+DriveLetter) →
`Get-Partition`(IsBoot/IsSystem/GptType=ESP 三重排除) → `Get-Disk`(BusType；
USB=可移动、其余=内置；本机 NV-UNLOCK H: 是 SATA 附着=按内置分类——BusType 是
Windows 唯一干净的内部/外部信号，SATA 硬盘盒会算内置，属可接受偏差)。Linux 侧
`lsblk -rno KNAME,RM,TYPE,FSTYPE,MOUNTPOINT,PARTTYPENAME,LABEL`：PARTTYPENAME
"EFI System" 直接排除（宪法 #3 ESP 禁写），`/boot*/`、`/efi*`、`--out` 所在挂载点
排除；只收**已挂载且可写**的卷（udisks 桌面自动挂载覆盖主流），未挂载的打印提示不收。
空间门槛 = 布局实测字节数 + 16MB 余量。

两段式菜单：内置候选先问"持久化安装"，未装再问 U 盘候选；皆无/皆跳 → 双语指引
（内置划非 ESP 数据 FAT32 分区 / 准备够大 FAT32 的 U 盘）。自动化出口：
`-NoInstallPrompt`/`--no-install-prompt`、`-InstallTo D:\`/`--install-to <挂载点>`
（与探测同一套资格校验，不合格即 throw/die）。

### 2. 数据安全契约（读回纪律的文件域投影）

`nvuu-stick.json`（maker 每次生成写入）= 目标身份标记：无标记目标出现同名异内容文件
→ **整个目标拒绝**并逐个列出，零写入；有标记 → 视为本项目旧布局允许原地升级。
逐字节同内容 → 跳过不重写。写完全量 SHA-256 读回比对，失败即报"勿从该副本引导"。
引导路径=固件一次性菜单（`\EFI\BOOT\BOOTX64.EFI`），零 NVRAM 写入——与 #73 宪法完全
相容；对内安装明确警示"不是所有固件枚举内置 FAT 卷"。

### 3. 瘦身三件

① assets `*_raw.c`（438K 级取证转储）改 `ASSETS_DEBUG_C=1` 门控发射；② `gsp-sections`
按族过滤——`.fwimage` 是族中性的，旧默认导出 5 族=每族一份 84M/29M 重复镜像，maker
现在只传本构建的族（ga10x/ad10x/tu10x/tu11x）；③ 提取中间 ELF（gsp_ga10x.bin 84M /
gsp_tu10x.bin 29M）在 gsp-sections 消费完即从输出根删除（loader 只读卷根
`fwimage_*` 等，`tx.rs` 卷根解析从不碰这两个 ELF）。实测 tu10x-only 输出 11 文件
~16.5 MB（badge-mode GSP）。

### 4. sh↔ps1 同步（20 系不出 tu11x；tu11x 仅 16）

sh 补齐 tu11x 分支、`--family 16|rtx16` 别名、`family_of_dev` 按 ps1 范围拆分
（0x1E02-0x1F80=tu10x；0x1F81-0x1FFF+0x2182-0x21D1=tu11x——此前 Linux localhost 会把
16 系误路由 tu10x）、scrubber 排除 tu11x、TU/GA 签名族表。badge-mode 增加 Git Bash
（/c/...）与 WSL（/mnt/c/...）的 DriverStore glob。

### 5. sh 三处潜伏 bug（顺手修，均有复现）

① `resolve_utils` 的 fwdir 捕获用 `for d in …; do [ -f ] && [ -f ] && echo; done |
sort | tail`：最后一条 glob 不匹配时**循环退出码=1**，在 pipefail+set -e 下穿过命令
替换杀死赋值——脚本在 SRC1 缓存行后**静默死亡**（2026-11-01 引入 badge mode 起，
Linux 自动档必死）。修复=改 `if` 形式。经验：bash 捕获内的 for 循环体，`&&` 链失败
会成为循环退出码——捕获必须 if 化或 `|| true`。② MSYS df 无 `--output` 同一死法，
捕获全部补 `|| true`。③ `[ "$(…)" ]` 类比较照旧。

### 6. 验证与证据

assets 重建绿+门控实测；双脚本语法过；tu10x/16 双端到端干跑绿；`-InstallTo D:\`
拒绝路径、管道 stdin 越界应答→指引路径实测；Copy-UnlockRoot / install_copy harness
各 4 测（fresh/identical/拒绝不落笔/标记升级）全绿。证据
`docs/evidence/maker-install-offer/`，回归 harness `tools/maker-tests/`。
**未做**：release/assets-x86_64-windows.exe 仍旧版（发版时按三定律重建换件）。
