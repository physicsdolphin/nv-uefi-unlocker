# 社区硬件与第三方方案记录

本文保留独立社区实验的平台、版本、EFI 读回与性能数据。现行支持结论见[兼容性矩阵](compatibility.md)，操作与排障见 [FAQ](faq.md)。表中的版本和结果只描述对应实验。

## 硬件记录

吞吐单位为 TFLOPS。各行使用的脚本、矩阵规模和功耗条件见对应报告。

| 来源 | GPU / 版本 | 平台与进入方式 | 固件与系统侧证据 | 补充记录 |
| --- | --- | --- | --- | --- |
| [#3](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/3#issuecomment-6434240012) | RTX 4070 SUPER，AD104 `2783`；v0.1.4 | Palit，MSI B760 TOMAHAWK WIFI，i5-13400F，Windows 11，驱动 621.01，220 W；每次冷启 | 桥 BME 修复后 replay/unlock 通过；marker、SS0/SS1、FLR 后出口读回正确。PyTorch/cuBLAS，7 次取中位数，8192：FP16→FP32 累加 70.5→100.1、BF16 71.0→107.0、TF32 35.5→48.4；FP16 累加与 FP32 SIMT 对照基本不变 | 报告中附关键日志，完整日志可由报告者提供；1024³ 对 FP64 参考的检查前后逐位一致，其他工作负载待测 |
| [#27](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/27) | RTX 3080 Ti Laptop 16GB，GA103M `2420`；v2.1.0 | MSI Vector GP76 12UHSO，独显直连，175 W，AC；Windows 11，驱动 621.01，VBIOS `94.03.19.00.90`；独立 FAT32 分区经 EFI Shell 进入，未写系统 ESP | replay 通过，PLM 翻转、选择器读回及 FLR 存活；8192 GEMM 各 3 轮中位数：FP16 49.23→77.80/80.35，BF16 50.35→77.64/80.84；FP32 12.80→12.64/12.68，数值检查通过。[日志与方法](https://gist.github.com/INsincere123/f0778d5378fdb5bb64da1aaabdefa341) | 未做 Windows 内 FEAT 读回或长期稳定性验证。报告者曾同会话再次执行后出现 BIOS 显示异常、NVMe 不可见，EC 重置恢复；首次日志被覆盖，原因不明。后续单次 replay 与两次解锁正常 |
| [#28](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/28) | RTX 3070 Laptop 8GB，GA104M `249d`；v2.1.0 | AMD Cezanne，AMI，MUX 独显直连，140 W；Windows，驱动 616.56 | PLM 翻转约 158 ms，SS0/SS1、签名还原、FLR 和 HANDOFF-OUT 读回正确；cuBLAS FP16 + FP32 累加：8192 为 35.43→56.04，4096 为 37.82→60.96 | 该平台混合模式下仅枚举音频功能，切独显直连后通过；#6 另有同 ID 混合输出机器的异常记录 |
| [#29](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/29) | RTX 3080，GA102 `2206`；v2.1.0 | MSI MEG Z490 UNIFY，单卡直连，340 W；Windows 10，驱动 610.74，VBIOS `94.02.26.c0.b6` | WPR2、PLM 翻转、选择器、签名还原及 FLR/出口读回通过，Windows 设备状态正常。torch/cuBLAS FP16，median / best：4096 为 48.36/56.83→104.55/105.93；8192 为 63.96/65.65→99.67/106.44；12288 为 66.10/66.69→101.98/104.57 | 62 秒负载持续 99.01，约 338 W、80 °C，报告 SW power cap；报告者据此判断收益受功耗墙限制，长期稳定性待测 |
| [#31 已合入报告](evidence/ad106-2805-nuc13/report.md) | RTX 4060 Ti 16GB，AD106 `2805`，ASUS `1043:8921`；v2.1.0 | Intel NUC13RNGi9，Windows 11，驱动 617.42 | marker、PLM、选择器及 FLR 存活通过；同次启动 8192 GEMM FP16 45.78→75.24、BF16 44.84→68.80，FP32 11.95→11.87；75 秒负载，最高 72 °C | 单卡单 VBIOS；Windows 侧使用性能测量，未安装 ssdiag 的内核通道；ComfyUI 稠密 bf16 3.39→2.16 s/it，报告的流式量化视频工作流无收益 |
| [#33](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/33) | MSI RTX 4060 Ti 16GB，AD106 `2803`，子系统 `1462:5174`，VBIOS `95.06.34.00.f3`，165 W（未调）；v2.1.4（8d925da 自编译，`-Family ad102`） | MSI MAG B550M MORTAR WIFI（MS-7C94）+ Ryzen 5 5600X（**无内显，屏幕只接 4060 Ti**），Windows 11 Pro 26200，Secure Boot 关，驱动 610.88 | 先做无 v67 的 replay 演练全过，进 Windows 无 nvlddmkm/WHEA 错误；解锁 `MAILBOX0=0x67b00001`、SS0/SS1=0x88888888/0x8、FLR 后保留、readout1=0。同驱动/功耗/模型/种子：FP16→FP32 8192³ 45.72→67.30、BF16 46.06→69.00、TF32 22.20→32.47，FP32 对照 12.15→12.02；ComfyUI Z-Image-Turbo 1024² 12.1→8.0 s/张（×1.51）、Wan 2.2 I2V ×1.47，同 seed 图逐像素相同 | 工具与自动解锁脚本（不含 NVIDIA 固件）：https://github.com/chungtair/rtx4060ti-auto-unlock 。两个对其他用户有用的观察：①**免 U 盘开机自动解锁**——数据盘 300 MB FAT32 分区放 BOOTX64.EFI+资产+`i_know_what_i_am_doing.txt`，`bcdedit /copy {bootmgr}` 建固件启动项排 `{fwbootmgr}` 第一，每次开机 5 秒倒数自动解锁再接 bootmgfw；②**MSI B550 无视 Windows 写的 `{fwbootmgr}` 顺序**——须进 BIOS 手动设 Boot Option #1，否则 EFI 根本不被启动。持续满载测法被 165 W 压在 2190 MHz，67 比其他 4060 Ti 报告的 75~78 低属测法差异 |
| [#36](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/36) | RTX 4070 Ti Super 16GB，AD103 `2705`，VBIOS `95.03.45.40.2b`；v2.1.0（sha256 `b5e96f4e…`） | AMD 桌面（上游桥 `1022:14db` @ 00:01.0），dGPU 独显直连（NVIDIA 持有 4K 控制台），Secure Boot 关；Windows 高级启动 → `UEFI: NVUEFI`；驱动 621.01 | 完整四写解锁：marker `0x67b00001`、FEAT/WPR `0xffffffff`、WPRCFG `0xfffff0ff`、FBPA `0xffffff0f`（观察项非致命）、选择器穿 FLR 存活、readout1 由 `0x00410080` 归零；前置 replay 同绿。sm_89 wmma 发射率微基准 5 轮最好：61.98→125.29 TFLOPS（**2.02×**，同二进制同条件）；满载 143.5 W / 50.9 °C / 2805 MHz（未动功耗墙） | BRIDGE-BME：上游桥 `1022:14db` 固件阶段 command `0x0003` 未开 Bus Master，PR #5 修复在该 AMD 平台为必需（无修复预计落 0x89）。另两次中止运行（30s 超时/非空格键）确认确认闸门零写按设计工作。`SEC2_CPUCTL=0xbadf5620 [BADF_VARIANT]` 与 docs 的 AD103 特征一致 |
| [#50](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/50) | RTX 4060 Laptop 8GB，AD107 `28a0`，VBIOS `95.07.16.80.e1`，TGP 140 W；v2.1.4（c2b0706，源码自建 `cargo efi-tx`） | MECHREVO 极光15（同方 GM5AQ0O），i7-12800HX，**独显直连**（无 Intel 显示适配器），Windows 11 26300，驱动 621.01，AMI BIOS `N.1.06MRO13`，Secure Boot 关；固件启动菜单进 U 盘，标准空格确认 | 全链一次通过：WPR2 验证 → marker `0x67b00001` → 四写 → 选择器读回 → 签名还原 → FLR 后 AON 存活 → bootmgfw 交棒；出厂冷值 FEAT_PLM=0xffffff8f、SS0=0x50404010、SS1=0x3。PyTorch 2.11+cu128 同次冷启 A/B：FP16 8192 中位 26.37→50.29（×1.91）、30 s 持续 49.30（×1.86），BF16 8192 ×1.89，FP16 4096 ×1.79；板级功耗 88.0→139.1 W（峰值 140.09 钉 TGP 上限），SM 频率反降 1.5%，核心温度 73→84 °C | 解锁后瓶颈已从发射率转为 TGP——"频率降、功耗涨、吞吐翻"指纹一致。BRIDGE-BME：Intel RPL 根口 `8086:460d` 默认未开 Bus Master，PR #5 修复在该平台触发（无修复预计 0x89）。FP16 4096 短时爆发在功耗墙上下振荡（best 54.76 / worst 34.52），建议以 8192 为准。附完整 loader.log（356 行） |
| [#51](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/51) | RTX 4060 Ti 16GB，AD106 `2803`（16 GB 卡报 2803，nvidia-smi 16380 MiB）；v0.2.0 | AMD APU 平台桌面（Radeon iGPU 在位，显示接 4060 Ti，上游桥 `1022:43f4`/`43f5`），Windows 11 22621，Secure Boot/BitLocker 关，驱动 610.62 | 一次通过全链单发解锁，自动链回 Windows Boot Manager；PR #5 桥 BME 修复在该平台开箱即用（上游桥 command 已为 0x0007，无 0x89）。marker、四写、FLR 后 AON 存活、HANDOFF-OUT 读回正确。PyTorch 2.13+cu126 4096³ 7 轮中位：FP16→FP32 累加 45.96→75.40（+64.1%）、BF16 44.61→78.72（+76.5%）、TF32 20.81→36.44（+75.1%），FP32 strict 对照 13.05→12.03（噪声） | 数值检查对 FP64 参考前后逐位一致。与 PR #5 桌面 2803 结果（46.78→75.28）几乎复现。报告者指出 `2803`+16 GB 组合与 Linux 侧 gpu-unlock-ada 记录一致，值得记入矩阵 |
| [#53](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/53) | RTX 4080 16GB，AD103 `2704`（原厂 VBIOS 未改装）；v2.1.0（sha256 `b5e96f4e…`，`-Family ad102`） | Ryzen 9 9950X3D / 64 GB DDR5-6000，Windows 11 24H2（26100），驱动 621.01，4080 直接出画面（无第二张 NVIDIA 卡），Secure Boot/BitLocker 关 | **冷启动一次通过**：WPR2 → marker `0x67b00001` → PLM 翻转 → 四写 → 选择器读回 → FLR 后 AON 存活 → bootmgfw 交棒。PyTorch 2.11+cu128 FP32 累加两次独立 A/B：FP16 4096³ 100.53→185.50/185.24（**1.85×**）、FP16 8192³ 104.49→164.31/162.94（1.57×）、BF16 4096³ 100.37→192.18/189.44（**1.91×**）、BF16 8192³ 104.39→172.31/177.28（1.65×），FP32 strict 对照 34.47→34.42/34.20 不变 | 8192³ 倍数低于 4096³ 的解释：大 tile 单 kernel 更长更易撞 340 W 功率上限（实测峰值 341 W）；FP32 对照排除热/状态解释。FBPA=0xffffff0f 与所有 Ada 报告一致（非致命）。附完整 loader.log、两次 bench.log 可按需提供 |
| [#56](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/56) | 主卡 RTX 5060 Ti 16GB（Blackwell `2d04`，显示输出）+ 副卡 RTX 4060 Ti 16GB（AD106 `2803`，PCIe 4.0 x4）；v2.1.0 | Ryzen 5 9600X + 32 GB DDR5-6000，Windows 11 26H2（26300.9711），驱动 617.42 Studio | **双卡环境 loader 正确路由**：`2d04` 无资产家族映射被安全跳过，`2803` 进 ad102 流程全链通过（marker、四写、SS0/SS1 读回 0x88888888/0x8、FLR 后 AON 存活、HANDOFF-OUT）。cli-stressor-cuda-rs 8192³（副卡）：FP16 44.94→71.81（+59.8%）、BF16 45.36→74.15（+63.5%）、TF32 22.29→33.77（+51.5%）；SDXL 1024² DPM++ 2M 24 步 2.20→2.94 it/s（+33.6%，与 gpu-unlock Real workloads 表预期相符） | **首个 Blackwell 负控数据点**：5060 Ti 作为显示主卡被安全跳过且系统正常。副卡 PCIe 4.0 x4 属带宽瓶颈，TFLOPS 略低于理论极限；测试工具不支持 FP8。副卡连续稳定运行 24 h+，日常满载无掉驱动/死机/计算错误 |
| B 站评论区反馈（2026-10-30 收录，无 issue） | RTX 4070 Ti Super 16GB，AD102 `2689` | 平台细节未回传（B 站评论区） | 驱动侧确认 `VERDICT [ada]: UNLOCKED`：PLM=0xffffffff（OPEN）、SS0=0x88888888、SS1=0x8、READOUT_1=0x00000000 [full speed]，四写值在驱动加载后仍有效。中位对比：FP16 4096 57.00→150.93（**2.65×**）、BF16 4096 57.51→127.32（2.21×）、FP16 8192 55.50→127.28（2.29×）、BF16 8192 54.97→128.39（2.34×） | 功耗由 stock 约 227 W 升至 291.58 W / 295 W 上限，温度 59 °C，`throttle_reasons=0x4`（功率制动）——瓶颈已从发射率转为功耗墙；倍数与 AD103/AD106 实测一致。**这是 AD102 的首个实测数据点**；EFI 侧 loader.log 未归档，如报告者后续回传可补齐固件侧证据 |
| [#63](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/63) | RTX 4060 Laptop 8GB，AD107 `28e0`，PCI 0000:01:00.0；v2.1.0（BOOTX64.EFI sha256 `b5e96f4e…`） | Acer Predator PHN16-71，i9-13900HX，INSYDE BIOS V1.18；Windows 11 Pro for Workstations 26300，驱动 616.92；140 W/180 W 上限由用户调整（测试脚本未写任何寄存器） | 三份 loader 日志全链读回一致（`TX: execution verified`、SS0/SS1、`POST-FLR: verified`、HANDOFF-OUT、链回 bootmgfw）。MMA `mma.sync.m16n8k16` 指令微基准 8 次记录：stock 33.512 → 解锁 67.527 TFLOPS（**+101.5%**，24 SM，2820–2865 MHz 快照；含后台占用时的低分 26.772 一并保留）；cli-stressor FP16 GEMM 3 次 55.36/56.53/56.83（140/140/180 W；`total_flops/compute_s` 口径，与 MMA 不可直接比）；140→180 W 增益仅 +0.26%/+0.53%（与频率快照差 2.14% 相符） | **28e0 的首个独立社区复现**。全部原始数据（8 MMA + 3 GEMM + 功耗遥测 + 三份日志）打包附 SHA256SUMS/provenance.json 归档在该 issue；两方法数值校验均零失败。与本报告 28e0 首测（29.89→49.33，1.65×）差异属口径不同：MMA 指令发射率 vs cuBLAS GEMM |
| [#69](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/69)（**已定案：钉频归一化 2.00×；issue 仍开放跟进残留项**） | RTX 4090 24GB，AD102 `2684`（SUBSYS 1458:40C3）；v2.1.6 + `v67-ad102-four-plm.bin`，驱动 581.80，torch 2.13.0+cu130 | Gigabyte，AMD 平台，BDF 10:00.0；同机 3× GTX 1080 Ti 正确跳过（负控）；功耗上限顶满 479 W | loader 全链绿：POST-BOOT `FEAT_PLM=0xffffffff`、marker `0x67b00001`、SS0/SS1 写入读回、`POST-FLR: verified (selectors survive AON)`、HANDOFF-OUT 全对；约 2 h 后 Windows 侧 ssdiag `--reapply` 确认 `VERDICT [ada]: UNLOCKED`（值已在效，驱动不改写） | boost 频率原始增益 1.45–1.71×（TF32 4096 61.52→104.99、FP16 4096 118.42→171.52；FP32 shader 对照 1.04× 不变）。**钉频消融定案（2026-10-31 回传）**：1500 MHz 钉频 + 210 W 余量（功耗不可能是限制项）下做同频 A/B，每时钟每 SM 归一化后无钳路径达全速率（938 FLOP/clk/SM）的 88–93%，相对 stock 50% 基准即**理论 2.00×**；功耗墙确认为 boost 频率下 1.45–1.71× 的原因（fp16acc/fp32acc 比率 1.52@boost → 1.39@1500 MHz 的移动方向相符）。报告者撤回 `0x823810` 第二速率选择器假说（无证据）。残留项：gated（fp32 累加）路径同频下仍为无钳速率的 ~67% 而非齐平，继续在 issue 跟进 |
| [#70](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/70) | RTX 3060 12GB，GA106 `2504`（SUBSYS 1043:8818，REV A1）；v2.1.6（`-Family ga102`），驱动 617.42 | ASUS ROG STRIX X470-F GAMING（AMI 6223）+ Ryzen 5 5500，独显直连；Windows 10 Pro 22H2 19045.6466 | 第一跑：wait_plm PLM 翻转 ~151 ms、SS0/SS1=0x88888888/0x8、FLR 后 AON 存活——**事务全绿后递归 `ConnectController` 复位平台**（#13 缺陷首个 AMD 平台复现，跨芯片组确认）；第二跑 `\no-reconnect.txt`：完整成功链回 bootmgfw（NVIDIA 显示自动跳过）；ssdiag `VERDICT [ampere]: UNLOCKED`（FUSE_VER=0x3、PMC_BOOT_0=0xb76000a1、FB 0x3000 MiB） | torch 2.12.1+cu130 FP32 累加：FP16 4096 25.37→45.79（**1.80×**）、BF16 46.31（1.83×）、FP16 8192 46.90（1.82×）、BF16 8192 47.56（1.82×）；ComfyUI anima BF16 ×1.62–1.75；峰值功耗 147.9→170.2 W（TGP 墙）、SM 频率 −5%、温度 +3 °C——"频率降、功耗涨、吞吐翻"指纹。stock 25.3–26.1 与官方标称 25.6 吻合。**GA106 首个全链跑通**（此前 2560 被旧判据误杀、25A2 有入口竞态）。两轮均见 `scrubber did not complete (ETimeout) - proceeding`（社区链形态）后事务成功，GA106 是否稳定特征待更多样本；两份 loader.log 归档在 issue |
| [#71](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/71) | RTX 3060 12GB，GA106 `2520`（SUBSYS 000010DE；报告标题自述 VRAM 改动卡"3060m 12G"）；v2.1.6（`-Family ga102`，**Google Colab 构建**） | HUANANZHI X99-4MT V1.0，Secure Boot 关；Windows 10 Enterprise LTSC 19044，驱动 610.47 | 解锁成功，PyTorch GEMM 对比：FP16 **≈1.83×**、BF16 **≈1.86×**、FP32 high ≈1.60×；FP32 highest/INT8/SDPA 基本不变（报告者已注明：INT4/2/1 与 FP8 条目为量化输入走 FP16 GEMM，非原生吞吐） | **首例 Colab（云端 Linux）构建 ga102 资产→U 盘引导**的成功报告，构建流程可复现；before/after 截图归档在 issue。报告者注明结果只代表该卡该配置 |

已有的 [#8（4070 `2709`）](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/8)、[#11（3060 Ti LHR `2489`）](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/11)、[#12（2060 Super `1f06`）](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/12)记录见[完整兼容性矩阵](compatibility.md)；[#2](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/2) 的 Turing 计算解锁与 BAR1 扩展是不同工作，后者没有随计算解锁一并实现。

## 补充实测数据

以下记录补足兼容性矩阵所引用的实验条件；单位为 TFLOPS，数值仅代表对应平台和方法。

### RTX 4060 Ti 8GB · AD106 `2803`（首个 8 GB `2803` 数据点）

来源：[PR #87](https://github.com/physicsdolphin/nv-uefi-unlocker/pull/87)，报告者 @fujiaghost789-glitch。ASUS PRIME B760M-F D4（Intel B760/AMI），8 GB `2803`——此前所有 `2803` 报告均为 16 GB。解锁读回通过（PLM 开、SS0/SS1=`0x88888888`/`0x8`、READOUT_1=0），cuBLAS 4096³ 三次取中位：FP16 45.59→77.36（+69.7%）、BF16 45.64→77.74（+70.3%）、TF32 20.06→41.91（+108.9%），FP32 对照为噪声。**附带修复**：该机 VT-d 关闭后上游桥 `8086:a70d` 以 BME 清零态起步，暴露 `BridgeMaster` guard 在 else 块内提前 Drop 的缺陷（booter `0x89` 家族）——修复随 PR #87 落入 main（ae7b128），对 BME 已开的既有验证配置为无操作。

### RTX 4070 · AD103 `2709`

来源：[#8](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/8)。B760 台式独显直连，四写读回通过，选择器在 FLR 后保留。系统侧 sm_89 MMA 微基准：FP16 约 61→123.91、BF16 118.90、FP8 243.55，报告的倍率均约 2.0×。

### RTX 3080 LHR · GA102 `2216`

来源：[#15](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/15)。B550、无 iGPU、单显示卡。FP16 GEMM 8192 为 57.72→94.0–96.9（+63~68%），BF16 +74~79%，TF32 +52~67%，ComfyUI SDXL +28.8%；per-SM 速率与 3090 Ti 参考卡同带。

### RTX 3060 Ti LHR · GA104 `2489`

来源：[#11](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/11)，报告者 @hunan-NRT，实测版本 v2.1.0。B450M 迫击炮 + 5700X3D，使用跨厂映众冰龙 3060 Ti VBIOS。PLM `0xffffff8f`→`0xffffffff` 翻转读回约 149 ms；SS0/SS1=`0x88888888`/`0x8`，FEAT_READOUT_1=0，Windows 交棒正常。

### RTX 3070 · GA104 `2484`（Thunderbolt 3）

来源：[#54](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/54)。v0.2.0 本地构建的 EFI 全链成功，FLR 后选择器保留，但进入 Windows 后状态不保留。同机 Linux `host_reset=N` 路径可保留，该结果不等同于本工具的 Windows 交棒成功。

### RTX 2060 Super · TU106 `1f06`

来源：[#12](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/12)，报告者 @vadash，实测版本 v0.2.0。AMD AM4/Promontory 平台、8 GB SKU；FWSEC、WPR2、BOOT_DIRECT booter 与 PLM 开启通过，SS0/SS1=`0x88888888`/`0x8`，签名恢复、PLM 重锁后值保留，解锁路径跳过 FLR，Windows 交棒正常。

### 上游互连条件观察

[cmp170hx](https://github.com/Consensus-Protocol/cmp170hx) 的 Thunderbolt 3 eGPU 记录为 `Booter Load 0x15`，Oculink 路径可用。这是上游工具的互连观察，不等同于本 EFI 工具在所有 Oculink 平台上的验证。

## 社区免 U 盘工具记录（非官方，不背书）

§1.4 对项目交付物绝对（决策记录见 [#73](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/73)）：本项目不收录常驻引导项 installer，也不提供官方教程。以下社区方案仅作存在性记录——非官方、不背书、不承诺兼容；常驻启动项的风险模型（每次上电自动重放固件事务、chain-load 前失败即黑屏且需恢复介质或固件菜单）由使用者自担：

- [#33 报告者的脚本](https://github.com/chungtair/rtx4060ti-auto-unlock)：数据盘 300 MB FAT32 分区 + `{fwbootmgr}` 固件启动项，AD106/MSI B550 实测（上表 #33 行；注意 B550 无视 Windows 写的启动顺序，须进 BIOS 手设 Boot Option #1）。
- [#73 提案者发布的 downstream 工具](https://github.com/Catnip202X/nv-uefi-unlocker/tree/downstream)：`downstream/install-partition.ps1`，独立 FAT32 分区安装/卸载（SHA-256 逐文件校验、manifest 先于启动项写入、拒绝 ESP/可移动盘/部分资产集、`-AllowArmed` 门控自动确认）。作者如实声明脚本从未实际执行，首个真实运行即测试。

## 仍待复测的反馈

逐卡状态与例外统一列在[兼容性矩阵](compatibility.md#reports)。系统内无收益按 [FAQ 读回检查](faq.md#ssdiag)定位，Turing `0x91` 的研究见[取证矩阵](tu10x-booter-0x91-matrix.md)。

## 使用说明与实现边界

- [USB 布局、确认键、回放与高级选项](faq.md#usb)
- [安全约束与实现边界](faq.md#safety)
- [家族执行与交棒判据](faq.md#mechanism)
