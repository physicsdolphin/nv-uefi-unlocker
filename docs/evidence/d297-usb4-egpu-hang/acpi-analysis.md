# D#297 深度调研（2026-11-26）：BME 证伪后的机制候选——ACPI 侧 iTBT 管理路径反编译分析

前提事实（报告者 BME 消融轮 + 此前细化轮）：

1. **桥 BME 写被证伪**：`tx-no-bridge-master` 生效（`BridgeMaster::enable skipped` 行在）仍挂，pre-EBS 不变（ebs-watch 仍无 ENTER）。
2. 挂点随交棒方式漂移（reconnect pass / LoadImage / 固件 logo），**手术后到挂死之间机器全程活**（MEMMAP/BootOrder/SFS 全正常）。
3. Esc 中止（无手术）= 同样的 reconnect pass + 同批句柄 = **不挂**。
4. 报告者自己点明：GPU 自身 COMMAND 的 BME 位（`COMMAND IO|MEM|BME exercised`）仍在做——**与桥 BME 是两个不同的位，本轮消融只剥了后者**。

## ACPI 反编译发现（iasl，ssdt-0d = iTBT/TCSS 主体，40KB）

### A. iTBT 的 ACPI 侧管理栈全在这张表里

| 对象 | 含义 |
|---|---|
| `TDM0/TDM1`（_ADR 0D.2/0D.3） | "ICL TBT DMA0/1 controller"——iTBT DMA 控制器，带完整 `_PS0/_PS3/_S0W` |
| `TRP0..TRP3`（动态 _ADR，默认 07:00/…） | TBT Root Ports，各带 `TUID`、D3Cold exit/entry（`D3CX/D3CE`） |
| `PowerResource TBT0/TBT1` | D3Cold 电源资源：`_ON`→`TBON(n)`→`TG0N/TG1N`；`_OFF`→`TBOF` |
| `TXDC`（0D.1，xDCI） | USB-C PD 控制器，`XDMM` 1MB MMIO 区 + PMU 电源态 DSM |
| `PXSX`（隧道下设备位）×4 组 | `_PR0/_PR3` 返回 `{D3C, TBT0/TBT1}`——**隧道下游设备（含我们的 GPU）在 ACPI 电源模型里挂在 TBT 电源资源后面** |

### B. 关键机制一：`_PR0` 电源依赖链（挂死的第一候选）

`PXSX._PR0`（隧道下设备的 D0 电源依赖）= `{\_SB.PC00.D3C, \_SB.PC00.TBT0}`。语义：**任何使隧道下设备进/出 D0 的固件动作（ConnectController 的驱动绑定、StartImage 里子镜像的设备初始化、BDS 枚举）都要先评估/打开 TBT0 电源资源 → `TBT0._ON` → `TBON(0) → TG0N()`**。

`TG0N` 的完整动作序列（ssdt-0d:444-488）：

```
TG0N: 读 TDM0.VDID（过隧道 PCIe 读）
  → TDM0.D3CX()：DMA 控制器出 D3Cold（写 DPME 寄存器组）
  → TRP0/TRP1.D3CX()：root ports 出 D3Cold
  → 若 ALCT==1 && CTP0==1：TDM0.CNTP()   ← 拓扑重连命令
  → Sleep(ITRE)
```

而 `CNTP`（2301 行）内部是**双轮询**：force-power 等 `INFR`（1ms 步进至 ITFP 超时）+ `ITMB(0x3E)` 邮箱命令后 `WTBS` 等 `TB2P` 应答位。**AML 轮询无全局超时的部分存在**（WTBS 只有计数值上限；TB2P 永不清 → While 到计数耗尽；但 D3CX 内部的等待若固件侧用 Stall/Sleep 循环且条件永假 → 死循环在 SMM/AML 解释器里）。

**假说 H-PR0**：手术后隧道内 config 状态（GPU COMMAND 位、交换机状态）与固件 CM 的内部簿记失同步；下一个触发 `_PR0` 评估的固件动作（= 我们观察到的三处挂点全属于此类）走进 `TG0N` 的 D3Cold-exit 序列，卡在轮询上——**AML 解释器在 boot services 上下文里自旋，机器表现为活但永不前进**（CAD 活——中断还开着，与观察吻合）。

### C. 关键机制二：`SCME`（CM 模式选择）分叉

`TBT0._ON` 有守卫：`If (TDM0.SCME == Zero) { TBON(0) }`。`SCME` 由 DOCM（DSDT:46364 一带）按 OS `_OSC` 协商设置——**FW CM 模式（本机实况）下 SCME 路径不同**。注意 `TDM0._PS0`（1754 行）在 `SCME == One` 时也调 `TBON(DUID)`——**`_PS0`（D0 电源状态切换）同样能走进整条 TG0N 链**。

### D. 关键机制三：DMA remap（TBT DMA _DSM）

`TDM0` 的 `_DSM`（TBT DMA DUID 系列）管理 PCIe→内存地址重映射表的编程。**我们的 DMA 分配（~100MB radix+五缓冲，AllocateBuffer→EfiBootServicesData）在 TBT DMA remap 视角是新的映射需求**；手术后固件任何 `_DSM(Func DUID)` 重编程若发现映射表与设备实际状态不一致 → 同样可能锁在固件内部等待。

## 修正后的候选排序（BME 出局后）

| 候选 | 机制 | 现有证据拟合 | 判别实验 |
|---|---|---|---|
| **H1：GPU 自身 COMMAND 写（IO/MEM/BME set）** | 穿隧道的 endpoint config 写让固件 CM 簿记失同步（报告者点明的下一个单变量） | ✅ 三挂点全吻合（_PR0/TG0N 触发类）；BME 消融只剥了桥位 | **claim-no-command diag 臂**：detach+claim 但跳过 `attributes_enable(0x0007)`+raw COMMAND 写（保留断连+独占） |
| **H2：DisconnectController 本身** | 断连让 TRP/PXSX 进 ACPI 待变电网；重连路径走 D3Cold-exit 全链 | ✅ 同上；且解释“为什么 Esc 轮（无断连）不挂” | **disconnect-only diag 臂**：只做 detach+reconnect，无 claim 无任何写 |
| **H3：DMA 分配/释放（~100MB churn）** | TBT DMA remap 表被 AllocateBuffer/FreePool 大块搅动 | ⚠️ 中：LoadImage/BDS 也是分配密集；但 Esc 轮无分配也不挂只说明非充分 | **dma=false diag 臂**：手术全做、跳过分配 |
| H4：MSI/中断态（手术后 GPU 中断使能态与固件预期不符） | 弱——agitate 未动 MSI | 低 | 观察类，优先级末 |

H1/H2 都预测同一件事：**挂死发生在"下一次 D3Cold-exit/_PR0 评估"**，与三处挂点一致；区别在毒源（COMMAND 写 vs 断连事件本身）。

## 对“联想+雷电”家族的统一解释（更新版）

`PXSX._PR0 = {D3C, TBT0/TBT1}` 是集成 TCSS 平台的 ACPI 通用件（Intel 参考代码 ICL 世代沿用至 Arrow Lake）。凡是：
- 卡在隧道上 → 毒直接落在 TBT 电源依赖链管理的设备上 → pre-EBS 挂（TG0N 链，D#297）；
- 卡在内部（#239/#242）→ 隧道下没有我们的设备，但 **EBS 扇出里固件会做平台电源态收尾（含 TBT 电源资源评估）** → 扇出尾段挂（黄无绿）；
- 离散 JHL（G614JV）→ 无这套 iTBT ACPI 电源依赖链（NHI 独立芯片、无 PXSX._PR0→TBT0 挂钩）→ 免疫。

三个家族现象一个 ACPI 结构解释。**判别升级**：#239/#242 若也抓 ACPI dump，看 ssdt 里有无 `TG0N/CNTP/PXSX._PR0→TBT0` 同款结构（预测：#242 有；#239 待验）。

## 下一步（等 owner 定夺，不回复）

1. 三个 diag 臂设计（claim-no-command / disconnect-only / dma=false）——优先 claim-no-command（H1 是报告者自己点名的位）。
2. 请报告者（下一轮沟通时）：①readonly 成功轮 vs agitate 轮的 CHAIN-MEMMAP 描述符对比（H3 侧证）；②ssdt-12 的 NHI 部分与 ssdt-0e/14 电源表可再反编译深挖 `_OFF` 侧。
3. 家族数据：#239/#242 的 ACPI dump 同款检查。

## 附：本轮产物

- 反编译六表（dsdt/ssdt-00/0d/12/13/16 .dsl）在本目录 acpi/
- 报告者附件四件已归档（loader×2、ebs-watch、acpi-dump.zip）
