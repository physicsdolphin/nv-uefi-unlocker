# 寄存器冷值表（M1 采样产出，按族分节）

> 勘误（2026-10-28）：TU106 表中的 13/13 BADF 来自 `0x823xxx`；Turing 真 FEAT 地址是 PLM `0x409650`、SS0 `0x409664`、SS1 `0x40966c`，后续部分样本在这些地址可读。CRC32 为采样指纹，逐卡 SHA-256 校验待实现。28e1 的 0x89 现已定位为[桥 BME 问题](bridge-bme-root-cause.md)。表尾 WPR2 lo/hi=0/0 来自旧任务书，实测冷态空窗口为 lo=`0x1ffffe00`、hi=0。表中原始采样值保留。

> 每次实机采样后把 `\loader.log` 的 SAMPLE 段誊入对应节，保留原始日志于
> `docs/evidence/m1-<machine>/`。冷值=驱动加载前（UEFI 阶段）读回值。
> badf 桶：PRIV_VIOLATION=0xBADF5040 / UNDECODED=0xBADF1100 / FLOORSWEPT=0xBADF20xx。

## TU106（RTX 2070 台式，10de:1f07）——已采样（2026-10-26）

| 寄存器 | 冷值 | 解读 |
|---|---|---|
| PMC_BOOT_0 | 0x166000a1 | TU106 ✓ |
| **FEAT 簇 13/13 badf5040** | PRIV_VIOLATION | **簇存在但 Turing 在 PL0 全锁定**（开放问题 #1 关闭） |
| FEAT_READOUT_1 | badf5040 | PL0 无法判定钳制态 |
| GSP_CPUCTL 0x110100 | **0x10 可读** | RISC-V 核 CPUCTL 界面 PL0 可读、HALT 位同义（开放问题 #4 部分关闭；BCR 0x668 切换语义待事务轮核实） |
| SEC2_CPUCTL | **0x10 可读** | 与 AD107（badf5620 门控）代际差异 |
| BSI handoff (SCRATCH_15 0x1180fc) | 0x202001bf（bits31:29=1） | 与 AD107 一致：pre-driver 卡在 state 1 |
| WPR2_LO/HI 0x1fa824/28 | 0x1ffffe00 / 0x0 | 与 AD107 冷值完全相同——GFW 后空窗口默认态（非异常）；hi=0 守卫放行 |
| MMU_LOCK_LO/HI 0x1fa82c/30 | 0x00100000 / 0x01effff0 | 可读，WPR 布局计算输入（VBIOS 锁定区） |
| MMU_PLM / FUSE 簇 | badf5040 | 锁定 |
| FBPA_PLM 0x9a0148 | 0xffffffcf | 可读，值异于 AD107 |
| AD1xx 名义寄存器 CYA/CFG/BAR1_CAP | 0x8 / 0x2 / 0x1c00 | Turing 上地址语义未核实，仅记录（跨族地址采样噪声） |
| NV_PROM 64K | crc32 0x552e185c | 本卡身份 |
| post-FLR GFW_BOOT | 0x101 卡住 | 与 AD107 一致（跨代 FLR 行为） |
| **XP3G 冷态（v2.1.9 EFI 首采，2026-11-02）** | PLM0 `0x8e1b0`=**0xffffffff [NO_DECODE]**；PLMC `0x8e1bc`=badf5040（锁）；OVR0/VAL0=0 | PLM0 全一与"OPEN"读数歧义（开放问题 #12 输入）；对照 ssdiag Windows 侧同并集。XVE 镜像全可读：LNKCAP 0x00453d03 / LNKCTL 0x11010043 / LNKCAP2 0x0e / LNKCTL2 0x001f0003 |
| **TU106 回锁值 = 0xffffffef**（v2.1.9 回归，事务后） | bits5/6 置位，与 TU102 MSI（2654419）跨 die 一致 | 回锁方差非单板特例；已列 known-good |
| **出厂 FEAT_READOUT_1（payload 窗口捕获）** | `0x00400080`（POST-BOOT、selector 写前） | 开放问题 #10 的 TU106 consumer 出厂钳制态直接读回；解锁后归零 |
| PSTRAPS SET0 0x101000 / SET1 0x10100c | **0x00400080 / 0x02013000（2026-10-27 实测，均可读）** | 按 SET0[15:14]+SET1[22:20] 和值解码 = **0 → 64MiB，override_en=0**——与运行时 BAR1（2070 标称 256MiB）不符。解读：这是 ROM 加载的**原始 straps 默认值**；NvStrapsReBar 在**枚举期（devinit 之前）**改写才生效，而我们的 app 在 GOP/devinit 之后采样，运行时的 256MiB 由 devinit 自行编程（或采样 straps 后按 VBIOS 表覆盖）。BAR0 索引 0（16MiB ✓ 与 BAR0 base 0xde000000 的 16MB 窗口一致）、BAR3=2×BAR0。**待办**：Windows 侧 `nvidia-smi -q` 的 BAR1 Memory Usage 实证运行时孔径；若做 straps 写实验须确认 devinit 时机 |

## GA106（RTX 3060，10de:2504）——未采样

原 M4 候选机。M4 实际在 AD107M 28e0 上闭环（见 `ada28e0-uefi-report.md`）；
GA106 卡在库未采样，本节待实机后补。

## GA104（RTX 3070，10de:2484）——社区采样（issue #164，2026-11-04）

非本机；来源为报告者 U 盘 `\loader.log` 的 SAMPLE 段（X570 AORUS ULTRA 桌面、
独显单卡、VBIOS 94.04.25.00.ed、驱动 617.42）。**条件限定**：该轮是 Windows
重启（快速启动开），入口状态是上一轮驱动复位后留下的厂态，不是断电冷采样；
即便如此仍是首张 GA10x 台式卡表。

| 寄存器 | 值 | 解读 |
|---|---|---|
| PMC_BOOT_0 | 0xb74000a1 | GA104 ✓ |
| FB_SIZE_MIB | 0x2000（8192MiB） | 8GB 布局 |
| VGA_WORKSPACE | 0x00010e09 | 本卡为 console owner（bit3=1） |
| GFW_PLM / SCRATCH | 0x00008b8f / 0x000003ff | GFW 已跑完 |
| GSP_CPUCTL 0x110100 | 0x10（HALTED） | 驱动未点火 |
| PGC6_BSI_SCRATCH15 | 0x201001bf | handoff=1, DONE=3（与 GA10x 同形态） |
| MMU_PLM / LOCK | 0x0045cb8f / 0x00100000 / 0x01effff0 | 与 AD107 同族 |
| WPR2_LO/HI | 0x1ffffe00 / 0x0 | 空窗口；hi=0 守卫放行 |
| SEC2_CPUCTL | 0xbadf5620 | 与 AD107 同一 0x56xx 冷态变体 |
| FUSE_OPT_FEAT_OVR_DISABLE | 0x0 | 解锁域存在 |
| FUSE_VER / DECODE | 0x3 / 0x3 | v3 槽位（与 TU106 的 1 不同） |
| AD1xx CYA / CFG | 0x28 / 0x80000007 | 与 AD107 同（≠4090 的 0x30/0x8000000a） |
| AD1xx XVE_PLM | 0xffffff8f | 锁定基线 |
| AD1xx BAR1_CAP_MIRROR | 0x0003fc00 | 与 AD107 同 |
| Ada 链 WPR/WPRCFG/FBPA PLM | 0x4cb8f / 0x4cb8f / 0xffffff0f | 与 AD107 逐位一致 |
| TU10x FEAT 0x409650/64/68 | 0xbadf5040 | GA10x 上为 PLM 挡（特权桶） |
| XP3G PLM0 / PLMC | 0xffffff8f / 0xffffffcf | 与 AD107 同 |
| FEAT 0x823800/04 | 0xffffff8f / 0xffffff8f | 锁定基线 |
| FEAT 0x823808/80c/810/814 | 0x01000283 / 0x00888888 / 0x002aaaaa / 0x233 | 其他 feature 类簇 |
| **FEAT_READOUT_1 0x823818** | **0x00400080** | 字段 2/7 = 2（约 1/4 速率）——本卡也有钳制；与 AD107 的 0x00410080 差字段 5 |
| **FEAT SS0 / SS1** | **0x00000000 / 0x00000000** | **首张 SS 对为 0 的卡**（#109 的 1f06 是 0x42072154/0x6）——"SS=0 即被抹"的判据需按卡校准 |
| FEAT 0x823824/82c | 0x1 / 0x5 | 记录 |
| PSTRAPS SET0/SET1 | 0x00400080 / 0x02013000 | straps BAR1=64MiB；活窗 256MiB（枚举期 CTRL 写，见 #12） |
| NV_PROM 64KiB | crc32 0x63d10fc1，magic 0xeb7faa55 | 本卡身份指纹 |

## TU104（RTX 2060 KO，10de:1e89）——社区只读熔丝图（issue #187，2026-11-05）

非本机；EVGA 2060 KO（06G-P4-2068，TU104 A1，VBIOS `90.04.63.40.55`）、ASUS TUF
B450-PLUS GAMING + R7 3700X（无 iGPU，本卡带显示）。SystemRescue 13.02（6.18.41-lts）、
nouveau 绑定态、`resource0` 只读 + `PROT_READ` mmap、32 位对齐读、零写入。
**条件限定**：这是驱动已绑定、GPU D0 的读点（`runtime_status=active`，PGRAPH 已上电），
不是 pre-driver 冷采样；熔丝块本身是 boot-static，故数值等同于冷值，但调用链上它仍是"在线读"。

**为什么值得单列**：这是本仓第一份 Turing die 的 floorsweep 熔丝图。TU104 上
`0x021xxx` 块的地址与 Ampere `gv11b` 命名表逐一对应，统一差 **−0x7FF000**（下表右列）；
报告还另用 GR 拓扑（`0x409604`、`GPC_UNIT(g,0x2608)`）做交叉验证，两源逐项吻合。

| 寄存器（gv11b 名） | gv11b 地址 | −0x7FF000 | 本卡实测 | 值 / 解读 |
|---|---|---:|---:|---|
| STATUS_OPT_GPC | 0x820c1c | 0x21c1c | 0x021c1c | `0x8` → 物理 GPC3 熔断（TU104 满配 6 GPC） |
| STATUS_OPT_TPC_GPC(0..2) | 0x820c38+4g | 0x21c38+4g | 0x021c38/3c/40 | 全 `0x31` → 每 GPC 的 TPC0 熔断 |
| STATUS_OPT_TPC_GPC(3) | 同上 | 同上 | 0x021c44 | `0x3f` → 整个 GPC3 关（含不存在的 TPC4/5） |
| STATUS_OPT_TPC_GPC(4,5) | 同上 | 同上 | 0x021c48/4c | 全 `0x31`，与 GPC0-2 同形 |
| CTRL_OPT_TPC_GPC(0..5) | 0x820838+4g | 0x21838+4g | 0x021838+4g | 全 `0x0` → 无活覆盖 |
| STATUS_OPT_FBIO | 0x820c14 | 0x21c14 | 0x021c14 | `0x4` → FBIO2 关 |
| STATUS_OPT_FBP | 0x820d38 | 0x21d38 | 0x021d38 | `0xf4`（bits 2,4,5,6,7）→ 5 个 FBP 关，3 个活 |
| STATUS_OPT_ROP_L2_FBP(0..3) | 0x820d70+4f | 0x21d70+4f | 0x021d70+4f | `0x0/0x0/0x3/0x0` → FBP2 的 ROP/L2 切片关 |
| OPT_FEATURE_FUSES_OVERRIDE_DISABLE | 0x8203f0 | 0x213f0 | 0x0213f0 | **`0x0`——覆盖总开关未烧，解锁域存在** |
| OPT_PRIV_SEC_EN | 0x820434 | 0x21434 | 0x021434 | `0x1`，特权安全态 |
| OPT_SEC_DEBUG_EN | 0x820218 | 0x21218 | 0x021218 | `0x1` |
| GR 拓扑 0x409604 | — | — | 0x06030005 | 5 个活 GPC / 3 个 ROP 分区（与熔丝互证） |
| GPC_UNIT(g,0x2608) g=0..4 | — | — | 3,3,3,3,3 | 15 个活 TPC = 30 SM = 1920 CUDA |
| PMC_BOOT_0 / PMC_ENABLE | — | — | 0x164000a1 / 0xd0fdd1c1 | TU104 A1；PGRAPH 已上电（在线读的特征） |

**解码口径**：`STATUS_OPT_TPC_GPC` 的置位=该 TPC 被关（与 gpcprobe 的 GA100 语义一致：
整 GPC 关时读出 `0xff`）。Turing 上该寄存器是 **6 位宽**（TU102 每 GPC 6 TPC），TU104 每
GPC 只有 4 个 TPC，**bit4/bit5 恒置位**，只有低 4 位有信息。本卡 floorsweep = GPC3 整关
（4 TPC）+ 其余 5 个 GPC 各关 TPC0（5 TPC）= 关 9 TPC = 18 SM，48−18=30 SM。
同一 TPC0 在五个活 GPC 上同关，形态更像配置切割而非逐 die 修补；熔丝不区分二者。
显存侧 FBP2 + FBIO2 关 + ROP/L2 `0x3` 自洽：192-bit、48 ROP、3 MB L2（KO PCB 上对应
GDDR6 焊位空置）。**`1e89` 是 2060 SKU 用 TU104 die 的"大 die 降配"特殊品，与 #137 同卡型。**

**一个否定结果是假的**：报告的 `EN_SW_OVERRIDE` / `DIS_SW_OVERRIDE` 猜测用了 −0x800000
的基址（读 0x021040 / 0x021084，均 `badf5040`），而实测通过的九个寄存器全部走 −0x7FF000——
两个猜测地址各偏 +0x1000。因此 `0x021084=badf5040` **不构成"Turing 无此寄存器"的证据**，
应重试的候选址：`0x020040`（EN）/`0x020084`（DIS）/`0x0205c4`（OPT_GPC_DEFECTIVE）/
`0x020350`（OPT_GPC_DISABLE）。Turing 侧这三类地址尚无任何一手头文件可核（本仓 reference/
只有 Ampere/Blackwell 的公开 swref），上面的 −0x7FF000 关系是**实测归纳**，不是源码事实。

## AD107 台式（RTX 4050 LP，10de:28e1）——社区采样（issue #1，2026-10-26）

非本机实测：报告者 U 盘 `\loader.log` 的 SAMPLE 段（AMD 台式机 + 15bf iGPU，
dGPU 非 console owner，1 个 GOP 句柄）。首张台式 AD107 样本，6GB 与 28a1 同
几何。采样当时事务被**平台固件 IOMMU（AMD-Vi）**挡在 booter（status 0x89 =
wpr_desc_validate 拒收 WprMeta DMA 读回）；报告者关闭 BIOS 相关选项后全链
通过——平台 DMA 前提见 [桥 BME 根因](faq.md#bridge-bme)。

| 寄存器 | 值 | 解读 |
|---|---|---|
| PMC_BOOT_0 | 0x197000a1 | AD107 ✓ |
| FB_SIZE_MIB | 0x17fd（6141MiB） | 6GB 布局（与 28a1 同） |
| VGA_WORKSPACE | 0x00000001 | 非 console owner 的健康回退值（bit3=0，与 28a1 毒值路径同判） |
| SEC2_CPUCTL | 0xbadf5620 | 与 28e0/28a1 相同的冷态域门控（复位+scrubber 清散） |
| FEAT_READOUT_1 | 0x00410080 | 与 28e0 逐位一致（字段 2/5/7=2） |
| FEAT SS0 / SS1 | 0x60042107 / 0x6 | 本卡熔丝派生冷值（非覆盖态；SS 漂移族 n=1） |
| WPR/WPRCFG PLM | 0x0004cb8f / 0x0004cb8f | 与 28e0 冷值一致（四写目标在位） |
| FBPA_PLM | 0xffffff0f | 与全系 AD107 一致 |
| WPR2_LO/HI | 0x1ffffe00 / 0x0 | 空窗口默认态，hi=0 守卫放行 |
| FUSE_VER / 总开关 | 0x1 / 0x0 | 签名槽 v1；解锁域存在 |

## AD107M（RTX 4060 Laptop，10de:28e0）——已采样（2026-10-25，见 evidence/m1-ad107-laptop/）

| 寄存器 | 冷值 | 解读 |
|---|---|---|
| PMC_BOOT_0 | 0x197000a1 | impl 0x197 = AD107 ✓ |
| FB_SIZE_MIB | 0x1ffc（8188MiB） | 8GB 布局 |
| GFW_PLM / SCRATCH | 0x00008b8f / 0x000003ff | GFW 已跑完（scratch 就绪态） |
| GSP_CPUCTL 0x110100 | 0x10（HALTED） | GSP 冷态停机 ✓ 驱动未点火 |
| MMU_PLM | 0x0045cb8f | 写锁定型 PLM |
| WPR2_LO/HI | **0x1ffffe00 / 0x00000000** | GFW 后的空窗口默认态；hi=0→M3 冷启动守卫放行（任务书"恒 0"应修正为"hi 恒 0"） |
| SEC2_CPUCTL 0x840100 | **0xbadf5620** | **新发现的 badf 变体（0x56xx 桶）**，语义待 M2 对照 cmp170hx register-reference |
| FUSE_OPT_FEAT_OVR_DIS 0x8203f0 | **0x0** | **总开关未烧——解锁域在本芯片存在**（与 CMP 170HX 同前提） |
| FUSE_VER / DECODE | 0x1 / 0x1 | 签名槽位信息（M2 资产核对） |
| FEAT 0x823800（ECC PLM） | 0xffffff8f | 与 GA10x 锁定基线一致 |
| **FEAT 0x823804（FEAT_OVR_PLM）** | **0xffffff8f** | **锁定基线，与全系 Ampere 相同——钳制门就在这里** |
| FEAT 0x823808/80c/810/814 | 0x182 / 0x00888888 / 0x00aaaaaa / 0x233 | 其他 feature 类别 fuse 派生值（M2 对照解码） |
| **FEAT 0x823818（FEAT_READOUT_1）** | **0x00410080 ≠ 0（两次引导稳定）** | **算力钳制存在！** 九个 3 位字段中 #2/#5/#7 = 2（约 1/4 速率），其余 0 |
| **FEAT 0x82381c（SS0）** | boot1: 0x17142071 / boot2: 0x05142071 | ≠GA102 默认 0x00000001——**且引导间有漂移**（最高 nibble 变化；n=2，M2 模拟器按"SS0 冷值非确定"建模） |
| **FEAT 0x823820（SS1）** | boot1: 0x00000004 / boot2: 0x00000005 | 同上，引导间漂移 |
| FEAT 0x823824/828/82c | 0 / 1 / 0xa | 记录 |
| FEAT 0x823b00（行重映射 PLM） | 0x00004b8f | 与 WPR 重锁型 PLM 同族 |
| AD1xx CYA / CFG | 0x2d / 0x80000007 | CFG≠4090 的 0x8000000a——**逐 SKU 差异实证** |
| AD1xx XVE_PLM | 0xffffff8f | 锁定基线 |
| AD1xx BAR1_CAP_MIRROR | 0x0003fc00 | 记录 |
| **Ada 链 WPR_PLM 0x1fa7c4** | **0x0004cb8f** | **与 cmp170hx 已知模式逐位吻合**：WPR/WPR_CFG 在原厂驱动加载后重锁值——冷态即此值，四写链目标在位 |
| **Ada 链 WPRCFG_PLM 0x1fa7cc** | **0x0004cb8f** | 同上 |
| **Ada 链 FBPA_PLM 0x9a0148** | **0xffffff0f** | 与任务书 §7.4 Ada 链"观察项，卡 0xffffff0f 记非致命"记录完全一致（社区在真卡上见过的同一个值） |
| NV_PROM 64KiB | crc32 0xd4a7daa2，magic 0xeb7eaa55 | 本卡身份钉死（0xaa55 ROM 签名 ✓，三引导一致） |

**SS0/SS1 引导间漂移（n=3）**：SS0 = 0x17142071 / 0x05142071 / 0x04140071（三个不同值，
高位字节漂移+低位偶变），SS1 = 0x4/0x5/0x4；FEAT_READOUT_1 恒 0x00410080——
漂移只在覆盖寄存器冷值，不在生效读出。sim 已按此建模。

**M1 判据核对（本机）**：采样落盘 ✓；badf 分桶（发现新桶 0x56xx，分类器已扩展）✓；
FEAT 簇 13/13 可读（存在性=是，PL0 可读）✓。

**两个采样器 bug 同轮修复**：FEAT 扫描 Range<u32> 按字节迭代（36 次非对齐读被
计为 ok——教训：迭代语义与读取宽度要对齐建模）；0xbadf5620 无桶（加 BADF_VARIANT
兜底桶 + 读错误独立计数）。

## AD107M 第二张卡（RTX 4050 Laptop，10de:28a1）——首次实机（2026-10-26，混合输出模式）

同一 G614JV，卡换成 4050。事务在 layout 读数处失败（VGA 毒值把整条 layout 拒掉），
FLR+链回 bootmgfw 正常。

| 寄存器 | 值 | 解读 |
|---|---|---|
| FB_SIZE_MIB | 可读（6GB 量级） | 显存已训练，不是 D3cold |
| **VGA_WORKSPACE 0x625f04** | **0xbadf1301** | **新毒值（0x13xx 桶）**——该寄存器门控。只有 bit3 参与布局判决，bit3=0 → 与健康卡走同一条"FB 顶 1MiB 保留"路径 |
| SEC2_CPUCTL 0x840100 | 0xbadf5620 | 与 28e0 相同的冷态域门控（复位后可读，scrubber 通过） |
| GOP 句柄数 | **2** | iGPU 也在 console 里 |

**结论（2026-10-26，v0.1.3）**：该卡全链验证通过（WPR2→booter→四写→选择器→FLR 存活），
FP16 tensor 22 → 38~42 TFLOPS（1.7~1.9×）。门控寄存器经回退未造成任何后续影响。

**同机同模式的对照**（2026-10-26，28e0 混合关机、只有 dGPU 的 1 个 GOP、全程跑完：
VGA 读回正常 → layout 走 bit3=0 → vga=FB 顶；FWSEC WPR2 验证、booter 点火、四写、
选择器穿 FLR）。因此 0xbadf1301 的关联量是"**dGPU 是否是 console owner**"而不是"混合模式"：
dGPU 主显时该寄存器正常，iGPU 占 console 时被门控。第三种状态是 iGPU-only 关机——
dGPU 冷、无 GOP、BAR0 全 0xffffffff（claim 前采样即此现象，claim 后仍全 1 即 D3cold）。
混合模式下的第四种：休眠/唤醒后 dGPU 停在 D3（resume 不走 POST、不跑 devinit），
同样读到 0xbadf1301；独显直连则休眠与重启均正常。

**修复**：`layout_from_registers` 的五个输入改 raw 读，FB/MMU 锁仍严格拒毒，
仅 VGA workspace 容毒并回退（log 打出原值与 MUX 提示）。教训沿用 §3.3：毒值先分桶再
决定"可用/不可用"，全 BADF 一刀切成错误会丢掉可用信息。

## GB206（RTX 5060 Ti，10de:2d04）——社区采样（issue #168，2026-11-04）

非本机；Linux 6.12.113、驱动 615.71.10、`nvidia` 绑定态逐字 `ioread32`（**不是**
pre-bind 冷采样；GB202 的熔丝镜像为 boot-static，故应等于 pre-bind，但这在 GB206
上只是推论）。GB202/GB203M 两列来自同一探针的对照卡。50 系无事务路径，
`asset_family()` 继续跳过；本节只作只读档案。机理见
[blackwell-gb20x-status](blackwell-gb20x-status.md)。

| 寄存器 | GB206 | GB202 | GB203M | 解读 |
|---|---:|---:|---:|---|
| PMC_BOOT_0 | 0x1b6000a1 | 0x21120000 | 0x1b3000a1 | GB206 ✓（die 0x1b6） |
| FUSE_SS_FMLA16 0x8207d4 | 0 | 0 | 0 | 满速 |
| **FUSE_SS_FMLA32 0x8207d8** | **3** | **3** | **3** | **钳制源** |
| FUSE_SS_IMLA1/2/4 | 3/3/3 | 3/3/3 | 3/3/3 | 钳制源 |
| FUSE_SS_DP/FFMA/IMLA0/IMLA3 | 0 | 0 | 0 | 满速 |
| FUSE_FEAT_OVR_DIS 0x8203f0 | 0 | 0 | 0 | 覆盖通路存在 |
| FUSE_EN_SW_OVERRIDE 0x820040 | 0 | 0 | 0 | 只管 floorsweep，不管速率 |
| FUSE_SS_PLM 0x8200fc | 0xfffffbff | — | 0xfffffbff | **不是**有效门 |
| **FEAT_OVR_PLM 0x823804** | **0xfffffb8f** | **0xfffffb8f** | **0xfffffb8f** | 读 0xf / 写 0x8：**仅 PL3 可写** |
| **FEAT_READOUT_1 0x823818** | **0x00c36180** | **0x00c36180** | 0x00c36180 | 三卡逐字节相同；字段非速率 |
| FEAT_READOUT_0 0x823814 | 0x00000233 | 0x00000233 | — | ECC 标志 |
| FEAT_OVR_ECC 0x82380c | 0x00888888 | 0x00888888 | — | ECC |
| FEAT_OVR_ECC_1 0x823810 | 0xaa2a2aaa | 0x002a2aaa | — | 逐卡方差 |
| SECURE_FEAT_OVR_QUADRO_WR_SEC 0x82038c | 2 | 2 | — | 逐卡 |
| SM_ISSUE_RATE_MODIFIER 0x504204 | 5 | 5 | — | 可写但 inert |
| GFW falcon CPUCTL 0x110100 | 0xbadf5720 | 0xbadf4100 | — | 不可读/缺失（无 FWSEC 阶段） |
| SS0 0x82381c | 0x06277073 | per-boot | per-boot | nibble `3 7 0 7 7 2 6 0`，enable 位全清 |
| SS1 0x823820 | 0x00025005 | per-boot | per-boot | 同上 |

写入梯（每次写后读回、随后恢复原值）：`SS0=0x88888888` / `SS1=0x8` /
`PLM=0xffffffff` **0/3 接受，读回不变**——静默丢弃。

`0x00c36180` 正确解码（位移表 `0,3,7,10,13,16,19,22,28`，字段序
`imla0 fmla16 fmla32 ffma imla1 imla2 imla3 imla4 DP`）= `0 0 3 0 3 3 0 3 0`；
连续 3-bit 解码会得到四个越界的 6。`fmla32=3` 实测约半速，不是 Ada 枚举的 1/8。

## 已知对照值（社区来源，供采样比对）

| 寄存器 | GA100/GA102 锁定冷值 | 来源 |
|---|---|---|
| FEAT_OVR_PLM 0x823804 | 0xffffff8f | cmp170hx（全系 Ampere 一致） |
| FEAT_OVR_ECC_PLM 0x823800 | 0xffffff8f | 同上 |
| 0x823B00 行重映射 PLM | 0xffffff8f | 同上 |
| FEAT_READOUT_1 0x823818 | 0x016db6ed（170HX 锁定） | cmp170hx |
| WPR2 lo/hi | 0/0（FWSEC-FRTS 前恒 0） | 任务书 §3.2 |
| FUSE_OPT_FEAT_OVR_DIS 0x8203f0 | 0x0=总开关未烧（存在前提） | cmp170hx |

### Turing FEAT 回锁值对照（2026-11-02 补录，来源 travis-mcdevitt fork `2654419`）

| 值 | 语义 | 证据 |
|---|---|---|
| **0xffffffef** | TU102（MSI 2080 Ti 22G，子系统 1462:371d）在驱动接管后的 **PLM 重锁写回值 = bits5/6 置位**，≠ 冷态默认 0xffffff8f——回锁值存在板/驱动版本方差，采样轮不应假设固定回锁模式 | guarded-auto build 004 三试（driver 610.88 / GSP 610.57.01）：001/002 停在 fork 本地守卫误判（预期 0xffffff8f 而板回锁 0xffffffef），003 按 0xffffffef 判定全绿；归因与过程见 fork 提交 `2654419`（docs/evidence/msi-tu102-guarded/） |

采样影响：PLM 翻转验证与"重锁读回"对照点应把 0xffffffef 列为 TU102 合法回锁态；GA102/Ada 侧是否同样板卡相关待采（开放问题 #12）。

2026-11-08 起口径更新：三代交棒默认**门开**（`0xffffffff`；TU106 keep-open bench 见 `docs/evidence/tu106-1f07/keep-open-2026-11-08/`），上表 0xffffffef/0xffffff8f 回锁读数此后只在 `\tx-relock.txt` marker 臂或平台/驱动重锁事件中出现——用金丝雀与 SS 对区分（FAQ 回锁章）。
