# 常见问题

[中文](faq.md) | [English](faq.en.md) · [README](../README.md) · [兼容性矩阵](compatibility.md)

- [USB 文件和高级选项怎么放？](#usb)
- [Turing 为什么要先启用 GSP？](#turing)
- [混合输出、重启和休眠有什么区别？](#display)
- [Linux 日志全绿，为什么没有提升？](#linux)
- [雷电 eGPU 能保留解锁吗？](#thunderbolt)
- [怎样在系统内检查或修复解锁？](#ssdiag)
- [booter status 0x89 是什么？](#bridge-bme)
- [事务成功后为什么会复位或黑屏？](#reconnect)
- [黑屏、STOP 或驱动异常后怎样恢复？](#recovery)
- [有哪些安全约束和实现边界？](#safety)
- [解锁是怎样实现的？](#mechanism)
- [与 nvoc 配合有什么条件？](#nvoc-results)
- [怎样反馈不兼容或异常？](#reporting)
- [代码和研究记录在哪里？](#layout)

<a id="usb"></a>

## USB 文件和高级选项怎么放？

maker 在本地 `usbroot/` 平铺输出文件。按脚本末尾清单复制所选家族的文件，另建 `EFI/BOOT/` 放启动程序；不要把抽取中间文件当作运行资产复制。

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
| `i_know_what_i_am_doing.txt` | 5 秒倒计时后自动运行，任意键中止；文件存在即启用，内容忽略 |
| `tx-quiet.txt` | 安静模式（issue #38）：console 只显示裁决行（banner/确认/每卡目标与结果/HANDOFF/FINAL/CHAIN），`loader.log` 仍逐行全量记录；log 文件打不开时自动回退全量 console。慢 iGPU（~1 行/秒刷新）机器上把分钟级回显压到秒级；与 `i_know_what_i_am_doing.txt` 组合即全自动无人值守快速跑。事务期间画面停在最后一行属正常，不是卡死 |
| `tx-gop-disconnect.txt` | 诊断开关：断开所有 GOP 句柄（含 iGPU）而非仅目标卡。**不要当提速手段用**——console 静默后无增益，且他卡 console 被杀后 loader 不会恢复它（依赖 bootmgfw 重建，存在 Code 12 风险形态，见 issue #35） |

无自动运行标记时，事务只接受空格或回车确认，其他键或 30 秒超时均中止。链回顺序为 Windows Boot Manager → 受支持的 Linux shim/grub/systemd-boot/Limine → 通用 `EFI/BOOT/BOOTX64.EFI`；最后一档提供 5 秒按键退出窗口。慢速 console 的提速走 `tx-quiet.txt` 的 console 白名单（issue #38），不做异步输出——UEFI 引导服务单线程，GOP 文本渲染本身是同步调用，异步提不了吞吐；每行日志带 `[+ms]` 阶段计时前缀，回显耗时与事务等待在 `loader.log` 里直接可分。

多卡默认按固件枚举顺序处理所有受支持的卡；已解锁或不支持的卡跳过，单卡失败按失败路径处理，不重试该卡。只想解锁其中一张时必须明确指定 BDF。

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

掉电清除的状态需要重跑 U 盘事务；只重写配置不会恢复它。来源：[RTD3 平台记录](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/9)。

<a id="thunderbolt"></a>

## 雷电 eGPU 能保留解锁吗？

不保证。支持环境以原生 PCIe 插槽和独显直连为准。雷电隧道重建、hot reset / PERST# 或坞供电管理可能在交棒后复位或断电，清除解锁；纯链路重训本身不复位端点。

进入系统后先做 [只读检查](#ssdiag)：

- Ampere/Ada 的 FEAT 门仍开、SS 值恢复出厂：可以用 `--reapply` 写回并验证。
- 门已锁、设备失电或读数不可访问：`--reapply` 不能开门或恢复断电状态，需按恢复流程重新进入 U 盘。
- SS 值仍正确但性能低：检查功耗、频率与测试方法。

Turing 的交棒本来就包含 PLM 重锁，不可套用 Ampere/Ada 的“门关即失效”判断。来源：[雷电平台记录](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)。

<a id="ssdiag"></a>

## 怎样在系统内检查或修复解锁？

Windows 使用发布包中的 `ssdiag-x86_64-windows.exe`。以下简写为 `ssdiag`：

```text
ssdiag
ssdiag --select 2504
ssdiag --reapply
```

默认只读诊断；`--select` 按 DeviceID 筛选。`--reapply` 是写操作，仅在门仍开放时重写 SS0/SS1 并读回，不写 PLM。Turing 保持只读。Linux 在源码树根目录运行 `sudo python3 tools/diag-unlock.py`，可用 `--bdf` 指定实际目标地址，也提供 `--reapply`。

| 读回结果 | 判断与下一步 |
| --- | --- |
| SS0=`0x88888888`，SS1 低四位=`0x8` | 覆盖值有效；再用匹配负载验证性能 |
| Ampere/Ada 门开、SS 值不符 | 可修复选择器，修复后再次读回 |
| Turing 门重锁、SS 值正确 | 符合正常交棒状态 |
| 门关闭且 SS 无效，或卡已失电 | 修复工具不能重新开门；按恢复流程重放 |

将系统侧结果与 `loader.log` 的 `HANDOFF-OUT` 对照。若驱动初始化异常，同时检查设备管理器、nvlddmkm/Xid 与电源状态；不要只用 EFI 的成功状态替代系统内验证。

交棒后无收益时，按以下读回区分问题；这些是排查方向，需结合电源和驱动日志确认：

| 系统侧现象 | 排查方向 |
| --- | --- |
| Ampere/Ada 门和选择器均回到锁定基线 | 断电或完整复位 |
| 门仍开，选择器恢复出厂值 | devinit 重写覆盖值 |
| 选择器被重写，同时出现 Code 43 或驱动复位 | 驱动初始化失败及恢复路径 |
| 选择器仍有效但性能低 | 频率、功耗墙与工作负载 |

ssdiag 从 OS 发现 BAR0，不接受手填物理地址；会按需唤醒停泊卡。内嵌的 `pmxdrv.sys` 经校验，以自有 `SSDIAG_PMXDRV` 服务临时加载，退出时清理；若 nvoc 等已有可用设备则借用。部署驱动通常需要管理员。更多寄存器范围及驱动说明见 [ssdiag 文档](../tools/ssdiag/README.md)。

<a id="bridge-bme"></a>

## booter status 0x89 是什么？

`0x89` 属于 Booter 的 WPR 描述符校验错误。GPU 上游 PCI 桥未开 Bus Master 时，设备 DMA 无法正确读到 WprMeta，可出现该错误。

loader 会为 GPU 上游路径中需要的桥开启 Bus Master 并读回，结束时还原；日志包含 `DMA-PATH` / `BRIDGE-BME`。遇到错误时保留完整日志，检查这些行与 GPU 状态，不要仅凭状态码自行归因或改 BIOS。

SVM、VT-x 和 VT-d 在已验证平台上不需要关闭。任务管理器的“虚拟化”不是 PCI Command 的 Bus Master 位。部分 AMI 机型 BIOS 的 "BME DMA Mitigation" 选项即是桥 BME 清零态的固件侧来源，保持默认即可，loader 不依赖它的开关状态。机制与证据见 [桥 BME 根因报告](bridge-bme-root-cause.md)。

<a id="reconnect"></a>

## 事务成功后为什么会复位或黑屏？

部分 AMI 固件会在 GPU 事务完成后的 GOP 重连中卡死或复位。loader 对断开后仍存在的无路径 GOP 使用非递归厂商重连，链式加载前跳过 NVIDIA 显示功能；运行日志即时写盘。

若仍停在 `RECONNECT: ConnectController ... begin` 或该阶段复位，完全断电恢复后，在 U 盘根放 `no-reconnect.txt` 跳过目标 GPU 重连。Windows Boot Manager 可自行建立控制台；跳过重连不代表跳过系统内验证。

Turing 若返回 `0x91`，或 EFI 成功但 GSP 交棒蓝屏，查 [兼容性矩阵](compatibility.md#device-16)，不要把所有黑屏都归为 GOP。平台证据见 [AMI 报告](ad106-asus-z790-windows-unlock.md)和 [X470 记录](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70)。

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
