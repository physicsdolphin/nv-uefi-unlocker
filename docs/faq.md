# 常见问题

[中文](faq.md) | [English](faq.en.md) · [README](../README.md) · [兼容性矩阵](compatibility.md)

- [什么是 UEFI / DXE / 链式引导？（背景概念）](#uefi-primer)
- [USB 文件和高级选项怎么放？](#usb)
- [Turing 为什么要先启用 GSP？](#turing)
- [混合输出、重启和休眠有什么区别？](#display)
- [Linux 日志全绿，为什么没有提升？](#linux)
- [解锁成功、性能却没有提升，怎么判断还会不会有收益？](#gain)
- [为什么解锁过的卡一进系统又被锁回去？](#relock)
- [Windows 睡眠后解锁还在吗？（S3 / S0ix 现代待机）](#sleep)
- [雷电 3/4/USB4 eGPU 能保留解锁吗？](#thunderbolt)
- [Secure Boot 开着能用吗？（自签）](#secureboot)
- [确认倒计时被“幽灵按键”中止（卡键）](#ghost-key)
- [怎样在系统内检查或修复解锁？](#ssdiag)
- [pmxdrv.sys 的安全模型与权衡（必读披露）](#pmxdrv)
- [杀毒软件报毒怎么办？](#av-fp)
- [booter status 0x89 是什么？](#bridge-bme)
- [选 U 盘一闪回到启动菜单或直接进系统，盘上没有 loader.log？](#no-boot)
- [事务成功后为什么会复位或黑屏？](#reconnect)
- [黑屏、STOP 或驱动异常后怎样恢复？](#recovery)
- [有哪些安全约束和实现边界？](#safety)
- [解锁是怎样实现的？](#mechanism)
- [与 nvoc 配合有什么条件？](#nvoc-results)
- [怎样反馈不兼容或异常？](#reporting)
- [代码和研究记录在哪里？](#layout)

<a id="uefi-primer"></a>

## 什么是 UEFI / DXE / 链式引导？（背景概念）

**UEFI** 是 BIOS 的后继者：按下电源后、任何操作系统启动之前，主板固件运行的那段程序。它负责初始化硬件、枚举 PCIe 设备、决定从哪个盘的哪个文件继续引导；它本身是一个小型运行环境——有内存分配、有文件系统驱动、可以加载并执行 `.efi` 应用。

**DXE**（Driver eXecution Environment）是 UEFI 启动过程的主体阶段：固件先把最基础的驱动搭起来，再逐个加载各 PCI 设备的驱动——显卡的 GOP（显示输出）、NVMe/USB 存储控制器都在这一阶段就位。**本工具就是一个在 DXE 阶段被固件加载的 UEFI 应用**：此时 NVIDIA 驱动还没有影子，而固件对显卡自身的初始化已经完成——这正是"驱动加载之前"的唯一可编程时窗。

**链式引导**（chain loading）指一个 UEFI 应用干完自己的活之后，用 `LoadImage`/`StartImage` 把下一个引导程序（比如 Windows 的 `bootmgfw.efi` 或 Linux 的 GRUB）加载进内存并把控制权交出去——接力赛交棒。本工具解锁完成后自动链回硬盘上原有的引导程序，所以"从 U 盘解锁"和"正常进系统"是同一趟开机，不重不漏。

**UEFI 是怎么进入操作系统的？** 固件按启动顺序在各盘的 EFI 系统分区（ESP）里找引导程序并执行；Windows 的引导器拿到控制权后加载内核、启动驱动，之后固件就完全退出舞台。本工具写入的解锁状态留在显卡的常开岛（AON）上，由操作系统原样继承——这就是"易失覆盖、每次引导重放"的含义。

**为什么说各种操作系统都能用？** 解锁发生在操作系统启动**之前**、作用在显卡硬件的速率选择器上；工具不关心、也不知道后面要启动哪个系统——Windows、Linux、Hackintosh 对它来说只是"下一个被链式引导的程序"，所以解锁流程对三者完全相同。与操作系统有关的只有启动**之后**的持久性表现（驱动 init 是否重推导、睡眠是否掉电），见[回锁](#relock)、[睡眠](#sleep)与[雷电](#thunderbolt)各节。

<a id="usb"></a>

## USB 文件和高级选项怎么放？

maker 在本地 `usbroot/` 输出资产（`*.bin` 平铺在该目录，另已建好 `EFI/BOOT/BOOTX64.EFI`）；把该目录的**内容**复制到 U 盘根目录（别把目录本身拷成 `U:\usbroot\`）。输出只含运行所需文件——抽取中间产物（完整 GSP ELF、取证用 `*_raw.c` 转储）不再混在输出里；`ASSETS_DEBUG_C=1` 可让 assets 管线重新吐出取证转储（仅调试用）。

**免 U 盘复制（可选）**：生成完成后 maker 会探测本机**现有** FAT32 分区并询问是否一键安装——内置盘（非 USB）上**非引导**（非 ESP、非系统/启动卷）的数据 FAT32 分区提示"持久化安装"（以后从主板一次性启动菜单直接引导该卷；部分主板不枚举内置盘 FAT 分区，U 盘始终是通用路径），U 盘上的 FAT32 分区提示"安装到 U 盘"。安装只做**追加复制**：目标上已有文件一律不覆盖、不删除，同名异内容文件会让整个目标被拒绝并逐个列出（除非盘根 `nvuu-stick.json` 表明这是本项目先前安装的布局，才允许原地升级）；绝不创建/调整/格式化分区，绝不写系统 ESP，绝不添加 NVRAM 启动项。两个探测都没有结果时，脚本会给出两条路线的准备指引（内置盘划一块非 ESP 数据 FAT32 分区 / 一个有足够大 FAT32 分区的 U 盘）。脚本化场景用 `-NoInstallPrompt` / `--no-install-prompt` 跳过询问，或 `-InstallTo D:\` / `--install-to <挂载点>` 指定已存在的目标（同一套资格校验）。

**`\tx.ini` 聚合配置（可选，实验性）**：下表的大量 marker 可以收进盘根一个 `\tx.ini`（如 `i_know_what_i_am_doing = 5`、`rebar = 11/9`、`reconnect = off`）。没有这个文件 = 与从前完全一致；**同一臂在 ini 和 marker 两处都设 = 拒绝引导**（日志列出冲突，删一边再跑）；解析/取值错误同样拒绝（GPU 零写入）。台架/消融旋钮故意不进 ini，仍是 marker-only。完整键表与语法见 [docs/tx-ini.md](tx-ini.md)（issue #290）。

**为什么放到这个路径就能被识别？** UEFI 固件对**可移动介质**有一个固定的回退引导路径——`\EFI\BOOT\BOOTX64.EFI`（x86_64）。文件在这个路径上，固件就会把它当作引导项自动列出/执行，**不需要**写入或修改任何 NVRAM 启动项——这也是本工具"无常驻引导项"纪律（[安全约束](#safety)）能成立的原因：拔掉 U 盘，机器回到原样。其他架构有各自的默认文件名（如 aarch64 是 `BOOTAA64.EFI`）：当前工具、资产与所有实测都只覆盖 x86_64；如果你在 aarch64（ARM 笔记本/服务器）上有需求，请开 feature request/PR 说明平台与固件，开发者暂时没有测试条件。

**`-Out G:\` 在 Windows PowerShell 5.1 上会报 `New-Item : The path is not of a legal form`**（盘符根以 `\` 结尾，`New-Item -Path` 不接受；[#195](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/195)）。脚本创建目录时会自动剥掉尾反斜杠；若用旧包，改传 **`-Out G:`**（不带 `\`）即可。

maker 不带参数运行时自动识别制盘机上的 NVIDIA 显卡代际，只生成对应家族；`-Family` / `--family` 可显式指定 `ad102`（40 系）、`ga102`（30 系）、`tu10x`（20 系）、`tu11x`（16 系，RE 推导实验线）、逗号并列的精确集合（如 `30,40`，混合代际机用）或 `all`（三代全家桶，一张盘服务多种卡；不含 tu11x）。多家族盘的文件是各家族清单的并集。两个脚本行为一致：`20`/`tu10x` 只产出 tu10x，`16`/`tu11x` 才产出 tu11x。

`F` 为 `ad102`、`ga102` 或 `tu10x`：

| 位置 | 文件 | 用途 |
| --- | --- | --- |
| `EFI/BOOT/` | `BOOTX64.EFI` | 启动程序，替换前备份原文件 |
| 盘根 | `booter_load_F.bin` | Booter bundle（BTL1） |
| 盘根 | `gsp_boot_F.bin`、`gsp_boot_desc_F.bin` | GSP 引导段与 v5 描述符 |
| 盘根 | `fwimage_F.bin`、`fwsignature_F.bin` | GSP-RM 镜像与原厂签名 |
| 盘根 | `v67-F-four-plm.bin` | 解锁载荷 |
| 盘根，Ada / Ampere | `scrubber_ad10x.bin` | SEC2 scrubber |
| 盘根，Turing | `sec2_bl_<tu10x|tu11x>.bin` | FWSEC 所需引导段 |
| 盘根，Turing | `gsp-gate.txt` | maker 生成的 GSP 检查记录 |
| 盘根，运行后 | `loader.log` | 阶段日志，下次运行前另存 |

移除 `v67-*.bin` 会进入回放模式：不执行解锁载荷，但仍会断开驱动、执行签名固件并可能复位 GPU。**回放不是只读检查**；只读采样使用 `cargo efi` 构建。

以下标记均放盘根，普通使用不需要创建：

| 标记文件 | 行为 |
| --- | --- |
| `tx-require-d0.txt` | **已退役**（2026-11-25 大扫除，[#298](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/298)）：强制 D0+读回确认本就是无条件默认行为，直接中止臂连同函数删除（#296）；老盘上此 marker 无任何效果，见 [tx-ini 退役节](tx-ini.md) |
| `tx-select-bdf.txt` | 内容为 `bb:dd.f` 或 `dddd:bb:dd.f`，仅处理匹配的卡；解析失败或目标缺失时不选其他卡 |
| `input-reset.txt` | CONFIRM 前的可选"按设备"USB 端口复位：每行 `usb VVVV:PPPP`（十六进制 VID:PID，`#` 起注释）。列出的设备在位时调用一次 UsbPortReset 并等 1.5s 重枚举——对付固件 USB 栈一起来就持续上报幽灵键的设备（带键盘接口的 USB 鼠标/接收器）；端口复位对部分设备无效（已实测），此时由 INPUT-SETTLE 的幽灵键抑制兜底。伴随每个 USB 接口一行 `USB-INV` 枚举，用于定位是谁在报键 |
| `tx-ignore-key.txt` | 幽灵键过滤:每行一个键,`#` 起注释,写法与 ABORT 日志行一致(`Special(LEFT)`、`Printable('z')`),或 `0x1A` / `scan 0x104`。列出的键在所有按键等待中视为未按下(CONFIRM、武装倒计时、链式加载选择器)。**Esc 永远不能被忽略。** 最多 8 项;无法解析的行会记录并跳过。卡键(EC 合成键、带键盘接口的 USB 鼠标/接收器)的免重编译方案:从自己的 `loader.log` 读出键值写入此文件 |
| `chain-target.txt` | 第一个非空、非 `#` 行指定优先 EFI 路径，逐卷查找；找不到时记录日志并回退默认顺序。**固件 BootOrder 里有跨盘残留引导项时先看这条**：默认解析会按 BootOrder 取到别的盘上的旧 loader（[#204](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/204) 实例是另一块盘的 `\EFI\ubuntu\shimx64.efi`，事务全绿但链到 Linux），首行写 `\EFI\Microsoft\Boot\bootmgfw.efi` 即可钉死本机链载目标。制盘菜单可代写（Windows / Ubuntu / systemd-boot / 发行版 GRUB 与 shim / 自定义路径，默认不钉） |
| `tx-preload.txt` | 启用预驱动预载链回（P2-7，TOTORONG 工艺）：事务前 LoadImage OS loader，交棒尾 PE 探针通过后直接 StartImage。**默认自 2026-11-09 起不预载**，走事务后固件 BootOrder 解析的旧链载路径——[#198](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/198) 九格矩阵：预载+脏卡与每一轮交棒后 logo 硬挂死逐一对应，而预载+干净卡（readonly）与无预载+脏卡（v2.1.5 run-10，无此特性）都活，默认回到久经田野的基线；预载原有的"FLR 后分配面退化时链载失败"鲁棒性动机已由固件 BootOrder 解析链（#128/#142）覆盖。需复现旧行为做 A/B 时加此标记；旧 `\tx-no-preload.txt` 退役，存在不再有任何效果 |
| `tx-no-reconnect.txt` / tx.ini `reconnect = off` | 跳过目标 GPU 的重连（native-GOP 重建），暗交棒——Windows Boot Manager 自建控制台。适用于事务后重建阶段挂死/复位（[#52](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/52)/[#35](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/35)/[#61](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/61) 家族）。**默认 on，但 rebar 臂激活时自动默认 off**（rebar 对写+任何 GOP 重连=已证 Code 12 族，pci 在新扩 BAR 上重平衡；显式 `reconnect = on` 可覆盖）。旧 legacy 递归重连路径及其 marker 三件套（`tx-reconnect-legacy.txt`、`tx-reconnect-nonrecursive.txt`、`no-reconnect.txt`）已于 2026-11-25 整体退役 |
| `i_know_what_i_am_doing.txt` | N 秒倒计时后自动运行，任意键中止；首行数字（0–30）可覆盖秒数，缺省 5。**0 = 不等待**：只读预备（枚举/选卡/资产预读进内存）一完成立即 PROCEED，中止窗=预备窗本身（预备期间按键仍中止）；自 v2.1.12-rc1 起 0 合法（此前钳到 1）。armed 路径在倒计时窗口内完成只读预备，自动运行从内存起跑 |
| `tx-flush-lines.txt` | 首行数字（0–4096）覆盖日志计数 flush 频率（缺省 16 行/次；0=仅阶段标记行 flush）。调大减少 USB 偶发慢写停顿（2026-11-01 TU106 实测 4 次 ~1.2s 停顿全与 flush 对齐），代价是挂死时丢更多日志尾巴；阶段标记行始终 flush |
| `tx-quiet.txt` | 安静模式（issue #38）：console 只显示裁决行（banner/确认/每卡目标与结果/HANDOFF/FINAL/CHAIN），`loader.log` 仍逐行全量记录；log 文件打不开时自动回退全量 console。慢 iGPU（~1 行/秒刷新）机器上把分钟级回显压到秒级；与 `i_know_what_i_am_doing.txt` 组合即全自动无人值守快速跑。事务期间画面停在最后一行属正常，不是卡死 |
| `gop-disconnect-all.txt`（2026-11-25 自 `tx-gop-disconnect.txt` 改名——原名没说 ALL） | 诊断开关：断开**所有** GOP 句柄（含 iGPU）而非仅目标卡。**不要当提速手段用**——console 静默后无增益，且他卡 console 被杀后 loader 不会恢复它（依赖 bootmgfw 重建，存在 Code 12 风险形态，见 issue #35） |
| `tx-native-gop.txt`（tx 构建） | **可选 pin**：native-GOP 已是默认行为，此 marker 不再是启用开关；首行写卷上 GOP 模块路径可指定用该模块替代本卡影子 ROM（兼容包形态），空/缺省=影子 ROM |
| GOP 句柄序重排（**默认开启**，tx 构建，issue #228） | native-GOP 重建后 console 卡的 child 是**最新创建**的 GOP 句柄（排队尾），而 grub gfxterm / Linux efistub / bootmgr 都取**第一个** GOP（sink 盲）——多卡+无头 BMC 类拓扑上句柄序翻转后启动图形画到无头头上（#228 C621E 实证：grub/plymouth 全盲、Windows 登录才被驱动拉回）。console 重建成功后自动把 EDID-less 竞争者控制器驱动停/启一轮（新 child 追加队尾）让 console child 居首，竞争者功能保留。**四道门全绿才写**：verified child 枚举在位 + 带 EDID（sink 证明）+ 翻转在位 + 竞争者无 EDID 且非 console 卡自身；任一失败=响亮日志+零写入，术后读回。健康机负控实证：console 已居首=Noop 零写入。**范围=in-session 链**（每靴重放解锁→交棒→OS）；暖启动句柄序由固件重建自决 |
| `tx-gop-no-reorder.txt`（tx 构建） | **退出阀**：跳过默认的 GOP 句柄序重排（GOP-ORDER dump 仍落日志）；仅当默认行为在你的板上有异常时使用 |
| `tx-gop-hide.txt`（tx 构建） | **手动兜底**：每行一个 BDF（`bb:dd.f` 或 `dddd:bb:dd.f`）或 vendor:device（`vvvv:dddd`），对应 GOP child 直接 Disconnect（不重建）。拒绝条件：清单命中 console 卡自身（兄弟头规则，**整单拒执行**）、无可解析行（拒绝猜）、术后读回失败响亮上报。与默认重排互斥：hide 先执行 |
| `tx-prom-sha256.txt`（tx 构建） | **可选身份门**：64 位十六进制，钉住 `BAR0+0x300000` 固定 1 MiB PROM 窗口的 SHA-256。缺省=未配置，事务照常进行；存在=必须匹配（两次稳定读取）才会调用 `run_transaction`，缺失/格式错/读不稳/不匹配都在任何写入之前拒绝，判定与资产缺失同级。登记方法：此文件存在时所有构建（含只读构建）都会把实测 `IDENTITY:` SHA-256 打进日志，默认引导不产生 PROM 读取开销——先放一个占位值启动一次，把日志里的实测值抄回文件即可 |
| `tx-no-canary.txt` | 整组跳过复位金丝雀的读写（A/B 对照；见[回锁章](#relock)） |
| `tx-relock.txt`（tx 构建） | **退出阀**：交棒前把 FEAT PLM 锁回 `0xffffff8f`（三代通用）。默认自 2026-11-08 起三代都门开交棒（Turing 加入 Ampere/Ada 的开门形态；TU106 bench 端到端坐实门开无害且系统内写入面开放，见 `docs/evidence/tu106-1f07/keep-open-2026-11-08/`）。Turing 臂实测回锁值 `0xffffffef`/`0xffffff8f` 随板方差；Ampere/Ada 臂（FLR 判定后重锁）未上过 bench。**注意重锁本身有代价**：部分驱动会对重锁态发起 init 期 PF FLR 并清零 SS（[#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99) 同卡 A/B），所以 v2.1.11 及更早的默认行为（Turing 臂专属；GA10x/Ada 交棒一直门开）在那些驱动上会丢锁 |
| `tx-ultrafast.txt`（tx 构建） | **速度档**（已验证机器用）：跳过全部纯诊断性质的事前/事后探测——M1 冷态采样、GOP 探测、AUTH-SCAN、ROOT-DIR 转储、opener/拓扑诊断、载荷后 FEAT sweep、HANDOFF-PCI / HANDOFF-VGA-FINAL 转储、POST-GOP 探针、CHAIN-MEMMAP——并折叠 quiet + **仅事务锚点 flush**（比阶段标记集紧得多：~80 次/轮 → ~15 次/轮；挂死时锚点之间的日志盲区更大，显式 `tx-flush-lines.txt` 仍优先）。**裁决链读回一个不少**：selector 写读回、WPR2/POST-BOOT 判定、booter 载入审计照跑——读回是唯一成功判据。**复位金丝雀不省**（写读回毫秒级、复位取证价值高；自 2026-11-09 起 ultrafast 照跑，`HANDOFF-OUT` 显示真实判定；确要跳过用 `tx-no-canary.txt`）。被跳过的阶段在日志里各留一行 `ultrafast - skipped`。只建议在已多轮验证的机器上使用（制盘菜单可代写，默认 N） |
| `tx-no-log.txt`（tx 构建） | 不开 `\loader.log`（全程仅 console 输出；quiet 因"console 是唯一记录"守卫结构性失效，console 保持完整）。挂死即零取证，只建议与 armed、已多轮验证的机器同用 |
| `tx-no-console.txt`（tx 构建） | 完全不回显（慢速 GOP 文本输出的回显成本归零，issue #38 的分钟级回显形态直接消失）。确认提示不可见——**必须与 armed 同用**（键盘语义不变：armed 倒计时内任意键仍中止）；与 `tx-no-log.txt` 同置即全程零输出的完全静默跑，挂死时屏与盘两处都无证据 |
| staging 自动重试（**默认行为**，v2.1.12-rc1 起） | **booter 侧 STAGING-FALLBACK**：Turing 的 HIGH 默认下若 booter `0x89` 且宿主 meta magic 完好（=写侧落盘、读侧未送达，[#74](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/74)/[#156](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/156)/[#191](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/191) 类），自动单次低位重试——无 FLR、单发无循环；`0x91`（[#16](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/16) 反向 sub-4GB 读锁类）与宿主 magic 损坏不触发，回退也失败则走原失败路径并提示查 VT-d。带要求按机器双向分化，判例表见[兼容矩阵](compatibility.md)。**FWSEC 侧同形自动回退（FWSEC-FALLBACK，2026-11-22）**：主链 FWSEC 点火死于 A 类指纹（WPR2-never + 空窗残影 `0x1ffffe00/0` + `CPUCTL=0` + EXCI 低 16 位 `0x3ff`——GSP Falcon FBIF→sysmem 孔径失效，[#89](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/89)/[#275](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/275) 双机）时，免 marker 单次函数级 FLR + BME 重臂 + VRAM staging 重点火；重试再死走原失败路径。健康台架无法真机复现该孔径失效——触发器真机验证依赖两台无 marker 复测（sim t41–t43 覆盖门控逻辑）。staging 手动 marker 三件套（`tx-low-staging.txt`/`tx-fwsec-high-staging.txt`/`tx-fwsec-vram-staging.txt`）已于 2026-11-25 全部退役（零探测，盘上惰性）——自动梯覆盖其全部失败类；VRAM 预设的唯一收益（省一次失败尝试 + 一次函数级 FLR）发生在 GOP 已断、屏幕已黑的窗口内，操作者无感知 |
| `tx-acpi-dump.txt` | 只读：把固件 DSDT 与全部 SSDT 连同关键词索引落到卷根 `\acpi\`（离线 `iasl -d` 反编译；回答"谁在管这张卡的电"）。确认键之前执行，没有 N 卡也生效 |
| `tx-exit-fw.txt` | **退出阀**（[PR #209](https://github.com/physicsdolphin/nv-uefi-unlocker/pull/209)，issue #208 A 线）：跳过链式 `bootmgfw`，事务完成后直接交回固件引导管理器、由固件按 BootOrder 原生引导 Windows。适用于链载 `bootmgfw` 报 `\BCD` 0xc0000034 的机器（MSI GP66 11UG/AMI 实证：路径/卷/映像字节全精确仍死=bootmgfw 内部启动上下文差异，只能让固件自己拉）。**前提**：固件持久 BootOrder 里 Windows Boot Manager 须排在 U 盘之前，否则循环回 U 盘；一次性启动菜单进入则正常落穿。中止路径（30 秒超时/按错键）同样过此检查，marker 存在时中止也交回固件——链路坏掉的机器上这反而是更好行为（链载会落到 Recovery 屏）。marker 同置时事务前预载自动跳过 |
| `tx-revive-dead-slot.txt`（tx 构建） | **死槽复活阶梯**（[#276](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/276)，现场验证 2026-11-23）：对枚举读 vendor 0xffff 的死槽执行——ROM 身份门（固件缓存影子的 PCIR 读 vendor/device，只对 N 卡执行并打印死槽身份；无 ROM 的设备一律 decline）→ 杠杆阶梯 PMCSR→D0 / 清 Link Disable / **总线号恢复**（重编号族）/ **Secondary Bus Reset**（每级读回验证，复活只认 vendor 读回）。死槽自有 console GOP 在任何复位前自动断联（GOP 绑定态 SBR=黑屏卡死，台架实证）。复活后从固件簿记（GetBarAttributes）恢复 BAR，**复活槽保留候选表、同 boot 交易**。无 ROM 的死槽（桥类）会被身份门 decline |
| `tx-revive-force-sbr.txt`（tx 构建） | **FORCE-SBR 强推档**：对「悖论端口」（端口全绿但子端 0xffff、总线号一致）绕过保守 decline 强制 SBR。台架三连阴性教训在档：软死桥扛过 SBR（链路重训、config 仍死）——对软死设备无效，对真断电/链路关闭设备才有效 |
| `tx-sbr-gpu.txt` | **已退役**（2026-11-25 大扫除，[#298](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/298)）：SBR 内建进所有 rebar 臂——XVE 页 reset-gated 是硬前提不是旋钮，纯解锁 boot 的事务内 SBR 实验臂同轮退役（#296）；老盘上此 marker 无任何效果，见 [tx-ini 退役节](tx-ini.md) |
| PORT-DIAG（无条件） | 无 marker：枚举时每个 PCI-PCI 桥/根口一行寄存器快照（PMCSR/LnkCap/LnkCtl/LnkSta/Bridge Control + **当前**总线号），死功能行（vendor 0xffff）无条件记录。判别「链路真关闭（D3hot/LinkDisable/L2L3-Ready）vs 总线号隐藏（DLLLA=1 + 当前 bus ≠ 句柄创建 bus）」两家族——[#276](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/276) 现场即由它裁决 |

无自动运行标记时，事务只接受空格或回车确认，其他键或 30 秒超时均中止。链回顺序为 Windows Boot Manager → 受支持的 Linux shim/grub/systemd-boot/Limine → 通用 `EFI/BOOT/BOOTX64.EFI`；最后一档提供 5 秒按键退出窗口。慢速 console 的提速走 `tx-quiet.txt` 的 console 白名单（issue #38），不做异步输出——UEFI 引导服务单线程，GOP 文本渲染本身是同步调用，异步提不了吞吐；每行日志带 `[+ms]` 阶段计时前缀，回显耗时与事务等待在 `loader.log` 里直接可分。

多卡默认按固件枚举顺序处理所有受支持的卡；已解锁或不支持的卡跳过，单卡失败按失败路径处理，不重试该卡。只想解锁其中一张时必须明确指定 BDF。

<a id="auto-run"></a>

### 不想在 30 秒里按键，怎样全自动跑？

在盘根放一个 `i_know_what_i_am_doing.txt` 即可——**文件放不放，就是"做盘时决定要不要确认"的那个选项**：

```
U盘根\
  i_know_what_i_am_doing.txt   <- 内容写 0
  tx-quiet.txt                 <- 可选，安静模式
```

这个选择也可以由制盘脚本代写（[#259](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/259)）：`make-usbroot.ps1` / `make-usbroot.sh` 在构建结束时会问一次确认闸门——**默认（直接回车）保留 30 秒按键**，选 `1` 即写入本 marker（追问 0–30 秒，回车=5）。同一菜单还问三件：`tx-quiet.txt` 安静模式（y/N）、`chain-target.txt` 链载钉定（`0`=Windows Boot Manager / `1`=GRUB 追问发行版 / `2`=systemd-boot / `3`=Ubuntu / `4`=发行版 shim / `5`=自定义路径，默认不钉——[issue #204](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/204) 跨盘残留 BootOrder 的修复）、`tx-ultrafast.txt` 加速档（y/N，**默认 N——只建议给已多轮验证的机器**，首轮排障靠的就是它跳过的诊断阶段）。脚本化跑用 `-AutoRun 0..30|off`（bash `--auto-run`）、`-Quiet`/`--quiet`、`-Ultrafast`/`--ultrafast`、`-ChainTarget windows|ubuntu|systemd|路径|off`（bash `--chain-target`）显式指定，`-NoMarkerPrompt` / `--no-marker-prompt` 跳过提问。本次写入了哪些 marker 记录在 `nvuu-stick.json` 的 `markers` 字段。菜单只收"做盘人无需症状证据就能替目标机决定"的项（默认值都是文档基线），debug/诊断类 marker 仍是按症状查 FAQ 的诊断/退出阀。

- 文件存在 = 不再等按键，改为 N 秒倒计时后自动执行；倒计时内任意键仍可中止。
- 内容留空默认 5 秒；首行数字改秒数，有效范围 0–30。
- **内容写 `0` = 完全不等待**：只读预备（枚举/选卡/资产预读进内存）一结束立即开跑。`0` 需要 **v2.1.12-rc1 或更新**，v2.1.11 及更早会钳到 1 秒。
- 看到 `CONFIRM: \i_know_what_i_am_doing.txt present - ARMED: Ns countdown auto-runs` 就是已识别。

为什么默认仍然是按键：那 30 秒是**零写安全闸门**——超时或按错键直接退出，一根寄存器都不写（日志明写 `CONFIRM: 30s timeout - ABORT`）。从"每次开机按一下键"到"一次确认、永久自动"的正确切法，是把意图声明提前到做盘那一刻（marker），而不是把闸门整体拆掉。所以**看到 30 秒超时 ABORT 不是失败，是闸门按设计工作**——进系统读回 `NOT_UNLOCKED` 也别误判成工具坏了。

倒计时被莫名打断（屏幕出现 `CONFIRM: key Printable(...)`）是固件在上报幽灵键，用 `tx-ignore-key.txt` 过滤（见上文表格与[幽灵按键](#ghost-key)一节）。

### 能不能装到内置硬盘分区、开机自动解锁？

可以，但属于**专业用户自担操作**：本项目交付物保持 U 盘/一次性启动菜单形态，不提供、也不建议普通用户改内置分区引导。技术事实与边界：

- `bcdedit /create /application osloader` 造出的 Windows Boot Loader 条目 `device` 落在盘符上、重启即失效，固件不枚举；`bootsector` 类型则 `device` 字段静默为空——**两条路都进不了 `{fwbootmgr}`**，别在这上面花时间（[Discussion #104](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/104) 双 4080 SUPER 实测记录）。
- 可行形态（#104 实证）：数据卷缩出 ~2 GB FAT32 分区，按 U 盘同款布局放置 EFI+资产，BIOS 启动菜单里将其（显示为 `UEFI OS`）手动调前或 F12 一次性选择；可随时从 BIOS 菜单改回 Windows Boot Manager，不刷任何东西。
- 系统 ESP 内嵌变体（[D#249](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/249) 实测，GA102/300 MB ESP）：`mountvol S: /S` 挂 ESP → 资产平铺 `S:\` 根、loader 放 `\EFI\NVUNLOCK\BOOTX64.EFI` → 写 `S:\chain-target.txt` 为 `\EFI\Microsoft\Boot\bootmgfw.efi` → `bcdedit /copy {bootmgr}` 造独立项、`device partition=S:`（落为 bootmgr 同卷的 `\Device\HarddiskVolume1`）、`path` 指 loader、`/addfirst` 置顶；原 `\EFI\Boot\bootx64.efi` 不动。**容量**：ga102 整包约 162 MB，300 MB ESP 装完余 103 MB（#29 的 100 MB ESP 装不下 80.7 MiB 载荷）。**两个运维坑**：固件/Windows 更新重排 BootOrder 会静默失效，重跑 `displayorder` 即恢复；不放 `i_know_what_i_am_doing.txt` 时无人确认就是 30 秒零写 ABORT、进系统读回 `NOT_UNLOCKED`，别误判成"装了没用"。
- 为什么不提供自动化脚本：多数用户的认知是"引导分区炸了=工具的锅"；缩容场景千差万别（双系统/跨盘引导/btrfs、zfs 等不支持在线缩容的文件系统），工具面强行覆盖全部存储拓扑有真实的数据安全风险。**策略：教程可查（如上）、自动化不做、允许下游 fork 提供自动化脚本并互链。**

异机制盘先显式指定 `-Family`（或用 `-Family all`）；**目标机同机有多个代际时必须把代际逗号并列**（如 `-Family 30,40`）或直接 `-Family all`，单代盘会让其余代的卡在启动时被跳过（`loader.log` 有专门一行点名缺哪一族并给重建命令）。Turing 检查不适用时可用 `-SkipGspCheck` / `--skip-gsp-check` 跳过制盘机检查，但目标机仍须满足 GSP 前置。离线 Turing 引导归档 `gsp_rm_boot_dbg.bin` 用 `-ProvenBlob` / `--proven-blob` 指定；长度和 hash 校验仍有效。下载或校验失败时不要换版本或绕过检查。Windows PowerShell TLS 下载失败时可用 WSL / Git Bash 运行 bash maker。

**中国大陆网络与下载源**（2026-11-08 更新）：`raw.githubusercontent.com` 被阻断，Turing 引导归档的抓取会自动依次回退到 jsDelivr CDN 与代理传输（同一对象，钉定 SHA-256 逐字节把关）。`nvidia-utils` 输入自 2026-11-08 起是**源组自动回退**（[#212](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/212)：CachyOS 归档是每包约三版的滚动窗口，钉定的 610.43.03 已被挤出窗口 404）：**Arch 官方归档（永久，版本不清除）→ 第一方 NVIDIA `.run` 安装器（XFree86 树只增不删，us/tw/jp 多区域节点同字节）→ CachyOS 旧容器（仅作字节缓存还魂通道）**，三者内含的 GSP 固件逐位一致——内容寻址闸只认内层 ELF 的钉定 SHA-256。`-UtilsSource arch|run|cachyos`（bash 版 `--utils-source`）可钉定单一路线；全部失败时报告会列出每个源、其 URL 与钉定哈希。任何钉定文件都可以离线获得后直接放进 `inputs/`（哈希匹配即接受）——这是官方认可的离线路径。某传输源失效时不要换**版本**（钉定哈希按容器字节算，610.57/615.x 容器字节不同即不命中），换**源**或离线投放皆可。**但解锁本身对 GSP 版本不挑**——按世代限定：图灵谱系（`gsp_tu10x`）525–621 全跨度（16.7–37.1 MB）已在 TU106 台架实测全链解锁；**各代 GSP 只用于本代卡**（40 系/AD10x 走 `gsp_ga10x.bin` 的 `fwsignature_ad10x` 段，其兼容下限=40 系起家的最早 GSP）。GSP 只是砸开 FEAT 门的工牌，机制与边界详见 [docs/gsp-compatibility.md](gsp-compatibility.md)（英文版 [gsp-compatibility.en.md](gsp-compatibility.en.md)）。

<a id="turing"></a>

## Turing 为什么要先启用 GSP？

Turing 的 Windows 驱动需要 GSP-RM 模式才能正常接管解锁状态。legacy 初始化可能导致 TDR / Code 43。管理员 PowerShell 中执行与安装方式对应的一条命令：

```powershell
.\enable-turing-unlock.ps1
```

源码树中的脚本为 `tools\windows\enable-turing-unlock.ps1`。脚本仅为 Turing 显示设备设置 `EnableGpuFirmware=1`；加 `-Remove` 可撤销。**必须重启**，休眠恢复不会重新初始化驱动。

maker 会检查注册表开关和 `nvidia-smi -q` 中的 GSP 固件运行态：不满足时给出醒目警告但不阻塞制盘——**目标机**的 GSP 模式才是关键，制盘机状态不构成门槛。制盘机侧的卡检测先读驱动的绑定设备列表（`Services\nvlddmkm\Enum`），维修平台换卡残留的显示类注册表键不会被误认成本机在位卡，被剔除的设备 ID 会以 ghost 警告逐条列出。Linux Turing 同样需要运行 GSP-RM；专有驱动 545+ 默认使用该模式，但可能被 `NVreg_EnableGpuFirmware=0` 关闭，仍以运行态检查为准，不运行 Windows 注册表脚本。30 / 40 系无需这项 Windows 注册表前置。

Turing 的 `gsp_boot_tu10x.bin` 是 proven GspRmBoot 归档的前 `0x6000` 字节（24576 B）。maker 对输入归档与输出切片分别校验，不可用另一种引导镜像代替。

<a id="display"></a>

## 混合输出、重启和休眠有什么区别？

优先使用独显直连，并从系统选择“重启”进入 U 盘；关闭快速启动，避免“关机”实际走休眠路径。

| 入口状态 | 处理方式 |
| --- | --- |
| 独显直连，GPU 有电且完成初始化 | 可以运行，进入系统后仍需验证 |
| 混合输出，dGPU 有电且寄存器可读 | 可以尝试，核对交棒前后状态 |
| dGPU 处于 D3cold，BAR0 全 `0xffffffff` | 改用能给 dGPU 上电并初始化的启动路径 |
| 休眠恢复或 D3 门控入口 | 优先重启；不要仅凭 EFI 成功判断系统内仍有效 |

固件可能只初始化负责显示的 GPU；本工具不能通过 ACPI `_PS0` / PR3 给完全断电的 dGPU 上电。非 D0 入口会触发 PM 预检，请求 D0 并读回确认——这就是现场分辨（原 `tx-require-d0.txt` 直接中止臂已随 2026-11-25 大扫除退役，marker 无任何效果）。

**混合输出"无收益"族的机理与实证绕法（#171→[#224](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/224)，2026-11-10）**：神舟 TX9 3070M（`249d`，混合输出/非独显直连）曾"进系统无解锁、算力无提升"（[#171](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/171)，已关）。**机理**：混合模式下进系统后厂商电源管理把 dGPU 置入 ACPI D3（甲族 AON 复位/断电路径），易失的解锁态随之被抹——另有独立用户同形印证（"混合模式把解锁状态抹了，改独显直连就好了"，B 站评论，弱证据仅作旁证）。**实证绕法 = HDMI 欺诈器（dummy plug）+ 系统内"复制显示器"模式**：让平台把 dGPU 当作"有显示活动"从而不停电；该法由报告者在维护者的 D3 诊断下自行猜想并本机验证成功（armed 全链绿 + ssdiag `UNLOCKED (override values live)`）。**硬前提：至少一个视频口原生从独显出线**——全部视频口都从 iGPU 出线的机器用不了这招，只能改独显直连（BIOS/MUX 有该项时）；两样都没有则此路不通。欺诈器本身是一次性小开销（十几元档，通用配件），对混合本用户完全可行。该记录无性能 A/B 与睡眠持久性数据；另注：3070M（Ampere）无需 Turing 的 GSP 注册表前置，开了也无害。**固件侧替代（2026-11-16 起实证）**：固件（含隐藏项）提供 ACPI D3 开关的机器，直接全局关闭即可从根上去掉这条断电路径——G614JV 实测解锁 100% 保留（[丢锁一节实例记录](#relock-acpid3)）；零独显直出、欺诈器用不上的机器优先找这个开关。

**笔记本内屏与亮度（DDC-CI）**：部分笔记本上，旧版固件重连会在解锁后把内屏链路留在未训练状态——背光直接黑、DDC-CI 亮度控制失灵（一台拯救者 R7000 2024 实证；旧路径首次尝试即复现，v2.1.10 在同一台机上修复）。v2.1.10 起默认的 native-GOP 重建会重新 `StartImage` 本卡原厂 GOP 驱动，进系统前重跑一次链路训练，自动消除该故障。致病的旧 legacy 重连路径（`tx-reconnect-legacy.txt`）已于 2026-11-25 退役，此类机器上复现该症状请改用 `reconnect = off` 暗交棒。

TU106 + GSP 模式有休眠恢复后成功交棒的实测，但这不能代替其他机器的系统侧检查。详见 [系统内验证](#ssdiag)。

<a id="linux"></a>

## Linux 日志全绿，为什么没有提升？

先核对系统侧选择器和电源状态。RTD3 让空闲 GPU 进入 D3cold 时会清除易失解锁；EFI 读回成功不能证明状态在系统内一直保留。

**系统内寄存器探针的内核前提（2026-11-23，[#246](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/246) 实证）**：`tools/diag-unlock.py` 经 sysfs `resource0` mmap 直读 BAR0——在 `CONFIG_IO_STRICT_DEVMEM=y` 的内核（Arch/Manjaro 式配置；Ubuntu generic 未启用）上，被驱动占用的 BAR 一律拒绝映射（EINVAL，root 亦然；机制=pci-sysfs 的 `iomem_is_exclusive` 检查，读写同生死）。判据一行：`grep -E "CONFIG_(IO_)?STRICT_DEVMEM" /boot/config-$(uname -r)`；解法=内核命令行加 `iomem=relaxed`（**无运行时开关**——该变量仅由 `iomem=` 引导参数设置，root 不可热改；读写同开、含 /dev/mem）。工具在 mmap EINVAL 时会打印本提示。

若发行版在 `/usr/lib/modprobe.d/nvidia-runtimepm.conf` 启用 `NVreg_DynamicPowerManagement=0x02`，可在 `/etc/modprobe.d/` 使用同名文件覆盖。先备份已有配置：

**一键脚本**：`sudo bash tools/disable-rtd3.sh`——实现本节全部步骤（同名覆盖+备份、设备级 `power/control=on` 立即生效、initramfs 工具探测、冲突文件点名、合并配置终检）；`--check` 只读体检当前状态。手工步骤如下：

```bash
echo 'options nvidia NVreg_DynamicPowerManagement=0x00' | \
    sudo tee /etc/modprobe.d/nvidia-runtimepm.conf
sudo update-initramfs -u
```

`update-initramfs` 用于 Debian/Ubuntu；Fedora/RHEL 使用 `sudo dracut -f`。其他发行版按其配置与 initramfs 工具处理。不同名文件的 options 会累加，需排除冲突项。

重启后检查生效值，而不只查看配置文件：

| 检查 | 命令 | 预期 |
| --- | --- | --- |
| 驱动参数 | `grep DynamicPowerManagement /proc/driver/nvidia/params` | 0 / `0x00` |
| 合并配置 | `modprobe -c \| grep -i DynamicPowerManagement` | 仅一项 `0x00` |
| GPU 电源 | `cat /sys/bus/pci/devices/<bdf>/power_state` | 实际目标 BDF 为 `D0` |
| 解锁状态 | `sudo python3 tools/diag-unlock.py`（源码树） | SS 覆盖值有效 |

掉电清除的状态需要重跑 U 盘事务；只重写配置不会恢复它。来源：[RTD3 平台记录](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/9)。完整机制盘点、判据与可做的事见 [为什么解锁过的卡一进系统又被锁回去](#relock)。

**RTD3 关了还是没提升？（驱动 init 重推导，[#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)）**：GA10x 笔记本上存在第三种丢失机制——专有驱动在 `modprobe` 初始化时**主动重推导 FEAT 块**，与空闲掉电无关。实测（GA107M `25a0`，Linux Mint / 595.71.05）：RTD3 已设 `0x00`、GSP-RM 与 legacy 两种 init 模式**都**重推导且写入不同的位（SS0 相对出厂 delta 分别 `0x00400100` / `0x00400000`）——是主动编程，不是"没保留"；事务内 FLR 后 AON 存活也排除了 FLR 冲掉。这意味着在重推导型机器上，EFI 读回绿 ≠ 系统内有收益：**EFI 读回证明的是写入发生，不是它活过 `modprobe nvidia`**。Linux 侧现状：持久存活有实证的有 TU102 桌面（[#103](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/103)）与 GA102 桌面（2026-11-21 维护者台面：GSP 620 驱动加载后保留、多轮驱动级复位恢复不掉，见[机制盘点](#relock)节）；GA10x 笔记本方面，目前仅 `25a0` 一例 Linux 观测到 init 重推导——同族 `25ad` 在 Windows 持久存活，OS/驱动版本/MUX-EC 平台态三个变量尚未切开（详见 [#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)），不宜外推为 GA10x 笔记本普遍结论；门关后 OS 内重写主机侧无法补救（SEC2 内全链属上游 kmod 谱系，超出本工具范围）。Windows 侧同族机制见 [拓扑维度](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/82)（[#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) GA106M 同形态）。**2026-11-21 重定性（GA102 桌面对照 + 名称勘误）**：现象保留——两种 init 模式写不同位的读回表是硬数据；但"devinit 重推导"是误名：该现场**无复位**（FLR 已被 `POST-FLR` 排除），复位后 devinit 重算的时序前提不成立，写入方（主机 RM vs GSP 固件）与触发变量均未切开。同日 GA102 + 620.70.06 桌面实验里 rebind + RmInitAdapter 完整执行而 FEAT 三连原样——**重写不是 init/rebind 的固有步骤**。机制表第 3 行由此更名"init 期主动重写"（原始记录：`docs/evidence/ga102-2204-oc-reset-recovery/`）。

读回工具细节：Linux 下 `resource0` **只支持 mmap**，不支持 `read()`/`dd`——`dd` 会静默返回空，看起来像"PLM 已关"的假阴性；`tools/diag-unlock.py` 走的就是 mmap 路径。

**先看日志的 `POST-GOP` 行，别急着怪驱动**：从 v2.1.11 起交棒尾部会再读一次 FEAT 三连（`POST-GOP: FEAT_PLM=… SS0=… SS1=…`）。位置在**控制台重建之后**——默认的 native-GOP 重建会 `StartImage` 卡自己的 GOP 驱动，而此前的 `POST-FLR` 验证在它之前，所以两行合起来才能切分写入方：`POST-GOP` 已锁/已丢 = 丢在 bootmgfw 之前（卡自己的 GOP/devinit 或平台固件，Windows 驱动出局）；`POST-GOP` 完好、系统内被抹 = 驱动这一 pass 负责。这也是 [#164](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/164) 的取证入口。

<a id="gain"></a>

## 解锁成功、性能却没有提升，怎么判断还会不会有收益？

先分清两件事：**解锁是否还活着**（看 [系统内读回](#ssdiag)，门关/选择器回厂是另一回事），和**解锁活着但负载吃不到**。后者可以用一条规则判掉，不必逐个负载试：

> **收益 ≈ GEMM 收益 × 负载中受频率限制的时间占比。**

解锁抬高的是**发射率**，不是功耗上限。所以：锁定态平均功耗明显低于功耗墙、SM 未饱和的负载，解锁后会把省下的时间换成瓦数，时间随之下降；锁定态已经贴着功耗墙、SM 全程 100% 的负载，解锁只把瓦数抬上去，时间几乎不变。

一台 TB4 + AD102（RTX 4090）的同机同会话 A/B（[#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)，已关闭 completed，每次两端都过 `ssdiag`）：

| 负载 | 锁定 | 解锁 | Δ | 锁定态平均/峰值功耗 |
| --- | --- | --- | --- | --- |
| 多参考图图像编辑（双 UNet pass） | 170.4 s | 139.0 s | −18.4% | 301 / 436 W |
| 文生视频，15 s 片段 | 119 s | 89 s | −25.2% | 298 / 484 W |
| 参考图生视频（20 步） | 169 s | 169 s | 0% | 362 / 466 W |
| 单 pass 文生图 | 24.8 s | 24.1 s | ~0% | — |
| 量化 MoE LLM（prefill+decode） | — | — | 0% | — |

那三行 0% 的旁证：参考图生视频那行锁定态就已 362 W 均值、SM 100%（三次基线 169/169/169 s），解锁后 405 W 均值、时间不变；显存峰值 22.1/24.6 GB，没有溢出到系统内存，排除了换页这个混淆项。

**怎么用这条规则**：

- 先读锁定态的 `nvidia-smi` 功耗与 SM 利用率（同一次会话内）。贴着上限且 SM 饱和 → 这类负载上解锁注定看不出来，不需要再调工具。
- 明显低于上限 → 时间应该下降；若没下降，才回头查门是否已被关（[被锁回去](#relock) / [睡醒与 RTD3](#sleep)）。
- 笔记本尤其注意：整机 TGP 共享时，解锁后的额外发射率会被功耗墙吃掉，详见 [Linux 无提升一节](#linux) 的共享 TGP 实测。

还有第三种情形，判据之外也值得知道：**解锁活着、负载也 compute-bound，但端到端 GEMM 数字仍明显低于裸管线口径**——CUDA 库的 kernel 自动选择按出厂半速硬件校准（完全静态的启发式表，感知不到解锁态），会给解锁后的卡选中偏小 tile 的次优 kernel（8×RTX 4090 实测：默认与库内最优差 1.2–1.9×，FP8 最甚）。这不影响[读回判定](#ssdiag)——硅态判据与应用侧读数是两回事；应用侧想回收这部分，可按形状实测遍历候选择优（autotune）或手写 CUTLASS，具体方法与各精度推荐配置见 [Harry-Chen/unlocked-ada-gemm](https://github.com/Harry-Chen/unlocked-ada-gemm) 与[原始博客](https://harrychen.xyz/2026/11/17/unlocked-rtx-4090-gemm-perf/)。

**证据等级**：上表是一台桌面机（TB4 坞、驱动与场景见 #54）的结果，规则与其中的功耗判据一致；不同架构（如 Turing）的"满速"基准不同（`READOUT` 语义见 [读回判定](#ssdiag)），不要把这里的倍率直接套到别的卡上。

**一个可复算的分解（[#281](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/281)，2026-11-22）**：ComfyUI SDXL+LoRA 同机同种子，把四个工作负载的端到端比值代回 `r = 1/[(1−f)+f/1.86]`，解出**计算占比 44.6%（img2img 976×1248）/49.1%/52.3%/80.8%（txt2img+hires）**，以及**每图固定开销 12.8–15.3 s**（VAE、文本编码、搬运，基本与分辨率无关）。这解释了轻负载为何封顶 ~1.2–1.3×、重负载才接近 GEMM 上限；同批 9 对图像逐像素相同（max\|d\|=0）说明只改速度不改结果。**16 GB 主机的 offload 陷阱**也一并记下：ComfyUI 动态显存默认把模型放系统内存，内存紧张时每个提示重读 ~5 GB——340/750/793 s 单图（采样本身仍只有 40–52 s，750 s 一例卡在 VAE decode 11 分钟），强制 `--highvram` 回到稳定 46 s。这是最容易误判"解锁没生效"的形状。

<a id="relock"></a>

## 为什么解锁过的卡一进系统又被锁回去？

先说结论：**不是 NVIDIA 在检测我们**。解锁被抹掉是三条各自独立的机制叠加的结果——物理层的复位语义、固件的初始化契约、以及 FEAT 块本身的性质。绝大多数"进系统就丢"的报告属于**平台给了驱动一个复位或断电事件**，跟驱动版本几乎无关。issue 是自选样本、没有分母，所以下文的比例只能当量级看，不能当概率。

### 先认识 FEAT 块是什么

`FEAT_PLM`（`0x823804`）和 `SS0/SS1`（`0x82381c` / `0x823820`）在 NVIDIA 自己的寄存器头里叫 **fuse feature readout**——即**熔丝/strap 的软件投影层**。它们不是可持久配置：

- `0xffffff8f` 不是"某个值"，是 PLM 的**上电复位值**；
- `SS0/SS1` 的**覆盖字段归零**才是复位态；其余位由 strap 决定，整寄存器是否读 0 因卡而异（`#148` 是 `0x57340065` / `0x7`）。`#164` 现场读到 `0x0`，但那是在丢失之后采的样，该卡基线无法与"被零化"区分；
- 我们写的 `0xffffffff` / `0x88888888` 是在投影层上**加覆盖位**，既没动熔丝也没动 PLL。

所以解锁被还原是**恢复出厂**，不是损坏。**这也意味着"覆盖字段归零 + 门关"本身就是一条证据**：只写门的写者不会顺手把选择器写成基线，把两者一起还原的只能是完整重算或整域复位。

`FEAT_PLM` 还活在 AON（常开岛）里——这是它能活过事务内 FLR 的原因（`POST-FLR` 验证的就是这一点），也是全部麻烦的根：**AON 只在掉电时复位，其它复位源够不着它**。

### 判据：先分类，再找原因

| 现场 | 含义 | 下一步 |
| --- | --- | --- |
| `POST-FLR` 已丢 | 事务内就被抹 | 先当作事务问题查 |
| `POST-GOP` 已丢、`POST-FLR` 完好 | 丢在 bootmgfw 之前 | 卡自己的 GOP/devinit 或平台固件；Windows 驱动出局 |
| `POST-GOP` 完好，系统内门关、`SS` 仍是覆盖值 | 窄写入，只碰门 | 见机制表第 4 行 |
| `POST-GOP` 完好，系统内门关、`SS` 变成既非基线也非覆盖值 | 重推导（驱动 init 主动编程） | 机制表第 3 行；见 [#148 记录](#relock-148) |
| `POST-GOP` 完好，系统内门关、`SS` 回到该卡开机基线（覆盖字段归零） | 整域复位（掉电 / SBR / 全芯片复位） | 机制表第 1、2 行 |

Turing 自 2026-11-08（`061aef8`，v2.1.12-rc1 起）交棒默认也是门开（与 Ampere/Ada 一致）；只有 `\tx-relock.txt` 在位时才会出现重锁值——别把 marker 臂的正常值当丢失，判读见[雷电一节](#thunderbolt)与[冷态对照](cold-values.md)。**v2.1.11 及更早的默认是交棒前重锁（Turing 臂专属——GA10x/Ada 的载荷四写链自带门开、交棒一直门开，`061aef8` 对它们是 no-op），那是当时已知的最大一条自造回锁源（机制表第 7 行，[#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99)）。**

**基线纪律：** 判「SS 被清零」必须先有一条**同一 boot、解锁之前**的锁态 `ssdiag` 采样。stock 的 `SS0/SS1` 并非恒为零（#99 同卡实测两次 stock 分别 `0x00040000`/`0x6` 与 `0x0`/`0x0`，READOUT 都是 `0x00400080`）；只有「基线 → 交棒 `POST-GOP` → 系统内」三段同 boot 可比，SS 的变化才有判别力。

**配对法：拿"复位金丝雀"当见证。** 光看 FEAT 只能说"写被抹了"，说不清是**被写**还是**整片被复位**。v2.1.11 起载具在 FLR 验证之后往 PBUS 软件 scratch（`0x1480`/`0x1484`（即索引 `0x20`/`0x21`；早于本轮构建的不算，偏移曾错写成 `0x140020/24`），open RM 的老驱动只写 `0x1e`，这两格干净；BUS 域随 FLR/SBR/撤电清零）写一对魔数并立刻读回，`POST-GOP`、`HANDOFF-OUT` 与系统内 `ssdiag`（0.6.7+）各给同一状态。判读必须与 FEAT 成对：**金丝雀完好 + FEAT 被抹 = 没复位，是写入；金丝雀清零 + FEAT 完好 = FLR/SBR 级复位且 AON 存活；两者一起没 = 撤电/全片复位。**

WPR2 是旁证：本次引导里唯一由 booter 自己写过的寄存器（暂存窗口基址），`wpr2_lo` 自 v2.1.11 起与 FEAT 同行记录，仍可与金丝雀交叉核对。

两点方法学限制（都会导致假结论，先看再跑实验）：

- **安全模式配对有两条失效路**：① MUXless 机器 dGPU 停泊时 BAR0 读不到，`ssdiag` 无从取数——安全模式只能证明"驱动没参与"，不能给出读回（Catnip 在 [#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) 指出）；② 即使 BAR0 读得到，`ssdiag` 的 pmxdrv 载具在安全模式里 `StartService` 会撞 `win32 1084`（`ERROR_NOT_SAFEBOOT_SERVICE`）——未列入 `SafeBoot\Minimal` 白名单的内核驱动在安全模式一律不可启动（#37 Victus 实测；SCM 与内核加载器两层都查这道名单）。②有现成解法且**零攻击面**：正常模式提权跑 `ssdiag --safeboot-arm` 只预置两个白名单键（检查按服务名匹配，不落 .sys 不建服务；自带临时布局验证轮），安全模式内驱动照现行模式抽出、退出即删；事后 `--safeboot-disarm` 删键。①无解，该情形转 [Linux live 读回法](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/82)。
- **`WPR2_HI` 单独不能用**：driver 自己的 FWSEC 会在下一个 boot 重新建窗，值本来就变，只有和 `wpr2_lo` 成对、且与本次交棒值逐位对照才有意义。

### 机制盘点：谁能让它回厂

| # | 机制 | AON 域 | 典型场景 | 看到的形态 |
| --- | --- | --- | --- | --- |
| 1 | **真掉电**（rail 断） | 复位 | 带 `_PR3` 的根口 + D3cold/RTD3/现代待机（笔记本常见，桌面也可有——本机 Z170 三个 PEG 全有）、EC 电源门控、外置坞断电、S4/快速启动 | 门锁 + 选择器回开机基线；`power_state` 曾为 D3cold |
| 2 | **整域复位** | 复位 | 平台 SBR、链路热复位、全芯片 SW_RESET；多见于老固件/"奇怪"主板 | 同上，且 `POST-GOP` 就已经坏 |
| 3 | **init 期主动重写**（驱动 init 自己重算；旧名"devinit 重放"是误挂——#148 现场无复位、复位后 devinit 重算的时序不成立；2026-11-21 GA102 对照：rebind + RmInitAdapter 完整执行而 FEAT 原样，非 init 固有步骤，写入方与触发变量未切开） | 存活 | 部分 GA10x 笔记本（[#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)，Linux 实测；Windows 侧尚无同形态样本） | 门锁 + `SS` 被写成推导值（≠开机基线、≠覆盖值） |
| 4 | **窄写入**（只碰门） | 存活 | 少数平台 | 门锁 + `SS` 完好（[#82](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/82) 类） |
| 5 | **我们自己的尾巴**：native-GOP 重建 `StartImage` 卡自己的 ROM | — | 某些平台的 ROM POST 会带 devinit | `POST-GOP` 坏而 `POST-FLR` 好 |
| 6 | **OS 侧连接管理器（USB4 CM）建立/重建 USB4/雷电隧道** | 复位 | 雷电/USB4 eGPU（[#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)，已关闭，桌面 TB4 + AD102、笔记本原生 USB4 + GA104 实测）：Windows `Usb4DeviceRouter`、Linux `thunderbolt` 模块绑定/接管时拆掉并重建整棵下游总线。固件 CM 的独立雷电主机（Intel 雷电面板那类）开机不走此行，但重插线时管理栈重建隧道、同样触发 | 金丝雀 `cleared`（域复位）。两 OS 都有抑制：Linux `thunderbolt.host_reset=0`、Windows `Usb4DeviceRouter` `Start=4`（均实测，见本节正文） |
| 7 | **我们自己的交棒重锁写**（v2.1.11 及更早的 **Turing 臂**默认行为；GA10x/Ada 交棒一直门开） | 复位（PF FLR） | 交棒前把 `FEAT_PLM` 锁回 `0xffffff8f`（硬件读回 `0xffffffef`）；驱动 init 再把门推回并触发 PF FLR | SS 在交棒时完好、系统内被清零；`nvlddmkm` id=14 明写 `None → PF FLR`（[#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99) 2026-11-12 同卡 A/B；`061aef8` 起默认门开，见 `docs/evidence/issue99-1f07-x99-gate-open/`） |

第 6 行的特殊性：复位由**建立/重建隧道的 CM** 执行——USB4 主机上是 OS 侧 CM，发生在任何驱动绑定之前，所以"接管之后再武装"没有主机写路径（`--reapply` 结构性无效，热插拔也一样）；固件 CM 主机开机不触发此行（见雷电节分型），重插才触发。唯一出路是抑制那次拆建，两套 OS 都已实测可行（正文给命令与代价）。取证走 chain 到 Linux + `modprobe.blacklist=nouveau,thunderbolt` + `tools/diag-unlock.py`（rEFInd 顶替 ISO 自带 GRUB、Secure Boot 可全程开着）。

第 1、2 行是物理事件（掉电/复位后 devinit 重算回厂，契约见下节），第 3 行是驱动侧的平台条件分支（2026-11-21 定性，见上行注），第 5、7 行是我们自己代码里能改的（第 7 行已在 2026-11-08 默认关闭），第 6 行是 OS 侧旋钮（不在本工具内）。

**第 1 行已有两轴同抓的完整现场对**（2026-11-04，[#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37)，HP Victus 16 `25a2` 纯混合平台，`docs/evidence/issue37-hp-victus-25a2/`）：EFI 侧金丝雀设置并读回完好、交棒时卡 D0 → 系统内金丝雀清零 + FEAT 回厂推导值 + 根口 `00:01.0` PMCSR `D3`。FLR 被同日志排除（本链 `POST-FLR` 自证 GA10x 选择器活过 FLR），事件超出 FLR 即常规复位族，根口 D3 把形态压向第 1 行。对照组：TU106 桌面金丝雀 intact + 双端 D0（写入型才会长这样）；同为 GA107M 的 [#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)（Linux）是无复位的 devinit 重写（金丝雀应完好）——同 die、不同平台、不同行。金丝雀表的第四格（`intact` + FEAT 完好）也已有首个现场实例（2026-11-05，[#109](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/109#issuecomment-6650530730)：同卡同 VBIOS 同子系统、B650 桌面，交棒与系统内均 `intact`、双端 D0、`UNLOCKED`）——"什么都没发生"同样是一条读数，它把卡/VBIOS/驱动从鉴别变量里踢了出去，只剩平台。

**运行时蓄意复位的对照实验（2026-11-21，维护者 GA102 Linux 台面）**：双路 Xeon Gold 6326（Supermicro X12DPG-OA6）上的两张 RTX 3090（`2204`）跑 Linux GSP 驱动 620.70.06，超频失败双卡挂死（Xid 62/154，驱动自报 PF FLR 恢复动作）——恢复脚本走完 FLR（`reset_method=flr`）与 SBR（`reset_method=bus`）两路、每轮 rebind 后等 RmInitAdapter 完成 GSP 重初始化，连续多轮恢复后，`tools/diag-unlock.py` 多次读回 `VERDICT: UNLOCKED (override values live)` + `CANARY cleared`。歪打正着补上一块拼图：**运行时驱动发起的 FLR/SBR + GSP 重初始化不碰 AON 岛**——此前"FLR 不掉"只有事务内 `POST-FLR` 自证与 TU106 bench（Windows 驱动 init 不重关）两条单发记录，这是第一条系统内、驱动发起、蓄意多轮的；金丝雀四格的"清零 + FEAT 完好"格也首次有了**复位来源已知**的实例（G614JV 那格是被动读数、FLR/SBR 归类靠推断，这里就是恢复脚本自己执行的那次复位）。两条判读纪律：①金丝雀只见证"本引导发生过 ≥1 次 FLR/SBR 级复位"，多轮存活由每轮恢复后的 `VERDICT`（FEAT 读数）自证，别把 cleared 当逐轮见证；②**rebind + RmInitAdapter 在每轮恢复里完整执行（多次 `NVRM: loading` 与等待 RmInitAdapter 自证）而 FEAT 三连原样**——#148 式 init 期重写不是 init/rebind 的固有步骤，是平台条件分支（该机未触发；#148 的 25a0 是已知唯一触发形态，写入方与触发变量未切开）。GA102 桌面由此成为 **GA10x 首个 Linux 持久存活数据点**（此前 Linux 侧只有 TU102 桌面，[#103](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/103)）。运维含义：Linux 下挂死（超频失败等）走驱动级复位恢复**不丢解锁**，不必重跑 U 盘事务——rail 在位即可。原始控制台记录归档在 `docs/evidence/ga102-2204-oc-reset-recovery/`；恢复工具为 NVOC 仓的 [linux_oc_recover.sh](https://github.com/Skyworks-Neo/nvoc/blob/main/auto-optimizer/test/linux_oc_recover.sh)（五层复位梯子，FLR/SBR 两路均经其手）。

### 平台上到底是谁在断电

dGPU 的电只有两种来源：桌面由插槽直供；笔记本/一体机的 dGPU 集成在平台里，主供电通常由一个 GPIO 切换（BOCO 形态），固件通过 ACPI 电源资源（`_PR3` 的 `_ON` / `_OFF`）把它暴露给 OS。**D3cold 是设备电源状态：设备主供电已被移除**——这是 PCI/ACPI 的通用定义，与实现无关；本类平台用 `_PR3` 电源资源实现它，`_OFF` 是平台实现方式而非定义本身。本平台观察到的 `_OFF` 下电顺序是根口广播 PME_Turn_Off → 链路进 L2/3 → 拉 PERST# → 撤电。**"桌面没有这条路径"不成立**：Z170 桌面板实测（2026-11-04，`docs/evidence/tu106-1f07-acpi-dump/`）三根 PEG 根口各有一个 `_PR3` 指向自己的 `PG0x`，复位与供电各是一对真 GPIO，`_OFF` 的动作就是**先拉 PERST# 再关槽供电**——插槽直供不等于根口不能切它。

更有解释力的一条（**假说，证据来自 Linux，Windows 侧未验证**）：OS 启动时会对**没人引用**的电源资源做状态同步，而 ACPI 允许对已处于关闭态的电源资源再执行一次 `_OFF`；固件 AML 若不幂等，多余的那次就是真断电（coreboot 的 Brya 板专门记录 dGPU 电源状态来躲开源内核的多余调用）。

这块板的 `_ON` / `_OFF` **是幂等**的，但幂等靠的是回读真实硬件而不是 `_STA`：两边入口都先跑 `CCHK`，端口已经能读到 vendor ID、或供电 GPIO 已是目标态就直接返回。所以重复调用不致命，**前提是 GPIO 状态可信**。上面这条假说需要在"AML 不幂等"的机器上才成立，本机更可能的路径是 Windows 自己发起 D3cold。这条假说能同时解释三件事：

- [#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) 是**确定性**复现，但后续轮次把写入方切开了：不修根口时金丝雀 `cleared` 且根口读 `D3`（平台 `_PR3/_OFF` 断轨）；根口退出项把轨保持住（`D0`）后金丝雀**仍** `cleared`，钉出第二条"驱动 bind 期主动复位"（run 5–6）；仅加 `EnableRuntimePowerManagement=0` 后金丝雀 `intact`、`UNLOCKED`（run 7，见 [WinRTD3 实例记录](#relock-winrtd3)）。早先"驱动未绑定 → 写入方不在驱动"的读法过强——轨保住后，发起复位的就是驱动的运行时 D3 路径；
- 为什么 `D3Cold` 注册表开关无效：它拦的是 D3cold 请求，拦不住启动期对电源资源的状态同步；
- 为什么"进入方式"能反转结果：从 Windows 高级启动进，dGPU 被上一轮驱动持有；从 BIOS 冷启进，它是被 loader 从停泊态拉起来的、在 OS 记账里无人引用。

[#28](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/28) 给了一个旁证：混合模式下固件阶段只枚举到 dGPU 的音频功能，即整卡被停泊在未供电态。Modern Standby 还额外要求根口在 D0→D3 时对下游设备施加基础复位（微软固件要求文档），是第三条平台侧触发器。

中招需要三件事同时成立：拓扑上 dGPU 由 ACPI/EC 管电（或整链路走 USB4）、交接时无人引用、且该机 AML 真的切了电或拉了 PERST#——第三件是厂商相关的，所以比例按平台类分布而不是随机。桌面那几个例外（Z390P、B450M-A、B350M）与 B650 的对照原因未知，CSM 状态也没人报过。**issue 是自选样本、没有分母**，所以"百分之几"只能是量级描述。

自己看的话：卷根放 `	x-acpi-dump.txt` 把平台 AML 带回来（标记表 `tx-acpi-dump`），再查父根口的 `_PR3` 是否存在、`_ON` / `_OFF` 是否幂等、PERST# 挂在哪个 GPIO。一半的争论到这一步就有答案了。AML 说的是"平台**有权**怎么做"，`ssdiag` 0.6.8 起的 `POWER:` 两行说的是"**此刻**做到了哪一步"：卡与上游根口各自的 `PMCSR`/D 态，根口读 `D3` 就是 `_OFF` 已经跑过。注意它只是快照——`D0` 不证明这一引导没进过 `D3`，那是金丝雀回答的。

尚未切开：**GC6 与 GCOFF 是两个不同目标**（前者保自刷新，后者是完整断电），`_PR3` 路径与 GC6 唤醒路径目前被混着说，需要一次专门的对照。（2026-11-16 起有一条相邻实证：固件关 ACPI D3 后 GCOFF 不再发生，机理定位见[实例记录](#relock-acpid3)；两目标的专用对照仍未做。）

### 为什么重算不会把卡弄坏

- **devinit 是幂等的**：它就是把芯片从"刚上电"带到"可用"的那一段，冷启时天天跑一遍。重算一次的结果是一张正常工作的卡。
- **我们的覆盖不在功能契约里**：PLM 只改"谁有权写"，SS 只改覆盖分频，底下的 PLL、几何、链路训练全按 devinit 的账走。
- 反过来说：**卡还能正常用，本身就是"完整重算已经发生"的证据**——重算不完整才会坏。
- 代价是失效必然**静默**：没有错误码、没有 WHEA、没有降级，只有收益归零。这就是[系统内验证](#ssdiag)不可省的原因。

### 是驱动的蓄意行为吗

分三层，答案不同：

- **没有针对解锁的检测。** `0x823804` / `0x82381c` / `0x823820` 在 NVIDIA 开源内核模块的全部寄存器头里**一条定义都没有**（唯一沾边的是 `NV_FUSE_FEATURE_READOUT 0x00823814`）。驱动连这些符号都不存在，不可能读它、比较它、决定要不要保留。
- **"复位后必须重算"是蓄意的。** 这是 ASIC 正确性契约：`kbifPrepareForFullChipReset` 的注释原文就是"确保复位后 devinit 脚本能正常执行"，FLR 之后驱动会阻塞等待 devinit 完成（超时打印 "VBIOS boot failed!!"），还有专门的超时缩放键 `RMPcieFlrDevinitTimeout`。它不可跳过，因为它建立的东西比 FEAT 底层得多。
- **"不保留外部覆盖"是结构性疏漏，不是取舍。** 对 NVIDIA 而言 FEAT 覆盖本来是给内部调试用的，出厂全锁，从没人需要"跨复位保留一次外部写入"；再叠上 AON 复位这条物理路径，**根本没有地方存放"有人开过门"这个事实**。

**所以：不是反制，但反制成本低到接近于零**——devinit 已经基本在这么干了。最坏形态永远是"失效而非关锁"，这是这套设计给我们的下限保障。

### 能做什么

做不到的：**门已经关上之后，在系统内重新开门**。开门要靠 SEC2 内跑签名 booter，属上游 kmod 谱系，超出本工具范围；重跑 U 盘事务是唯一恢复。

做得到的：

0. **先排除我们自己的旧尾巴**：v2.1.11 及更早的载具默认在交棒前重锁 FEAT 门，部分驱动会因此在 init 期发起 PF FLR 并把 SS 清零（[#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99)，2026-11-12 同卡 A/B）。`061aef8`（v2.1.12-rc1 起）默认改成门开交棒、SS 原样交棒，升级载具本身即消除该触发；不要用 v2.1.11 复测这个问题。
1. **阻断复位源（优先免固件的驱动级键）**：Windows 侧给 NVIDIA 显示适配器注册表键设 `EnableRuntimePowerManagement=0`（一键脚本 `tools/disable-rtd3.ps1`；不碰固件、可脚本化撤销，Intel+AMD 双平台实测 [#254](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/254)/[#266](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/266)，**G614JV 实测与固件开关在系统内行为等价**：混合解锁保留、GCOFF 不再发生、ghelper 热切换纯核显失效——2026-11-20，见[实例记录](#relock-winrtd3)）；第三个平台、首个 `2520`（MUXless 82K2）随后给出"轨已保住、复位仍发生"现场里键为唯一变量的转绿（[#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) run 7，FP16 +54% / BF16 +57%，同一[实例记录](#relock-winrtd3)）；Linux 侧 `disable-rtd3.sh`（[Linux 一节](#linux)）。**固件侧 ACPI D3 开关（含隐藏项）为备选**（2026-11-16 实证，见[实例记录](#relock-acpid3)）：全局生效（含 Linux、免疫驱动升级），但要进隐藏菜单且纯核显档失效。再关快速启动与现代待机、BIOS 里关 PCIe 空闲电源门控；**双显三模机型插电启动**——关 RTD3 后离电（电池）启动可能让 EC 启动阶段就不给 dGPU 上电、驱动绑定未上电设备而内核态 PnP 阻塞（2026-11-22 实测，[实例记录](#relock-winrtd3)）；不要用“关机”当重启。
2. **改变入口态**：独显直连优先；混合输出下 MUX/面板归属决定设备是否被枚举（[#148 记录](#relock-148)）。
3. **事后修复**：门仍开时 `ssdiag --reapply` / `diag-unlock.py --reapply` 写回选择器；门已锁只能重放。
4. **报告时带齐证据**：`loader.log` 的 `POST-FLR` / `POST-GOP` / `HANDOFF-OUT` 三行（含 `CANARY` 与 `wpr2_lo`）、系统内读回值（用带 `CANARY:` 判定行的 `ssdiag` 0.6.5+）、`powercfg /a`、设备电源状态与事件日志。**外加同一 boot 的锁态基线**（解锁前、同一次引导内先跑一遍 `ssdiag`）：不少卡的 stock `SS0/SS1` 本来就含 0（本机实例见诊断协议 §4），没有基线时「SS=0」判不了是清零还是出厂。缺这些只能反复猜。
5. **把平台属性化**：确认某平台有 `_PR3` 且 rail 真断，就按平台属性登记，而不是当驱动缺陷追。
6. **给 dGPU 制造引用**：在 dGPU 直连的接口上接一台外接显示器再冷启。Linux 社区的经验说法是"接在 NVIDIA 一侧的显示器会挡住 D3cold"（轶事）；若丢锁随之消失，就支持"无人引用 → 被同步成关闭"这条假说。**Windows 侧程序级引用已实证（[D#229](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/229)，2026-11-11）**：常驻持有 CUDA context + 每 5 s 真 kernel 提交（计划任务 SYSTEM 开机自启）即可阻止 RTD3 park——`nvidia-smi` 仍 P8/1.5 W 不抬功耗、混合拓扑零改动不接显示器，同引导 6/6 采样近 12 min 全 `HELD`（对照无保活 ~2 min 空闲即被抹）；边界如实：未做停保活因果对照、Modern Standby/合盖未测、"交棒→驱动接管"空窗用户态无解。Linux 侧等价物=关 RTD3（[一键脚本](#linux)）。

<a id="relock-acpid3"></a>

### 实例记录：混合本在固件层全局关闭 ACPI D3 后解锁 100% 保留（2026-11-16，开发者本机）

机器：ASUS ROG Strix G16 G614JV（RTX 4060 Laptop `10de:28e0`，混合输出/Optimus——机制表第 1 行“真掉电”的标准宿主形态）。操作只有一项：在该机隐藏 BIOS 菜单里把 **ACPI D3 support 全局关闭**。结果：**解锁 100% 保留**——#224 的欺诈器、D#229 的 CUDA 保活、独显直连切换等系统侧补偿一概不再需要。

**系统内金丝雀读数（同日补录）**：进系统 `ssdiag`（0.6.7+ 判定行）读回 `VERDICT [ada]: UNLOCKED (override values live)`、`CANARY: cleared + SS live - a reset happened but the AON island survived it (FLR/SBR class, not a rail-off)`。这把结论在**第二条独立证据轴**上又钉了一遍：金丝雀清零=本引导确实发生过一次 FLR/SBR 级复位，SS 活着=**撤电没有发生**——与第 1 行 rail 断的形态（金丝雀和 FEAT 一起没）正相反。这也是判读四格里“清零 + FEAT 完好”格的**首个实例记录**（#37 Victus 占“两者一起没”、#109 B650 占“完好+完好”、TU106 桌面对照占“完好+被抹”写入型）。关掉 ACPI D3 的效果由此在两条轴上同时可见：状态轴（解锁保留）与复位轴（只剩 FLR/SBR 级复位，AON 岛按设计存活——它只在撤电时复位，其余复位源够不着它）。

价值在方向上：此前“混合模式进系统丢解锁 = ACPI D3 断电”是从 `POWER:`/`_PR3` 采样与 [#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37)/[#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126)/[#224](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/224) 现场归纳出的**判读**；这是第一次从**固件侧旋钮**反向验证——把固件给平台的 D3 电源路径本身关掉，丢失随之消失。第 1 行的处理顺序由此多出最优先的一档：固件有 ACPI D3 开关（含隐藏项）就先关它，比一切系统侧手段都更根本；零独显直出、欺诈器用不上的机器也多了一条路。 **2026-11-20 排序更新**：无固件开关可用的机器直接用驱动级键——[#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) 在 AMD MUXless 无 MUX 无出线的极端形态（IdeaPad Gaming 3 82K2 / `2520`）上单键转绿并收官，键的验证面现为三形态（Intel 混合本 [#254](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/254) / AMD 零出线 [#266](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/266) / AMD MUXless [#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126)），见[下一实例](#relock-winrtd3)。
**已知代价（2026-11-18 实测补充，同一台 G614JV）**：全局关闭 ACPI D3 会让**双显三模的热切换失效**——纯核显档需要 dGPU 断电，而 D3 被关后这条供电路径不复存在。表现为：切到纯核显后设备管理器里独显不消失；或消失但等价于一次单纯的驱动卸载（驱动不在了、电还在耗）。需要纯核显档（省电/续航场景）时把隐藏选项切回，或接受该档不可用——与机制表第 1 行自洽：D3 开关抑制的是"真掉电"复位源，它同时挡住了掉锁与省电，一体两面。 **换用驱动级键（[实例记录](#relock-winrtd3)，AMD+Intel 双平台）不能换回纯核显档**——2026-11-20 G614JV 实测：撤本开关、只设 `EnableRuntimePowerManagement=0` 后，混合解锁保留与 GCOFF 失效逐项与本开关一致，**ghelper 热切换进纯核显同样失效**（切档断电需要驱动发起运行时 D3 配合，键把它一并拦了；本节早先"模式切换走固件路径、不受键影响"的推断被此轮证伪）。两旋钮在系统内行为等价（保留/GCOFF/热切换三项一致），选择只看运维属性：键=免固件、单卡粒度、可脚本撤销、仅 Windows；本开关=全局生效（含 Linux）、免疫驱动升级、需隐藏菜单。

固件里找不到这个开关的话：[gsetupmod](https://github.com/MikeWuPing/gsetupmod)（第三方 UEFI 设置浏览器，解析固件 HII/IFR、把 `SUPPRESS_IF` 隐藏项以可编辑形式显示、经 UEFI 变量写回）——**与本项目无关**，开发者在 G614JV 上实测：在其中关闭 ACPI D3 与解锁隐藏 BIOS 菜单效果一致。约束如实：镜像未签名（需 Secure Boot 关闭）；个人使用免费、商用需联系作者；改的是固件 setup 变量——一次只改一项并记录改了什么。

**机理补注（2026-11-16，对照 `reference/open-gpu-kernel-modules` 615.78.08 源码定位）**：

- **深层状态的准入是固件+GSP 协商出来的，不是驱动硬编码**：RTD3 族的 GC6/GC8/GCOFF 能力位由 GSP 固件经 POWER_FEATURES 键值上报（`kernel_gsp.c`：`POWER_FEATURES_IS_{GC6,GC8,GCOFF}_RTD3_ALLOWED` → `bIsGcOffRtd3Allowed` 等），RM 落成 `PDB_PROP_GPU_RTD3_GCOFF_SUPPORTED`，`dynamic-power.c` 的 `RmCheckRtd3GcxSupport()` 只认这些属性。而 GCOFF 的动作本身就是"整颗芯片撤电"——根口 PME_Turn_Off → L2/3 → PERST#（驱动源码里连 `DEFAULT_GCOFF_PEXRST_DELAY` 都为它而设）→ 撤电，这条物理链只有平台固件走得通（见上文"平台上到底是谁在断电"），驱动还会专门评估上游根口的 D3cold `_DSM`（`nv_acpi_d3cold_dsm_for_upstream_port()`）。固件关掉 ACPI D3 后，平台断电资源（`_PR3`）从枚举里消失，GCOFF 失去准入——"关 ACPI D3 后 GCOFF 不再发生"的观察与此吻合。（`IS_GCOFF_RTD3_ALLOWED` 如何感知平台 `_PR3` 这一跳未在源码内钉死，此处按实测事实+上述定位如实记录。）
- **NVPCF 是这条断电-唤醒循环的加速器**：`\_SB.NPCF` 是 Dynamic Boost 的平台 ACPI 设备，Linux 驱动注册 `NPCF` 通知回调并直接调 `\_SB.NPCF._DSM`（`nv-acpi.c`）。上游报告其通知会把已进 D3cold 的 GPU 周期性唤醒（open-gpu-kernel-modules [#1300](https://github.com/NVIDIA/open-gpu-kernel-modules/issues/1300)；[#1299](https://github.com/NVIDIA/open-gpu-kernel-modules/pull/1299) 尝试把通知推迟到运行时恢复再处理）——NVPCF 机器上这条循环本就更频繁，混合本"几分钟内掉锁"的节奏与此一致；固件关掉 ACPI D3 后 GPU 不再进 D3cold，NVPCF 通知也就没有可唤醒的对象。
- **两层关法不同**：固件层（BIOS 选项，本条记录）最干净——驱动在能力协商阶段就看不到断电路径，整族需要平台断电的深层状态一并失去准入；运行时内核层禁（`pci_d3cold_disable()` / `d3cold_allowed=0`）只拦 D3hot→D3cold 的最后一步、设备停在 D3hot，驱动若未感知仍会按断电流程走（上游 [#1228](https://github.com/NVIDIA/open-gpu-kernel-modules/pull/1228) 描述的 eGPU 形态故障：GPU 侧走完断电流程、板电未断、GSP 报错掉总线）。

<a id="relock-winrtd3"></a>

### 实例记录：混合模式本只关驱动的运行时电源管理就保住了解锁（2026-11-18，MSI Vector 16 HX）

机器：MSI Vector 16 HX A13VHG（MS-15M1，i9-13980HX，RTX 4080 Laptop `27a0`，BIOS `E15M1IMS.914`，驱动 610.47 LEGACY，Win11 26H2）。同一份报告（[#254](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/254) / [D#255](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/255)）把两种显示模式都测了：**独显直连一次通过、进系统保留**；**混合（MSHybrid）模式则在 Windows 驱动 init / 运行时电源管理阶段被清**——EFI 侧全链与 POST-FLR 都绿，失效发生在交棒之后。

处理只有一项：在**动态解析到的 NVIDIA 显示适配器注册表键**下设 DWORD `EnableRuntimePowerManagement = 0`（一键脚本 `tools/disable-rtd3.ps1`：默认应用、`-Check` 只读体检、`-Undo` 还原；键索引随驱动安装变化，脚本运行时动态解析、永不硬编码；写模式需管理员），重启后重放 EFI 事务。**制盘脚本现在会主动提示**（2026-11-20 起）：`make-usbroot.ps1`/`.sh` 在“笔记本机箱 + 运行时 D3 开启 + 无显示输出接独显”三条件齐时打着色警告并指向本脚本——正是本节签名的前置条件组合，混合模式下解锁即保留进 Windows（同引导 ssdiag 判 `UNLOCKED`、canary intact）。这是机制表第 1 行"真掉电 / RTD3"在 Windows 侧的一个**驱动级旋钮**：不碰固件、不碰 ACPI 表，只让 NVIDIA 驱动不再对这张卡发起运行时 D3 转换——因此也不同于上面 G614JV 的"固件关 ACPI D3"（后者把平台侧 `_PR3` 路径一起去掉，代价是纯核显档失效）。

边界如实：①该报告只用 ssdiag 0.7.9 读了寄存器与 canary，**没有"设 / 不设该键"的同会话因果对照**（混合模式的失败轮与成功轮之间隔着一次重启 + 重放）；②`EnableRuntimePowerManagement` 是 Windows 显示驱动的通用电源策略键，不同驱动分支是否都认这个键未逐一核对；③能用独显直连时仍优先独显直连，混合模式保留不等于所有平台可复现。排查顺序照旧：先确认 `POST-GOP` 与 `HANDOFF-OUT` 完好（排除事务与交棒），再看 `POWER:` 行的 D 态与 **RTD3 策略行**（`ssdiag` 0.8.3 起，直接读这个键的现值；`--inventory` 也打 `GPU n RTD3:`），最后才动这个键——键的现值探测不需要动它。

**第二个平台 + 因果对照补强（2026-11-20，[#266](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/266)，AMD Acer Swift SFX14-41G / Ryzen 7 5800U）**：`25a2`（3050 Laptop 4GB，GA107）在**纯 MUXless、零 dGPU 出线**的轻薄本上（连欺骗器都物理不可行，BIOS 也无 ACPI D3 开关），v2.1.12-rc4 两次独立引导 `UNLOCKED`、金丝雀 intact、张量发射率 **2.00×**（寄存器级 WMMA 微基准，FP32 FFMA 对照 1.00× 平）。本报告还补上了上面边界 (1) 缺的**同机因果对照**：同机同盘，不设键的一轮=金丝雀 cleared + SS 回厂（[#262](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/262) 同形签名逐项一致），只加 `EnableRuntimePowerManagement=0`（+ASPM AC/DC=0）的一轮=intact + `UNLOCKED`——键是两轮间唯一变量，机制表第 1 行（轨掉）就此在 AMD 平台坐实。**也是 [#262](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/262)"混合+无 MUX+无 dGPU 出线"形态的第一条可行路**：固件开关与欺骗器都不可行的机器，先试这个驱动级键。边界更新：边界 (1) 已由本报告补上（同机、键为唯一变量）；新增一条——该键挂在驱动键下，**驱动升级/重装可能重建键，升级后须复查重设**（2026-11-28 已有一次 in-OS 升级保留样本，见下方 G614JV 驱动升级轮；键值是否重建仍未复查）；边界 (2)(3) 维持。

**与固件开关的系统内等价性（2026-11-20，G614JV 4060L 实测）**：同一台机先有固件 ACPI D3 全局关闭的 100% 保留记录（[上节](#relock-acpid3)），撤开关、只设本键后复测——**三个行为逐项一致**：混合模式解锁保留；GCOFF 不再发生；ghelper 热切换进纯核显失效（切换断电需要驱动发起运行时 D3 配合，键拦了它就拦了切换）。两旋钮在系统内等价，差异只在：固件开关全局生效（含 Linux）、免疫驱动升级、要进隐藏菜单；本键单卡粒度、Windows-only、可脚本化撤销（`tools/disable-rtd3.ps1 -Undo`）。**作为免固件方案优先提供**。

**驱动升级轮（2026-11-28 补录，同一台 G614JV 4060L）**：在 Windows 内把 NVIDIA 驱动从 610 系更新到 621.40，更新后解锁保留。这给 [#266](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/266) / [#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) 的驱动升级/重装可能重建键这一边界补上第一个 in-OS 升级保留样本：**这一次升级路径没有抹掉解锁**，但**不**足以判定注册表键本身跨升级存活。边界如实：未记录升级是否伴随重启、是否在重启后重放、当时生效的是固件 ACPI D3 开关还是驱动级 `EnableRuntimePowerManagement=0`、以及保留判据（`ssdiag` 的 `UNLOCKED`/canary 还是仅性能）；按易失模型，若升级中发生过重启，解锁必须由 U 盘/内置分区重放才能保留，因此不能读作跨重启保留。仅覆盖 in-OS 升级，不覆盖干净重装；键值是否被重建也未复查——升级后仍应按 [#266](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/266) 的纪律跑一次 `ssdiag`/`-Check`。

**离电（电池）启动的新边界（2026-11-22，G614JV 4060L 维护者侧实测）**：RTD3 被关闭（键或固件开关，两旋钮等价故边界共享）的**双显三模热切换机型**上，以**离电（电池）模式**启动时，EC 可能决定**启动阶段就不给 dGPU 上电**——进入系统后，被关掉 RTD3 的 GPU 无法正常启动，出现**内核态设备 PnP 阻塞**。事件签名如实转录，先后两段：先 `\Device\UVMLiteController0x1` 报 `timeout error waiting for NV_UFLUSH_FB_FLUSH = 0xffffffff cnt=320330/320169/xxxxxx`（`0xffffffff` 是 BAR 全 f 读数——卡不在供电态；第三段计数随现场变化）；随后 `nvlddmkm` **id=14 以每秒 10~20 次连发**：`\Device\UVMLiteController0x1 CMDre 000000XX 00003ffc ffffffff 00000007 ffffffff`（或尾部 `00fffffff`，XX=00~ff 递增计数）。**归类：双显三模热切换的副作用——平台电源管理限制**，不是工具或键的 bug：EC 的启动供电决策（电池模式省电）与被禁用的运行时 D3 叠加，驱动绑定到一台未上电的设备，FB flush 等待全 f 超时，PnP start 卡死。处置：这类机器**插电（外接电源）启动**——EC 正常给 dGPU 上电则一切照常；已卡死的现场先接 AC 再重启。与解锁无关：该形态发生在 stock 供电决策层，先于任何事务状态。

**第三个平台、首个原生笔记本 `2520`，也是首例"已定性为驱动 bind 期复位"的机器仅靠该键转绿（2026-11-20，[#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126)）**：联想 IdeaPad Gaming 3 82K2（RTX 3060 Laptop `10de:2520`，GA106M，AMD 平台，根口 `00:01.1` = `1022:1633`，**无 MUX、零 dGPU 出线**，驱动 621.01）此前已积累六轮反例——NVCP 最高性能优先、ASPM AC=0、端点 D3Cold 退出项、根口类实例 `PnPCapabilities=24` + 根口 D3Cold 退出项、开机 60 s 内抢跑基准、驱动 591.86→621.01。run 7 的分量来自这段历史：前面的金丝雀轮已把丢失切成**两层叠加的凶手**——(a) **平台 `_PR3/_OFF` 断轨**，run 4 读到金丝雀 `cleared`、根口 `D3`；(b) **驱动 bind 期主动复位**，run 5 用根口退出项把轨保持住（根口 `D0`）后金丝雀**仍** `cleared`，run 6 在更新驱动上签名不变。run 7 只在 run 6 的注册表栈上加 `EnableRuntimePowerManagement=0`：EFI 日志照旧全绿，系统内 ssdiag 0.7.0 读到卡 `PMCSR=D0`、根口 `D0`、**金丝雀 `intact`**、`SS0=0x88888888`/`SS1=0x8` 活、`VERDICT [ampere]: UNLOCKED`；GEMM（8192³、FP32 累加、15 取中位）FP16 26.3–26.7 → **40.83 TFLOPS（+54%）**、BF16 26.6–26.8 → **42.20（+57%）**、FP32 对照平（8.58–8.62 → 8.60）；固定种子 SDXL（Forge，832×1216，25 步）25.3 s → **17.2–18.4 s（约 −30%）**，满载 73 °C / ~50 W / 1770 MHz。**这是该键首次在"轨已保住、复位仍发生"的现场作为唯一变量转绿**——它覆盖双层叠加形态里的两条机制，而不只是第 1 行的断轨。也是首个原生笔记本 `2520`、第三个 Windows 平台（Intel MSI / AMD Acer / AMD 联想）。边界：run 7 未记录撤掉前几轮的注册表开关，应按"run 6 + 键"读（对 run 6 单变量，不是从原厂态起的 A/B）；睡眠/S4 仍按设计清锁；键挂在驱动键下，驱动升级/重装后须复查重设。Linux 侧等价物仍是 `NVreg_DynamicPowerManagement=0x00`（[#230](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/230)）。完整运行矩阵与读回：[`docs/evidence/issue126-82k2-2520/`](evidence/issue126-82k2-2520/README.md)。

<a id="relock-revive"></a>

### 实例记录：总线号隐藏族——死槽复活阶梯首战定谳（2026-11-23，[#276](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/276)）

Dell XPS 8940（0427JK，i7-11700，Rocket Lake-S）+ Dell OEM RTX 3060 Ti `10de:2486`（GA104）：**每次 UEFI boot 枚举时 PEG 槽读 vendor 0xffff**，loader 报 "no transaction target"；而卡全程驱动 console（DP/HDMI 双口验证）、Windows 里健康且编号 43:00.0。iasl 反解证明 DSDT/SSDT **无任何 ACPI 电源资源**管这条槽。这个家族的机理与「谁把解锁冲掉」（本章主线）完全不同——它发生在**枚举可见性**层：

- **机理 = bus-number hiding（总线号隐藏）**：rc6 的 PORT-DIAG 一次裁决——PEG 口 `D0 / ldis=0 / DLLLA=1（Gen3 x16 训练着）/ secbus=0x00`，死功能句柄创建于 bus 2。固件在 boot-manager 阶段把端口的 **secondary/subordinate 总线寄存器清零**（config 隔离门），发往 bus 2 的 config TLP 失去路由 → master-abort → 0xffff。**卡从未复位也从未断电**：display engine 连续翻新（mem 窗口没动、scanout 自治）、链路稳定训练、config 内容含 BARs 原样存活在路由断点后面。Windows 43:00.0 = pci.sys 自己重编号写的新标签（Windows 自行枚举所以不受影响）。规范层面 spec-legal：sec/sub=0/0 即上电默认值，UEFI 对 handoff 时桥编号无不变量；行为假设是「下一个枚举者会自己重新发现设备」——Windows 会，通用工具不会。
- **复活链（rc7，零复位零写入）**：`SLOT-REVIVE` 的 ROM 身份门从固件缓存影子读出 `vendor=0x10de device=0x2486 (NVIDIA)`（config 死了影子还在——它也是死槽 device ID 的唯一来源）→ `NoneRelocated` 杠杆把记录总线写回父口 sec/sub（`0x00000000 → 0x00020200`，读回验证）→ reprobe vendor `0x10de` = **路由隔离、从未死** → 槽保留候选表 → **交易在原始 bus-2 地址同 boot 运行**（Windows 接管后才变 43——这本身即重编号诊断的实证）→ 全链绿 → 系统内 `VERDICT [ampere]: UNLOCKED`、canary intact、双 `D0`。零复位、零 BAR 恢复——重编号族根本不需要那些杠杆。
- **判别纪律**（PORT-DIAG，无条件）：DLLLA=1 + 当前 bus ≠ 句柄创建 bus = 重编号族（写回即活）；DLLLA=0 = 链路真关闭族（D3hot/LinkDisable/L2L3-Ready，SBR 阶梯）；DLLLA=1 + bus 一致 + 子端仍 0xffff = 软死族（台架 ASM1080 三连阴性：SBR 无效，需真实断电）。无 ROM 影子的死槽（桥类）被身份门 decline——不重置不可识别的设备。
- 边界：单机 n=1；11-20 一晚该机曾正常枚举（门控按 boot 路径变量触发、变量未定位）；性能基准后续补录。证据：[`docs/evidence/issue276-xps8940-renumbering/`](evidence/issue276-xps8940-renumbering/README.md)。

**独显直连分支的字段数据点（2026-11-21，[#273](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/273)，`2860`/ASUS FA607PI）**：RTD3 键 absent（驱动默认允许 runtime D3）+ 独显直连（`display_active=Enabled`）的机器，解锁态两轮引导 + 负载下三采存活，与 maker/FAQ 预检的分级框一致（display-active 保活 → risk low while outputs stay on the card）；S3/休眠未测（同族 `2820` 的 [D#226 追评](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/226) = 睡眠抹除，预期行为）。

<a id="relock-148"></a>

### 实例记录：GA107M 笔记本在 Linux 上被驱动 init 重推导（[#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)）

这是目前唯一一份把 RTD3、GSP-RM、FLR 三个嫌疑一次切干净的记录，值得留档。

配置：RTX 3050 Ti Laptop（GA107M `10de:25a0`）、Linux Mint 22.3 / 595.71.05、RTD3 按 FAQ 关到 `0x00` 并核对 `/proc/driver/nvidia/params`、`supergfxctl -m AsusMuxDgpu`。事务全绿（含 `POST-FLR: verified`），系统内 BAR0 只读 mmap 读回：

| 寄存器 | 开机 | 载荷写入 | 系统内（GSP 开） | 系统内（GSP 关） |
| --- | --- | --- | --- | --- |
| `FEAT_PLM 0x823804` | `0xffffff8f` | `0xffffffff` | `0xffffff8f` | `0xffffff8f` |
| `FEAT_SS0 0x82381c` | `0x57340065` | `0x88888888` | `0x57740165` | `0x57740065` |
| `FEAT_SS1 0x823820` | `0x00000007` | `0x00000008` | `0x00000007` | `0x00000007` |
| `FEAT+0x08 0x823808` | `0x00000082` | — | `0x00000082` | `0x00000083` |

三条推论：

- **不是 RTD3**：RTD3 已关，状态在 `modprobe nvidia` 时丢，不在空闲掉电时丢。
- **不是 FLR**：事务内 `POST-FLR` 验证存活。
- **是 init 路径主动编程**：`SS0` 相对开机值的 delta 在 GSP 下为 `0x00400100`、关掉 GSP 后为 `0x00400000`——两条 init 路径写**不同的位**，`FEAT+0x08` 也随之在 `0x82` / `0x83` 之间变。这是写入，不是"没保留"。

两个操作细节：`resource0` **只支持 mmap**，`dd` 会静默返回空、看起来像"门已关"的假阴性；另一台机器上同一张盘在 Hybrid 模式下枚举不到卡（`vendor 0xffff`），切成 dGPU 独占面板后才可跑——**面板归属是入口轴，不是冷/热**。

性能上没有收益且略降：`80 W` TGP 被内屏共享，FP16/BF16 约 `0.92–0.93×`，连不用 tensor 路径的 FP32 也是 `0.92×`——说明不是张量速率效应，是供电预算。**不外推**：`25a0` 是唯一一例 Linux 重推导，同族 `25ad` 在 Windows 持久存活，OS / 驱动版本 / MUX-EC 平台态三个变量尚未切开。

### Linux 上关掉 GSP 有什么用、代价是什么

关掉 GSP 是**诊断杆**，不是修复手段：它能把"GSP-RM 晚期的 devinit"从内核 init 路径里切出去。`#148` 的结论是**关掉也照样重推导**，所以它治不了丢锁；但正因为两种模式的写入位不同，它仍是分辨写入方的有效工具。

关法（仅 Linux，临时用）：

```bash
echo 'options nvidia NVreg_EnableGpuFirmware=0' | \
    sudo tee /etc/modprobe.d/nvidia-no-gsp.conf
sudo update-initramfs -u     # Fedora/RHEL: sudo dracut -f
```

重启后核对 `nvidia-smi -q` 的 `GSP Firmware Version: N/A` 与 `/proc/driver/nvidia/params` 的 `EnableGpuFirmware: 0`，否则测试无效。

**副作用（实测，[#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)）**：动态电源管理失效——空闲停在 P0 / 1927 MHz / **26.9 W**，而不是 P5 / 约 8 W。算力不变，但待机功耗与温度明显上升，**不适合日常常开，用完即撤**。另外这是 Linux 专有驱动的旋钮：**Windows 的 Turing 反过来需要 `EnableGpuFirmware=1`**（[Turing 一节](#turing)），别混着抄。

<a id="sleep"></a>

## Windows 睡眠后解锁还在吗？（S3 / S0ix 现代待机）

会掉——"睡眠"不是一个状态，而是几类电源路径的统称，其中大多数会在唤醒时让 GPU 重新走一遍上电初始化（devinit），易失解锁随之清除。先弄清你的机器用的是哪种：

| 状态 | 常见于 | 睡眠期间 GPU 供电 | 唤醒后 | 处理 |
| --- | --- | --- | --- | --- |
| S0 工作态（熄屏/锁屏） | 所有机器 | 保留 | 解锁不受影响 | 无需处理 |
| S0ix / 现代待机（Modern Standby） | 近年笔记本 | 可被驱动运行时电源管理降入 D3cold（rail 掉电） | 多数掉；选择器回厂（有带金丝雀判据的保留反例，见下方机制说明） | 先只读检查，多数可用 `--reapply` 修回 |
| S3 传统睡眠 | 多数台式机 BIOS 的"睡眠" | CPU 停、内存保电；主板上哪些芯片/设备断电因平台而异（多数台式机 dGPU 掉） | 通常掉（与 GPU rail 是否掉一致） | 同上，视检查结果 |
| S4 休眠 / 快速启动"关机" | 笔记本休眠、Win10/11 默认快速启动 | 全机断电 | 必然掉，等同冷启动 | 同上，视检查结果 |

查看本机支持哪些状态：管理员命令行 `powercfg /a`——"待机(S0 低电量待机)"即 S0ix，"待机(S3)"即传统睡眠；两者都列出的机器由 BIOS/驱动决定实际走哪个。

几点机制说明：

- **判据是 rail 是否掉电，不是"睡没睡"**。S0ix 名义上停在 S0，但 dGPU 空闲时可被独立断电（与 Linux 的 RTD3 同机制，见[上一节](#linux)）；S3 停掉 CPU、丢失芯片组上下文，保留内存自刷新供电；主板上哪些芯片/设备断电因平台而异（多数台式机 dGPU 掉）。两者在唤醒后的表现可能一样：选择器被 devinit 重新推导回厂。
- **S0ix 不必然掉——首个带金丝雀判据的保留现场（2026-11-20，[#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99#issuecomment-7041057712)）**：GA104M `249D`（神舟 NH5xEPER，混合无 MUX）在 v2.1.12-rc3 上"开始 → 电源 → 睡眠"四轮全部存活、金丝雀 `intact`——其中一轮**删除 `EnableRuntimePowerManagement` 键后**照常保留。金丝雀完好意味着整个睡眠→唤醒周期无 FLR/SBR、无断轨、无写入：该机的 S0ix 路径对 dGPU 保电，"通常掉"在这台机上不发生——睡眠掉锁按平台分布，不是定律。同机 v2.1.6 一次唤醒即丢且**形状不同**：门关 + SS 非覆盖值（`0x03000141`/`0x7`）——落机制表第 1/2 行（回开机基线）或第 3 行（重推导），`--reapply` 会被 `REFUSED: gate closed` 拒绝，只能重跑 U 盘事务；该轮丢失归因未定（v2.1.6 在 GA10x 上交棒本就门开，重锁是 Turing 臂专属，机制表第 7 行），单次负样本不足以判版本因果，判读仍以同 boot 锁态基线 + 金丝雀 + `POWER:` 为准。注意旧载具（< v2.1.11-diag1）没有金丝雀写者，其 boot 上金丝雀读到 `0x0/0x0` 是"从未武装"的默认态，ssdiag 的罐头复位判读在该场景是假阳性。
- **FLR 不掉、掉电掉**：解锁写活在 AON（常开岛），事务内 FLR 后选择器存活是设计判据；但 AON 不活过失电——所以"重启进不了系统就丢"和"睡眠唤醒丢"可以是同一件事。运行时同理：Linux 驱动发起的 FLR+GSP 重初始化复位恢复多轮不掉（2026-11-21 GA102 实测，见[机制盘点](#relock)节）——挂死恢复不必重跑 U 盘，rail 在位即可。
- **门与选择器要分开判**：唤醒后 Ampere/Ada 常见形态是 FEAT 门仍开、SS 值回厂——跑 [系统内检查](#ssdiag)确认后用 `ssdiag --reapply` 写回即可，不必重跑 U 盘；若门已锁或设备不可读，说明 rail 确实掉了，需重跑 U 盘事务。**全机断电路径（S4 休眠 / 快速启动）必然门锁**：AON 岛不活过失电，此时 `--reapply` 会以 `REFUSED: gate closed` 拒绝——这是正确判读不是工具故障，唯一恢复 = 重跑 U 盘事务（D#129 用户侧实证）。Turing 的 ssdiag 保持只读（其 SS 覆盖值在驱动加载后持久，devinit 重推导行为不同）。
- **不想让它掉**：台式机可在 BIOS/驱动侧避免空闲掉电；笔记本 S0ix 机器可关掉"现代待机"（部分固件提供）或接受唤醒后 reapply。Linux 侧对应配置见 [RTD3 节](#linux)。
- **台式机"便利与持久解锁兼得"的推荐配方**（[#137](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/137)，`1e89` 实测）：关闭自动 **S3 睡眠**（"睡眠=从不"——空闲 S3 会静默抹掉解锁：canary cleared、选择器回厂），改用 **休眠（S4）**——并把解锁 loader 保留在引导路径上（U 盘 BootOrder 靠前，或内置分区配方）。休眠的下一次上电就是一次正常固件引导，loader 会在 bootmgfw 碰 hiberfile 之前重放解锁：会话原样恢复且卡已解锁。每次引导都重放解锁是设计本身，休眠把这个设计变成了功能。

<a id="thunderbolt"></a>

## 雷电 3/4/USB4 eGPU 能保留解锁吗？

先定义本节的关键词：**连接管理器（CM，Connection Manager）**——雷电/USB4 架构里负责枚举路由器、建立并管理隧道的那个实体，**既可以在固件里，也可以在操作系统里**。雷电主机芯片自带 Firmware CM（Windows 不参与；系统里表现为 Intel 雷电控制面板、没有 USB4 设置项）；操作系统侧的 CM 是 USB4 CM——Windows 的 `Usb4HostRouter`/`Usb4DeviceRouter` 服务栈，或 Linux 的内核 `thunderbolt` 模块。

**丢不解锁，取决于开机后由哪一侧的 CM 去建立/重建隧道**：谁拆建下游总线，谁的那次动作就整域复位，把 AON 里的门与选择器一并清掉（金丝雀 `cleared`）。

| 主机形态 | 判别 | 开机后谁建/重建隧道 | 结果 |
|---|---|---|---|
| **独立雷电 host 芯片 + 固件/legacy CM**（Intel JHL8xxx 类雷电主机芯片，如 JHL8540；BIOS 侧 `USB4 CM Mode = Firmware CM` 一类设置） | 系统里有 **Intel 雷电控制面板**、设置里**没有 USB4 项目**（G614JV 实测）；设备管理器见 "Thunderbolt Controller"、无 `USB4\ROOT_*_ROUTER` 节点 | **开机：没人拆建**——固件 POST 建好的拓扑被原样继承 | **首次开机即保留**。但重插线时管理栈重建隧道 → 域复位（JHL8540 那台实测：PLM 锁回、速率回厂、**金丝雀死亡 `cleared`**）——所以是**每次引导级**保留，不是热插拔级 |
| **USB4 主机路由器**（集成的 Intel/AMD，或绑定 MS `usb4hostrouter.inf` 的独立芯片如 ASMedia；含被 BIOS 设成 OS CM 模式的独立雷电芯片） | `PCI\…&USB4_MS_CM` 存在、`Usb4HostRouter`/`Usb4DeviceRouter` 服务在 | **开机：OS 侧 CM 一绑定就拆建**（Windows `Usb4DeviceRouter`、Linux `thunderbolt` 模块） | **开机即丢**。[#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54) 桌面 TB4 + AD102：交棒后、POST 交 OS 后、乃至已启动的无驱动 Linux 里金丝雀都 `intact`、门 `0xffffffff`——丢只丢在 CM 拆建那几秒 |
| TB4 设备接在 USB4 主机上（向下兼容） | 同 USB4 行 | 同上 | 同上 |

依据（上表即本仓全部已知的雷电/USB4 实验数据点）：维护者 G614JV（JHL8540 = 雷电 4 主机芯片 + TB3 坞，ATX 强制供电，首开保留/重插域复位）；[#74](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/74) 的 PHN16-72（BIOS `Firmware CM Only` + TB4 坞强制供电，系统内 `UNLOCKED`）；Ambolio（ASMedia 芯片但被 MS CM 绑定，开机丢、抑制后留）；GWASDW123（Intel 集成 USB4 + TB3 坞，同前，另测**时序路线无效**——把两路由提前到内核早期、隧道窗口 8.06 s→4.31 s 解锁照样丢，丢锁只看"拆建是否发生"不看"多晚"）；Linux 三轮（`thunderbolt` 模块拆建 / `host_reset=0` 抑制）。**#54 与 #74 均已关闭 completed（2026-11-07）**——后续新的主机形态或丢失形态开新帖，附 `USB4:` 行与金丝雀读数。

**维护者结论（2026-11-06）**：有独立雷电 host 芯片（如 JHL8xxx）+ 坞强制 ATX 上电 → 雷电 3、4 都能保留；USB4 与 USB4 兼容模式下的雷电 4 → 开机即丢，按下面的抑制配方处理。三个必须带上的限定：①判别变量是**绑定哪个 CM 栈**，不是"有没有独立芯片"——ASMedia 是独立芯片但被 MS `usb4hostrouter.inf` 绑定，照样拆建；独立雷电芯片也可能被 BIOS 设成 OS CM 模式，所以以"Intel 雷电控制面板 vs USB4 路由节点/`USB4:` 行"为准。②PHN16-72 那台的抑制状态未记录；若同为 `Firmware CM Only` 则本就无需抑制。③Linux 侧 `thunderbolt` 内核 CM 对所有主机形态都接管（Run A），同样只有 `host_reset=0` 一条路。

**`--reapply` 在这条路上救不了。** 门是被那次复位关掉的，发生在任何驱动（重新）绑定之前；拆建之后没有主机写路径，系统内读到的是一个已关的门，写不回去。能救的只有"不让 OS 侧 CM 做那次拆建"，而两个 OS 都已有实测可行的抑制：

- **Linux**：内核命令行 `thunderbolt.host_reset=0`。已实测（#54 Run C）：模块照常加载、坞在雷电侧照常被发现，但不再出现 `pci_bus … is released`，层级不拆，金丝雀 `intact`、门 `0xffffffff`、`UNLOCKED`。
- **Windows**：把 `Usb4DeviceRouter` 服务的 `Start` 设为 `4`（Disabled），`Usb4HostRouter` 保持 `3`（Manual）。**只禁设备侧、留主机侧**——两个都禁会让隧道根本不存在。三台不同主机栈（ASMedia TB4 + AD102 桌面 / Intel 原生 USB4 + GA104 笔记本 / Dell WD22TB4 坞 + Sonnet eGPU 盒 + GA102 3090 Ti 笔记本外接，[D#260](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/260)）实测：卡正常枚举并绑定 `nvlddmkm`、金丝雀 `intact`、`ssdiag` 判 `UNLOCKED`，AD102 那台 8192³ FP16 达满血张量速率的 95% 并叠加 `nvidia-smi -lgc` 做了逐时钟扫描；D#260 那台 3090 Ti FP16 +76.9% / TF32 +81.5%、FP32 对照平（也是**坞→eGPU 盒链式拓扑**的首例——坞的 USB4 路由节点在解锁会话中显示 Unknown，与代价栏吻合）。代价如实说：坞的设备路由器服务没了，坞上 USB 口失效、两个 `USB4\ROOT_DEVICE_ROUTER` 节点 `Problem 32`（Code 32 = 服务被禁用，不是故障）；GPU、卡载音频、`nvidia-smi` 均正常。`DevPnPCapabilities=0x100` 已证伪；`Usb4HostRouter` 的 `Device Parameters` 子键只有 WDF/DMA/中断三类，无复位策略面。

```bat
:: Windows：抑制（管理员）——改完重启生效，只影响本机
reg add HKLM\SYSTEM\CurrentControlSet\Services\Usb4DeviceRouter /v Start /t REG_DWORD /d 4 /f
:: 恢复默认
reg add HKLM\SYSTEM\CurrentControlSet\Services\Usb4DeviceRouter /v Start /t REG_DWORD /d 3 /f
```

**抑制的因果已单变量钉死**（#54）：`Usb4DeviceRouter\Start` 改回 `3`、其余一切不动 → 金丝雀 `cleared`、`NOT_UNLOCKED`；再改回 `4` → `intact`、`UNLOCKED`，同机同日双向完成。`Start=4` 跨重启持久（连载数日，唯一改过它的是 `sc config` 本身）；抑制态下卡**完全可用**——`nvlddmkm` `Problem 0`、CUDA/显示/卡载音频日常运行数日（含把显示器接卡上做主屏）。

**断开/重插已实测（TB4 那台，抑制态）**：会话内拔线重插不回来——卡 `Code 45` Unknown、`pnputil /scan-devices` 无效、坞断电重启隧道也不起；唯一恢复 = 重启走一次 U 盘链，**且重启后解锁仍在**（金丝雀 `intact`）。"开机前插好、不当热插拔用"由保守建议升级为实测事实。睡眠唤醒仍未测。

**第三个主机形态（维护者实测，2026-11-06/07）**：雷电 4 主机芯片 JHL8540 + TB3 坞（逍遥君 BX，JHL6540）+ GA102 3090（跨刷 VBIOS；G614JV 内置 4060 关闭防冲突，驱动 610.74）：**首次开机即保留**——UEFI 解锁后进系统卡直接识别、PLM 门开、SS 全绿；但**拔插 TB3 线（坞 ATX 强制供电不断电）→ 解锁丢失、PLM 锁回、速率回厂，金丝雀死亡（`cleared`）**。两个悬点已闭合：①栈归属——JHL8540 是雷电 4 芯片不是 USB4 芯片，系统里有 Intel 雷电控制面板、设置里没有 USB4 项目（legacy 雷电栈，无 MS USB4 CM 服务，无需 `USB4:` 采样）；②重插复位是**域复位**（canary `cleared`），不是窄写。统一读法：**复位随隧道的（重）建立发生**——固件 CM 主机开机继承 POST 建好的拓扑（不复位），重插时栈重建隧道（复位）；USB4 CM 主机则在绑定时就拆了重建（复位）。两个栈在"开机保不保留"上的差别由此而来。

`ssdiag` 0.6.13 起会打印 `USB4:` 行（两个服务的 `Start` 与 USB4 节点），报告自带配置，不必口述。

**时机路线已被排除**（#54，2026-10-30 轮，`GWASDW123` 实测）：把两个路由器服务提前到内核早期——`Start=0` + `Group=Boot Bus Extender`（`Usb4DeviceRouter` 原在 `Base` 组、`ServiceGroupOrder` 第 43 位，只改 `Start` 不会改变加载序）——隧道建立窗口从开机后 +8.06 s 压到 +4.31 s、GPU 绑定提前 3.8 s，两个服务照常 RUNNING、eGPU 正常枚举，**解锁照样丢**（门回 `0xffffff8f`、SS 清零）。所以丢锁与"接管多晚"无关，只与"接管发生了"有关：压缩 unprotected 窗口没有逃逸路径，唯一出路就是不让设备路由器做那次接管（上面的抑制）。这也与"`--reapply` 结构性无效"同源——早接管晚接管都是那次复位。

进入系统后仍可先做 [只读检查](#ssdiag)，解读随之变化：

- 金丝雀 `intact`、门仍开：接管尚未发生或已被抑制，解锁有效。
- 金丝雀 `cleared`、门锁：接管已复位，本引导内无解；下轮重跑 U 盘，或先做上面的抑制。
- SS 值仍正确但性能低：检查功耗、频率与测试方法。

**取证方法（eGPU 报告者可直接照抄）**：chain 到 Linux live USB，`modprobe.blacklist=nouveau,thunderbolt`，用 `tools/diag-unlock.py` 在无驱动环境读回；rEFInd 顶掉 ISO 自带 GRUB 可绕开机介质陷阱，Secure Boot 全程开着也能链过去，不必为此关它。完整读数与阴性对照见 `docs/evidence/issue54-tb4-4090-linux-router-reset/`。

Turing 的交棒自 2026-11-08 起默认**门开**（与 Ampere/Ada 一致；TU106 bench 端到端坐实：固件 GOP 重建与 Windows 驱动 init 都不重关它，系统内 PLM 仍 `0xffffffff`、金丝雀完好，见 `docs/evidence/tu106-1f07/keep-open-2026-11-08/`；v2.1.11 及更早默认交棒前重锁，`\tx-relock.txt` 可恢复；旧重锁在部分驱动上会诱发 init 期 PF FLR 并清零 SS，[#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99) 同卡 A/B 定谳）。门开的附带收益是系统内写入面开放：`ssdiag --reapply` 的 Turing 只读限制具备解除条件（工具侧待跟进），SS0 各 4 位速率字段的编码语义也可在系统内实验。**marker 臂的回锁值本身不是失败**：`\tx-relock.txt` 在位时实测常为 `0xffffffef`（bits 5:6 置位），而冷态默认是 `0xffffff8f`；该值随板/驱动版本存在方差——`1e04` 22G 改装卡（[#74](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/74)）实测过 `fully closed 0xffffff8f` 且 SS 仍持解锁值，`8f` 本身不作失败判读，看同一行的 `ss0/ss1`。`HANDOFF-OUT` 里 `tu10x: plm=0xffffffff`（默认）或 `tu10x: plm=0xffffffef`（marker 臂）配 `tu10x_unlock=true`、SS0/SS1 仍为 `0x88888888`/`0x8` 都是正常交棒形态（[冷态对照](cold-values.md)、[#102](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/102)、[#55](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/55)；#54/#74/#102 均已关闭 completed）。来源：[雷电平台记录](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)。

### BIOS 里关闭雷电解决"U 盘启动后进不去系统"（链载交接挂死）——两例

上面整节讲的是**进系统之后**解锁的保留（CM 拆建掉锁）；雷电还有另一个完全不同的角色：**引导交接本身挂死的参与者**。已知两例"事务全绿、卡在 OEM logo（链载交接挂死）、BIOS 关闭 Thunderbolt 后即可正常引导"：

- [#239](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/239) Thinkbook 16 Pro G5 IRX（内置 4060）：带完整日志的对照——开雷电挂 logo（事务链全绿、日志止于 CHAIN 段），BIOS 关雷电后正常引导；判读细节（显示自建失败形态、雷电角色待三份数据判别）见该帖。
- B 站评论区一例（2026-11-20，无机器细节无日志）：症状同为"U 盘启动后进不去系统"，关 BIOS 雷电解决——只有症状与解法，证据等级低于 #239，但"症状-解法"对与 #239 一致。

机理假说：雷电 host 控制器（及其 DXE/SMM 代理）是交接扇出的参与者之一，活态解锁 GPU 摆在总线上时它先撞墙；BIOS 关雷电 = 把这个 actor 从扇出里摘掉。两个轴别混：**CM 拆建掉锁**（上文，进系统后的保留问题）≠ **交接挂死**（这条，引导本身过不去）。台式机没有雷电硬件，其同类挂死与此 actor 无关（[#278](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/278)/[#279](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/279) 主板即无雷电）。

操作顺序建议：先按常规抓证据（`loader.log`；rc5 起加 `\tx-ebs-watch.txt` 可把挂死定位到具体扇出阶段——判读见 [v2.1.12-rc5 release notes](https://github.com/physicsdolphin/nv-uefi-unlocker/releases/tag/v2.1.12-rc5)）；关雷电确认能引导后，若愿意回开雷电做一轮 ebs-watch 对照，挂死阶段的读数就是雷电 actor 的直接见证。

**2026-11-22 机理级更新（[#239](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/239) ebs-watch A/B）**：rc5 `	x-ebs-watch.txt` 在该机拿到了**真机挂死的阶段定位**——TB-on 轮色块=黄（NOTIFY，已过内存图校验进入扇出）有、绿（RETURN）无、蓝（BEFORE）无（Insyde 与 AMI 一样不发 BeforeEBS 组）；TB-off 轮同路径黄→绿完整返回。两轮 ebs-watch 文件逐字节等价（各恰好一次预算内 INVALID_PARAMETER 自扰、gmm 计数逐位相同）→ **"雷电后台活动让 MapKey 失效、反复重试"类机理被证伪**；楔子=**扇出尾段（NOTIFY→RETURN 间隙），且仅 TB-on**。下一判别轮=`	x-agitate-only.txt`+`	x-ebs-watch.txt`+TB-on（区分"活态解锁 GPU×雷电" vs "loader 手术×雷电"）。证据：`docs/evidence/issue239-tb-hang-ebswatch/`。

<a id="secureboot"></a>

## Secure Boot 开着能用吗？

**本节范围**：在不关闭 Secure Boot 的前提下启动 U 盘工具。它不处理「要求 Secure Boot 关闭才能玩」的反作弊客户端——那类平台（例如校验链式度量的 ACE 系）请按厂商要求处理，本节不给步骤。

先澄清定位：这不是绕过 Secure Boot。引导链照旧被完整校验，只是信任根从「微软签发的证书」扩到「你追加的那张证书」。工具唯一能工作的时窗（驱动加载之前的固件阶段）本来就在 Secure Boot 管辖之下，本项目也不持有微软或 UEFI CA 的签名身份。

三条边界，不看清就会对后面的步骤产生错误预期：

- **自签不解决 `ssdiag`**。`pmxdrv.sys` 是 Intel/COMODO 2018 交叉签名、没有微软根，`tools/ssdiag/README.md` 的原话是 load is **expected** to be refused；它列的三个豁免（≤1607 升级来的系统 / Secure Boot 关闭 / 签名证书早于 2015-07-29）在这张 2018-10-24 签发的叶证书上都不成立。所以 SB 常开时，系统内读回走 `loader.log` 的 `HANDOFF-OUT`。Linux 侧 `tools/diag-unlock.py` 是零驱动路径，但**没有人在 Secure Boot 开启状态下验证过它**。要 `ssdiag --reapply` 修回锁，仍需临时关一次 SB。

  **一个反例数据点（2026-11-09，[bailu974 的记录](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot)）**：一台 Insyde 笔记本（COLORFIRE MEOW R15 24）在 Secure Boot 开启下，`ssdiag` 打印了这条预置警告，**但驱动实际加载成功**——末尾有 `driver service removed`，且 `0x823xxx` 的 MMIO 读数完整（读不出这些值就说明驱动没在）。机理未知（该机的 DSE/漏洞驱动 blocklist 状态未采集），**默认预期仍是"被拒"**，但请把这条警告当作提示而非判词：看末尾的 `driver service removed` 与寄存器读数是否完整再下结论，回报时请附上你的结果是"加载成功"还是"被拒"——这能把上表往任一方向推。
- **反作弊此前不保证，现有首个 Vanguard 实测通过**。自签不改 Secure Boot 的状态位；据维护者陈述 Vanguard（LoL）看的就是这个状态位。**2026-11-14 首个实测**：MOK+shim 路线下 Riot Vanguard 内核驱动干净加载、零 VAN9001/VAN9003（[D#243](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/243)，N=1 单机单游戏）——ACE 系仍未测，别把单点外推成保证。ACE 系校验 TPM/PCR[7] 链式记录，自签链的度量与原厂引导不同，不保证通过（机制与「刷进固件」的对比见下文[两条路的度量代价](#secureboot-vs-ffs)）。
- **换版本必须重签**：签名后改一个字节即失效；`*.bin` 资产不参与 PE 校验，不需要签。

### 动手之前：BitLocker

改 `db`/PK 会改变 PCR[7] 的度量内容，可能触发恢复密钥提示；改引导路径**可能**也会，取决于 BitLocker profile。两条路动手前都先挂起一次保护，并确认恢复密钥在手：

```powershell
Suspend-BitLocker -RebootCount 1
```

有一份报告用这条命令后未出现提示——那是一个机位的结果，不构成对其他配置的保证。

### 三条路怎么选

| | 路线 A：签进 db | 路线 B：MOK + shim | 路线 C：固件按文件信任 |
| --- | --- | --- | --- |
| 改动范围 | 固件 `db` 里多一张证书 | 固件密钥库不动，密钥放在 U 盘的 MOK | 固件私有信任列表多一个文件条目（PK/KEK/db 不动，多数机器连 db 都不暴露） |
| 最坏结果 | 清 PK 且该板无法恢复厂商 PK → Secure Boot 长期开不起来 | 引导失败，删掉 U 盘上的文件即回原状 | 引导失败，删掉 U 盘上的文件即回原状；信任条目残留但只信任你的文件 |
| 固件升级 / 清 CMOS 之后 | `db` 可能被重置，需重新追加 | 不受影响（MOK 在盘上） | 未测（按文件哈希绑定的话，固件更新后多半要重新加） |
| 可用前提 | 该板 Key Management 可编辑 | 有微软签名的 shim（任意发行版的 `shim-signed` 包） | 固件页面里有 `Select a UEFI file as trusted for execution`（部分 Insyde 机型有，多数机器没有） |
| 证书/密钥 | 需要（第 0 步） | 需要（第 0 步） | **完全不需要** |

**Key Management 不可编辑的板子：先看有没有路线 C 的那个菜单项，有就是最省事的路；没有则只有路线 B 可选。**

### 第 0 步：生成密钥（路线 A/B 共用；路线 C 不需要任何密钥）

需要 Linux 或 WSL。工具包：Debian/Ubuntu `sbsigntool binutils mokutil`，Fedora `sbsigntools binutils mokutil`，Arch `sbsigntools binutils mokutil`；路线 A 另需 `efitools`（或者 `sbctl`），路线 B 另需 `shim-signed` / `shim-x64`。Windows 上只能在 WSL 里做，签名也可换用 `signtool`（仅路线 A 第 1 步）。

```bash
openssl req -new -x509 -newkey rsa:4096 -nodes -days 3650 \
  -subj "/CN=NVUEFI Self-Signed/" \
  -addext "keyUsage=digitalSignature" \
  -addext "extendedKeyUsage=codeSigning" \
  -keyout NVUEFI.key -out NVUEFI.crt
openssl x509 -in NVUEFI.crt -outform DER -out NVUEFI.cer
```

`extendedKeyUsage=codeSigning` 不能省：Windows `signtool` 会校验它（`sbsign` 不校验）。私钥只留本机。路线 B 把文件名换成 `MOK.key` / `MOK.crt` / `MOK.der`，命令相同。

**这一段是本节唯一被本项目实测过的命令**（OpenSSL 4.0.3，产出 `Key Usage: Digital Signature` + `Extended Key Usage: Code Signing` + DER 导出）。后面所有涉及 `sbsign`、`objcopy`、`sbverify`、`efi-updatevar`、`sbctl` 的步骤，以及任何固件操作，**都没有被执行过**，是按工具语义写的。

### 路线 A：把证书签进主板 db（二手报告，本项目未复现）

依据只有一份二手反馈（[#145](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/145) 引用的用户说法），没有日志、没有主板型号；下面每条命令本项目都未执行。是否值得走，先看上一节「最坏结果」一栏。

1. **签名**（先 `python3 check_efi.py BOOTX64.EFI` 确认 banner 是你要的构建）：

   ```bash
   sbsign --key NVUEFI.key --cert NVUEFI.crt --output BOOTX64.signed.efi BOOTX64.EFI
   sbverify --cert NVUEFI.crt BOOTX64.signed.efi
   ```

   Windows 侧等价：`New-SelfSignedCertificate -Type CodeSigningCert` 生成证书，`signtool sign /fd SHA256 /a /f NVUEFI.pfx /p <密码> BOOTX64.EFI`，再 `signtool verify /pa BOOTX64.EFI`。

2. **交给 maker**（新版 `make-usbroot.ps1` 的 `-BootEfi` 会把签名版写到盘根与 `EFI/BOOT/`）：

   ```powershell
   .\make-usbroot.ps1 -Family ad102 -Out G:\ -BootEfi .\BOOTX64.signed.efi
   ```

3. **动固件之前先读这三条**（第 4 步是破坏性操作，警告必须放在它前面）：

   - **只追加，不替换** `db` 里的微软证书：`bootmgfw.efi` 靠 Microsoft Windows Production PCA 2011 签名，删了 Windows 起不来。
   - **清 PK 是最后手段，不是第一步**。先把非破坏性原因排掉：BIOS/管理员密码未设置会让 Key Management 灰着；Secure Boot 模式仍是 Standard（没切到 Custom）时 `db` 不接受写入；部分 OEM 板根本不开放 Key Management。确认以上都不是、且**这块板能重新导入厂商 PK**，才考虑清 PK——无法恢复厂商 PK 的板子会长期开不起 Secure Boot。
   - **先导出 PK/KEK/db** 到 U 盘备份。

4. **追加证书**（`NVUEFI.cer`，DER 格式）——只做追加的两种方式：

   - 固件 UI：Security / Secure Boot → Key Management → `db` → **Append / Append from file**。
   - Linux + `efitools`（有 KEK 私钥时）：`efi-updatevar -a -c NVUEFI.crt -k KEK.key db`

   **这不是追加操作**：`sbctl enroll-keys --microsoft` 会把你自己的密钥装成 PK/KEK/db（微软证书一并写入），需要 Setup Mode，并且**替换**厂商信任根；随后 `sbctl sign` 用的是 `sbctl create-keys` 生成的密钥，不是第 0 步的 `NVUEFI.key`。要在这条路上继续，得先把第 0 步的密钥导入 sbctl（见其文档），否则被固件校验的签名会用另一把密钥。

5. **验证**：`mokutil --sb-state` 应为 `SecureBoot enabled`；固件真正放行的证据是盘上出现新的 `loader.log` 且含 TX banner。

### 路线 B：MOK + shim（四台机报告，三台为台式机全流程）

固件只校验微软签名的 shim；你的密钥通过 MOK 在 shim 之后生效，主板密钥库不动。依据是四份现场报告：[#162](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/162)（2026-11 上旬，Secure Boot 开启、v2.1.10、RTX 30 笔记本、`ga102`，PK/KEK/db 未动）与 [#210](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/210)（2026-11-08，RTX 3080 20GB + ASUS ROG Z790，官方 v2.1.10 原包 + 自加 `.sbat` + 自签 MOK，全链绿并交棒，系统内 `Confirm-SecureBootUEFI=True`、`MokListRT` 含证书、PK/KEK/db 不变），以及 [#241 追补](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/241#issuecomment-6894516749)（2026-11-15，RTX 3090 桌面机，走下面的**哈希登记变体**并在活性 BitLocker 下三次受保护引导），以及 [D#287](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/287)（2026-11-22，MSI B450M-A PRO MAX + RTX 3070 `2484` 桌面机，官方 **v2.1.12-rc6** 加 `.sbat`（该版新节 VMA `0x140093000`）+ shim-signed 15.8 + **哈希登记变体**，Secure Boot 全程开启、EFI 全绿、同机 torch A/B FP16/BF16/TF32 1.76–1.77×）；#162 的脚本仓 [J0s3f/nv-unlock-secureboot-esp](https://github.com/J0s3f/nv-unlock-secureboot-esp) 可作参考实现，未 vendor 进本仓库。#210 相对 #162 的增量是**可复算的补节参数**（第 3 步）与"倒计时必须按键"这一步（第 7 步），两者都只来自 #210；#241 追补的增量是**哈希登记变体 + 活性 BitLocker/TPM 度量侧经验**（见第 7 步之后）。

1. 用第 0 步生成 `MOK.key` / `MOK.crt` / `MOK.der`。

2. **先检查发布件是否已有 `.sbat`**：

   ```bash
   objdump -h BOOTX64.EFI | grep -i sbat
   ```

   有输出就跳到第 4 步。v2.1.10 的发布件**没有**这个节（本项目实测：`.text` / `.rdata` / `.data` / `.eh_fram` / `.reloc` 五节，无 `.sbat`）；**v2.1.11 同样没有**（[#281](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/281) 报告者实测：同样五节、`SizeOfImage=0x72000`）；**v2.1.12-rc6 也没有**（本项目对该发布件实测：同样五节，`ImageBase=0x140000000`、`SizeOfImage=0x93000`）。#210 在真机上确认了后果：缺节时 shim 只回 `Verification failed: (0x1A) Security Violation`，连 MokManager 都进不去，且不说明原因。

3. **没有就加一个**。shim ≥ 15.3 会拒绝缺少合法 `.sbat` 的第二阶段（[Arch Wiki, Secure Boot](https://wiki.archlinux.org/title/Unified_Extensible_Firmware_Interface/Secure_Boot)，[Gentoo bug 925902](https://bugs.gentoo.org/show_bug.cgi?id=925902)）；预期这条拒绝由 shim 打印，而不是由我们打印，且不会说明原因。

   先写 `sbat.csv`：第一行是 SBAT 元数据行，必须存在；第二行描述本组件（名字自取、generation 保持 1）：

   ```text
   sbat,1,SBAT Version,sbat,1,https://github.com/rhboot/shim/blob/main/SBAT.md
   nvuefi,1,nv-uefi-unlocker,nv-uefi-unlocker,<构建版本>,https://github.com/physicsdolphin/nv-uefi-unlocker
   ```

   再算新节的 VMA：用 `objdump -h BOOTX64.EFI` 看**最后一节**的 `VMA` 与 `Size`，取 `VMA + Size` 向上对齐到 PE 头里的 `SectionAlignment`（常见 `0x1000`）。v2.1.10 的算法：`.reloc` 的 `VMA 0x67000` + `Size 0x4fc` → 对齐后 **`0x68000`**（正好等于原 `SizeOfImage`）。

   ```bash
   objcopy --add-section .sbat=sbat.csv --change-section-vma .sbat=0x68000 BOOTX64.EFI BOOTX64.grub.efi
   objdump -h BOOTX64.grub.efi | grep -i sbat       # 必须列出 .sbat，SizeOfImage 应变大
   ```

   顺序不能反：加节要针对**未签名**的 `BOOTX64.EFI`；若手上已经是签过名的文件，加完节必须重签。#210 在 v2.1.10 上实做的结果与此算法一致（`.sbat` 177 字节、节数 5→6、`SizeOfImage` `0x68000→0x69000`），可作为自查对照：三个数任一不符，说明动手的文件不是那版发布件。**v2.1.11 的对照值：新节 VMA `0x72000`**（[#281](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/281) 报告者按同一算法实测；该版 `SizeOfImage=0x72000`，加节后应增大）。**v2.1.12-rc6 的对照值：新节 VMA `0x140093000`**（本项目对该发布件实测：`ImageBase=0x140000000`、末节 `.reloc` 结束 `0x92800` 对齐 `0x93000`；[D#287](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/287) 报告者按此值实做成功——objdump 显示的是含 ImageBase 的绝对 VMA，`--change-section-vma` 用同一个值）。

4. **签名并验证**（产物名 `grubx64.efi`，shim 默认在自身目录找这个名字）：

   ```bash
   sbsign --key MOK.key --cert MOK.crt --output grubx64.efi BOOTX64.grub.efi
   sbverify --cert MOK.crt grubx64.efi
   ```

   Windows 侧等价物（#210 实做）：`signtool sign -fd SHA256 -f MOK.pfx grubx64.efi`。签完**独立重算 Authenticode 摘要**，与镜像内嵌 `SpcIndirectDataContent` 的摘要比对；只跑 `signtool verify` 不足以证明嵌进去的就是你这份文件。

5. **取 shim 与 MokManager**：发行版 `shim-signed` 包里的 `shimx64.efi.signed` 与 `mmx64.efi`。报告者用的是 shim-signed 15.8；**shim < 15.3 不做 SBAT 校验**，其行为未在本仓库验证过。

6. **制盘再替换**：先用 maker 正常制盘（会生成 7 个 `ad102` 资产与 `EFI/BOOT/BOOTX64.EFI`），然后在 U 盘的 `\EFI\BOOT\` 里替换成四个文件：`BOOTX64.EFI` ← shim、`grubx64.efi` ← 第 4 步产物、`mmx64.efi` ← MokManager、`MOK.der` ← 你的证书。**资产留在卷根，不要动**（`ga102` 约 85 MB）。

7. **首次引导**：shim 报 `Verification failed: (0x1A)` 属于预期（此时 MOK 还没登记），随后出现 `Press any key to perform MOK management` 的 **10 秒倒计时——必须在这个窗口里按键**。不按的话 shim 会静默继续往下走，loader 根本不会被执行，看起来就像"什么都没发生"（#210 实测）。进入 MokManager 后：Enroll key from disk → `MOK.der` → 设一次性密码 → 重启确认 Enroll。之后每次引导：shim（微软签）→ 用 MOK 验你的签名 → 启动 loader → 解锁 → 链回 `bootmgfw`，Secure Boot 全程开启。

**哈希登记变体（不留任何私钥；[#241 追补](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/241#issuecomment-6894516749)实测，RTX 3090 桌面机）**：不生成密钥对，MokManager 直接登记 loader 镜像本身的哈希——菜单路径是 **`Enroll hash from disk`**（不是 `Enroll key from disk`，后者要 DER 证书；选错会报 `Unsupported Format`）；该记录的 TX 镜像走的是另一条等价路：Windows 侧工具预置 `MokNew`/`MokAuth` 请求，下次 shim 引导时 MokManager 的 `Enroll MOK` 里出现待确认哈希。四个实测教训（全部来自该记录）：① MOK 登记的是 **PE/Authenticode 镜像哈希**，`Get-FileHash` 算的是整文件哈希，两者不同（PE 校验和等字段不参与镜像哈希）——**别拿文件哈希当 MOK 哈希登记**；且 EFI 里嵌了构建时间，同源码重建哈希也变，**旧登记不会授权新字节**，每次重建都要重新登记。② 首次引导未登记镜像时同样报 `0x1A`，同样要在倒计时窗口按键。③ 该机同时把**活性 BitLocker 全程保护**（XTS-AES-128、PCR 0/2/4/11 profile）走通：三次受保护引导、两次零 recovery、提升权限复查 SB/保护/PK-KEK-db-dbx 全部原样；首次设置期曾有一次明确授权的保护挂起（RebootCount=3，事后恢复并另测受保护确认航）——挂起不属常规操作，别自动延续。④ **TPM 度量侧**：同机 TCG2 日志显示 Chain-only 与 TX-skip 的差异落 PCR 4、真实 TX 与 skip（同一文件）的差异落 PCR 2/7——**不同模式=不同的度量引导序列**，skip 也可能触发 BitLocker recovery（**recovery ≠ TX 失败**），要固定一种模式用；换 shim/MokManager 版本也会改变度量，需重新接纳两条引导路径。该记录还有一份配套的机器绑定硬化构建（GPU/ROM/资产/ESP 四重钉死 + marker 从签名镜像合成不从 U 盘读），见 [community-reports](community-reports.md) 的 #241 行。**第二份哈希登记实做（[D#287](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/287)，2026-11-22，MSI B450M-A PRO MAX + RTX 3070 `2484` 桌面机）**：官方 v2.1.12-rc6 加 `.sbat`（VMA `0x140093000`）后走 shim-signed 1.58+15.8（shim 15.8）+ `Enroll hash from disk`，Secure Boot 全程开启、EFI 全绿、桌面解锁保留（同机 torch A/B FP16 1.77×/BF16 1.76×/TF32 1.76×、FP32 平）。代价观测：**该解锁引导下 Windows Hello PIN 不可用（疑 PCR7 随 shim 链变化），密码登录正常**——这不是 TX 失败，重设 PIN 前先确认自己走的是哪条引导路径。

内置 ESP 变体（**本项目不推荐；现有两份现场记录，都属于用户自担风险的偏离**）：把四个文件放进内置 ESP，免掉每次插 U 盘。变体 1（#162 报告者）：文件放 `\EFI\BOOT\`、排在 BootOrder 首位——loader 在 CHAIN-BOOT 里会跳过自己的 `Boot####` 与同一 ESP 上的 Windows 项，不循环。变体 2（[bailu974 的记录](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot)，2026-11-09）：那块 Insyde 固件**拒绝新建 `BootXXXX`**（`SetFirmwareEnvironmentVariableEx` 回 `err=87`，但改写已有项放行），于是改写 `Boot0000`（Windows Boot Manager 项）指向 `\EFI\nvunlock\BOOTX64.EFI`，并把原始项备份到 `Boot2001` 兜底；资产摊在 ESP 根（loader 只从自己所在卷的根读资产，不看子目录，84 MB 需 ESP 有余量）。共同代价：README 的明确建议是「[不要复制到系统 ESP](../README.md)」，改 `Boot0000`/改 ESP 都可能让机器起不来——动手前先备份、确认能进 BIOS 恢复；两份记录的作者都自报了备份与回滚脚本。

### 路线 C：固件自带的按文件信任（一台机记录，本项目未复现）

部分 Insyde 固件的 `Administrator Secure Boot` 页面提供 **`Select a UEFI file as trusted for execution`**：直接把指定的 `.efi` 文件加进固件信任列表，**不动 PK/KEK/db、不需要任何证书**（第 0 步整段跳过）。依据是 [bailu974 的记录](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot)（COLORFIRE MEOW R15 24，Insyde `1.07.11COLO1`）：加信任 → `Enforce Secure Boot` 改回 Enabled → U 盘 loader 被放行，全链跑完，`Confirm-SecureBootUEFI` 为开。

操作要点与坑（全部来自该记录）：

1. **先确认你的固件有这一项**——没有的话这条路不存在（不要为此去清 PK；那属于路线 A 的风险范畴）。页面里的 `Erase all Secure Boot Settings` / `Restore Secure Boot to Factory Settings` 是**立即执行的操作项**，不是开关，别点。
2. 信任**很可能是按文件哈希绑的**：该机固件连 `db`/`dbx` 都不通过标准变量暴露（读回 `ERROR_ENVVAR_NOT_FOUND`），信任列表存在私有 NVRAM 区。**换掉 `BOOTX64.EFI` 之后必须回 BIOS 重新加一次信任**。
3. BitLocker 先决：该机 C: 未加密所以没有弹恢复密钥；若你的盘开着 BitLocker，改 Secure Boot 状态前先挂起保护（见上文[动手之前](#动手之前bitlocker)）。
4. `bootmgfw.efi` 有微软签名，Enforce 之后 Windows 照常启动；最坏结果是「我们的 loader 被拒」，不是「系统起不来」。
5. 证据等级：一台机、记录者自报、未由本项目复现；且该记录里被信任的是**改过的本地构建**（见[卡键](#幽灵按键)一节）而非发布件——信任 UI 对原版发布件应同样可用，但没人测过。

<a id="secureboot-vs-ffs"></a>

### 和「把 loader 刷进固件」有什么区别？

先回答更容易被误解的那半：**自签为什么仍然不保证过 ACE**。

ACE 系（腾讯）用的是内核驱动 + 服务端策略，它可能同时看这几类输入，而公开资料里没有完整判定清单：

1. **Secure Boot 状态位**——Vanguard 类据报告只看这个，自签不改它，所以「可能有效」。
2. **引导链的 TPM 度量与事件日志**：被加载的引导程序进入 PCR[4]，而在把 Secure Boot 策略一并度量的 profile 里，PK/KEK/db/dbx 进入 PCR[7]。自签链下这两者都与出厂链不同——**连路线 A 那种「只往 db 追加一张证书」也不是「什么都没变」**。
3. **内核态驱动策略**：驱动签名者、脆弱驱动 blocklist、VBS/HVCI 运行态。

所以「SB 开着」既不是充分条件，也不是 ACE 的唯一输入；而且**两条路都没有人做过 ACE 实测**。下面的对比说明它们的代价落在不同的度量上，而不是「有一条能稳过」。

| | 自签（路线 A / B） | 固件内置 FFS |
| --- | --- | --- |
| 改的是 | OS 引导链（`\EFI\BOOT\` 下的 PE） | 固件卷里的 DXE 驱动 |
| Secure Boot | 必须把证书加入信任根（A：`db`；B：MOK） | 不参与 OS 引导链验签（FV 属于受信镜像），SB 可保持开启【设计推论，未实机验证】 |
| 刷写 | 不需要 | **需要**：UEFITool/FPT 插入 + 完整镜像备份 + 手边外部编程器（CH341A）；刷写中断电或镜像有误 → 变砖；Boot Guard 机器改 IBB 区域可能直接拒启（因板而异） |
| 适用人群 | 能进固件设置即可 | 仅高级用户、自担风险；本项目不提供安装器、不代刷（[#73](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/73) 先例） |
| 生效时窗 | 用户手动从 U 盘进 → 跑事务 | `ReadyToBoot` 回调，与 U 盘同窗口（`bootmgfw` 之前），每次引导自动重放 |
| TPM / BitLocker | `db` 改动影响 PCR[7]（若 profile 含 [7]）；自签链改变被加载镜像的度量 | 改固件 → **PCR[1] 变**；BitLocker profile 含 [1] 时无法解封，会要求恢复密钥；刷前必须先挂起 |
| 不刷写的验证捷径 | — | UEFI Shell 里 `load fs0:\NvUnlockDxe.efi`，可先验形态再决定是否刷写 |
| 证据 | 路线 B 四台机（含三台台式机全流程，`.sbat` 参数与倒计时按键均已实做；#241 追补为哈希登记变体+活性 BitLocker，D#287 为 rc6 + 哈希登记第二例）；路线 A 二手 | 仅[可行性文档](dxe-driver-ffs-feasibility.md)，**未实机验证** |

一句话：**两条路都改变了某个度量**——自签动的是引导链（PCR[4]/[7] 侧），FFS 动的是固件（PCR[1] 侧）。对「只查 SB 状态位」的客户端，两条都可能有效；对「要求出厂引导链或出厂固件度量」的客户端，两条都不保证。反作弊的判定规则是厂商侧策略，会随时调整，本文不做任何「能过/不能过」的承诺。

### 证据状态

| 项目 | 证据 | 状态 |
| --- | --- | --- |
| 路线 B（MOK + shim） | #162 笔记本报告（解锁存活到 Windows）；**#210 台式机全流程**：SB 开启、v2.1.10 + `.sbat` + 自签 MOK，全链绿 + `Confirm-SecureBootUEFI=True` + `MokListRT` 含证书 + PK/KEK/db 不变；**#241 追补台式机全流程（哈希登记变体，无私钥）**：SB 开启 + 活性 BitLocker（PCR 0/2/4/11）、shim 15.8 钉 hash、三次受保护引导（两次零 recovery）、提升权限复查全绿、TCG2 度量分析（TX vs skip 差 PCR 2/7，Chain-only vs skip 差 PCR 4）；**D#287 台式机全流程（rc6 + 哈希登记）**：SB 全程开启、EFI 全绿、FP16/BF16/TF32 1.76–1.77×、FP32 平 | 四台机（三台台式机全流程）；第 3/4/7 步按 #210 实做参数写死，本项目未在自有硬件上复跑 |
| 发布件缺 `.sbat` | 本项目核对 v2.1.10 / v2.1.12-rc6 产物节表；#210 真机确认缺节时 shim 只回 `0x1A`；D#287 在 rc6 上按 `0x140093000` 补节后通过 | 已验证（产物层 + 真机） |
| 第 0 步的 `openssl` 命令 | 本项目执行（OpenSSL 4.0.3） | 已验证 |
| 路线 A（db 自签） | 二手反馈，无日志、无主板型号 | 未复现、未执行 |
| Secure Boot 开启 + `ssdiag` | `tools/ssdiag/README.md`：expected to be refused；#210 据此选择不加载 | 未实测（#210 明确标 not verified） |
| Secure Boot 开启 + `diag-unlock.py` | 无 | 未验证 |
| 反作弊（Vanguard / ACE） | Vanguard 侧已有单个实测：MOK+shim 路线下 `vgk.sys`/`vgc` 干净加载、零 VAN 错误（[D#243](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/243)，2026-11-14）；ACE 系仍无实测 | 单点实测，N=1 |
| 固件内置 FFS 路线 | `docs/dxe-driver-ffs-feasibility.md`（设计、风险、PCR[1]/BitLocker 影响） | 未实机验证 |
| 路线 C（按文件信任） | [bailu974 记录](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot)：Insyde 文件信任 + Enforce，全链跑通 | 一台机；本项目未复现 |
| `ssdiag` 在 SB 开启下加载成功 | 同上记录：预置警告打印但驱动加载、MMIO 读数完整 | 一台机反例；机理未知，默认预期仍是被拒 |
| 内置 ESP + 改写 `Boot0000`（固件拒建新 `BootXXXX`，`err=87`） | 同上记录：`Boot2001` 兜底 + 备份脚本 | 一台机；偏离 README 纪律，自担风险 |

跑了任一条路，欢迎回报**主板型号**、**`ssdiag` 是否被拒**、**BitLocker 是否弹过恢复密钥**、**`diag-unlock.py` 在 SB 开启下是否可读**——这四项各自能把上面的行往前推一格。

<a id="ssdiag"></a>

<a id="ghost-key"></a>

## 确认倒计时被“幽灵按键”中止（卡键）

症状：没人碰键盘，loader 却打出 `CONFIRM: key Printable('\u{3}') ... ABORT`（或任意其他键值）并中止；`i_know_what_i_am_doing.txt` 的武装倒计时同样被打断。这是**固件阶段有设备在持续上报假按键**，已确认三例：

| 键值 | 来源设备 | 机型 |
| --- | --- | --- |
| `0x1A` | 卡死的 Z 键（EC） | Alienware m18 R1（[#110](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/110)） |
| `0x03`（Ctrl+C） | EC 在固件阶段持续合成 | COLORFIRE MEOW R15 24（[bailu974 记录](https://github.com/bailu974/nv-uefi-unlocker-on-SecureBoot)，含 v2.1.10 补丁 diff） |
| `Special(LEFT)` | **USB 鼠标**的 boot-keyboard 接口持续上报（[PR #220](https://github.com/physicsdolphin/nv-uefi-unlocker/pull/220)，Legion 5 Pro 16ACH6H） |

自助方法两件，都要守一条纪律——**只忽略你实际观测到的那个键值，空格/回车/超时的语义一个都不能动**（这些窗口是零写保证的守门人）：

1. **本地补丁忽略该键值**：bailu974 仓库的 `patches/nv-uefi-unlocker-v2.1.10-stuck-key.patch` 是范例（`wait_confirm`/`wait_confirm_armed`/tx_prep 的中止窗口里对观测字节免疫，其余语义不变）。#110 同思路。
2. **PR #220 的“settle + ESC-only”路线**（评审中，未合并）：CONFIRM 打开前先排空输入并等安静窗；若输入永不安静则倒计时降级为“仅 Esc 可中止”——方向安全，因为 Esc 永远不可能推进事务，假键最多被忽略、绝不可能“按一下就解锁”。

判别方法：把 U 盘上其他 USB 设备逐个拔掉二分（#220 的假键来自鼠标，拔掉即安静）；确认中止行里的键值与某颗实体键对得上则先换键盘。

**BitLocker 附带发现（#220 报告，重要）**：TPM 绑定 PCR 0,2,4,11 的 BitLocker 机器上，**中止轮与解锁轮的引导链行为不同**（中止轮跳过了第二次 native-GOP 驱动加载）→ PCR[2] 度量不一致 → Windows 弹恢复密钥（事件 24707 `MismatchedPcr=2`），温启动不受影响。这是首条“引导链**行为差异**（而非 SB 状态变化）触发 PCR 度量失配”的现场记录：在 BitLocker TPM 绑定含 PCR[2] 的机器上，要么每次引导保持同一路径（都解锁或都中止），要么先挂起保护。FAQ 此前只覆盖 PCR[7]（SB 状态）与 PCR[1]（FFS 刷写）。

## 50 系（RTX 50 / Blackwell）能解锁吗？

**现在不能。** 钳制和门本身与 40 系同形且更硬：GB206 的熔丝 `FMLA32/IMLA1/2/4=3`、`FEAT_OVR_PLM=0xfffffb8f`（仅固件内部的 PL3 层可写），主机写 3/3 静默丢弃；同 die 的 RTX PRO 6000 满速，证明是产品分割。

真正断在引导链：Blackwell **没有 SEC2 阶段**（FSP→GSP BootROM→加密 FMC→GSP-RM），也就没有 40 系那种"签名 booter 栈溢出 → ROP 写 PLM"的载体。公开世界没有 GB20x booter，debug FMC 与 prod 的密文逐字节相同、只有签名不同且无槽表。所以**换 U 盘跑也打不开门**，`ssdiag --reapply` 会正确拒绝。

可以做的：ssdiag / diag-unlock / U 盘 SAMPLE 都会记录它的 FEAT 块和熔丝（只读）。读数纪律：`READOUT_1` 是熔丝镜像、字段位不连续，且 Blackwell 的 `3` 约等于半速而不是 Ada 的 1/8；SS0/SS1 非零只是影子态，nibble 的 bit3 全清才是"熔丝生效"。机理、探索现状与还开着的面见 [blackwell-gb20x-status](blackwell-gb20x-status.md)（[#168](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/168)）。

## 怎样在系统内检查或修复解锁？

Windows 使用 release 页单独附带的 `ssdiag-x86_64-windows.exe`（zip 内不含它，理由见[杀软误报](#av-fp)）。以下简写为 `ssdiag`：

```text
ssdiag --inventory
ssdiag
ssdiag --select 2504
ssdiag --reapply
```

`--inventory`（0.7.0 起，0.7.1 扩容）打印**免管理员、只读**的硬件采集块：OS/CPU（含 CPUID 拓扑与 F/M/S 判读——ES/QS 烂名牌也能读出家族型号与核线程数）/主板/BIOS/机箱形态/内存插槽分布（容量+频率，SMBIOS Type 17，不读序列号）/磁盘型号与容量/分区布局（偏移、文件系统、盘符、余量）/其他 PCI 设备清单，以及每张卡的设备 ID/子系统 ID/VBIOS/驱动版本/GSP 模式（经驱动绑定列表定位，无换卡 ghost），并给出家族准入判定（IN-SCOPE/OUT-OF-SCOPE）与预检提示（Secure Boot 开启、Turing 未开 GSP 模式、笔电掉锁风险）。这就是 issue 模板让你粘贴的那一块——发帖前过目并脱敏你不想公开的内容；它不读主机名、用户名或序列号。完整运行（寄存器读回）仍需管理员；`--select` 按 DeviceID 筛选。`--reapply` 是写操作，仅在门仍开放时重写 SS0/SS1 并读回，不写 PLM。Turing 保持只读。Linux 在源码树根目录运行 `sudo python3 tools/diag-unlock.py`，可用 `--bdf` 指定实际目标地址，也提供 `--reapply`；对应的免 root 采集块为 `python3 tools/diag-unlock.py --inventory`（sysfs/DMI/nvidia-smi 只读，输出格式与 ssdiag 相同）。

若遇到 `MAP_PHYS win32 error 5`（映射被拒）：0.6.3+ 会输出一次前置分类清单（HVCI 运行态 / 内核态杀软过滤驱动 / 驱动加载被拒）。**杀软的「暂停保护」或退出用户态进程不会卸载其内核钩子**——已两例实机闭环（Kaspersky，卸载后痊愈），详见 [平台门与 MAP 被拒排查](ssdiag-platform-gates.md)。

**运行 ssdiag（或任何直读 PCI config 的工具）瞬间整机蓝屏**：蓝屏码 `0x1EA SECURE_PCI_CONFIG_SPACE_ACCESS_VIOLATION`（[#244](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/244) 首个在役样本）。机理（[微软文档](https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/bug-check-0x1ea--secure-pci-config-space-access-violation)明文）：VBS 的 "secure PCI" 层禁止 VTL0 直接映射 MCFG/ECAM 区间读 config space，强制在 hypervisor/VTL1 的 EPT 层——**映射能成功、首次读取即整机蓝屏**（不是可恢复的 `MAP_PHYS error 5`）；WinDbg 四参数 = 故障指令 opcode（#244 实例 `0x8b`=MOV r32,r/m32）/ 设备 RID / config 偏移 / 故障 RIP。

分两层看，**别把内核隔离（HVCI）当凶手**：HVCI/VBS 运行本身不触发 0x1EA——#188（VBS+HVCI on，SB off）/#204（HVCI ON，SB off）跑通**全量 probe**（#218 只跑过 `--inventory` 免驱动层，不作全量证据）。触发对按文档是 **SDEV（Secure Devices）ACPI 表在场 + VBS running**：自 Windows 10 2004（19041.423）起，[MS 排障页](https://learn.microsoft.com/en-us/troubleshoot/windows-client/performance/stop-error-lenovo-thinkpad-kb4568831-uefi)明文封堵非受支持的 config space 访问，**"解析 MCFG 表并映射到虚拟内存"被点名**；[ESS 文档](https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/windows-hello-enhanced-sign-in-security)给出执行方——**SDEV 表由 Secure Kernel 解析并强制**，不支持方式的读写"result in bug checks"；[CHIPSEC 文档](https://chipsec.github.io/installation/InstallWindows.html)同一条件公式。公开案例都是普通装机、非 OEM 镜像专属：Framework+Speccy、LG Gram+Speccy（cpuz149_x64.sys）、AMD Ryzen Master、Vanguard vgk；我方自有观察：**2024+ 部分华硕 OEM 镜像（天选6 实测）上 Intel FPTW64 读 SPI-flash MMIO 同样直接蓝屏**（SDEV 限制是否覆盖 MCFG 以外的 MMIO，公开文档未写明）。

**为什么重装后消失（更正：SDEV 表在固件里，重装系统不动它）**——消失必然出自 VBS 侧或固件选项侧，按可查性排序：① 重装后 VBS 实际没跑（**WMI `VirtualizationBasedSecurityStatus==2` 或 msinfo32 为准**——注册表值在部分 VBS-running 机器上缺席，不能当反证，#244 实测）；② BIOS 的 ESS 类选项被重置/刷 BIOS 改动——**"Enhanced Windows Biometric Security"（Lenovo，Security > Virtualization）/"Enhanced Sign-in Security"/"Enhanced Hello" 类选项**，关掉即撤 SDEV 限制（排障页明示 workaround）；③ OS 侧未公开前置（ESS provisioning/生物识别 trustlet 等——纯假设）；④ OEM 镜像与重装 build 不同。此前"OEM 镜像 provision SystemGuard 注册表键"的推测**作废**（无公开文档支持该键与 secure PCI 的关联；#244 的 `SecurityServicesRunning=[2,3,4,5]` 按 secured 风格机器四件套共现处理）。

ssdiag 0.8.2 起：`GetSystemFirmwareTable('ACPI','SDEV')` 在场时按 VBS 读取结果分支——`==2`（或 SystemGuard 键在场）→ 部署前拒绝；**注册表 VBS 值缺席（unverified）→ 也拒绝（fail-closed）**——#244 的 T14s 实测 WMI 报 2 而该注册表值不存在，0.8.0/0.8.1 的 `==2` 正判据在那台机器上静默失效（报告者 ASSESSMENT 抓出后修正）；仅 VBS 明确 off/未运行才放行。无覆盖旗标（代价是整机蓝屏不是拒绝读）；SystemGuard 键保留为保守二级信号；`--inventory` 不受影响。判别采集配方（一个天选6 双态样本即可钉死）：

```powershell
Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard |
  Select-Object VirtualizationBasedSecurityStatus, SecurityServicesRunning
# SDEV 表：ssdiag 平台门 "SDEV table (ACPI)" 行（只读固件表查询，不会触发蓝屏）；
# BIOS 里对照 Enhanced Windows Biometric Security / Enhanced Sign-in Security / Enhanced Hello 类选项状态
# 蓝屏样本：minidump 的 Arg1（opcode）/ Arg2（RID）/ Arg3（偏移）
```

SS/PLM 读回请走 EFI 侧只读采样（#244 报告者即此安排：Windows 侧 PnP/CUDA，读回留在 UEFI）。详见 [平台门与 MAP 被拒排查](ssdiag-platform-gates.md) §2a。

| 读回结果 | 判断与下一步 |
| --- | --- |
| SS0=`0x88888888`，SS1 低四位=`0x8` | 覆盖值有效；再用匹配负载验证性能 |
| Ampere/Ada 门开、SS 值不符 | 可修复选择器，修复后再次读回 |
| Turing 门开（`0xffffffff`）、SS 值正确 | 默认交棒形态（2026-11-08 起，与 Ampere/Ada 一致）；系统内写入面随之开放 |
| Turing 门重锁（`0xffffffef`/`0xffffff8f`）、SS 值正确 | `\tx-relock.txt` 在位的正常形态（回锁值随板方差，bits 5:6 置位 ≠ 冷态默认 `0xffffff8f`），或平台/驱动重锁事件——用金丝雀区分，见 [冷态对照](cold-values.md) |
| Turing 交棒门重锁、系统内 SS 被清零 | 旧载具（≤v2.1.11）的默认重锁诱发了驱动 init 的 PF FLR（[#99](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/99)）；换 v2.1.12-rc3+ 重放，别再在旧版上复测 |
| 门关闭且 SS 无效，或卡已失电 | 修复工具不能重新开门；按恢复流程重放 |

将系统侧结果与 `loader.log` 的 `HANDOFF-OUT` 对照。若驱动初始化异常，同时检查设备管理器、nvlddmkm/Xid 与电源状态；不要只用 EFI 的成功状态替代系统内验证。

交棒后无收益时，按以下读回区分问题；这些是排查方向，需结合电源和驱动日志确认：

| 系统侧现象 | 排查方向 |
| --- | --- |
| Ampere/Ada 门和选择器均回到锁定基线 | 断电或完整复位 |
| 门仍开，选择器恢复出厂值 | devinit 重写覆盖值（见 [回锁机制](#relock)） |
| 选择器被重写，同时出现 Code 43 或驱动复位 | 驱动初始化失败及恢复路径 |
| 选择器仍有效但性能低 | 频率、功耗墙与工作负载 |
| 卡在设备管理器报 Code 12（`CM_PROB_NORMAL_CONFLICT`），loader/`HANDOFF-OUT` 全绿 | 开机时刻的资源仲裁 artifact（[#165](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/165) 十二轮定谳：条件=多卡在场+第一枚举位卡被解锁；A4G/devnode 残留/卡身份/足印改变均已排除）。**自救：设备管理器把冲突卡禁用→启用**——仲裁重触发即愈，且 PnP 重载不走 FLR，解锁态原样保留（[#196](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/196) 同操作的第二用途）；或预防性地解锁第二枚举位卡（多卡时） |

ssdiag 从 OS 发现 BAR0，不接受手填物理地址；会按需唤醒停泊卡。内嵌的 `pmxdrv.sys` 经校验，以自有 `SSDIAG_PMXDRV` 服务临时加载，退出时清理；若 nvoc 等已有可用设备则借用。部署驱动通常需要管理员。**若直跑报 `OpenSCManager win32 5`**（本机安全策略拒绝服务管理器，[D#248](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/248) 实证）：先确认已提权；仍失败就手动把随包 `pmxdrv.sys` 装成服务并启动，再跑 ssdiag 走借用路径（读回正常、不受该策略影响）。其安全模型与权衡单独成节：[pmxdrv.sys 的安全模型与权衡](#pmxdrv)。更多寄存器范围及驱动说明见 [ssdiag 文档](../tools/ssdiag/README.md)。

<a id="vgpu-vf-guest"></a>

## vGPU / SR-IOV VF guest（如 geforce-sriov）里跑 ssdiag，读数能信吗？

**不能作为宿主卡的解锁判据。** vGPU 分两型：SR-IOV VF 直通（[geforce-sriov](https://github.com/noon-at-cgn/geforce-sriov) 类栈）里 guest 拿到的是**虚拟化寄存器窗**——MMIO 是真硬件访问，但按 per-VF 过滤路由；mdev / GPU-PV 型（vgpu_unlock 世系、Hyper-V GPU-PV）则连真 MMIO 都没有，BAR 访问被 hypervisor trap 后转给宿主 mediator。首个现场样本（宿主 RTX 3090 Linux + 上游 gpu-unlock 解锁 + geforce-sriov，guest 见 `10de:2230`「RTXA6000-8Q」）测得的 VF 窗形态（[证据与解码](evidence/sriov-vf-guest-2230/findings.md)）：

- **扩展配置空间全 FF**（0x0100 起即 all-FF）：ReBAR/link 探针自动跳过（<0.7.2 在此静默崩溃，0.7.2 修复）；
- **除 `PMC_BOOT_0` 外全部寄存器硬线回 0**：不是 badf——badf 编码器在 PF 的 PRI 上，VF 窗没有。FECS/PLM 属 PF 裸 PRI 状态，guest 里没有可读投影；`closed (non-standard)` 标签是填充效应，不是门状态。0.7.3 起对这种形态打印 `NOTE: ... virtualized / SR-IOV VF window`；
- **BOOT0 与设备 ID 都是固件合成的 profile 身份**：`0x1fa52a02`（die 段 0x1fa，无已知 die 段）；guest 枚举的 "RTX A6000" 不是宿主那块 3090 硅的身份——伪装两层：宿主 identity hook 放出 A6000 profile，PF 固件再把该身份投进 guest；
- **有效的 VM 数据点只有 VFIO/DDA 整卡直通**（[#135](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/135) 先例，guest 拿真 PF MMIO）。

要验证宿主卡的解锁状态，探测点在**宿主**：Linux 宿主跑上游 gpu-unlock 的验证路径或 `tools/diag-unlock.py`；Windows 裸机跑 ssdiag。guest 里读出什么都不构成宿主证据。

<a id="pmxdrv"></a>

## pmxdrv.sys 的安全模型与权衡（必读披露）

`ssdiag` 需要读 PCI BAR0 寄存器，而 **Windows 没有任何受认可的用户态通道**可以做到这一点——读 MMIO 寄存器必须有 ring-0 物理内存传输载具。对照：Linux 上 root 即一切，对 sysfs 资源做一个 `mmap` 寄存器就在眼前（`tools/diag-unlock.py` 正是这么工作的，零第三方内核模块）；Windows 没有等价路径。

**为什么是 pmxdrv.sys**：经典载具 WinRing0 已被微软列入漏洞驱动封禁清单，HVCI 座位在加载前直接拒收；在现存安全模型下、绝大多数真实装机上仍可加载的，是 Intel 签名的 `pmxdrv.sys`（PCILeech 系谱）。我们对它做的收窄：SHA-256 内嵌钉死、按需解压到 `%TEMP%`、自有服务名 `SSDIAG_PMXDRV`、退出即停即删、0.4.9 起 preflight 前置告知（封禁清单/HVCI/EDR 状态）。默认只读，`--reapply` 是唯一显式写路径。

**这个选择是行业日常，不是特例——作者的经验**：在系统内刷写 UEFI BIOS 本来就是主流厂商支持的维护动作，其工具链全部强依赖同类机制——AMI 的 AFUWIN、Insyde 的 InsydeFlash、Intel 的 CSME 刷写工具，都靠内核驱动打开 UEFI 固件对应的 PCI MMIO 物理映射进行读写。作者常年使用这三套工具做必要的 BIOS 维护，对这一机制早已是日常。在这个类谱里：AFUWIN 的驱动已被微软封禁清单收录；Intel 自家 CSME 签名的传输载具反而是其中安全模型最平凡的一个——厂商签名、随维护工具正常分发、无任何特殊仪式。`pmxdrv.sys`（Intel 签名）正是这个日常类谱中仍然存活的一员：选它不是伸手去够异类武器，而是使用行业标准机制中尚存的、有厂商签名的那一份。

**我们用它的什么机制（透明公开）**：`ssdiag` 只调用该驱动的两个接口——`MAP_PHYS`（0x222AB8）与 `UNMAP_PHYS`（0x222ABC），即**物理内存映射与解除映射**；此外不触碰驱动的任何其他能力。每次运行至多建两个映射窗口：一页 ECAM 配置空间（ReBAR/链路探测）与 BAR0 全窗（4096 页 = 16 MiB，基址由 OS/配置空间自动发现，不接受手填）；此后**所有寄存器访问都是这块映射窗口上的普通内存读写**——读即 MMIO 读，`--reapply` 的 SS0/SS1 写即同一窗口内的 MMIO 写，退出即解除映射。要如实说明的是：该驱动的底层能力是**无界的任意物理内存读写**，上述边界（只映射这两个窗口、地址全部来自设备自身配置空间）收窄在我们的代码里，**不在驱动里**——驱动本身无法被收窄，任何持管理员权限者都可以用同一驱动做别的事。这正是"拒绝权"必须由你自己行使的原因。

**缺陷态如实陈述**：该系谱存在已知漏洞的历史构建；微软封禁清单**没有** pmxdrv 条目——外部没有任何东西替你拦截它，唯一的闸门是你自己的判断；加载会留下 SCM 事件 7045 永久审计记录；EDR 可能告警。这是一个能力，不是行为：设计收窄它，使用决定它。定谳过程见 [#40](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/40)。

**你有两项权利，同等重要**：

1. **亲眼见证权**——运行 `ssdiag` 直接读 FEAT PLM/SS0/SS1/READOUT，亲自确认解锁状态，不依赖任何人的转述；
2. **拒绝权**——你有权在你的任何或全部物理机器上拒绝这个内核驱动，并且照样获得验证：loader 自身的 EFI 阶段读回（预引导、零驱动）、GEMM 输出哈希对照原厂基线、`nvidia-smi` 遥测；Linux 用户另有零驱动的 `diag-unlock.py`。不跑 `ssdiag` 的实测报告（如 [#135](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/135)）在兼容矩阵中同样按完全验证收录。

不装、不用、不理解都不影响解锁本体——`pmxdrv.sys` 只服务于"系统内零依赖零负载测试，一手证据确诊"这一件事。

<a id="av-fp"></a>

### 杀毒软件报毒怎么办？

**`ssdiag` 从来不是主包的一部分，zip 里也不再包含它**（v2.1.10 起原地调整）：它只是 release 页面上一个可选附件，而它是唯一会被报的东西。

报毒的形状本身就是判据。`ssdiag` 是未签名的 Rust 宿主，`.rdata` 里嵌着一整张内核驱动 PE 映像（文件偏移 `0x2C7C9`）——这是按静态包络打分的启发式引擎最不喜欢的组合。2026-11-05 的两份报告都指向同一个 zip，而两处写的微软名字还互不相同（`Trojan:Script/Sabsik.EN.A!ml` 与 `Trojan:Script/Wacatac.H!ml`）；第三方扫描里真正命中的是 BitDefender 与其引擎 OEM 的同一族通用名（ALYac / Arcabit / eScan / Emsisoft / GData / VIPRE 的 `Trojan.GenericKD.82039403`，Bkav `W32.Malware.*`，CTX 的容器启发 `Zip.trojan.generic`，Elastic 的 `Malicious (moderate)`）——**一组引擎报同一个泛化名、微软全库不报，就是典型误报形态**。本机复测：Defender 签名 1.463.142.0（2026-11-05 更新），对官方 zip 扫完 `found no threats`。

**拿掉 `ssdiag` 之后仍有残留**：复测里只剩 Bkav 一家（`W32.Malware.AEFAF8C4`），命中对象是 Linux 侧的 `assets-x86_64-linux`——同样是未签名的 Rust 二进制，而 ELF 根本没有 Authenticode 签名这条路（`W32.` 前缀套在 ELF 上是 Bkav 的惯例，不是判据）。Windows 用户不加载这个文件，可以完全无视；Linux 用户本来也不跑这套引擎。

判定与处置：

1. **先核对 hash**——**校验分两层**：zip 本体不在 `SHA256SUMS.txt` 里（sums 盖的是解压后的成员文件），zip 用 release 页的 GitHub 资产 digest 校验，解压后逐成员对 `SHA256SUMS.txt`；`ssdiag` 的 hash 同时钉死在二进制内、运行时自校验。hash 对得上就排除"下载被换包"。（两层各管各的口径由 [D#223](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/223) 实测澄清，本行旧表述"zip 在 sums 里"有误已修。）
2. **主包照用**——解锁本体（`BOOTX64.EFI` + 生成脚本 + `assets`）不因这条误报受任何影响，跳过 `ssdiag` 也能完成解锁与验证（EFI 读回、GEMM 哈希、`nvidia-smi`）。
3. **要用 `ssdiag` 就放行**——它是绿色免装工具，不留服务、退出即删，把下载的文件或目录加进 Defender 排除即可；报 `Generic`/`!ml` 名的都按误报处理，只有报出具体木马家族名才值得复查。

<a id="bridge-bme"></a>

## booter status 0x89 是什么？

`0x89` 属于 Booter 的 WPR 描述符校验错误。GPU 上游 PCI 桥未开 Bus Master 时，设备 DMA 无法正确读到 WprMeta，可出现该错误。

loader 会为 GPU 上游路径中需要的桥开启 Bus Master 并读回，结束时还原；日志包含 `DMA-PATH` / `BRIDGE-BME`。遇到错误时保留完整日志，检查这些行与 GPU 状态，不要仅凭状态码自行归因或改 BIOS。

SVM、VT-x 和 VT-d 在已验证平台上不需要关闭。任务管理器的“虚拟化”不是 PCI Command 的 Bus Master 位。部分 AMI 机型 BIOS 的 "BME DMA Mitigation" 选项即是桥 BME 清零态的固件侧来源，保持默认即可，loader 不依赖它的开关状态。机制与证据见 [桥 BME 根因报告](bridge-bme-root-cause.md)。

<a id="no-boot"></a>

## 选 U 盘一闪回到启动菜单或直接进系统，盘上没有 loader.log？

先说结论：**EFI 根本没被执行，什么也没写。** loader 在读取任何资产之前（约 110 ms）就开始写 `loader.log`；盘上没有这个文件、也没有 `tx-log.txt`，就说明一条指令都没跑。解锁本身是易失的，即使真跑过也随断电消失。

按顺序查，命中即停：

1. **盘上没有 `EFI\BOOT\BOOTX64.EFI`。** 固件对可移动介质只自动引导这一条默认路径；`BOOTX64.EFI` 放在盘根，或整包被拷成 `U:\usbroot\`，都不会被自动引导。重跑 maker 再复制**内容**（布局里已带该路径），或手工把 `BOOTX64.EFI` 放进 `EFI\BOOT\`。
2. **Secure Boot 还开着。** 多数主板只切“其他操作系统”不够：进 Key Management 删除 PK/KEK/db 全部密钥，重启后用 `Confirm-SecureBootUEFI` 确认返回 `False`。
3. **从非 `UEFI:` 前缀的条目进了 CSM。** 启动菜单里选带 `UEFI:` 前缀的那条，顺手关掉 Fast Boot。
4. 以上都正确仍无 `loader.log`：换 USB 口（优先板载后置口）、换 U 盘；仍不行就是固件侧限制，回报时附主板型号与 BIOS 版本。

### 盘上有 `loader.log`，但是 0 字节

**不要用字节数倒推死在哪儿。** 一部分固件（AMI/Insyde 上已多次实测）的 `Flush()` **不提交 FAT 目录项**，只有 `Close` 会提交。所以在这些板子上，「文件已建、0 字节」只说明**这份日志从来没有被 Close 过**，不说明执行停在哪一行：死在中止屏、死在准备段、甚至事务跑完但没走到收尾，看到的都是 0 字节。对应对照：

- [#170](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/170)：从 U 盘整盘镜像里按 BPB 解析 FAT、扫描簇链，从「0 字节」还原出两份完整 261 行日志——事务实际跑完了 detach、AUTH-SCAN、scrubber、WPR2 layout，屏幕却早在中途定格；「挂在 DETACH」的判读因此作废。
- [#198](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/198)：新构建改成「创建即 Close 提交目录项」的写法后，在这种固件上 **reopen 反而失败**，logger 整轮降级为只写屏幕，文件永远空着；而同机旧构建（单句柄 + `Flush` + 交棒时一次 `Close`）能正常写出 47–56 KB。

因此排障顺序是：

1. **别调 `tx-flush-lines.txt` 想救它**（`\tx-flush-lines.txt` 只改刷写节奏，改不了「谁来提交目录项」）。
2. 用逐提交点 `Close` 的构建跑（`2.1.11-diag3` 起；`diag4/diag5` 已把点火段也逐行化），日志会停在真正的最后一行。
3. 已经拿到 0 字节的场景，按 #170 的整盘镜像 + 簇链扫描还原正文；正文只存在于 console 的情况，屏幕照片/串口输出也算证据。
4. 回报时写明构建版本与 SHA——同一个「0 字节」在旧构建和新构建里含义不同。

盘能起来、能出日志之后的失败才轮到事务排障，见[恢复说明](#recovery)。

### 双 ESP，链到了死的那个（`0xc000000f` / `0xc0000225`）

重装过的机器可能留着**两个 ESP**：一个在用，另一个的 ESP 里还躺着过期的 `bootmgfw` 和指向无效位置的 BCD 存储（`{default}`/`osdevice` 指到没有 `\Windows` 的数据卷）。链式启动的 `bootmgfw` 会采用**它所加载卷**的 BCD 存储，Windows 随后一直持有该存储——链错 ESP 的表现就像 Windows 装坏了，而不是链的 bug。已有两台实机：[#128](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/128)、[#142](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/142)。

**v2.1.10 起自动解决**：loader 遍历固件自己的 `BootOrder`，HD 短格式选项按**分区身份**匹配（真实固件写的就是这个形状），链 Windows 实际被引导的那一项；旧的枚举序扫描只作最后回落。

判据看日志：

```
CHAIN-BOOT: firmware resolve: [0] Boot0001 short-form HD identity match on Handle(...) - not by path alignment
CHAIN-BOOT: firmware resolve: [0] Boot0001 -> '\EFI\MICROSOFT\BOOT\BOOTMGFW.EFI' on the firmware's volume - chaining its choice
```

出现 `identity match` 且全程没有 `volume scan fallback`，即解析器采用了固件的选择。只有旧版构建才需要手改 BCD；v2.1.10+ 上的变通可以还原。`CHAIN-BOOT:` dump 会打印完整 `BootOrder`、逐项解码设备路径，并给每个卷打 `hd(partN,<guid>)` 标签，选错一次引导即可看见。

### Legacy/MBR 系统盘：chain 无目标 → MBR2GPT 迁移配方（[#122](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/122) P3 实测）

链载目标必须是 **UEFI+GPT 系统盘上的 ESP**；Legacy/MBR 安装的系统盘没有 ESP，chain 段以 `NOT_FOUND` 结束（loader 事务本身可以全绿——#122 rc1 即此形态：`unlock=true` 但 `chainload NOT_FOUND`）。这不是 loader 可绕的限制：链载机制的本质就是找到 ESP 上的 `bootmgfw.efi`。

**迁移配方（#122 报告者 eway800 实测跑通）**：

1. 管理员命令行先干跑：`mbr2gpt /validate /allowFullOS`——通过再 `/convert`（硬前置：系统盘 ≤3 个主分区、BitLocker 先挂起、Windows 1703+；转换会征用一块主分区变 ESP 并配置好 `bootmgfw`）；
2. 固件从 Legacy/CSM 切到纯 UEFI 启动（CSM off）；Secure Boot 关（OS Type=Other OS，unsigned loader 必要条件）；
3. 清理固件 BootOrder 里因转换而失效的 Legacy HD 死条目，别让它排在 U 盘/Windows 前面；
4. 用 v2.1.11+ 的 maker/U 盘重跑——chain 段应能按 Pass 0/0.5 找到新 ESP。

**两条边界事实**（同一实测）：①**loader 引导介质本身不需要 GPT**——rc2 从一块 MBR 数据盘的 FAT32 分区经固件自动 "UEFI OS" 条目引导成功；需要 GPT+ESP 的只有**链载目标**（Windows 系统盘）。②切换到纯 UEFI 后老 AMI 板（Z97-K，最老 AMI 台架）全链跑通——固件能力在本机先验（能从 FAT32 卷跑 EFI loader 即具备 UEFI 引导能力）。

<a id="handoff"></a>

## 解锁日志全绿，但 Windows 起不来（`0xc000009a` / 恢复环境）？

`0xc000009a` = `STATUS_INSUFFICIENT_RESOURCES`，发生在 winload 阶段（能看到 “按 F8 / F1 / Esc” 的恢复屏即已跑到这一步）。winload 有两道互相独立的天花板，均已通过对 `winload.efi` 的静态分析证实（2026-11-12，[#146](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146)）：

1. **描述符条数——决定性的一道。** winload 把继承内存图的每一条描述符装进一个**恰好 1024 条的静态池**（`winload.efi!BlpMmInitialize`：`MmGlobalMemoryDescriptorCount = 1024`，48 字节/条，无动态扩容）；池耗尽即以 `0xc000009a` 杀死启动。实测分界：998 条能启动，1005 条不能。碎片按条计价、与大小无关——4 KiB 的洞与 80 MiB 的洞花同一个槽。现场中招的固件（Gigabyte B550 族）要么稳定高位 ~998，要么在同样的零写 boot 间摇摆 448↔1005 条；map 是固件自己的产物，EFI 应用既不能合并也不能改型——**断电重试**是正当绕行（同板 compact-map boot 能过），换 BIOS 版本才是真修。950 条以上日志会打硬 WARNING。
2. **最大单块——原 #146 机制。** winload 的启动预留只从单个描述符里取、不拼接；ExitBootServices 不做转换，驻留的资产池会把低 4 GiB 唯一的大块劈开，总空闲再多也没用。

判据直接读 `CHAIN-MEMMAP` 摘要行的 `(... N descriptors ...)` 总数，两条天花板看是哪条：**> ~1000** = 条数顶板（重试 / 换 BIOS）；**≤ ~1000 且最大块被砍半** = 尺寸顶板（下面的修复对象）。条数高时**先问走的是哪条引导路径**：在中招的 B550 固件上，**F12 / BBS popup 引导菜单**比 **DEL → BIOS → Save/Exit → Boot Override** 多留 ~370–475 条图描述符（popup 以原生分辨率重建 console、其分配撕裂内存图），足以把 BIOS-override 能启动的板推过池线。绕行按序：走 BIOS override 流引导；升级 BIOS（首台板已实证）；或把布局永久装进内置 FAT32 分区、BIOS 固定 BootOrder 引导——菜单路径永不执行（见[交棒低位内存 §0c](handoff-lowmem.md)）。

**v2.1.11 起默认在交棒前把资产池还给固件分配器**，代价为零（过了事务就再没人读资产）。判据看日志：

```
CHAIN-MEMMAP: ... largest block 2604 MiB (...)     <- 修复后；失败态这个数被砍半
TX-ASSET-CACHE: cleared 9 entries, 84388324 bytes handed back to the firmware allocator
```

- 老版本遇到本症状：确认盘根有 `tx-free-assets.txt`（零字节）再跑一次；v2.1.11+ 该标记已是空操作。
- 释放自 v2.1.11 起无条件执行（2026-11-25 起 `tx-keep-assets.txt` 对照臂亦退役，日志不再有 `cache left resident` 分支）。
- 机制、三跑内存图与规范/EDK2 依据见 [交棒前的低位内存](handoff-lowmem.md)。

<a id="reconnect"></a>

## 事务成功后为什么会复位或黑屏？

部分 AMI 固件会在 GPU 事务完成后的 GOP 重连中卡死或复位。loader 对断开后仍存在的无路径 GOP 使用非递归厂商重连，链式加载前跳过 NVIDIA 显示功能；运行日志即时写盘。

若仍停在 native-GOP 重建阶段（日志 `NATIVE-GOP` 行内）或该阶段复位，完全断电恢复后，在 U 盘根放 `tx-no-reconnect.txt`（或 tx.ini 写 `reconnect = off`）跳过目标 GPU 重连。Windows Boot Manager 可自行建立控制台；跳过重连不代表跳过系统内验证。

Turing 若返回 `0x91`，或 EFI 成功但 GSP 交棒蓝屏，查 [兼容性矩阵](compatibility.md#device-16)，不要把所有黑屏都归为 GOP。平台证据见 [AMI 报告](ad106-asus-z790-windows-unlock.md)和 [X470 记录](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70)。

**事务失败时屏幕可能保持黑屏，但机器是活的。** `native-GOP` 重建只在成功路径执行；失败路径只做回滚（恢复原厂签名→FLR→链回引导），断开过的 console 不会重建。实测（[#191](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/191)）：失败的 HIGH staging 轮之后系统照常启动、SSH 可用、驱动与桌面正常，只有内屏是黑的。单显笔记本站到这种画面按"黑屏但活着"处理——等它进系统用 SSH 或外接屏确认，或直接断电重来；不要因为黑屏就中途拔盘断电。

**日志里的 `NATIVE-GOP: no gop child on target` 通常不是显示故障，而是"控制台本来就不在这张卡上"**（[#261](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/261) 首个形态记录）。看同一份日志的三处就能判：①`GOP-PROBE` 只有 1 个 GOP 句柄、路径是核显（`PciRoot(0)/Pci(0x2,0x0)`）且带 EDID；②目标卡行 `DETACH: ... is not on the target GPU (multi-GPU) - left untouched`；③目标卡 pre-claim VGA 子集 `0x0`。这说明固件层控制台归核显所有、`DETACH` 根本没拆过目标卡的显示路径，native-GOP 重建无事可做。rc4 及更早的日志仍会跑一遍重建——从影子 ROM 把本卡原厂 GOP 驱动 `LoadImage`/`StartImage`/binding `Supported`+`Start` 全部跑绿，但驱动在这个状态下不发布 GOP child（它认为本卡没有自己拥有的显示路径），三个探针如实报空并打印 `native-GOP recovery failed`；**main 起（尚未发版）**，`DETACH` 会记下"目标卡本引导没有任何 GOP child"，默认路径直接跳过重建并记 `TX: native-GOP rebuild skipped - the target had no GOP child at detach ...`，不再出现那条 `failed` 行（`\tx-gop-disconnect.txt` 强制断开全部 GOP 的模式仍照旧重建，保持其原有诊断语义）。此时控制台保持固件原样、bootmgfw 照常接管，`POST-GOP` 读回证明解锁不受影响——属于结构化降级里的**信息性**形态，与 #236 的 `console rebind failed NOT_FOUND`（child 已建、挂回 console 失败）和 #188 的 `no rom image`（影子 ROM 都没有）都不是同一型。要让重建真正发生，只能让固件在开机时就以 dGPU 为启动显示（BIOS 级独显直连 / 关核显），EFI 阶段改变不了显示归属。

### 解锁后一跑负载就报 CUDA 错误？先撤超频和降压

解锁让张量发射率翻倍，**同一功耗包络内**的电流尖峰、VRM 与散热应力随之上升；原本刚好稳住的超频/降压工作点会在新的电流形态下失效。温度越高、漏电越大、同一频率需要的电压越高，所以"以前一直稳"不构成证据。

**实证**（[#147](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/147)，3060 12GB）：NVFP4/AWQ 权重做指针转换时抛 `CUDA error: an illegal memory access` 与 `HostBuffer.truncate failed`，`--disable-cuda-malloc --disable-pinned-memory --lowvram` 全无效。撤掉 0.875 V 的 UV V-F 曲线、核心回默认频率后一切正常。**这不是工具缺陷**：解锁只是更快地撞上原有工作点的余量。

排查顺序：①先**关掉解锁**在默频默压下烤机——默频都不稳说明卡本身需要降频；②解锁后仍不稳，核心与显存**同时降 200 MHz**复跑；③再逐步加回。解锁后的收益按功耗墙结算，核心降压换来的频率往往抵不过掉下来的稳定性。

### 计算负载被钉在 P8 低频？先查魔改 VBIOS

**实锤先例**（[#90](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/90)，2080 Ti）：EFI 事务源码级确认成功（booter 退出码 0x0 + PLM 开 + selector 读回），但 Windows 里**图形负载正常 P0、一建 CUDA 上下文立刻钉死 P8 645 MHz**——利用率 70–100% 功耗反而从 71 W 跌到 44 W。根因：该卡曾刷过流出的 **kingpin 类极限测试 VBIOS**（EVGA 3090/4090 1000W、XOC 890W 一族——NV 发给 OEM 超频工作室做极限冷却跑分用的特殊 BIOS，带计算类负载的独立 P-state 策略；识别线索：`nvidia-smi` 功耗读数远超零售设计，如 565 W vs 零售 300 W 墙）。**换正常零售 VBIOS 后立即恢复**。机理分析列为[开放问题 #14](open-questions.md)；改卡/超频用户建议先跑 `nvoc-cli get-vbios --verbose` 解析当前 VBIOS 的功耗表再解锁。

<a id="recovery"></a>

## 黑屏、STOP 或驱动异常后怎样恢复？

**同会话不要重试。** 固件可能仍在运行，DMA 缓冲可能仍被占用。

1. 完全关机断电；笔记本按厂商流程执行 EC 放电。
2. 拔出 U 盘，走原系统引导项正常启动。工具不需要写 ESP 或修改永久启动顺序。
3. 检查设备管理器错误码与系统事件日志；保留 `loader.log`，再按具体症状排障。
4. 若驱动仍异常，完成另一次彻底断电后再检查；必要时用 DDU 重装驱动。

解锁效果会随断电清除。黑屏、事务失败或驱动错误本身不能证明硬件永久损坏，也不能代替恢复后的设备检查。

<a id="safety"></a>

## 有哪些安全约束和实现边界？

- 写入以读回为验收依据，状态码和“运行完成”提示不构成独立执行证明。
- 异常后同会话不重试；只从 U 盘 / 一次性启动菜单进入，不覆写系统 ESP，不设置常驻 BootOrder。
- 替换 U 盘启动文件前先备份。Secure Boot 关闭或使用受信任的自签 EFI。
- NVIDIA 固件从钉版输入本地生成，不分发、不入 git；校验失败即停止。
- loader 按家族路由并记录 PROM CRC32，**尚未实现逐卡 ROM/PROM SHA-256 强制匹配**。
- [WPR2 守卫](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/4)与 pre-FLR、家族路径有关；不能把所有拒绝都称为“零写”。确认界面中止、严格 D0 入口中止有对应零写语义；`EBUSY` 前可能已经修改 PCI 配置并断开控制台。
- [描述符解析](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/25)支持 V2/V3；V1 未实现，encrypted 位未保留或强制检查。[运行装载](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/20)使用 portal，Falcon DMA 仍属实验代码。
- [模拟器](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/22)部分 verdict 来自故障开关和模拟侧 marker；真机验收仍以读回为准。

Turing `0x91` 排查需核对实际 DMA 地址与可用显存，不能只看 HIGH 标签或标称容量。[诊断记录 #14](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/14)、[#19](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/19)与[取证矩阵](tu10x-booter-0x91-matrix.md)保留相关条件；`tu10x-fb-gb.txt` 是寄存器无法解码时的手工 GiB 覆盖，仅按诊断要求使用，不作为普通安装选项。

边界跟踪：[诊断偏移与掩码](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/18)、[ROM 边界检查](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/21)、[bash 多卡 GSP 检查](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/23)、[LIC 描述符处理](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/24)。

开发门控与实验要求见 [AGENTS.md](../AGENTS.md)，部署操作见 [AGENTS.deploy.md](../AGENTS.deploy.md)。BAR1 扩展不是本工具计算解锁的一部分。许可为 GPL-2.0-only。

<a id="mechanism"></a>

## 解锁是怎样实现的？

PLM 控制寄存器的写权限，主机不能直接改写受保护的覆盖值。本工具在 NVIDIA 驱动加载前执行签名固件引导链，经 Booter 中的 V67/ROP 路径打开相应权限，再写入速率覆盖值、读回并交棒系统。载入使用寄存器 portal；签名 DMA 是载荷触发链的一部分。

| 家族 | 执行与交棒检查 |
| --- | --- |
| Ada | marker、HALT、目标 PLM 与选择器读回；FLR 后检查 AON 状态 |
| Ampere GA10x | 以 PLM 翻转验证执行，再检查选择器和交棒状态 |
| Turing | FEAT 位于 `0x4096xx`；写选择器后重锁 PLM，解锁路径跳过 FLR |

Ada 的 FBPA 读回 `0xffffff0f` 是允许的观察值，不应单独据此判失败。scrubber 的 handoff/超时警告也不是最终执行判据，应核对后续 FWSEC、家族执行判据、选择器及交棒结果。

原厂签名在交棒前恢复。解锁不刷写 VBIOS、不跨断电保留。详细机制与证据见 [Ada 报告](ada28e0-uefi-report.md)及 [Turing 报告](evidence/m3-tu106/host-and-final-result.md)。

<a id="nvoc-results"></a>

## 与 nvoc 配合有什么条件？

本工具解除 Tensor 发射速率限制，nvoc 提供系统内功耗、电压和超频控制。kmd 功耗墙解锁在移动端可用；10/20 系桌面卡有驱动读回，性能效果尚待验证。`set-volt-rail-limit` 的电压墙控制适用于 10 系及之后 GPU。

先独立验证本工具，再按 [nvoc 文档](https://github.com/Skyworks-Neo/nvoc)选择适用的控制和测试方式。提高发射率后，收益可能受功耗墙、频率、散热、访存和负载精度限制，不能把微基准倍率直接当作所有应用的倍率。

基准工具自身也须核验：确认输入张量非零、未被编译期常量折叠（社区实例：上游 mmapeak 存在编译期零输入坍缩，KohakUnlock 2026-10-28 抓出并改用运行时随机化 fork），并与锁定态做同机对照、FP32 平价控制标定噪声（受控基线惯例见 [兼容性矩阵](compatibility.md)与[社区报告](community-reports.md)）。

组合操作可能超出原厂功耗与温度包络。监控 Board Power、Hot Spot、限频原因及供电链路，确认散热与电源余量。极限功耗、电压不是其他显卡的推荐设置，超功耗、超压、超温造成的物理损坏由使用者承担。详细测量与条件见 [组合实测报告](ada28e0-uefi-report.md)；吞吐测试工具为 [cli-stressor-cuda-rs](https://github.com/Skyworks-Neo/nvoc/tree/main/cli-stressor-cuda-rs)。

<a id="reporting"></a>

## 怎样反馈不兼容或异常？

向 [Issues](https://github.com/physicsdolphin/nv-uefi-unlocker/issues)提交完整 `loader.log`、GPU 型号与 PCI ID、平台、驱动、显示模式、进入方式及系统侧诊断结果。**硬件信息不用手查**：Windows 跑 `ssdiag --inventory`、Linux 跑 `python3 tools/diag-unlock.py --inventory`，把输出块直接粘进表单（两工具均只读、分别免管理员/免 root；VBIOS/驱动/子系统 ID/机箱形态都在块里）。魔改卡附原型号、VBIOS 与显存容量；日志可先脱敏机器信息。性能问题附相同条件下的测试规模、精度、功耗和前后结果。

分享成功案例可使用 [Discussions](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions)。系统内无提升时先查 [RTD3](#linux)与 [读回判定](#ssdiag)。

<a id="layout"></a>

## 代码和研究记录在哪里？

| 目录 | 内容 |
| --- | --- |
| `loader/` | UEFI 应用 |
| `assets/` | 下载、校验、抽取与载荷生成 |
| `sim/` | 宿主模拟器与事务测试 |
| `tools/` | 系统内诊断工具 |
| `docs/evidence/` | 原始实验与里程碑证据 |

`cargo build --release` 构建宿主 assets/sim；loader 用 `cargo efi-tx` 构建事务程序，或用 `cargo efi` 构建只读采样程序。`cargo xt` 运行宿主测试。不要对宿主目标执行 `cargo build --workspace`。

设备验证与例外以 [兼容性矩阵](compatibility.md)为入口；[社区硬件记录](community-reports.md)、[桥 BME 报告](bridge-bme-root-cause.md)、[TU10x 0x91 研究](tu10x-booter-0x91-matrix.md)保留独立证据与推理。开发使用 [AGENTS.md](../AGENTS.md)，部署使用 [AGENTS.deploy.md](../AGENTS.deploy.md)。
