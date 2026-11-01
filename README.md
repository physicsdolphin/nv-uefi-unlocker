# nv-uefi-unlocker

[中文](README.md) | [English](README.en.md)

**你的 GeForce 正以半速跑 AI——这个工具在驱动加载前把它修复。**

[![version](https://img.shields.io/badge/version-2.1.8-brightgreen)](https://github.com/physicsdolphin/nv-uefi-unlocker/releases/latest)
[![GeForce](https://img.shields.io/badge/GeForce-GTX%2016%20%7C%20RTX%2020%2F30%2F40-green)](docs/compatibility.md)
[![platform](https://img.shields.io/badge/platform-UEFI%20%E9%A2%84%E9%A9%B1%E5%8A%A8%20%7C%20Win%20%2F%20Linux%20%2F%20Hackintosh-blue)](docs/faq.md)
[![license](https://img.shields.io/badge/license-GPL--2.0-blue)](LICENSE)
[![stars](https://img.shields.io/github/stars/physicsdolphin/nv-uefi-unlocker?label=stars&color=yellow)](https://github.com/physicsdolphin/nv-uefi-unlocker/stargazers)

**实测 FP16 / BF16 / TF32 张量吞吐最高约 2.6× · FP8 最高约 1.9× · 图像 / 视频生成 +14–75% · DirectML 推理算子最高 +94% · LLM 推理 prefill 最高 +19%（decode +5%）· 无 VBIOS 刷写，断电即复原**（逐卡数据见[兼容性矩阵](docs/compatibility.md)；增益受功耗墙与精度路径制约——LLM 行为双卡 MoE 实测，非微基准外推）

GTX 16 + RTX 20 + RTX 30 + RTX 40 系列通用的 UEFI 预驱动 NVIDIA GPU 解锁工具。

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

[完整兼容性矩阵与逐卡实测](docs/compatibility.md)（含 4090M 2757 / Alienware m18 R1 约 2 倍） · [常见问题](docs/faq.md)

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

在**目标机**（要解锁的机器）的解压目录直接运行：脚本自动识别本机显卡代际，只生成并下载对应固件。

Windows PowerShell：

```powershell
.\make-usbroot.ps1
```

Linux / WSL / Git Bash：

```bash
bash make-usbroot.sh
```

在另一台机器上制作时显式指定代际：40 系 `-Family 40`，30 系 `30`，20 / 16 系 `20`。`-Family all` 把三代固件全部打进同一张 U 盘（一条命令即可服务多种卡）。

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

> 请根据 https://github.com/physicsdolphin/nv-uefi-unlocker/blob/main/AGENTS.deploy.md ，帮我安装 nv-uefi-unlocker，为我的显卡解锁。

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

## 关于作者与团队

作者是一名长期硬件超频与显卡改装爱好者，经手超频魔改维修运维的 NVIDIA 显卡累计超过
数百 块（含大量矿卡与魔改卡），多年从事笔记本、桌面计算机、服务器运维、硬件维修、UEFI BIOS魔改、硬件调试；

本项目是与同学一起进行的学生兴趣研究。正因如此，工具的每个写路径
都按"读回验证 + 证据归档"纪律设计，失败场景的恢复手段见
[安全约束与实现边界](docs/faq.md#safety)。

本项目的开发过程使用了 AI 编程助手。与所有代码相同，AI 产出不享受任何
默认信任：事务代码是上游 GPL-2.0 实现的行为级移植，须先通过行为级模拟器
全绿、再经实机读回验证才进入发布——这条规则对人类与 AI 写的每一行代码
一视同仁。

## 参考资料与仓库

本项目是社区工作的直接受益者与衍生者，完整致谢见
[docs/credits.md](docs/credits.md)（按仓库历史中的实际作用核对归类）。
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

- [ReBarUEFI](https://github.com/xCuri0/ReBarUEFI)（xCuri0，MIT）——30 系+
  ReBAR 平台侧机制权威（枚举期 hook + PCIe ReBAR 扩展能力）与平台大 BAR
  限制补丁目录；[NvStrapsReBar](https://github.com/terminatorul/NvStrapsReBar)
  是其 Turing fork——推翻本项目此前"Turing BAR1 不可扩"的判断；M1 采样
  PSTRAPS 解码直接依据
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
- 社区讨论与报导（语言随条目标注）：
  [Reddit](https://www.reddit.com/r/overclocking/comments/1xj5jp3/new_uefi_gpu_unlocking_tool_revealed_for_windows/)（EN） ·
  [X/RoundtableSpace](https://x.com/RoundtableSpace/status/2115749232980140458)（EN） ·
  [X/Pirat_Nation](https://x.com/Pirat_Nation/status/2116303886604739008)（EN） ·
  [TechPowerUp](https://www.techpowerup.com/354316/new-uefi-tool-lifts-nvidias-tensor-cap-on-rtx-20-to-40-series-cards-in-windows?type=Tools)（EN） ·
  [Wccftech](https://wccftech.com/new-nvidia-uefi-gpu-unlocker-brings-up-to-75-higher-ai-compute-performance-to-rtx-20-30-40-series-gpus-on-windows/)（EN） ·
  [NotebookCheck](https://www.notebookcheck.net/UEFI-level-pre-driver-Nvidia-GPU-unlocker-now-available-for-RTX-2xxx-3xxx-and-4xxx-GPUs-on-Windows-and-Linux.1438153.0.html)（EN） ·
  [Mezha Media](https://mezha.net/eng/news/mezha-2ffa1478104cdc175bc0e096_uefi_pre-driver_nvidia/)（EN） ·
  [Digg](https://digg.com/tech/vfyuouiy)（EN） ·
  [XenoSpectrum](https://xenospectrum.com/en/nvidia-uefi-windows-tensor-unlock/)（EN） ·
  [HotRepo](https://hotrepo.ai/physicsdolphin/nv-uefi-unlocker)（EN） ·
  [HardwareCooking](https://www.hardwarecooking.fr/nv-uefi-unlocker-tensor-cores-rtx-20-30-40-windows/)（FR） ·
  [DNS Club](https://club.dns-shop.ru/amp/digest/187732-instrument-nv-uefi-unlocker-povyishaet-ii-proizvoditelnost-kart/)（RU） ·
  [Playground.ru](https://www.playground.ru/misc/news/novaya_utilita_pozvolyaet_uvelichit_proizvoditelnost_videokart_nvidia_rtx_do_75_v_zadachah_ii-1886614)（RU） ·
  [FanaticosDelHardware](https://fanaticosdelhardware.com/nv-uefi-unlocker-multiplica-hasta-265-veces-el-rendimiento-tensor-de-las-geforce-rtx-en-windows/)（ES） ·
  [OMGPU](https://www.omgpu.com/nvidia-luefi-gpu-unlocker-dope-de-75-lia-des-rtx-20-a-40-sur-windows/)（FR） ·
  [PauseHardware](https://pausehardware.com/nvidia-tensor-rtx-debridees-sous-windows/)（FR） ·
  [HardwarePremium](https://www.hardwarepremium.com/noticias/78275/uefi-nvidia-desbloqueador-rtx-2000-3000-4000/)（ES） ·
  [Gaming-ST](https://gaming-st.com/gpu/nv-uefi-unlocker-windows-rtx20-30-40-tensor-unlock/)（JA） ·
  [Shahrsakhtafzar](https://www.shahrsakhtafzar.com/fa/news/graphic-card/62809-nvidia-geforce-rtx-tensor-unlock-windows-tool)（FA） ·
  [Quasar Zone](https://quasarzone.com/bbs/qn_hardware/views/2085634)（KO） ·
  [GiggleHD](https://gigglehd.com/gg/hard/19358883)（KO） ·
  [GECID](https://ua.gecid.com/news/novij_nstrument_podvoyu_produktivn_st_tenzornih_yader_dlya_v_deokart_rtx_20_40_u_windows/)（UK） ·
  [DicasPC](https://dicaspc.com.br/noticias/nv-uefi-unlocker-libere-ate-75-mais-ia-na-sua-rtx/)（PT-BR） ·
  [Diario da Midia](https://diariodamidia.com.br/autor/thiago-nogueira)（PT-BR） ·
  [GPU-Unlock.net](https://gpu-unlock.net/windows)（EN） ·
  [IT-SVR](https://it-svr.com/svrforum-news-3234984/)（KO） ·
  [HKEPC](https://www.hkepc.com/27140/)（ZH-HK） ·
  [Facebook/PCMR HUB](https://www.facebook.com/PCMRHUB/posts/nv-uefi-unlocker-is-an-open-source-uefi-level-pre-driver-tool-that-removes-nvidi/1546978424130406/)（EN） ·
  [ShiftDelete](https://shiftdelete.net/en/nv-uefi-unlocker)（EN） ·
  [YouTube/Max Herrera](https://www.youtube.com/shorts/AMhmWjjpgFk)（EN） ·
  [GizNewsDaily](https://giznewsdaily.com/nvidia-gpu-unlocker-at-uefi-pre-driver-level-released-for-rtx-2xxx-4xxx/)（EN） ·
  [ProfesionalReview](https://www.profesionalreview.com/2026/11/01/nv-uefi-unlocker-75-mas-rendimiento-ia-rtx-20-30-40/)（ES） ·
  [Facebook/ComfyUI 群组](https://www.facebook.com/groups/comfyui/posts/1122947967144569/)（EN） ·
  [搜狐](https://m.sohu.com/a/1095783758_163726)（ZH） ·
  [网易](https://www.163.com/dy/article/LAM8N7DS05527PMU.html)（ZH） ·
  [X/NFT_Chen](https://x.com/NFT_Chen/status/2116389085682241717)（ZH） ·
  [PCGH Extreme 论坛](https://extreme.pcgameshardware.de/threads/nv-uefi-unlocker-modder-schalten-zusaetzliche-ki-leistung-bei-aelteren-geforce-gpus-frei.679185/)（DE） ·
  [Instagram](https://www.instagram.com/p/DfKvpmvgUoO/)（ZH-HK） ·
  [Tom's Hardware](https://www.tomshardware.com/pc-components/gpus/open-source-usb-tool-unlocks-nvidias-half-speed-gpu-ai-limiter-on-windows-lifts-gpus-fp16-math-performance-up-to-65-percent-uefi-unlocker-covers-rtx-20-30-and-40-series-desktops-and-laptops)（EN） ·
  [PixLib](https://pixlib.ru/article/nv-uefi-unlocker-kak-uskorit-tenzornye-yadra-rtx-do-75-v-zadachakh-ii)（RU） ·
  [ShiftDelete 论坛](https://shiftdelete.net/forum/konular/eski-rtx-kartlara-yapay-zeka-takviyesi-nv-uefi-unlocker.5000006)（TR） ·
  [网易](https://www.163.com/dy/article/LALVCN6L0556OH4N.html)（ZH） ·
  [X/xiax0603](https://x.com/xiax0603/status/2116550883018494257)（ZH） ·
  [VideoCardz](https://videocardz.com/newz/modders-create-automated-tool-to-unlock-ai-performance-on-nvidia-geforce-rtx-20-30-and-40-series-gpus)（EN） ·
  [WindowsForum](https://windowsforum.com/news/nv-uefi-unlocker-lifts-rtx-20-30-and-40-tensor-core-limits-ai-gains-and-secure-boot-risks.450453/)（EN） ·
  [Hacker News](https://news.ycombinator.com/item?id=50272846)（EN）

## Star History

<a href="https://www.star-history.com/?repos=physicsdolphin%2Fnv-uefi-unlocker&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&legend=top-left" />
 </picture>
</a>
