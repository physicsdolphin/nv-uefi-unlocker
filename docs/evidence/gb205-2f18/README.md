# GB205（`10de:2f18` / RTX 5070 Ti Laptop）只读寄存器、PROM BIT 与张量钳制评估

> 对象：NVIDIA GeForce RTX 5070 Ti Laptop GPU（PCI `10de:2f18`，SUBSYS `1558:6586`，46 SM，计算能力 `12.0`，VBIOS `98.05.31.00.44`）。
>
> 本文记录 **GB205**（`PMC_BOOT_0 = 0x1b5000a1`）的只读硅片/固件结构证据与跨平台张量钳制评估。所有操作均为**只读**，未对 GPU 寄存器、VBIOS、FMC/FSP 或功耗策略执行任何写入。

## 1. 证据口径与采集环境

| 数据集 | 采集日期 | 驱动 | VBIOS | 工具与来源 |
|---|---|---|---|---|
| BAR0 寄存器 + 拓扑 + PROM BIT 元数据 | 2026-11-22 | `621.01`（并对照同机先前 `616.64` 样本） | `98.05.31.00.44` | 本 PR 新版提权只读 `ssdiag --prom-bit`；脱敏摘录见 [gb205-bar0-prom-metadata.txt](gb205-bar0-prom-metadata.txt) |
| 多精度稳态 GEMM + 配对 FP16 累加 A/B（`N=8192` / `N=4096`） | 2026-11-22 | `621.01` | `98.05.31.00.44` | `PyTorch 2.11.0+cu128` + [blackwell_clamp_eval.py](../../../tools/bench/blackwell_clamp_eval.py)；原始 JSON 见 [clamp-eval-621.01.json](clamp-eval-621.01.json)（`N=8192`）与 [clamp-eval-621.01-n4096.json](clamp-eval-621.01-n4096.json)（`N=4096`，400 次，与 `bench_v2.py` 同规模） |

注：BAR0 寄存器、PROM BIT 元数据与稳态 GEMM 基准全部统一在同一台机器、同一套驱动 `621.01` 与 VBIOS `98.05.31.00.44` 上采集；与同日早先 `616.64` 采集对比，全部熔丝、PLM、拓扑与 PROM BIT 结构逐字节一致，而预驱动影子态 `SS0/SS1` 跨冷启动变化（`616.64`：`0x56027063` / `0x00032004` → `621.01`：`0x76027063` / `0x00030004`，enable bit 均清），`WPR2` 窗口随驱动版本微调（`0x02fa7e00..0x02fb0400` → `0x02fb8800..0x02fc1000`）。

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
| `SM_ISSUE_RATE_MODIFIER` | `0x504204` | `5` | 与 GB206 / GB202 一致 |
| `SEC2_PLM` | `0x8403c4` | `0x00000b0f` | 与 GB206 一致（全读、零写） |
| `FEAT_SS0` / `SS1` | `0x82381c` / `0x823820` | `0x76027063` / `0x00030004` | 预驱动影子态，enable bit 均清（非解锁；跨引导变化） |
| `GSP_CPUCTL` / `SEC2_CPUCTL` / `SEC2_EXCI` | `0x110100` / `0x840100` / `0x8400d0` | `0xbadf5720` / `0x00000010` / `0xbadf5720` | 与 GB206 `ssdiag` 实测（`x99-f8d-gb206`）完全同形 |
| `GA100_SM_OVR_RGN` | `0x820a00..0x820b1c` | `0x820a40..0x820abc` 32 字全 `0`，两侧全 `0xbadf5040` | 与 AD102/AD103 解码孤岛形态一致 |

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
3. 针对该 VBIOS 完整导出文件（1,961,983 字节，补齐 `0x18`/`0x24` 共 6/6 个 V6 描述符）的 80 B V6 字段布局、`0xA00` 残差、`+30` 可读标签（`0x23=BHUB`、`0x24=_RUN`）以及 Class A（`0x19`/`0x15`）20 字节二级 manifest 算术闭合链分析，详见同目录 [v6-descriptor-re.md](v6-descriptor-re.md)（不含固件字节转储）。

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
- 数值代价：在 `N=8192` 下开启全 FP16 累加后，样本相对 L2 误差从 `0.000196` 升至 `0.003244`（约 16.6×）；在 `N=4096`（400 次，[clamp-eval-621.01-n4096.json](clamp-eval-621.01-n4096.json)）下，四个 FP32 累加路径同样稳定落在半速（FP16 `0.49×`、BF16 `0.47×`、TF32 `0.49×`、FP8 `0.54×`），FP16 累加为 `0.80×`，相对 L2 误差从 `0.000205` 升至 `0.002123`（约 10.4×）。随着内积维度 `K` 翻倍，FP16 累加误差按预期继续放大，而 FP32 累加保持在 `~2e-4` 水平，不能将 FP16 累加作为无条件替代。

## 5. 跨平台复现与测试验证（Linux / Windows）

- **Windows 11（Python 3.13 + Rust MSVC）**：`cargo test`（`ssdiag` 29/29 + workspace 全绿）、`cargo efi` + `check_efi.py`（确认 read-only build）、`python -B -m unittest` 22/22，以及上述 `N=8192` / `N=4096` 实机基准。
- **Linux（同机 WSL2 Ubuntu 26.04，内核 `6.6.114`，Python 3.14.4）**：原生运行 `tools/tests/` 全部 22/22 单元测试、`diag-unlock.py --inventory` 与 `blackwell_clamp_eval.py` 的 `nvidia-smi` 元数据探测全部通过（WSL2 未安装 PyTorch，CUDA 基准于 Windows 主机运行）。

```sh
# 跨平台张量钳制与精度权衡评估（Linux / Windows 通用，无需内核驱动）
python tools/bench/blackwell_clamp_eval.py --n 8192 --warmup 8 --min-iters 30 --min-seconds 2.0 --json clamp-eval-local.json

# 只读 PROM BIT / 微码表元数据扫描（Linux）
sudo python3 tools/diag-unlock.py --prom-bit

# 只读 PROM BIT / 微码表元数据扫描（Windows 管理员终端）
ssdiag.exe --prom-bit
```
