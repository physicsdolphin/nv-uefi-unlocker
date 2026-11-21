# #273 · RTX 4070 Laptop `2860`（AD106）— ASUS TUF Gaming A16 FA607PI，AMD 7940HX

来源：[issue #273](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/273)（2026-11-21，报告者 dzkdzsh，原始件在其 [secret gist](https://gist.github.com/dzkdzsh/e096b243bb9bdfb56612e26cd1a44368)，本目录为逐文件抓取归档）。2860 第二样本（首样本 #202 = 同卡 Intel 平台）、首个完整双态 A/B、首个「内置非 ESP FAT32 数据分区 + 一次性 F8 菜单」进入形态。

平台：Ryzen 9 7940HX / BIOS `FA607PI.311` / Win 11 25H2 26200.8655 / 驱动 610.62 / SB off / 独显直连（display_active=Enabled）/ RTD3 键 absent / VBIOS `95.06.32.00.38`。工具 v2.1.12-rc4 + maker `-Family 40` 自制载荷（#270 `-BootEfi` 显式、#271 fail-closed 手工收尾——两案当日修复 32be9c9）。

| 文件 | 内容 |
| --- | --- |
| `loader-readonly-pass1.log` | 只读负控轮（零写入），23 KB，全链跑通确认枚举/staging |
| `loader-tx-pass1.log` | TX 事务轮（60 KB）：POST-BOOT `FEAT_PLM=0xffffffff`、SELECTOR `0x88888888/0x8`、`CANARY-SET 0x0164c0de/0x0165cafe`、stock 签名还原、POST-FLR 存活、POST-GOP 门保持、FINAL 全 DMA 释放、交棒 Boot0004 |
| `ssdiag-locked.txt` | 锁定态（冷启动直进 Windows）：`VERDICT [ada]: NOT_UNLOCKED`、canary cleared、PLM `0xffffff8f`、READOUT `0x00410080` |
| `ssdiag-unlocked.txt` | 解锁态（解锁引导后）：`VERDICT [ada]: UNLOCKED (override values live)`、canary intact（与 loader CANARY-SET 逐位一致）、PLM `0xffffffff`、SS0/SS1 `0x88888888`/`0x8`、READOUT `0x00000000` |
| `20261121_*.clpeak.json` ×5 | clpeak 3.0.5 CUDA 五轮（A/150314 锁、B/163812 解、C/164605 解、D/165800 锁、E/170111 锁）：受钳四路径（fp16/bf16/tf32/fp8 的 fp32-acc wmma）1.96–2.01×，f16acc/FP32 1.0× |
| `clocks-locked.csv` | 锁定态负载 1 s 间隔 246 样本（显存 8101 / SM 2520–2610 / ≤129.8 W，排除超频） |

归档时的核验（维护者，2026-11-21）：ssdiag 双态 VERDICT/canary/PLM 与 issue 表逐项一致；loader TX 关键行与 issue 引文一致；五份 clpeak json 的 wmma/带宽列与 issue 表逐位一致（`wmma:fp16` 47.26/92.96/93.21/47.45/47.80、local float4 7.11/9.65/9.57/9.75/9.52 TB/s、global float4 181.9/235.4/232.1/235.7/222.2 GB/s）。

新观察（未解，登记 [open-questions](../../open-questions.md) #20）：解锁态空闲 PCIe 降速 Gen2 x8（锁定态空闲 Gen4 x8；解锁态施载回 Gen4 x8，LNKCAP `0x00456884`→`0x00457884` 同步恢复）——H2D/D2H 13.2–13.3 GB/s 判定吞吐无损。
