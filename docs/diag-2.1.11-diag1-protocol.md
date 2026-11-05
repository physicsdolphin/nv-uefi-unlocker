# 诊断构建 2.1.11-diag 取证协议（2026-11-04）

> 服务 issue [#164](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/164)（GA104 2484，EFI 全绿但系统内 SS 对被抹）与 [#165](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/165)（双 3060，交棒后一张卡在 Windows 侧消失）。
>
> **资产已于 2026-11-04 第五次替换（新加 POWER 电源态读数，`ssdiag` 升 0.6.8；事务本体一字未动）**：事务链、payload、FLR 顺序、native-GOP 重建仍与 v2.1.10 逐字节同源；新加的写只有两个 PBUS 软件 scratch 字（`0x1480`/`0x1484`，即索引 `0x20`/`0x21`，FLR 验证之后写、立刻读回），其余全部是只读采样。**此前下过同名文件的请重新下载**——更早的文件金丝雀偏移写错、SSDT 全部写盘失败、索引行无换行，TU10x 的 `POST-GOP` 判定也会误报 `NOT intact`。
>
> 金丝雀的判读在 §3；开关是卷根 `\tx-no-canary.txt`（放上即整组跳过，用于 A/B）。可选 ACPI 转储见 §5，配套的 `POWER:` 系统内读数见 §5b。
>
> 版本串 `2.1.11-diag1`，banner 带 `TX BUILD`。资产：`BOOTX64.EFI.2.1.11-diag1`，SHA-256 `a85e282be230eb1413fe6718930a343103381b8858893d18f6b98777c738f107`。
>
> **2026-11-04 追加：`v2.1.11-diag2` 已发布**（`BOOTX64.EFI.2.1.11-diag2`，SHA-256 `1046c64a17817b78481a521d457919b0d4409a7556e75e5896ad90453d088c7a`）。与 diag1 的唯一行为差异 = **交棒前默认把资产池还给固件分配器**（issue #146 的修复，`\tx-keep-assets.txt` 可退回）＋ `CHAIN-MEMMAP` 摘要新增 `largest block` 与描述符总数。**两个 issue 的协议一字不变**，只换 `EFI\BOOT\BOOTX64.EFI` 即可；新构建跑出来的金丝雀与 `POST-GOP` 判读与 diag1 同源逐字节等价（事务链未动）。
>
> `ssdiag.exe.0.6.8` 同附在本 release（含 `CANARY:` 判定行、安全模式/未启动设备上的发现回退、`POWER:` 卡与根端口电源态；金丝雀偏移为 `0x1480/0x1484`。管理员运行），自行编译亦可。
>
> **2026-11-04 追加：`ssdiag.exe.0.6.9` / `0.6.10` 也都已附上**（SHA-256 见 `SHA256SUMS.2.1.11-diag2`）：0.6.9 加 Blackwell（GB20x）只读面（#168 的 FEAT/熔丝行、`READOUT_1` 非连续位解码、PL3-only 判读，`--reapply` 在 50 系上明确拒绝）；0.6.10 加 Ampere/Ada 的逐 GPC TPC floorsweep 只读块（#169 的 STATUS/CTRL、GPC disable/defect、缺陷 vs 覆盖候选判读）。`CANARY:`/`POWER:` 与上文协议完全不变，本协议仍钉 0.6.8 的行义（0.6.9/0.6.10 全含）；任选其一即可。
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

1. **安全模式**：先在**正常模式**的提权会话里跑 `ssdiag --safeboot-arm`——**只注册表预置**：写 `SafeBoot\Minimal`+`Network` 两个白名单子键（安全模式只启动白名单里的内核驱动，这是 win32 1084 = `ERROR_NOT_SAFEBOOT_SERVICE` 的闸；检查按服务名匹配，键的默认值只是标签），**不落任何 .sys、不建任何服务**——安全模式内 `ssdiag` 照现行运行时模式抽 `%TEMP%`、建服务、启动、退出时照常 remove-when-done，驱动文件的整个生命周期只有探测运行的几秒。arm 自带一轮**与安全模式完全相同的临时布局**验证（deploy→start→开设备→退出自清）。然后 `bcdedit /set {current} safeboot minimal` 重启进安全模式，提权跑 `ssdiag`；回正常模式后 `ssdiag --safeboot-disarm` 删两个键（兼清旧版残留，零残留有回读）。安全模式不加载 `nvlddmkm`（基础显示驱动接手）：安全模式里也被抹 → 不是驱动包；安全模式完好、正常模式被抹 → `nvlddmkm` 的 init 就是写入方。注意 BitLocker 会要恢复密钥；安全模式在本仓库有黑屏先例（#82 X99），黑屏就转第 2 轮。忘掉 arm 直接跑会撞 1084——0.6.12 会当场写白名单键自愈重试一次（若该 Windows 构建的白名单是逐次活查询则成功，工具会提示补跑 `--safeboot-disarm`），失败则打印上面三步。**两个残余未知**：①安全模式可能存在比正常模式更严的签名形状检查（社区观察：EV+微软联签可过、SHA-1 双签被拒）；②MUXless 机器 dGPU 停泊时 BAR0 读不到（#126）——这两者任一命中时该腿拿不到读数，转第 2 轮。
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

## 3. 复位金丝雀（两 issue 都适用）

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

`cleared` 两行只说明"发生过复位"，不说明是谁做的。按机制分行（撤电/全片复位/devinit 重推导/窄写/本工具尾巴/OS 连接管理器）见 [FAQ 回锁一节](faq.md#relock)。

## 4. 采样面的一条警告（两 issue 都适用）

M1 `SAMPLE` 对非 console 卡可能整行全 `0xffffffff`，连 `PMC_BOOT_0` 都是全 1——这是固件把该卡留在 `cmd=0x0000` 未解码的探针假阴性（#165 日志里 `03:00.0` 即如此），**不是坏卡**。判身份用枚举行的 `10de:2504` / `2484`，不要拿 SAMPLE 全 1 下结论。

## 5. `\tx-acpi-dump`：把平台 AML 带回来（可选，两 issue 都适用）

回锁争论最后总会落到"谁拥有这张卡的电"上：根端口下的设备有没有 `_PR3` 电源资源、`_ON`/`_OFF` 是否幂等、PERST# 挂在哪个 GPIO。答案在平台 AML 里，而 AML 需要反编译器——EFI 侧不内置，只负责把原表搬出来。

卷根放 `\tx-acpi-dump.txt`（内容随意）即生效：在确认按键**之前**执行，与有没有 N 卡无关，纯只读。产物落在卷根 `\acpi\`：

- `dsdt.aml`（经 FACP 的 DSDT/X_DSDT 指针取得——DSDT 通常不在 XSDT 里）；
- `ssdt-NN.aml`（XSDT 里每张 SSDT）；
- `index.txt`：每张表的签名、长度、校验和状态（`ok`/`BAD`）、文件名、creator，以及 `_PR3`/`_ON_`/`_OFF`/`PRST`/`_ADR`/`_HID`/`_CID`/`GPIO`/`_DSM`/`_STA`/`_PS0`/`_PS3`/`_PXM`/`_DOD` 的出现次数与首个偏移。

反编译在开发机上做（WSL：`apt install acpica-tools`，然后 `iasl -d dsdt.aml ssdt-*.aml`），把 `.dsl` 与 `index.txt` 一起附回即可。

边界说明：这是**固件当时的视图**，OS 运行期动态生成的 SSDT 不在其中；表可能包含整机序列号类信息，回收时留意。dump 失败不影响事务，日志里有 `ACPI-DUMP:` 行记录每一步。

一个健康的 `index.txt` 是**一表一行**、以文件名接 `written` 结尾。首次实机跑暴露过三个只在这条路径上的缺陷，现已修：SSDT 落盘文件名误带 `\acpi\` 前缀（句柄本来就是 `\acpi`，于是每张 SSDT 都 `WRITE FAILED (create failed)`）、索引行之间没有换行、creator 偏移误读在 +36（正确 +28，表体也自 +36 起）。若 `index.txt` 又是连成一行，说明盘里的 `BOOTX64.EFI` 还是旧构建。

## 5b. `POWER:`：卡与根端口的电源态（`ssdiag` 0.6.8 起，系统内只读）

§5 带回来的是"平台**有权**怎么做"，`POWER:` 两行是"平台**此刻**做到了哪一步"：分别打印 GPU 本身与其上游根端口的 `PMCSR`（PCI PM 能力 +4）与解码后的 D 态。PMCSR 的 `PowerState` 位是 R/W，但本工具只读不写。

判读：

| `POWER:` 读数 | 判读 |
|---|---|
| 卡 `D0`、根端口 `D0` | 两端都在 D0，电源路径此刻未介入——丢锁是**写入型**，与金丝雀 `intact` 相互印证（本机 TU106 基线即此形态） |
| 根端口 `D3` | 平台 `_PR3`/`_OFF` 路径**已介入**：PERST# 已拉低、槽位电源已切。此状态下解锁**不可能存活**（`0x1480/0x1484` 也被清），同时是"卡显示上消失"（#165）的直接候选 |
| 卡 `D3`、根端口 `D0` | 仅卡片自身 D3hot/D3cold，链路复位级；根端口轨道仍在，`_OFF` 未跑 |
| 卡 `config space unreadable` | 卡已被切电或函数未解码——先看根端口一行再下结论 |

两行的根端口身份直接打印（`BB:DD.F` + `VEN_xxxx&DEV_xxxx`），可与 `\acpi\index.txt` 里的 `_PR3` 命中位置对上：被打印的端口正是那个拥有 `_OFF` 的对象。**局限**：只是探针时刻的快照，`D0` 不证明这一引导内没进过 `D3`（那由金丝雀回答），`D3` 也不区分是谁（驱动/系统电源策略/空闲计时器）造成的。

**English**

- `POWER:` prints the card's and its upstream root port's `PMCSR` (PCI PM capability +4) with the decoded D-state. Read-only; the `PowerState` bits are never written by `ssdiag`.
- Both `D0` ⇒ the power path did not engage at probe time; a lost unlock is write-type (pairs with a canary `intact`).
- Root port `D3` ⇒ the platform `_PR3`/`_OFF` path has run (PERST# asserted, slot rail cut); the unlock cannot survive that state, and it is the prime candidate for a card disappearing (#165).
- Card `D3` with the port in `D0` ⇒ card-side D3hot only, rail intact, `_OFF` not run.
- It is a snapshot: `D0` does not prove the port never went to `D3` this boot (the canary owns that), and `D3` does not say who drove it.

## 5c. 雷电/USB4 预设与 `USB4:` 行（#54 类，`ssdiag` 0.6.13 起）

雷电形态的丢失来自 OS 连接管理器在**设备路由器侧**接管隧道时的那次总线复位（[#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)，两台主机栈实测）。跑之前先把抑制打开，否则金丝雀必然 `cleared`、结果没有信息量：

```bat
reg add HKLM\SYSTEM\CurrentControlSet\Services\Usb4DeviceRouter /v Start /t REG_DWORD /d 4 /f
:: 复原
reg add HKLM\SYSTEM\CurrentControlSet\Services\Usb4DeviceRouter /v Start /t REG_DWORD /d 3 /f
```

- 只禁 `Usb4DeviceRouter`，`Usb4HostRouter` 保持 `3`；两个都禁 = 隧道不存在。
- 代价：坞的设备路由器服务失效、坞上 USB 口死、`USB4\ROOT_DEVICE_ROUTER` 节点 `Problem 32`（Code 32=服务被禁用，非故障）；GPU 与卡载音频不受影响。
- `ssdiag` 0.6.13 起 `PLATFORM:` 段后打印 `USB4:`：两个服务的 `Start`、USB4 节点 ID、以及"抑制是否生效"的判读行。报告必附这四行。
- 未测：抑制状态下重插坞/睡眠唤醒后卡是否回来（保守=开机前插好）。Linux 侧对应项是内核命令行 `thunderbolt.host_reset=0`（已实测可行）。

## 6. 回收清单

1. 完整 `loader.log`；
2. Windows 侧：`Get-PnpDevice -Class Display | fl FriendlyName,Status,Problem,ProblemDescription,InstanceId`（#165 在**解锁后未重启的病态状态里先取**，再谈重启）；
3. `ssdiag` 输出（#164 必附；#165 附可用卡的读数即可，`ssdiag` 不加 `--select`，读 Asus 卡那一块——`--select` 只匹配 DeviceID 子串，BDF 不在其中）——需 **ssdiag v0.6.8 及以上**，含 `CANARY:` 判定行与 `POWER:` 两行；雷电场景用 0.6.13 及以上并附 `USB4:` 四行；发现阶段空结果时工具会逐条打印跳过原因，连 `display-class devnodes carrying VEN_10DE: N` 一起贴回来；
4. BIOS 状态与电源瓦数（#165 必附）。
