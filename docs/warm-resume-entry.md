# 休眠恢复与交棒排障

现行操作与判定统一见 FAQ：

- [启动入口、混合输出与休眠](faq.md#display)
- [系统内读回、状态修复与驱动检查](faq.md#ssdiag)
- [Linux RTD3](faq.md#linux)

独立运行的环境、观测及候选机理保留在[休眠恢复实验记录](evidence/warm-resume-entry/report.md)，不作为通用操作步骤。

### Linux kmod 侧对照（2026-11-02 补录）

上游 Zagitta/gpu-unlock 的同域语义：suspend 后 selector 失效，**resume 时在驱动恢复前自动重放**（约 +1s；`gpu_unlock.resume=0` 关闭）；**休眠（hibernation）不覆盖**；笔记本 dGPU 掉电即失，社区惯例 `NVreg_DynamicPowerManagement=0x00` 保活。与本工具同构关系：易失覆盖=每次引导重放；Windows 侧对应动作为休眠唤醒后 `ssdiag --reapply`（RTD3/D3cold 擦解锁判定见 FAQ [Linux RTD3](faq.md#linux) 与 issue #9 记录）。差异点：kmod 在驱动绑定后仍可经 PRI 通路重放，本工具依赖 UEFI 预驱动窗口或系统内 PLM 开窗——因此 warm 入口机理（#9）在本载具上是独立问题。
