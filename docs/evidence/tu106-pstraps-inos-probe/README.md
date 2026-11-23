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
