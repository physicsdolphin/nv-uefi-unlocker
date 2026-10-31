# booter status 0x89 根因：GPU 上游 PCI 桥的 Bus Master 未使能

2026-10-27 定稿。状态：**根因闭环，修复已并入主线（PR #5，已合并）**。
本文覆盖：现象、误诊史、证据链、Bus Master 机理、为何台式固件默认关、
修复方式、以及"显存 staging 绕过"备选线为何走不通。

问题发现与修复贡献：[8bit2qubit](https://github.com/8bit2qubit)（PR #5）。

## 1. 结论

booter 阶段的 meta DMA 是全链**第一笔设备发起的主机内存读**。台式机固件在
boot services 阶段没有给 GPU 上游 PCI 桥使能 Bus Master（Command Register
bit 2），该读在桥上被终结，booter 读回全 1，`wpr_desc_validate` 的最早检查
失败，booter 以 `MAILBOX0=0x89` 自报退出。与芯片组厂商、IOMMU、TSME、
iGPU/无头、显存大小全部无关——只与"路径上每座 PCI 桥的 bit 2 是否为 1"有关。

修复：loader 在 booter 点火前自动为上游路径上所有未使能的桥置位并读回确认，
事务结束原样还原（`loader/src/pci.rs` 的 `BridgeMaster`，PR #5）。

## 2. 现象

```
POST-BOOT: SEC2 MAILBOX0=0x00000089 WPR2_HI=0x…      ← booter 自报 0x89
POST-BOOT: FEAT_PLM=0xffffff8f                        ← 四写未发生
TX: transaction failed: EIO - NOT retrying this session
```

之前各阶段全绿：scrubber（BSI handoff 1→3）、FWSEC（WPR2 建立并验证）、
radix/WprMeta 发布、booter 装载与点火（CPUCTL start-probe 正常）。
`0x89`–`0x90` = `wpr_desc_validate`（IMEM 0x154a，cmp170hx 逆向）错误区间：
booter DMA 读回的 256 字节 WprMeta 与写入内容不符。

## 3. 证据链（AD106 4060 Ti 桌面，单机单变量，PR #5 附带）

平台：H310 + i5 + RTX 4060 Ti 16GB（`10de:2803`）。上游 Root Port
`8086:1901`，pri/sec/sub = 0/1/1。

1. **失败态**：Root Port command=`0x0003`（IO+MEM，**无 bit2**）；同机 PCH
   侧其它桥为 `0x0007`。回放 → `MAILBOX0=0x89`；GPU 的 PCI status 由
   `0x0010` 变 `0x2010`（**bit13 Received Master Abort**）——设备 DMA 读被
   根端口终结的一线证据。
2. **控制实验**：故意把 WprMeta 的 magic 改坏，状态码**仍是 0x89**——内容
   无关，死在传输层。这解释了为什么四台不同厂商/芯片的台式机给出逐位相同的
   错误码。
3. **修复态**：`BRIDGE-BME: command 0x0003 -> 0x0007` 后，回放
   `MAILBOX0=0`；解锁 `MAILBOX0=0x67b00001` + 四写读回 + FLR 后 AON 存活。
4. **性能**：GEMM 8192³，FP16 46.78 → **75.28** TFLOPS，BF16 47.24 →
   **76.71**（`docs/evidence/ad106-4060ti-desktop/`）。

受影响机器（均为台式塔机，均与本文机理一致）：H310+4060Ti 2803（本文）、
B850+4050LP 28e1（issue #1，BIOS 改动间接修复）、B850+4070Ti 2782（9900X）、
B760+4070S 2783（issue #3）。对照可用机全部为笔记本（固件初始化路径为桥
使能了 Bus Master，或以其它方式完成了设备 DMA）。

## 4. Bus Master 使能位机理

### 4.1 位定义与两个方向

PCI Command Register（offset 0x04，16-bit）低三位：

| bit | 名称 | 门控方向 |
|---|---|---|
| 0 | I/O Space Enable | 设备是否响应 I/O 访问 |
| 1 | Memory Space Enable (MSE) | 设备是否响应**对 BAR 的**内存访问 |
| 2 | Bus Master Enable (BME) | 设备是否**主动发起**总线事务 |

MSE 与 BME 是两个独立方向：MSE 管"进来"（host 访问设备的 MMIO/BAR），
BME 管"出去"（设备作为 master 发起对系统内存的读/写）。本案的讽刺之处：
我们的 claim 把 **GPU 自身**的 command 写到 `0x0007`（MSE+BME+IO 都有），
MMIO 全通——但真正缺的 BME 在**上游桥**上。

### 4.2 桥的 BME 语义：必要但每级都要

PCI-to-PCI 桥的 BME 门控"secondary 侧收到的主控事务向 primary 侧转发"。
GPU 自身 BME=1 只保证请求离开 GPU；路径上**每座桥**的 BME 都必须是 1，请求
才能到达内存控制器。任何一级缺位：

- **Memory Read**（需要 completion）：桥终结请求，返回 Unsupported Request
  completion → 设备侧 `Received Master Abort`（status bit13）置位 → DMA
  数据为全 1 → `wpr_desc_validate` magic 校验挂 → **0x89**。
- **Memory Write**（posted，无 completion）：静默丢弃，无任何状态位。

本案 booter 的第一笔主机 DMA 是**读**（meta → SEC2 DMEM 0x600），所以表现
为读回垃圾而非静默。MMIO（BAR 寄存器）不经这条转发路径——这就是为什么
scrubber/FWSEC 阶段全部正常、唯独 DMA 死。

### 4.3 固件为什么默认关

UEFI 规范允许固件按启动期需求决定各设备/桥的 BME。台式固件的通常策略：
只为"本 boot 内固件会对其做 DMA"的设备启用（NVMe/SATA 启动盘、USB 等）；
GPU 的 GOP 显示由 CPU 侧 MMIO 写帧完成，**不需要设备 DMA**，所以上游桥保持
`0x0003`。笔记本固件的初始化路径不同（POST 期间有设备 DMA 需求），桥上是
开的——这正好产生"台式全倒、笔记本全过"的表面规律，但本质是固件策略差异，
不是厂商差异（H310/B760/B850 三代台式全中，R9000P AMD 笔记本可用）。

### 4.4 设计意图：安全默认态，不是针对解锁

BME 的设计目的是权限控制：(a) 防止未完成初始化的设备误发 DMA 破坏内存
（BAR 未分配、映射未建立时的安全默认态）；(b) 缩小 DMA 攻击面（未授权设备
不能主动发起事务）。它与 IOMMU 分层正交：**BME=设备级发起权**，**IOMMU=
地址级授权**——即使 BME=1，IOMMU 仍可限制访问范围；IOMMU 配好，BME=0 照样
一道闸。二者都不是"为了阻止固件解锁"设计的，但叠加效应恰好压在我们唯一的
主机 DMA 上。社区 Linux 链要求 `iommu=off` 的本质相同：绕开一个与本用途
无关但作用域重叠的层。真正针对解锁的机制（NVIDIA PLM/签名/熔丝）在本轮
全部被证明执法有效——PLM 执法由 force-continue 选择器探针单独实证。

## 5. 修复（PR #5，已合并）

`loader/src/pci.rs`：

- `log_dma_path(tag)`：只读，打印 GPU 与全部 PCI 桥的 command/status/总线号
  （booter 点火前后各一次，取证用）。
- `BridgeMaster::enable`：同 segment、secondary..subordinate 覆盖 GPU 总线的
  每座 PCI-to-PCI 桥，bit2 为 0 则置位并**读回确认**；只记录实际置位的桥。
- `Drop`：事务结束（含早退）把置过的位**原样清回**——固件的 BME 状态不留给
  Windows（pci.sys 启动设备时会自行置位，实测无影响）。

`loader/src/tx.rs`：`booter_fire` 之前建立守卫，活到 `run_transaction` 返回。
不改动 GPU 寄存器操作顺序、资产、booter 装载方式与解锁判定。

## 6. 历史勘误：两条被实验否定的路线

1. **固件 IOMMU（AMD-Vi）/TSME 翻译论**：曾把 0x89 归因于平台 DMA 重映射/
   内存加密。证伪点：9900X 关闭 SVM/IOMMU 后仍 0x89；TSME 的加解密在内存
   控制器、对包括 DMA 在内的所有访问透明（AMD 官方文档），不会产生密文误读。
2. **显存 staging 绕过论**：把 meta/签名/radix/镜像全部经 BAR1 搬进显存、
   把 SEC2 FBIF 的 TRANSCFG TARGET 切到 LOCAL_FB（`0x115→0x114`，读回确认），
   期望 booter 的读完全不跨 PCIe。实测（好平台 28e0 笔记本）：staging 写入
   确认（BAR1 读回 + GOP 断联后屏幕扫出 raw 像素），booter 仍 0x89，且
   halt 后审计 SEC2 FBIF TRANSCFG = **0x00000000（LOCAL_FB + VIRTUAL）**——
   booter 在自己的 prologue 里把 DMA 通道切换为虚地址寻址，恒等映射指向
   主机 sysmem。即：**meta 邮箱里的地址永远落在主机 sysmem，显存里的副本
   原理上不可达**。FBIF TARGET 位管不到 booter 的 DMA。该线退役，结论与
   cmp170hx 的 va_to_pa_walk（booter 三级软件页走）互证。

## 7. 遗留

- issue #4（banner 超时语义与 WPR2 冷态守卫）待单独核实。
- 28e1 issue #1 中"关闭 BIOS 选项后修复"的确切机理未定（选项变动可能触发
  固件重枚举设了 BME），有该机复测数据可补。
