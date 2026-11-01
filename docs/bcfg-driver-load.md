# BCFG driver-load 形态——机制、操作与安全准则

> 2026-11-01。PR #100 的第三种交付形态：驱动文件放在文件系统上，用 UEFI Shell 的
> `bcfg driver add` 登记为 `Driver####` 加载选项，由 BDS 每次引导自行加载。
> 零固件镜像改动、零刷写；**拔掉介质/删除文件 = 当次引导恢复原样**。
> 事务内核与 FFS 形态完全共用（同一 `nvunlock-dxe.efi`、同一 NVUX2 授权、同一
> fail-closed 门组）；差异只在投递与校验层。验证记录见
> `docs/evidence/dxe-bcfg-load/`（QEMU + EDK2 Shell v2.2）。

## 1. 形态选择

| | USB app（现状） | FFS 插固件（PR #100 主形态） | **BCFG driver-load（本文）** |
| --- | --- | --- | --- |
| 载体 | U 盘上的应用 | 驱动 PE 包成 FFS 写进 SPI flash | 驱动 .efi 放卷上 + NVRAM 一条 `Driver####` 指针 |
| 固件侵入 | 无 | 改 BIOS 镜像（刷写） | **无刷写**，仅 NVRAM 配置项 |
| 每次引导 | 手动进入 | 必然 dispatch | `DriverOrder` 命中即 dispatch；**文件不在→加载失败→跳过继续引导** |
| Secure Boot ON | 需关闭或 db 自签 | **过**（FV 内镜像豁免验签，ReBarUEFI 先例） | **需 db 自签**（外部镜像走 LoadImage 验签） |
| 出问题的恢复 | 拔盘 | 重刷/编程器 | 删文件 / `bcfg driver rm` / 清 NVRAM |
| 变砖类别 | 无 | Boot Guard/刷写类 | **无此类** |
| 适用人群 | 每次手动操作 | 高级用户（接受刷写） | 高级用户（接受 UEFI Shell 操作） |

## 2. 机制原理

### 2.1 NVRAM 侧

`bcfg driver add` 写两个 UEFI 变量：

- `Driver0001`（编号按首个空闲槽递增）：一个 `EFI_LOAD_OPTION` 结构——描述串 +
  驱动文件的**完整设备路径**（如
  `PciRoot(0x0)/Pci(0x1F,0x2)/Sata(0x0,0xFFFF,0x0)/HD(1,MBR,0x00000000,0x800,0x20000)/\NvUnlockDxe-readonly.efi`）
  + 可选载荷；
- `DriverOrder`：加载顺序数组，`bcfg driver add N` 把新选项插到第 N 位。

设备路径绑定的是**分区**（HD 节点含磁盘签名+起始 LBA），不是盘符——换端口/换机
（同盘）不失效；这就是 FFS 形态里同一个"分区身份"原子（`volumes::partition_guid`）。

### 2.2 BDS 侧的加载时序

EDK2 `BdsEntry` 的顺序（简化）：平台 `BeforeConsole` 钩子 → **处理全部 `Driver####`
（LoadImage + StartImage）** → 控制台连接 → boot option 处理（连接启动设备 →
`SetWatchdogTimer` → **信号 ReadyToBoot** → StartImage）。

两个后果，驱动已按此设计（`dxe.rs` 分派：**ReadyToBoot 事件恒注册**；文件来源镜像仅当
**PCI 总线已枚举**（`pci_live`）才走立即路径）：

- **OVMF 类平台**（BeforeConsole 就 ConnectAll）：Driver-option 时点 PCI 已枚举 →
  立即路径在通知外跑（`notification=false`）——此时**控制台尚未连接**，日志只落文件
  （logging 的无-stdout 安全跳过），无键盘窗 → 必须 noconfirm（见 §5）。
- **stock EDK2/多数 AMI**（Driver-option 时点未枚举）：立即路径不触发（`pci_live`=false，
  不会白烧尝试预算）→ ReadyToBoot 回调接管（`notification=true`，同样强制 noconfirm）。
  此时启动设备的卷必然已连接；**armed 卷若是不随启动选项连接的可移动介质，会
  fail-closed 跳过并消耗一次尝试**（见 §6 恢复基线）——所以 BCFG 形态的 armed 卷
  推荐=ESP/启动盘（与 FFS 形态一致的放置建议）。

`ENTERED`（镜像内原子）+ `NvUnlockDxeRan`（NVRAM boot latch）保证：多实例、
立即+通知双路径、boot option 重试，每次引导至多执行一次事务。

### 2.3 与 FFS 形态的授权关系

完全一致：NVUX2 变量（卷身份 + 全控制文件快照指纹）仍是唯一授权；`Driver####`
只解决"代码从哪来"。快照把 `\tx-dxe-arm-install.txt` 的**在场**也钉死——装机卷与
运行卷的控制文件集必须逐字节一致（QEMU 实证：不一致=静默 fail-closed）。

## 3. edk2 Shell `bcfg` 命令解释（Shell v2.2 实测）

`bcfg` 是 ShellPkg 的引导配置命令，操作 `Boot####/Driver####/SysPrep####` 三族。
本文只用 driver 族：

```
bcfg driver dump                       # 列出全部 Driver 选项（Option 序号=DriverOrder 位次，
                                       #  Variable=选项变量名，DevPath=设备路径，Desc=描述）
bcfg driver add <N> <fs-path> "<描述>" # 把文件登记为驱动选项并插到 DriverOrder 第 N 位
bcfg driver rm <N>                     # 删除第 N 位选项（变量与 Order 条目一并删除）
bcfg driver mv <N> <M>                 # 移动次序
bcfg driver opt <N> [数据]             # 读写选项的可选载荷（本项目不用）
```

实测输出样例（QEMU，Shell v2.2）：

```
bcfg: Add Driver0007 as 0
Option: 00. Variable: Driver0007
  Desc    - NvUnlockDxe
  DevPath - PciRoot(0x0)/Pci(0x1F,0x2)/Sata(0x0,0xFFFF,0x0)/HD(1,MBR,0x00000000,0x800,0x20000)/\NvUnlockDxe-readonly.efi
  Optional- N
```

要点与坑（全部实测踩过）：

1. **`add` 的变量名取"下一个空闲编号"**（`Driver0007`），插入位次才是 `N`——反复
   add 会堆积 `Driver0000..000N`（本验证轮因此产生过 8 个重复选项，全指向同一文件；
   多实例无害——`ENTERED`+latch 去重——但请先 `dump` 后 `rm` 清理）。
2. **`rm` 按位次**，删一个后面整体前移；批量清理就重复 `rm 0`。Shell 脚本里
   `reset` 放进 startup.nsh 会造成"每引导一次加一条"的自循环（实测）——脚本化
   bootstrap 必须**自删**（`rm fs0:\startup.nsh`）后再 reset。
3. `bcfg` 在 ShellPkg 标准构建里都有（v2.0+）；厂商定制 shell 可能裁剪。无 shell 的
   板子退路：AMI Setup 的 "Driver Options" 页（若有）做等价登记，或回退 FFS 形态。
4. 操作本身写 NVRAM 变量——**BitLocker 机器先 suspend**（`DriverOrder/Driver00XX`
   进 PCR1 测量，变更可触发恢复密钥提示）。

标准操作序列（Shell 内，驱动已放在 fs0 根）：

```
bcfg driver dump                       # 确认现状
bcfg driver add 0 fs0:\NvUnlockDxe-readonly.efi "NvUnlockDxe"
bcfg driver dump                       # 确认 Option 00 = 本驱动
reset                                  # 重启生效
```

## 4. Secure Boot 与 BitLocker

- **SB-ON：BCFG 形态默认被拒**。外部镜像 LoadImage 走 `Security2` 验签（与 FV 内镜像
  的豁免相反）。两条合规路径：① 关闭 Secure Boot；② 用自签证书签驱动并在 db 注册
  （`sbsign` 签 PE，KeyTool/Setup 的 db 页注册证书——板子相关）。签名后**每次重建
  驱动都要重签**：发布口径=固定 release 二进制 + 钉 SHA-256。
- **BitLocker**：`DriverOrder/Driver00XX` 进 PCR1 → 添加/删除选项都可能触发恢复提示，
  操作前 suspend。FFS 形态动 PCR0（固件本体），同款警告。
- **反面含义（与 FFS 形态的镜像反差）**：FFS 的驱动在 flash 里=信任根的一部分，
  不可被 OS 侧替换；BCFG 的驱动是卷上的普通文件——**SB-ON+已签名时替换者无法通过
  验签（更强）；SB-OFF 时任何能写该卷的进程都能换掉驱动二进制本身**。安全准则见 §7。

## 5. 门控与运行语义（与 FFS 形态的异同）

| 项 | FFS 形态 | BCFG 形态 |
| --- | --- | --- |
| 授权 | NVUX2（卷身份+控制快照） | **同** |
| 执行时机 | FvFile→ReadyToBoot 通知；或 OVMF 类即跑 | Driver-option 时点即跑（OVMF 类）/ ReadyToBoot 通知（stock）——`pci_live` 自动选择 |
| 键盘中止窗 | Fv 路径无（TPL 纪律） | 仅当走立即路径（shell `load` 同款） |
| **无人值守标记** | 建议 | **必须**：通知上下文没有键盘窗，无 `\tx-dxe-noconfirm.txt` = fail-closed 放弃并消耗尝试 |
| 尝试预算 | 2 次/授权，app 重装复位 | **同**（环境未就绪的 fail-closed 也消耗——armed 卷放启动盘可避免）；每次预日志出口都留 NVRAM 面包屑 `NvUnlockDxeExit` |
| 日志 | armed 卷 `\loader.log`（轮换 prev） | 同；Driver-option 时点跑则仅文件（控制台未接） |

## 6. 恢复基线（症状 → 处置）

| 症状 | 根因 | 处置 |
| --- | --- | --- |
| 引导多一行 `BdsDxe: failed to load Driver0001 ... Not Found`，其余正常 | 驱动文件被删/介质拔了 | 无害。要么放回文件，要么 `bcfg driver rm` 清掉选项 |
| 想彻底卸载 | — | `bcfg driver rm <N>`（dump 确认清空）；Setup 恢复默认值也可清 Driver 变量 |
| 驱动不再执行（`dump` 有选项） | 尝试预算耗尽（2 次）或 NVUX2 被删 | 重新走装机流程（app+`\tx-dxe-arm-install.txt`，Space 确认——同时复位预算） |
| 想只停事务、保留登记 | — | 放 `\tx-dxe-arm-remove.txt` 后跑一次 app（删 NVUX2），或 `dmpstore -g 2E7F7609-… -d NvUnlockDxeArm` |
| 驱动为何不执行（无日志可看） | 预算耗尽/指纹失配/缺 noconfirm 等 pre-logging 出口 | 读 NVRAM 面包屑 `NvUnlockDxeExit`（`dmpstore -g 2E7F7609-…`）；装机引导的 app 也会打印 `last driver exit reason` |
| 控制文件集与装机时不一致 | 卷内容变动 | 恢复装机时的文件集，或重新装机（指纹=授权的一部分，fail-closed 是设计） |
| BitLocker 恢复提示 | PCR1 变化 | 输恢复密钥一次，随后 suspend/重新封存 |

**无变砖类别**：本形态从不写 flash；最坏情形=删文件/删变量，任一 UEFI Shell 或
Windows 侧（管理员 GetFirmwareEnvironmentVariable 族）都可完成。

## 7. 安全准则（BCFG 形态专用，其余沿用 FFS 形态条款）

1. **代码来源即攻击面**：SB-OFF 时驱动二进制=卷上可替换文件。强制三选一：
   ① SB-ON + db 自签（最严）；② 驱动放 ESP 且操作者控制 ESP 写权限；③ 驱动放
   物理保管的 U 盘（拔盘=失效，天然回滚）。**禁止**放在多用户可写的共享分区。
2. **NVUX2 快照是装机时刻的封印**：装机后不得在 armed 卷增删改任何
   `\tx-*` 控制文件（含 install 标记本身）——指纹失配=静默不执行。要改控制面：
   先 `\tx-dxe-arm-remove.txt` 卸装 → 改文件 → 重新装机。
3. **无人值守是显式授权**：BCFG 形态必须放置 `\tx-dxe-noconfirm.txt`（通知上下文
   无键盘窗；这不是"跳过安全"，是把"每次引导自动执行"的意图固化进装机快照）。
   反向操作（要键盘窗）= 用 shell `load` 手动跑（立即路径有窗）。
4. **尝试预算=健康探测节拍**：2 次执行后驱动停摆是设计（"固件返回≠OS 健康"）；
   重新装机即复位。不要试图绕过它——它是对抗"挂死后无人值守反复执行"的最后闸门。
5. **S3 resume 不重入**（固件 S3 路径不跑 DXE dispatch）；RTD3/D3cold 与本形态无关。
6. 介质与授权绑定：NVUX2 钉的是分区身份——克隆卷（同签名同 LBA 的镜像盘）会被
   视为同一卷；不要把克隆盘留给不受信环境。

## 8. 验证记录（QEMU + Shell v2.2，2026-11-01）

环境：QEMU 11.1 + 随附 `edk2-x86_64-code.fd`（含 EFI Internal Shell）；MBR FAT32
分区镜像（分区身份=磁盘签名+LBA）；驱动=当前 `nvunlock-dxe`（tx-OFF，ffspack
subsystem 补丁）。日志在 `docs/evidence/dxe-bcfg-load/`。

- **R1 装机**：app（BOOTX64）+ `\tx-dxe-arm-install.txt`，串口投递 Space 过
  `explicit_authorize` 门 → `DXE-ARM: authorization result Ok(())`（NVUX2 落变量、
  预算复位）。重复装机同样成功（复位语义）。
- **R2 登记**：Shell 内 `bcfg driver add 0 fs0:\NvUnlockDxe-readonly.efi "NvUnlockDxe"`
  → `bcfg: Add Driver0007 as 0` + dump 确认 DevPath 绑定分区。重复 add 堆积现象
  与多实例共存亦被记录（8 实例无冲突——ENTERED+latch 去重）。
- **R3 armed 全链**：登记后的下一次引导，BDS 在控制台连接前加载驱动 →
  Driver-option 时点立即路径（OVMF 先枚举；`notification=false`）→ FORM/ARM/
  FAILSAFE/OVERRIDES/FINAL 全链落 `\loader-prev.log`（app 随后启动并把上一份
  轮换走——轮换机制在"驱动+app 同引导"下顺带得到实证）。控制台无驱动输出=该时点
  ConOut 未接（设计行为，非故障）。
- **R4 负控**：全新 vars（无 NVUX2）→ Driver#### 照常加载 → 控制台与文件日志
  **零驱动行**：无授权=完全惰性（含装机提示外的所有阶段）。
- **失败注入**（R3 前身轮次，保留为证据）：控制文件集与装机快照不一致 → 静默
  fail-closed；预算耗尽 → 惰性直至重装。两者都是设计行为的实证。
- 未覆盖：真机 Driver#### 顺序（stock EDK2/AMI 的 ReadyToBoot 路径——OVMF 走的
  是立即路径）、SB-ON 签名链、真机 BitLocker 行为。真机验收与 FFS 形态的 V2/V3
  同批进行。
