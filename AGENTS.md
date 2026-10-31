# UEFI 预驱动 GPU 解锁器——开发代理任务书（v1，2026-10-25）

> Deployment agents: use [AGENTS.deploy.md](AGENTS.deploy.md) for the installation workflow and current documentation index. This file remains the development task brief.

> 用法：把本文整篇作为新项目开发代理的首条指令（或放入其 AGENTS.md）。任务书自包含；所有外部仓库给公开 URL，需自行 clone 到本地 `reference/` 目录。本任务书源于 NVOC 项目的生态考察与规划（docs/reverse-engineering/uefi/windows-uefi-pre-driver-unlocker-dev-plan.md），技术事实均已实测或三源交叉核实。

---

## 0. 你的角色与使命

你是一个 GPU 固件安全研究项目的全栈开发代理。目标：在 Windows 机器上开发一个 **UEFI 应用（BOOTX64.EFI）**，在 NVIDIA 驱动加载**之前**执行预驱动事务——从只读冷态侦察到（门控后的）固件事务复刻——完成后链式交回 Windows Boot Manager。用户拥有全部目标硬件（自有研究，授权明确）。

三个芯片族的分工：
- **TU106（RTX 2070 台式机，主力实验机）**：事务脚手架与只读侦察的验证台。**Turing CMP 存在**（30HX=TU116-100、40HX=TU106 硅马甲、50HX=TU102），consumer 侧出厂钳制状态未知（开放问题 #10，2026-10-27 修正——旧稿"20 系无已知 Segmentation"前提不成立）；**FEAT 簇存在性已回答：存在但 Turing 在 PL0 全锁**（#1），且 Turing BAR1 经 PSTRAPS 可扩（#11，NvStrapsReBar 实证）。
- **GA10x（RTX 3060 GA106 在库）**：社区 V67 载荷与引导链的成熟复刻目标（Linux kmod 已验证 +80% BF16）。
- **AD10x（参考实现：4090/4080 桌面卡；用户现有 AD107 笔记本**不**作实验机）**：四写 PLM 链 + 可选 64G BAR1 的参考研究，有硬件才动手。

## 1. 安全宪法（不可协商，任何里程碑不得绕过）

1. **读回是唯一成功判据**。状态码/返回值/中断/自检输出一律不构成执行证明；每次写后必须读回比对。TOURORONG 原话："FINAL status=0 单独不构成提速证据"。
2. **STOP/异常后同会话绝不重试**。固件可能仍在运行、DMA 缓冲可能仍被占用——完全断电后走正常 Windows 引导恢复，再来。
3. **只读链式加载** ESP 的 `\EFI\Microsoft\Boot\bootmgfw.efi`（LoadImage/StartImage），**永不覆写 ESP 任何文件**；替换 U 盘 BOOTX64.EFI 前先备份原有。
4. **无常驻引导项**：只从 U 盘/启动菜单一次性进入；不动 NVRAM BootOrder 常驻配置。
5. **每张卡钉死身份**：PCI vendor:device + ROM/PROM hash 校验通过才继续；换卡/换 ROM 即拒绝（"改 ID 常量不构成已验证移植"）。
6. **资产纪律**：NVIDIA 固件不分发、不入 git；从钉 SHA-256 的官方输入本地再生；密钥、解密产物、生成的载荷永不入库。钉住的下载失败时不得换用其他版本。
7. **写操作分级门控**：M1 之前零写；M3 事务含固件点火，进入前需用户显式确认；M4/M5（载荷劫持）单独立项再决策。
8. **先 mock 后真机**：任何事务代码必须先在行为级模拟器上全绿，才允许接触真机。
9. Secure Boot：实验机关闭，或自签证书入 db（优先后者）。
10. 许可：参考实现均为 GPL-2.0；本项目**直接声明 GPL-2.0-only**，规避衍生争议；行为借鉴+独立实现仍照此声明。

## 2. 背景：为什么是 UEFI（30 秒版）

PLM（Privilege Level Mask）是 NVIDIA PRI 总线的**硬件门**：每个寄存器块配一个 PLM，默认 `0xffffff8f` = 全层级可读、仅 L3（heavy-secure，认证固件内部）可写。**主机/驱动/kmd 写 PLM 保护的寄存器被静默丢弃**——三个独立来源交叉证实（4090 fork 主机写 CYA/CFG 无效、TOTORONG 必须经 SEC2、NVOC 3060 残局"写了不动钳"）。越过的唯一路径是让**已签名的固件自己在 heavy-secure 模式里写**（ROP 复用原厂指令，无未签名代码执行）。而 Windows 没有用户可及的"驱动加载前"钩子——唯一等价时窗是 **UEFI DXE 阶段**（Linux 侧等价物是 initramfs 里的 kmod，即参考仓库 gpu-unlock 的做法）。

解锁在硅片引导协议层操作（booter 微码跨驱动版本逐位恒定），因此与驱动版本解耦；效果是易失覆盖（FLR 后存活、断电即失）→ **每次引导重放**，不是一次刷写。

## 3. 硬件目标档案与寄存器速查

### 3.1 本机硬件（写代码前重新核实）

| 机器 | GPU | 角色 |
|---|---|---|
| 台式机 | RTX 2070（TU106, 10de:1f07, VBIOS 90.06.18.00.EE） | 主力实验机（M0–M3） |
| 工作机 | RTX 3060 12GB（GA106, 10de:2504） | M4 候选（门控） |
| 笔记本 G614JV | RTX 4060 Laptop（AD107） | **不作实验机**（移动端供电/显示链复杂，社区未验证） |
| 服务器 | P100（GP100） | 不适用（无 GSP，pre-Turing） |

### 3.2 核心寄存器（BAR0 偏移；跨族通用除注明）

| 寄存器 | 偏移 | 语义 |
|---|---|---|
| PMC_BOOT_0 | 芯片 ID 判定 | 冷启动身份验证 |
| FEAT_OVR_PLM | 0x00823804 | 默认 0xffffff8f；开到 0xffffffff 后主机可写 SS0/SS1。**常开岛（AON）成员——FLR 打不回去**（这就是算力解锁活过 FLR 而显存几何不活的原因） |
| FEAT_OVR_SM_SPD (SS0) | 0x0082381c | 八个 4 位字段（bit3=覆盖使能，[2:0]=速率：0=全速 1=半 2=1/4 3=1/8）；目标值 0x88888888 |
| FEAT_OVR_SM_SPD_1 (SS1) | 0x00823820 | 单字段；目标值 0x8 |
| FEAT_READOUT_1 | 0x00823818 | 生效值读出（九个 3 位字段，掩码 0x07ffffff），全 0=全速 |
| WPR2 lo/hi | 0x001fa824/0x001fa828 | 上电到 FWSEC-FRTS 前恒 0；非 0=本周期已被初始化（冷启动判据+守卫） |
| MMU PLM / lock | 0x001fa7c8 / 0x001fa82c/30 | 布局计算用 |
| GFW: PLM / scratch | 0x00118128 / 0x00118234 | GFW 引导状态等待 |
| GSP Falcon CPUCTL | 0x00110100（基址 0x00110000） | FWSEC 点火对象（TU106 上 GSP 为 RISC-V，Falcon 界面是否同构=开放题） |
| SEC2 | 基址 0x00840000 | Booter 执行体；邮箱 +0x40/0x44；FALCON2/RISC-V 切换 BCR +0x668 |
| 熔丝: 版本 / 总开关 | 0x00824140（解码 0x008241c0）/ OPT_FEATURE_FUSES_OVERRIDE_DISABLE 0x008203f0 | 签名槽选择按熔丝版本位图计数（低于该版本的位数=槽序号）；总开关=0 是整个解锁域的存在前提 |
| FB 大小 / VGA 工作区 | 0x001183a4（MiB）/ 0x00625f04 | WPR2 布局计算 |
| NV_PROM（VBIOS 镜像） | BAR0+0x300000 | 自解析 BIT 表取 FWSEC |
| AD10x 专属: CYA / CFG / XVE PLM / BAR1 cap 镜像 | 0x88724 / 0x88dcc / 0x88ff4 / 0x88bbc | 64G BAR1 用；**主机写被硬件无视，必须 SEC2 内写**（CYA=0x30, CFG=0x8000000a） |

### 3.3 badf 毒值分类（读回来的垃圾要分桶，勿混）

`0xBADF5040`=特权违规（PLM 挡了）；`0xBADF1100`=未解码；`0xBADF20xx`=floorswept（物理不存在）。全 BADF 一律报"不可读"会丢掉这三类信息。

## 4. 参考仓库地图（开工第一步：clone 并按序读）

```
reference/
├── gpu-unlock        https://github.com/Zagitta/gpu-unlock          # Linux kmod,GA10x 事务权威范本
├── gpu-unlock-ada    https://github.com/JustAResearcher/gpu-unlock-ada  # Ada 四写链+验证纪律范本
├── gpu-unlock-bar1   https://github.com/84634E1A607A/gpu-unlock-4090-48g-64g-bar1  # AD102 BAR1 fork
├── cmp170hx          https://github.com/Consensus-Protocol/cmp170hx # 机制百科+方法论
├── envytools         https://github.com/envytools/envytools         # envydis(Falcon ISA)+nvbios
├── open-gpu-kernel-modules  https://github.com/NVIDIA/open-gpu-kernel-modules  # 一手源码:kgsp*/kernel_gsp_fwsec.c
├── cmp90hx-unlock    https://github.com/WildFlash1st/cmp90hx-unlock # GA102 V67 链源头(补丁形式)
└── faucon            https://github.com/vbe0201/faucon              # fuc5 反汇编(备选;主力 envydis)
```

**必读顺序**（预计 2–3 天，别跳）：
1. `gpu-unlock/docs/How Ampere was unlocked.md`（193 行，读三遍——引导链+ROP+PLM 的最佳入门）；
2. `cmp170hx/docs/unlock/{overview,how-it-works,falcon-and-booter,privilege-level-masks,rop-chain}.md`；
3. `gpu-unlock/kernel/gpu_unlock/unlock_610.c` 逐段对照 1/2 的概念（注释即教材）；
4. `gpu-unlock-ada/docs/ada-port.md` + `rtx4090-register-report.md`（Ada 差异+验证纪律）；`tools/{trace_booter_stack,compare_booter_gadgets,inspect_v67_prefix}.py`（审计方法论）；
5. `gpu-unlock/tests/test_unlock_adapter.c`（mock 模式——你要在 Rust 里复刻它）;
6. `open-gpu-kernel-modules`: `kgspBootGspRm` / `kgspPopulateWprMeta_TU102` / `kernel_gsp_fwsec.c`（一手权威，与 3 的注释互为字典）;
7. TOTORONG（HuggingFace，不 clone 也要读模型卡+`docs/INSTALL.en.md`/`ANALYSIS.en.md`）: https://huggingface.co/TOTORONG/AD103_win_unlocker —— EFI 载具工艺的唯一 Windows 先例。
8. `gpu-unlock-bar1/docs/bar1-64g-report.md`（若做 40 系 BAR1）。

## 5. 技术栈与工程规范（已定稿，勿另起炉灶）

- **Rust + uefi-rs**，目标 `x86_64-unknown-uefi`，cargo+rust-lld 原生 Windows 构建。API 已验证覆盖：`boot::locate_handle_buffer(SearchType::ByProtocol(&PciIo::GUID))` + `boot::open_protocol_exclusive::<PciIo>`（枚举与 BAR 访问）、`boot::disconnect_controller/connect_controller`（GOP 断连/重连）、`LoadImage/StartImage`（链式启动 bootmgfw）。若包装层吞错误码细节，降级到 raw system table 调用。
- 仓库结构：`loader/`（EFI 应用 crate，no_std）、`assets/`（host 侧工具：下载钉 hash 输入+ELF/BINDATA 抽取+载荷生成，纯 Rust）、`sim/`（行为级模拟器+事务状态机测试）、`docs/`（evidence-as-commits 归档）。
- 资产管线沿 gpu-unlock 惯例：钉 NVIDIA 610.43.03 源包/nvidia-utils 的 SHA-256（Turing 族资产从对应钉版驱动抽），本地再生、逐级校验、产物 hash 不符即硬失败。
- 每个里程碑的实验产物（采样文件/模拟日志/读回记录）作为 commit 归档，`.gitattributes` 把证据文件标 `-text` 保证跨平台逐字节一致。
- 运行时输出串一律英文（kmd 域既有惯例）；注释/文档中文可。

## 6. 里程碑（每步有验收判据/风险/回滚；进入 M3 前必须请示用户）

### M0 骨架（0.5–1 天）
QEMU+OVMF 冒烟（无 NVIDIA 设备→优雅退出=天然负控）；实机 TU106 U 盘路径：枚举 10de→打印 PCIe caps/ReBAR 能力→链回 bootmgfw。
**验收**：实机 U 盘启动到 app、屏显枚举结果、无缝进 Windows；代码审查零 PciIo 写调用。**回滚**：黑屏→强制重启走原引导项（未动任何固件配置）。

### M1 只读冷态采样（1–2 天）
BAR0 映射→读 §3.2 全表（FEAT 31 字簇 0x823804–0x823820 重点：**TU106 存在性判定**）→badf 分桶→屏显+写 U 盘 FAT 文件（SimpleFileSystem）。
**验收**：采样文件落盘、三类 badf 正确分类、TU106 FEAT 簇有明确答案（存在/不存在都是一手产出，如实记录）。

### M2 资产管线 + mock 事务（2–4 天，无 GPU）
Rust 重写 `extract_boot_assets`（gsp_ga10x.bin ELF 段抽取：`.fwimage`/`.fwsignature_ga10x`/`.fwsignature_ad10x`；BINDATA C 数组=raw-DEFLATE+gzip CRC32/ISIZE 尾校验）；VBIOS FWSEC 描述符解码（V1/V2/V3 三版布局见 §7.3）；`sim/` 里移植 gpu-unlock 的行为级假 MMIO 模式（SS1 按 4 位掩码、WPR2 仅在模拟 FWSEC 后出现、复位传播 N 次读、DMA 地址簿记、FLR 时序）+ 62 场景级故障注入矩阵（marker 不符/HALT 不置位/WPR2 预启/几何不符/签名未复原……）。
**验收**：资产逐级 hash 绿；模拟下全事务状态机绿。

### M3 TU106 事务复刻（1–2 周，**门控**：M0–M2 全绿+用户显式确认）
完整走一遍"无载荷"链：claim（D0/FLR 能力/清 BME）→布局计算→DMA 建页表→GFW 等待→**FWSEC V2 BOOT_WITH_LOADER 点火（TU106 实测：FWSEC_PROD IMEM 0x9600+DMEM 0x410）**→WPR2 建立→Booter **原签**加载→邮箱/PLM 翻转验证→FLR→交棒原厂驱动。
**验收**：WPR2 建立→交棒前清零的完整读回记录；Windows 驱动正常接管。**风险**：中途失败=黑屏至断电。**TU106 专属注意**：booter/gadget 几何逐芯片不同（GA102 载荷在 AD102 上 HALT 的教训），Turing 资产必须从钉版 Turing 驱动自抽；TU106 GSP 为 RISC-V，Falcon CPUCTL 界面是否同构要在 M1 采样中一并核实。

### M4 GA106 载荷（单独立项，默认不在本任务书 scope）
社区 GA10x V67 四写链（见 §7.4）在 3060 上点火。前置：M0–M3 全绿+独立立项+变砖承受评估。

### M5 AD10x（仅当获得桌面 40 系卡）
四写 PLM 链载荷（TOTORONG/gpu-unlock-ada 谱系）+可选 BAR1（CYA/CFG 经 SEC2，主机写无效）。

## 7. 关键事实库（压缩版，展开读 §4 的仓库）

### 7.1 引导时序（驱动加载时真正发生的事）
上电 devinit（PMU，VBIOS 脚本）→ GFW_BOOT 就绪 → 驱动从 VBIOS PROM 取 FWSEC 以 heavy-secure 点火在 GSP（建立 WPR2）→ 驱动送签名 Booter 到 SEC2（邮箱传 WPR 元数据 DMA 地址）→ SEC2 验签加载 GSP-RM → RM 与主机建 RPC。**解锁攻击发生在"驱动送 Booter"一步**：用扩大的签名 DMA 覆盖 SEC2 DMEM 栈，让签名映像自己的指令（ROP gadget）替我们写 PLM。

### 7.2 Falcon 界面速查（各引擎 Falcon 通用）
CPUCTL +0x100（bit0 启动、bit4 HALT、bit6 别名启动路径）、邮箱 +0x40/+0x44、boot base +0x104、IMEM/DMEM DMA +0x110–0x128（256 字节块，cmd `6<<8`）、FBIF transcfg（bit7 物理模式）、引擎复位 +0x3c0（10 周期传播）、HWCFG2 +0xf4（bit12 scrub 等待、bit31 预复位等待——HW bug 3419321）、RISC-V 切换 BCR +0x668。哈佛架构：IMEM 不可写→ROP 只能重排不能注入。envydis 目标 `fuc5` 解码 SEC2 booter（GA102 11,153 条/AD102 10,384 条实测连贯）；**IDA 没有 Falcon 处理器模块，别浪费时间为它配置 IDA**（x64/RV64 才是 IDA 领地）。

### 7.3 VBIOS → FWSEC 链路（已在 NVOC 侧实测，TU106）
BIT 头 `FF B8 "BIT\0"`+校验和（size@+8/stride@+9/tokens@+10）→ token 0x70 v2 → ucode 表（ver=1/hdr=6/stride=6/条目 6B{app,target,desc_off}）→ 描述符。**指针需加 adjust=ext−base**（TU106=0x11600）。app ID：0x01=PRE_OS、0x05=FIRMWARE_SEC_LIC、0x08=LS_UDE、**0x45=FWSEC_DBG / 0x85=FWSEC_PROD**（驱动按 debug 模式二选一；TU106 双轨几何逐位相同）。描述符三版（vDesc 头：bit0=有头、bit2=加密位、[15:8]=版本、[31:16]=长）：
- **V1**（40B，Pascal-）：Stored@4/Uncompressed@8/VirtualEntry@12/IMEMPhys@16/IMEMLoad@20/DMEMPhys@32/DMEMLoad@36；
- **V2**（60B，Turing）：InterfaceOffset 插在@16，IMEM 字段后移 4；**IMEMSecBase@32/IMEMSecSize@36**（TU106 实测自洽：IMEMLoad 0x9600 = Sec 0x300+0x9300；stored 0x9a10=IMEM+DMEM 0x410）；启动方式 BOOT_WITH_LOADER；
- **V3**（44B，GA10x+）：PKCDataOffset@8 替代 Uncompressed；EngineIdMask@36/UcodeId@38/SigCount@39/SigVersions@40；描述符后跟 384B/条 RSA3K 签名区，槽位=低于熔丝版本的版本位数。
加密态判据（TU106 实测）：FWSEC IMEM 熵 **7.99**=AES 密文（IDA 与 envydis 都瞎，密钥在熔丝）；PRE_OS 熵 6.63=明文 fuc5 可直接反汇编；GOP EFI 镜像压缩存放无 PE 签名。**任何 blob 先做熵和头判别再谈分析。**

### 7.4 V67 载荷结构（GA10x，M4 参考）
0xFA00 字节 fill 字 `0x0000019c`（自循环指令——PC 被劫后停在可观察循环）+ 补丁字：写值 0xffffffff@0xf948、目标 0x00823804@0xf960、终止符 0x81ee@0xf9ec（GA102；Ada 全套不同：gadget 0x21b2/0x33fc/0x256f/0x7a02、marker 0x67b00001@0xf9e0）。栈几何：canary 槽 0xff4c、返回槽 0xff50、拷贝基 DMEM 0x600、全局 canary 0x6140(GA102)/0x5F50(AD102)。canary 攻法=均匀性：同一常量 0xc0deca7e 覆盖全局+所有保存槽→每个尾声 V==V。漏洞=booter 签名 DMA 无界（长度逐字取 WprMeta+0x50，DMEM 0x0800+0xF800=0x10000 恰越顶）。**marker 进邮箱 0 + SEC2 HALT + PLM 翻转三证齐全才算执行成功；Booter 报错 0x31 不代表失败（写已发生）——读回是唯一判据。** Ada 四写链顺序：WPR_CFG 0x1fa7cc=0xfffff0ff → FBPA 0x9a0148（观察项，卡 0xffffff0f 记非致命）→ WPR 0x1fa7c4 → FEAT 0x823804=0xffffffff。

### 7.5 EFI 载具工艺（TOTORONG 实测拆解，Windows 专属四难题）
①时窗=EFI 应用即 pre-driver 窗（U 盘启动，无常驻项）；②**GOP 断连/重连**（DisconnectController 独占设备→事务→ConnectController 恢复；无 iGPU 机器期间黑屏/冻结末帧）；③`LoadImage/StartImage` 预载 bootmgfw.efi 链式交接（易失覆盖=每引导重放）；④DMA 用 PciIo AllocateBuffer（EFI 保留内存）。事务日志按 TOTORONG 阶段名：SELECTOR RESULT / POST-FLR / POST-GOP / FINAL。TOTORONG 工具链=Python3+Zig 0.14.1（仅参考；我们用 Rust）。

## 8. 环境自检（开工时重新核实，勿信快照）

已知在位（2026-10-25 快照）：VS2022 Community（MSVC 14.44）、Windows SDK 10.0.26100、Python 3.14.5、Rust 1.95 stable-msvc、WSL2(archwsl)。已知缺失：**QEMU for Windows**（M0 必装，自带 OVMF 固件）、Zig（仅参考方案）、NASM（仅 edk2，不需要）。待办：`rustup target add x86_64-unknown-uefi`。首次实机前查：目标机 Secure Boot 状态、BIOS 是否允许 U 盘首启。

## 9. 已知坑（前人踩过，别重蹈）

1. **逐芯片几何**：booter 尺寸/gadget 地址/签名段名（.fwsignature_ga10x vs _ad10x）每族不同；GA102 载荷在 AD102 上 HALT 且不开权限。跨族移植=重新抽取+重新审计，不是改常量。
2. **SEC2 复位语义**：`ksec2ResetHw_TU102` 止于 RESET_FALSE；留着 RESET_TRUE 会让后续主机读返回 FECS_PRI_RESET（0xbadf12 混进 bits31:8），RM 的 scrub 等待永不完成。
3. **BME 脏状态**：板载固件可能开着 BME 交棒（Supermicro X11 实锤）——事务前必须清；Linux 驱动 resume 不重设 BME（Windows 侧自行验证）。
4. **devres/资源契约**（EFI 侧无此物，但 sim 里要模拟）：事务 unwind 必须把 DMA/BAR/命令位全部复原并验证无人中途改动。
5. **签名复原**：用 V67 载荷占位签名槽后，FLR 前**必须**恢复原厂签名（"绝不让 V67 签名留给后续原厂引导"）。
6. **WPR2 守卫**：HI.VAL≠0=本周期已被初始化，直接拒绝（-EBUSY）；PLM 已开=已解锁或非冷（-EALREADY）。这两个错码是好东西，照抄语义。
7. **kmd 式陷阱在 EFI 的投影**：EFI 阶段没有驱动竞争，但也意味着没有 NVML/NVAPI 旁证——读回+QEMU 负控就是全部证据链。
8. gitlink/子模块：若用子模块，推送前确认子模块提交已推（丢提交=下游编译炸）。

## 10. 工作节奏与门控规则

- 每个 M 开工前：向用户复述该 M 的验收判据与风险，确认后进入。
- M3/M4/M5 进入前：**必须停下来等用户显式确认**（写操作升级点）。
- 真机实验日历：一次实验=一次完整记录（U 盘采样文件+屏显照片/抄录+事后 Windows 侧驱动状态检查），按 evidence-as-commits 归档。
- 任何"看起来成功"的结果，先问自己：读回了吗？负控了吗？换卡会怎样？
- 遇到本任务书未覆盖的事实冲突：以 open-gpu-kernel-modules 源码 > 参考仓库实现 > 文档 > 记忆 的顺序裁决，并把裁决写进 docs。

## 11. 交付物定义

1. `loader/` EFI 应用：版本屏+单键确认+30s 超时+阶段日志+Final 报告（沿 TOTORONG 交互范式）；
2. `assets/` 管线：钉 hash 下载/校验/抽取/生成，全程可离线复跑；
3. `sim/` 行为级模拟器+事务状态机+故障注入矩阵（M2 核心）；
4. `docs/`：里程碑证据、寄存器冷值表（按族分文件）、开放问题清单（TU106 FEAT 簇答案置顶）；
5. README：安全宪法副本+回滚手册（黑屏/STOP/驱动不认卡三种场景的恢复步骤）。

——任务书完。第一动作：环境自检（§8）→ clone 参考仓（§4）→ 必读清单（§4 序）→ 向用户复述 M0 验收判据 → 开工。
