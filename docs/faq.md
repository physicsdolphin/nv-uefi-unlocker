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

maker 在本地 `usbroot/` 输出资产（`*.bin` 平铺在该目录，另已建好 `EFI/BOOT/BOOTX64.EFI`）；把该目录的**内容**复制到 U 盘根目录（别把目录本身拷成 `U:\usbroot\`）。不要把抽取中间文件当作运行资产复制。

**为什么放到这个路径就能被识别？** UEFI 固件对**可移动介质**有一个固定的回退引导路径——`\EFI\BOOT\BOOTX64.EFI`（x86_64）。文件在这个路径上，固件就会把它当作引导项自动列出/执行，**不需要**写入或修改任何 NVRAM 启动项——这也是本工具"无常驻引导项"纪律（[安全约束](#safety)）能成立的原因：拔掉 U 盘，机器回到原样。其他架构有各自的默认文件名（如 aarch64 是 `BOOTAA64.EFI`）：当前工具、资产与所有实测都只覆盖 x86_64；如果你在 aarch64（ARM 笔记本/服务器）上有需求，请开 feature request/PR 说明平台与固件，开发者暂时没有测试条件。

**`-Out G:\` 在 Windows PowerShell 5.1 上会报 `New-Item : The path is not of a legal form`**（盘符根以 `\` 结尾，`New-Item -Path` 不接受；[#195](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/195)）。脚本创建目录时会自动剥掉尾反斜杠；若用旧包，改传 **`-Out G:`**（不带 `\`）即可。

maker 不带参数运行时自动识别制盘机上的 NVIDIA 显卡代际，只生成对应家族；`-Family` / `--family` 可显式指定 `ad102`（40 系）、`ga102`（30 系）、`tu10x`（20 / 16 系）或 `all`（三代全家桶，一张盘服务多种卡）。多家族盘的文件是各家族清单的并集。

`F` 为 `ad102`、`ga102` 或 `tu10x`：

| 位置 | 文件 | 用途 |
| --- | --- | --- |
| `EFI/BOOT/` | `BOOTX64.EFI` | 启动程序，替换前备份原文件 |
| 盘根 | `booter_load_F.bin` | Booter bundle（BTL1） |
| 盘根 | `gsp_boot_F.bin`、`gsp_boot_desc_F.bin` | GSP 引导段与 v5 描述符 |
| 盘根 | `fwimage_F.bin`、`fwsignature_F.bin` | GSP-RM 镜像与原厂签名 |
| 盘根 | `v67-F-four-plm.bin` | 解锁载荷 |
| 盘根，Ada / Ampere | `scrubber_ad10x.bin` | SEC2 scrubber |
| 盘根，Turing | `sec2_bl_tu10x.bin` | FWSEC 所需引导段 |
| 盘根，Turing | `gsp-gate.txt` | maker 生成的 GSP 检查记录 |
| 盘根，运行后 | `loader.log` | 阶段日志，下次运行前另存 |

移除 `v67-*.bin` 会进入回放模式：不执行解锁载荷，但仍会断开驱动、执行签名固件并可能复位 GPU。**回放不是只读检查**；只读采样使用 `cargo efi` 构建。

以下标记均放盘根，普通使用不需要创建：

| 标记文件 | 行为 |
| --- | --- |
| `tx-require-d0.txt` | 入口非 D0 时中止，不进行 GPU 写入或复位 |
| `tx-select-bdf.txt` | 内容为 `bb:dd.f` 或 `dddd:bb:dd.f`，仅处理匹配的卡；解析失败或目标缺失时不选其他卡 |
| `chain-target.txt` | 第一个非空、非 `#` 行指定优先 EFI 路径，逐卷查找；找不到时记录日志并回退默认顺序。**固件 BootOrder 里有跨盘残留引导项时先看这条**：默认解析会按 BootOrder 取到别的盘上的旧 loader（[#204](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/204) 实例是另一块盘的 `\EFI\ubuntu\shimx64.efi`，事务全绿但链到 Linux），首行写 `\EFI\Microsoft\Boot\bootmgfw.efi` 即可钉死本机链载目标 |
| `tx-no-preload.txt` | 禁用预驱动预载链回（P2-7）：不提前 LoadImage OS loader，回到事务后逐卷扫描的旧路径 |
| `no-reconnect.txt` | 跳过目标 GPU 的 GOP 重连，适用于事务后重连卡死或复位 |
| `tx-reconnect-nonrecursive.txt` | 只把重连改为非递归、保留 vendor GOP 绑定（console 栈交给 bootmgfw 重建）；比 `no-reconnect.txt` 温和，适用于"能进系统但重连卡死"一族（[#52](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/52) X470、[Discussion #131](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/131) X99/C610 实证） |
| `i_know_what_i_am_doing.txt` | N 秒倒计时后自动运行，任意键中止；首行数字（1–30）可覆盖秒数，缺省 5。armed 路径在倒计时窗口内完成只读预备（枚举/采样/选卡/资产预读进内存），自动运行从内存起跑 |
| `tx-flush-lines.txt` | 首行数字（0–4096）覆盖日志计数 flush 频率（缺省 16 行/次；0=仅阶段标记行 flush）。调大减少 USB 偶发慢写停顿（2026-11-01 TU106 实测 4 次 ~1.2s 停顿全与 flush 对齐），代价是挂死时丢更多日志尾巴；阶段标记行始终 flush |
| `tx-quiet.txt` | 安静模式（issue #38）：console 只显示裁决行（banner/确认/每卡目标与结果/HANDOFF/FINAL/CHAIN），`loader.log` 仍逐行全量记录；log 文件打不开时自动回退全量 console。慢 iGPU（~1 行/秒刷新）机器上把分钟级回显压到秒级；与 `i_know_what_i_am_doing.txt` 组合即全自动无人值守快速跑。事务期间画面停在最后一行属正常，不是卡死 |
| `tx-gop-disconnect.txt` | 诊断开关：断开所有 GOP 句柄（含 iGPU）而非仅目标卡。**不要当提速手段用**——console 静默后无增益，且他卡 console 被杀后 loader 不会恢复它（依赖 bootmgfw 重建，存在 Code 12 风险形态，见 issue #35） |
| `tx-reconnect-legacy.txt`（tx 构建） | **退出阀**：恢复旧版重连路径（递归 ConnectController + PR#13 自动非递归逻辑）。默认已改为 native-GOP console 重建（本卡影子 ROM 原厂 GOP），仅当新版在你的板上有异常时使用 |
| `tx-native-gop.txt`（tx 构建） | **可选 pin**：native-GOP 已是默认行为，此 marker 不再是启用开关；首行写卷上 GOP 模块路径可指定用该模块替代本卡影子 ROM（兼容包形态），空/缺省=影子 ROM |
| `i_know_what_i_am_doing.txt` | 5 秒倒计时后自动运行，任意键中止；文件存在即启用，内容忽略 |
| `tx-no-canary.txt` | 整组跳过复位金丝雀的读写（A/B 对照；见[回锁章](#relock)） |
| `tx-keep-assets.txt` | **退出阀**：交棒前不释放资产缓存（默认自 v2.1.11 起释放，见[交棒内存](#handoff)）。仅用于 A/B 或排查"链载后仍要读资产"的形态；`tx-free-assets.txt` 旧标记仍被接受，已是空操作 |
| `tx-acpi-dump.txt` | 只读：把固件 DSDT 与全部 SSDT 连同关键词索引落到卷根 `\acpi\`（离线 `iasl -d` 反编译；回答"谁在管这张卡的电"）。确认键之前执行，没有 N 卡也生效 |
| `tx-exit-fw.txt` | **退出阀**（[PR #209](https://github.com/physicsdolphin/nv-uefi-unlocker/pull/209)，issue #208 A 线）：跳过链式 `bootmgfw`，事务完成后直接交回固件引导管理器、由固件按 BootOrder 原生引导 Windows。适用于链载 `bootmgfw` 报 `\BCD` 0xc0000034 的机器（MSI GP66 11UG/AMI 实证：路径/卷/映像字节全精确仍死=bootmgfw 内部启动上下文差异，只能让固件自己拉）。**前提**：固件持久 BootOrder 里 Windows Boot Manager 须排在 U 盘之前，否则循环回 U 盘；一次性启动菜单进入则正常落穿。中止路径（30 秒超时/按错键）同样过此检查，marker 存在时中止也交回固件——链路坏掉的机器上这反而是更好行为（链载会落到 Recovery 屏）。marker 同置时事务前预载自动跳过 |

无自动运行标记时，事务只接受空格或回车确认，其他键或 30 秒超时均中止。链回顺序为 Windows Boot Manager → 受支持的 Linux shim/grub/systemd-boot/Limine → 通用 `EFI/BOOT/BOOTX64.EFI`；最后一档提供 5 秒按键退出窗口。慢速 console 的提速走 `tx-quiet.txt` 的 console 白名单（issue #38），不做异步输出——UEFI 引导服务单线程，GOP 文本渲染本身是同步调用，异步提不了吞吐；每行日志带 `[+ms]` 阶段计时前缀，回显耗时与事务等待在 `loader.log` 里直接可分。

多卡默认按固件枚举顺序处理所有受支持的卡；已解锁或不支持的卡跳过，单卡失败按失败路径处理，不重试该卡。只想解锁其中一张时必须明确指定 BDF。

### 能不能装到内置硬盘分区、开机自动解锁？

可以，但属于**专业用户自担操作**：本项目交付物保持 U 盘/一次性启动菜单形态，不提供、也不建议普通用户改内置分区引导。技术事实与边界：

- `bcdedit /create /application osloader` 造出的 Windows Boot Loader 条目 `device` 落在盘符上、重启即失效，固件不枚举；`bootsector` 类型则 `device` 字段静默为空——**两条路都进不了 `{fwbootmgr}`**，别在这上面花时间（[Discussion #104](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/104) 双 4080 SUPER 实测记录）。
- 可行形态（#104 实证）：数据卷缩出 ~2 GB FAT32 分区，按 U 盘同款布局放置 EFI+资产，BIOS 启动菜单里将其（显示为 `UEFI OS`）手动调前或 F12 一次性选择；可随时从 BIOS 菜单改回 Windows Boot Manager，不刷任何东西。
- 为什么不提供自动化脚本：多数用户的认知是"引导分区炸了=工具的锅"；缩容场景千差万别（双系统/跨盘引导/btrfs、zfs 等不支持在线缩容的文件系统），工具面强行覆盖全部存储拓扑有真实的数据安全风险。**策略：教程可查（如上）、自动化不做、允许下游 fork 提供自动化脚本并互链。**

异机制盘先显式指定 `-Family`（或用 `-Family all`），Turing 检查不适用时可用 `-SkipGspCheck` / `--skip-gsp-check` 跳过制盘机检查，但目标机仍须满足 GSP 前置。离线 Turing 引导归档 `gsp_rm_boot_dbg.bin` 用 `-ProvenBlob` / `--proven-blob` 指定；长度和 hash 校验仍有效。下载或校验失败时不要换版本或绕过检查。Windows PowerShell TLS 下载失败时可用 WSL / Git Bash 运行 bash maker。

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

固件可能只初始化负责显示的 GPU；本工具不能通过 ACPI `_PS0` / PR3 给完全断电的 dGPU 上电。非 D0 入口会触发 PM 预检，请求 D0 并读回确认；`tx-require-d0.txt` 可改为直接中止。

**笔记本内屏与亮度（DDC-CI）**：部分笔记本上，旧版固件重连会在解锁后把内屏链路留在未训练状态——背光直接黑、DDC-CI 亮度控制失灵（一台拯救者 R7000 2024 实证；旧路径首次尝试即复现，v2.1.10 在同一台机上修复）。v2.1.10 起默认的 native-GOP 重建会重新 `StartImage` 本卡原厂 GOP 驱动，进系统前重跑一次链路训练，自动消除该故障。这类机器上退回 `tx-reconnect-legacy.txt` 会让它复现。

TU106 + GSP 模式有休眠恢复后成功交棒的实测，但这不能代替其他机器的系统侧检查。详见 [系统内验证](#ssdiag)。

<a id="linux"></a>

## Linux 日志全绿，为什么没有提升？

先核对系统侧选择器和电源状态。RTD3 让空闲 GPU 进入 D3cold 时会清除易失解锁；EFI 读回成功不能证明状态在系统内一直保留。

若发行版在 `/usr/lib/modprobe.d/nvidia-runtimepm.conf` 启用 `NVreg_DynamicPowerManagement=0x02`，可在 `/etc/modprobe.d/` 使用同名文件覆盖。先备份已有配置：

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

**RTD3 关了还是没提升？（驱动 init 重推导，[#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)）**：GA10x 笔记本上存在第三种丢失机制——专有驱动在 `modprobe` 初始化时**主动重推导 FEAT 块**，与空闲掉电无关。实测（GA107M `25a0`，Linux Mint / 595.71.05）：RTD3 已设 `0x00`、GSP-RM 与 legacy 两种 init 模式**都**重推导且写入不同的位（SS0 相对出厂 delta 分别 `0x00400100` / `0x00400000`）——是主动编程，不是"没保留"；事务内 FLR 后 AON 存活也排除了 FLR 冲掉。这意味着在重推导型机器上，EFI 读回绿 ≠ 系统内有收益：**EFI 读回证明的是写入发生，不是它活过 `modprobe nvidia`**。Linux 侧现状：持久存活有实证的只有 TU102 桌面（[#103](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/103)）；GA10x 笔记本方面，目前仅 `25a0` 一例 Linux 观测到 init 重推导——同族 `25ad` 在 Windows 持久存活，OS/驱动版本/MUX-EC 平台态三个变量尚未切开（详见 [#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)），不宜外推为 GA10x 笔记本普遍结论；门关后 OS 内重写主机侧无法补救（SEC2 内全链属上游 kmod 谱系，超出本工具范围）。Windows 侧同族机制见 [拓扑维度](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/82)（[#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) GA106M 同形态）。

读回工具细节：Linux 下 `resource0` **只支持 mmap**，不支持 `read()`/`dd`——`dd` 会静默返回空，看起来像"PLM 已关"的假阴性；`tools/diag-unlock.py` 走的就是 mmap 路径。

**先看日志的 `POST-GOP` 行，别急着怪驱动**：从 v2.1.11 起交棒尾部会再读一次 FEAT 三连（`POST-GOP: FEAT_PLM=… SS0=… SS1=…`）。位置在**控制台重建之后**——默认的 native-GOP 重建会 `StartImage` 卡自己的 GOP 驱动（`\tx-reconnect-legacy.txt` 路径改为重连原厂栈、不自己 StartImage），而此前的 `POST-FLR` 验证在它之前，所以两行合起来才能切分写入方：`POST-GOP` 已锁/已丢 = 丢在 bootmgfw 之前（卡自己的 GOP/devinit 或平台固件，Windows 驱动出局）；`POST-GOP` 完好、系统内被抹 = 驱动这一 pass 负责。这也是 [#164](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/164) 的取证入口。

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

**证据等级**：上表是一台桌面机（TB4 坞、驱动与场景见 #54）的结果，规则与其中的功耗判据一致；不同架构（如 Turing）的"满速"基准不同（`READOUT` 语义见 [读回判定](#ssdiag)），不要把这里的倍率直接套到别的卡上。

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

Turing 交棒本来就带 PLM 重锁，别把它的正常交棒值当丢失——判读见[雷电一节](#thunderbolt)与[冷态对照](cold-values.md)。

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
| 3 | **devinit 重放**（驱动 init 自己重算） | 存活 | 部分 GA10x 笔记本（[#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)，Linux 实测；Windows 侧尚无同形态样本） | 门锁 + `SS` 被写成推导值（≠开机基线、≠覆盖值） |
| 4 | **窄写入**（只碰门） | 存活 | 少数平台 | 门锁 + `SS` 完好（[#82](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/82) 类） |
| 5 | **我们自己的尾巴**：native-GOP 重建 `StartImage` 卡自己的 ROM | — | 某些平台的 ROM POST 会带 devinit | `POST-GOP` 坏而 `POST-FLR` 好 |
| 6 | **OS 侧连接管理器（USB4 CM）建立/重建 USB4/雷电隧道** | 复位 | 雷电/USB4 eGPU（[#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)，已关闭，桌面 TB4 + AD102、笔记本原生 USB4 + GA104 实测）：Windows `Usb4DeviceRouter`、Linux `thunderbolt` 模块绑定/接管时拆掉并重建整棵下游总线。固件 CM 的独立雷电主机（Intel 雷电面板那类）开机不走此行，但重插线时管理栈重建隧道、同样触发 | 金丝雀 `cleared`（域复位）。两 OS 都有抑制：Linux `thunderbolt.host_reset=0`、Windows `Usb4DeviceRouter` `Start=4`（均实测，见本节正文） |

第 6 行的特殊性：复位由**建立/重建隧道的 CM** 执行——USB4 主机上是 OS 侧 CM，发生在任何驱动绑定之前，所以"接管之后再武装"没有主机写路径（`--reapply` 结构性无效，热插拔也一样）；固件 CM 主机开机不触发此行（见雷电节分型），重插才触发。唯一出路是抑制那次拆建，两套 OS 都已实测可行（正文给命令与代价）。取证走 chain 到 Linux + `modprobe.blacklist=nouveau,thunderbolt` + `tools/diag-unlock.py`（rEFInd 顶替 ISO 自带 GRUB、Secure Boot 可全程开着）。

第 1、2 行是物理事件，第 3 行是固件契约（见下节），第 5 行是我们唯一还能改自己代码的地方，第 6 行是 OS 侧旋钮（不在本工具内）——`\tx-reconnect-legacy.txt` 可以让交棒尾部改走重连原厂栈、不自己 `StartImage`。

**第 1 行已有两轴同抓的完整现场对**（2026-11-04，[#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37)，HP Victus 16 `25a2` 纯混合平台，`docs/evidence/issue37-hp-victus-25a2/`）：EFI 侧金丝雀设置并读回完好、交棒时卡 D0 → 系统内金丝雀清零 + FEAT 回厂推导值 + 根口 `00:01.0` PMCSR `D3`。FLR 被同日志排除（本链 `POST-FLR` 自证 GA10x 选择器活过 FLR），事件超出 FLR 即常规复位族，根口 D3 把形态压向第 1 行。对照组：TU106 桌面金丝雀 intact + 双端 D0（写入型才会长这样）；同为 GA107M 的 [#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)（Linux）是无复位的 devinit 重写（金丝雀应完好）——同 die、不同平台、不同行。金丝雀表的第四格（`intact` + FEAT 完好）也已有首个现场实例（2026-11-05，[#109](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/109#issuecomment-6650530730)：同卡同 VBIOS 同子系统、B650 桌面，交棒与系统内均 `intact`、双端 D0、`UNLOCKED`）——"什么都没发生"同样是一条读数，它把卡/VBIOS/驱动从鉴别变量里踢了出去，只剩平台。

### 平台上到底是谁在断电

dGPU 的电只有两种来源：桌面由插槽直供；笔记本/一体机的 dGPU 集成在平台里，供电是一个由 ACPI `_PR3` 的 `_ON` / `_OFF` 方法切换的 GPIO（BOCO 形态）。**D3cold 的定义就是对这些电源资源执行 `_OFF`**，下电顺序是根口广播 PME_Turn_Off → 链路进 L2/3 → 拉 PERST# → 撤电。**"桌面没有这条路径"不成立**：Z170 桌面板实测（2026-11-04，`docs/evidence/tu106-1f07-acpi-dump/`）三根 PEG 根口各有一个 `_PR3` 指向自己的 `PG0x`，复位与供电各是一对真 GPIO，`_OFF` 的动作就是**先拉 PERST# 再关槽供电**——插槽直供不等于根口不能切它。

更有解释力的一条（**假说，证据来自 Linux，Windows 侧未验证**）：OS 启动时会对**没人引用**的电源资源做状态同步，而 ACPI 允许对已处于关闭态的电源资源再执行一次 `_OFF`；固件 AML 若不幂等，多余的那次就是真断电（coreboot 的 Brya 板专门记录 dGPU 电源状态来躲开源内核的多余调用）。

这块板的 `_ON` / `_OFF` **是幂等**的，但幂等靠的是回读真实硬件而不是 `_STA`：两边入口都先跑 `CCHK`，端口已经能读到 vendor ID、或供电 GPIO 已是目标态就直接返回。所以重复调用不致命，**前提是 GPIO 状态可信**。上面这条假说需要在"AML 不幂等"的机器上才成立，本机更可能的路径是 Windows 自己发起 D3cold。这条假说能同时解释三件事：

- [#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) 是**确定性**复现：dGPU 无显示、驱动未绑定、OS 内 `D3` 且 MEM decode 关闭——驱动全程没参与还是丢，写入方就不在驱动里；
- 为什么 `D3Cold` 注册表开关无效：它拦的是 D3cold 请求，拦不住启动期对电源资源的状态同步；
- 为什么"进入方式"能反转结果：从 Windows 高级启动进，dGPU 被上一轮驱动持有；从 BIOS 冷启进，它是被 loader 从停泊态拉起来的、在 OS 记账里无人引用。

[#28](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/28) 给了一个旁证：混合模式下固件阶段只枚举到 dGPU 的音频功能，即整卡被停泊在未供电态。Modern Standby 还额外要求根口在 D0→D3 时对下游设备施加基础复位（微软固件要求文档），是第三条平台侧触发器。

中招需要三件事同时成立：拓扑上 dGPU 由 ACPI/EC 管电（或整链路走 USB4）、交接时无人引用、且该机 AML 真的切了电或拉了 PERST#——第三件是厂商相关的，所以比例按平台类分布而不是随机。桌面那几个例外（Z390P、B450M-A、B350M）与 B650 的对照原因未知，CSM 状态也没人报过。**issue 是自选样本、没有分母**，所以"百分之几"只能是量级描述。

自己看的话：卷根放 `	x-acpi-dump.txt` 把平台 AML 带回来（标记表 `tx-acpi-dump`），再查父根口的 `_PR3` 是否存在、`_ON` / `_OFF` 是否幂等、PERST# 挂在哪个 GPIO。一半的争论到这一步就有答案了。AML 说的是"平台**有权**怎么做"，`ssdiag` 0.6.8 起的 `POWER:` 两行说的是"**此刻**做到了哪一步"：卡与上游根口各自的 `PMCSR`/D 态，根口读 `D3` 就是 `_OFF` 已经跑过。注意它只是快照——`D0` 不证明这一引导没进过 `D3`，那是金丝雀回答的。

尚未切开：**GC6 与 GCOFF 是两个不同目标**（前者保自刷新，后者是完整断电），`_PR3` 路径与 GC6 唤醒路径目前被混着说，需要一次专门的对照。

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

1. **阻断复位源**：关 RTD3（[Linux](#linux) / Windows 对应驱动电源设置）、关快速启动与现代待机、BIOS 里关 PCIe 空闲电源门控；不要用"关机"当重启。
2. **改变入口态**：独显直连优先；混合输出下 MUX/面板归属决定设备是否被枚举（[#148 记录](#relock-148)）。
3. **事后修复**：门仍开时 `ssdiag --reapply` / `diag-unlock.py --reapply` 写回选择器；门已锁只能重放。
4. **报告时带齐证据**：`loader.log` 的 `POST-FLR` / `POST-GOP` / `HANDOFF-OUT` 三行（含 `CANARY` 与 `wpr2_lo`）、系统内读回值（用带 `CANARY:` 判定行的 `ssdiag` 0.6.5+）、`powercfg /a`、设备电源状态与事件日志。缺这些只能反复猜。
5. **把平台属性化**：确认某平台有 `_PR3` 且 rail 真断，就按平台属性登记，而不是当驱动缺陷追。
6. **给 dGPU 制造引用**：在 dGPU 直连的接口上接一台外接显示器再冷启。Linux 社区的经验说法是"接在 NVIDIA 一侧的显示器会挡住 D3cold"（轶事）；若丢锁随之消失，就支持"无人引用 → 被同步成关闭"这条假说。

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
| S0ix / 现代待机（Modern Standby） | 近年笔记本 | 可被驱动运行时电源管理降入 D3cold（rail 掉电） | 通常掉；选择器回厂 | 先只读检查，多数可用 `--reapply` 修回 |
| S3 传统睡眠 | 多数台式机 BIOS 的"睡眠" | 整机除内存外掉电 | 必然掉 | 同上，视检查结果 |
| S4 休眠 / 快速启动"关机" | 笔记本休眠、Win10/11 默认快速启动 | 全机断电 | 必然掉，等同冷启动 | 同上，视检查结果 |

查看本机支持哪些状态：管理员命令行 `powercfg /a`——"待机(S0 低电量待机)"即 S0ix，"待机(S3)"即传统睡眠；两者都列出的机器由 BIOS/驱动决定实际走哪个。

几点机制说明：

- **判据是 rail 是否掉电，不是"睡没睡"**。S0ix 名义上停在 S0，但 dGPU 空闲时可被独立断电（与 Linux 的 RTD3 同机制，见[上一节](#linux)）；S3 则整机掉电。两者在唤醒后的表现可能一样：选择器被 devinit 重新推导回厂。
- **FLR 不掉、掉电掉**：解锁写活在 AON（常开岛），事务内 FLR 后选择器存活是设计判据；但 AON 不活过失电——所以"重启进不了系统就丢"和"睡眠唤醒丢"可以是同一件事。
- **门与选择器要分开判**：唤醒后 Ampere/Ada 常见形态是 FEAT 门仍开、SS 值回厂——跑 [系统内检查](#ssdiag)确认后用 `ssdiag --reapply` 写回即可，不必重跑 U 盘；若门已锁或设备不可读，说明 rail 确实掉了，需重跑 U 盘事务。**全机断电路径（S4 休眠 / 快速启动）必然门锁**：AON 岛不活过失电，此时 `--reapply` 会以 `REFUSED: gate closed` 拒绝——这是正确判读不是工具故障，唯一恢复 = 重跑 U 盘事务（D#129 用户侧实证）。Turing 的 ssdiag 保持只读（其 SS 覆盖值在驱动加载后持久，devinit 重推导行为不同）。
- **不想让它掉**：台式机可在 BIOS/驱动侧避免空闲掉电；笔记本 S0ix 机器可关掉"现代待机"（部分固件提供）或接受唤醒后 reapply。Linux 侧对应配置见 [RTD3 节](#linux)。

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
- **Windows**：把 `Usb4DeviceRouter` 服务的 `Start` 设为 `4`（Disabled），`Usb4HostRouter` 保持 `3`（Manual）。**只禁设备侧、留主机侧**——两个都禁会让隧道根本不存在。两台不同主机栈（ASMedia TB4 + AD102 桌面 / Intel 原生 USB4 + GA104 笔记本）实测：卡正常枚举并绑定 `nvlddmkm`、金丝雀 `intact`、`ssdiag` 判 `UNLOCKED`，AD102 那台 8192³ FP16 达满血张量速率的 95% 并叠加 `nvidia-smi -lgc` 做了逐时钟扫描。代价如实说：坞的设备路由器服务没了，坞上 USB 口失效、两个 `USB4\ROOT_DEVICE_ROUTER` 节点 `Problem 32`（Code 32 = 服务被禁用，不是故障）；GPU、卡载音频、`nvidia-smi` 均正常。`DevPnPCapabilities=0x100` 已证伪；`Usb4HostRouter` 的 `Device Parameters` 子键只有 WDF/DMA/中断三类，无复位策略面。

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

Turing 的交棒本来就包含 PLM 重锁，不可套用 Ampere/Ada 的“门关即失效”判断。**回锁值本身不是失败**：实测常为 `0xffffffef`（bits 5:6 置位），而冷态默认是 `0xffffff8f`；该值随板/驱动版本存在方差——`1e04` 22G 改装卡（[#74](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/74)）实测过 `fully closed 0xffffff8f` 且 SS 仍持解锁值，`8f` 本身不作失败判读，看同一行的 `ss0/ss1`。`HANDOFF-OUT` 里 `tu10x: plm=0xffffffef` 配 `tu10x_unlock=true`、SS0/SS1 仍为 `0x88888888`/`0x8` 就是正常交棒形态（[冷态对照](cold-values.md)、[#102](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/102)、[#55](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/55)；#54/#74/#102 均已关闭 completed）。来源：[雷电平台记录](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)。

<a id="secureboot"></a>

## Secure Boot 开着能用吗？

**本节范围**：在不关闭 Secure Boot 的前提下启动 U 盘工具。它不处理「要求 Secure Boot 关闭才能玩」的反作弊客户端——那类平台（例如校验链式度量的 ACE 系）请按厂商要求处理，本节不给步骤。

先澄清定位：这不是绕过 Secure Boot。引导链照旧被完整校验，只是信任根从「微软签发的证书」扩到「你追加的那张证书」。工具唯一能工作的时窗（驱动加载之前的固件阶段）本来就在 Secure Boot 管辖之下，本项目也不持有微软或 UEFI CA 的签名身份。

三条边界，不看清就会对后面的步骤产生错误预期：

- **自签不解决 `ssdiag`**。`pmxdrv.sys` 是 Intel/COMODO 2018 交叉签名、没有微软根，`tools/ssdiag/README.md` 的原话是 load is **expected** to be refused；它列的三个豁免（≤1607 升级来的系统 / Secure Boot 关闭 / 签名证书早于 2015-07-29）在这张 2018-10-24 签发的叶证书上都不成立。所以 SB 常开时，系统内读回走 `loader.log` 的 `HANDOFF-OUT`。Linux 侧 `tools/diag-unlock.py` 是零驱动路径，但**没有人在 Secure Boot 开启状态下验证过它**。要 `ssdiag --reapply` 修回锁，仍需临时关一次 SB。
- **反作弊不保证**。自签不改 Secure Boot 的状态位；据维护者陈述 Vanguard（LoL）看的就是这个状态位，而**反作弊本身没有任何一方实测过**（本条目所依据的报告者也明确写了未测）。ACE 系校验 TPM/PCR[7] 链式记录，自签链的度量与原厂引导不同，不保证通过（机制与「刷进固件」的对比见下文[两条路的度量代价](#secureboot-vs-ffs)）。
- **换版本必须重签**：签名后改一个字节即失效；`*.bin` 资产不参与 PE 校验，不需要签。

### 动手之前：BitLocker

改 `db`/PK 会改变 PCR[7] 的度量内容，可能触发恢复密钥提示；改引导路径**可能**也会，取决于 BitLocker profile。两条路动手前都先挂起一次保护，并确认恢复密钥在手：

```powershell
Suspend-BitLocker -RebootCount 1
```

有一份报告用这条命令后未出现提示——那是一个机位的结果，不构成对其他配置的保证。

### 两条路怎么选

| | 路线 A：签进 db | 路线 B：MOK + shim |
| --- | --- | --- |
| 改动范围 | 固件 `db` 里多一张证书 | 固件密钥库不动，密钥放在 U 盘的 MOK |
| 最坏结果 | 清 PK 且该板无法恢复厂商 PK → Secure Boot 长期开不起来 | 引导失败，删掉 U 盘上的文件即回原状 |
| 固件升级 / 清 CMOS 之后 | `db` 可能被重置，需重新追加 | 不受影响（MOK 在盘上） |
| 可用前提 | 该板 Key Management 可编辑 | 有微软签名的 shim（任意发行版的 `shim-signed` 包） |

**Key Management 不可编辑的板子只有路线 B 可选。**

### 第 0 步：生成密钥（两条路共用）

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

### 路线 B：MOK + shim（两台机报告，其中一台为台式机全流程）

固件只校验微软签名的 shim；你的密钥通过 MOK 在 shim 之后生效，主板密钥库不动。依据是两份现场报告：[#162](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/162)（2026-11 上旬，Secure Boot 开启、v2.1.10、RTX 30 笔记本、`ga102`，PK/KEK/db 未动）与 [#210](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/210)（2026-11-08，RTX 3080 20GB + ASUS ROG Z790，官方 v2.1.10 原包 + 自加 `.sbat` + 自签 MOK，全链绿并交棒，系统内 `Confirm-SecureBootUEFI=True`、`MokListRT` 含证书、PK/KEK/db 不变）；#162 的脚本仓 [J0s3f/nv-unlock-secureboot-esp](https://github.com/J0s3f/nv-unlock-secureboot-esp) 可作参考实现，未 vendor 进本仓库。#210 相对 #162 的增量是**可复算的补节参数**（第 3 步）与"倒计时必须按键"这一步（第 7 步），两者都只来自 #210。

1. 用第 0 步生成 `MOK.key` / `MOK.crt` / `MOK.der`。

2. **先检查发布件是否已有 `.sbat`**：

   ```bash
   objdump -h BOOTX64.EFI | grep -i sbat
   ```

   有输出就跳到第 4 步。v2.1.10 的发布件**没有**这个节（本项目实测：`.text` / `.rdata` / `.data` / `.eh_fram` / `.reloc` 五节，无 `.sbat`）。#210 在真机上确认了后果：缺节时 shim 只回 `Verification failed: (0x1A) Security Violation`，连 MokManager 都进不去，且不说明原因。

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

   顺序不能反：加节要针对**未签名**的 `BOOTX64.EFI`；若手上已经是签过名的文件，加完节必须重签。#210 在 v2.1.10 上实做的结果与此算法一致（`.sbat` 177 字节、节数 5→6、`SizeOfImage` `0x68000→0x69000`），可作为自查对照：三个数任一不符，说明动手的文件不是那版发布件。

4. **签名并验证**（产物名 `grubx64.efi`，shim 默认在自身目录找这个名字）：

   ```bash
   sbsign --key MOK.key --cert MOK.crt --output grubx64.efi BOOTX64.grub.efi
   sbverify --cert MOK.crt grubx64.efi
   ```

   Windows 侧等价物（#210 实做）：`signtool sign -fd SHA256 -f MOK.pfx grubx64.efi`。签完**独立重算 Authenticode 摘要**，与镜像内嵌 `SpcIndirectDataContent` 的摘要比对；只跑 `signtool verify` 不足以证明嵌进去的就是你这份文件。

5. **取 shim 与 MokManager**：发行版 `shim-signed` 包里的 `shimx64.efi.signed` 与 `mmx64.efi`。报告者用的是 shim-signed 15.8；**shim < 15.3 不做 SBAT 校验**，其行为未在本仓库验证过。

6. **制盘再替换**：先用 maker 正常制盘（会生成 7 个 `ad102` 资产与 `EFI/BOOT/BOOTX64.EFI`），然后在 U 盘的 `\EFI\BOOT\` 里替换成四个文件：`BOOTX64.EFI` ← shim、`grubx64.efi` ← 第 4 步产物、`mmx64.efi` ← MokManager、`MOK.der` ← 你的证书。**资产留在卷根，不要动**（`ga102` 约 85 MB）。

7. **首次引导**：shim 报 `Verification failed: (0x1A)` 属于预期（此时 MOK 还没登记），随后出现 `Press any key to perform MOK management` 的 **10 秒倒计时——必须在这个窗口里按键**。不按的话 shim 会静默继续往下走，loader 根本不会被执行，看起来就像"什么都没发生"（#210 实测）。进入 MokManager 后：Enroll key from disk → `MOK.der` → 设一次性密码 → 重启确认 Enroll。之后每次引导：shim（微软签）→ 用 MOK 验你的签名 → 启动 loader → 解锁 → 链回 `bootmgfw`，Secure Boot 全程开启。

内置 ESP 变体（**本项目未验证，也不推荐**）：把上面四个文件放进内置 ESP 的 `\EFI\BOOT\`。报告者观察到 loader 在 CHAIN-BOOT 里会跳过自己的 `Boot####` 与同一 ESP 上的 Windows 项，因此可以排在 BootOrder 首位而不循环。代价有两个：README 的明确建议是「[不要复制到系统 ESP](../README.md)，也不需要调整永久启动顺序」，以及那 85 MB 资产要放在 Windows ESP 的根目录——动手前先看剩余空间。

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
| 证据 | 路线 B 两台机（含一台台式机全流程，`.sbat` 参数与倒计时按键均已实做）；路线 A 二手 | 仅[可行性文档](dxe-driver-ffs-feasibility.md)，**未实机验证** |

一句话：**两条路都改变了某个度量**——自签动的是引导链（PCR[4]/[7] 侧），FFS 动的是固件（PCR[1] 侧）。对「只查 SB 状态位」的客户端，两条都可能有效；对「要求出厂引导链或出厂固件度量」的客户端，两条都不保证。反作弊的判定规则是厂商侧策略，会随时调整，本文不做任何「能过/不能过」的承诺。

### 证据状态

| 项目 | 证据 | 状态 |
| --- | --- | --- |
| 路线 B（MOK + shim） | #162 笔记本报告（解锁存活到 Windows）；**#210 台式机全流程**：SB 开启、v2.1.10 + `.sbat` + 自签 MOK，全链绿 + `Confirm-SecureBootUEFI=True` + `MokListRT` 含证书 + PK/KEK/db 不变 | 两台机；第 3/4/7 步按 #210 实做参数写死，本项目未在自有硬件上复跑 |
| 发布件缺 `.sbat` | 本项目核对 v2.1.10 产物节表；#210 真机确认缺节时 shim 只回 `0x1A` | 已验证（产物层 + 一台真机） |
| 第 0 步的 `openssl` 命令 | 本项目执行（OpenSSL 4.0.3） | 已验证 |
| 路线 A（db 自签） | 二手反馈，无日志、无主板型号 | 未复现、未执行 |
| Secure Boot 开启 + `ssdiag` | `tools/ssdiag/README.md`：expected to be refused；#210 据此选择不加载 | 未实测（#210 明确标 not verified） |
| Secure Boot 开启 + `diag-unlock.py` | 无 | 未验证 |
| 反作弊（Vanguard / ACE） | 维护者陈述 | 未测量 |
| 固件内置 FFS 路线 | `docs/dxe-driver-ffs-feasibility.md`（设计、风险、PCR[1]/BitLocker 影响） | 未实机验证 |

跑了任一条路，欢迎回报**主板型号**、**`ssdiag` 是否被拒**、**BitLocker 是否弹过恢复密钥**、**`diag-unlock.py` 在 SB 开启下是否可读**——这四项各自能把上面的行往前推一格。

<a id="ssdiag"></a>

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

| 读回结果 | 判断与下一步 |
| --- | --- |
| SS0=`0x88888888`，SS1 低四位=`0x8` | 覆盖值有效；再用匹配负载验证性能 |
| Ampere/Ada 门开、SS 值不符 | 可修复选择器，修复后再次读回 |
| Turing 门重锁、SS 值正确 | 符合正常交棒状态；实测回锁值常为 `0xffffffef`（bits 5:6 置位，≠ 冷态默认 `0xffffff8f`），见 [冷态对照](cold-values.md) |
| 门关闭且 SS 无效，或卡已失电 | 修复工具不能重新开门；按恢复流程重放 |

将系统侧结果与 `loader.log` 的 `HANDOFF-OUT` 对照。若驱动初始化异常，同时检查设备管理器、nvlddmkm/Xid 与电源状态；不要只用 EFI 的成功状态替代系统内验证。

交棒后无收益时，按以下读回区分问题；这些是排查方向，需结合电源和驱动日志确认：

| 系统侧现象 | 排查方向 |
| --- | --- |
| Ampere/Ada 门和选择器均回到锁定基线 | 断电或完整复位 |
| 门仍开，选择器恢复出厂值 | devinit 重写覆盖值（见 [回锁机制](#relock)） |
| 选择器被重写，同时出现 Code 43 或驱动复位 | 驱动初始化失败及恢复路径 |
| 选择器仍有效但性能低 | 频率、功耗墙与工作负载 |

ssdiag 从 OS 发现 BAR0，不接受手填物理地址；会按需唤醒停泊卡。内嵌的 `pmxdrv.sys` 经校验，以自有 `SSDIAG_PMXDRV` 服务临时加载，退出时清理；若 nvoc 等已有可用设备则借用。部署驱动通常需要管理员。其安全模型与权衡单独成节：[pmxdrv.sys 的安全模型与权衡](#pmxdrv)。更多寄存器范围及驱动说明见 [ssdiag 文档](../tools/ssdiag/README.md)。

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

1. **先核对 hash**——zip 与每个成员都在 `SHA256SUMS.txt` 里，`ssdiag` 的 hash 同时钉死在二进制内、运行时自校验。hash 对得上就排除"下载被换包"。
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

<a id="handoff"></a>

## 解锁日志全绿，但 Windows 起不来（`0xc000009a` / 恢复环境）？

`0xc000009a` = `STATUS_INSUFFICIENT_RESOURCES`，发生在 winload 阶段（能看到 “按 F8 / F1 / Esc” 的恢复屏即已跑到这一步）。**不是内存不够，是最大可用块不够**：ExitBootServices 不把任何东西转成 conventional，我们自己的资产缓存会作为 `LOADER_DATA` 描述符劈开低 4 GiB 里唯一那段大块，而 winload 的启动预留只从单个描述符里取、不拼接——一台 32 GB 机器空闲 2.4 GiB 照样失败（[#146](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146)，唯一一例）。

**v2.1.11 起默认在交棒前把资产池还给固件分配器**，代价为零（过了事务就再没人读资产）。判据看日志：

```
CHAIN-MEMMAP: ... largest block 2604 MiB (...)     <- 修复后；失败态这个数被砍半
TX-ASSET-CACHE: cleared 9 entries, 84388324 bytes handed back to the firmware allocator
```

- 老版本遇到本症状：确认盘根有 `tx-free-assets.txt`（零字节）再跑一次；v2.1.11+ 该标记已是空操作。
- 需要看"不释放"的对照时放 `tx-keep-assets.txt`（零字节），日志会打 `cache left resident across the chain`。
- 机制、三跑内存图与规范/EDK2 依据见 [交棒前的低位内存](handoff-lowmem.md)。

<a id="reconnect"></a>

## 事务成功后为什么会复位或黑屏？

部分 AMI 固件会在 GPU 事务完成后的 GOP 重连中卡死或复位。loader 对断开后仍存在的无路径 GOP 使用非递归厂商重连，链式加载前跳过 NVIDIA 显示功能；运行日志即时写盘。

若仍停在 `RECONNECT: ConnectController ... begin` 或该阶段复位，完全断电恢复后，在 U 盘根放 `no-reconnect.txt` 跳过目标 GPU 重连。Windows Boot Manager 可自行建立控制台；跳过重连不代表跳过系统内验证。

Turing 若返回 `0x91`，或 EFI 成功但 GSP 交棒蓝屏，查 [兼容性矩阵](compatibility.md#device-16)，不要把所有黑屏都归为 GOP。平台证据见 [AMI 报告](ad106-asus-z790-windows-unlock.md)和 [X470 记录](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70)。

**事务失败时屏幕可能保持黑屏，但机器是活的。** `native-GOP` 重建只在成功路径执行；失败路径只做回滚（恢复原厂签名→FLR→链回引导），断开过的 console 不会重建。实测（[#191](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/191)）：失败的 HIGH staging 轮之后系统照常启动、SSH 可用、驱动与桌面正常，只有内屏是黑的。单显笔记本站到这种画面按"黑屏但活着"处理——等它进系统用 SSH 或外接屏确认，或直接断电重来；不要因为黑屏就中途拔盘断电。

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
