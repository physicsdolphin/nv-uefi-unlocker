# 双卡 RTX Pro 6000 Blackwell（GB202 2bb5）只读面 + fuserow 拒读根因（owner 采集，2026-11-28）

环境：Ubuntu 24.04.5 / kernel 6.8 / 开放内核模块 580.173.02 / Hygon C86 7285 /
Lenovo SR658H V2 / SB off。两卡同型（SUBSYS 204E:10de）。

## 只读面要点

- `FEAT_PLM = 0xfffffb8f`（**PL3-only 写**，GB20x 家族形态，与 blackwell-gb20x-status 表一致）；
  `FEAT_SS0/SS1` 每卡不同（binning：0x06335050/0x00010006 vs 0x22775040/0x00042000），
  READOUT_1 全速——**pro 线 Blackwell 与 A6000（GA102）一样不额外锁 FEAT 门**（门是代际收紧非 SKU 行为）；
- `SM_ISSUE_RATE_MODIFIER` 两卡 7 vs 5（**每卡 binning 新面**）；`SECURE_FEAT_OVR_QUADRO_WR=2` 两卡同；
- `FBPA_PLM = 0x80842bcf`（高位还带 trim 数据的混合形）；
- IFF_RECORD `0x04590434`：START=52 LAST=89（**GB202 行域 90 行**，Ada 832/Ampere 576 的
  又一 per-die 域数据点；FUSEADDR rest=0 与 LAST 不等=首例 `!=FUSE_ADDR`）；

## fuserow 拒读根因（不是 bug，是硬件门）

```
FUSECTRL_PLM = 0xfffffb8f   bit4=0：PL0(host) 对 fuse 块的写被 PRI 静默丢弃
PLM_PL0_WE   = bit4=0       （dump 自证行）
```

- 写 FUSEADDR 行号这一步即被丢弃 → `pass 1 row 1: FUSEADDR did not latch`；
- **row 0 "成功"是巧合**：resting 地址本来就是 0，读回 0==0 假通过（该行值 0x213a4e97
  双卡一致，是 GB202 的行 0 真值——唯一可见的一行）；
- 对照：TU106 fuse 块 PLM 0x210f8 全开、GA102 `0xffffffff` 全开（四卡 A6000 fuserow 全绿）、
  **GB20x `0xfffffb8f` PL3-only**——fuse 控制器块的 host 写权限逐代收紧，
  与"GB20x 无 SEC2 段无载具"同一封堵方向（plm-and-rate-decode.md 谱系第四数据点）；
- **不可修**：非预检排序问题（0.8.15 已修的那个），是 PRI 比较器上无位可借。
  工具侧已做的=诊断精化（0.8.17 + py 同步）：拒锁存时读 FUSECTRL_PLM，
  PL0-write 关即明确报"fuse block PLM-gated against host writes"而非误导性的
  "controller not in the expected state"；row!=0 才判失败（row 0 假通过不再计入）。

## 0.8.18 复测轮（同日追加，diag-unlock-2x-0818.txt）

新诊断文本首战即中：`FUSEROWS ABORT: ... FUSECTRL_PLM 0xfffffb8f has PL0-write CLOSED:
the fuse-block registers are PLM-gated against host writes (observed on GB20x
professional parts; ...)` ——措辞限缩版（不再说 GB20x family shape）。双卡一致。
FBPA_TRIM 每卡差（0xee/0xff vs 0xdd/0x11/0x0f/0xf0/0x11）、CANARY_B 每卡随机
（0xe82f043c vs 0x84472f5d）——与 A6000 的每卡 binning 图谱一致。

## nvidia-smi BAR=128GB 与 ssdiag BAR0=16TiB 矛盾问题 + BAR4-7 编号来源

**不矛盾——两个输出在测不同的东西**：

- nvidia-smi 的 "BAR1 Memory Usage" 报的是**驱动映射的 FB 孔径**（BAR1/LFB），
  96GB 卡 + 32GB headroom = 128GB 地址空间是**驱动/RM 侧的约定**；
- ssdiag 的 ReBAR 表读的是 **PCIe Resizable BAR capability 的 config 空间**
  （per-BAR cap/control 寄存器对）——BAR0 行 `supported [2G/4096G/524288G/
  8388608G/16777216G]` = 该 BAR 的**可声明尺寸档位**（2G/4T/512T/8P/16P——
  Blackwell 的超大档是新解码），`current disabled` 是我们 PL0 直读 config 时
  control 里的 size_idx=0 + 未使能位——**不是驱动实际分配**。两者测量点不同
  （cap 空间 vs RM 映射），并存即合理。

**BAR4/5/6/7 的编号**：PCIe 规范允许单功能设备最多 **6 个 BAR（BAR0-5）**
（config 0x10-0x24 六个 32 位槽）。ssdiag 表里出现 BAR6/7 是因为 **64 位 BAR
占两个槽位**：BAR0(64位)=槽0+1、BAR2(64位)=槽2+3、BAR4(64位)=槽4+5——
但表中编号是**capability 寄存器的 BAR index 字段**（ctrl/cap 高位 nibble），
Blackwell 报了 8 个 index 是**设备侧把 64 位对声明成了两个独立 index**
（0x3c00/0x1122/0x0024/0x0c22/0x0025/0x0001/0x0001 的 index 位域各异）——
即"编号膨胀"是 **GB20x 的 capability 解码差异**，不是设备真有 8 个物理 BAR。
同表 A6000（GA102）只报 BAR1/3/4/5 四个、TU106 报三个——per-die 的 capability
声明面本来就不同。工具侧待办（登记）：GB20x 的 cap 位域解码需要校准（cap
0x18810026 的档位列表含 512T/8P/16P 级，超出既有 2^n 表假定时按原始位打印，
现表已如此，但 index 字段与新档位语义标注是开放项）。
