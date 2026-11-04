# issue #146 证据：B550 交棒前低位内存驻留

- 日期：2026-11-04
- 机器：Gigabyte B550 Aorus Elite V2 / Ryzen 5 5600 / RTX 3060 12GB（`10de:2504`）/ 32 GB DDR4-3600 / Win11 26200.8457（俄文）/ 单 DP 2K
- BIOS：UEFI、CSM off、Secure Boot off、Above4G on、ReBAR Auto
- 构建：v2.1.9-diag.6（`BOOTX64-146-residency.efi`，SHA-256 `d87a7587…-c5e0568`）
- 报告者：@TheHarbit（issue 正文与三轮回传均在其名下）

## 文件

| 文件 | 内容 |
|---|---|
| `loader_r1.log` | R1 `\tx-dry-run.txt`：跳过事务（该路径不读资产，缓存不填写），预载照旧，**通过** |
| `loader_r4.log` | R4 真事务、无标记：**0xc000009a**，带失败态 `CHAIN-MEMMAP` |
| `loader_r3.log` | R3 真事务 + `\tx-free-assets.txt`：**通过**，缓存 84,388,324 B 归还，2604 MiB 块复原 |
| `memmap-diff.md` | 三跑内存图主干对照与差值表 |
| `console.txt` | 报告者系统内 `ssdiag` 输出（解锁态：PLM=0xffffffff、SS0/SS1 命中、READOUT 全 0） |
| `qemu-negative-control.log` | 默认化后 QEMU 负控：`cleared 0 entries`、`largest block` 摘要、链载 NOT_FOUND 收尾、零 panic |
| `qemu-keep-assets.log` | `\tx-keep-assets.txt` 退出阀：`cache left resident across the chain` |

## 判据链

1. abort 负控通过 → 驱动链载/BCD 无罪；两个重连旋钮都盲崩 → 重连双向排除。
2. R1（跳过事务）通过，但该路径**不读资产**、缓存未填写（resident 91 MiB、无 `TX-ASSET-DIAG`）——它只证明预载无害并给出健康态基线，不隔离 GPU 副作用。
3. R4 复现，且 `CHAIN-MEMMAP` 显示唯一的 2604 MiB 大块被 80 MiB `LOADER_DATA` 劈成 2443+80。
4. R3 在 cache 被显式释放后通过，块恢复 2604 MiB、总量回到 2871 MiB——与 R1 完全同形。

## 复现要点

- 标记为盘根零字节文件；R3 用的就是 `\tx-free-assets.txt`（默认化后该标记已是空操作）。
- 每跑一份 `loader.log`；`CHAIN-MEMMAP` 是每跑的读回，比较 **largest block** 即可。
- 失败态需带 `CHAIN-MEMMAP` 才有价值（R4 提供了第一份失败态内存图）。

## 未覆盖

- 单机单次；无第二台机器、无 CJK 系统对照（字体假说见 [docs/handoff-lowmem.md](../../handoff-lowmem.md) §4）。
- QEMU 无资产路径，`cleared 0 entries` 只验证调用顺序与日志，不验证释放量。
