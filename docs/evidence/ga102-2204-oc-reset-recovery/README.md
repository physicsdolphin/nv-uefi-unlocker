# GA102 3090 ×2 超频失败 → 驱动级复位恢复：解锁全程存活（2026-11-21，维护者台面）

一次"歪打正着"的运行时对照实验：超频失败双卡挂死后走驱动级复位恢复梯子（FLR 与 SBR 两路都实证成功），**每轮恢复含完整 rebind + RmInitAdapter，FEAT 三连原样**——运行时驱动发起的 FLR/SBR + GSP 重初始化不碰 AON 岛；同时坐实机制表第 3 行（init 期重写）不是 init/rebind 的固有步骤而是平台条件分支。

## 现场

| 项 | 值 |
| --- | --- |
| 主机 | Supermicro X12DPG-OA6（BIOS 2.5, 2025-11-26），2× Xeon Gold 6326（Ice Lake-SP） |
| OS / 驱动 | Ubuntu 24.04.5，kernel 7.0.0-43-generic，NVIDIA open RM **GSP 620.70.06**（2026-11-20 构建），`DynamicPowerManagement=3`（RTD3 开着，advisory 在场） |
| 卡 | **2× RTX 3090**，`10de:2204`，SUBSYS `38811462`，VBIOS `94.02.59.00.8A`，PCI `ce:00.0`（NVML 0）+ `d6:00.0`（NVML 1），family ga102 |
| 本 boot 起点 | 2026-11-20 23:21（journalctl `NVRM: loading` 行）；loader 在该次引导武装金丝雀 |
| 事件线 | 11-21 12:54 D6：Xid 62 + `_kgspRpcGspEventPmuHalted` + **Xid 154（恢复动作 None → PF FLR）**；13:31 CE：同样三连；之后 nvidia-smi 双卡 `ERR! [GPU requires reset]` |

## 恢复过程（工具见下）

- **D6（一次成功，14:16:42）**：Tier 1 `nvidia-smi -r` 拒绝（Not Supported）→ Tier 2：停 DM → 卸 nvidia_uvm/drm → unbind → **`reset_method=bus`（SBR）** → rebind → 等 RmInitAdapter → SUCCESS。
- **CE（五轮迭代后成功，14:56:32）**：前几轮 unbind 被 fd holder 挡、Tier 4 remove+rescan 失败（`write error: Invalid argument`）、Tier 5 模块重载后仍不健康——期间日志留下完整 wedge 签名（`gpuWaitForGfwBootComplete_TU102: failed` progress `0x9`、`kgspWaitForGfwBootOk: 0x55`、`RmInitAdapter failed! (0x62:0x55:2205)`、probe `-EIO`）；脚本修复（设备未绑定时直接复位 + **`reset_method` 默认 `flr`**）后一次 SUCCESS。

**恢复工具**：NVOC 仓 [`linux_oc_recover.sh`](https://github.com/Skyworks-Neo/nvoc/blob/main/auto-optimizer/test/linux_oc_recover.sh)（五层梯子：`nvidia-smi -r` → GSP-wedge 总线复位 → unbind+FLR → remove/rescan → 模块重载；`--force` / `--kill-holders`）。

## 结果与判读

恢复全部完成后 `tools/diag-unlock.py` 双卡读回（日志尾部含第二轮复读，再双绿）：

```
FEAT_PLM  = 0xffffffff (OPEN)     FEAT_SS0  = 0x88888888 [full*×8]
FEAT_SS1  = 0x00000008 [full*]    READOUT_1 = 0x00000000 full speed
CANARY_A/B = 0x00000000/0x00000000 -> cleared
VERDICT: UNLOCKED (override values live)      ← 双卡
```

1. **运行时驱动发起的 FLR/SBR + GSP 重初始化不碰 AON 岛**——此前"FLR 不掉"只有事务内 `POST-FLR` 自证与 TU106 bench 两条单发，这是第一条系统内、驱动发起、蓄意多轮的记录。
2. **rebind + RmInitAdapter 完整执行（多次 `NVRM: loading` 自证）而 FEAT 原样** → 机制表第 3 行（[#148](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/148)）的 init 期重写**不是 init/rebind 固有步骤，是平台条件分支**（该机未触发；25a0 是已知唯一触发形态，写入方与触发变量未切开）。第 3 行由此更名"**init 期主动重写**"（"devinit 重放"误名撤回——#148 现场无复位，复位后 devinit 重算时序不成立）。
3. **金丝雀见证粒度**：cleared 只见证"本引导 ≥1 次 FLR/SBR 级复位"（武装于 11-20 23:21 引导，12:54/13:31 的 wedge 复位清零后无人再武装）；多轮存活由每轮恢复后的 `VERDICT`（FEAT 读数）自证。
4. GA102 桌面由此成为 **GA10x 首个 Linux 持久存活数据点**（此前 Linux 侧仅 TU102，[#103](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/103)）。运维含义：Linux 挂死走驱动级复位恢复不丢解锁、不必重跑 U 盘——rail 在位即可。

文档落点：FAQ [机制盘点](https://github.com/physicsdolphin/nv-uefi-unlocker/blob/main/docs/faq.md#relock)（实验段 + 第 3 行更名）与 [#148 重定性注记](https://github.com/physicsdolphin/nv-uefi-unlocker/blob/main/docs/faq.md#relock-148)。

日志：`recovery-session.log`（控制台原始捕获，含失败轮与 wedge 签名、双卡完整 diag dump，未删节）。
