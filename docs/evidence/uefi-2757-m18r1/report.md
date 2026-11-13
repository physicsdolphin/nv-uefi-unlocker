# AD103M · RTX 4090 Laptop（10de:2757）：Alienware m18 R1 Windows UEFI 全链解锁与约 2 倍张量吞吐

2026-11-01，单机单卡。Windows 11 + Alienware m18 R1（`10de:2757`，subsystem `1028:0c49`）。

使用 v0.2.0 release 资产与 `make-usbroot.ps1 -Family ad102`；loader 为 2026-10-28 从 main 用 `cargo efi-tx` 构建（banner v2.1.0）。本地 loader 带一个只影响确认键的机器特定补丁，见下文。完整 `loader.log`、原始 benchmark 输出与本报告放在同一目录。

## 结果

- 一次事务全链通过：`TX: device 10de:2757 -> asset family 'ad102'`、`SCRUB: handoff DONE`、`FWSEC: WPR2 verified`、`SEC2 MAILBOX0=0x67b00001`、`FEAT_PLM=0xffffffff`、`SELECTOR RESULT: SS0=0x88888888 SS1=0x8`、`POST-FLR: verified`，随后链回 `bootmgfw`，NVIDIA 596.49 正常绑定。
- 进入 Windows 后，匹配的 `mma_bench`（20000 iters x 5 reps）：
  - FP16/FP32：88.8 -> 177.4 TF/s（1.9977x）
  - BF16/FP32：88.7 -> 176.0 TF/s（1.9842x）
  - FP8/E4M3：177.3 -> 352.6 TF/s（1.9887x）
  - 控制组：f16/f16 1.0023x、s8/s32 0.9992x、dp4a 0.9989x、ffma 1.0024x，基本持平。
- 这与 Alienware m18 R2（gpu-unlock-ada #76）和 Lenovo 2757 的正结果一致；Clevo 2757 的“事务全绿但吞吐不变”是板级差异（gpu-unlock-ada #4）。

## 环境

| 项目 | 内容 |
| --- | --- |
| GPU | NVIDIA GeForce RTX 4090 Laptop GPU，`10de:2757`（AD103M），76 SMs |
| 子系统 / VBIOS | `1028:0c49` / `95.03.2a.00.47` |
| 机器 / BIOS | Alienware m18 R1，BIOS `1.35.0`，i9-13980HX，64 GB |
| 显示 / 功耗 | Hybrid / Advanced Optimus；125 W 默认 / 175 W 最大；未做 NVOC、功耗墙或超频修改 |
| OS / 驱动 | Windows 11 Pro for Workstations 25H2，build 26220.9757；NVIDIA 596.49 |
| 启动 | UEFI，Secure Boot 关闭；FAT32 测试盘；`v67-ad102-four-plm.bin` 在根目录 |
| 工具 | `nv-uefi-unlocker` v0.2.0 release 资产；`make-usbroot.ps1 -Family ad102`；loader 从 main 构建 |
| 基准 | `mma_bench.cu`（`gpu-unlock-ada/diagnostics/bench`），CUDA 13.3 + MSVC，`20000` iters x `5` reps |

## 关键 log

```text
[INFO ] loader: TX: device 10de:2757 -> asset family 'ad102'
[INFO ] loader: TX: V67 present - unlock mode (family-specific write chain)
[INFO ] loader::tx_fwsec: SCRUB: handoff DONE - SEC2/GSP domains un-gated
[INFO ] loader::tx_fwsec: FWSEC: WPR2 verified
[INFO ] loader::tx_booter: POST-BOOT: FEAT_PLM=0xffffffff SS0=0x57310161 SS1=0x1
[INFO ] loader::tx_booter: POST-BOOT: SEC2 MAILBOX0=0x67b00001 WPR2_HI=0x3ff6e00
[INFO ] loader::tx_booter: POST-BOOT: WPRCFG=0xfffff0ff FBPA=0xffffff0f WPR=0xffffffff
[INFO ] loader::tx: SELECTOR RESULT: SS0=0x88888888 SS1=0x8
[INFO ] loader::tx_booter: POST-FLR selectors: ss0=0x88888888 ss1=0x8
[INFO ] loader::tx: POST-FLR: verified (selectors survive AON)
[INFO ] loader::tx: FINAL: transaction complete, all DMA released
[INFO ] loader::tx: HANDOFF-OUT: ss0=0x88888888 ss1=0x8 readout1=0x00000000 feat_plm=0xffffffff wpr2_hi=0x0 pmcsr=0x0008 dstate=D0 | tu10x: plm=0xbadf5040 ss0=0xbadf5040 ss1=0xbadf5040 | tu10x_unlock=false ss0_unlock=true ss1_unlock=true plm_open=true
```

`tu10x` 的 `0xbadf5040` 是 Ada 上该只读探测的预期值，与本次 2757 事务无关。完整日志见 `loader.log`。

## 匹配基准

同一个 `mma_bench.exe`、同一驱动、同一 Windows power plan。

- Stock：完全关机，拔 AC 约 30 秒，启动 Windows。
- Unlocked：UEFI 启动并带 `v67-ad102-four-plm.bin`，随后进入 Windows。

| mode | stock TF/s | unlocked TF/s | ratio |
| --- | ---: | ---: | ---: |
| f16/f16 | 177.3 | 177.7 | 1.0023 |
| **f16/f32** | **88.8** | **177.4** | **1.9977** |
| **bf16/f32** | **88.7** | **176.0** | **1.9842** |
| s8/s32 | 354.2 | 353.9 | 0.9992 |
| fp8/e4m3 | 177.3 | 352.6 | 1.9887 |
| dp4a | 88.0 | 87.9 | 0.9989 |
| ffma | 42.2 | 42.3 | 1.0024 |

原始输出：`mma-stock.txt`、`mma-unlocked.txt`。

## 本地 loader 说明

- 本地 loader SHA-256：`6707B44EC20239301EE52ED09FF950D12D07E429C25D8E67228387ED7F516188`。
- 标准 loader 备份 SHA-256：`ECA8248A891EA74D21E208B07F0B4BEA72E918FCDC2B4D89EB6B09EC3A58A708`。
- 本地构建只改 `keywait.rs`：在确认等待中忽略持续发出的 Z 键（`0x1a` / `'z'` / `'Z'`），事务代码未改。
- 这台机器的 Z 键在启动阶段会卡住并向固件持续送 `0x1a`；标准 loader 的 `wait_confirm` 和 `wait_confirm_armed` 会把它当作非 Space 输入而 ABORT。该补丁只是本机的确认键规避，不拟作为项目功能；keywait 的可配置容错仍按维护者意见另行讨论。`loader.log` 来自这个本地构建。

## 局限

- 单机、单卡、一组 stock/unlocked；这是 Windows/UEFI 数据点，不是 `gpu-unlock-ada` 的 Linux 内核模块结果。
- `mma_bench` 是寄存器驻留的 instruction issue rate；本机没有 CUDA 12 的 cuBLAS runtime，因此未跑 cuBLAS GEMM。
- 没有做长时间 soak、功耗墙或超频测试；没有修改 VBIOS/EC/BIOS。
- 2757 的 Clevo 报告（gpu-unlock-ada #4）是事务成功但吞吐不变；Alienware m18 R1（本报告）和 m18 R2（#76）是正结果。结论是板级相关。
- `loader.log` 含 PCI 地址和寄存器值，不含 UUID 或主机名；本目录不含 NVIDIA 固件、生成 payload 或二进制。

## 参考

- gpu-unlock-ada #132：<https://github.com/JustAResearcher/gpu-unlock-ada/issues/132>
- gpu-unlock-ada PR #133（Windows UEFI 证据的 docs-only PR）：<https://github.com/JustAResearcher/gpu-unlock-ada/pull/133>
- gpu-unlock-ada #4（2757 Clevo/Lenovo 线程）：<https://github.com/JustAResearcher/gpu-unlock-ada/issues/4>
- gpu-unlock-ada #76（Alienware m18 R2）：<https://github.com/JustAResearcher/gpu-unlock-ada/issues/76>
- nv-uefi-unlocker #110（本机 Z 键确认冲突）：<https://github.com/physicsdolphin/nv-uefi-unlocker/issues/110>

## 证据文件

`SHA256SUMS` 覆盖本目录的 `report.md`、`loader.log`、`mma-stock.txt`、`mma-unlocked.txt`。

| 文件 | SHA-256 |
| --- | --- |
| `loader.log` | `F554BCF506ED2B79C1DCF89550CA83AA8D8464B7342A49DC0EBE24CA79C13934` |
| `mma-stock.txt` | `CA5398AF7584747EEACACB94FBB2C8FB946784792A269C734081182CD0F44892` |
| `mma-unlocked.txt` | `216AF9BB10BB9E4BEE0EF87B7E887E64AF8C633A97227BA28BE635CD758BB480` |

原始 Windows 采集的 `mma-stock.txt` 是 UTF-16LE（原始 SHA-256 `86DD71947518246AD522B986C1A761873BDF7412E01F0D3497ACEE64E3A8B152`），`mma-unlocked.txt` 是带 BOM 的 UTF-8；本目录转存为无 BOM 的 UTF-8，内容不变，表中哈希对应提交后的文件。
