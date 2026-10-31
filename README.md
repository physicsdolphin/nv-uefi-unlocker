# nv-uefi-unlocker

[项目宣传片（Bilibili, 简体中文）](https://www.bilibili.com/video/BV1d6xN66ECM/)

[原理科普 - 专业用户友好版（Bilibili, 简体中文）](https://www.bilibili.com/opus/1264436370549506065)

[原理科普 - 入门用户友好版（Bilibili, 简体中文）](https://www.bilibili.com/opus/1264493051219279872)

[Reddit 讨论（English）](https://www.reddit.com/r/overclocking/comments/1xj5jp3/new_uefi_gpu_unlocking_tool_revealed_for_windows/)

[X 讨论（English）](https://x.com/RoundtableSpace/status/2115749232980140458)

[中文](README.md) | [English](README.en.md)

20 \~ 40系 全平台操作系统通用 UEFI 预驱动 NVIDIA GPU 解锁器（开发环境为 Windows，Rust）。在 NVIDIA 驱动加载**之前**的
UEFI DXE 时窗执行完整固件事务：SEC2 scrubber → FWSEC-FRTS（WPR2）→ 签名
Booter 启动 GSP-RM →（门控后）V67 四写 payload → 读回验证 → FLR → 链式交回
Windows Boot Manager。

**当前状态：40 系（AD10x）+ 30 系（GA10x）+ 20 系（TU10x）三轨闭环——
v2.1.0 起 30 系全系兼容。** 支持矩阵：

- **RTX 20 系（Turing）✅ 全系兼容 实测支持（v0.2.0）**——RTX 2070（TU106, `10de:1f07`）
  - ⚠️ RTX 2080（TU104-400, `10de:1e87`）v2.1.1~v2.1.6 实测 booter 不起（mailbox0=0x91），
    与 2080 Super（`1e81`）逐项对齐后仍失败，排查中（[issue #62](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/62)）。
  全链解锁：**FP32 累加 GEMM 36 → 60–63 TFLOPS（+67~75%）**；2060 Super
  （`10de:1f06`）社区实测同链通过（[issue #12](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/12)）。
  **Turing 用户
  必须先运行 [GSP 固件模式脚本](#turing20-系专属gsp-固件模式前置必读)并重启**，
  详见下方 Turing 专属节。
- **RTX 30 系（Ampere/GA10x）✅ 全系兼容（v2.1.0）**——2026-10-28 起 GA10x
  设备 ID 全段放行（0x22xx/0x24xx/0x25xx），**30 系全兼容**。实证：RTX
  3060 Ti LHR（GA104, `10de:2489`）v2.1.0 解锁成功（[issue
  #11](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/11)：
  wait_plm 判据 PLM 翻转 + SS0/SS1 读回全对，AMD 平台 + 刷过跨厂 VBIOS
  均通过）；同链在上游社区工具已覆盖 3060（2487，+58~85%）。**30 系用户
  无需** Turing 的 GSP 固件模式前置。
  补充实测：3080（`2206`）、3070 Laptop（`249d`）、3080 Ti Laptop（`2420`）、3060（GA106，`2504`/`2520`），见[社区验证汇总](docs/community-reports.md)。
- **RTX 40 系（Ada）✅ 全系兼容 实测支持**——RTX 4060 Laptop（`10de:28e0`，AD107M）
  四写解锁双端验证：固件侧 marker `0x67b00001` + 四 PLM 读回，Windows 侧
  FP16 GEMM 29.89 → 49.33 TFLOPS（1.65×）。完整报告见
  [docs/ada28e0-uefi-report.md](docs/ada28e0-uefi-report.md)。

- **RTX 16 系（Turing）理论兼容**，欢迎大家实机测试。

三代机制差异见 [兼容性矩阵](#兼容性矩阵)（Turing 的 FEAT 块在 0x4096xx、
FLR 会重锁 PLM、Windows 驱动需 GSP 模式——均与 Ada 不同）。

## 使用（RTX 40 系笔记本/台式，AD10x 家族）

### U 盘根布局（FAT32；`make-usbroot.ps1` 自动产出，手摆按下表）

| 文件 | 作用 | 必需 |
| --- | --- | --- |
| `\EFI\BOOT\BOOTX64.EFI` | 本工具（先备份该路径原有文件） | ✅ |
| `booter_load_ad102.bin` | AD102-family Booter bundle（BTL1） | ✅ |
| `gsp_boot_ad102.bin` + `gsp_boot_desc_ad102.bin` | GSP-RM boot 段 + v5 描述符 | ✅ |
| `fwimage_ad102.bin` | GSP-RM 固件镜像（~80 MiB，radix3 装载） | ✅ |
| `fwsignature_ad102.bin` | GSP-RM stock 签名（WprMeta 引用） | ✅ |
| `scrubber_ad10x.bin` | 内核同款 SEC2 scrubber（BSI handoff 1→3） | ✅ |
| `v67-ad102-four-plm.bin` | **V67 四写 payload——放上即解锁模式** | 可选 |
| `tx-require-d0.txt` | 入口非 D0（休眠恢复）时直接 abort、零写入 | 可选 |
| `tx-select-bdf.txt` | 多卡钉卡：内容 `bb:dd.f` 或 `dddd:bb:dd.f`（十六进制） | 可选 |
| `i_know_what_i_am_doing.txt` | **武装模式**：确认环节变为 5 秒倒计时后自动运行，倒计时内按任意键终止（无人值守/自动化用；文件存在即意图声明，内容忽略） | 可选 |
| `chain-target.txt` | 链回目标钉定：第一个非空且非 `#` 的行 = 优先尝试的 EFI 路径（如 `\EFI\limine\limine_x64.efi`），逐卷尝试；全卷找不到则回退默认顺序并留痕（issue #8 建议） | 可选 |
| `no-reconnect.txt` | 事务完成后跳过目标卡 GOP/控制台重连：重连只是提前恢复画面（bootmgfw 会自建控制台），个别 AMI 固件在重连时卡死/复位平台（见下方已知问题）——日志若停在 `RECONNECT: ConnectController ... begin`，重跑时在盘根放此文件 | 可选 |
| `\loader.log` | （运行产物）全部阶段日志，evidence record | 输出 |

表中使用 Ada 资产名；GA10x 和 Turing 分别在 maker 中选择 `ga102`、`tu10x`。启动文件放在 `\EFI\BOOT\BOOTX64.EFI`，所选家族的 `.bin` 与可选标记文件放在盘根。当前 maker 输出为平铺目录，还含抽取中间文件：按脚本末尾 `expected` 清单选取该家族文件，并为启动文件建立 `EFI/BOOT/` 目录，不必复制全部中间固件镜像。

不放 `v67-*.bin` 即为**回放/诊断模式**：走完全链但不执行解锁载荷。它仍会断开驱动、启动签名固件，并可能复位 GPU，属于写事务；只读采样应使用 `cargo efi` 构建。日志中 `family 'xxx' transaction track not enabled`
表示该芯片族尚未开放事务（20/30/40 三族均已接入；缺资产则跳过）。

**多卡**：机内多张受支持的卡时，缺省按固件枚举顺序**逐张处理**——一张
已解锁（EALREADY）或不受支持会自动让位给下一张，单张失败只对那张卡做
emergency FLR、不阻断后续；控制台在另一张卡上时全程有屏（GOP 只断目标
卡的）。`\tx-select-bdf.txt` 钉死单卡：内容与哪张卡对不上则拒绝猜测、
本轮零事务（钉的卡不在了也绝不默默跑别的卡）。

### 操作流程

1. BIOS：关 Secure Boot。显示模式/进入方式按下面的实测表选——**推荐独显直连**，
   混合模式只在"重启、且 dGPU 有电"这一种进入方式下可用（见原因分析）。
2. U 盘启动 → 版本屏 → **空格/回车确认**（其他键均中止；30 秒无操作自动回 Windows）。若 U 盘根
   存在 `i_know_what_i_am_doing.txt`（武装模式），确认环节变为 **5 秒倒计时后
   自动运行**，倒计时内按任意键终止——文件本身就是明确的运行意图声明，供
   自动化/无人值守场景使用；缺省（无此文件）永远是"必须有人按键确认"。
3. 之后全自动：事务完成或失败都会自动链回 bootmgfw 进 Windows，全程日志
   落 U 盘 `\loader.log`。引导器查找顺序固定为 **bootmgfw → 各发行版
   shim/grub/systemd-boot/Limine → U 盘 `\EFI\BOOT\BOOTX64.EFI`**；只有走到最后
   这一档（即本工具自己所在的通用路径）才给 5 秒窗口，按任意键回固件
   shell（此时解锁已完成、日志已落盘，可从 shell 手动启动任何系统）。
   双系统用户可用 `\chain-target.txt` 把自定义 EFI 路径插到最前（见上表）。
4. 每次冷启动重放一次（解锁易失，无持久化、无刷写、断电即清）。

V67 payload 与全部资产再生成：`cargo run -p assets -- generate-ad102` /
`release/make-usbroot.ps1`（从钉 SHA-256 的 610.43.03 公开输入本地再生，
产物 hash 硬校验；期望值见报告哈希表）。

release 包只发脚本与自产二进制（`BOOTX64.EFI`、`assets-x86_64-windows.exe`、
`assets-x86_64-linux`、`make-usbroot.ps1`、`make-usbroot.sh`、
`enable-turing-unlock.ps1`），**不含任何 NVIDIA bin**——
全部固件件由 maker 从上面钉 SHA-256 的公开输入现场再生：

```powershell
.\make-usbroot.ps1 -Family ad102 ...   # 40 系（默认；rtx40 / 40 同义）
.\make-usbroot.ps1 -Family ga102 ...   # 30 系（rtx30 / 30 同义）
.\make-usbroot.ps1 -Family tu10x ...   # 20 / 16 系（rtx20 / 20 同义；GSP 门强制）
```

Linux/WSL/Git Bash 等价：`./make-usbroot.sh`，参数与 PS1 一一对应
（`--family/--out/--work/--assets/--boot-efi/--proven-blob/--skip-gsp-check`）。
release 包内含 Linux 版管线 `assets-x86_64-linux`（glibc ≥ 2.34，Ubuntu
22.04+/Debian 12+ 均满足），脚本优先取用并自动补执行位（zip 不带 Unix 权限位，
脚本本身用 `bash make-usbroot.sh` 运行或先 `chmod +x`）；仓库检出里找不到该
文件时自动 `cargo build -p assets` 兜底（需 Rust 工具链）。tu10x 的
GSP 门在 Linux 端读 nvidia-smi 运行态（Linux 专有驱动 545+ 对 Turing 及之后
架构缺省运行 GSP-RM，无需 EnableGpuFirmware 类开关；门拦的是"驱动过旧"与
"被 modprobe 配置强关 GSP"两种 legacy 态），并会探测本机 RTD3 给出 issue #9 提示。

Turing 的 proven BL 归档 `gsp_rm_boot_dbg.bin` 由脚本自动从钉 SHA-256 的
cmp50hx 仓库下载；离线时用 `-ProvenBlob <path>` 指本地文件。

### 显示模式 × 上次离开方式：可用性实测（G614JV / 4060L / 三模完整支持，2026-10-26）

| 上次离开方式 | 本次启动后的 dGPU | 本工具 |
| --- | --- | --- |
| 混合模式 · 纯 iGPU 关机 | 冷：无 GOP，BAR0 全 `0xffffffff` | ❌ claim 前置即报 "dGPU powered off (D3cold)" |
| 混合模式 · 混合关机 → 重启 | 热：有 GOP，寄存器可读 | ✅ 全链跑通（四写 + 选择器穿 FLR，即 evidence 里的成功日志） |
| 混合模式 · 休眠/唤醒之后 · D3 门控态 | 停在 D3、域门控（VGA_WORKSPACE 读回 `0xbadf1301`） | ❌ claim 前置即报 |
| 混合模式 · 休眠/唤醒之后 · 事务可跑 | 寄存器可读、全链能绿，但卡是热的 | ⚠️ 事务绿 ≠ 解锁有效：出口后可能被作废，先看 `HANDOFF-OUT` 并做 Windows 侧读回（`docs/warm-resume-entry.md`） |
| 混合模式 · 休眠/唤醒之后 · **TU106 实测** | 寄存器可读、全链绿 | ✅ **写入解锁成功且 Windows 验证通过**（2026-10-27，2070 台式实测：休眠恢复 → 进 U 盘 → 全链 → 解锁存活进 Windows）。上两行的"作废"警告在 TU106 + GSP 模式组合下未复现 |
| 独显直连 · 休眠或重启 | 始终是 panel owner、始终有电 | ✅ 均可用 |

操作建议：**从 Windows 选"重启"**（或冷启后直接从启动菜单进 U 盘）再运行；不要用
"关机再开"的 iGPU-only 路径，也不要在休眠唤醒后直接进 U 盘。已实现 PM 预检：入口
非 D0 会打 `PM: WARM ENTRY` 并把卡拉回 D0（读回确认），要更严格就在 U 盘根目录放
空文件 `\tx-require-d0.txt`——此时 warm 入口直接 abort、零写入零复位。

**原因分析（不是模式好坏，而是固件为谁无条件跑 POST）**

固件只在"自己要用来显示的那块 GPU"上无条件执行 VBIOS POST/devinit：

- 独显直连时 dGPU 是 panel owner。冷启、热重启、S3/S4 唤醒后的任何路径，固件都必须
  重新初始化它，dGPU 一直有电、有 GOP、寄存器域可读——所以这里休眠和重启都可用。
- 混合模式下 iGPU 能承担显示，固件对 dGPU 只是尽力而为：冷启与混合重启路径会 POST
  它（因此"混合关机→重启"可用，这也正是那份 4060 成功日志的状态）；但
  - 纯 iGPU 关机会让 EC 在 POST 之前就断掉 dGPU 供电（D3cold），BAR0 全 1；
  - 休眠走的是 resume 路径而不是 POST：固件不重跑 devinit，EC 把 dGPU 停在 D3，
    PRI 域门控——VGA_WORKSPACE 读回 `0xbadf1301` 就是它在寄存器层的投影。
- 本工具在驱动加载前工作，没有 ACPI 解释器，无法像 Windows 驱动那样跑
  `_PS0`/PR3 给卡上电，因此这三种失败态只能靠换进入方式规避，不能靠代码绕过。

## Linux 用户注意：runtime D3（RTD3）会在交接后擦掉解锁（issue #9）

解锁是易失覆盖（设计上无持久化、断电即清），而 **D3cold 对相关电源域等价于
断电**。Linux 下若 NVIDIA 驱动启用了 runtime D3（RTD3），GPU 一空闲就会被
挂起进 D3cold，解锁状态随之丢失——典型症状：`loader.log` 全绿（marker/PLM
读回全部正确）但进入系统后性能回到 stock（[issue #9](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/9)，
4050 Laptop `28a1` + Ubuntu 实测：GPU 停在 D3cold/suspended、生效的
`DynamicPowerManagement` 为 `0x02`）。

Ubuntu 及衍生版在受支持的笔记本上缺省启用 RTD3（发行版文件
`/usr/lib/modprobe.d/nvidia-runtimepm.conf` 写死
`options nvidia NVreg_DynamicPowerManagement=0x02`）。关闭它有三个坑，
少一个都可能白改：

1. **复用发行版文件的 basename**。modprobe.d 规则是"同名文件按目录顺序
   先到先得，后续目录的同名文件**整体忽略**"（modprobe.d(5)）；而**不同名**
   文件的 options 是累加关系，实测会输给发行版值（issue #9：先试了不同名的
   `nvidia-pm.conf` 写 `0x00`，`modprobe -c` 显示两条、驱动仍取 `2`）。复用
   basename 即整体遮蔽发行版文件：

   ```bash
   echo 'options nvidia NVreg_DynamicPowerManagement=0x00' | \
       sudo tee /etc/modprobe.d/nvidia-runtimepm.conf
   ```

2. **重建 initramfs**——NVIDIA 模块在 initramfs 内早加载，配置不进镜像等于白改：

   ```bash
   sudo update-initramfs -u      # Debian/Ubuntu（Fedora/RHEL：sudo dracut -f）
   ```

3. **重启后验证生效值而不是文件内容**（与本项目宪法第 1 条同款逻辑）：

   ```bash
   grep DynamicPowerManagement /proc/driver/nvidia/params   # 应为 0x00
   modprobe -c | grep -i DynamicPowerManagement             # 只应有一行 0x00
   ```

排障速查——"日志全绿但系统内无提升"先过这张表再报 issue：

| 检查 | 命令 | 解锁存活应看到 |
| --- | --- | --- |
| 驱动参数生效值 | `grep DynamicPowerManagement /proc/driver/nvidia/params` | `0x00`（`0x02/0x03` = RTD3 开着） |
| 合并后配置 | `modprobe -c \| grep -i DynamicPowerManagement` | 仅一行，且为 `0x00` |
| GPU 电源状态 | `cat /sys/bus/pci/devices/<bdf>/power_state` | `D0`（`D3cold` = 已失电，解锁已丢） |
| 挂起旁证 | `nvidia-smi` | 有正常读数（GPU 挂起时全 N/A） |

同理：S3/S4 挂起再唤醒也是失电，重跑一次 U 盘事务即可。D3cold 是最隐蔽的
一种——不报错、不黑屏，只是安静地把解锁擦掉。Linux 版 maker
（`make-usbroot.sh`）构建 U 盘时会探测本机 RTD3 状态并给出提示。

## 雷电（Thunderbolt）/ 热插拔根口 eGPU：解锁易失环境（已知限制）

2026-10-28 用户反馈：雷电拓展坞下 eGPU 在 EFI 阶段全链解锁成功（读回全绿），
进入 Windows 后解锁丢失。机理上这是环境问题而不是事务问题：

- EFI 阶段能解锁，是因为固件在 POST 时已建好 PCIe 隧道，eGPU 对 DXE 可见；
- 进入 Windows 后，TB 连接管理器（固件 ICM / Windows TB 栈的审批与隧道重建）
  可能**拆除并重建隧道**，对 eGPU 等价于一次 hot reset / PERST#；坞内供电管理
  （TB 控制器 D3cold、S0ix、enclosure 供电策略）还可能直接断电。解锁是易失
  覆盖，任何 conventional reset（hot reset/SBR、PERST#）或失电都会把它擦掉
  ——与 issue #9 的 RTD3/D3cold 同属一族。

参考系：cmp170hx 实测 Thunderbolt 3 eGPU 连 kmod 解锁本体都失败
（`Booter Load 0x15`）；Oculink 可用（本质是延长线）。**受支持环境以原生
PCIe 插槽 / 独显直连为准**；雷电/热插拔根口用户请把"解锁后进 OS 无提升"
视为预期行为，不必提 issue。

**雷电用户实测征集（2026-10-29）**：`tools/ssdiag` 让这个场景有了一个低成本
实验，欢迎实测并把数据发上来（Discussions 或新开 issue，附完整输出）：

1. U 盘解锁 → 进 Windows 后先跑 `ssdiag` 看判定。**门开 + 值回出厂**
   （SS0/SS1 非 0x88888888/0x8）→ 属"值被重推导"，`ssdiag --reapply` 可
   免重启写回；**门已被关**（读回 badf）→ 隧道重建做了真复位，只能重跑
   U 盘，此时 reapply 无意义。
2. 判定为"门开"时跑 `ssdiag --reapply`，用同一 GEMM 负载测 reapply 前后
   吞吐（cuBLAS/GPU-Z 均可）。
3. 把 `ssdiag` 两次输出 + 吞吐对比贴上来。无论成功失败都是数据点：它首次
   能区分"隧道重建只重推导了值"还是"做了真复位"——这正是此前无法判断、
   只能整体标注为已知限制的原因。

边界照旧：坞休眠/断电（D3cold）后 reapply 无意义（卡已失电，重新上电过
PERST# 等于重擦）；纯链路重训（见下）不影响解锁，无需 reapply。

顺带澄清一个常见混淆（PCIe 语义）：**纯链路重训（Retrain Link / TLS 变更 /
ASPM 配置触发的 LTSSM 重跑）不复位 endpoint**——设备寄存器与配置空间全部
保留，解锁不受影响。会丢解锁的是 conventional reset 家族（hot reset/SBR、
PERST#、Turing 上的 FLR 重锁）与失电（D3cold/S3）。内核正常引导不会无故
重训；x86 上内核"动链路"的真实场景是 AER/DPC 错误恢复（先出错才有，Turing
GSP 门前置掐断的就是这条触发源）与热插拔重扫——后者就是本节的雷电场景。

## 与 nvoc kmd 功耗墙解锁配合使用

本工具与[本人](https://github.com/physicsdolphin)主导开发的 **[Skyworks-Neo/nvoc](https://github.com/Skyworks-Neo/nvoc)** 的 kmd
功耗墙解锁**兼容且正交**：本工具在 UEFI 预驱动窗口解除 tensor 发射速率钳制
（FEAT/WPR/WPRCFG PLM），nvoc kmd 在操作系统内解除功耗墙（目前适用移动端 GPU（实测可用） 和 10系/20系桌面端 GPU（驱动读回有效但未测试效果，我手上的这类卡全都shuntmodded改成“0W”了）），nvoc set-volt-rail-limit 解除 slot 0,1,2 的Target Wall，Vbios Wall，VRM Max Wall(Vop)电压墙（适用于所有10系后GPU） ——两者作用于
不同层级（驱动“下面”+驱动“上面”），可叠加，推荐配合使用以获得极限性能。

<img width="1280" height="595" alt="image" src="https://github.com/user-attachments/assets/1f60b619-b367-4823-9764-1caa44575501" />


### 组合实测极限（RTX 4060 Laptop 28e0，FP16 GEMM + nvoc）

> 本文所有 FP 精度吞吐数据均由
> [Skyworks-Neo/nvoc](https://github.com/Skyworks-Neo/nvoc) 的
> **[cli-stressor-cuda-rs](https://github.com/Skyworks-Neo/nvoc/tree/main/cli-stressor-cuda-rs)**
> 测得。

| 工况 | 数值 |
| --- | --- |
| FP16 GEMM 4096（仅本工具解锁） | 49.0–49.3 TFLOPS（stock 29.89，1.65×） |
| FP16 GEMM 8192（+nvoc 功耗墙） | **60.8–63.1 TFLOPS**（相对 stock ≈ **2.1×**） |
| GPU 核心时钟峰值 | **3045 MHz** @ 1.200 V |
| 板级功耗峰值 | **206.8 W**（芯片 ~163.7 W + 显存 21.8 W + PWR_SRC 24.8 W；16-pin 164.4 W @ 19.8 V） |
| 温度 | Hot Spot 峰值 **101.8–104.4 °C**，核心 79–81 °C，显存 58–66 °C |
| 限流原因 | PerCap Reason 在 **Pwr**（功耗墙）与 **Thrm**（热墙）间切换 |

判读：解锁后卡在满载时先顶功耗墙（Pwr），nvoc 放开功耗墙后转为顶热墙
（Thrm）——**3045 MHz / 206.8 W / 104 °C Hot Spot 就是这块 8 GB 4060
Laptop 的物理极限区间**，再往上任何优化都只能来自降温。

### 解锁的本质：被禁用的晶体管恢复工作，而非超频

额外的算力不可能是白嫖得到的，仍然遵循硅芯片电气、热力学等物理定律！

被钳制的 SM 发射率是**被NVIDIA刻意软件层面用efuse在固件中物理关闭的tensor数据通路**（manufacturing
segmentation），不是降频或限功率——频率、电压、功耗墙都与它无关。对照
实验（RTX 4060 Laptop，ASUS ROG Strix G16 **G614JV**，FP16 tensor，
同卡同精度唯二变量为发射率与功率墙）：

| 状态 | 频率 | 芯片功耗 | 墙 | FP16 tensor |
| --- | --- | --- | --- | --- |
| stock | **2.52 GHz** | 95~105 W | **电压墙** | 30.5 TFLOPS |
| 只解锁 SM 发射率 | **2.37 GHz** | 130-140 W | **撞功耗墙** | 47.50 TFLOPS |
| 只解锁功率（+nvoc 放开电压墙+极限超频） | **3.00 GHz** | 155–160 W | **不撞功耗墙** | 35.9 TFLOPS |
| 解锁 SM 发射率+功率（+nvoc 放开电压墙+极限超频） | 平均 **2.89 GHz** | 190–208 W | 撞温度墙+供电极限 | **64.30 TFLOPS** |

发射率解锁后频率**下降 3.7%**、功耗上升 34%，吞吐却提升 **79%**——多出的
每 FLOP 都不可能来自频率或供电，只能来自**完整晶体管不再空转**： segmentation
掩码放行后，原本每个时钟沿都在执行 NOP 的挂起槽位重新装载真实指令，
参与发射的tensor核心数量恢复设计全集——功耗上升正是空转变工作的直接体现。
这也是为什么单纯超频/超压/放功耗墙永远追不平差距（stock 在超频到 3.0 GHz、不撞
任何墙也只有 35.9 T），而解锁在更低频率下直接翻倍。

> **⚠️ 硬件风险警告**
> 两项解锁叠加后芯片功耗与热密度会**大幅超出原厂设计包络**（上表为实测
> 极限：板级 200+ W、Hot Spot 105 °C 附近持续）。使用前必须自行确认并
> 持续监控：
> - **散热**：液金/硅脂状态、风扇策略、均热板与热管规格——Hot Spot 与核心
>   温差 >20 °C 时意味着界面材料已接近极限；
> - **供电**：适配器功率、VRM 余量、16-pin/电源链路温升（实测 16-pin
>   164 W @ 19.8 V）；
> - **监控**：GPU-Z/HWiNFO 的 Board Power、Hot Spot、PerCap Reason 三项
>   常驻；Hot Spot 持续 >100 °C 属于裸奔区。
>
> **因超功耗、超压、超温导致的芯片、供电或整机物理损坏，本项目概不负责**
> ——解锁是易失的，但烧坏的硅片不会恢复。
> <img width="287" height="520" alt="image" src="https://github.com/user-attachments/assets/d5e5fb50-4504-4b76-ba43-56e5db078b8b" />


## 工作原理速览

NVIDIA 的算力钳制是 PRI 总线上的硬件门（PLM，默认 `0xffffff8f` = 仅
heavy-secure 固件可写），主机/驱动写受保护寄存器被静默丢弃。唯一解锁路径是
让**已签名的固件自己**在特权态写——本工具在 UEFI DXE 窗口（驱动加载前的唯一
等价时窗）复刻内核驱动的完整引导链，并在"送 Booter"一步注入社区逆向出的
V67 载荷：

```
claim(断 GOP/驱动+独占) → scrubber(BSI handoff 1→3) → FWSEC-FRTS(WPR2)
→ booter(正确签名+BROM 验签, 邮箱=WprMeta) → [V67: 签名 DMA 无界拷贝
溢出 DMEM 栈 → ROP → FEAT/WPR/WPRCFG/FBPA 四写 + marker 0x67b00001]
→ selector 写入(PLM 开后主机写生效) → 签名还原 → FLR(冷卡交还) → bootmgfw
```

全程只读回验证（marker + HALT + PLM 逐项读回 + FLR 后 AON 存活），失败即
拒绝并 FLR；无持久化、无刷写、断电即清。详见
[docs/ada28e0-uefi-report.md](docs/ada28e0-uefi-report.md)。

## 兼容性矩阵

判定依据：本工具按**芯片家族**路由（同一家族共用同一 Booter/payload 资产，
与上游 `ad102_family()`/GA10x 路由一致）。层级含义：
✅ 实测验证（本机或注明来源的社区实机跑通全链+payload）；🟢 上游硅验证（同家族在 kmod 侧已验证，
本 EFI 路径同构，预期兼容）；🟡 路径存在未验证（代码就绪，无硅证据）；
❌ 不兼容。

**2026-10-28 起路由全量放开**：Turing / Ampere（GA10x）/ Ada 三族按 NVIDIA
官方驱动列表（open-gpu-kernel-modules README "Compatible GPUs"，217 ID）**全段
放行**——`0x1e02..=0x1fff`、`0x2182..=0x21d1`（Turing，1650 SUPER `2187`、
1630/1650 移动版均在段内）；
`0x2200..=0x22ff`、`0x2400..=0x25ff`（Ampere；0x23xx=Hopper 不在列）；
`0x2600..=0x28ff`（Ada）。同族任何 ID（移动/工作站/CMP/魔改卡）不再需要逐个
加表；未列 ID 按所属家族行继承证据层级。**GA100（A100 `0x20b0`/`0x20c2`、
A30 `0x20f1`）不属于 GA10x，已从 ga102 路由剔除**（此前为跨族错配隐患）；
Blackwell（0x2bxx+）不在范围。

| 家族 | 卡（device ID） | 层级 |
| --- | --- | --- |
| AD107 | 4060 Laptop **28e0** | ✅ 实测（本报告）；[issue #63](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/63) 独立复现（Acer PHN16-71，v2.1.0，三份启动日志全链读回一致；MMA 指令微基准 33.512→67.527 TFLOPS（**+101.5%**，24 SM），FP16 GEMM 56.8 @180 W；11 次记录与原始数据包见该 issue） |
| AD107 | 4060 2882 / 4060 Laptop 28a0 / 4050M 28e1 | **28a0 ✅ 实测**（[issue #50](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/50)，2026-10-29，v2.1.4，MECHREVO 极光15 独显直连 140 W）：全链一次通过（WPR2 验证→marker→四写→签名还原→FLR 后 AON 存活）；FP16 8192 **26.37→50.29 TFLOPS（×1.91）**，功耗钉 TGP 上限、频率反降 1.5%——TGP 瓶颈指纹；Intel RPL 根口桥 BME 修复（PR #5）实测触发。其余（2882/28e1）保留上游证据层级 |
| AD107 | 4050 Laptop **28a1** | ✅ 实测（2026-10-26，v0.1.3）：全链通过（WPR2 验证→booter marker→四写→选择器→签名还原→FLR 后 AON 存活）；FP16 tensor **22 → 38-42 TFLOPS（1.7-1.9×）**，完整性自检绿。门控值 `0xbadf1301` 走容毒回退 |
| AD106 | 4060Ti 2803 / 4060Ti 16G 2805 | **2803 ✅ 实测**（PR #5，AD106 桌面：回放 + 解锁 + 四写读回，FP16 46.78→75.28 TFLOPS）；**2805 ✅ 实测**（[报告](docs/evidence/ad106-2805-nuc13/report.md)：ASUS `1043:8921` + Intel NUC13RNGi9，全链读回，FP16 45.78→75.24 TFLOPS，ComfyUI 稠密 bf16 ×1.57）。**2803 追加平台实测**：[issue #51](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/51)（AMD APU 桌面，v0.2.0，FP16 45.96→75.40、BF16 44.61→78.72，与 PR #5 结果复现）；[issue #33](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/33)（MSI B550 无内显单卡，v2.1.4，持续满载测法 FP16 45.72→67.30、ComfyUI ×1.51；附[免 U 盘开机自动解锁方案](https://github.com/chungtair/rtx4060ti-auto-unlock)与 MSI B550 需 BIOS 手动设启动项的坑）；[issue #56](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/56)（与 RTX 5060 Ti `2d04` 双卡环境，Blackwell 主卡被安全跳过=负控数据点，副卡 FP16 44.94→71.81，24 h+ 稳定） |
| AD106 | 4070 Laptop **2820**（魔改卡同 ID） | 🟡 路由已加（2026-10-28）：同 AD106 家族资产、与 2803 同 die 家族（上游表同样缺此 ID）。首次实机请按下文"不兼容/异常时"提交 `\loader.log` 全文，验证后再升级层级；**魔改卡另请附原卡型号与 VBIOS 版本** |
| AD104 | 4070Ti 2782 / 4070S 2783 / 4070 2786 | **2783 ✅ 社区实测**（[issue #3](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/3)，v0.1.4，Palit RTX 4070 SUPER + MSI B760 TOMAHAWK WIFI，220 W）：回放、解锁读回及 FLR 存活通过；8192 GEMM FP16 70.5→100.1、BF16 71.0→107.0 TFLOPS。其余列出的 ID 保留上游证据层级 |
| AD103 | 4080S 2702 / 4080 2704 / 4070TiS 2705 / **4070 2709** / 4090M 2757 | **2709 ✅ 实测**（[issue #8](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/8)，B760 台式独显直连：四写全链读回、选择器穿 FLR；OS 侧 sm_89 mma 微基准 FP16 ~61→**123.91**、BF16 **118.90**、FP8 **243.55** TFLOPS，均 **2.0×**）；**2705 ✅ 实测**（[issue #36](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/36)，v2.1.0，AMD `1022:14db` 桥独显直连：四写全链读回 + 前置 replay 同绿；sm_89 发射率微基准 61.98→**125.29 TFLOPS（2.02×）**，未动功耗墙 143.5 W / 50.9 °C）；**2704 ✅ 实测**（[issue #53](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/53)，v2.1.0，Windows 11 24H2 零售 16 GB 卡：冷启一次通过；PyTorch FP32 累加 A/B FP16 4096³ 100.53→185.50（1.85×）、BF16 4096³ 1.91×，FP32 strict 对照不变）；其余（2702/2757）🟢 上游硅验证 |
| AD102 | 4090 2684 / 4090D 2685 / 4070TiS **2689** | **2689 ✅ 实测**（B 站评论区反馈，2026-10-30 收录；驱动侧确认 `VERDICT [ada]: UNLOCKED`——PLM=0xffffffff、SS0=0x88888888、SS1=0x8、READOUT_1=0x00000000，四写值在驱动加载后仍有效）：FP16 4096 57.00→150.93（**2.65×**）、BF16 4096 57.51→127.32（2.21×）、FP16 8192 55.50→127.28（2.29×）、BF16 8192 54.97→128.39（2.34×）；功耗 227→291.58 W（295 W 上限）、59 °C、`throttle_reasons=0x4`——瓶颈已从发射率转为功耗墙；与 AD103/AD106 实测倍数一致，符合"上游硅验证"预期。**2684 ⚠️ EFI 全链绿、增益部分（跟进中）**（[issue #69](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/69)，v2.1.6，Gigabyte 4090 + AMD 平台，同机 3×1080 Ti 正确跳过=又一负控数据点）：事务/读回/交棒全绿（marker `0x67b00001`、SS0/SS1 写入读回、FLR 后 AON 存活、ssdiag `VERDICT [ada]: UNLOCKED`），但吞吐仅 1.45–1.71×（TF32 8192 70.18→111.77；FP32 shader 对照 1.04× 不变；解锁后 fp16acc/fp32acc 发射率比 1.95→1.52——突破 stock 165.6 上限但约为无钳路径的 66%）。头号疑似功耗墙：报告者功耗滑条已顶 479 W（2× 发射率配 450–600 W 上限通常不足，建议 `nvidia-smi -lgc` 钉低频消融）；报告者另疑 FEAT 扫描读到的 `0x823810=0x00aaaaaa`（nibble 'a'=使能+速率 2）为 payload 未触及的第二速率选择器——暂无证据，复测与消融在该 issue 跟进。其余（2685）🟢 上游硅验证 |
| GA102 | 3090 2204 / **3080 2206** / **3080 LHR 2216** / 3080 Ti 2208 / 3070 Ti 2205 等（0x22xx 全段放行） | **2206 ✅ 实测**（[issue #29](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/29)，v2.1.0，MSI MEG Z490 UNIFY 单卡直连 340 W，Windows 10）：WPR2/PLM 翻转/选择器/签名还原/FLR 读回通过；FP16 4096 48.36→104.55、8192 63.96→99.67 TFLOPS，收益受功耗墙限制（[详情](docs/community-reports.md)）。**2216 ✅ 实测**（[issue #15](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/15)，B550 无 iGPU 单显示：FP16 8192 57.72→94.0–96.9（+63~68%）、BF16 +74~79%、TF32 +52~67%，ComfyUI SDXL +28.8%；per-SM 速率与 3090 Ti 参考卡同带；随附两个 GA10x 判据假阴性修复）。其余 GA102 同族保留全段放行结论 |
| GA103M | 3080 Ti Laptop **2420** | **2420 ✅ 实测**（[issue #27](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/27)，v2.1.0，MSI Vector GP76 独显直连 175 W：PLM 翻转、选择器读回及 FLR 存活；FP16 49.23→77.80/80.35、BF16 50.35→77.64/80.84，FP32 不变；[结果与注意事项](docs/community-reports.md)） |
| GA104 | 3070 2484–248c / 3060 Ti **2489** 等（0x24xx 全段放行） | **2489 ✅ 实测**（[issue #11](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/11)，2026-10-28，v2.1.0，@hunan-NRT，B450M 迫击炮 + 5700X3D）：wait_plm 判据通过（PLM 0xffffff8f→0xffffffff 翻转读回 ~149 ms；SS0/SS1 写入读回 0x88888888/0x8、FEAT_READOUT_1=0）→ 交棒 Windows 正常；卡刷映众冰龙 3060 Ti VBIOS（跨 VBIOS 不影响，FWSEC PROM 读取正常）。**2484 ✅ EFI 实测**（[issue #54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)，Thunderbolt 3 eGPU，v0.2.0 本地构建：UEFI 全链成功、FLR 后选择器存活，但 **Windows 侧状态不保留**（Linux `host_reset=N` 路径可保留；同机 v2.1.5 事务失败未复现）——雷电坞已知限制见下文）。其余 GA104 保留全段放行结论 |
| GA104M | 3070 Laptop **249d** | **249d ✅ 实测**（[issue #28](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/28)，v2.1.0，AMD Cezanne MUX 独显直连 140 W：PLM 翻转 ~158 ms，SS0/SS1、签名还原、FLR 和 HANDOFF-OUT 读回正确；FP16 8192 35.43→56.04、4096 37.82→60.96）。混合输出机器有异常记录（[#28 注意事项](docs/community-reports.md)、issue #6 另一台 249d），见"不兼容/异常时" |
| GA106（含 M） | 3060 2501–2508 / 3060 Laptop 2560 等（0x25xx 全段放行） | **2504 ✅ 实测**（[issue #70](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70)，v2.1.6，ASUS ROG STRIX X470-F（AMI）+ Ryzen 5 5500 独显直连：wait_plm PLM 翻转 ~151 ms、SS0/SS1 写入读回 0x88888888/0x8、FLR 后 AON 存活、HANDOFF-OUT 全对——**GA106 首个全链跑通**；FP16 4096 25.37→45.79（**1.80×**）、BF16 46.31（1.83×）、ComfyUI ×1.62–1.75；stock 25.3–26.1 TFLOPS 与官方标称 25.6 吻合=测量基准旁证。**注意：递归 `ConnectController` 在该 AMI 固件上复位平台（#13 缺陷的首个 AMD 平台复现），`no-reconnect.txt` 绕过后正常**，见已知问题节）。**2520 ✅ 实测**（[issue #71](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/71)，v2.1.6，3060 12GB（标题自述 VRAM 改动卡），华南 X99-4MT：FP16 **+83%**、BF16 **+86%**、FP32 high +60%；**首例 Google Colab 构建 ga102 资产→U 盘引导**的成功报告）。其余（2501–2508/2560 等）🟢 上游硅验证（同族 ga102 资产）。`2560` 首例：[issue #6](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/6) 四写已落 PLM 但被旧 HALT/marker 判据误判为失败（[判据修复溯源](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/6)），选择器未写入，不计成功；该机复测待回传 |
| GA107 | 3050 2507 / 3050 Laptop 25a2 等 | 🟡 路径存在未验证。[issue #37](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/37)（25A2 4 GB）为 wait_plm 入口竞态（payload 先于入口读翻门），修复见该 issue |
| **TU106** | **RTX 2070 1f07 / 2060 Super 1f06** | **1f07 ✅ 实测**（2026-10-27，v0.2.0）：全链通过（FWSEC 点火 → BOOT_DIRECT booter → V67 执行 → PLM 0x409650 全开 → SS0/SS1 写入读回 → 签名还原）；**FP32 累加 36 → 60–63 TFLOPS（+67~75%）**。**前置：Windows 侧 GSP 固件模式（见下节），必须重启生效**。实测机：Intel 10980HK ES（QTJ1）+ 2070 混合输出台式平台；**休眠恢复后进 U 盘写入解锁 → Windows 验证通过**。**1f06 ✅ 实测**（[issue #12](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/12)，2026-10-27，v0.2.0，@vadash）：全链通过（FWSEC → WPR2 验证 → BOOT_DIRECT booter → PLM 全开 → SS0/SS1 写入读回 0x88888888/0x8 → 签名还原 → PLM relock 后解锁值持续 → FLR 按语义跳过）→ 交棒 Windows 正常；**AMD 平台**（AM4/Promontory，桥 BME 修复路径实测覆盖）；8 GB SKU 与 2070 共用 8192 MiB 几何表（其他容量 TU106 SKU 需扩表后才可跑） |
| TU102/TU104/TU116（20 系其余/16 系） | 1e02–1fff、2182–21d1 等 | 🟢 同族资产+社区硅验证（2080Ti 22G chenxluo +51~75%、2080Ti 11G nlzy/haohervchb ~1.6×，同链同构）。**EFI 侧尚无成功实例**，追踪中：2080 Ti 22G `1e07` 0x91（[issue #17](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/17)）、2080 `1e87` 0x91（[issue #62](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/62)）、2080S `1e81` EFI 解锁成功但 GSP 交棒蓝屏（[issue #55](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/55)） |
| Turing 之前（Pascal…） | — | ❌ 无事务轨 |
| Blackwell (50 系) / GA100 | — | ❌ 链路不同，不在范围 |

## Turing（20 系）专属：GSP 固件模式前置（必读）

Turing 的 Windows 驱动**只有在 GSP-RM 固件模式下才接受 pre-OS 解锁状态**；
legacy 初始化路径撞上打开的 FEAT PLM 会 TDR 风暴/Code 43（40 系/Ada 无此
要求，v0.1.x 的 40 系用户不受影响）。**运行一次**（管理员 PowerShell）：

```powershell
tools\windows\enable-turing-unlock.ps1           # 源码树；检测 Turing 卡并写 EnableGpuFirmware=1
.\enable-turing-unlock.ps1                       # release 包内（与 maker 同目录）
tools\windows\enable-turing-unlock.ps1 -Remove   # 回滚到 stock
```

脚本枚举 `HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-...}\` 下的
`MatchingDeviceId`，只对 Turing 设备 ID 段（VEN_10DE，0x1E02–0x1FFF、
0x2182–0x21D1）写 `EnableGpuFirmware=1`。

**必须 Restart（重启）生效，不要依赖休眠恢复**：休眠恢复从休眠镜像还原驱动、
不重新初始化——GSP 模式不生效，而旧驱动会话遇到解锁态会 TDR。流程：运行
脚本 → 重启（或直接从重启进 U 盘解锁）→ 直通 Windows 验证。

Turing U 盘由 maker 现场再生（release 包只发脚本）：
`.\make-usbroot.ps1 -Family tu10x ...`，产物比 40 系多
`sec2_bl_tu10x.bin`（768B，WITH_LOADER FWSEC 需要）+
`v67-tu10x-four-plm.bin`（64000B）。maker 在 tu10x 家族**强制检查**
`EnableGpuFirmware=1` 与 nvidia-smi 的 GSP 固件运行态，任一不满足即拒绝
构建；异机构建加 `-SkipGspCheck`。通过后 U 盘根会落 `gsp-gate.txt` 存档。
TU106 出厂钳制实测：SS0 冷值非默认（0x15000/0x11000/0x0 引导间
漂移）vs 解锁值 0x88888888——"20 系无出厂 segmentation"的旧假设已被修正。

**为什么强制剪裁版 BL 且钉死 hash**：BOOT_DIRECT booter 消费的是 GspRmBoot
镜像前 `0x6000` 字节（`GSP_RM_BOOT_SIZE`，即 24576 字节，SHA-256
`de676bae...`）——这正是 v0.2.0 实测成功那一火的输入（日志
`gsp_boot_tu10x.bin loaded 24576/24576 bytes`）。内核 BINDATA 的 4 KiB v4
图和 GSP ELF 的前 0x6000（`c76c43b1...`）都不是它；maker 对源归档与输出
双层校验，长度或 hash 不符即硬失败。


**Turing 2070 解锁前 VS 解锁后 ，大家有相同卡的可以自己跑一下对比见真知**


解锁前
<img width="800" height="400" alt="image" src="https://github.com/user-attachments/assets/6e61c642-7202-47a6-b6a0-cc2d986892d3" />

解锁后
<img width="800" height="450" alt="image" src="https://github.com/user-attachments/assets/8c103363-8e44-4d46-ab47-a6c063ae1e6f" />


**不兼容/异常时**：请提交 U 盘根的 `\loader.log` 全文到
[Issues](https://github.com/physicsdolphin/nv-uefi-unlocker/issues)（日志含
完整阶段轨迹，可与系统侧状态和测量结果一起用于诊断；发前可脱敏机器信息）。也可以在
[Discussions](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions)
分享成功/失败数据点（卡型号 + 关键日志行 + 判据值），帮助扩展矩阵。

**Linux 用户**：症状为"`loader.log` 全绿但进系统后无提升"的，先过上文
「Linux 用户注意」一节的排障表——RTD3/D3cold 会在交接后静默擦掉易失解锁
（issue #9），这不是事务失败，别急着交日志。

### 已知问题（已在 loader 内修复）：booter status 0x89 = GPU 上游 PCI 桥的 Bus Master 未使能

**现象**：日志走到 `POST-BOOT: booter status 0x89 (nonzero = failed)`，事务以
EIO 中止。`0x89`–`0x90` 是 booter 的 WPR 描述符校验（`wpr_desc_validate`）
错误区间——booter 经 DMA 读回的 256 字节 WprMeta 不是写入的内容。

**根因**（PR #5，[@8bit2qubit](https://github.com/8bit2qubit) 发现并修复）：
booter 阶段是全链**第一个设备发起的主机 DMA**（之前的 scrubber/FWSEC 全是
portal 拷贝与 PROM 读取，不需要设备 DMA）。台式机固件在 boot services 阶段
通常**没有给 GPU 上游 PCI 桥使能 Bus Master**（本 boot 内固件从未用 GPU 做过
DMA——GOP 显示由 CPU 侧 MMIO 完成，而 Bus Master 位只门控**设备发起的**总线
事务），GPU 的 DMA 请求在根端口被终结：GPU 的 PCI status bit13
`Received Master Abort` 置位，读回全 1，描述符校验即失败。这只与桥的
Bus Master 位有关，与芯片组厂商无关：H310 / B760 / B850 台式均实测复现。

**修复**：loader 已内置（PR #5 已合并）——booter 点火前自动为 GPU 上游路径上
所有未开 Bus Master 的 PCI 桥置位并读回确认，事务结束原样还原；日志以
`DMA-PATH` / `BRIDGE-BME` 行留痕。AD106 4060 Ti 桌面（2803）单机单变量实测：
修复前 0x89 → 修复后回放 `MAILBOX0=0`、解锁 marker + 四写全部读回验证，
FP16 46.78 → **75.28** TFLOPS（`docs/evidence/ad106-4060ti-desktop/`，
完整机理见 `docs/bridge-bme-root-cause.md`）。

**历史勘误**：本节曾把 0x89 归因于固件 IOMMU（AMD-Vi）/TSME 翻译——已证伪。
根因是桥 Bus Master；"把 staging 搬进显存绕开平台 DMA"的备选线也被实验否定
（booter 的 DMA 走自己的虚地址映射，恒等指向主机 sysmem，FBIF TARGET 切换
无效），详见 `docs/bridge-bme-root-cause.md`。

**实测不影响**（无需关闭）：

- **SVM（AMD-V）**：R9000P 7945HX 开启 SVM（任务管理器"虚拟化：启用"）
  解锁照常工作；
- **Intel VT-x（虚拟化）与 VT-d**：验证机开启状态下全链实测通过。

> 术语注意：任务管理器的"虚拟化"只反映 SVM/VT-x；桥的 Bus Master 是 PCI
> Command Register bit 2，与"虚拟化"完全无关。

### 已知问题（已在 loader 内修复）：AMI 固件的无路径 GOP 子句柄导致事务成功后平台复位

**现象**：事务全链通过（`MAILBOX0=0x67b00001`、PLM 读回正确、`POST-FLR: verified`），但在进入
Windows 之前整机复位或黑屏卡死。v2.1.5 之前的构建在这种情况下 `loader.log` 为 0 字节，因为日志只在
链式加载前才写盘。

**根因**（PR #13，在 ASUS ROG MAXIMUS Z790 APEX + RTX 4060 Ti 8 GB 上发现）：该 AMI 固件在 NVIDIA
GOP 之上叠加了一个**没有设备路径的 GOP 子句柄**（固件的分屏/控制台句柄）。断开驱动时 NVIDIA GOP
被移除，但这个无路径子句柄保留了下来。随后的**递归** `ConnectController` 会把控制台栈重新绑定到这个
失效的子句柄上，平台在调用内部复位。有两处这样做：post-FLR 判定之后的 `reconnect_all`，以及
`chain_load_bootmgfw` 中类似 `map -r` 的重连（会对包括 GPU 在内的每个 PciIo 句柄递归连接）。

**修复**：已内置于 loader（PR #13；节点对齐的 GOP 归属判断见 `fe92339`）。重连默认仍为递归（单 GOP
主机板行为不变），只有当断开后仍残留无路径 GOP 时才只连接厂商 GOP（非递归）；链式加载前的重连跳过
NVIDIA 显示功能；`loader.log` 在运行过程中即写盘。受影响的主机板日志会出现
`RECONNECT: pathless GOP survived the detach - vendor GOP only (non-recursive)`；若仍有异常，可在 U 盘根目录
放 `\no-reconnect.txt` 完全跳过重连。证据见 `docs/evidence/ad106-asus-z790/`，完整报告见
`docs/ad106-asus-z790-windows-unlock.md`。该缺陷并非 Intel 专属：[issue #70](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70)
在 AMD X470 + AMI BIOS 上复现同一复位（事务全绿后递归重连阶段平台复位），`\no-reconnect.txt` 实测绕过有效。

> 维护者**非全职**，可能无法随时查看并回复 issue，敬请谅解——日志会保留，
> 有进展会回。

## 仓库结构

```
loader/    EFI 应用 crate（no_std, x86_64-unknown-uefi, Rust + uefi-rs）
assets/    host 侧资产管线（钉 hash 下载/校验/抽取/载荷生成，纯 Rust）
sim/       行为级模拟器 + 事务状态机测试（无 GPU 开发）
tools/     ssdiag：Windows 在系统内解锁状态诊断/免重启修复（pmxdrv.sys 内嵌）
docs/      报告、里程碑证据（evidence-as-commits）、寄存器冷值表、开放问题
```

## 安全宪法（副本，与 AGENTS.md §1 等义，违反任何一条即停工）

当前 loader 记录 PROM CRC32 指纹；下列要求中的逐卡 ROM/PROM SHA-256 校验待实现，详见[实现进度](docs/community-reports.md#使用说明与实现边界)。

1. **读回是唯一成功判据**。状态码/返回值/中断/自检输出一律不构成执行证明。
2. **STOP/异常后同会话绝不重试**；完全断电后走正常 Windows 引导恢复。
3. 只读链式加载 ESP 的 `\EFI\Microsoft\Boot\bootmgfw.efi`，**永不覆写 ESP
   任何文件**；替换 U 盘 BOOTX64.EFI 前先备份原有。
4. 无常驻引导项：只从 U 盘/启动菜单一次性进入；不动 NVRAM BootOrder。
5. 每张卡钉死身份（PCI vendor:device + ROM/PROM hash）；换卡/换 ROM 即拒绝。
6. NVIDIA 固件不分发、不入 git；从钉 SHA-256 的官方输入本地再生；
   密钥/解密产物/载荷永不入库。
7. 写操作分级门控：M1 前零写；M3 进入前需用户显式确认；M4/M5 单独立项。
8. 先 mock 后真机：事务代码必须先在行为级模拟器全绿。
9. Secure Boot 关闭或自签证书入 db（优先后者）。
10. 许可：GPL-2.0-only。

## 回滚手册

### 场景 A：黑屏（链式启动失败/事务中断）
未动任何固件配置与 ESP 文件 → 强制重启 → 走原 Windows 引导项即恢复。
U 盘 BOOTX64.EFI 不在启动链上时机器根本不会进我们的代码。

### 场景 B：STOP/异常（事务中途失败）
同会话**绝不重试**（固件可能仍在运行、DMA 缓冲可能仍被占用）→ 完全关机断电
（笔记本按厂商流程放空 EC，ASUS 为拔电按住电源 40 s）→ 拔 U 盘 → 正常
Windows 引导 → 检查设备管理器/事件日志驱动状态 → 记录证据后再考虑下一次。

### 场景 C：Windows 驱动不认卡（交棒后异常）
设备管理器看错误码；事件日志查 nvlddmkm/Xid。解锁效果是易失覆盖（断电即失），
一次冷启动即回到出厂状态。若仍异常：再次完全断电；最后手段 DDU 重装驱动
（本工具不写任何持久存储，驱动状态异常只可能来自交棒时序，不是永久损坏）。

## Windows 侧解锁状态诊断与修复：`tools/ssdiag`

不重启即可回答"这次开机解锁还在吗"，并在 Ampere/Ada 上支持**免重启修复**：
FEAT 门开着但 SS 覆盖值被驱动 devinit 重推导掉时（驱动初始化、睡眠唤醒、
雷电掉电重枚举后），`--reapply` 直接把 SS0=0x88888888 / SS1=0x8 写回并读回
验证（separate-lock 方法，issue #6/#19 谱系）。Turing relock 链在 Windows 内
门是关的，工具保持只读——但 Turing 的 SS 覆盖值在 Windows 会话内持久，
**系统内读回即为有效判据**；门关则重跑 U 盘即可。

```text
ssdiag                        # 诊断：转储每张 NVIDIA 显示卡的寄存器并给出解锁判定
ssdiag --reapply              # 修复：对门开但值不在的卡写回 SS 覆盖值
ssdiag --select 2504          # 多卡系统只看 DeviceID 含 2504 的卡
```

能力与边界（v0.4.x）：

- **BAR0 一律由 OS 自发现**（物理赋值每次引导都变，旧地址曾硬死机；陈旧
  地址一律拒绝）。多卡按 DeviceID 分组逐卡转储，仅 Display 类函数
  （伴生 audio/USB-C 排除）。
- 转储面 = 参考 unlocker 们触碰寄存器的并集：核心/debug 块（PMC_BOOT_0、
  PSTRAPS 解码、FB_SIZE_MIB、GFW、BSI_SCRATCH15、WPR2、MMU PLM+lock、
  GSP/SEC2、熔丝行）、**双家族 FEAT 簇**（Turing 0x4096xx 含 READOUT +
  Ampere/Ada 0x823xxx）、Ada 四写目标（WPR/WPR_CFG/FBPA PLM）、
  AD10x 64G-BAR1 线（CYA/CFG/XVE/cap 镜像）、26 寄存器 PLM survey、
  Ampere FEAT sweep。读回垃圾按 badf 分桶（PRIV_VIOLATION/UNDECODED/
  FLOORSWEPT），判定与地址着色（管道/`NO_COLOR` 退化为纯文本）。
- 家族识别 Maxwell..Rubin（含 GA10x BOOT_0=0xb7xx00a1 怪癖）；FEAT 解锁
  判定仅 Turing/Ampere/Ada 提供该簇映射的卡。
- 对读数退化的卡按需经驱动自家 NVAPI GC6Control 唤醒 GC6/GCOFF 停泊卡
  （笔记本实测）。
- 驱动安全：`pmxdrv.sys`（Intel 签名、构建期 hash 钉死、启动期复验
  fail-closed）**内嵌在 exe 里**，部署到自有服务名 `SSDIAG_PMXDRV`，
  每条退出路径都停服务+删服务+删临时文件；绝不碰其他工具的服务。若
  nvoc 等已部署 PMXDRV 设备则直接借用、零服务改动。部署路径需管理员
  （借用已运行设备可免提权）。
- 雷电口 eGPU：仅 rail 不掉电的环境下重apply 有意义（隧道重建≈热复位，
  见上文雷电小节）。

获取：v2.1.5 起 release zip 附带 `ssdiag-x86_64-windows.exe`；源码在
`tools/ssdiag`（独立 crate，`cargo build --release` 即可重建，详见其
`README.md`）。

<img width="750" height="780" alt="image" src="https://github.com/user-attachments/assets/d475654e-bc0a-4def-b48a-05ff6095b118" />

## 构建

```
cargo build --release   # 宿主侧 crate（assets/sim）——loader 不在 default-members
cargo efi               # EFI 只读构建（M0/M1 诊断模式）
cargo efi-tx            # gated 事务构建（含写路径；仅显式需要时使用）
cargo xt                # 全部宿主测试（assets + sim；数量以当前输出为准）
```

裸 `cargo build --workspace` 会以宿主三元组构建 loader 并报错——loader 只能
为 `x86_64-unknown-uefi` 构建，请用上面的别名。

## 社区贡献与参照源

本项目是社区工作的直接受益者与衍生者，完整致谢见
[docs/ada28e0-uefi-report.md](docs/ada28e0-uefi-report.md) 的同名章节。核心：

- [gpu-unlock](https://github.com/Zagitta/gpu-unlock)（GPL-2.0）——事务链移植母本
- [gpu-unlock-ada](https://github.com/JustAResearcher/gpu-unlock-ada)（GPL-2.0）——
  AD10x 四写链源头；28e1 报告（Windroid234）是本项目直接依据
- [cmp170hx](https://github.com/Consensus-Protocol/cmp170hx)——PLM 语义与验证方法论
- [cmp90hx-unlock](https://github.com/WildFlash1st/cmp90hx-unlock)——V67 链源头
- [AD103_win_unlocker](https://huggingface.co/TOTORONG/AD103_win_unlocker)（TOTORONG）——
  Windows EFI 载具工艺先例
- [open-gpu-kernel-modules](https://github.com/NVIDIA/open-gpu-kernel-modules)（MIT）——
  引导路径一手权威

许可：**GPL-2.0-only**（`LICENSE`）。NVIDIA 固件与 VBIOS 版权归 NVIDIA，按其
原许可条款从钉版公开包本地再生，不在本仓库分发。

## Star History

<a href="https://www.star-history.com/?repos=physicsdolphin%2Fnv-uefi-unlocker&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=physicsdolphin/nv-uefi-unlocker&type=date&legend=top-left" />
 </picture>
</a>
