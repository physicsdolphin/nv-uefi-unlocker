# 常见问题

[中文](faq.md) | [English](faq.en.md) · [README](../README.md) · [兼容性矩阵](compatibility.md)

- [USB 文件和高级选项怎么放？](#usb)
- [Turing 为什么要先启用 GSP？](#turing)
- [混合输出、重启和休眠有什么区别？](#display)
- [Linux 日志全绿，为什么没有提升？](#linux)
- [为什么解锁过的卡一进系统又被锁回去？](#relock)
- [Windows 睡眠后解锁还在吗？（S3 / S0ix 现代待机）](#sleep)
- [雷电 eGPU 能保留解锁吗？](#thunderbolt)
- [怎样在系统内检查或修复解锁？](#ssdiag)
- [pmxdrv.sys 的安全模型与权衡（必读披露）](#pmxdrv)
- [booter status 0x89 是什么？](#bridge-bme)
- [选 U 盘一闪回到启动菜单或直接进系统，盘上没有 loader.log？](#no-boot)
- [事务成功后为什么会复位或黑屏？](#reconnect)
- [黑屏、STOP 或驱动异常后怎样恢复？](#recovery)
- [有哪些安全约束和实现边界？](#safety)
- [解锁是怎样实现的？](#mechanism)
- [与 nvoc 配合有什么条件？](#nvoc-results)
- [怎样反馈不兼容或异常？](#reporting)
- [代码和研究记录在哪里？](#layout)

<a id="usb"></a>

## USB 文件和高级选项怎么放？

maker 在本地 `usbroot/` 输出资产（`*.bin` 平铺在该目录，另已建好 `EFI/BOOT/BOOTX64.EFI`）；把该目录的**内容**复制到 U 盘根目录（别把目录本身拷成 `U:\usbroot\`）。不要把抽取中间文件当作运行资产复制。

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
| `chain-target.txt` | 第一个非空、非 `#` 行指定优先 EFI 路径，逐卷查找；找不到时记录日志并回退默认顺序 |
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

无自动运行标记时，事务只接受空格或回车确认，其他键或 30 秒超时均中止。链回顺序为 Windows Boot Manager → 受支持的 Linux shim/grub/systemd-boot/Limine → 通用 `EFI/BOOT/BOOTX64.EFI`；最后一档提供 5 秒按键退出窗口。慢速 console 的提速走 `tx-quiet.txt` 的 console 白名单（issue #38），不做异步输出——UEFI 引导服务单线程，GOP 文本渲染本身是同步调用，异步提不了吞吐；每行日志带 `[+ms]` 阶段计时前缀，回显耗时与事务等待在 `loader.log` 里直接可分。

多卡默认按固件枚举顺序处理所有受支持的卡；已解锁或不支持的卡跳过，单卡失败按失败路径处理，不重试该卡。只想解锁其中一张时必须明确指定 BDF。

### 能不能装到内置硬盘分区、开机自动解锁？

可以，但属于**专业用户自担操作**：本项目交付物保持 U 盘/一次性启动菜单形态，不提供、也不建议普通用户改内置分区引导。技术事实与边界：

- `bcdedit /create /application osloader` 造出的 Windows Boot Loader 条目 `device` 落在盘符上、重启即失效，固件不枚举；`bootsector` 类型则 `device` 字段静默为空——**两条路都进不了 `{fwbootmgr}`**，别在这上面花时间（[Discussion #104](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions/104) 双 4080 SUPER 实测记录）。
- 可行形态（#104 实证）：数据卷缩出 ~2 GB FAT32 分区，按 U 盘同款布局放置 EFI+资产，BIOS 启动菜单里将其（显示为 `UEFI OS`）手动调前或 F12 一次性选择；可随时从 BIOS 菜单改回 Windows Boot Manager，不刷任何东西。
- 为什么不提供自动化脚本：多数用户的认知是"引导分区炸了=工具的锅"；缩容场景千差万别（双系统/跨盘引导/btrfs、zfs 等不支持在线缩容的文件系统），工具面强行覆盖全部存储拓扑有真实的数据安全风险。**策略：教程可查（如上）、自动化不做、允许下游 fork 提供自动化脚本并互链。**

异机制盘先显式指定 `-Family`（或用 `-Family all`），Turing 检查不适用时可用 `-SkipGspCheck` / `--skip-gsp-check` 跳过制盘机检查，但目标机仍须满足 GSP 前置。离线 Turing 引导归档 `gsp_rm_boot_dbg.bin` 用 `-ProvenBlob` / `--proven-blob` 指定；长度和 hash 校验仍有效。下载或校验失败时不要换版本或绕过检查。Windows PowerShell TLS 下载失败时可用 WSL / Git Bash 运行 bash maker。

**中国大陆网络**（2026-11-01 实测）：`raw.githubusercontent.com` 被阻断，Turing 引导归档的抓取会自动依次回退到 jsDelivr CDN 与代理传输（同一对象，钉定 SHA-256 逐字节把关）；`download.nvidia.com` 与 `archive.cachyos.org` 当前可直连，暂无镜像回退。任何钉定文件都可以离线获得后直接放进 `inputs/`（哈希匹配即接受）——这是官方认可的离线路径。某传输源失效时不要换版本，换源或离线投放皆可。PowerShell 内建 `curl` 别名与下载脚本无关：maker 的 PowerShell 版只用 `Invoke-WebRequest`，bash 版用真实 `curl`/`wget`。

<a id="turing"></a>

## Turing 为什么要先启用 GSP？

Turing 的 Windows 驱动需要 GSP-RM 模式才能正常接管解锁状态。legacy 初始化可能导致 TDR / Code 43。管理员 PowerShell 中执行与安装方式对应的一条命令：

```powershell
.\enable-turing-unlock.ps1
```

源码树中的脚本为 `tools\windows\enable-turing-unlock.ps1`。脚本仅为 Turing 显示设备设置 `EnableGpuFirmware=1`；加 `-Remove` 可撤销。**必须重启**，休眠恢复不会重新初始化驱动。

maker 会检查注册表开关和 `nvidia-smi -q` 中的 GSP 固件运行态，后者不能为 N/A。Linux Turing 同样需要运行 GSP-RM；专有驱动 545+ 默认使用该模式，但可能被 `NVreg_EnableGpuFirmware=0` 关闭，仍以运行态检查为准，不运行 Windows 注册表脚本。30 / 40 系无需这项 Windows 注册表前置。

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

- **安全模式配对有两条失效路**：① MUXless 机器 dGPU 停泊时 BAR0 读不到，`ssdiag` 无从取数——安全模式只能证明"驱动没参与"，不能给出读回（Catnip 在 [#126](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/126) 指出）；② 即使 BAR0 读得到，`ssdiag` 的 pmxdrv 载具在安全模式里 `StartService` 会撞 `win32 1084`（`ERROR_NOT_SAFEBOOT_SERVICE`）——未列入 `SafeBoot\Minimal` 白名单的内核驱动在安全模式一律不可启动（#37 Victus 实测）。安全模式切分对 `ssdiag` 结构性不可用，直接走 [Linux live 读回法](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/82)或看 ACPI。
- **`WPR2_HI` 单独不能用**：driver 自己的 FWSEC 会在下一个 boot 重新建窗，值本来就变，只有和 `wpr2_lo` 成对、且与本次交棒值逐位对照才有意义。

### 机制盘点：谁能让它回厂

| # | 机制 | AON 域 | 典型场景 | 看到的形态 |
| --- | --- | --- | --- | --- |
| 1 | **真掉电**（rail 断） | 复位 | 带 `_PR3` 的根口 + D3cold/RTD3/现代待机（笔记本常见，桌面也可有——本机 Z170 三个 PEG 全有）、EC 电源门控、外置坞断电、S4/快速启动 | 门锁 + 选择器回开机基线；`power_state` 曾为 D3cold |
| 2 | **整域复位** | 复位 | 平台 SBR、链路热复位、全芯片 SW_RESET；多见于老固件/"奇怪"主板 | 同上，且 `POST-GOP` 就已经坏 |
| 3 | **devinit 重放**（驱动 init 自己重算） | 存活 | 部分 GA10x 笔记本（[#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)，Linux 实测；Windows 侧尚无同形态样本） | 门锁 + `SS` 被写成推导值（≠开机基线、≠覆盖值） |
| 4 | **窄写入**（只碰门） | 存活 | 少数平台 | 门锁 + `SS` 完好（[#82](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/82) 类） |
| 5 | **我们自己的尾巴**：native-GOP 重建 `StartImage` 卡自己的 ROM | — | 某些平台的 ROM POST 会带 devinit | `POST-GOP` 坏而 `POST-FLR` 好 |

第 1、2 行是物理事件，第 3 行是固件契约（见下节），第 5 行是我们唯一还能改的地方——`\tx-reconnect-legacy.txt` 可以让交棒尾部改走重连原厂栈、不自己 `StartImage`。

**第 1 行已有两轴同抓的完整现场对**（2026-11-04，[#37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37)，HP Victus 16 `25a2` 纯混合平台，`docs/evidence/issue37-hp-victus-25a2/`）：EFI 侧金丝雀设置并读回完好、交棒时卡 D0 → 系统内金丝雀清零 + FEAT 回厂推导值 + 根口 `00:01.0` PMCSR `D3`。FLR 被同日志排除（本链 `POST-FLR` 自证 GA10x 选择器活过 FLR），事件超出 FLR 即常规复位族，根口 D3 把形态压向第 1 行。对照组：TU106 桌面金丝雀 intact + 双端 D0（写入型才会长这样）；同为 GA107M 的 [#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)（Linux）是无复位的 devinit 重写（金丝雀应完好）——同 die、不同平台、不同行。

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

## 雷电 eGPU 能保留解锁吗？

不保证。支持环境以原生 PCIe 插槽和独显直连为准。雷电隧道重建、hot reset / PERST# 或坞供电管理可能在交棒后复位或断电，清除解锁；纯链路重训本身不复位端点。

进入系统后先做 [只读检查](#ssdiag)：

- Ampere/Ada 的 FEAT 门仍开、SS 值恢复出厂：可以用 `--reapply` 写回并验证。
- 门已锁、设备失电或读数不可访问：`--reapply` 不能开门或恢复断电状态，需按恢复流程重新进入 U 盘。
- SS 值仍正确但性能低：检查功耗、频率与测试方法。

Turing 的交棒本来就包含 PLM 重锁，不可套用 Ampere/Ada 的“门关即失效”判断。**回锁值本身不是失败**：实测常为 `0xffffffef`（bits 5:6 置位），而冷态默认是 `0xffffff8f`；该值随板/驱动版本存在方差，`HANDOFF-OUT` 里 `tu10x: plm=0xffffffef` 配 `tu10x_unlock=true`、SS0/SS1 仍为 `0x88888888`/`0x8` 就是正常交棒形态（[冷态对照](cold-values.md)、[#102](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/102)、[#55](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/55)）。来源：[雷电平台记录](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)。

<a id="ssdiag"></a>

## 50 系（RTX 50 / Blackwell）能解锁吗？

**现在不能。** 钳制和门本身与 40 系同形且更硬：GB206 的熔丝 `FMLA32/IMLA1/2/4=3`、`FEAT_OVR_PLM=0xfffffb8f`（仅固件内部的 PL3 层可写），主机写 3/3 静默丢弃；同 die 的 RTX PRO 6000 满速，证明是产品分割。

真正断在引导链：Blackwell **没有 SEC2 阶段**（FSP→GSP BootROM→加密 FMC→GSP-RM），也就没有 40 系那种"签名 booter 栈溢出 → ROP 写 PLM"的载体。公开世界没有 GB20x booter，debug FMC 与 prod 的密文逐字节相同、只有签名不同且无槽表。所以**换 U 盘跑也打不开门**，`ssdiag --reapply` 会正确拒绝。

可以做的：ssdiag / diag-unlock / U 盘 SAMPLE 都会记录它的 FEAT 块和熔丝（只读）。读数纪律：`READOUT_1` 是熔丝镜像、字段位不连续，且 Blackwell 的 `3` 约等于半速而不是 Ada 的 1/8；SS0/SS1 非零只是影子态，nibble 的 bit3 全清才是"熔丝生效"。机理、探索现状与还开着的面见 [blackwell-gb20x-status](blackwell-gb20x-status.md)（[#168](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/168)）。

## 怎样在系统内检查或修复解锁？

Windows 使用发布包中的 `ssdiag-x86_64-windows.exe`。以下简写为 `ssdiag`：

```text
ssdiag
ssdiag --select 2504
ssdiag --reapply
```

默认只读诊断；`--select` 按 DeviceID 筛选。`--reapply` 是写操作，仅在门仍开放时重写 SS0/SS1 并读回，不写 PLM。Turing 保持只读。Linux 在源码树根目录运行 `sudo python3 tools/diag-unlock.py`，可用 `--bdf` 指定实际目标地址，也提供 `--reapply`。

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

向 [Issues](https://github.com/physicsdolphin/nv-uefi-unlocker/issues)提交完整 `loader.log`、GPU 型号与 PCI ID、平台、驱动、显示模式、进入方式及系统侧诊断结果。魔改卡附原型号、VBIOS 与显存容量；日志可先脱敏机器信息。性能问题附相同条件下的测试规模、精度、功耗和前后结果。

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
