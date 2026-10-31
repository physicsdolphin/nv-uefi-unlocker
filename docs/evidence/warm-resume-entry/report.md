# 休眠恢复（warm/resume 入口）下的解锁：log 为什么绿、Windows 为什么坏

> 本文保留特定运行的实验记录。现行操作与读回判定见 [FAQ](../../faq.md#display)。


> 状态补注（2026-10-28）：交棒后的三种候选机制可通过系统侧读回区分。仓库现提供 [Windows ssdiag v0.2](../../../tools/ssdiag/README.md) 和 Linux `tools/diag-unlock.py`，可与 HANDOFF-OUT 对照；下文 nvoc 探针是早期方法。#6 另有 GA104M 混合输出平台进入 Windows 后无收益的反馈，见[社区汇总](../../community-reports.md)。

2026-10-26，AD107M/28e0，混合模式，休眠唤醒后进 U 盘的一次完整运行。事务本身
逐项绿：`MAILBOX0=0x67b00001`、`FEAT_PLM=0xffffffff`、`WPRCFG=0xfffff0ff`、
`WPR=0xffffffff`、SS0/SS1 写入并读回 `0x88888888/0x8`、签名已还原、FLR 后
`WPR2=0x1ffffe00/0x0`、`selectors survive AON`、随后正常链回。与 28e0 上全部
M4 成功记录同形。**写入是真的，进 Windows 后解锁仍然不成立。**

## 为什么这两件事不矛盾

日志在 `HANDOFF-OUT`/`CHAIN: LoadImage` 处结束。所有读回都发生在"交出控制权
之前"，而能作废这次写入的事情全部发生在这之后——覆盖边界是结构性的：

- **读回证明的是"卡收下了写"，不是"下一个消费者认得这个状态"。**
- `SS` 在 AON 里只保证扛得住 FLR，不保证扛得住断电；而断电正是 Windows 在混合
  模式下对非 panel-owner dGPU 会做的事（显示权回到 iGPU 的那一步）。
- 这条运行路径也不是 POST：固件没跑 devinit，卡是带着上次驱动会话的残余上下文
  进来的。写入前 `SS0` 已经是熔丝派生值 `0x5112071`（冷态对照组同一位置是 0）——
  唯一一条能看出"卡不冷"的寄存器证据。

## 三条候选机理与判别

| # | 机理 | Windows 侧读回的指纹 |
|---|---|---|
| 1 | **导轨被切**：显示权移交 iGPU 时 dGPU 被打进 D3cold，AON 随电消失 | `FEAT_PLM` 也回到锁定基线 `0xffffff8f`，`READOUT_1` 回到节流值，SS 为熔丝值或 0 |
| 2 | **驱动 devinit 覆盖**：驱动从熔丝算速率并写回 SS | SS 被改写（`0x5112071/0x4` 之类），但 `FEAT_PLM` 仍是 `0xffffffff`（PLM 是 AON） |
| 3 | **驱动在半熟卡上初始化失败再恢复**：残余上下文 + D3hot 让首次 bring-up 走不通，恢复路径做全量重初始化，SS 顺带回熔丝值 | 同 2，外加设备管理器错误码 43/黄色感叹号、事件日志里 nvlddmkm 复位/重试 |
| — | 三者都排除 | SS 仍是 `0x88888888/0x8` 但性能是 stock → 查找别的消费者（时钟钳、功耗墙） |

`WPR2_HI` 是辅助判据：驱动起来后会为 GSP-RM 建自己的 WPR2，`HI≠0` 说明驱动确实
初始化过这张卡；`HI=0` 说明根本没人消费这次解锁。

## Windows 侧怎么读（nvapi-rs，需管理员）

`nvoc` 里已加好只读探针（NVAPI `RegisterOp 0x2EB3C140`，实测走真 MMIO：

```powershell
# 管理员 PowerShell
cd D:\git-repo\nvoc\nvapi-rs
cargo test --test uefi_handoff_read_live -- --nocapture --ignored
```

非管理员时所有偏移一律 `rc=-1`（通道的内核权限门，与偏移无关），探针会直接说明。
输出是五行寄存器 + 一句 `VERDICT:`，与 U 盘 `\loader.log` 里最后一行 `HANDOFF-OUT:`
逐字段对照即可。临时读别的偏移：`set HANDOFF_OFFSETS=0x82381c,0x823820`。

`PMCSR` 属于配置空间，RegisterOp 读不到；需要时用 `lspci -s <bdf> -xxx`（pciutils
Win32）或 RWEverything 的 PCI 视图看 `PMCSR` 低两位。

## 代码侧已做的两件事

1. **PM 预检**（`tx_detach::pm_preflight`，在任何断开/写入之前）：读 PMCSR，
   非 D0 时打 `PM: WARM ENTRY ...`，然后请求 D0 并**读回确认**；不落实即拒绝本次
   事务。U 盘根目录放 `\tx-require-d0.txt` 可把警告变成直接 abort（此时零写入、
   零复位，直接进 Windows）。
2. **出口状态行**（`tx::handoff_out`，链回前最后动作）：单行 `HANDOFF-OUT:
   ss0= ss1= readout1= feat_plm= wpr2_hi= pmcsr= dstate=` 加三个布尔判据。
   失败/EWarm/EALREADY 路径也打，便于对照。
3. FLR 前后保存/恢复 PMCSR 并把状态位钉在 D0（两个 FLR 实现都改了）——PCIe 只在
   D0 定义 FLR，Linux `pci_reset_function()` 同样先回 D0。

## 规则（不用改代码也该守）

- 从 Windows 选**重启**，或冷启后从启动菜单进 U 盘；不要在休眠唤醒后直接进。
- **关掉快速启动**：开着时"关机"是 S4 型休眠启动，下次开机仍走 resume 路径。
- 恢复模式（独显直连）不受影响：dGPU 始终是 panel owner，固件必须无条件 POST，
  休眠与重启都可用。
