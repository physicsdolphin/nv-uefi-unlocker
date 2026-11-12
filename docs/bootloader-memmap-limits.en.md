# Peer survey: descriptor-count limits on the EFI memory map across boot chains (issue #146 companion)

[中文](bootloader-memmap-limits.md) | [English](bootloader-memmap-limits.en.md) · Background: [the winload 1024-entry static pool](handoff-lowmem.en.md) · [#146](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146)

> 2026-11-12. Verified from actual sources (not from memory): GRUB (rhboot/grub2 `grub-core/kern/efi/mm.c`), systemd-boot (systemd/systemd main, boot path read in full), Linux EFI stub (libstub `mem.c`, `efistub.h`, `drivers/firmware/efi/memmap.c`). Verdict: **the fixed descriptor pool exists only in the Windows bootlib**.

## Bottom line

| Boot chain | How the map buffer is obtained | Fixed count cap | Headroom |
|---|---|---|---|
| **winload.efi** | `MmStaticMemoryDescriptors[1024]` static pool, no dynamic overflow | **1024 (field death line ~1000–1005)** | **0** (plus ~20–25 entries of its own working set) |
| bootmgfw.efi | same bootlib family (same 1024 pool) | 1024 | comfortable (its working set is a few entries — field data: maps of 1007–1022 entries reach winload through it alive) |
| **GRUB** | `grub_malloc(find_mmap_size())` — freshly sized every time | none | +1 page (≈85 descriptors at 48 B); BUFFER_TOO_SMALL re-runs the sizing query |
| **Linux EFI stub** | `allocate_pool(map_size + desc_size×32)` | none | `EFI_MMAP_NR_SLACK_SLOTS = 32` |
| **kernel runtime** | `size = desc_size × nr_map` (memblock/kmalloc) | none | fully dynamic |

## Per-source details

### GRUB (grub-core/kern/efi/mm.c, rhboot/grub2 master)

- `grub_efi_find_mmap_size()` (L320–336): sizing call, then `ALIGN_UP(mmap_size + GRUB_EFI_PAGE_SIZE, PAGE)` — the comment names the reason for the slack: "UEFI can alter the memory map itself on callbacks or explicit calls, **including console output**". One 4 KiB page ÷ 48 B ≈ 85 descriptors of headroom.
- `grub_efi_get_memory_map()` (L340–396): a single call; `BUFFER_TOO_SMALL` returns 0 (partial) and callers re-size via `find_mmap_size`. **No fixed arrays, no count constants anywhere.**
- Extra: `sort_memory_map()` sorts its own copy in place (winload does not sort; it ingests the firmware order as-is).
- Handoff (`finish_boot`, L676): `grub_malloc(find_mmap_size())` → read map → ExitBootServices; on `EFI_INVALID_PARAMETER` (map changed mid-flight) it retries with the **updated** measured size.

### systemd-boot (systemd/systemd main, src/boot/ read in full)

- **The modern main path never manages the memory map at all**: `linux_exec()` (boot/linux.c L175) starts the kernel as a PE image via `BS->LoadImage + BS->StartImage` (kernels ≥5.8 ship their EFI stub as PE); boot.c (157 KB) contains the string ExitBootServices exactly once, in a comment. **EBS and map ingestion are the kernel's job.**
- The deprecated x86 EFI-handover fallback (boot/linux_x86.c `linux_exec_efi_handover`) likewise does not call EBS — it hands the system table and boot_params to the kernel entry with boot services still up.
- Consequence: sd-boot is **structurally immune** to the descriptor count — its consumer is Linux.

### Linux kernel

- EFI stub (drivers/firmware/efi/libstub/mem.c `efi_get_memory_map`, L20–65): sizing call → `allocate_pool(sizeof + map_size + desc_size × EFI_MMAP_NR_SLACK_SLOTS)` → real call. `EFI_MMAP_NR_SLACK_SLOTS = 32` (efistub.h L178).
- Runtime ingestion (drivers/firmware/efi/memmap.c): `size = efi.memmap.desc_size * efi.memmap.nr_map`; both early and late phases are fully dynamic.
- **No count ceiling**; a map with thousands of entries is eaten as-is.

## Projections for this project

1. **This is a Windows-bootlib family disease, not a general EFI-ecosystem constraint** — Linux users (#148, #230, #221, #2705) never reporting `0xc000009a` is fully consistent with this.
2. **bootmgfw is looser than winload**: same 1024 pool, but a working set of a few entries; in the field, the 1007/1022-entry maps of runs 6/7 passed through bootmgfw alive and died at winload — our 950-entry WARNING line is calibrated to the tightest gate.
3. **winload's true headroom is negative**: GRUB allows 85, libstub 32, winload allows 0 and starts ~20–25 in the hole for its own working set. On boards whose firmware pushes the map past ~998, Windows is the only death.
4. The histogram (CHAIN-MEMMAP-TYPES) plus this table is the complete ammunition pack for B550 users reporting the bug to Gigabyte: same board, same path, 448↔1005 swing; Linux survives it, Windows does not.
