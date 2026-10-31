# nv-uefi-unlocker

[中文](README.md) | [English](README.en.md)

**GTX 16 + RTX 20 + RTX 30 + RTX 40 系列通用的 UEFI 预驱动 NVIDIA GPU 解锁工具。**

解开 NVIDIA 显卡的 Tensor 计算性能限制。从 U 盘启动，解锁后自动进入 Windows 或 Linux 或任意操作系统，无需刷写 VBIOS；断电后恢复原状。

[观看项目宣传片 ↗](https://www.bilibili.com/video/BV1d6xN66ECM/)

## 兼容性速览

| 系列 | 支持情况 |
| --- | --- |
| GTX 16 · Turing | 理论兼容，欢迎实机验证 |
| RTX 20 · Turing | 全系兼容；部分型号有已知问题，见完整矩阵 |
| RTX 30 · Ampere GA10x | 全系兼容，支持笔记本与台式 |
| RTX 40 · Ada | 全系兼容，支持笔记本与台式 |
| Pascal 及更早 / GA100 / RTX 50 | 不支持 |

[完整兼容性矩阵与逐卡实测](docs/compatibility.md) · [常见问题](docs/faq.md)

## 开始使用

### 1. 获取工具并检查前置条件

从 [Releases](https://github.com/physicsdolphin/nv-uefi-unlocker/releases) 下载并解压发布包。发布包不附带 NVIDIA 固件，制盘脚本会自动下载指定版本并校验、生成所需文件。

准备一个 FAT32 U 盘，先备份其中的文件。目标机器须使用 UEFI 启动，关闭 Secure Boot，或为 EFI 自签并信任证书。笔记本优先使用独显直连，从系统选择“重启”后进入一次性启动菜单。

**Turing 用户先在目标 Windows 的管理员 PowerShell 运行一次，再重启：**

```powershell
.\enable-turing-unlock.ps1
```

重启后确认 GSP 固件已运行，再制作 U 盘。Linux Turing 目标同样需要 GSP-RM；详见 [GSP 前置说明](docs/faq.md#turing)。Linux 用户还应检查 [RTD3](docs/faq.md#linux)，避免进入系统后掉电丢失解锁。

### 2. 制作启动 U 盘

在解压目录运行下面的命令。40 系选择 `40`，30 系改为 `30`，20 / 16 系改为 `20`。

Windows PowerShell：

```powershell
$gpuSeries = "40"
.\make-usbroot.ps1 -Family $gpuSeries
```

Linux / WSL / Git Bash：

```bash
gpu_series=40
bash make-usbroot.sh --family "$gpu_series"
```

Windows 需要 PowerShell 5+；Linux 版需要 glibc ≥ 2.34（如 Ubuntu 22.04+ / Debian 12+）。首次生成需要联网，下载或校验失败时先处理错误，不要换固件版本。

生成的文件位于本地 `usbroot/`。将脚本完成时列出的固件文件复制到 U 盘根目录，再把 `BOOTX64.EFI` 放入 `EFI/BOOT/`。其余抽取中间文件不需要复制。

```text
U 盘（FAT32）
├── EFI/BOOT/BOOTX64.EFI
└── 固件文件（脚本末尾列出的 .bin 文件）
```

保留脚本生成的 `v67-*.bin` 解锁文件。[完整文件清单与用途](docs/faq.md#usb)。

替换已有 `EFI/BOOT/BOOTX64.EFI` 前先备份。不要复制到系统 ESP，也不需要调整永久启动顺序。完整文件用途、异机制盘、多卡选择和其他选项见 [USB 文件与选项](docs/faq.md#usb)。

### 3. 启动并验证

1. 从固件的一次性启动菜单选择 U 盘。
2. 版本屏按 **空格或回车**运行；其他键或 30 秒不操作会中止事务并交回系统引导器。目标 GPU 暂时黑屏或冻结末帧属正常现象。
3. 等待工具完成并自动链回系统，日志在 U 盘根目录 `loader.log`。
4. Windows 下用发布包的 `ssdiag-x86_64-windows.exe` 检查系统内状态，并用相同负载比较解锁前后的性能。Linux 可在源码仓库运行 `sudo python3 tools/diag-unlock.py`。详见 [验证与修复](docs/faq.md#ssdiag)。每次冷启动需要重放。

遇到 STOP、异常或黑屏卡死，**同会话不要重试**；完全断电后拔 U 盘，走原系统引导，再按 [恢复说明](docs/faq.md#recovery)检查。

让 Agent 帮忙部署：[部署指南](AGENTS.deploy.md)。可以直接把下面这句话交给它：

> 请根据 https://github.com/physicsdolphin/nv-uefi-unlocker/blob/main/AGENTS.deploy.md，帮我安装 nv-uefi-unlocker，为我的显卡解锁。

## 自行构建（可选）

使用发布包可跳过本节。需要 Rust 工具链；Windows 选择 MSVC 版本。在仓库根目录运行：

```text
rustup target add x86_64-unknown-uefi
cargo build --release
cargo efi-tx
```

EFI 程序生成在 `target/x86_64-unknown-uefi/release/loader.efi`。用以下命令制作启动文件（40 系示例；30 系改 `-Family 30`，20 / 16 系改 `-Family 20`）：

```powershell
.\release\make-usbroot.ps1 -Family 40 -AssetsExe .\target\release\assets.exe -BootEfi .\target\x86_64-unknown-uefi\release\loader.efi
```

```bash
bash release/make-usbroot.sh --family 40 \
  --assets ./target/release/assets \
  --boot-efi ./target/x86_64-unknown-uefi/release/loader.efi
```

之后按上面的布局复制到 U 盘。[只读诊断、测试与开发说明](docs/faq.md#layout)。

## 与 nvoc 配合

本工具解除 Tensor 计算性能限制；[nvoc](https://github.com/Skyworks-Neo/nvoc) 在系统内提供功耗、电压和超频控制，两者可以叠加。

1. 先完成本工具的解锁，在系统内确认状态并记录基准。
2. 按 [nvoc 文档](https://github.com/Skyworks-Neo/nvoc#readme)选择适用于显卡的控制方式。kmd 功耗墙解锁在移动端可用，10/20 系桌面卡的效果尚待验证；`set-volt-rail-limit` 的电压墙控制适用于 10 系及之后 GPU。
3. 用 [cli-stressor-cuda-rs](https://github.com/Skyworks-Neo/nvoc/tree/main/cli-stressor-cuda-rs) 在相同负载下对比，监控功耗、Hot Spot 和限频原因。

组合后功耗和温度可能大幅超过原厂包络，需自行确认散热与供电余量。[组合测试结果与风险说明](docs/faq.md#nvoc-results)。

## 参考资料与仓库

本项目是社区工作的直接受益者与衍生者，完整致谢见
[docs/ada28e0-uefi-report.md](docs/ada28e0-uefi-report.md) 的同名章节。
开发期实际研读与引用的仓库已 clone 至本地 `reference/`（不入库），按作用分组：

**事务链母本与一手权威**

- [gpu-unlock](https://github.com/Zagitta/gpu-unlock)（GPL-2.0）——事务链移植母本；
  `unlock_610.c` 逐段对照与 mock 测试蓝本
- [gpu-unlock-ada](https://github.com/JustAResearcher/gpu-unlock-ada)（GPL-2.0）——
  AD10x 四写链源头；28e1 报告（Windroid234）是本项目直接依据，并驱动本项目
  三处判据修正（FBPA `0xffffff0f` 接受、scrubber 非致命、EALREADY 不 FLR）
- [gpu-unlock-bar1](https://github.com/84634E1A607A/gpu-unlock-4090-48g-64g-bar1)——
  AD10x 大 BAR1 参考线（未并入主线）
- [cmp90hx-unlock](https://github.com/WildFlash1st/cmp90hx-unlock)——V67 载荷链源头
- [open-gpu-kernel-modules](https://github.com/NVIDIA/open-gpu-kernel-modules)（MIT）——
  引导路径一手权威：Turing 引导路径四处代际差异（BOOT_DIRECT、portal 直写、
  RM_RISCV_UCODE_DESC v4、FWSEC generic-BL）全部据此考据落进代码
- [AD103_win_unlocker](https://huggingface.co/TOTORONG/AD103_win_unlocker)（TOTORONG）——
  Windows EFI 载具工艺先例（GOP 断连/链式交接/事务日志范式）

**Turing 谱系与定址（0x4096xx FEAT 块）**

- [cmp170hx](https://github.com/Consensus-Protocol/cmp170hx)——PLM 语义、booter
  0x89/0x91 状态码字典与验证方法论
- [cmp50hx-unlock](https://github.com/xrip/cmp50hx-unlock)——Turing 谱系实机闭环
  源头：50HX（TU102）31.7× 实测，40HX/50HX 共用 TU102 booter 归档，是 TU106
  全链可行性的实机依据；FEAT 块 `0x409650/64/6c` 定址亦出自此
- [turing-unlock](https://github.com/nlzy/turing-unlock)（nlzy）——Zagitta kmod
  的 Turing 官方改编（Dell 2080 Ti 22GB，FP16 约 1.6×）；TU102 gadget/栈几何
  表与 GSP IDA 库来源；`TU_FEAT_READOUT 0x409668` 引用源（ssdiag 与 loader
  的 Turing readout 同源）
- [travis-mcdevitt/nv-uefi-unlocker](https://github.com/travis-mcdevitt/nv-uefi-unlocker)——
  本项目下游 fork，其 Turing 成功链记录进入 0x91 取证对照
- [tu102-22g-uefi-unlock](https://github.com/chenxluo/tu102-22g-uefi-unlock)——
  TU102 22G 改装卡 UEFI 方案（22G 支持线的同类对照）

**同类 Windows/UEFI 先行工作**

- [rtx30-unlock](https://github.com/craftingmod/rtx30-unlock)——Zagitta kmod 的
  Windows/UEFI 移植（GA10x 同类先行）
- [rtx30-unlock-windows-toolkit](https://github.com/Siselea/rtx30-unlock-windows-toolkit)——
  上者的单卡单显伴侣工具
- [rtx3080-unlock-public](https://github.com/rough2515-hub/rtx3080-unlock-public)——
  RTX 3080 10GB UEFI 实验先行（韩文）
- [KohakUnlock](https://github.com/KohakuBlueleaf/KohakUnlock)——Turing 谱系
  （boot-hook 形态）与 4090 Windows/UEFI 的源码优先研究
- [Turing-unlock](https://github.com/haohervchb/Turing-unlock)（haohervchb）——
  2080 Ti（TU102）易失解锁，2026-10-22 单跑成功记录
- [nv_unlock_win](https://github.com/mirabarukaso/nv_unlock_win)——Titan RTX
  Windows 内核例试验（同目标的另一条路）
- [gpuunlock](https://github.com/miounet11/gpuunlock)——4090/4070 Ti SUPER
  合并发行版

**寄存器/熔丝侧线**

- [NvStrapsReBar](https://github.com/terminatorul/NvStrapsReBar)——推翻本项目
  此前"Turing BAR1 不可扩"的判断；M1 采样 PSTRAPS 解码直接依据
- [ga100-fuse-re](https://github.com/ahnguyen17/ga100-fuse-re) ·
  [gpcprobe](https://github.com/matstatman/gpcprobe) ·
  [170hx-rir-tool](https://github.com/lixiangwuxian/170hx-rir-tool)——GA100
  熔丝/RIR 架构（熔丝语义参考线）
- [p104-gen2-unlock](https://github.com/skbskhud/p104-gen2-unlock)——P104 PCIe
  Gen2 链路工具（PCI 配置空间操作参考，与算力解锁正交）

**逆向工具链**

- [envytools](https://github.com/envytools/envytools)——envydis（fuc5 Falcon
  反汇编，量产 booter 安全段为密文、需密钥在场才可解）+ nvbios
- [faucon](https://github.com/vbe0201/faucon)——fuc5 反汇编备选（实测解不了
  量产 booter）

许可：**GPL-2.0-only**（`LICENSE`）。NVIDIA 固件与 VBIOS 版权归 NVIDIA，按其
原许可条款从钉版公开包本地再生，不在本仓库分发。

- 原理科普：[专业用户版](https://www.bilibili.com/opus/1264436370549506065) · [入门用户版](https://www.bilibili.com/opus/1264493051219279872)
- 社区讨论：[Reddit](https://www.reddit.com/r/overclocking/comments/1xj5jp3/new_uefi_gpu_unlocking_tool_revealed_for_windows/) · [X](https://x.com/RoundtableSpace/status/2115749232980140458)

## Star History

<a href="https://www.star-history.com/?repos=physicsdolphin%2Fnv-uefi-unlocker&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&legend=top-left" />
 </picture>
</a>
