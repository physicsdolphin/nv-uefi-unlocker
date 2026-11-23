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

## 轮三（同日）：S4 休眠（rail-off）判别完成——reset-reload 模型确认，载具革命不成立

S4 断电 + 解锁 boot 后：**PSTRAPS SET0/SET1 全部回厂值**（0x00400080/0x02013000、
override_en=0、BAR1 index 0）+ **config ReBAR cap 0x15 回归**（3 resizable BARs，
BAR1 [256M/128M/64M]）——#135 先例兑现：**straps=上电默认，in-OS 写不活过断电**。
in-OS PSTRAPS 路线定性：会话内诊断/实验工具，无持久化载具价值；§2.4 ReBAR 载具
维持 DXE（NvStrapsReBar）路线，in-OS 活窗增长留给 0x610 型 FB override 思路。
轮二的两个机制 datum（part1 硅掩码→in-OS 上限 8GiB；override_en 抑制 ReBAR cap
广告）保持成立。
