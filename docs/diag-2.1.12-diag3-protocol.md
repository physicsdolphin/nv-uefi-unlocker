# 诊断构建 2.1.12-diag3 取证协议（2026-11-05）

服务两个"怪死"平台：

- issue [#170](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/170)（MSI B85M，`DETACH` 阶段确定性挂死，Xeon E3-1231 v3 无核显）——用 **tx 构建**；
- issue [#122](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/122)（ASUS Z97-K，约 14% 概率的极早期挂死，同款无核显）——用 **只读构建**，连跑 20 次冷启动。

两块板 + 讨论 [#121](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/121)（H61，正常）的共同点与差异（iGPU 有无、Aptio 代次、显卡）都已入档；本构建只回答一个问题：**挂死发生在"控制台被拆掉"之前还是之后**，并把枚举/控制台/宿主控制器一次钉死。

## 0. 本构建是什么 / 改了什么

**没有任何新增硬件写**。事务链、payload、FLR 顺序与 `2.1.11-diag2` 逐字节同源（`2.1.12-diag3` 只在日志与枚举外围加东西）：

1. **A：标记行落盘改为"写 → 关文件 → 重开"**。此前 0 字节日志的根因已查明：`FatIFileAccess` 成功路径不刷缓存，`Flush()` 在这些固件上等于空操作，只有 `Close` 才提交目录项（大小）。现在每个 `DETACH`/`POST-`/`FINAL` 等标记行后都关闭并重开 `\loader.log`（写入位置在末尾，文件内容不丢）。**挂死点现在会留在 U 盘上，一个字符不少**。
2. **D：拆挂前静音控制台镜像**。`DETACH: console mirror MUTED until reconnect` 之后所有日志只进文件，直到重连/交棒前再解除（`CONSOLE: mirror re-enabled ...`）。这一刀把两种死法分开：**过得了静音点 = 死在 `DisconnectController` 的 `Stop()` 链里；过不了 = 死在"对已被拆掉的控制台写屏"上**。
3. **两个只读身份转储**（banner 后立即打印，挂死前就已落盘）：
   - `DIAG-CONSOLE-OUT/IN`：固件给我们的是哪一路控制台——设备路径解码出的锚点设备，`(NVIDIA GPU)` 标记表示控制台挂在被拆的那张卡上，`uart=1` 表示串口；
   - `DIAG-BOOTVOL`：本 U 盘所在的宿主控制器。8 系 PCH 的 EHCI #1=`:1d.0`、EHCI #2=`:1a.0`、xHCI=`:14.0`（USB2 口的实际路由由 `xUSB2PR` 决定，光看插哪个口不算证据）。
4. **枚举前移**：chain 阶段那次递归 `ConnectController`（跳过 NVIDIA 显示句柄）提前到 banner 之后、一切枚举之前再跑一遍（`EARLY:` 前缀）。Fast Boot 的懒连接策略若让 SFS 面不完整，#122 的"无日志早挂"会长得一模一样；这一步同时是它的排除实验。

版本串 `2.1.12-diag3`，tx 构建 banner 带 `TX BUILD`。

资产（放 U 盘 `EFI\BOOT\BOOTX64.EFI`）：

| 文件 | 用途 | SHA-256 |
|---|---|---|
| `BOOTX64.EFI.2.1.12-diag3` | #170（tx，完整事务） | `7ebe9c4cc33457f4f45e99a807334f5276cda4b3a3ccb122d2cd946b24d6d033` |
| `BOOTX64.EFI.readonly-diag.2.1.12-diag3` | #122（只读，20 连跑） | `38881809164eb2e0f361cc98c03baacb856ea3220bb746b9087d3a736e6c65af` |

两文件勿混用：#122 的 20 连跑若用 tx 构建，第一轮就会带写，14% 基线不再是"只读基线"。

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

## 4. 两个 dump 怎么读

`DIAG-CONSOLE-OUT ... anchor=<seg>:<bus>:<dev>.<fn> = <vendor>:<device>`：

- `(NVIDIA GPU)` = 控制台由被拆的卡自己的 GOP 驱动——#170 的挂死若在静音点后，这一条就是必读证据；
- `uart=1` = 串口控制台（很可能还有一路 VGA/GOP 并行，两条都会打印）；
- `no PCI node` / `no PciIo handle` = 固件没给设备路径或不是 PCI 设备（如内置 VGA 的 legacy 句柄），如实记录即可。

`DIAG-BOOTVOL ... anchor=`：看 `:14.0`（xHCI）/`:1d.0`/`:1a.0`（EHCI #1/#2）。若两块问题板都落在 EHCI 而 #121 落在 xHCI，强制 EHCI/xHCI 的 A/B 才有意义；反之就可以先把 USB 控制器变量放下。
