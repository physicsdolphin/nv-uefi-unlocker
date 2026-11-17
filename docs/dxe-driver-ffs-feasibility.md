# FFS (DXE driver) 形态可行性调研——固件内置 NvUnlockDxe

> 2026-11-01。回答的问题：能否把现有 uefi-rs 应用（`BOOTX64.EFI`）编译为 FFS 文件
> （`EFI_FV_FILETYPE_DRIVER`），提供高级用户**手动插入 BIOS 固件**的选项、在 DXE 末尾调度。
> 所有仓库事实均已读源码/产物核实；标注【待 V0 实证】的条目是 EDK2 源码推理、需 OVMF 冒烟确认。

## 0. 结论（TL;DR）

**可行，且比直觉更顺**——uefi-rs 0.41 已官方支持 boot-service driver，当前镜像已可重定位，
事务代码可 1:1 复用。三层改动各自明确：

| 层 | 改动 | 难度 |
| --- | --- | --- |
| 编译 | PE subsystem 0x0A→0x0B、拆 lib+双 bin、卷发现改造 | 小 |
| 封装 | 纯 Rust FFS 打包器（.EFI→.ffs，含 subsystem 头补丁） | 小 |
| 装载 | **"DXE 末尾同步跑事务"不可行，必须改为 depex TRUE + ReadyToBoot 事件回调** | 设计点 |

核心设计结论："放在 dxe 末尾加载"字面上 = FFS 插 FV 末尾 + `TRUE` depex（第一遍按 FV 序调度，
插末尾即最后调度）。但**事务本身不能在 entry 里同步跑**：DXE 调度结束时 PCI 尚未枚举、ESP 文件
系统尚未挂载（都在 BDS ConnectAll 里发生），而阻塞等待会卡死 dispatcher（BDS 永不启动 → 死机）。
正确形态是：entry 里只注册 `ReadyToBoot` 事件回调即返回；回调在 BDS 发出 ReadyToBoot
（即将 StartImage bootmgfw）时执行——**这正是现在 USB app 的时窗语义**（PciIo/GOP/ESP SFS
全部就绪、Windows 驱动尚未存在），事务代码路径不变，还省掉 chainload 一步。

发布口径：FFS 本体（无 NVIDIA 资产）可随 release 发布；资产仍放卷上、用户本地再生（1.6 资产纪律
不变）；我们不做安装器、不代刷（issue#73 先例）；USB app 仍是默认路径，FFS 是"高级用户、自担刷写
风险"的平行形态。

## 1. 现状基线（已核实）

- `uefi 0.41.0` + `uefi-macros 0.20.0`，`loader` 单 bin，`tx` 为显式 feature 门。
- `uefi` 0.41 **自带 `driver` 模块**（`src/driver.rs`）：官方文档明确 driver 配方 =
  build.rs 加 `cargo::rustc-link-arg=/SUBSYSTEM:EFI_BOOT_SERVICE_DRIVER`；crate 文档
  声明支持 "applications, UEFI boot service drivers, and EFI runtime service drivers"。
  本项目不需要 Driver Binding 模型（fire-and-forget 事务，不 bind 控制器），只用其子系统约定。
- **`#[entry]` 宏无出口清理**（uefi-macros 0.20 源码：只插入 `set_image_handle` +
  `set_system_table`，返回后全局不重置）→ entry 返回后注册的事件回调可继续使用
  `uefi::boot::*`，无需裸写 `efi_main`。
- 当前产物 PE 头（release/BOOTX64.EFI，231 KiB）：Subsystem=**0x0A**；
  **可重定位**（`.reloc` 0x2f0，DllCharacteristics 0x8160 含 DYNAMICBASE）；
  ImageBase=0x140000000（首选基址，非约束）；SizeOfImage≈248 KiB。
- marker/资产读取走 `get_image_file_system(image_handle())` = **app 自卷**
  （`loader/src/chainload.rs:213`、`loader/src/tx.rs:912`）。FV 内加载的 driver 没有
  SFS 自卷 → 该调用必失败 → 所有 marker/资产读成 "absent"、静默不动作。
  **这是行为层面唯一的结构性改造点**（卷发现策略，见 §5.1）。
- 已知地雷沿用：uefi entry API 在 LTO 下引 wcslen（见 memory `ada-four-write-chain-track`）。

## 2. 编译层：app→driver 的全部差异

### 2.1 PE subsystem 0x0A→0x0B（必须）

EDK2 DxeCore 以 PE subsystem 决定 ImageType【待 V0 实证，MdeModulePkg/Core/Dxe/Image
的 `CoreLoadPeImage`/`CoreStartImage` 语义】：

- `EFI_APPLICATION (0x0A)`：entry 返回后镜像被按 app 语义回收（boot manager 就是靠这个
  语义拿回控制权的）→ 我们的 ReadyToBoot 回调会指向已释放代码，**不可用**；且分配进
  LOADER_CODE 而非 BOOT_SERVICES_CODE。
- `EFI_BOOT_SERVICE_DRIVER (0x0B)`：entry 返回 EFI_SUCCESS 后常驻（代码/数据留在
  boot-services 内存），回调有效。FFS 文件类型 `DRIVER (0x07)` + subsystem 0x0B 是标准组合。

改法权衡：build.rs link-arg 是 **per-crate** 的，会把 app bin 也改成 0x0B——而 boot manager
拒绝把非 APPLICATION 镜像作为启动项。因此：

- **推荐**：`loader` 拆 `lib.rs` + 双 bin（`main.rs` app / `dxe_main.rs` driver），
  FFS 打包器对 driver `.EFI` 做 **post-build PE 头补丁**（Optional Header +68 处 u16：
  0x0A→0x0B，一处确定性写入，check 工具终验）。
- 备选：独立 crate（自己的 build.rs）走 uefi-rs 官方 link-arg 配方。改动更大，暂不取。

### 2.2 代码组织

```
loader/src/
├── lib.rs          // 暴露全部共享模块（tx*、pci、gop、logging、timing、chainload 的卷工具）
├── main.rs         // app bin：banner/按键/chainload 流程不变
└── dxe_main.rs     // driver bin：entry 注册 ReadyToBoot 回调即返回；回调 = tx 主流程
```

driver bin 裁掉 `keywait`/`banner` 大屏版/`chainload`（不再需要链式启动，BDS 自然交棒），
体积应明显小于 231 KiB；事务模块（tx/claim_plm/tx_fwsec/tx_booter/tx_detach/tx_portal/
tx_prep/pci/gop/sample/timing/logging）原样复用。

### 2.3 入口与生命周期

```rust
// dxe_main.rs（示意）
#[entry]
fn dxe_main() -> Status {
    logging::init(VERSION);           // driver 版静默策略见 §5.3
    let ev = boot::create_event(EventType::NOTIFY_SIGNAL, TPL::CALLBACK, ready_to_boot_cb)?;
    boot::register_event_notify(&ev, &EventGuid::READY_TO_BOOT)?;  // gEfiEventReadyToBootGuid
    Status::SUCCESS                    // 返回 = 驱动常驻，等 BDS 信号
}
```

- `gEfiEventReadyToBootGuid` = `7CE88FB3-4BD7-4679-87A8-A8D8DEE50D2B`（UEFI 规范事件组，
  EDK2 `EfiBootManagerBoot` 在 StartImage 前发出）。
- 防重入：回调首行置一次性标志（Boot Service 事件可能随多次 boot option 重试多次触发）+
  复用既有设备态守卫（PLM 已开 → -EALREADY；WPR2 非零 → -EBUSY）。
- `set_watchdog_timer`、`stall`、PciIo、AllocateBuffer 在 ReadyToBoot 全部可用（与 app 同）。

## 3. 装载层：FFS/DXE 机制与"末尾"的真实语义

### 3.1 DEPEX 三案分析（本调研的核心论证）

DXE dispatcher 的行为：反复扫描 FV 中所有 depex 已满足的 FFS，直到无进展；BDS 本身是一个
最后被调度的驱动。**dispatcher 只在 DXE 阶段跑；BDS 启动 ConnectAll 之后 dispatcher 已退出，
不再有"下一拍"**。由此：

| 方案 | 判定 | 理由 |
| --- | --- | --- |
| `TRUE` depex + entry 同步跑事务 | ❌ | 双重死结：① DXE 末尾 PciIo 未安装（枚举/连接在 BDS ConnectAll）；② 阻塞等 PciIo/SFS = 阻塞 dispatcher → BDS 永不启动 → 死机 |
| depex 挂协议（SFS / `gEfiPciEnumerationCompleteProtocolGuid`） | ❌ | 这些协议在 BDS ConnectAll 期间才安装，dispatcher 已退出 → 永不调度（除非平台在 DXE 内自行连接，不可依赖） |
| `TRUE` depex + ReadyToBoot 事件回调 | ✅ | 调度立即完成（注册即返回）；干活点 = StartImage 前一刻，PciIo/GOP/ESP 全就绪 |

`gEfiPciEnumerationCompleteProtocolGuid`（`30CFE3E7-3DE1-4586-BE20-DEABA19BB99C`，
PciBusDxe 枚举完成即安装）作为 **协议 notify**（非 depex）仍是"静默窗"实验的正确钩子：
在 OpROM/GOP 绑定之前动手，比 USB app 更冷。但回调内**不可阻塞等待存储**（会卡住
ConnectAll 序列，NVMe/分区/FAT 绑定排在后面 → 死锁），ESP 资产又未必就绪 → 该变体要求
内嵌资产进 FFS，与"FFS 零 NVIDIA 资产"的发布口径冲突。列为单独门控实验（§10 C6），不进首个交付。

### 3.2 FFS 封装规格（打包器实现依据）

```
FFS file (24B header)
├── Name:        2E7F7609-A04C-4593-B88A-651A03812F2A   (项目固定 GUID)
├── Type:        0x07 EFI_FV_FILETYPE_DRIVER
├── Attributes:  0x00（对齐/校验和属性编码对照 EDK2 Ffs.h 细化；不用 file checksum）
├── IntegrityCheck: Header=补和校验字节；File=0xAA（未设 FFS_ATTRIB_CHECKSUM 的惯例值）
├── State:       0xF8（擦除极性 1 的"构造+头有效+数据有效"；UEFITool 插入时会按目标 FV 规整）
└── Sections（顺序无关，dispatcher 全扫）:
    ├── EFI_SECTION_PE32 (0x10)：driver .EFI（subsystem 已补 0x0B）
    ├── EFI_SECTION_DXE_DEPEX (0x0B)：TRUE depex = 字节 06 08
    └── EFI_SECTION_USER_INTERFACE (0x15)："NvUnlockDxe\0"
```

- 空间不足时用 **LZMA GUIDED section**（0x02，GUID
  `EE4E5898-3914-4259-9D6E-DC7BD79403CF`，DataOffset=0x18，payload=LZMA_ALONE 13B 头+
  流）——几乎所有 AMI/Insyde/EDK2 固件都带 `LzmaCustomDecompressLib`（主 FV 自己就靠它解压），
  典型压缩比 ~1/3。打包器两档都出，默认非压缩。
- 镜像可重定位已核实（§1）→ FV 内任意基址加载无忧，无需 TE 转换。

### 3.3 重放语义（与易失解锁模型吻合）

| 引导路径 | driver 行为 |
| --- | --- |
| 冷启动/暖复位/S4 恢复 | 完整 DXE → ReadyToBoot 触发 → 事务重放（解锁是易失的，每引导重放=设计要求） |
| S3 resume | 不走 DXE dispatch（S3Resume2Peim 脚本直跳唤醒向量）→ **不重入** ✓（与 display-mode 矩阵中"休眠恢复态"教训一致） |
| S0ix (Modern Standby) | 固件不参与，无影响 |

## 4. 时窗对照：USB app vs 固件 driver

| | USB app（现状） | FFS driver（新形态） |
| --- | --- | --- |
| 进入方式 | U 盘/启动菜单一次性 | 常驻固件，每次引导自动 |
| 时窗 | bootmgfw 启动前（用户手动进） | ReadyToBoot（bootmgfw 启动前）——**同一窗口** |
| chainload bootmgfw | 需要（§1 宪法 rule 3） | 不需要（BDS 自然交棒） |
| GOP 断连/重连 | 需要（已有 gop_belongs_to_target） | 同代码复用 |
| 用户确认 | 屏显版本+按键（armed 倒计时） | 无按键窗口；arm 文件=意图声明（§5.2） |
| marker/资产位置 | U 盘根 | U 盘根或 ESP 根（同卷原则，§5.1） |
| 日志 | U 盘 `\loader.log` | arm 所在卷 `\loader.log`（ESP 可写） |
| boot 成本 | 仅显式进入时 +17~22s | armed 时每次引导 +17~22s |
| Secure Boot | 需关闭或签名（rule 9） | **不经过 SB 验签**（SB 只验 OS 引导链）——SB-ON 机器理论可用【待 V3 实证】 |

## 5. 驱动形态行为设计

### 5.1 卷发现改造（结构性改动，driver 行为正确性的前提）

现 marker/资产读 app 自卷（§1）。driver 改为：**扫描全部 SFS 卷**，命中"arm marker 与
（tx 构建的）家族资产集同卷"的卷即采用（first match，逐卷日志）。chainload 已有 per-volume
枚举与 `open_volume_logged` 基建可复用。推论：

- 资产集放 U 盘根 = 现状文件不动；放 ESP 根 = 免 U 盘运行形态（用户手动放置，我们不写）。
- 同卷原则防误配：ESP 里有 arm 没资产 → 只读模式照跑；有资产没 arm → 静默。

### 5.2 门控（宪法投影）

- 独立 arm marker `\tx-dxe-arm.txt`（命名待定，开放项）：与 U 盘 armed 语义解耦——U 盘上的
  `\i_know_what_i_am_doing.txt` 不应意外武装常驻驱动。既有旋钮（`tx-select-bdf` /
  `tx-quiet` / `tx-flush-lines` / `tx-fwsec-*`）语义照用。
- 无按键窗口：arm 文件即操作者意图声明（main.rs CONFIRM 语义的既有原则），
  driver 形态无倒计时、默认 quiet。
- tx 构建照旧走 `--features tx` 显式门（1.7 写分级）。

### 5.3 失败安全（driver 形态的新增义务：挂死=无法进系统）

1. 所有等待保持有界（既有 retry/timeout 模式不变）。
2. **失败计数 NVRAM 变量**：回调入口自增、成功清零；≥2 → 本引导跳过事务并日志。
   防两个循环：panic 挂死 → 断电重启 → arm 仍在 → 再挂（计数断链）；事务"成功返回但系统
   起不来"→ 下次引导自动放弃。用户干预 = 删 arm 文件或 Windows 侧清变量。
3. watchdog 维持 app 同款 disable（事务 ~20s « 5min；保留 armed 的自动复位会把
   BitLocker 恢复键输入窗一并算进 5min，风险更大）；panic 挂死的恢复路径 =
   断电重启（宪法 rule 2 同族，失败计数兜底 boot loop）。
4. 日志落 arm 所在卷；只读构建零写。

## 6. 封装器与发布形态

- 新纯 Rust 小工具（沿 `assets/` 惯例，可离线复跑）：输入 driver `.EFI` → 输出
  `NvUnlockDxe.ffs`（§3.2 规格，含 subsystem 头补丁）+ SHA256。
- **FFS 内零 NVIDIA 资产**：资产仍由 maker 本地再生放卷上（release 不发 NVIDIA bin 口径不变；
  含资产 FFS 由用户自制、永不分发）。
- 用户流程（ReBarUEFI 同款模板，`reference/ReBarUEFI` README 为准）：
  1. UEFITool NE 打开 BIOS 镜像 → 选最大空闲 FV → Insert FFS file → Save image file；
  2. **先备份完整原始镜像 + 手边有外部编程器**（CH341A/烧录座），再刷写（板上编程器或 FPT）；
  3. BIOS 更新会冲掉插入件，更新后需重插。
- 真机冒烟捷径：**UEFI Shell `load fs0:\NvUnlockDxe.efi`** 可在**不刷写**的前提下验证
  driver 语义（shell load 自动走 ConnectController 序列，uefi-rs driver.rs 文档确认此路径）。

## 7. 风险清单

| 级别 | 风险 | 缓解 |
| --- | --- | --- |
| 刷写 | 变砖：Boot Guard 机器改 IBB 内区域会拒启（是否在 IBB 内因板而异，不可一概而论）；刷写中断电 | 强制备份+外部编程器；文档显著位置声明"仅 Boot-Guard-off/已核实平台"；ReBarUEFI 社区经验可引用 |
| 刷写 | BitLocker 恢复提示（固件测量 PCR 变化） | 刷前 suspend BitLocker（文档强制步骤） |
| 运行 | 挂死/panic → 当次引导失败 | 有界等待+失败计数变量；断电重启恢复；arm 默认关 |
| 运行 | armed 时每次引导 +17~22s | 文档明示；后续可做成功态跳读（PLM 已开 → -EALREADY 秒退，已有守卫语义） |
| 发布 | 被误当"安装器"传播 | README/宪法增补条款：我们只发工具与 FFS 本体，插入动作永远由用户执行（issue#73 裁决） |
| 宪法 | rule 4"无常驻引导项"冲突 | 化解=用户显式刷入 + arm 默认关 + 无安装器 + USB app 仍是默认路径；本条写入 README 安全宪法副本 |

## 8. 分级验证计划（每级判据照宪法：读回=唯一成功判据）

- **V0 QEMU/OVMF**：UEFITool NE 改 OVMF.fd 插 FFS → debugcon 看 dispatch 顺序
  （"Loading driver"）、ReadyToBoot 相对 ConnectAll 的时序日志、负控（无 NVIDIA → 静默退出
  进 shell）。同时实证 §2.1/§3.1 的【待 V0 实证】条目。
- **V1 真机 shell load（tx-off 构建）**：U 盘放 .EFI → shell load → 枚举/卷发现/marker 语义
  全验证，零刷写零写。
- **V2 真机 shell load（tx 构建）**：门控=与 M3 同级用户显式确认；读回记录照 evidence-as-commits。
- **V3 固件插入（仅高级用户自担）**：备份→插入→刷写→armed 引导→读回；
  Secure Boot-ON 平台行为一并观察。

## 9. 工程改动清单（issue 化）

- **C1** `loader` 拆 lib + `dxe_main.rs` 双 bin（§2.2）；LTO/wcslen 地雷回归。
- **C2** FFS 打包器（纯 Rust，§3.2）+ check 工具扩展（subsystem/FFS 头/sections 校验）。
- **C3** 卷发现改造：SFS 全卷扫描 + 同卷原则（§5.1）；app 路径行为不变。
- **C4** 失败安全：失败计数变量 + 防重入（§5.3）。
- **C5** 文档：README 安全宪法增补 FFS 条款；FAQ 增"固件插入"节（引用 ReBarUEFI wiki）。
- **C6**（独立实验，默认不做）静默窗变体：PciEnumerationComplete 协议 notify + 内嵌资产，
  需单独立项裁决资产入 FFS 的口径。

## 10. 开放问题

1. DxeCore 对 FFS-DRIVER+PE-0x0B 组合的 residency/内存类型语义——源码推理已给，V0 实证。
2. AMI/Insyde 平台 `gEfiPciEnumerationCompleteProtocolGuid` 安装时机（ConnectAll 内的位置），
   决定静默窗变体（C6）的真实冷度。
3. driver bin 实际体积与 LZMA 压缩率（决定 FV 空间紧张平台的文案口径）。
4. UEFITool NE 对 OVMF FVMAIN_COMPACT 重压缩的兼容性（V0 用）。
5. 驱动形态的 arm marker 终名与 `\i_know_what_i_am_doing.txt` 的关系（独立 vs 桥接）。
6. 多次 boot option 重试时 ReadyToBoot 的确切重入次数（V0 日志可答）。

---

## 11. 审计修订（2026-11-01，PR #100 三轮对抗审计后）

合作者 Catnip202X 以零上下文对抗审计 AI 对本 PR 跑了三轮（21 项：F1-F5 / N1-N10 / M1-M6），
逐条对码确证后修复。设计相对本文前几节的**变更**以此节为准：

### 11.1 权威重设计（F1/M1，blocker）

"卷上文件名=授权"被推翻。现在的授权链：

1. 操作者**主动**引导装有 `\tx-dxe-arm-install.txt` 的 U 盘（app 形态的既有确认模型）；
2. app 计算本卷 **GPT 分区 GUID**（固件自身设备路径），写入 NVRAM 变量
   `NvUnlockDxeArm`（项目 GUID 2E7F7609-…，payload=`NVUX1`+GUID，NV|BS|RT）；
3. driver 每次引导读该变量 → 只接受 GUID 匹配的那个卷，且其上必须存在**普通文件**
   `\tx-dxe-arm.txt`（N5：目录不算）；`\tx-dxe-arm-remove.txt` 卸装。

文件投放无法伪造 NVRAM；卷身份=分区 GUID（整盘 FAT/CD 无 GUID，不可武装）；
卸装=删变量（shell `dmpstore -g 2E7F7609-…` / remove 标记）。倒计时地板回到 **1..=30**
（N1：中止窗不可被内容删除）；无头模式走独立标记 `\tx-dxe-noconfirm.txt`；ConIn 缺席且无
该标记 → fail-closed 中止。

### 11.2 失败安全（F2/N3/M3）

- 失败计数迁入 NVRAM 变量 `NvUnlockDxeFail`：读失败=跳过本引导；写失败=跳过本引导
  （双 fail-closed）；跨卷漂移/拔盘不再可绕过。
- 计数自增发生在一切可能 panic 的工作之前（N3 的窗口收敛）。panic 本身不"干净返回"：
  uefi 0.41 的 panic handler 走 `ResetSystem(SHUTDOWN)`——按 reset 后计数推进，
  两次后跳过。**按 bin 自定义 panic handler（COLD reset）需要 crate 拆分，列为后续项**。
- driver 不再碰平台 watchdog（M3）：BDS 的 5 分钟计时保持 armed，挂死自恢复。

### 11.3 卷解析与完整性（N2/N4/N7/M4/M1c）

- `volumes.rs` 不再持有裸 handle：钉住**分区 GUID**，每次 `marker_fs()` 调用对活动句柄
  数据库重解析——拔盘/句柄池复用一律 `NOT_FOUND`（fail-closed）。
- 事务前 live 重探资产文件在位（N7：缓存使拔盘失效的旧保证被恢复）；缓存改 Arc 单副本
  + 256 MiB 总量帽 + 事务后/中止即清（M4）。
- **驱动形态资产完整性**（M1c）：maker 产出 `\dxe-manifest.txt`（sha256sum 格式），
  驱动形态事务前逐文件 SHA-256 校验，不符/缺失=拒绝（fail-closed）。app 形态维持
  stick-trust 语义不变。
- `OVERRIDES:` 启动行枚举本次生效的全部 guard 覆盖标记（N4），进 FLUSH/quiet 白名单。

### 11.4 封装器（F4/F5/N9/N10/M5）与自证（M2）

- ffspack 拒绝无 `DXE_BUILD_ID` 标记的输入（app 镜像无法再被打包为驱动）；FFS 启用
  `FFS_ATTRIB_CHECKSUM` 数据校验和；严格校验=类型/极性/depex 字节/单 PE32/子系统/身份标记；
  30 字节文件不再 panic；`--state` 显式极性；输出临时文件+改名、拒绝覆盖（--force）、
  双产物哈希、`--expect-sha256`。
- driver 运行时打印 `banner` + `FORM:`（构建身份/tx 类）+ `ARM:`（变量→卷 GUID）行；
  上一轮日志轮换为 `\loader-prev.log`（F3）；`check_efi.py` 按 PE 子系统 0x0A/0x0B 分支
  判类（M2）。

### 11.5 政策声明（M1/M6，明确写入而非默许）

- **§1.3（ESP 不写）**：本形态要求的 marker/资产/日志放在操作者**自选卷**（U 盘或 ESP），
  由操作者亲手放置；本项目仍不提供任何写 ESP/固件的安装器。驱动对 armed 卷的
  日志/轮换写入是该卷被武装的自然后果，已在文档与日志行中明示。
- **§1.4（无常驻引导项）**：FFS 形态是操作者亲手刷入固件的常驻代码，§1.4 的字面条款
  （不动 NVRAM BootOrder）仍然满足，但其"无常驻存在"的意图被本形态**有意放弃**——
  这是该形态存在的目的（每次引导重放），非副作用。**BCFG driver-load 形态已作为
  并行交付落地**（docs/bcfg-driver-load.md + docs/evidence/dxe-bcfg-load/，QEMU
  四轮验证）：零固件侵入、拔盘/删文件即失效、无变砖类别；代价=SB-ON 需 db 自签
  （外部镜像走 LoadImage 验签）、DriverOrder 进 PCR1（BitLocker suspend）、
  SB-OFF 时驱动文件本身是可替换面（其 §7 安全准则强制三选一）。
- **§1.9（Secure Boot）**：固件内置驱动与从文件加载的驱动经过的验证路径不同，取决于
  平台的固件验证、Secure Boot 和 TPM 测量策略。ReadyToBoot 早于 ExitBootServices
  不能证明它处于 SRTM 测量之外，也不能据此宣称对 Secure Boot 不可见或兼容。
  这些性质必须在目标平台单独验证；本轮没有验证 SB-ON 或 TPM/PCR 行为。
- **资产完整性**：app 形态维持 stick-trust；驱动形态强制 manifest（11.3）。二者口径
  不同是**有意的**：无人在场的形态不允许信任一个可被任何进程投放文件的卷。

### 11.6 未被本轮修复的审计项（诚实清单）

- N1-4"早于 deadline proceed"：对码证伪（`armed_confirm_with_prep` 要求
  `now>=deadline && prep.done()`，取二者较大）——维持 #97 真机已测语义，未改。
- 逐 bin panic handler（COLD）：需 crate 拆分，后续项。
- N4 的深度方案（override 标记要求回显 BDF）与轮换深度（多代日志）：留待实机反馈。
- FV dispatch + ReadyToBoot 路径仍未执行（V3 首块真机）；模拟器标记数组漂移的
  CI 强制（N8 后半）为后续项，本轮已把 dxe 解析纳入 sim 镜像与测试。

## 12. 安全复审修订（2026-11-01，以本节及代码为准）

本节取代 §2.3、§5、§11 中的旧触发、授权、失败计数和 watchdog 语义。

- **触发与 TPL**：BDS_ARCH 在 DXE 调度期即可存在，不能作为 BDS 已启动的证据。
  FV 或未知来源的镜像只注册一次 ReadyToBoot 回调；具有 MEDIA_FILEPATH 的文件加载
  镜像在 entry 的 TPL_APPLICATION 执行（Shell `load`、Driver#### 等）。事件回调处于
  TPL_CALLBACK，禁止调用 APPLICATION-only 的 ReadKeyStroke，也不通过降低 TPL 绕过限制。
  因此 **FV 自动路径必须明确授权 `\tx-dxe-noconfirm.txt` 无人值守模式**，否则跳过；
  文件加载路径保留 1–30 秒任意键中止窗口，两种构建均在按键或输入错误时中止。
- **授权**：旧 NVUX1 变量不再有效。NVUX2 保存分区身份和 SHA-256：摘要覆盖
  `dxe-manifest.txt` 及 `dxe_policy::CONFIG_PATHS` 中所有控制文件的路径、存在状态和完整
  内容。授权只接受普通文件，控制文件上限 4 KiB、manifest 上限 64 KiB，超限或读取错误
  均拒绝，不接受截断前缀。驱动运行时重新校验，再固定内存快照；改变载荷与 manifest、
  更换目标 BDF、添加危险覆盖项或取消确认标记均需重新授权。未知控制文件默认不生效。
- **授权操作**：先准备好资产、manifest、`tx-dxe-arm.txt` 和所有控制文件，再放置
  `tx-dxe-arm-install.txt` 并主动启动 app。授权使用独立的 Space/Enter 确认，其他键、
  无输入或 30 秒超时均取消；不会被 app 的自动执行标记代替。授权后移除 install 标记。
  `tx-dxe-arm-remove.txt` 删除 arm 变量；只读 app 构建仍可执行这些明确要求的 NVRAM 操作。
- **边界**：摘要保护已授权卷上的文件投放/替换，不防拥有固件变量写权限的管理员或
  已执行的恶意 UEFI 代码。分区 GUID/MBR 身份本身不是密码学身份。MBR 身份现在包含
  完整 64 位起始 LBA 和类型标记。
- **重试保护**：读取 latch 失败、写入失败或读回不符均拒绝执行，镜像内也有一次性守卫。
  `NvUnlockDxeFail` 现在是严格解析的**尝试预算**，最多两次；在文件 I/O、时间校准和
  控制台输出之前递增并读回。无论事务正常返回、报告失败、中止或后续 OS 黑屏，都不会
  自动清零。达到两次后，在操作者确认系统/GPU 已恢复正常、再次通过 app 明确授权前
  保持跳过。卷缺失或配置校验失败也可能消耗尝试；这是保守的恢复策略，不是 OS 健康检测。
  重新授权不会清除本次引导的 latch。
- **watchdog**：标准 EDK2 在 ReadyToBoot 之后、StartImage 之前才启动 OS-loader watchdog。
  驱动必须自行成功设置 300 秒 watchdog 才继续；不支持或设置失败即跳过。正常返回保留
  并刷新 300 秒，后续由 boot manager/OS 接管。没有通用 API 查询并恢复此前超时值。
  该机制不替代硬件断电恢复，也不宣称覆盖固件变量服务自身的故障。
- **控制台**：依据系统表的实际 stdin/stdout 判断可用性；无 stdout 时仍可写文件日志，
  不再因为 logger 的 with_stdout 断言而 panic。
- **资产**：scrubber 和其他资产一起在首次 GPU 写操作之前读取、验证，之后使用同一份
  已验证的内存字节；不再于事务中途从卷上重新读取。manifest 同时约束 direct/family 两条载荷候选的内容与缺席，
  防止删除预期 V67 后静默降级；有意的无 V67 replay 仍可使用（仍是写事务）。
- **FFS**：PI 头布局为 Header checksum @16、File checksum @17；打包器及校验器已修正。
  新增独立 wire-format 测试使用规范固定偏移，而不是复用实现常量。

验证记录见 [DXE 安全修复验证](evidence/dxe-safety-review/README.md)。FV/ReadyToBoot 与
真实 GPU 的验收范围必须分别说明；通过单元测试或无 NVIDIA 的 QEMU 不代表真机事务已验证。

### 11.7 日志持久化补充（2026-11-01，用户裁定 ①+③）

驱动形态的日志本来就落 armed 卷（`\loader.log` + 轮换），但 4f688e7 的门序把预算/指纹/
marker/noconfirm 四类拒绝放在 `logging::init` 之前——**十二个静默出口零痕迹**。补两件（不动
"预算自增先于磁盘 I/O"的 N3 顺序）：

1. **NVRAM 退出原因面包屑** `NvUnlockDxeExit`（项目 GUID，NV）：每个 pre-logging 出口写
   `latch-write / counter-unreadable / counter-invalid / budget / counter-write / watchdog /
   policy-mismatch / marker-unreadable / no-window`，进入执行写 `ran`、中止写 `aborted`；值不变
   不重写（每次状态迁移一笔 NV 写）。装机器入口向操作者打印 `last driver exit reason`——
   QEMU 实证：预算耗尽后装机引导报出 `'budget'`、武装执行后报出 `'ran'`。
2. **三代日志轮换**：`\loader.log` → `\loader-prev.log` → `\loader-prev2.log` →
   `\loader-prev3.log`（每代 1MiB 封顶，最坏 ~3MiB/引导的卷 I/O）；QEMU 实证 Driver 时点的
   运行日志在 app 轮换后仍存活于 `\loader-prev.log`。

未采纳：logging 提前覆盖静默窗（②）——与"预算自增先于一切磁盘 I/O"的 N3 窗口设计冲突，
留给 Ajax 的门序设计意图优先；面包屑已覆盖同一证据需求。
