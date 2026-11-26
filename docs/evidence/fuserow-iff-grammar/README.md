# IFF fuserow op 文法：五卡交叉定谳（#237 round 10，2026-11-26）

## 文法（JeusChrit 提出，repo 四份入档独立复现验证）

熔丝行 IFF 池是**op 流**：

- `op = word & 0x3f`：`0x3d` = 双词 `WRITE_IMM`（地址, 值）、`0x3c` = 三词
  `RMW_MODIFY`（地址, and-mask, or-mask）
- `addr = (word & 0xffffffc0) >> 4`
- 池外（0 区与乱数区）非 op 流；`0x47-0x4a` 类高 位全 1 为烧断区

## 五卡解码（AD102=报告者实时；四份=repo 入档）

| die | op 流 | 误析 | 关键结构 |
|---|---|---|---|
| AD102 4090 | 91 词全解（报告者） | — | PJTAG 开/双链（0xce1ba64d/0xce08894e，与其 ad103_disasm_tables.md Appendix B 逐字节互证）/关、PTRIM 0x1373a0/0x1370bc、SEC2 0x8416a4←0x800、GC6_AON 对、PLM **0x21068←0xef8f**、CYA_0_DISG2 RMW `0x8c2c0 & 0xff7fdffb \| 0x00802004` |
| AD107 4060L | 31 ops（0x2ff-0x33f） | 0 | 与 AD102 逐段同形；CYA RMW 逐位同；resting FUSERDATA=0x00802004=末 op or-mask 直接复现；**池前缘 0x2ff/0x300 词未闭合（切分规则开放）** |
| GA106 3060 | 17 ops（结构段） | — | 0x21068←0xef8f、GC6_AON 对、0x1370bc、CYA RMW 逐位同 |
| TU106 2070 | 39 ops（尾段 0xb1-0xff） | 0 | **PLM 对 0x118fec/0x118ffc←0xffffff8f（GFW 族）**、PTOP/PTHERM 成对脚本、RMW 0x20c & 0xdffffff3 \| 0x2000000c |
| TU117 1650 | 35 ops | 0 | 与 TU106 逐段近同形 |

## 硬结论

1. **resting FUSERDATA = 末 op 的 or-mask**（0x00802004，AD102/AD107 双证）；
2. **五卡 IFF 池均无解锁域写**——唯一 PLM 写是出厂闭值（Ada/Ampere 0x21068←0xef8f，
   Turing 0x118fec/0x118ffc←0xffffff8f）→ **IFF 从非解锁载具**的熔丝侧独立证明，
   payload 路径是唯一门的又一证据（同答 Willian-Zhang 专业卡假设线）；
3. FWSEC DMEM 交叉锚（熔丝阵列 vs 反汇编两独立来源逐字节吻合）= 文法最强背书。

## 工具化

ssdiag 0.8.16：`--fuserows` 输出每行附 op 角色（`[WRITE_IMM <addr>]` /
`[value of WRITE_IMM <addr>]` / `[RMW <addr>]` / `[and-mask|or-mask of RMW <addr>]`），
operand 用回扫配头，孤立头只孤立自身不污染窗口；池前缘裸词无注记（开放规则可辨）。

数据源：issue #237 评论 7189687218（AD102 解码+4090 捕获附件）；
docs/evidence/{ad107-28e0,ga106-2504,tu106-1f07,tu117-1f83}/fuserows*.log。
