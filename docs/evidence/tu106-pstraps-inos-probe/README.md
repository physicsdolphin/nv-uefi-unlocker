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
