# 诊断构建协议：~~2.1.12-diag3 / 2.1.13-diag4~~ → 2.1.11-diag5（2026-11-05/06）

> **⚠️ 2026-11-06：diag3 与 diag4 已从 release 撤除（release 与 tag 均删）。**
> 撤回原因：#170 报告者从 U 盘 FAT 簇链还原出两份完整 v2.1.10 日志，证明事务在 5.6s 完成 detach 并继续跑过 AUTH-SCAN、SEC2 scrubber（ETimeout，容忍）、WPR2 layout，证据停在 ~10.9s 的 `TX: layout ok`——**diag3/diag4 的判读前提"挂点在 DETACH 的 DisconnectController 段"不成立**（屏幕末行只是控制台拆除后的定格帧，不是执行终点），按其判读表排查会走错方向；diag4 对"首行前窗口"的推断也被同一证据推翻（0 字节/黑屏在 Flush 不提交目录项的固件上不可作窗口判据）。
> 两版中仍然有效的机制（标记行关闭落盘、首行起逐行提交、`DIAG-CONSOLE`/`DIAG-BOOTVOL` 身份 dump、EARLY 枚举前移、拆挂前静音）**全部由 [2.1.11-diag5](https://github.com/physicsdolphin/nv-uefi-unlocker/releases/tag/v2.1.11-diag5) 继承**，并把判读重新定向到 FWSEC 点火段。**现行协议 = §5（diag5）+ §1 通用纪律（含 FAT 簇链还原法，现为主判据之一）**；§2/§3 保留作历史记录，其判读表已失效，勿按其排查。
> 两个 issue 的四条推荐评论已原地 PATCH 加撤回横幅（历史未删）。

> **2026-11-05 更新：`2.1.13-diag4` 已发布，请用 diag4。** #122 报告者自建 A/B 后，挂死窗口收窄到"日志文件已创建、首行落盘之前"一条约三条语句的窄带；diag4 把这条窄带本身变成可读记录：
>
> - `\loader.log` 的**创建本身现在立即提交**（空目录项先落盘）：出现 0 字节文件即证明已进入 `logging::init` 的创建路径，死于其后三条调用（set_logger / 首次 format / 首次写）之内；
> - `"log file"`、`"nv-uefi-unlocker"`、`"BOOT"`、`"DIAG"`、`"EARLY"`、`"PREP"`、`"CONFIRM"` 全部成为落盘标记——从第 1 行起逐行提交，挂死点落到哪一行停在哪个词；
> - 其余（A/D、两 dump、EARLY 前移）与 diag3 完全一致，事务链未动。
>
> 资产与 SHA-256（**旧 diag3 资产仍可用，但诊断分辨率低一档**）：

服务两个"怪死"平台：

- issue [#170](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/170)（MSI B85M，`DETACH` 阶段确定性挂死，Xeon E3-1231 v3 无核显）——用 **tx 构建**；
- issue [#122](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/122)（ASUS Z97-K，约 14% 概率的极早期挂死，同款无核显）——用 **只读构建**（20 连跑改判读见 §3 增补）。

两块板 + 讨论 [#121](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/121)（H61，正常）的共同点与差异（iGPU 有无、Aptio 代次、显卡）都已入档；本构建只回答一个问题：**挂死发生在"控制台被拆掉"之前还是之后**，并把枚举/控制台/宿主控制器一次钉死。

## 0. 本构建是什么 / 改了什么

**没有任何新增硬件写**。事务链、payload、FLR 顺序与 `2.1.11-diag2` 逐字节同源（构建只在日志与枚举外围加东西）：

1. **A：标记行落盘改为"写 → 关文件 → 重开"**。此前 0 字节日志的根因已查明：`FatIFileAccess` 成功路径不刷缓存，`Flush()` 在这些固件上等于空操作，只有 `Close` 才提交目录项（大小）。现在每个 `DETACH`/`POST-`/`FINAL` 等标记行后都关闭并重开 `\loader.log`（写入位置在末尾，文件内容不丢）。**挂死点现在会留在 U 盘上，一个字符不少**。
2. **D：拆挂前静音控制台镜像**。`DETACH: console mirror MUTED until reconnect` 之后所有日志只进文件，直到重连/交棒前再解除（`CONSOLE: mirror re-enabled ...`）。这一刀把两种死法分开：**过得了静音点 = 死在 `DisconnectController` 的 `Stop()` 链里；过不了 = 死在"对已被拆掉的控制台写屏"上**。
3. **两个只读身份转储**（banner 后立即打印，挂死前就已落盘）：
   - `DIAG-CONSOLE-OUT/IN`：固件给我们的是哪一路控制台——设备路径解码出的锚点设备，`(NVIDIA GPU)` 标记表示控制台挂在被拆的那张卡上，`uart=1` 表示串口；
   - `DIAG-BOOTVOL`：本 U 盘所在的宿主控制器。8 系 PCH 的 EHCI #1=`:1d.0`、EHCI #2=`:1a.0`、xHCI=`:14.0`（USB2 口的实际路由由 `xUSB2PR` 决定，光看插哪个口不算证据）。
4. **枚举前移**：chain 阶段那次递归 `ConnectController`（跳过 NVIDIA 显示句柄）提前到 banner 之后、一切枚举之前再跑一遍（`EARLY:` 前缀）。Fast Boot 的懒连接策略若让 SFS 面不完整，#122 的"无日志早挂"会长得一模一样；这一步同时是它的排除实验。

版本串 `2.1.13-diag4`（diag3 同源），tx 构建 banner 带 `TX BUILD`。

资产（放 U 盘 `EFI\BOOT\BOOTX64.EFI`）：

| 文件 | 用途 | SHA-256 |
|---|---|---|
| `BOOTX64.EFI.2.1.13-diag4` | #170（tx，完整事务） | 见 `SHA256SUMS.2.1.13-diag4` |
| `BOOTX64.EFI.readonly-diag.2.1.13-diag4` | #122（只读） | 见 `SHA256SUMS.2.1.13-diag4` |
| ~~`BOOTX64.EFI.2.1.12-diag3`~~ | 旧 tx（分辨率低一档） | `7ebe9c4cc33457f4f45e99a807334f5276cda4b3a3ccb122d2cd946b24d6d033` |
| ~~`BOOTX64.EFI.readonly-diag.2.1.12-diag3`~~ | 旧只读 | `38881809164eb2e0f361cc98c03baacb856ea3220bb746b9087d3a736e6c65af` |

两文件勿混用：#122 的只读基线与 #170 的事务复现各用各的资产。

## 1. 通用纪律（两 issue 相同）

- 一次实验 = 一次**完整冷启动**（关机拔插头/长按 10 s 再上电），不要用重启代替；**挂死/STOP 后同会话绝不重试**。
- 每轮回收**完整 `loader.log`**（不是截图、不只贴尾部），并记录：轮次、冷启动还是重启、插 U 盘进系统与否、BIOS 相关项（CSM / Above 4G / xHCI Hand-off / EHCI Hand-off）。
- 日志缺行按"该路径未执行"解读，**不要为补行补跑**——缺行本身就是判据。
- 若 `loader.log` 仍为 0 字节：文件内容其实已在 FAT 链里，只是目录项未提交。把整个 U 盘做镜像（`dd`/Win32DiskImager/`dd for Windows`），对镜像跑 `fatcat -r <偏移>` 或直接按根目录项里的起始簇号顺着 FAT 表拷出簇链，即可还原到最后一个标记行。**请把镜像和还原文本一起附上**，不要只报"又是 0 字节"。

## 2. #170（tx 构建）：静音点切分

放常规 tx 卷根 marker（`\i_know_what_i_am_doing.txt` 等按你已有的配置即可，不放也行——手动确认同样有效）。期待路径：`DETACH` → `MUTED` → 拆挂/声明 → 事务 → `CONSOLE: mirror re-enabled after reconnect` → `FINAL` → Windows。

判读表（**日志最后一条 `DETACH*`/`CONSOLE*` 行是哪个，就查哪一行**）：

| 日志尾部 | 判读 | 下一步 |
|---|---|---|
| 过 `mirror re-enabled` 且 `FINAL`、Windows 正常 | 控制台镜像不是凶手；挂死发生在拆挂的 `Stop()` 链内 | D 保不住，按 2b 逐项夹（stage 句柄 → PCI 句柄 → 是哪个绑定驱动的 `Stop`） |
| 停在 `DETACH: console mirror MUTED until reconnect` 之后、`GOP disconnect status` 之前 | 同上，而且已缩到"`DisconnectController` 第一跳"之内；此时屏幕定帧属预期（镜像已关） | 把 `protocols` 数与 `DIAG-CONSOLE-OUT` 的锚点一起附上 |
| 停在 `MUTED` 之前（`DETACH: stage GOP handle` 或更早） | 与拆挂/控制台无关，是 claim/属性捕获段 | 附 `EARLY` 与 `DIAG-*` 全段 |
| 停在 `EARLY:` 行 | 早期枚举（递归连接）本身挂 | 附完整段；标出 `EARLY: reconnect skipped on ...` 列表 |

同一平台上 **2 轮即可定性**（1 轮成功 1 轮挂 = 非确定性，按概率实验对待，至少 5 轮）。每轮都冷启动。

## 3. #122（只读构建）：只读是不是也 14%

**先做这个从没人做过的实验**：把 tx 换成只读构建，其余一切不变，连跑 **20 次冷启动**。只读构建从不 detach、从不写硬件，所以：

- 只读也挂 ≈ 同样概率 → 病灶在事务之前（固件/枚举/控制台初始化），事务与 PLM 线全部出局；
- 只读不挂（20/20 全绿） → 14% 是 detach/事务段引入的，与 #170 同源方向查。

每轮填一行（轮次 / 结果 / 尾部行 / 是否 0 字节）：

| 轮 | 结果 | 日志尾行 | 0 字节 | 备注 |
|---|---|---|---|---|
| 1 | | | | |
| … | | | | |
| 20 | | | | |

判读要点：

- 挂死点若在 `EARLY: reconnect pass` 段 → 枚举/`ConnectController` 路径（正是前移的那段）；
- 挂死在 `DIAG-CONSOLE-*` 前后 → 控制台身份相关（看 `uart=`/`(NVIDIA GPU)` 标记）；
- 若 20 轮里出现 ≥1 次挂死且日志停在 `EARLY` 之前 → 连"枚举前"都不是，是固件在我们代码之前的阶段（与 #122 首轮 0 字节现象一致），此时只读结论就足够结案，不用再上写操作。
- 20 轮全绿也要交：全绿本身即"只读基线干净"，下一轮再谈是否换 tx 复现。

只读构建的确认段与旧版相同（任意键/30 s 继续），**不会**有 `DETACH`/`FLR` 行，这是正常的。

### 3b. 增补（2026-11-05，报告者已带回自建 A/B）

报告者用自编构建跑出 **只读 2/2 全绿、tx 0/4 全挂**（同盘同码），并把挂死窗口推断为"`logging::init` 创建文件后、首行落盘前"。两点修正与一条补测：

1. **0 字节不能证明"没到首行落盘"**。这些 AMI 固件上 `Flush()` 是空操作（#170 实测：40+ 行含 `DETACH` 仍 0 字节，而优雅轮在 Close 时写了 13 KB），只有 Close 提交目录项——所以旧构建里**任何**挂死（含确认屏、含准备段）都表现为 0 字节。窗口要靠 diag4 的逐行提交来测，不靠字节数倒推。
2. **自建 A/B 有 profile 混淆**：只读用 `cargo build`（dev），tx 用 `--release`。缺的两格必须补：`cargo build --release` 的只读、与 `cargo build` 的 tx（命令见 issue 回帖）。若 release 只读也挂 → 是镜像尺寸/profile，不是 `tx` feature；若 dev tx 也挂 → feature 引入的代码。
3. diag4 下**每一行都是提交点**，文件创建本身也先提交。判读直接读最后一行：`log file` → 死在 logger 之后；`nv-uefi-unlocker` → 死在版本行之后；`BOOT: logger init returned` → 死在 quiet 探测/watchdog 段；`PREP:` → 死在武装准备段（枚举/GOP/采样/资产读）；`DIAG-`/`EARLY` 段同理。只有"文件已创建且 0 字节"才代表死在 init 的三条调用内。

## 4. 两个 dump 怎么读

`DIAG-CONSOLE-OUT ... anchor=<seg>:<bus>:<dev>.<fn> = <vendor>:<device>`：

- `(NVIDIA GPU)` = 控制台由被拆的卡自己的 GOP 驱动——#170 的挂死若在静音点后，这一条就是必读证据；
- `uart=1` = 串口控制台（很可能还有一路 VGA/GOP 并行，两条都会打印）；
- `no PCI node` / `no PciIo handle` = 固件没给设备路径或不是 PCI 设备（如内置 VGA 的 legacy 句柄），如实记录即可。

`DIAG-BOOTVOL ... anchor=`：看 `:14.0`（xHCI）/`:1d.0`/`:1a.0`（EHCI #1/#2）。若两块问题板都落在 EHCI 而 #121 落在 xHCI，强制 EHCI/xHCI 的 A/B 才有意义；反之就可以先把 USB 控制器变量放下。

## 5. diag5（2026-11-05）：FWSEC 点火段逐行解析（#170）

**背景**：报告者从 U 盘 FAT 簇链还原出两份完整的 v2.1.10 日志（各 261 行），证明事务在 5.6s 完成 detach、跑过 AUTH-SCAN、SEC2 scrubber（ETimeout，按 community 形态容忍）与 WPR2 layout，证据停在 ~10.9s 的 `TX: layout ok`。两个修正随之成立：①"挂在 DETACH"作废（屏幕末行只是控制台拆除后的定格帧，不是执行终点）；②但"停在 layout ok"**同样未证实**——`layout ok` 与下一条日志调用之间是相邻语句，中间没有任何可挂的东西，所以在 v2.1.10 的取证管线（Flush 不提交目录项）下，"后面没有行"是弱证据。

**diag5 的改动**：FWSEC 点火段每一步都发 `FWSEC:` 前缀的提交点行；1MiB PROM 批量读改为 16 块分读 + 每 25% 一条进度行（行为等价：同一窗口、同一访问器、同序）；`TX:` 与 `layout` 加入落盘标记集。**寄存器操作与顺序零改动**。

判读表（**读最后一行**；若目录项又没提交，簇链还原法照用——本报告者已实证可行）：

| 日志末行 | 停点 | 判读 |
|---|---|---|
| `TX: layout ok` 之后无 `FWSEC: reading PROM` | 那一行日志自身（屏写或写盘） | **见 §6**：BAR0 读在死点两侧都通，FWSEC 尚未开始；它是日志管线嫌疑，不是点火嫌疑 |
| `FWSEC: PROM read NN%` | 1MiB PROM 批量读中途 | BAR0 PROM 窗口在读取中段挂——MMIO 层，新失败形态 |
| `FWSEC: image staged` / `fuse` / `GSP falcon reset begin` / `reset+unblock ok` | staging/复位段 | 复位传播或 FBIF 解锁挂；附 `Falcon reset begin` 内部行 |
| `FWSEC: IMEM portal begin (N blocks)` | IMEM 逐块寄存器写中途 | GSP 寄存器门户写在第 N 块附近挂 |
| `FWSEC: DMEM portal done` / `PKC+BOOTVEC writes begin` | 点火参数段 | 点火前最后一步 |
| `FWSEC: HALT wait begin (5s cap)` | 5 秒有界等待内 | 若后续断电前再无任何行：CPU 死在等待内的某次 MMIO 读；若 `portal forensic` 出现则等待已正常结束 |
| `FWSEC: portal forensic ...` 之后 | 等待结束后的读回/校验 | 看 CPUCTL/MAILBOX0/EXCI 三值定 FWSEC 自身结果 |

A/B 开关（每次冷启动只动一个，本地化之后再动）：GA10x 的 FWSEC staging 没有专用 marker（`\tx-fwsec-vram-staging.txt` 是 Turing 路径的），所以第一变量不是 staging；判读落在哪一段，再按该段开新变量。`scrubber did not complete (ETimeout)` 对 30 系**不是**判据（GA106 2504 全链成功轮同形态），不用为它做任何事。

资产：`BOOTX64.EFI.2.1.11-diag5`（tx）。回收清单照 §1，另加：**请把两份还原日志、目录项原始 32 字节、78 个孤立簇清单与 diag4 轮的屏幕照片（banner 是否出现过）随下一轮一起附上**。

## 6. 2026-11-06 勘误：#122 的 37 会话还原把死点指到日志管线

#122 报告者按 §1 做了整盘镜像 + 簇扫描，还原出本盘 37 个会话、6197 行。三条事实：

1. **32/37 止于 `DETACH: PCI disconnect status Ok(()) 2 -> 2`，横跨 4 种二进制**（v2.1.9 / 官方 diag2 / 自建默认 / 自建 slim）；另 2 个会话穿到 `TX: layout ok`；无一会话到达 FWSEC。
2. 两个会话（含 26.7s 那个）的**屏幕都定格在 `DETACH: stage GOP handle`，日志再走 1–2 行**——与 #170 同形。即屏幕不是挂死时定格的，是**在 GOP 那次 `DisconnectController` 调用内部被拆死的**：ConOut 的底层 GOP 子句柄被销毁，此后 `TextOut` 仍返回、只是画不出东西。
3. 只读构建从不 detach（`#[cfg(not(feature="tx"))]` 跳过整个 `tx_phase`），控制台永不死 → 自建 A/B 的 "只读 2/2 过、tx 0/4 挂" 量到的是**detach 与否**，不是 tx 代码与否；体积/opt/lto 矩阵全部无效，二进制布局一支销案。

**推论（改写 §5 的 `layout ok` 行）**：`log()` 是先写文件、后写屏，所以任何一行的死点落在"该行屏写 → 下一行写盘"之间时，文件恰好结束在该行。三条死线（`PCI disconnect`、`layout ok`）的上一动作都是成功的设备/BS 操作，下一动作都是一次 `log()`——**病灶在日志管线（对已死控制台的屏写，或写盘），不在 GPU 代码**。`layout ok` 判据尤其明确：`layout_from_registers()` 自身就是 5 次 BAR0 读且全部成功，紧随的 `entry WPR2` 也只是 `read32`，**BAR0 在死点两侧都通**，FWSEC 尚未开始。

**diag5 的静音就是这组判据的 A/B**（它在 GOP disconnect 前一行静音，直到 FLR 后的 reconnect）：

| diag5 结果 | 判读 |
|---|---|
| 穿过 detach | 屏写是凶手；静音进默认路径（对这两块板是真修复，不只是仪器） |
| 仍止于 `DETACH: PCI disconnect` | 屏写销案；下一嫌疑 = `log_display_topology()` 的枚举（`DIAG PCIIO-OPENERS` 段），下一轮把它逐行化 |
| 止于 `DIAG PCIIO-OPENERS` 前后 | 直接落在枚举里 |

**第三条候选（写盘，2026-11-06 补）**：37 个会话里穿到 `layout ok` 的 2 个恰好是全程最快的两个（4.37s），32 个死在 `PCI disconnect` 的在 5.4–26.7s，行间有 1.5/2.4/10.2s 的 BOT 停顿——**USB 写盘楔死也是候选，且速度相关性偏向它**（屏写若为凶手，每轮屏写次数几乎相同，速度不该有影响）。diag5 的标记行是 write→flush→Close→重开，写入量远大于 v2.1.9：**每轮必须记总耗时（首末时间戳差）**；diag5 比以往同位置更早死 → 写盘升格、屏写降级。另：无核显是前提不是充分原因（#121 唯一 GOP 被拆仍全绿），两板烈度差（2 行 vs ~150 行）在固件处理方式。

**读法变化**：diag5 之后**屏幕在 `DETACH: console mirror MUTED until reconnect` 冻结是正常的，每一轮都冻（含全绿轮）**，只认日志。

**仍成立的教训**：`protocols` 那两个数是 `protocols_per_handle()` 的**已安装协议接口数**，不是 opener 计数——GOP 的 `5 -> 4` 是子句柄接口随 Stop 卸载，PCI 句柄的 `2 -> 2` 是 DevicePath+PciIo 由 bus driver 持有、Stop 不卸载，属预期，不是泄漏。`DETACH: PCI disconnect` 之后不再进任何 `Stop()` 回调（回调已在返回 `Ok` 的那次调用里跑完）。

## 7. diag6（2026-11-06）：日志 Close 阶梯，diag4/5 在本板死在更上游

diag5 在 #122 上 3/3 冷启动同一签名，且比 diag5 瞄的点更上游：**dirent 每轮被新鲜重建、但零字节、零 FAT 新分配、零字符串命中（53 个孤儿簇 / FAT 双副本差异簇 / 沉积区全检）、显示器无信号**。diag4 一轮同签名，合计 4/4。判读表（PROM/GSP/IMEM/HALT）在该板不可达。

**窗口（代码逐条，无省略）**：`create(CreateReadWrite)` → `into_regular_file` → `Close`（dirent 在此提交；版本独有）→ `Open(ReadWrite)` → `set_position(EOF)` → `set_logger` → 第 1 行 `format`/`Write`/`Flush`/`Close`/`reopen`/屏写。v2.1.9 同窗口只有 `create` → `set_logger` → 第 1 行 `write+flush+echo`，无 Close、无 reopen、无 set_position。**banner 在 init 之后，`DIAG-*` 转储与 `EARLY` connect 在 banner 之后**——它们只能杀第 3 行之后，对第 1 行不成立。dirent 新鲜 = `Close #1` 完成，剩余候选只有 `Open#2 / set_position / 第 1 行 Write / Close#2 / reopen#3`。

**判定式（充分条件式，可用于任何报单）**：若目录项最后写入时刻新鲜（=当轮开机后数秒）**且** size=0 / first_cluster=0，且孤儿簇 / FAT 双副本差异簇 / 沉积区检索均零日志内容 → 死亡窗口 = create 提交之后、第 1 行写→提交完成之前（Write 曾执行但未达媒体，介质上不可区分）。**用户侧初筛：dirent 新鲜重建 + size=0 本身就值得上报。**

**`2.1.11-diag6` = 阶梯构建**（默认 v2.1.9 写路径，逐项开关，单二进制）：

| 根目录文件 | 作用 |
|---|---|
| 无 | 默认：write + 标记/节拍 Flush，单句柄，永不 Close（v2.1.9 语义） |
| `tx-log-commit-lines.txt` | + 标记行 Close/reopen |
| `tx-log-commit.txt` | + create 时 Close（= diag4/5） |
| `tx-no-ident-dump.txt` / `tx-no-early-enum.txt` | 跳过转储 / EARLY connect |

开关在日志创建**之前**读：死在这两个探测里则**连新鲜 dirent 都没有**——这是"create 之前"与"create 之后"唯一可用的判别位。命令顺序：空盘 → （不新鲜）预读探测 → （新鲜）先关 dump、再关 EARLY → 恢复后保留该级、再单独试 `tx-log-commit-lines`。

## 8. diag7（2026-11-06）：Close/reopen 机制的第二种固件失败形态（#198）

#198（Y9000P IRX8H / RPL-HX / Lenovo 固件）报 diag5 两构建日志恒 0 字节：tx 与 readonly 皆然，**readonly 成功跑完也 0 字节**，768MB 全簇考古零命中；v2.1.10/diag2 同盘 47–56KB 正常。代码定位：diag3+ 的 create-commit 舞步（`CreateReadWrite → Close → Open(ReadWrite) → set_position(EOF)`）在该固件上 **reopen 链干净失败** → logger 整轮降级 console-only → 数据从未进文件。同一套机制在 #122 Z97-K 上是**楔死**（4/4 init 窗口无信号）——两种失败形态、同一处代码、两个独立机型。

**`2.1.11-diag7`**（tag=提交 a0ebe01，资产 tx `f2340566…` / readonly `5c27e80b…`）：reopen 失败时 init 现场自动降级——趁零写入改取新句柄、整轮强制 level 0（v2.1.9 写路径）。日志第 3 行 `BOOT: diag7 log-commit=N (…; auto-degrades to 0 …)` 自报实际状态。开关与 diag6 一致（`tx-log-commit[-lines]` / `tx-no-ident-dump` / `tx-no-early-enum`）。

**判读修正**：①Y9000P 上 level 2/默认都能拿到完整日志（后者经降级）；②Z97 阶梯轮照旧，默认=0 从构造上避开楔死窗；③"Close/reopen 机制在 AMI 系固件上有两种失败形态"入档——**任何报"diag 线 0 字节且成功跑完"的轮，先查第 3 行 log-commit 值与是否出现降级路径**，不要再按窗口推断。铁律重申：diag 线默认 level 0 = 历代 shipping 路径，永远可用作基线。
