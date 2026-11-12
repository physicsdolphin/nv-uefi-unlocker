# 横向调研：各引导链对 EFI 内存图描述符的数量上限（issue #146 配套）

[中文](bootloader-memmap-limits.md) | [English](bootloader-memmap-limits.en.md) · 背景：[winload 1024 条静态池定谳](handoff-lowmem.md#0b-真正的天花板描述符条数2026-11-12-re) · [#146](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146)

> 2026-11-12。逐份读源码核实（非凭记忆）：GRUB（rhboot/grub2 `grub-core/kern/efi/mm.c`）、systemd-boot（systemd/systemd main 全文）、Linux 内核 EFI stub（libstub `mem.c` + `efistub.h` + `drivers/firmware/efi/memmap.c`）。结论：**整个开放生态只有 Windows bootlib 有固定描述符池**。

## 结论先行

| 引导链 | map 缓冲怎么来 | 固定条数上限 | 余量 |
|---|---|---|---|
| **winload.efi** | `MmStaticMemoryDescriptors[1024]` 静态池，无动态扩容 | **1024（实测死亡线 ~1000–1005）** | **0**（池-占用=0 余量，working set ~20–25 条） |
| bootmgfw.efi | 同一 bootlib 家族（同款 1024 池） | 1024 | 较大（自身 working set 只有几条——现场数据：1007–1022 条的 map 它都活着跑到了 winload） |
| **GRUB** | `grub_malloc(find_mmap_size())`，每次按实测现分 | 无 | +1 页（48B/条 ≈ 85 条）；BUFFER_TOO_SMALL 时重走 sized 查询 |
| **Linux 内核 EFI stub** | `allocate_pool(map_size + desc_size×32)` | 无 | `EFI_MMAP_NR_SLACK_SLOTS = 32` 条 |
| **内核运行期** | `size = desc_size × nr_map`（memblock/kmalloc） | 无 | 全动态 |

## 逐源细节

### GRUB（grub-core/kern/efi/mm.c，rhboot/grub2 master）

- `grub_efi_find_mmap_size()`（L320–336）：先用 `GetMemoryMap(0)` 探实测大小，然后 `ALIGN_UP(mmap_size + GRUB_EFI_PAGE_SIZE, PAGE)`——注释明说余量的原因："UEFI can alter the memory map itself on callbacks or explicit calls, **including console output**"。一页 4 KiB ÷ 48 B ≈ 85 条余量。
- `grub_efi_get_memory_map()`（L340–396）：单次调用；`BUFFER_TOO_SMALL` 返回 0（partial），调用方拿 `find_mmap_size` 重新现分。**没有任何固定数组、没有任何条数常量。**
- 额外动作：`sort_memory_map()` 把自己的副本就地排序（winload 不排序，逐条照抄固件顺序）。
- 交棒路径（`finish_boot`，L676）：`grub_malloc(find_mmap_size())` → 读 map → ExitBootServices；`EFI_INVALID_PARAMETER`（map 中途变了）时用**更新后的实测大小**重分重试。

### systemd-boot（systemd/systemd main，src/boot/ 全文通读）

- **现代主路径根本不管理内存图**：`linux_exec()`（boot/linux.c L175）把内核当 PE 用 `BS->LoadImage + BS->StartImage` 启动（≥5.8 内核 EFI stub 即 PE）；boot.c 全文（157 KB）只有一处 ExitBootServices 字样且是注释。**EBS 和 map 摄取全部由内核自己做。**
- 废弃的 x86 EFI-handover 兜底（boot/linux_x86.c `linux_exec_efi_handover`）：也不调 EBS——把系统表和 boot_params 递给内核入口，boot services 仍活着，内核自己收尾。
- 推论：sd-boot 对描述符条数**结构性免疫**——消费者是 Linux。

### Linux 内核

- EFI stub（drivers/firmware/efi/libstub/mem.c `efi_get_memory_map`，L20–65）：探大小 → `allocate_pool(sizeof + map_size + desc_size × EFI_MMAP_NR_SLACK_SLOTS)` → 真读。`EFI_MMAP_NR_SLACK_SLOTS = 32`（efistub.h L178）。
- 运行期摄取（drivers/firmware/efi/memmap.c）：`size = efi.memmap.desc_size * efi.memmap.nr_map`，early/late 两阶段都是全动态。
- **条数无上限**；几千条的 map 内核照吃。

## 对本项目的投影

1. **这是 Windows bootlib 的家族病，不是 EFI 生态的普遍约束**——Linux 用户（#148、#230、#221、#2705）从没报过 0xc000009a 与此完全自洽。
2. **bootmgfw 比 winload 宽松**：同款 1024 池但 working set 只有几条；现场 run6/run7 的 1007/1022 条 map bootmgfw 都活着跑完、死在 winload——所以我们的 950 条 WARNING 线标的是最紧的那道门。
3. **winload 的真实余量是负的**：GRUB 给 85 条、libstub 给 32 条、winload 给 0 条还倒欠 ~20–25 条 working set。固件侧把 map 弄到 998+ 的板子，Windows 是唯一死法。
4. 直方图（CHAIN-MEMMAP-TYPES）+ 这份对照表 = 给 B550 用户向技嘉报 bug 的完整弹药：同板同路径 448↔1005 摇摆，Linux 能活 Windows 不能。
