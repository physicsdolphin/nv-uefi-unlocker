# TU106 实测机与最终结果（v0.2.0，2026-10-27）

## 实测机

| 项 | 值 |
|---|---|
| GPU | NVIDIA GeForce RTX 2070（TU106，10de:1f07，8GB） |
| CPU | Intel Core i9-10980HK **ES 工程样品（QTJ1）** |
| 平台 | 台式（10980HK ES 桌面化改装平台），混合输出（iGPU UHD 630 + 2070） |
| 显示 | 2070 为 console owner（GOP 句柄唯一） |
| BIOS 前置 | Above 4G 开 / CSM 关 / Secure Boot 关（同 40 系要求） |

## 最终结果

- 解锁链：16 火从 layout EIO 推进到完整闭环（FWSEC 点火 WPR2 3ms →
  BOOT_DIRECT booter → V67 起爆 → PLM 0x409650=0xffffffff → SS0=0x88888888
  /SS1=0x8 写入读回 → 签名还原 → 无 FLR 交棒）
- Windows：**GSP 固件模式（EnableGpuFirmware=1，tools/windows/
  enable-turing-unlock.ps1，重启生效）+ 解锁 EFI = FP32 累加 GEMM
  36 → 60–63 TFLOPS（+67~75%）**
- **休眠恢复入口实测通过**：休眠恢复后进 U 盘 → 全链写入解锁 → Windows
  验证通过（解锁存活）。此前 Ada 侧"休眠恢复入口解锁可能被作废"的警告
  （docs/warm-resume-entry.md）在 TU106 + GSP 模式组合下未复现
- 失败路径全部可自愈（断电冷启即净卡；TDR 风暴轮驱动可自救恢复）

## 时序注意（Turing 专属）

GSP 模式注册表切换**必须 Restart**：休眠恢复从镜像还原驱动、不重新初始化，
切换不生效。流程=运行 enable-turing-unlock.ps1 → 重启（可顺路进 U 盘解锁）
→ Windows 验证。

## 解锁前后截图

与上述 RTX 2070 实测结果对应的截图：

解锁前：

<img width="800" height="400" alt="RTX 2070 解锁前测试截图" src="https://github.com/user-attachments/assets/6e61c642-7202-47a6-b6a0-cc2d986892d3" />

解锁后：

<img width="800" height="450" alt="RTX 2070 解锁后测试截图" src="https://github.com/user-attachments/assets/8c103363-8e44-4d46-ab47-a6c063ae1e6f" />
