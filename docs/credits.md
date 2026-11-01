# 致谢与参照源（社区贡献详细记录）

README 与 [ada28e0 报告](ada28e0-uefi-report.md)的致谢节指向本文件。条目按
**仓库历史中的实际作用**归类核对（引用痕迹见各 `docs/evidence/`、
[reading-notes](reading-notes.md) 与源码注释），不是泛列相关项目。本地
clone 位置 `reference/`（不入库）。

## 1. 事务链与引导路径（代码层面的直接来源）

- **[gpu-unlock](https://github.com/Zagitta/gpu-unlock)**（Zagitta，GPL-2.0）：
  GA10x kmod 权威范本。`unlock_610.c` 是本仓库事务链的移植母本
  （falcon_reset/HS 启动/WprMeta/FLR），`layout_610.c` 是布局算法原版，
  `tests/test_unlock_adapter.c` 的 mock 模式是本仓库模拟器测试蓝本。
- **[gpu-unlock-ada](https://github.com/JustAResearcher/gpu-unlock-ada)**
  （JustAResearcher 及全部卡片报告贡献者，GPL-2.0）：AD10x 四写链与审计纪律
  的源头。特别感谢 **Windroid234** 的 28e1 报告——与本卡一 ID 之差的完整
  实测（2.00×、稳定性验证、OEM 注意事项），是本项目 M4 立项的直接依据；
  该报告还驱动了本项目三处判据修正（FBPA `0xffffff0f` 接受、scrubber
  非致命、EALREADY 不 FLR）。以及 2705/2805/2882/28a0 等全部 canary 报告作者。
- **[cmp90hx-unlock](https://github.com/WildFlash1st/cmp90hx-unlock)**
  （WildFlash1st）：GA102 V67 链源头（rejoin15 补丁集）。
- **[cmp170hx](https://github.com/Consensus-Protocol/cmp170hx)**
  （Consensus-Protocol）：PLM 位编码语义、四写表、SS0/SS1、"读回是唯一
  判据"方法论；booter 0x89/0x91 状态码字典（falcon-and-booter.md）是两条
  取证线的日常工具。
- **[open-gpu-kernel-modules](https://github.com/NVIDIA/open-gpu-kernel-modules)**
  （NVIDIA，MIT；本仓库钉 610.43.03 源包）：Turing/Ada 引导路径的一手权威。
  Turing 四处代际差异（BOOT_DIRECT、portal 直写、RM_RISCV_UCODE_DESC v4、
  FWSEC generic-BL）全部据此逐文件考据落进代码（kernel_gsp_tu102/falcon/
  booter/frts、BINDATA、BCR/BROM 寄存器定义）。
- **[AD103_win_unlocker](https://huggingface.co/TOTORONG/AD103_win_unlocker)**
  （TOTORONG）：Windows EFI 载具工艺的唯一公开先例——GOP 断连/交棒纪律、
  FLR 与 PCI 恢复时序、containment 设计、阶段日志范式均对照其对齐。

## 2. Turing 谱系与定址（0x4096xx FEAT 块）

谱系源头：Jon Pry《A Canary in the Crypto Mine》（V67 canary 漏洞，DOI
10.5281/zenodo.20916112）→ PZH1gdmu/CMP40HX-Unlock v3.0.0（TU106，
unlock40x_v70.c，MIT，仓库已删）→ 下列各支。TU106 全链可行性由 40HX 项目
实机证明。

- **[cmp50hx-unlock](https://github.com/xrip/cmp50hx-unlock)**（xrip）：
  Turing 谱系实机闭环（50HX/TU102，FP32 31.7×）；40HX/50HX 共用 TU102
  booter 归档的判明直接支撑本仓库 Turing 资产管线；FEAT 块
  `0x409650/64/6c` 定址出处（unlock50x_v1.c 头注 + 294-296 行）。
- **[turing-unlock](https://github.com/nlzy/turing-unlock)**（nlzy）：
  Zagitta kmod 的 Turing 官方改编（Dell 2080 Ti 22GB `1e07`，FP16 约
  1.6×）；TU102 gadget/栈几何表（WprMeta@0x400、sigDMA@0x600、canary
  0x6140、reg_write_indirect@0x10aa）与 gsp_tu10x GSP IDA 库的来源；
  `TU_FEAT_READOUT 0x409668` 引用源（ssdiag 与 loader 同源）。
- **[travis-mcdevitt/nv-uefi-unlocker](https://github.com/travis-mcdevitt/nv-uefi-unlocker)**
  （travis-mcdevitt）：本项目下游 fork，其 Turing 成功链记录进入 0x91
  取证对照（与 PR#49、nlzy 链三路互证）。
- **[tu102-22g-uefi-unlock](https://github.com/chenxluo/tu102-22g-uefi-unlock)**
  （chenxluo）：TU102 22G 改装卡 UEFI 方案（`1e04`），22G 支持线
  （issue #17）的同类对照。

## 3. 同类 Windows/UEFI 先行工作

- **[rtx30-unlock](https://github.com/craftingmod/rtx30-unlock)**
  （craftingmod）：Zagitta kmod 的 Windows/UEFI 移植（GA10x 同类先行）。
- **[rtx30-unlock-windows-toolkit](https://github.com/Siselea/rtx30-unlock-windows-toolkit)**
  （Siselea）：上者的单卡单显伴侣工具。
- **[rtx3080-unlock-public](https://github.com/rough2515-hub/rtx3080-unlock-public)**
  （rough2515-hub）：RTX 3080 10GB UEFI 实验先行（韩文）。
- **[KohakUnlock](https://github.com/KohakuBlueleaf/KohakUnlock)**
  （KohakuBlueleaf）：Turing 谱系（boot-hook 形态）与 4090 Windows/UEFI
  的源码优先研究，offline tests 与 ROM 分析方法可借鉴。
- **[Turing-unlock](https://github.com/haohervchb/Turing-unlock)**
  （haohervchb）：2080 Ti（TU102）易失解锁，2026-10-22 单跑成功记录
  （结果 JSONL + 验证文档在库）。
- **[nv_unlock_win](https://github.com/mirabarukaso/nv_unlock_win)**
  （mirabarukaso）：Titan RTX Windows 内核例试验（同目标的另一条路）。
- **[gpuunlock](https://github.com/miounet11/gpuunlock)**（miounet11）：
  4090/4070 Ti SUPER 合并发行版。

## 4. BAR1 / 寄存器 / 熔丝侧线

- **[ReBarUEFI](https://github.com/xCuri0/ReBarUEFI)**（xCuri0，MIT）：
  30 系+ ReBAR 的平台侧机制权威（`EFI_PCI_HOST_BRIDGE_RESOURCE_ALLOCATION_PROTOCOL.PreprocessController`
  枚举期 hook + PCIe ReBAR 扩展能力读写）；UEFIPatch 平台大 BAR 限制
  补丁目录（<4G/<16G/<64G 上限、MMIO 空间扩充、64 位 BAR 降级防护）。
  考据见 [reading-notes](reading-notes.md)。
- **[NvStrapsReBar](https://github.com/terminatorul/NvStrapsReBar)**
  （terminatorul）：上者的 Turing 专用 fork——推翻本项目此前"Turing BAR1
  不可扩"的判断；PSTRAPS（PEXTDEV 0x101000）位义解码是 M1 采样的直接依据。
- **[gpu-unlock-bar1](https://github.com/84634E1A607A/gpu-unlock-4090-48g-64g-bar1)**
  （84634E1A607A）：AD10x 大 BAR1 参考线（CYA/CFG 经 SEC2，未并入主线）。
- **[ga100-fuse-re](https://github.com/ahnguyen17/ga100-fuse-re)**（ahnguyen17）·
  **[gpcprobe](https://github.com/matstatman/gpcprobe)**（matstatman）·
  **[170hx-rir-tool](https://github.com/lixiangwuxian/170hx-rir-tool)**
  （lixiangwuxian）：GA100 熔丝/RIR 架构（熔丝语义参考线）。
- **[p104-gen2-unlock](https://github.com/skbskhud/p104-gen2-unlock)**
  （skbskhud）：P104 PCIe Gen2 链路工具（PCI 配置空间操作参考，与算力
  解锁正交）。

## 5. 逆向工具链

- **[envytools](https://github.com/envytools/envytools)**：envydis（fuc5
  Falcon 反汇编，量产 booter 安全段为密文、需密钥在场才可解）+ nvbios、
  PSTRAPS 文档（hw/io/pstraps.html）。
- **[faucon](https://github.com/vbe0201/faucon)**（vbe0201）：fuc5 反汇编
  备选（实测解不了量产 booter）。

## 6. 本仓库的报告者与协作者

机制发现与修复：

- **[8bit2qubit](https://github.com/8bit2qubit)**（PR #5）：booter 0x89
  真根因（上游桥 Bus Master 未使能）+ 第四验证卡（AD106 `2803` 桌面），
  台式机全倒之谜的闭环；issue #98 的记录勘误——核对全部 release tag 源码
  形态并在 PR #5 机上双构建重跑，钉死守卫作用域缺陷引入于 `4b7e955`
  （非初版实现），PR #5/#33 机分类亦据此更正。
- **[fujiaghost789-glitch](https://github.com/fujiaghost789-glitch)**
  （PR #87）：`BridgeMaster` guard 生命周期缺陷（`4b7e955` 引入、
  `ae7b128` 修复）的诊断与实机前后对照（B760/VT-d-off 路径），首个
  8 GB `2803` 数据点。
- **[Catnip202X](https://github.com/Catnip202X)**：ROM 解析器越界修复
  （PR #44）、工具间寄存器表漂移（issue #42）、免 U 盘分区提案与裁决
  记录（issue #73）。

硬件实测与报告（详表见[社区硬件记录](community-reports.md)与
[兼容性矩阵](compatibility.md#reports)，按 issue 号引用；下表列出可具名的）：

| 报告者 | 贡献 |
|---|---|
| [hunan-NRT](https://github.com/hunan-NRT) | #11（3060 Ti 跨厂 VBIOS） |
| [vadash](https://github.com/vadash) | #12（2060 Super） |
| 各 issue 报告者 | #1/#3（台式 0x89 系列）、#6、#8（2709 B760）、#9（RTD3）、#14/#16/#17（0x91/staging 线）、#35/#61（Code 12 定律）、#52（X99 GA103 首解锁）、#54（Thunderbolt）、#63/#69/#70/#71（GA106 矩阵与钉频消融） |

## 7. 基础设施

- Rust 生态：`uefi-rs`、`miniz_oxide`、`crc32fast`、`sha2`。
- [EDK2](https://github.com/tianocore/edk2)：所有 OEM UEFI 的共同基座
  （也是 ReBarUEFI/NvStrapsReBar 载具的实现基座）。
- [UEFITool/UEFIPatch](https://github.com/LongSoft/UEFITool)（LongSoft）：
  固件模块手术的参考工具。

## 许可与边界

本仓库许可：**GPL-2.0-only**（与上游一致，见 `LICENSE`）。NVIDIA 固件与
VBIOS 版权归 NVIDIA，按其原许可条款从钉版公开包本地再生，不在本仓库分发。
以上各参考仓库的许可以其自身声明为准（正文已逐条标注已知者）。
