# 交棒前的低位内存：`0xc000009A` 与自有驻留（issue [#146](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146)）

[中文](handoff-lowmem.md) | [English](handoff-lowmem.en.md) · [FAQ](faq.md#handoff) · [README](../README.md)

> 2026-11-04。项目首例 winload 阶段 `0xc000009a`（`STATUS_INSUFFICIENT_RESOURCES`），Gigabyte B550 + RTX 3060 12GB（`10de:2504`）、32 GB、Win11 26200、俄文系统。事务本体全绿，死在事务之后：**我们自己驻留在低 4 GiB 的 84 MB 资产缓存，把系统唯一能用的那一段大块连续内存劈成了两半**。本文记录判据、三跑对照、规范/实现层结论，以及为什么把"交棒前把资产还给固件分配器"做成默认。

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
