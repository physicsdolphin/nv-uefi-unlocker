# 诊断构建 2.1.11-diag1 取证协议（2026-11-04）

> 服务 issue [#164](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/164)（GA104 2484，EFI 全绿但系统内 SS 对被抹）与 [#165](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/165)（双 3060，交棒后一张卡在 Windows 侧消失）。
>
> **本构建不改变事务本体**：解锁链、payload、FLR 顺序、native-GOP 重建与 v2.1.10 逐字节同源，只多两处**只读**采样。版本串 `2.1.11-diag1`，banner 带 `TX BUILD`。
>
> 资产：`BOOTX64.EFI.2.1.11-diag1`，SHA-256 `8e04988c171e814b069c3981ec322e7b0eb7f9ae8009793d06293d388a65b45b`。
>
> 构建入口：与 v2.1.10 相同（`make-usbroot.ps1`），把 `BOOTX64.EFI` 换成该文件即可；卷根 marker 按各 issue 下面的要求放。

## 0. 通用纪律（两 issue 相同）

- 一次实验 = 一次完整冷启动；**STOP/异常后同会话不重试**，完整断电（拔插头/长按 10s）后再来。
- 每次都回收**完整 `loader.log`**（不只贴成功片段），并注明：启动方式（冷启动/重启）、是否插着 U 盘进系统、BIOS 相关项（CSM / Above 4G）。
- 日志里新出现的两类行只读不写，出现与否都不影响事务成败；缺行按"该路径未执行"解读，不要补跑。

## 1. #164：`POST-GOP` 写入方切分（英文原文见下）

`POST-GOP` 行在**交棒尾部、控制台重建与 VGA 复原之后**、`StartImage bootmgfw` 之前打印，再读一次 FEAT 三连。它与更早的 `POST-FLR` 验证构成一对：把"谁抹了 SS 对"从三方（卡自己的 GOP/devinit、平台固件、Windows 驱动）压到两方。

**必须走默认路径**：不要放 `\tx-reconnect-legacy.txt`，也不要放 `\tu10x-no-reconnect.txt`（后者仅 Turing）。默认 native-GOP 重建会 `StartImage` 卡自己的 GOP 驱动，`POST-GOP` 才是"驱动加载后"的采样点；走 legacy 路径则该点只反映重连后的状态，判读要另注明。

| `POST-GOP` 读数 | 判读 | 下一步 |
|---|---|---|
| `plm=0xffffffff ss0=0x88888888 ss1=0x8` → `intact` | 交棒时完好，抹除发生在 OS 内 | 驱动这一 pass 负责；配 ssdiag 系统内读数即可闭环 |
| SS/PLM 已回厂值 → `NOT intact` | 丢在 bootmgfw 之前 | 卡自己的 GOP/devinit 或平台固件负责；Windows 驱动出局 |
| 行内带 `replay` 提示 | 本卡走了 replay 模式（快速 sanitize），FEAT 值不可作冷态判据 | 该卡数据单独标注 |

`POST-GOP` 与 `POST-FLR` 之间除重建外不改 FEAT；两行不一致即以 `POST-GOP` 为准。

**English**

- The `POST-GOP: FEAT_PLM=… SS0=… SS1=…` line is printed at the handoff tail, after the console rebuild and the VGA restore, right before `StartImage bootmgfw`. It is read-only.
- Please keep the **default** path (no `\tx-reconnect-legacy.txt`): the default rebuild `StartImage`s the card's own GOP driver, which is exactly the writer this line is meant to catch.
- `intact` here + wiped in the OS ⇒ the Windows driver pass did it. Already locked/lost here ⇒ it happened before bootmgfw (the card's own GOP/devinit or the platform firmware; the driver is excluded).
- If the line carries a `replay` note, treat the FEAT values as replay-mode, not cold-state.
- Report the full `loader.log` plus the `ssdiag` reading taken in Windows.

## 2. #165：`HANDOFF-VGA-FINAL` 交棒末态的路由位

旧构建的 `HANDOFF-PCI` 转储跑在 native-GOP 重建与 `VGA-RESTORE` **之前**，记录的 `bridgectl` 只是 detach 期间的中间态（`0x0010`），没有任何一行反映 Windows 实际继承的路由。本构建在 restore 之后补一次同格式转储，tag 为 `HANDOFF-VGA-FINAL`。

判读只看 `bridgectl`（PciIo 0x3e）低 4 位：

- **bit3 = VGA Enable**：该桥下允许 VGA 解码。**健康的双卡末态** = console 所属根端口 bit3 置位、另一张卡的根端口 bit3 清零。
- bit2 = ISA Enable、bit0/1 = VGA/ISA 16-bit 正向/反向译码，只作背景。
- 病态形态举例：两张卡的根端口 bit3 都置位（双 VGA owner，资源仲裁打架）；或 console 端口 bit3 反而清零（黑屏/Code 12 更前一步）；或非 console 端口 `cmd` 的 MEM 位（bit1）在先前 `HANDOFF-PCI` 里就一直是 0（桥窗没开）。

配套 marker：卷根放 `\tx-select-bdf.txt`，先写 `03:00.0`（丢的那张，Gigabyte）单跑一轮、再写 `04:00.0`（可用那张，Asus）单跑一轮——**每轮单独附上 Devices 现场照**。两轮各自单跑都过 → 双卡资源仲裁；`03:00.0` 单跑也丢 → 该槽或该卡。

## 3. 采样面的一条警告（两 issue 都适用）

M1 `SAMPLE` 对非 console 卡可能整行全 `0xffffffff`，连 `PMC_BOOT_0` 都是全 1——这是固件把该卡留在 `cmd=0x0000` 未解码的探针假阴性（#165 日志里 `03:00.0` 即如此），**不是坏卡**。判身份用枚举行的 `10de:2504` / `2484`，不要拿 SAMPLE 全 1 下结论。

## 4. 回收清单

1. 完整 `loader.log`；
2. Windows 侧：`Get-PnpDevice -Class Display | fl FriendlyName,Status,Problem,ProblemDescription,InstanceId`（#165 在**解锁后未重启的病态状态里先取**，再谈重启）；
3. `ssdiag` 输出（#164 必附；#165 附可用卡的读数即可，`ssdiag --select 04:00.0`）；
4. BIOS 状态与电源瓦数（#165 必附）。
