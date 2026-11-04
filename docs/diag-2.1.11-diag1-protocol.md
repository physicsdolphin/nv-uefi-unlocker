# 诊断构建 2.1.11-diag 取证协议（2026-11-04）

> 服务 issue [#164](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/164)（GA104 2484，EFI 全绿但系统内 SS 对被抹）与 [#165](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/165)（双 3060，交棒后一张卡在 Windows 侧消失）。
>
> **diag2 修订（2026-11-04，替换 diag1 资产）**：新增**复位金丝雀**。事务链、payload、FLR 顺序、native-GOP 重建仍与 v2.1.10 逐字节同源；新加的写只有两个 PBUS 软件 scratch 字（`0x140020`/`0x140024`，FLR 验证之后写、立刻读回），其余全部是只读采样。**已经下载 diag1 的请重新下载 diag2**——diag1 没有金丝雀。
>
> 金丝雀的判读在 §3；开关是卷根 `\tx-no-canary.txt`（放上即整组跳过，用于 A/B）。
>
> 版本串 `2.1.11-diag2`，banner 带 `TX BUILD`。资产：`BOOTX64.EFI.2.1.11-diag2`，SHA-256 `6f6d0ba305ef515b2e8ba39e2ebb32771801be976480693edb6fe0f513d2d64c`。
>
> `ssdiag.exe.0.6.5` 同附在本 release（含 `CANARY:` 判定行，管理员运行），自行编译亦可。
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

### 1b. 若两行都 `intact`、只有系统内被抹：在 OS 内继续切分

写入方在 OS 内部。两轮，按序，**能只做一轮就做第 2 轮**（它更强）：

1. **安全模式**：先 `bcdedit /set {current} safeboot minimal`，再照常插盘引导，进去后跑 `ssdiag`；事后 `bcdedit /deletevalue {current} safeboot`。安全模式不加载 `nvlddmkm`（基础显示驱动接手）：安全模式里也被抹 → 不是驱动包；安全模式完好、正常模式被抹 → `nvlddmkm` 的 init 就是写入方。注意 BitLocker 会要恢复密钥；安全模式在本仓库有黑屏先例（#82 X99），黑屏就转第 2 轮。
2. **零驱动 Linux live USB**：用第二支盘装 live ISO，在解锁盘根写 `\chain-target.txt` 指向该发行版的二级加载器（如 `\EFI\BOOT\grubx64.efi`，避免 pass 0 落回我们自己的 `BOOTX64.EFI`），**经解锁盘链过去**（解锁是每引导易失的，直接从固件启 live 盘只会读到厂值）。桌面里 `curl -O …/tools/diag-unlock.py && sudo python3 diag-unlock.py`（mmap `resource0`，零模块零驱动），并把 `dmesg | grep -iE 'nouveau|nvidia|pci'` 一并附上。live 桌面下、无驱动绑定就已被抹 ⇒ OS 交棒/电源态，不是厂商驱动；完好 ⇒ Windows 驱动这一 pass。

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

## 3. 复位金丝雀（diag2 新增，两 issue 都适用）

"还开着"不等于"没人碰过"：FEAT 值只说明写没被抹，说不清是**被写**、还是**整片被复位过**。金丝雀补这一维。

做法：`CANARY-PRE` 先抄一行现有值，然后在 FLR 验证之后往 PBUS 软件 scratch 写两个魔数（`a=0x0164c0de`、`b=0x0165cafe`），立刻读回验证（`CANARY-SET`）；此后 `POST-GOP`、`HANDOFF-OUT` 各带一次 `CANARY=<状态>(值/值)`，系统内由 `ssdiag` 的 `CANARY_A/B` 与 `CANARY:` 行给出同一状态。

为什么是这两个寄存器：open RM 的老驱动只写 `0x1e`（drain 路径），其余 scratch 都是 VBIOS/devinit 的地盘，`0x20/0x24` 两格干净；BUS 域随 FLR/SBR/撤电清零，正是要的见证。**链上唯一的写就是这两笔**，不参与任何判定。

判读（与 FEAT 值配对，单看无效）：

| 金丝雀 | FEAT | 判读 |
|---|---|---|
| `intact` | 完好 | 交棒到读回之间**没有任何复位**，也没人重写 |
| `intact` | 被抹 | 无复位 → **写入型**（驱动 init / devinit 重推导），不是电源事件 |
| `cleared` | 完好 | 有复位，但 AON 岛活下来 → **FLR/SBR 级**；恰好也解释"还能用" |
| `cleared` | 被抹 | 有复位且 AON 也死 → **撤电 / 全片复位**（平台侧），匿名回锁成立 |
| `partial` / `other` | — | 窄域复位或 devinit 重写过；按行内值个案分析 |
| `unreadable` | — | BAR0 读回毒值（停泊/未解码），本行不判 |

`CANARY-PRE` 里如果已经是旧魔数：说明上一轮实验的写入还活着——**上一次引导至今没复位过**，本身就是一个强数据点。

## 4. 采样面的一条警告（两 issue 都适用）

M1 `SAMPLE` 对非 console 卡可能整行全 `0xffffffff`，连 `PMC_BOOT_0` 都是全 1——这是固件把该卡留在 `cmd=0x0000` 未解码的探针假阴性（#165 日志里 `03:00.0` 即如此），**不是坏卡**。判身份用枚举行的 `10de:2504` / `2484`，不要拿 SAMPLE 全 1 下结论。

## 5. 回收清单

1. 完整 `loader.log`；
2. Windows 侧：`Get-PnpDevice -Class Display | fl FriendlyName,Status,Problem,ProblemDescription,InstanceId`（#165 在**解锁后未重启的病态状态里先取**，再谈重启）；
3. `ssdiag` 输出（#164 必附；#165 附可用卡的读数即可，`ssdiag --select 04:00.0`）——需 **ssdiag v0.6.5 及以上**，含 `CANARY:` 判定行；
4. BIOS 状态与电源瓦数（#165 必附）。
