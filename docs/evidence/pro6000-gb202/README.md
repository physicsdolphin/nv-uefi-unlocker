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
