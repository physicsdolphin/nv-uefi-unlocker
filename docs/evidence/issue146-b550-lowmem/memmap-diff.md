# #146 三跑内存图对照（原文摘录）

`CHAIN-MEMMAP` 在本文件里只保留 below-4GiB 的主干段（`0x10000000` 之后到 `0xb2f82000`），完整 64 行与全量描述符见同目录三个 `loader_*.log`。分析见 [docs/handoff-lowmem.md](../../handoff-lowmem.md)。

## R1 — `\tx-dry-run.txt`（跳过事务，保留预载；**通过**）

```text
CHAIN-MEMMAP: 0x10000000-0x10327000 (3 MiB) type=LOADER_CODE
CHAIN-MEMMAP: 0x10327000-0xb2f82000 (2604 MiB) type=CONVENTIONAL
...
CHAIN-MEMMAP: 2871 MiB conventional / 91 MiB resident (LOADER+BS) below 4GiB (64 descriptors shown)
```

## R4 — 真事务，无标记（**0xc000009a**）

```text
CHAIN-MEMMAP: 0x10000000-0x10327000 (3 MiB) type=LOADER_CODE
CHAIN-MEMMAP: 0x10327000-0xa8eda000 (2443 MiB) type=CONVENTIONAL
CHAIN-MEMMAP: 0xa8eda000-0xadf2e000 (80 MiB) type=LOADER_DATA
CHAIN-MEMMAP: 0xadf2e000-0xb2f82000 (80 MiB) type=CONVENTIONAL
...
CHAIN-MEMMAP: 2790 MiB conventional / 170 MiB resident (LOADER+BS) below 4GiB (64 descriptors shown)
```

`0xa8eda000-0xadf2e000` = 84,388,324 B 资产缓存池（日志 `TX-ASSET-CACHE` 行：`cleared 9 entries, 84388324 bytes`）。

## R3 — 真事务 + `\tx-free-assets.txt`（**通过**）

```text
CHAIN-MEMMAP: 0x10000000-0x10327000 (3 MiB) type=LOADER_CODE
CHAIN-MEMMAP: 0x10327000-0xb2f82000 (2604 MiB) type=CONVENTIONAL
...
CHAIN-MEMMAP: 2871 MiB conventional / 90 MiB resident (LOADER+BS) below 4GiB (64 descriptors shown)
```

## 差值

| 行 | R1 | R4 | R3 |
|---|---|---|---|
| 最大 conventional 块 | 2604 MiB | **2443 MiB** | 2604 MiB |
| conventional 总量 | 2871 | 2790 | 2871 |
| resident (LOADER+BS) | 91 | 170 | 90 |
| 结局 | 干净启动 | 0xc000009a | 干净启动 |

R1 与 R4 的唯一差别是那块 80 MiB 的洞；R3 把洞填回去，块与总量同时回到 R1 的数值。resident 总量三跑相差 80 MiB，与洞等大——即变量是**位置**（劈开唯一大块）而不是"多占了 80 MiB"。
