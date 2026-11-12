# #99 反证轮（2026-11-12）：同一张 1f07、同一驱动、同一 OS，只换掉交棒重锁 → 存活

## 实验问题

[#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99) 的第一现场（v2.1.8，1f06）与同型第二现场（v2.1.8，1f07）都是「EFI 全绿 → 进 Windows 约 20 s 内 `nvlddmkm` 连发 id=14/153 → PF FLR → FEAT 清零」。此前把它归到「621.01/630.20 驱动代」，但同一台 1f07 在 v2.1.11 + 617.42 上实测**不丢**（只回锁、SS 完好）。本轮由报告者自行完成单机反证：把载具从 v2.1.8 换到 v2.1.12-rc2，其他一切不变。

## 配置（除载具外与 11-01 那一轮相同）

- 卡：RTX 2070 `10DE:1F07`，SUBSYS `12AD10DE`，VBIOS `90.06.0b.00.01`（同 11-01 现场）
- 平台：ASRock X99E-ITX/ac，BIOS P3.80，Xeon E5-2675 v3，UEFI，Secure Boot off，BitLocker off
- OS：Windows 11 Insider **26340**.9843
- 驱动 / GSP：**630.20**（`32.0.16.3020`，KMD 09/29/2026）/ **630.03**，`EnableGpuFirmware=1`
  - 630.20 来源已核实：**26340 升级镜像的 in-box 显示驱动**（`setupapi.offline.log` 的 `$WINDOWS.~BT\NewOS` republish；`pnputil` 的 `oem102.inf` 签名者是 Microsoft Windows Hardware Compatibility Publisher）。NVIDIA App 服务端对该卡的 latest 仍是 621.01、查 630.20 详情 404——所以它不是公开 Game Ready 分支，而是 OS 分支驱动
- 显示：**双物理屏（Samsung HDMI + BenQ DP）+ `OrayIddDriver` 保持启用**
- 载具：v2.1.12-rc2 `BOOTX64.EFI` `c99779df…c569aef3`；`ssdiag` 0.7.2；无 `tx-relock.txt`，保留 1 字节 `tx-reconnect-nonrecursive.txt`；无 `i_know_what_am_i_doing.txt`
- 对照：11-01 同一张卡、630.20 / GSP 630.03、26340，v2.1.8，交棒 PLM `0xffffffef`

## 结果

| | 11-01（v2.1.8，默认重锁） | 11-12（v2.1.12-rc2，默认门开） |
|---|---|---|
| 交棒 PLM | `0xffffffef`（重锁 + bits 5:6 sticky） | **`0xffffffff`（门开）** |
| 交棒金丝雀 | 无（旧版无此机制） | `intact` |
| Windows PLM | `0xffffff8f`（关） | **`0xffffffff`（开）** |
| Windows SS0/SS1 | `0x0` / `0x0` | **`0x88888888` / `0x8`** |
| READOUT | `0x00400080` | **`0x00000000`** |
| `nvlddmkm` | 61×id14 + 4×id153，id14 明写 `None → PF FLR` | **0 条新事件、无 TDR** |
| 判决 | `NOT_UNLOCKED` | **`UNLOCKED (override values live)`** |

判据链（全绿）：`POST-BOOT` 载荷只开门不写选择器（shape B）→ 主机写 SS0/SS1 且读回 `0x88888888/0x8` → 金丝雀在链上最后一次复位之后武装 → 卡自身 native-GOP 重建后 `POST-GOP` 读回 `PLM=0xffffffff`、SS 在位、`CANARY=intact` → 交棒 → 系统内 2 min 40 s 读回 `PLM=0xffffffff`、SS 在位、`CANARY: intact`、卡与根口双 `D0` → 6 组 GEMM 之后复读仍相同。

三证配对（FAQ 判读表）：`intact` + SS 在位 + 卡 D0 + 根口 D0 = **既不是复位、也不是断电、也不是写入型抹除**，交棒到读回之间「什么都没发生」。

## 结论：触发者是我们自己的交棒重锁写

v2.1.8 在交棒前**无条件**写 `0x409650 ← 0xffffff8f`（`loader/src/tx.rs` 里的 `RE-LOCK the FEAT block PLM before handoff`）。该值在硬件上读回 `0xffffffef`，SS 当时完好——但驱动 init 随后把它推回 `0xffffff8f` 并发起 PF FLR，FEAT 清零。**重锁写是我们自己的尾巴（FAQ 机制表第 5 行的同族），不是驱动代际问题。**

修复是主线 `061aef8`（2026-11-08，`tx: hand the FEAT gate off OPEN by default on all families; \tx-relock.txt restores the legacy relock`）：默认交棒门开，`tx-relock.txt` 才恢复旧重锁。同日的 keep-open bench（`docs/evidence/tu106-1f07/keep-open-2026-11-08/`）在本项目自己的 1f07 上先给出了同一方向的全绿读数；本轮是第一个**由丢失现场翻绿**的独立证据。

版本下限：`061aef8` 只进 v2.1.12-rc1 及以后。**v2.1.11 在它之前**（tag `a06aecf`，11-08 14:26；`061aef8` 是当日 17:10），所以「换到 v2.1.11 复测」不能验证本修复，要 v2.1.12-rc3+ 或等稳定版。

## 报告者自带的第二个发现：stock 基线跨 boot 不一致

两次**都不是解锁 boot** 的锁态采样，同卡同驱动同 OS：

| 采样 | PLM | SS0 | SS1 | READOUT | CANARY |
|---|---|---|---|---|---|
| 2026-11-12 10:36（升级后连续运行约 10 天的 boot） | `0xffffff8f` | `0x00040000` | `0x00000006` | `0x00400080` | cleared（该 boot 未跑 loader） |
| 2026-11-12 15:02（解锁轮之后正常重启） | `0xffffff8f` | `0x00000000` | `0x00000000` | `0x00400080` | cleared |

`SS0/SS1` 读到 `0x0/0x0` 本身不足以证明「被清零」——这台卡在 stock boot 上就会读到全零。只有「同一 boot 的锁态基线 → 交棒 POST-GOP → 系统内」三段可比时，SS 的变化才有判别力；金丝雀与 `POWER:` 负责区分写入 / 复位 / 断电。该条已回写入 `docs/diag-2.1.11-diag1-protocol.md` §4。

## 性能（报告者 cuBLAS `gemmEx`，FP32 累加，各 6 s × 3 取中位）

| 精度 | N | stock | 解锁 | 倍数 | 同频归一化 |
|---|---|---|---|---|---|
| fp16 | 4096 | 32.79 | 50.96 | 1.55× | 1.77× |
| fp16 | 8192 | 34.83 | 52.37 | 1.50× | 1.71× |
| bf16 | 4096/8192 | 4.32/4.15 | 4.27/4.25 | 0.99×/1.02× | 0.98×/1.03× |
| fp32 | 4096/8192 | 7.59/7.91 | 7.83/7.64 | 1.03×/0.97× | 1.04×/0.96× |

FP16 解锁态**频率更低、功率更低、吞吐更高**（同频 1.71–1.77×），是发射率解锁的指纹；BF16/FP32 平符合「只解发射率、不动 FP32 管线」。

## 反向数据点：#105/#55

本轮双物理屏 + `OrayIddDriver` 启用，无黑屏、无 TDR、解锁保留。在本机上「多屏 / 向日葵」不是充分原因；它属于 #105/#55 的另一类故障（交棒后黑屏/不出图），不是寄存器被抹。

## 局限

- 单机 A/B 只隔离了「载具版本」，没有在 rc2 上再开 `tx-relock.txt` 做单变量回放；重锁为因是**机制一致 + 仓库自有 bench 同向**，因果仍欠一次 marker 回放。
- 卡与平台是报告者自有单台，n=1；`1f06` 开场者机器未复测。
- 完整 bundle 为报告者脱敏后的原文；下表为拆分产物与其 SHA-256。

## 文件

| 文件 | 来源 | SHA-256 |
|---|---|---|
| `bundle-full.txt` | 报告者 gist 全文（分节保留，BOM 已去） | `f1d96f30…09ebbf` |
| `loader.log` | bundle 节 A（v2.1.12-rc2 全量事务日志） | `bb8b35db…53e383f` |
| `ssdiag-in-os-unlocked.txt` | bundle 节 B（解锁读回 + CANARY 判定） | `19d981c5…f61bb17e` |
| `ssdiag-stock-baselines.txt` | bundle 节 D/E（两次锁态基线） | `08c829c0…e5ecf408bc2` |

原始 gist：<https://gist.github.com/KakaruHayate/3737aa7d3989b307267058462ec56695>
