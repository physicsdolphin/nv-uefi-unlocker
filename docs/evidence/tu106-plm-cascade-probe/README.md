# TU106 PLM 级联写试探（2026-11-24）——PLM 寄存器自身写域实测

## 上下文

- 机器：TU106 台架（RTX 2070, 10de:1f07），Windows，in-OS。
- 工具：ssdiag 0.8.9 `--target-offset/--value`（写+读回）。
- 问题：**可读的 closed-stock PLM（0xffffff8f 族）能否被 host 直写翻开？**
  即"系统内级联开门"是否存在——payload 垄断的"初始开门"是否可绕。
- 对象：11 个当前读出 plain 值的 closed-stock PLM，横跨 5 个块
  （GFW / XVE / FUSE-WDATA / PMU / FBPA）。

## 结果（11/11 全拒）

对每个 PLM 写 0xffffffff，读回逐位保持原值；三种原值形状
（0xffffff8f × 8、0xffffff88 × 1、0xffffffcf × 3）行为完全一致，
**无部分 sticky 位**——与 FEAT 0xef erratum（已开 PLM 上 bits 5:6 抵抗
host 清除）形成对照：closed 态的 PLM 一个位都不动。

```
0x118128 GFW_PLM        0xffffff8f -> MISMATCH 0xffffff8f
0x88ab4  XVE_B_PLM      0xffffff8f -> MISMATCH 0xffffff8f
0x210f4  FUSEWDATA_PLM  0xffffff8f -> MISMATCH 0xffffff8f
0x8200e0 (fuse survey)  0xffffff8f -> MISMATCH 0xffffff8f
0x8200e4 (fuse survey)  0xffffff8f -> MISMATCH 0xffffff8f
0x100b10 PMU block      0xffffff8f -> MISMATCH 0xffffff8f
0x100b38 PMU block      0xffffff8f -> MISMATCH 0xffffff8f
0x100b84 PMU block      0xffffff88 -> MISMATCH 0xffffff88
0x9a0148 FBPA_PLM       0xffffffcf -> MISMATCH 0xffffffcf
0x9a0168 FBPA block     0xffffffcf -> MISMATCH 0xffffffcf
0x9a0554 FBPA block     0xffffffcf -> MISMATCH 0xffffffcf
```

## 判读（实验阶段表述）

- **可读 ≠ 可写**：PLM 寄存器自身的写域 = L3，与其守卫对象的可读性无关。
  "开门只能由 payload 干"从推断变为 11 点实测边界（gate-inventory §3.9）。
- 反面既有实证：已开的门后面（FEAT 开后的 SS0/SS1，见 tu106 SS0 语义 bench）
  host 写照常粘住——即寄存器按所在块 PLM 过滤，而 PLM 寄存器本身恒 L3。
- 顺带结论：矿卡 gen2 的"载荷内开 XP3G PLM0"步骤没有免 payload 替代路径
  （§2.3），且 TU106 consumer 的 XP3G PLM0 原生 OPEN 归因（Phase 0 冷态读）
  仍待钉。

## 附

- `poke-log.txt`：ssdiag 原始输出（11 行 MISMATCH）。
