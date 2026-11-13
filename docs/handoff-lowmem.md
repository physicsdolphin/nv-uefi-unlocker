# 交棒前的低位内存：`0xc000009A` 与自有驻留（issue [#146](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146)）

[中文](handoff-lowmem.md) | [English](handoff-lowmem.en.md) · [FAQ](faq.md#handoff) · [README](../README.md)

> 2026-11-04。项目首例 winload 阶段 `0xc000009a`（`STATUS_INSUFFICIENT_RESOURCES`），Gigabyte B550 + RTX 3060 12GB（`10de:2504`）、32 GB、Win11 26200、俄文系统。事务本体全绿，死在事务之后：**我们自己驻留在低 4 GiB 的 84 MB 资产缓存，把系统唯一能用的那一段大块连续内存劈成了两半**。本文记录判据、三跑对照、规范/实现层结论，以及为什么把"交棒前把资产还给固件分配器"做成默认。
>
> **2026-11-12 更新**：重开后新增的三台机器（另两块 Gigabyte B550）把真正的约束钉死在**描述符条数而非尺寸**上——winload 的 1024 条静态描述符池。见 §0b；下文的尺寸分析仍成立，作为次级机制，也是 v2.1.11 默认修复的对象。

## 0b. 真正的天花板：描述符条数（2026-11-12 RE）

重开 issue 带来三台机器：事务死在 winload，但最大块尺寸与能启动的 abort 路径完全相同——同尺寸的 pass/fail 对，尺寸理论无法解释。对本机 `System32\Boot\winload.efi`（Windows 11 26200 世代，SHA-256 `7002fe01…f44f`，符号完整）的静态分析钉死机制：

- `winload.efi!BlpMmInitialize`（`0x1801d35bc`，写入点 `0x1801d3644`）：`MmGlobalMemoryDescriptorCount = 1024`。引导期**所有**内存描述符（页分配器区域、固件 map 翻译、堆边界……）全部从这一个 **1024 条静态池**分配（`MmStaticMemoryDescriptors` @ `0x1803457e0`，stride 48 字节，数组终点恰好顶着 count 变量）。`MmMdpAllocDescriptor`（`0x1801d6c48`）**没有动态扩容**：free list 空 ⇒ 只能 `Used < 1024` 取新槽，否则 NULL。
- `winload.efi!MmFwGetMemoryMap`（`0x1801d7138`）把继承 map 的**每一条**描述符过 `MmMdInitDescriptor` → `MmMdAddDescriptorToList`；NULL ⇒ 返回 `-1073741670` = **`0xC000009A`**。该函数在启动关键路径出现三次：`MmPaInitialize`（winload 入口）、`BlMmGetMemoryMap`、`MmFwSynchronizeMemoryMap`（后期对我们交棒 map 的再摄取）。
- 同函数的次级陷阱：两次 `GetMemoryMap`（探大小→真读）之间只留 **`4 × DescriptorSize`** 增长余量；期间 map 长出 >4 条，第二次调用以 `0xC0000023` 失败——即"无 Recovery 屏"的哑失败形态。

现场数据——重开线程里全部 `CHAIN-MEMMAP` 全图描述符总数（三台机器）：

| 描述符总数 | 路径 | 结果 |
|---|---|---|
| 448 / 998 | ESC abort（零 GPU 写） | **启动** |
| 1005 | ESC abort（零 GPU 写） | 失败，无 Recovery 屏（`0xC0000023` 形态） |
| 1007 / 1012 / 1013 / 1014 / 1021 / 1022 | dry-run 与事务路径 | `0xc000009a` |
| **1047**（v2.1.11，预载开） | 事务，资产释放已生效 | `0xc000009a` |
| **668**（rc3，预载关） | 事务 | **启动，解锁保持** |
| 1010（rc3，BIOS FFc） | 事务 | `0xc000009a` |
| **1004**（rc3，**BIOS FHc**） | 事务 | **启动，ssdiag `UNLOCKED`**——BIOS 轮首个修复案例 |

阈值 ∈ **(998, 1005]** ≈ 1024 减去 winload 自身 ~20–25 条并发工作集。**四个数据点零例外——十二个数据点、四块 Gigabyte B550**。末两行是首个** resolution** 数据点（[#146 评论](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146#issuecomment-6824222242)，B550 AORUS MASTER F18i，GA102 2206，64 GB，SB ON shim/MOK）：两次 run 恰好骑在判据两侧。归因注意：379 条差跨了两个构建和两次 boot——otacoo 板已证 boot-to-boot 摇摆（448↔1005）且同款预载 A/B 只差 ~1 条，这 379 = 固件摇摆 + 预载 + SB shim 路径的混合，已邀请报告者做同构建 `\tx-preload.txt` A/B 定量。668 run 的直方图（bs_data=318、bs_code=140、conv=160 其中 139 个 ≤64 KiB 小块、0 MiB）桶和自洽——buckets+oem==counted 不变量首次现场验证通过。

**BIOS 轮闭环（2026-11-12 晚，同线程）**：otacoo 的 B550I AORUS PRO AX 经 Q-Flash 从 FFc（2024-09-02）升到 FHc（2026-08-18），rc3 从 1010 条死变 **1004 条启动且系统内 ssdiag `UNLOCKED`**（[评论](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146#issuecomment-6829307694)，日志归档 `reopen-2026-11/run12-*`）。升版改变了两个独立的东西：条数 1010→1004（−6，过线；>950 WARNING 照设计触发——它标的是"进入建议带"不是"必死"）和布局，最大块 2028→2634 MiB（新固件重新整理了低位区——第二个签名，对尺寸顶板的板子才是关键）。同 BIOS 的 abort→tx 差 = **我方事务足迹在这块板实测 +17 条**，与 ivandrozdov 板的 ~+15 独立互证——这是 loader 侧合并优化必须打败的数字。保留的诚实注意：事务 boot 目前一次（升版前同板零写摇摆 448↔1005，重复性开放）；此机是 Windows 10 LTSC，winload 与被分析的 Win11 26200 镜像不同版本——1004 过 vs run4 哑失败 1005 死，两种解释都成立（那边边界略高，或 run4 死于增长余量陷阱而非池耗尽）；不跨 winload 版本拟合单条边界。启动证据方法值得推广：过的一次 `loader.log` 末写 → 内核事件 12 仅 **2.5 s**，失败轮全是 34–93 s 恢复间隔。

## 0c. 隐藏变量：进入路径（2026-11-12 深夜）

DCBURG3R 的七轮 A/B（[评论](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146#issuecomment-6844931232)，表归档 `reopen-2026-11/ab-data-table.md`）找到了整个线程一直缺的那个变量：**用户怎么进入引导**。同板同构建同天，两带零重叠：

| 路径 | 条数 | 结果 |
|---|---|---|
| **F12 / BBS popup**（热+冷，预载开+关） | 1024 / 1039 / 1040 / 1048（一个 668，现已认为标注有误） | 5 轮 4 死 `0xc000009a` |
| **DEL → BIOS → Save/Exit → Boot Override** | 549 / 665 /（668，R0） | 启动，解锁保持 |

物理旁证：F12 路径 console 以**原生分辨率**起来，DEL 路径低分辨率——BBS popup 在原生分辨率重建 console 栈（scanout 缓冲、字体/字形位图、绘制表面、console 重绑定），直方图签名精确对上：F12 轮 **bs_data ≈ 494–503、tiny_conv ≈ 300**；BIOS-override 轮 **bs_data ≈ 257–316、tiny_conv ≈ 98–145**。同表配对判读：**预载 ±8–15 条（噪声级——opt-in 翻转有帮助，但预载从来不是百条级项）**；冷/热次阶（~9–16 条）。推论：

- 历史上的"boot-to-boot 摇摆"（otacoo 的 448↔1005"同路径"）改写为**未记录的路径混用**，除非另有证明。今后任何日志必须记录进入路径。
- 成立的 workaround：DEL → BIOS → Save/Exit → Boot Override；永久形态：**把解锁布局装进内置 FAT32 分区、BIOS 固定 BootOrder 引导**——固件默认 BDS 路径永不运行菜单应用层（同 [D#229](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/229) ESP 型分区与 [#218](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/218) NVMe 分区流同形；仅手动步骤，按裁决不做 installer）。
- 给技嘉报告一句话：*同板同构建同天，F12/BBS-popup 引导路径比 Setup→Save/Exit→Boot-Override 路径多留 ~370–475 条内存图描述符，推过 winload 的 1024 条池。*

- **碎片按条计价，与大小无关**——4 KiB 的洞与 80 MiB 的洞花同一个槽。v2.1.11 的资产释放仍然正确（同时消掉尺寸劈裂与 ~2 个槽），但救不了固件自己就把 map 填到 ~998–1005 条的板子。
- **该错误类别里 GPU 态无罪**：失败的 abort（1005）与通过的 abort（448）只差 map；PLM/FLR/WPR2/GOP 不可能参与一个在任何 GPU 驱动存在之前就产生的失败。
- **map 是固件的产物，应用层动不了**：分配只能从 `EfiConventionalMemory` 切且不许跨描述符；EDK2 只在释放时合并同型相邻——4 KiB 型交替碎片不可修复。固件自己越线的板子只能等固件厂商修，或断电重试躲过去（同板的 compact-map boot 能过）。
- 我们自身的加项（heap arena 碎片 ~+15 条、GOP 重连重分配、预载）恰恰在这种板子上致命：预载自 2026-11-09 起 opt-in（`\tx-preload.txt`）；chainload 摘要在 950 条以上打硬 WARNING，让日志自解释。
- 实测固件病型：一块板稳定高位 998（离死线 1 条），另一块在同样的零写 boot 间摇摆 448↔1005。

完整走读 + BIOS 版本对照协议见 [issue #146](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146#issuecomment-6815041042) 的 2026-11-12 评论。下文的尺寸分析作为次级机制仍成立（winload 页分配器仍需单个描述符内装得下的大块）。


## 0. 结论先行

- **不是泄漏，是碎片**：ExitBootServices 不会把我们持有的页转成 conventional，它们以各自的描述符原样进入 Windows 的内存图；winload 的启动块只从**单个描述符**里分配、不跨描述符拼接，所以"总空闲 2.7 GiB"救不了"最大可用块被砍到 2.4 GiB"。
- **规范没有"低位内存必须干净"的要求**——恰恰相反（§7.2 NOTE：EBS 之后镜像隐式拥有全部未用内存）。干净是本项目对 consumer 的**事实性义务**，不是规范义务。
- 修复 = 交棒前释放自有资产池；自 v2.1.11 起**默认**，退出阀 `\tx-keep-assets.txt`。
- 代价为零：过了事务就再没人读资产，释放幂等，且已在 QEMU 负控 + 用户真机 R3 双向验证。

## 1. 病理与判据

| 观察 | 结果 |
|---|---|
| 事务本体 | 全绿：PLM 翻转 ~149.7 ms、SS0/SS1 过 FLR、HANDOFF-OUT 三值对、FINAL 释放全部 DMA |
| abort 负控（ESC，零 GPU 写/零 detach/零重连） | **通过**，干净进系统 → 链载与 BCD 无罪 |
| `\no-reconnect.txt` / `\tx-reconnect-nonrecursive.txt` | 都盲崩（无 console）→ 重连双向排除 |
| `\tx-dry-run.txt`（跳过事务；该路径不读资产，缓存不填写） | **通过** → 预载本身无害，且给出健康态的单块 2604 MiB |
| 真事务 + `\tx-free-assets.txt` | **通过**，同机同盘同 BCD |

三跑的内存图（`CHAIN-MEMMAP`，StartImage 前采）把变量钉死在一个数字上：

```text
R1 dry-run ：0x10327000-0xb2f82000 (2604 MiB) CONVENTIONAL           -> 干净启动
R4 真事务 ：0x10327000-0xa8eda000 (2443 MiB) CONVENTIONAL
             0xa8eda000-0xadf2e000   (80 MiB) LOADER_DATA   <- 84,388,324 B 资产缓存
             0xadf2e000-0xb2f82000   (80 MiB) CONVENTIONAL           -> 0xc000009a
R3 真事务+free：0x10327000-0xb2f82000 (2604 MiB) CONVENTIONAL        -> 干净启动
```

三跑的 conventional 总量分别是 2871 / 2790 / 2871 MiB（相差正好一个洞），resident 是 91 / 170 / 90 MiB。**R3 与 R4 只差"释放"这一个动作**（同事务、同盘、同 BCD），洞填回去就恢复——纯描述符层面的因果，不掺任何 GPU 状态。R1 的角色弱一些：dry-run 走不到资产读取，它的缓存根本没被填写（resident 91 MiB、无 `TX-ASSET-DIAG` 行），所以它证明的是"预载本身无害、且 2604 MiB 单块就是健康态"，不证明"GPU 副作用无罪"。

## 2. 规范怎么说

UEFI 2.10 §7.2（Memory Allocation Services）的 NOTE 是直接的：

> An image that calls ExitBootServices() (i.e., a UEFI OS Loader) first calls EFI_BOOT_SERVICES.GetMemoryMap() to obtain the current memory map. Following the ExitBootServices() call, the image implicitly owns all unused memory in the map. This includes memory types EfiLoaderCode, EfiLoaderData, EfiBootServicesCode, EfiBootServicesData, and EfiConventionalMemory. A UEFI OS Loader and OS must preserve the memory marked as EfiRuntimeServicesCode and EfiRuntimeServicesData.

也就是说：留给 OS 的 `EfiLoaderData` **是合法的**，OS 确实"拥有"它。规范没有一条要求应用在交棒前清空或合并低内存。§7.4 只规定 `ExitBootServices` 的键必须是最新内存图的键（否则 `EFI_INVALID_PARAMETER`），也只做运行时区的对齐校验。

**关键在于"拥有"≠"能用"**。看 EDK2 的实现：

- `MdeModulePkg/Core/Dxe/DxeMain/DxeMain.c::CoreExitBootServices()` 只调 `CoreTerminateMemoryMap(MapKey)`，后者（`Mem/Page.c`）**只校验 MapKey 与运行时区对齐**，一个字都不转换。源码注释写得很直白：
  > The map key they gave us matches what we expect. Fall through and return success. In an ideal world we would clear out all of EfiBootServicesCode and EfiBootServicesData. However this function is not the last one called by ExitBootServices(), so we have to preserve the memory contents.
- 于是我们的 80 MiB 池页以 `LOADER_DATA` 描述符原样交出去。
- 反过来，一旦我们**主动** `FreePool`，`CoreFreePoolPages -> CoreConvertPages` 走 `CoreAddRange`，它只与**同类型同属性且紧邻**的分支合并——R3 的 2604 MiB 单描述符就是这么回来的。
- 而分配侧，`CoreConvertPagesEx` 明确要求一次分配必须落在**单个描述符**内（"the entire range is covered by a single entry"）。

winload 不在开源范围，但行为同构：它得在一次描述符里找一块够大的低 4 GiB 连续区做启动预留，劈开就失败，报 `STATUS_INSUFFICIENT_RESOURCES`。**规范允许的，consumer 不一定接得住**——这是本条的全部内容。

## 3. 修复与默认化

`tx::handoff_free_assets()`，由 `chainload::chain_load_bootmgfw` 在 `CHAIN-MEMMAP` **之前**调用（顺序即读回证据）：

- 覆盖所有到达链载的路径：正常事务、abort 负控、`\tx-dry-run.txt`。free 幂等，多跑无害。
- 只动我们自己的读穿缓存；PciIo DMA/公共缓冲在 FINAL 已释放；预载镜像（`LOADER_CODE`，贴在 0x10000000，位于大块的下沿之外）必须留。
- `\tx-keep-assets.txt` = 退出阀（A/B 与后备）；`\tx-free-assets.txt` 仍被接受，已是空操作。
- `CHAIN-MEMMAP` 摘要新增 **largest block** 与全量描述符计数——下一次出现同类症状时，先看这个数，不用再猜。

**为什么敢默认**：

1. 语义上无消费方——`StartImage` 之后代码不再回来；即便回来，回退的逐卷扫描也不读资产。
2. 缓存只服务于事务期提速（armed 预备窗口内预读），事务结束即死重。
3. `read_boot_file` 给调用方的是 owned clone，清缓存不会悬垂。
4. 已实证两向：R3 真机通过；R1/R4 是对照；QEMU 负控与 `\tx-keep-assets.txt` 路径均绿（`docs/evidence/issue146-b550-lowmem/`）。

**什么情况下退回**：只可能出现在"链载之后还要读资产"的新功能的机器上——那时 `\tx-keep-assets.txt` 就是开关，且该机器照样要看 largest block。

## 4. 俄文系统 / 字体假说评估

报告者机器是唯一一例俄文 Windows（`Корпорация Майкрософт`、`D:\Временные файлы\…`）。字体假说能解释"为什么是这台"，但**解释不了边缘在哪里**：

- 这里的生死差是 **161 MiB**（2604 vs 2443），而 bootmgr 的字体按 MiB 计（`\EFI\Microsoft\Boot\Fonts\*`，西里尔走 `wgl4_boot.ttf` 一族；CJK 才是真正的大字体）。
- 字体在**我们交棒之后**才由 bootmgr 载入，落在同一段大块里，只会继续啃食余量，不会改变"洞把块劈开"这个前提。
- 所以定性：**字体/语言是修饰项（决定谁离边缘多近），劈开才是病因**。要坐实还需一台 CJK 机器做对照（CJK 字体更大、若边缘论成立应更早崩）——开放项，不影响修复。

顺带可收的旁证（低优先级）：`bcdedit /enum {current}` 的 `locale`、`dir \EFI\Microsoft\Boot\Fonts` 的字节数，以及同机把语言换成英文复跑。

## 5. 范围与未解

- 一机一盘一屏一次；`\tx-free-assets.txt` 未在第二台机器复验（默认化后所有机器都在跑该路径，等于持续验证）。
- winload 预留区的确切尺寸、选择算法未公开；本文用"单描述符、不拼接"概括，属行为归纳。
- 描述符**个数**本身是否也参与触发（而不只是最大块）未分离——摘要行现在同时记录个数与最大块，下一例可判。
- 32 GB 机在 2.4 GiB 空闲下仍失败，说明该预留远大于常见认知；8/16 GB 机器的余量更小，建议同样默认（本就默认）。

## 6. 证据索引

- `docs/evidence/issue146-b550-lowmem/`：三跑原始 `loader.log`、内存图对照、QEMU 负控与退出阀路径、报告者 `console.txt`。
- 事务与驻留旋钮的原始提交：PR [#158](https://github.com/physicsdolphin/nv-uefi-unlocker/pull/158)。
- EDK2 源码：[`MdeModulePkg/Core/Dxe/Mem/Page.c`](https://github.com/tianocore/edk2/blob/master/MdeModulePkg/Core/Dxe/Mem/Page.c)（`CoreTerminateMemoryMap` / `CoreConvertPagesEx` / `CoreAddRange`）、[`DxeMain.c`](https://github.com/tianocore/edk2/blob/master/MdeModulePkg/Core/Dxe/DxeMain/DxeMain.c)（`CoreExitBootServices`）。
