# GB205（`10de:2f18` / RTX 5070 Ti Laptop）只读寄存器、PROM BIT 与张量钳制评估

> 对象：NVIDIA GeForce RTX 5070 Ti Laptop GPU（PCI `10de:2f18`，SUBSYS `1558:6586`，46 SM，计算能力 `12.0`，VBIOS `98.05.31.00.44`）。
>
> 本文记录 **GB205**（`PMC_BOOT_0 = 0x1b5000a1`）的只读硅片/固件结构证据与跨平台张量钳制评估。所有操作均为**只读**，未对 GPU 寄存器、VBIOS、FMC/FSP 或功耗策略执行任何写入。

## 1. 证据口径与采集环境

| 数据集 | 采集日期 | 驱动 | VBIOS | 工具与来源 |
|---|---|---|---|---|
| BAR0 寄存器 + 拓扑 + PROM BIT 元数据 | 2026-11-22 | `616.64` | `98.05.31.00.44` | 提权只读 `ssdiag --prom-bit`；脱敏摘录见 [gb205-bar0-prom-metadata.txt](gb205-bar0-prom-metadata.txt) |
| 多精度稳态 GEMM + 配对 FP16 累加 A/B | 2026-11-22 | `621.01` | `98.05.31.00.44` | `PyTorch 2.11.0+cu128` + [blackwell_clamp_eval.py](../../../tools/bench/blackwell_clamp_eval.py)；原始 JSON 见 [clamp-eval-621.01.json](clamp-eval-621.01.json) |

注：BAR0 寄存器与 PROM BIT 元数据采自换驱动前的 `616.64`，两者反映的是硅片熔丝、特权门与 VBIOS（`98.05.31.00.44`，换驱动前后未变）的静态/硬连线结构；吞吐与精度误差则在升级到 `621.01` 后使用稳态采样重测。

## 2. BAR0 寄存器与拓扑核对（第四个 Blackwell 芯片样本：GB205）

`PMC_BOOT_0 = 0x1b5000a1`（bits `[31:20] = 0x1b5` → **GB205**）。与仓库既有的 GB206（`0x1b6`）、GB202（`0x2112`/`0x1b2`）、GB203M（`0x1b3`）样本逐字对比：

| 寄存器 | 偏移 | GB205（`2f18`） | 与 GB206/GB202/GB203M 对比 |
|---|---:|---:|---|
| `PMC_BOOT_0` | `0x000000` | `0x1b5000a1` | GB205 A1（`0x1b5`） |
| `FEAT_OVR_PLM` | `0x823804` | `0xfffffb8f` | 四卡逐字节相同（仅 PL3 可写） |
| `FEAT_READOUT_1` | `0x823818` | `0x00c36180` | 四卡逐字节相同；非连续解码 `0 0 3 0 3 3 0 3 0` |
| `FEAT_READOUT_0` | `0x823814` | `0x00000233` | 与 GB206 / GB202 一致 |
| `FEAT_OVR_ECC` / `ECC_1` | `0x82380c` / `0x823810` | `0x00888888` / `0xaa2a2aaa` | 与 GB206 一致 |
| `FUSE_SS_FMLA32` | `0x8207d8` | `3` | 四卡逐字节相同 |
| `FUSE_SS_IMLA1/2/4` | `0x8207e0/e4/ec` | `3 / 3 / 3` | 四卡逐字节相同 |
| `FUSE_SS_DP/FFMA/FMLA16/IMLA0/3` | — | 全 `0` | 四卡逐字节相同 |
| `FUSE_FEAT_OVR_DIS` | `0x8203f0` | `0` | 覆盖总闸未熔断 |
| `FUSE_EN_SW_OVERRIDE` | `0x820040` | `0` | 与跨卡一致 |
| `FUSE_SS_PLM` | `0x8200fc` | `0xfffffbff` | 与跨卡一致（非有效 FEAT 写门） |
| `SECURE_FEAT_OVR_QUADRO_WR` | `0x82038c` | `2` | 与 GB206 / GB202 一致 |
| `FEAT_SS0` / `SS1` | `0x82381c` / `0x823820` | `0x56027063` / `0x00032004` | 预驱动影子态，enable bit 均清（非解锁） |
| `GSP_CPUCTL` / `SEC2_CPUCTL` / `SEC2_EXCI` | `0x110100` / `0x840100` / `0x8400d0` | `0xbadf5720` / `0x00000010` / `0xbadf5720` | 与 GB206 `ssdiag` 实测（`x99-f8d-gb206`）完全同形 |

**Floorsweep 拓扑与 CUDA SM 数闭环验证：**

- `GR_TOPO_409604 (0x409604) = 0x06030005`（byte3 = 6 GPC 槽位，byte0 = **5 活跃 GPC**）；`STAT_GPC (0x820c1c) = 0x00000fe0`（bits `[4:0] = 0`）。
- `TPC_GPC (0x820c38)`: GPC0=`0xe1`（4 TPC 活跃）、GPC1=`0xe1`（4 TPC）、GPC2=`0xe0`（5 TPC）、GPC3=`0xe0`（5 TPC）、GPC4=`0xe0`（5 TPC）、GPC5..11=`0xff`。合计 **23 TPC = 46 SM**，与 CUDA/PyTorch 上报的 `sm_count = 46` 精确一致。
- `GB20X_PES (0x822a78)`: GPC0..4 均为 `0x8`（每 GPC 3 活跃 PES），GPC5..11 为 `0xf`；`OFA_DISABLE (0x822a68) = 0`。
- `STAT_FBP (0x820d38) = 0xf8`（3 活跃 FBP），`LTCNT = 6`（每 FBP 2 个 LTC，6 个活跃 LTC 均为 3/4 slice）。

## 3. VBIOS PROM BIT 与微码表结构（`--prom-bit` 只读元数据）

对 `BAR0+0x300000`（1 MiB `NV_PROM` 窗口）执行只读元数据扫描（不转储固件字节）：

- PROM 前段存在约 207 KiB 填充；校验 `PCIR`/`NPDS`/`RGIS` 签名可自动跳过 `0x33c00` 处的无效 `55AA`（PCIR 偏移为 0），定位主 x86 ROM 于 `PROM+0x35400`、`BIT` 头于 `PROM+0x361f0`（checksum OK，20 tokens）、expansion adjust `= 0x4d400`。
- token `0x70 v2` 指向位于 `PROM+0xa25a1` 的微码表（`ver=1, hdr=6, entry_size=6, slots=35`）：**29 个空槽**（`app=0, target=0, desc_rel=0`）+ **6 个非空表项**（4 个落在 1 MiB 窗口内，2 个超出 1 MiB `NV_PROM` 窗口）：

| APP ID | Target | 名称 | DESC_OFF | 描述符版本 | ENC（bit2） | 描述符大小 | `stored_size` |
|---:|---:|---|---:|---|---|---:|---:|
| `0x07` | `0x06` | — | `0x0adcd4` | V6 | **YES** | 80 B | `0x20600`（129 KiB） |
| `0x18` | `0x01` | — | `OOB`（`rel=0xceb04`, `abs=0x11bf04`） | — | — | — | — |
| `0x19` | `0x01` | — | `0x0ce324` | V6 | **YES** | 80 B | `0x20f00`（131 KiB） |
| `0x15` | `0x01` | — | `0x0ef364` | V6 | **YES** | 80 B | `0x5500`（21 KiB） |
| `0x23` | `0x0e` | `APP_0x23` | `0x0f48f0` | V6 | **YES** | 80 B | `0x27000`（156 KiB） |
| `0x24` | `0x0f` | — | `OOB`（`rel=0xdb504`, `abs=0x128904`） | — | — | — | — |

结论边界：
1. 经典 Turing/Ampere/Ada 的 App ID（`0x01 PRE_OS`、`0x05 LIC`、`0x08 LS_UDE`、`0x45/0x85 FWSEC`、`0x49 SCRUBBER`）在 GB205 表中全部消失，替换为新编号与 **80 字节 V6 描述符**。
2. 4 个窗口内可读描述符（含 [blackwell-gb20x-status.md](../../blackwell-gb20x-status.md) §4 列出的 `app 0x23`，`target=0x0e`）均置位 `vDesc bit2`（加密标记）；另外 2 项（`0x18`、`0x24`）位于 1 MiB `NV_PROM` 窗口之外，仅凭 BAR0 窗口无法读取其描述符，亦不能仅凭 bit2 推断具体加密算法或密钥源。

## 4. 稳态张量钳制与精度权衡评估（驱动 `621.01`）

使用跨平台脚本 [blackwell_clamp_eval.py](../../../tools/bench/blackwell_clamp_eval.py)（`N=8192`，每档持续 ≥ 2.0 s 稳态执行并过滤爬坡/尾部空闲采样，全部档位稳定在 135–140 W 功耗墙）：

| 路径 | 迭代数 | 吞吐 | 活跃 SM 时钟 | 稳态功耗 | `FLOP/clk/SM` | 满速上限 | `ratio` | 相对 L2 误差（`16×16` vs CPU FP64） | 判定 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `fp16_fp16acc` | 150 | 82.41 TFLOP/s | 2100 MHz | 138.6 W | 853.1 | 1024 | **0.83** | `3.24e-03` | 未钳制快路径对照 |
| `fp16_fp32acc` | 101 | 53.72 TFLOP/s | 2310 MHz | 137.7 W | 505.5 | 1024 | **0.49** | `1.96e-04` | **半速钳制** |
| `bf16_fp32acc` | 113 | 60.41 TFLOP/s | 2692 MHz | 137.6 W | 487.8 | 1024 | **0.48** | `1.75e-03` | **半速钳制** |
| `tf32_fp32acc` | 57 | 30.15 TFLOP/s | 2647 MHz | 138.1 W | 247.6 | 512 | **0.48** | `2.96e-04` | **半速钳制** |
| `fp8_fp32acc` | 206 | 111.80 TFLOP/s | 2392 MHz | 140.2 W | 1016.1 | 2048 | **0.50** | `1.67e-03` | **半速钳制** |
| `int8` | 122 | 65.04 TOP/s | 2670 MHz | 135.0 W | 529.6 | 2048 | 0.26 | — | 仅供参考（受通用 PyTorch kernel 形态限制） |
| `fp32_control` | 33 | 17.98 TFLOP/s | 1995 MHz | 139.5 W | — | — | — | `1.60e-06` | SIMT 对照（不受钳制） |

**顺序平衡的 FP16 累加配对 A/B（`off → on → on → off`，同种子 `1234`、`N=8192`）：**

| 区块 | `allow_fp16_accumulation` | 吞吐 | 活跃 SM 时钟 | 功耗 | `FLOP/clk/SM` | 相对 L2 误差 |
|---|---|---:|---:|---:|---:|---:|
| 1 | `False` | 52.56 TFLOP/s | 2265 MHz | 140.0 W | 504.5 | `0.000196` |
| 2 | `True` | 82.87 TFLOP/s | 2085 MHz | 139.9 W | 864.0 | `0.003244` |
| 3 | `True` | 81.65 TFLOP/s | 2107 MHz | 139.8 W | 842.4 | `0.003244` |
| 4 | `False` | 52.65 TFLOP/s | 2280 MHz | 139.7 W | 502.0 | `0.000196` |

- 正反两对高度一致：`off → on` 原始吞吐 **1.58×**（时钟归一化 **1.71×**）；`on → off` 原始吞吐 **1.55×**（时钟归一化 **1.68×**）。
- 同一 140 W 功耗墙下，未钳制的 `fp16_fp16acc` 单位时钟算力密度更高，导致稳态频率下降约 180 MHz（`2085–2107` vs `2265–2280 MHz`），因此仅看原始 TFLOP/s 会低估硬件发射率差距，**时钟归一化 `FLOP/clk/SM` 才是区分功耗墙与硬件钳制的决定性指标**。
- 数值代价：开启全 FP16 累加后，样本相对 L2 误差从 `0.000196` 升至 `0.003244`（约 16.6×），不能作为 FP32 累加的无条件替代。

## 5. 跨平台复现命令（Linux / Windows）

```sh
# 跨平台张量钳制与精度权衡评估（Linux / Windows 通用，无需内核驱动）
python tools/bench/blackwell_clamp_eval.py --n 8192 --warmup 8 --min-iters 30 --min-seconds 2.0 --json clamp-eval-local.json

# 只读 PROM BIT / 微码表元数据扫描（Linux）
sudo python3 tools/diag-unlock.py --prom-bit

# 只读 PROM BIT / 微码表元数据扫描（Windows 管理员终端）
ssdiag.exe --prom-bit
```
