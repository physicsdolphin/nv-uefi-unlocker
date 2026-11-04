# Low memory at handoff: `0xc000009A` and our own residency (issue [#146](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146))

[中文](handoff-lowmem.md) | [English](handoff-lowmem.en.md) · [FAQ](faq.en.md#handoff) · [README](../README.en.md)

> 2026-11-04. First winload-stage `0xc000009a` (`STATUS_INSUFFICIENT_RESOURCES`) in this project: Gigabyte B550 + RTX 3060 12GB (`10de:2504`), 32 GB, Win11 26200, Russian system language. The transaction itself was green; the boot died after it. Cause: **our own 84 MB asset cache resident below 4 GiB split the one large conventional run the system had left to allocate from.** This page records the discriminators, the three-run A/B, what the specification and the EDK2 implementation actually say, and why handing the assets back before the chain is now the default.

## 0. Bottom line

- **Not a leak - fragmentation.** ExitBootServices converts nothing, so pages we still hold cross into the Windows memory map as their own descriptors. winload takes its boot reservation out of a **single** descriptor and does not stitch, so "2.7 GiB free in total" does not save "largest usable run halved to 2.4 GiB".
- **The specification has no "keep low memory clean" rule** - the opposite (§7.2 NOTE: after EBS the image implicitly owns all unused memory). Cleanliness is a *de facto* duty toward the consumer, not a spec duty.
- Fix = free our own asset pool before the chain; **default since v2.1.11**, escape hatch `\tx-keep-assets.txt`.
- Cost is zero: nothing reads an asset after the transaction, the free is idempotent, and it is verified both ways (user's real-machine R3 and a QEMU negative control).

## 1. Symptom and discriminators

| Observation | Result |
|---|---|
| Transaction proper | Green: PLM flip ~149.7 ms, SS0/SS1 survive FLR, HANDOFF-OUT all three set, FINAL releases all DMA |
| Abort control (ESC: zero GPU writes, no detach, no reconnect) | **Passes**, boots cleanly → chain and BCD exonerated |
| `\no-reconnect.txt` / `\tx-reconnect-nonrecursive.txt` | Both blind-crash (no console) → reconnect exonerated in both directions |
| `\tx-dry-run.txt` (skip the transaction, keep the preload residency) | **Passes** → only "residency/side effects the transaction adds" is left |
| Real transaction + `\tx-free-assets.txt` | **Passes**, same machine, same stick, same BCD |

The memory maps of the three runs (`CHAIN-MEMMAP`, captured immediately before StartImage) pin the variable to one number:

```text
R1 dry run      : 0x10327000-0xb2f82000 (2604 MiB) CONVENTIONAL       -> clean boot
R4 real tx      : 0x10327000-0xa8eda000 (2443 MiB) CONVENTIONAL
                  0xa8eda000-0xadf2e000   (80 MiB) LOADER_DATA  <- the 84,388,324 B asset cache
                  0xadf2e000-0xb2f82000   (80 MiB) CONVENTIONAL       -> 0xc000009a
R3 real tx+free : 0x10327000-0xb2f82000 (2604 MiB) CONVENTIONAL       -> clean boot
```

Total conventional below 4 GiB: 2871 / 2790 / 2871 MiB (the difference is exactly the hole); resident: 91 / 170 / 90 MiB. **Between R1 and R4 the hole is the only difference**, and filling it back in (R3) restores the boot - a cause at the descriptor level, with no GPU state in it.

## 2. What the specification says

The NOTE in UEFI 2.10 §7.2 (Memory Allocation Services) is direct:

> An image that calls ExitBootServices() (i.e., a UEFI OS Loader) first calls EFI_BOOT_SERVICES.GetMemoryMap() to obtain the current memory map. Following the ExitBootServices() call, the image implicitly owns all unused memory in the map. This includes memory types EfiLoaderCode, EfiLoaderData, EfiBootServicesCode, EfiBootServicesData, and EfiConventionalMemory. A UEFI OS Loader and OS must preserve the memory marked as EfiRuntimeServicesCode and EfiRuntimeServicesData.

So leaving `EfiLoaderData` behind **is legal**; the OS does own it. Nothing requires an application to empty or merge low memory before handoff. §7.4 only requires the `ExitBootServices` key to match the latest map (else `EFI_INVALID_PARAMETER`) and only validates runtime-region alignment.

**The point is that "owns" is not "can use".** Look at the EDK2 implementation:

- `MdeModulePkg/Core/Dxe/DxeMain/DxeMain.c::CoreExitBootServices()` only calls `CoreTerminateMemoryMap(MapKey)`, and that function (`Mem/Page.c`) **only checks the map key and runtime alignment** - it converts nothing. Its own comment says so:
  > The map key they gave us matches what we expect. Fall through and return success. In an ideal world we would clear out all of EfiBootServicesCode and EfiBootServicesData. However this function is not the last one called by ExitBootServices(), so we have to preserve the memory contents.
- Our 80 MiB of pool pages therefore cross over as a `LOADER_DATA` descriptor, as-is.
- Conversely, when *we* `FreePool`, `CoreFreePoolPages -> CoreConvertPages` runs `CoreAddRange`, which merges only with an **adjacent descriptor of the same type and attributes** - that is how R3 gets the single 2604 MiB descriptor back.
- On the allocation side, `CoreConvertPagesEx` requires an allocation to lie inside **one** descriptor ("the entire range is covered by a single entry").

winload is not open source, but it behaves the same way: it has to find a large-enough low-4 GiB run inside one descriptor for its boot reservation. Split it and the reservation fails with `STATUS_INSUFFICIENT_RESOURCES`. **What the spec permits, the consumer does not necessarily survive** - that is the whole of this entry.

## 3. Fix and default

`tx::handoff_free_assets()`, called by `chainload::chain_load_bootmgfw` **before** `CHAIN-MEMMAP` (the order is the read-back):

- Covers every path that reaches the chain: normal transaction, abort control, `\tx-dry-run.txt`. The free is idempotent, so running it more than once is harmless.
- Touches only our own read-through cache; PciIo DMA/common buffers are already released at FINAL; the preloaded image (`LOADER_CODE`, pinned at 0x10000000, below the big run's lower edge) must stay.
- `\tx-keep-assets.txt` is the escape hatch (A/B and fallback); `\tx-free-assets.txt` is still accepted and is now a no-op.
- The `CHAIN-MEMMAP` summary now carries the **largest block** and a full descriptor count - the next occurrence of this shape can be read off one line.

**Why default:**

1. No consumer left by construction - nothing returns after `StartImage`; even on the fallback path the volume scan reads no assets.
2. The cache exists only to speed up the transaction (armed prep window warm-up); it is dead weight the moment the transaction ends.
3. `read_boot_file` hands callers owned clones, so clearing the cache cannot dangle.
4. Both directions are measured: real machine passes on R3, R1/R4 are the controls, and the QEMU negative control plus the `\tx-keep-assets.txt` path are green (`docs/evidence/issue146-b550-lowmem/`).

**When we would revert:** only if a future feature needed to read assets after the chain - `\tx-keep-assets.txt` is the switch then, and that machine still wants the largest-block number.

## 4. The Russian-language / font hypothesis

The reporter's machine is the only Russian Windows here (`Корпорация Майкрософт`, `D:\Временные файлы\…`). A font hypothesis explains "why this machine", but not where the edge is:

- The pass/fail margin here is **161 MiB** (2604 vs 2443), while bootmgr fonts are measured in MiB (`\EFI\Microsoft\Boot\Fonts\*`; Cyrillic rides the `wgl4_boot.ttf` family, CJK is where the genuinely large fonts are).
- Fonts load **after** our handoff, into the same big run, so they only eat into the margin; they do not change the premise that a hole splits the run.
- Verdict: **language/font is a modifier (it decides how close to the edge a machine sits); the split is the cause.** Settling it needs a CJK counter-machine (bigger fonts, should crash earlier if the edge theory holds) - open, does not affect the fix.

Cheap corroboration if wanted (low priority): the `locale` in `bcdedit /enum {current}`, the byte size of `dir \EFI\Microsoft\Boot\Fonts`, and one rerun with the language switched to English.

## 5. Scope and open points

- One machine, one stick, one session; `\tx-free-assets.txt` was not re-verified on a second machine (after defaulting, every machine exercises the path, which is continuous verification).
- winload's exact reservation size and selection algorithm are not public; "single descriptor, no stitching" is an observed summary.
- Whether descriptor **count** also contributes (not just the largest block) is not separated - the summary line now records both, so the next case decides it.
- A 32 GB machine failing with 2.4 GiB still free means that reservation is much larger than commonly assumed; 8/16 GB machines have less margin and want the default too (they get it anyway).

## 6. Evidence index

- `docs/evidence/issue146-b550-lowmem/`: raw `loader.log` for all three runs, the map diff, the QEMU negative control and escape-hatch run, the reporter's `console.txt`.
- Original commit for the transaction/residency knobs: PR [#158](https://github.com/physicsdolphin/nv-uefi-unlocker/pull/158).
- EDK2 sources: [`MdeModulePkg/Core/Dxe/Mem/Page.c`](https://github.com/tianocore/edk2/blob/master/MdeModulePkg/Core/Dxe/Mem/Page.c) (`CoreTerminateMemoryMap` / `CoreConvertPagesEx` / `CoreAddRange`), [`DxeMain.c`](https://github.com/tianocore/edk2/blob/master/MdeModulePkg/Core/Dxe/DxeMain/DxeMain.c) (`CoreExitBootServices`).
