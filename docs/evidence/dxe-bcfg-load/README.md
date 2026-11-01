# BCFG driver-load 形态验证（QEMU + Shell v2.2，2026-11-01）

配套文档：`docs/bcfg-driver-load.md`（机制/命令/恢复/安全准则）。
镜像构建：`mkbcfg.sh` + `mkbcfg2.sh`（WSL mtools，MBR FAT32，分区身份=签名 0 + LBA 0x800）。
驱动：`NvUnlockDxe-readonly.efi`（tx-OFF，subsystem 0x0B，含 pci_live 分派修复）。

| 轮次 | 日志 | 断言 |
| --- | --- | --- |
| R1 装机 | run-bcfg1-install2.log | 串口 Space 过 `explicit_authorize(30)` → `DXE-ARM: authorization result Ok(())`；重复装机复位预算 |
| R2 登记 | run-bcfg2-add2.log | `bcfg driver add` → `Add Driver0007 as 0`；dump 显示 DevPath 绑定分区 `HD(1,MBR,0x00000000,0x800,0x20000)/\NvUnlockDxe-readonly.efi`；10 次 `rm 0` 在本 shell 下静默无效（rm 语义坑，文档 §3.2） |
| R3 armed | run-bcfg3-armed3.log + loader-prev-bcfg3b.log | Driver-option 时点加载驱动（控制台未接，无输出=设计）；OVMF 已枚举 PCI → 立即路径 `notification=false` → FORM/ARM/FAILSAFE 1/2/OVERRIDES/FINAL 全链落文件；app 随后启动并轮换上一份日志（轮换机制顺带实证） |
| R4 负控 | run-bcfg4-negative.log + 文件日志 | 全新 vars（无 NVUX2）→ Driver#### 照常加载但**零驱动行**（console 与文件均无）：无授权=完全惰性；console 的 2 行 `loader::dxe` 为 app 的装机提示（install 标记在指纹内必须保留，属预期噪声） |
| 失败注入 | run-bcfg3-armed.log（前身轮次） | 控制文件集与装机快照不一致 → 静默 fail-closed；预算耗尽 → 惰性；startup.nsh 内 reset 自循环 → 每 boot 堆积一条 Driver 选项（多实例被 ENTERED+latch 去重，8 实例无冲突） |

未覆盖：真机 Driver#### 时序（stock EDK2/AMI = ReadyToBoot 通知路径；OVMF 走立即路径——
两条路径代码都在，`pci_live` 按环境自动选择）、SB-ON 签名链、真机 BitLocker。
