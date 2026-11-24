# TU106 PSTRAPS in-OS 写探针（2026-11-24）——§2.4 载具问题重开

## 结果

- PSTRAPS_SET0 0x101000 / SET1 0x10100c 主机直写**粘住**（ssdiag --target-offset
  readback OK），并经**设备管理器禁用-启用循环保留**（owner 实测）。
- 位段布局（NvStrapsReBar `SetupNvStraps.c`）：BAR1 size = SET0[15:14]（part1，
  2 bits）+ SET1[22:20]（part2，3 bits）**和值编码**（0=64MiB…10=64GiB；
  NvStrapsReBar 对 size≥10 写 part1=3/part2=7，size 3..9 写 part1=2/part2=size−2）；
  **override-enable = SET0 bit31 + SET1 bit31**（写时两字都置位）。
- 本卡现值 SET0=0x00400080 / SET1=0x02013000：part1=0+part2=0 → 64MiB 档、
  override_en=0——与 #135 先例同形（straps=上电默认，活窗=枚举期写）。

## 64GiB in-OS 配方（待测）

```
poke 0x101000 <- 0x8000c080   (part1=3 + bit31；其余位保留原值)
poke 0x10100c <- 0x82713000   (part2=7 + bit31；其余位保留原值)
```

三步判别：
1. 写+读回；
2. 重跑 inventory 看 config ReBAR cap "BAR1 supported" 列表是否长出 64G/32G/16G
   ——判 device 侧 cap 是否 PSTRAPS 派生；
3. 温重启 → inventory 看 BAR1 current size + Windows "Large Memory Range"。
   若 stock BIOS 枚举按新 cap 分配 → §2.4 载具从"主板固件 DXE 驱动"坍缩为
   "in-OS 一次写 + 重启"。

回退：`0x101000 ← 0x00400080` / `0x10100c ← 0x02013000`（清 bit31 恢复原值）。
风险注：override 生效时序未证——立即 resize 可能动活窗（驱动 BAR1 映射失效→TDR），
重启即恢复；NvStrapsReBar 原生场景在 DXE 枚举期写（驱动映射前），in-OS 时序属未测。

## 轮二（同日）：part1 硅掩码 + 8GiB 生效 + ReBAR cap 消失

```
poke 0x101000 <- 0x8000c080 -> readback MISMATCH 0x80000080 (dropped or masked)
poke 0x10100c <- 0x82713000 -> readback OK
```

- **SET0[15:14]（part1）被硅写掩码**：写入值含 bits[15:14]=3，读回为 0——同寄存器
  内逐位段写保护（bit31 与 bit7 可写、[15:14] 拒）。→ **in-OS 可达 BAR1 index 上限
  = part2 单独 = 7 = 8GiB**（2070 显存 8GB 恰为经典 ReBAR 目标；16/32/64G 需 part1，
  host 侧拿不到）。
- SET1 全粘（part2=7 + bit31）→ **BAR1 straps = index 7 = 8GiB，override_en=1**。
- **config ReBAR cap 0x15 整个消失**（原 "3 resizable BARs, BAR1 [256M/128M/64M]"
  → "no 0x15 capability"）——straps override 下设备改呈固定尺寸 BAR，可调 BAR
  机制被抑制（机制 datum：cap 广告是 straps 派生的）。
- **勘误（我方）**：轮二 SET0 算值漏保留原 bit22——应为 0x8040c080，实给
  0x8000c080 → bit22 已被清。bit22 不在 ssdiag 解码字段内（BAR0=SET1[19:17]、
  BAR3=SET1 bit23、override_en=SET0 bit31），语义未解码，回写 0x80400080 恢复。
- 待温重启判别：①BAR1 是否按 8GiB 枚举（依赖 BIOS Above-4G Decoding；放不下=
  Code 12，断电冷启恢复）②straps 是否复位重载洗掉（#135 先例预测洗；FEAT AON
  先例存疑）。若 straps 洗掉而仍要 in-OS BAR1 增长 → 下一候选 = #211 的 0x610 型
  FB override（GA102 枚举期写、活过 FLR）的 Turing 对应物。

## 轮三（同日）：S4 休眠（rail-off）判别——straps 不活过断电

S4 断电 + 解锁 boot 后：**PSTRAPS SET0/SET1 全部回厂值**（0x00400080/0x02013000、
override_en=0、BAR1 index 0）+ **config ReBAR cap 0x15 回归**（3 resizable BARs，
BAR1 [256M/128M/64M]）——#135 先例兑现：**straps=上电默认，in-OS 写不活过断电**。

## 轮四（同日）：驱动重载 vs 温重启判别——结构性死锁（catch-22）

owner 实测与判读（勘误轮三后的初读）：

- **restart-display-driver：straps 寄存器保留**（index 7 / override_en=1 仍在），
  但 **BAR1 活窗不变**（仍 256 MiB）——驱动重 init 不触发 PCI 资源重分派，
  窗口分配方（PnP 资源管理器）不重读 straps；
- **温重启：straps 洗掉**——reset 域重锁存（该轮多跑了一次解锁 boot，不影响判读）。

**结构性结论（catch-22）**：让固件按 override 枚举 8GiB 需要 straps 活到枚举时刻，
而枚举只发生在复位后——**复位洗 straps，保留 straps 的路径（驱动重载）又不重新
分配**。两条腿互斥，in-OS PSTRAPS 路线**结构性关闭**（比"可能活过温重启"更强）。
§2.4 载具终审：**DXE（NvStrapsReBar）单路线**——它赢在写时机就在枚举窗内；
0x610 型 FB override 为唯一剩余假想路（未探）。轮二机制 datum（part1 硅掩码
→in-OS 上限 8GiB；override_en 摘 ReBAR cap 广告）保持。
附（同日负控）：RM 自带 `OverrideFbSize` 注册键（NV_REG_STR_OVERRIDE_FB_SIZE，
宿主侧堆覆盖）在 GSP 模式下**无效**（16384/4096 两档+驱动重启：FB_SIZE_MIB 仍
badf、5G+ 分配照常）——注册表侧无 BAR1/堆增长捷径，0x610 仍需 MMIO 面模板。

## 轮五（同日）：NV_30_DXE 模板移植——0x610 FB override 在 TU106 生效，catch-22 破口

**地址同一性**（文考+实测双证）：config 0x610 = BAR0 0x88610（XVE PRI 页=config
space 镜像：LnkCap@0x88084、LnkCtl2@0x880a8 穿透已证、AD1X_BAR1_CAP_MIRROR
0x88bbc=config 0xBBC）——gen2 配方的"武装位"寄存器与 GA102 FB override 是**同一
寄存器**（ssdiag 字典名 VSEC_HIERARCHY = NV_30_DXE 名 FB_SIZE_OVERRIDE）。

模板（Ga102ReBarFix.c）：mask 0xF80 / value 0xC80（bit11=enable，bits[10:7]=9）。

实测：
```
基线  0x88610=0x1001  0x88bbc=0x1c00 (supported={64M,128M,256M})  0x88bc0=0x801 (control: BAR1, size_idx 8)
poke  0x88610 <- 0x1C81 -> readback OK
读回  0x88bbc = 0x0ffc00  —— supported 长出 10 位！
config 头行确认: BAR1 supported [32G/16G/8G/4G/2G/1G/512M/256M/128M/64M]
```

- **"0x610=GA102 专属"证伪**：TU106 同款接口，一条 MMIO 写就把 ReBAR 广告从
  3 档开到 10 档（8GB 卡上 32G 档只是广告，实际可用受 VRAM 钳制）。
- ReBAR control 镜像同位：0x88bc0=0x801 = {BAR index=1, size_idx=8(256MiB)}；
  8GiB = size_idx 13 → **0x0d01**。
- 载具意义： straps catch-22 不适用此路——0x610 写在 config 空间、复位后即可
  重写，而 **loader 本就跑在复位后/Windows 枚举前的时窗**——若 disable-enable
  能触发重分派则 in-OS 全程可解；否则 loader 加"0x88610|=0xC80 + 0x88bc0←idx"
  两写 = NvStrapsReBar 的 DXE 功能装进现成载具，零 BIOS 改动（#211 侧证：窗口
  一旦按 override 分配，FLR 洗不掉）。

## 轮六（同日）：control 写实测——尺寸阈值钉死（1G 活 / 2G 立死 / UAC 必死）

owner 实测（勘误我方"live resize 协议违规"初判——机制是**尺寸阈值**而非规程）：

- `0x88bc0 <- 0xd01`（8GiB）→ 内核级卡死，重启后重写可复现，偶尔 reset 键失效；
- **1G（idx 10）不死、512M 活**——阈值在两者之上；2G 起立死。**板阈值有谱系
  记载**（NvStrapsReBar README:92："for some boards you may need to use lower
  values, to limit BAR size to 4 GB or 2GB"）——Z170-A/BIOS 3802 实测上限 1G；
- **512M/1G 状态下弹 UAC prompt 必死**（安全桌面切换触发驱动路径踩进超出
  其预期的 BAR 窗）——运行时 resize 后的态是**易碎态**，不是可用态。
- 机制整合：control 写入**立即重锁** BAR 尺寸（无 MSE 拆除环节），新窗 ≤ 某阈值
  时覆盖面尚可存活、超阈值立死——in-OS 运行时 resize = **诊断工具**，非交付态。
- **载具结论不变且更硬**：loader 引导窗写入（Windows 枚举时按 cap 分配，驱动
  从第一天就以大窗初始化，无运行时改锁、无 UAC 脆弱性）= 唯一交付形态；
  尺寸必须做成**逐板参数**（本台板 1G；现代板 8G+ 待别的平台验）。
  4G=idx12→0x0c01 / 8G=idx13→0x0d01 留档备 loader 用。

## 轮七（同日）：loader `\tx-rebar.txt` EFI 阶段首战——死在首次 XVE 触碰；系统内全方向关门

- **EFI 阶段（复位后、console 活渲染、无驱动）**：size_idx 13 → `REBAR: BEGIN` 后
  硬死（CPU 单核吃满）；size_idx 10 → 同位死、死得慢些。日志粒度限制：BEGIN 与
  首条读回行之间夹读+写+读回三步，死点未分辨——但这是本 boot **首次 XVE 页
  （0x88xxxx）访问**（FEAT/PMC 早读全活），嫌疑=EFI console 正经 BAR1 活渲染
  （逐行画屏），孔径页状态变化撞活显示引擎。loader 代码已加细粒度日志+只读探针
  模式（下一轮分辨）。
- **系统内全方向关门（收口）**：缩到 128M 也死（UAC 卡死 + Ctrl+Shift+Win+B 内核
  死）——运行时改锁在放大/缩小×全尺寸档全部不稳定，**in-OS 路线正式关闭**。
- **GPU-Z 前提表**：Above 4G Decode=Yes、GPU 硬件支持=Yes、BIOS ReBAR=No
  （Z170 无原生 ReBAR=ReBarUEFI 类模块的存在理由）——平台侧无毛病。
- 剩余假想路（阶梯）：①只读探针分辨 EFI 期 XVE 可达性（零写，下一轮）；②若读
  活→claim 变体（GOP 断开后写，杀掉活渲染因子）；③若写能落→Windows 引导枚举
  是否接 control（ReBarUEFI 同机制）。每步硬死风险真实（reset 键曾失效），按
  ladder 走、不跳步。

## 轮八（同日）：rung 1 全绿——claim+SBR 窗口内 MMIO 写入侧打通

loader.log（`\tx-rebar.txt`=10 + `\tx-sbr-gpu.txt`，双路线 build）：

```
claim → SBR (设备验证存活, config 基线恢复) →
REBAR: mmio 0x88610 read 0x00000000 (pre-write)      ← SBR 洗净后全新值（非 0x1001：VSEC 域复位默认 0）
REBAR: mmio 0x88610 wrote 0xc80 -> readback 0xc80 OK  ← XVE 页在 claimed+SBR 窗口可用（第一轮 hang 根因=firmware-bound 态，11-22 教训精确复现）
REBAR: mmio 0x88bbc BAR1 supported mask 0x000ffc00 (10 sizes)
REBAR: mmio 0x88bc0 wrote 0xa01 (BAR1, size_idx 10) -> readback OK
```

**写入侧三要素全部闭环**：可达性（claim 窗口解决）、advertisement（mask 10 档）、
selection（control 读回一致）。剩 Windows 引导枚举的"接取"环节。
**路由淘汰记录**：PciIo 扩展 config（>0x100）本固件 UNSUPPORTED（rung-2 轮读回
门 fail-closed 拦截，无盲写）——config 直写路线出局，MMIO 镜像在纪律窗口内复用。
**Windows 侧首报 256M**：该 boot 为休眠恢复（hiberboot）——resume 不重枚举 PCI，
内核镜像带旧分配恢复，任何引导期写入都不可能在 resume 中生效。待完整冷启动
（关快速启动+完整关机）重验；冷启动若仍 256M，剩余杠杆=loader 内清 BAR1 分配
（0x88014/0x88018 归零 → Windows 仲裁器按新 size 重新分配）——未实装，待冷启动
结果决定。

## 轮九（同日）：冷启动终局——**Windows 接取成功，BAR1 = 32 GiB**

冷启动（hiberboot 关闭）后完整链路打通：

- Windows pci.sys 按 ReBAR cap 仲裁，**取 advertisement 内最大可容纳档**（我们写的
  control=idx10 只是建议值——Windows 是主动分配者，掩码开到 32G 且 64 位窗口放得
  下 → 选 idx15=32G 并自写 control）；ssdiag：`current 32768 MiB (size_idx 15)`；
- **终态与 #211 GA102 20G 改卡逐字同形**：驱动/GSP-RM 初始化后清 0x610 override 位
  → cap 广告回落 3 档（0x1c00）而窗口持有 idx15——ssdiag 读数（supported 3 档 +
  current 32G）= 该已记录标准终态的 TU106 复现；
- GPU-Z：Resizable BAR **Enabled**、"enabled in BIOS: Yes"（终态与原生 ReBAR 机器
  不可区分）；nvidia-smi：FB Total 8192 MiB / BAR1 Total 32768 MiB；owner 确认
  Windows 稳定运行。32G 孔径对 8G 显存无害（CPU 窗口 ≠ VRAM 容量，驱动只在 VRAM
  内映射）——20G 改卡社区跑 32G BAR 是日常形态；
- **易失重放哲学自洽**：温重启洗 config → 回 256M 原厂态；插盘引导 → loader 重写
  → 回 32G。不插盘零痕迹；
- 已知成本：claim 后 GOP 不重建 → 黑屏至登录屏（tx 流程的 native-GOP 恢复机制
  可后续并入）；
- **未竟组合**：rebar 模式目前**替代**解锁事务（互斥 marker）；Turing 解锁链本就
  跳过 FLR（FEAT PLM 不过 FLR）→ rebar 对可 ride-along 在 tx 窗口尾部（booter
  后、交棒前），一次引导同时拿解锁+32G BAR——待立项。若要精确限档（如 1G/8G），
  需 override 字段 bits[10:7] 非 9 值的编码（当前 9=全开 10 档）——未解。

## 轮十（同日）：字段编码地图 + --bar1-scan 硬件裁决——休眠恢复自愈、512G 可达

**字段编码地图（\tx-rebar-scan.txt，claim+SBR 窗口内 0..15 全扫）**：
`field v = top size_idx(v+6)` 线性到 **field 13 = idx19 = 512 GiB**（field 14 饱和
同 13；field 15 退化 3 档）——**advertisement 可达 512G，>32G 编码存在**（4090
48G 需 ≥64G=field 13 起）。cap-mirror（0x88bbc）写探针 read-only——field 是唯一
且足够的杠杆。Ada 侧注意：XVE 区 PLM closed（0x88ff4 badf），0x610 直写可能吃
PLM 挡，待实卡；Ada 本有 CYA/CFG 64G payload 链（cf3566d0）。

**--bar1-scan（ssdiag 0.8.12）休眠恢复会话裁决——幻象模型死亡**：
`control 0xbc0 = 0xf01 (32G)`、BAR1 base = **0x2000000000**（Windows 给 32G 窗的
64 位分配）、边界扫描 **31G+ 全 decoded（0/16 ff）**——硬件孔径真实 32G、驱动
信念一致，**无幻象**。修正后的生命周期模型：SBR 洗 control（rung-1 实测 0x801）
→ Windows 仲裁自写 0xf01+分配 → **休眠保存把 config 状态收进内核镜像 → resume
回写 → 硬件 re-latch** → 32G 跨休眠持有。唯一清除=不插盘冷启/断电。
0x610 恢复后读 0x1001（厂值形状，非清零）——#211 "override 被清" 的 TU106 细节
修正：GSP-RM 重写回厂位形，广告位消失而窗口持有。
**WDDM 行为旁证（owner 实测）**：5G+ 显存计算的进程 BAR1 工作集仅 2MB——
**窗口重指向模式**，CPU 侧暴露面极小，休眠态计算负载存活自洽。
**官方引用（2MB 窗口 = WDDM 正式机制）**：WDDM 2.0 起的 **CPU Host Aperture**
（learn.microsoft.com/windows-hardware/drivers/display/cpu-host-aperature；
VidMm 经可编程 CPU host aperture 访问 local memory 段，驱动以 DXGK_CPUHOSTAPERTURE
caps 声明、OS 经 DxgkDdiMapCpuHostAperture 映射/移出分配——"2MB used"=当前映射
工作集）；NVIDIA 语义=NVML nvmlDeviceGetBAR1MemoryInfo（"BAR1 is used to map the
FB so that it can be directly accessed by the CPU or by 3rd party devices (P2P)"，
docs.nvidia.com/deploy/nvml-api）。性能分层：计算负载（GPU 内部+DMA 通路）不受
影响（实测存活）；窗口重指向的开销只在 CPU 直访密集模式（remap 寄存器写/次）；
48G 4090 需 ≥64G 的刚需=CPU 可达性（远端显存 DMA-only 不可见），field 13=512G
广告直接解决。

## 轮十二（同日）：Code 12 风暴归因 + 512M 端到端交付 + GOP 重连并入

- **Code 12 风暴归因 = 固件资源表漂移**：成功轮参数（10/9）复现失败、多档全
  Code 12，**BIOS 载入默认值后立即恢复**——多轮 SBR/异常引导搅乱了 POST 期
  BAR 分配表。现场处置纪律：**Code 12 风暴 → BIOS 载默认值**（非 Windows 侧）。
- **预算阶梯实测（Z170-A）**：512M/1G 交付、2G+ Code 12——Above-4G 预算硬上限
  ~1G（老板限制非机制限制；现代板预算大得多）。**Windows 尊重 select 值**
  （1G 轮=1G、512M 轮=512M，逐位一致）——pci.sys 是"预算内尊重建议"型仲裁，
  非无脑取最大（32G 成功轮=当时 32G 放得下）。
- **Code 43 负控（8/9）**：select=256M（与厂态同值）+ 广告改写 → GSP-RM 仍拒
  ——ReBAR control 被非固件代理改写本身留 tamper 判据；禁用-启用可恢复。
- **GOP 重连并入 rebar 分支（新 build）**：pair 后 native-GOP 恢复全绿
  （console rebound 3840x2160、IO|MEM Enable、display path verified）——
  "黑屏到登录屏"成本消除。
- **512M 端到端交付**：select idx9 → Windows 分配 512M @ 0xc0000000（32 位
  空间够用不搬家），control 读回逐位一致；0x610=0x1001（驱动 init 重写回厂形）
  ——#211 标准终态再现。
- **--bar1-scan 边界扫描歧义（field-proven）**：+384 MiB（512M 孔径内）读
  all-ff=未写入 VRAM 内容；+31 GiB（孔径外）读 non-ff=aliasing/abort 数据——
  **内容采样非解码判据**，权威孔径=config control 0xbc0 读回。ssdiag 已改
  （verdict 降级 informational + zone 标注 inside/beyond）。
- **板阈值修正**：512M/1G 运行时改锁后 UAC 仍致死（早前轮），但**引导期分配**
  512M/1G 稳定——阈值属于运行时态，交付态（引导期分配）无此限制。

## 轮十一（同日）：SBR 必要性消融——**SBR 硬必要**，无 SBR 轮死于首次 XVE 读

A/B 单变量（同参数 10/9，仅去 SBR）：`REBAR: BEGIN` 后 4ms 内硬死（单核吃满），
**pre-write 读行未及打印** = 死在首次 XVE 读。与 rung-1（有 SBR）逐字同位置对照：
- 有 SBR：0x88610 pre-write 读 = **0x00000000**（SBR 洗净后的复位默认）→ 可写；
- 无 SBR：首次读即死 → XVE 页在**非复位域**下不可达。
**机制定读**：XVE 页时钟/电源域由复位门控——只有 SBR 之后才可达；owner 观感
"GO 没断联"实为死后残留：单核吃满=MMIO 读挂总线、console 画屏冻结在旧帧，
GOP 协议实例其实已 Disconnect（日志 `protocols 6 -> 4` 在案），扫描引擎按最后
配置继续扫旧帧——顺序没错，缺的是 SBR 这个前置条件。
**配方定型（SBR 为硬前置，不可拆）**：`\tx-rebar.txt`（sel/adv）+ `\tx-sbr-gpu.txt`
成套使用。待办清零后剩一个免费优化观察项：某轮若有兴趣可试"SBR 后再读 0x88610
的值域分布"以确认 0x00000000 是复位默认（rung-1 已有一次读数支持）。
