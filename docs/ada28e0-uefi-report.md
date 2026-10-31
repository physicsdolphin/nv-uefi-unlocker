# RTX 4060 Laptop（10de:28e0）：Windows UEFI 预驱动四写解锁

2026-10-26，一台 ASUS ROG Strix G16 G614JV（RTX 4060 Laptop GPU，`10de:28e0`，
AD107M，20 SM，8 GiB，rev a1，BIOS 独显直连模式）。全流程一次冷启动完成。

**结果**：本仓库的 UEFI 应用（`BOOTX64.EFI`，Rust + uefi-rs，x86_64-unknown-uefi）
在 NVIDIA 驱动加载之前完成了完整的事务链——SEC2 scrubber（BSI handoff 1→3）、
FWSEC-FRTS 点火（WPR2 建立）、stock Booter 经正确签名的 HS 引导启动 GSP-RM
（MAILBOX0=0）——随后在同一事务内执行 **V67 四写 payload** 并全部读回验证：

| 写入目标 | 解锁前 | 写入值 | 解锁后读回 |
| --- | --- | --- | --- |
| `0x823804` FEAT PLM | `0xffffff8f` | `0xffffffff` | `0xffffffff` ✅ |
| `0x1fa7cc` WPR_CFG PLM | `0x0004cb8f` | `0xfffff0ff` | `0xfffff0ff` ✅ |
| `0x9a0148` FBPA PLM（观察项） | `0xffffff0f` | `0xffffffff` | `0xffffff0f`（与 2705/2805/28e1 一致，非致命） |
| `0x1fa7c4` WPR PLM | `0x0004cb8f` | `0xffffffff` | `0xffffffff` ✅ |

payload marker `0x67b00001` 出现在 SEC2 MAILBOX0（ROP 执行证明）；
宿主侧 selector 写入 `SS0=0x88888888` / `SS1=0x8` 成功并读回（PLM 开启后主机写
首次生效）；FLR 后 selector 存活（AON 岛）；stock 签名在 FLR 前还原（永不把
payload 留给后续原厂引导）；全流程干净交棒 Windows。

**Windows 侧实测**（cli-stressor-rs，FP16 GEMM 4096，resident L2）：

| 状态 | 等效吞吐 | 倍率 |
| --- | --- | --- |
| stock | 29.89 TFLOPS | 1.00× |
| 解锁后 | 49.02 – 49.33 TFLOPS | **1.65×** |

1.65× 低于 28e1 报告的 2.00× 属预期：该倍率出自纯 tensor 发射速率微基准，
真实 4096 GEMM 含访存/累加/epilogue 分量，稀释倍率。解锁后绝对吞吐
（49.3 TF）与 28e1 的 52.97 TF 同档（时钟差异）。数据完整性自检
（pattern_clean / injection_capture）全绿。

本卡（28e0）此前未被社区验证——最近的公开 canary 是 28e1（同芯片、同为
ASUS 移动端）。本报告将其补上。解锁为易失性质：每次冷启动经 U 盘重放一次，
无持久化、无刷写、断电即清。

**与 nvoc kmd 组合**：本工具（UEFI 侧，解除 tensor 发射速率钳制）与
[Skyworks-Neo/nvoc](https://github.com/Skyworks-Neo/nvoc) 的 kmd 功耗墙解锁
（OS 侧，解除功耗/电压墙）正交可叠加。组合实测：FP16 GEMM 8192 达
60–63 TFLOPS、芯片功耗 ~163.7 W、Hot Spot 104.4 °C、PerCap Reason=Pwr
持续——远超原厂 TGP 包络，散热与供电自担，物理损坏概不负责（见 [组合测试风险说明](faq.md#nvoc-results)）。

## 与 Linux kmod 路径的差异（为什么这条链不同）

gpu-unlock / gpu-unlock-ada 的 Linux 方案在 initramfs kmod 中执行，SEC2 的
Falcon DMA 引擎可用。Windows 的等价时窗是 UEFI DXE 阶段，实测发现该窗口内
两台引擎的 Falcon DMA 队列"接受 64 命令（2×32 槽）但从不消费"，且与 VT-d、
FBIF 配置、REQUIRE_CTX 均无关。本仓库因此以**寄存器 portal 装载**替代全部
DMA 装载（IMEMC/DMEMC 字节地址直写），这也是 Turing 内核装载 Booter 的原生
方式。此外本窗口内 BSI handoff 停在 1（非 DONE），内核驱动的
`kgspExecuteScrubberIfNeeded` 路径在 kmod 中从不触发——我们移植了它（内核
同款 SecurescrubUcode BINDATA）并实测 scrubber 执行后 handoff 推进到 3。

## 链条分步（15 步，全部真机读回）

| # | 步骤 | 状态 |
| --- | --- | --- |
| 1 | 枚举/认领（GOP+驱动断开、独占、IO/MEM/BME） | ✅ |
| 2 | 入口门（chip / WPR2 hi=0 / FEAT=0xffffff8f / SS 合理） | ✅ |
| 3 | VBIOS PROM 读（BAR0+0x300000，1 MiB aperture） | ✅ |
| 4 | FWSEC 抽取/解析/签名槽/FRTS 补丁 | ✅ |
| 5 | 布局计算（fb/vga/mmu-lock → WPR 五段，84 MiB 镜像槽） | ✅ |
| 6 | FWSEC 点火（HS@GSP portal + BROM 参数）→ WPR2 立起 | ✅ 可复现 |
| 7 | SEC2 scrubber（内核同款 BINDATA）→ handoff 1→3 | ✅ 可复现 |
| 8 | GFW 状态评估 | ✅ |
| 9 | Booter 资产（BTL1 解析 + stock 签名槽） | ✅ |
| 10 | GSP-RM 装载准备（radix3 84 MiB + WprMeta + 签名 DMA） | ✅ |
| 11 | **Booter 点火**（HS@SEC2 portal + BROM 参数 + 邮箱=WprMeta） | ✅ MAILBOX0=0 |
| 12 | 执行判据（回放=mailbox0+WPR2；payload=四 PLM+marker） | ✅ |
| 13 | 签名还原（payload 模式，FLR 前） | ✅ |
| 14 | FLR（TOTORONG 工艺：devctl 保留、pending 等待、100 ms 静默、BAR 恢复） | ✅ |
| 15 | 交棒（FLR 后 selector 存活验证 → bootmgfw 链回） | ✅ |

## 本仓库实现的关键修复（对后来者最有价值的部分）

1. **Falcon v4 portal 地址编码**：`IMEMC/DMEMC` 的地址字段就是**字节地址
   本身**（OFFS[7:2] + BLK[23:8] 原样落在寄存器 bits[23:2]；权威定义
   `dev_falcon_v4.h`，内核 `kflcnMaskImemAddr/DmemAddr = addr & (OFFS|BLK)`）。
   按 `addr>>2` 写会把 IMEM 第 k 块放到字节 64k（每 4 块互相覆盖），非零地址
   的 DMEM 读也偏 4 倍。零地址操作会掩盖此 bug。
2. **BootFromHs 只装载 secure 段且物理目标为 0**（`dest=imemPa=0`，虚拟 tag
   从 imemVa 取；镜像的 NS/os-code 段**不装载**）。把 NS 段装在物理 0、sec 段
   装在 0x100 并打重复 tag 会让核心从引导向量取到 NS 垃圾后立即异常停机
   （特征：EXCI 0x0100、TRACEPC 钉在 0x100、MAILBOX 不变）。
3. **SEC2 scrubber 用内核同款 BINDATA**（`g_bindata_ksec2GetBinArchiveSecurescrubUcode_AD10X`，
   7424 B、ucode 0xd、2×RSA-384），不是 VBIOS PROM 的 app 0x49（22528 B）。
   签名槽熔丝寄存器 = `0x00824140 + 4*(ucodeId-1)`，版本 = 最高置位位 +1。
4. **每次 HS 执行前必须有引擎复位**：scrubber 成功后 BSI 状态机会重新锁
   SEC2 的 PRI 窗口（读回 badf5620），复位重新打开它。BCR 切换后**不能**再补
   复位周期（实测会把切换抹回 0x110 并挂死 scrub 等待）。
5. **Falcon DMA 在本窗口不可用是引擎私有状态**：FBIF 三件套
   （`FBIF_CTL|1<<7`、`DMACTL=0`、`TRANSCFG(0)=5`）修完、REQUIRE_CTX=0、
   VT-d 关闭后队列仍"接受 64 命令不消费"。不要在此浪费时间——portal 路径
   完全等效（内核对 Turing Booter 本身就用 portal）。
6. **payload 签名拓扑**：镜像永远打 stock 签名（BROM 先验签放行 booter）；
   V67 blob（0xFA00=64000 B）住在独立签名 DMA 缓冲，WprMeta 的
   `sizeOfSignature=0xFA00` 使 booter 自身的无界签名拷贝溢出 DMEM 栈——
   溢出即 ROP 入口。0xFA00 装不进任何 booter 镜像，不要尝试打进镜像。

## 运维事实

- 解锁易失：每次冷启动 U 盘重放一次（EFI + `\v67-ad102-four-plm.bin`
  即解锁模式；去掉该文件为纯回放/诊断模式）。
- 失败路径与回放相同：判据不满足 → 拒绝 → FLR 交还冷卡 → Windows。
  黑屏/STOP → 断电（笔记本按厂商流程放空 EC，ASUS 为按住电源 40 s），
  同一会话绝不重试。
- 热重启后的存活情况未测（社区同样未测）。

### U 盘根目录与期望哈希（`make-usbroot.ps1` 再生产物，2026-10-26 实测与
解锁用 U 盘逐字节一致）

| 文件 | SHA-256 |
| --- | --- |
| `BOOTX64.EFI` | `dd1ed81b5357a6983e10f9e71f55380405201b7b9c9ee8bf67d55c473c0992df` |
| `booter_load_ad102.bin` | `6ee05b20e1dc244c08c052f36ef28469b8d09f4955d1fe33e48a179e11a7fd49` |
| `gsp_boot_desc_ad102.bin` | `83f8f3f1f5f9496e0ea8fc7103c2beb64d0b6de2f5ffe725f13514de5a95fa42` |
| `gsp_boot_ad102.bin` | `45d6335a8185d6ff13e8d1484cea73f68d3d13aa1c19dc080ed1c3482dd5bac0` |
| `fwimage_ad102.bin` | `004394a6b5a217ef2445631f1e59fbca448d3d8aaa07e001898bf7569b98eec4` |
| `fwsignature_ad102.bin` | `394612a0279ef8f8b457f91bfa35860e086d195d079975efa07b9f0938e019e7` |
| `scrubber_ad10x.bin` | `1cfc78136a4dff4993b7e5a1a09345f6a67d0762555e5bd9660e406831379348` |
| `v67-ad102-four-plm.bin` | `44c36b2d6b3424e53e18f3e55db85c833655de071ce66ce42749588a4a7dc8cb` |

再生是确定性的（输入钉 hash + 抽取/生成硬校验），任何人在任何机器上跑
`make-usbroot.ps1` 都应得到逐字节相同的产物；不符即停（脚本自动拒绝）。

## 安全边界

本仓库遵循与上游相同的纪律：NVIDIA 固件 blob、密钥、解密产物、生成的
payload 永不入库；一切资产从钉 SHA-256 的公开输入本地再生；写操作分级门控
（M1 零写 / M3 事务 / M4 载荷单独立项）；先模拟后真机；读回是唯一成功判据。
完整宪法见仓库根 `AGENTS.md` §1。

## 社区贡献与参照源

本项目是社区工作的直接受益者与衍生者。完整致谢（事务链母本、Turing 谱系、
同类先行工作、寄存器/熔丝侧线、报告者与协作者，均按仓库历史中的实际作用
核对归类）已独立成文：**[docs/credits.md](credits.md)**。

许可：**GPL-2.0-only**（与上游一致；见 `LICENSE`）。NVIDIA 固件与 VBIOS
版权归 NVIDIA，按其原许可条款从钉版公开包本地再生，不在本仓库分发。

## 解锁本质的对照实验（G614JV，FP16 tensor，2026-10-26）

不限制功率的 stock：3.0 GHz、155–160 W、**不撞功耗墙** → 35.9 TFLOPS。
解锁 SM 发射率（+nvoc 放墙）：平均 2.89 GHz、190–208 W、撞温度墙与供电
极限 → **64.30 TFLOPS**。频率 -3.7%、功耗 +34%、吞吐 +79%：多出的算力
只能来自 segmentation 掩码放行后**完整晶体管不再空转**——被砍的 lane
原本在时钟照打的前提下周期性执行 NOP/挂起槽位，放行后这些槽位装载真实
指令。不是超频，不是功率换性能（stock 在更高频率且不撞墙时也只有 35.9 T）。

## 组合测量明细与配图

本节保存同一台 G614JV / RTX 4060 Laptop `28e0` 的完整测量条件和读数。吞吐由 [nvoc 的 cli-stressor-cuda-rs](https://github.com/Skyworks-Neo/nvoc/tree/main/cli-stressor-cuda-rs) 测得；4096 与 8192 的数据对应不同规模，不作为同条件倍率对照。

| 工况 | 数值 |
| --- | --- |
| FP16 GEMM 4096（仅本工具解锁） | 49.0–49.3 TFLOPS（stock 29.89，1.65×） |
| FP16 GEMM 8192（+nvoc 功耗墙） | **60.8–63.1 TFLOPS**（相对 stock ≈ **2.1×**） |
| GPU 核心时钟峰值 | **3045 MHz** @ 1.200 V |
| 板级功耗峰值 | **206.8 W**（芯片 ~163.7 W + 显存 21.8 W + PWR_SRC 24.8 W；16-pin 164.4 W @ 19.8 V） |
| 温度 | Hot Spot 峰值 **101.8–104.4 °C**，核心 79–81 °C，显存 58–66 °C |
| 限流原因 | PerCap Reason 在 **Pwr**（功耗墙）与 **Thrm**（热墙）间切换 |

同卡、同 FP16 精度的发射率与功耗控制对照：

| 状态 | 频率 | 芯片功耗 | 墙 | FP16 tensor |
| --- | --- | --- | --- | --- |
| stock | **2.52 GHz** | 95~105 W | **电压墙** | 30.5 TFLOPS |
| 只解锁 SM 发射率 | **2.37 GHz** | 130-140 W | **撞功耗墙** | 47.50 TFLOPS |
| 只解锁功率（+nvoc 放开电压墙+极限超频） | **3.00 GHz** | 155–160 W | **不撞功耗墙** | 35.9 TFLOPS |
| 解锁 SM 发射率+功率（+nvoc 放开电压墙+极限超频） | 平均 **2.89 GHz** | 190–208 W | 撞温度墙+供电极限 | **64.30 TFLOPS** |

组合测试中，限流原因在功耗墙与热墙间切换。供电与温度值只描述这台机器的测试，不是其他显卡的推荐配置。原测试特别关注液金/硅脂、风扇、均热板/热管、适配器、VRM 与电源链路温升，并将 Hot Spot 与核心温差超过 20 °C、Hot Spot 持续超过 100 °C 作为散热风险提示。

组合测试截图：

<img width="1280" height="595" alt="RTX 4060 Laptop 与 nvoc 组合测试截图" src="https://github.com/user-attachments/assets/1f60b619-b367-4823-9764-1caa44575501" />

温度与供电监测截图：

<img width="287" height="520" alt="RTX 4060 Laptop 组合测试监测截图" src="https://github.com/user-attachments/assets/d5e5fb50-4504-4b76-ba43-56e5db078b8b" />

nvoc 由 [physicsdolphin](https://github.com/physicsdolphin) 主导开发。报告中 10/20 系桌面卡只有驱动读回、未验证软件解除功耗墙后的性能效果：报告者持有的这类卡已做 shunt 改造并显示“0 W”，不具备对应软件操作的对照条件。原组合说明涉及电压控制的 slot 0/1/2、Target Wall、VBIOS Wall 和 VRM Max Wall（Vop）；具体命令及可写项目以 nvoc 的文档和设备读回为准。
