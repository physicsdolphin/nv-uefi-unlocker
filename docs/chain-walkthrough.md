# 链条分步对照：我们 / TOTORONG / Linux kernel（gpu-unlock-ada）

> 勘误（2026-10-28）：本文是 2026-10-26 的过程记录。顶部对 28e1 的 IOMMU 归因已被后续桥 BME 实验推翻，见[根因报告](bridge-bme-root-cause.md)。当前家族判据见 [FAQ](faq.md#mechanism)，实现限制见[安全与边界](faq.md#safety)。

日期：2026-10-26。依据：gpu-unlock-ada 全家桶（unlock_610.c + 15 卡报告）、
TOTORONG AD103_win_unlocker 源码、我们 28e0 实测日志（至 EFI c0fce9ac）。

> **状态更新（2026-10-26 深夜，v0.1.3 之后）**：本文写作时的第 11 步卡点
> **已解决并硅闭环**。解决路径 = portal 权威编码（SEC2 装载/启动序列修复，
> scrubber 与 booter 同一修复受益）+ **内核同款 scrubber 跑通 BSI handoff
> （SCRATCH_15 1→3，SEC2/GSP 域解锁）**；此后 CPUCTL START 正常生效
> （start-probe: pre=0x50 → started → HALT）。M3 回放（MAILBOX0=0，28e0
> 硅验证；28e1 因平台固件 IOMMU 阻断在 booter，见 [桥 BME 根因](faq.md#bridge-bme)）与
> M4 payload（marker+四写：28e0 1.65×、28a1 1.7~1.9×）均已硅验证。
> §二–§四保留为卡点过程记录，§五行动清单已被上述结果取代。

## 一、细分手步骤（15 步）

| # | 步骤 | 我们(28e0 EFI) | TOTORONG(AD103 EFI) | Linux(15+卡 kmod) |
|---|---|---|---|---|
| 1 | 枚举/认领：GOP+驱动断开、独占、IO/MEM/BME | ✅ | ✅ | ✅ |
| 2 | 入口门：chip/WPR2_hi=0/PLM=8f/SS 合理 | ✅ | ✅（更严，逐板钉值） | ✅ |
| 3 | VBIOS PROM 读（BAR0+0x300000, 1MiB） | ✅ | ✅（+SHA 钉板） | ✅ |
| 4 | FWSEC 抽取/解析/签名槽/FRTS 补丁 | ✅ | ✅ | ✅ |
| 5 | 布局计算（fb/vga/lock→WPR 五段） | ✅（修过 image_size 输入） | ✅ | ✅ |
| 6 | **FWSEC 点火**（HS@GSP）：装载+签名+BROM 参数+启动+halt→WPR2 立起 | ✅ **portal 版复现 2 次** | ✅（社区同码） | ✅ |
| 7 | scrubber（仅内核驱动流程有；社区解锁链无此步） | ✅ **内核同款 scrubber 跑通**：BSI handoff 1→3、SEC2/GSP 域解锁（v0.1.3 起为固定步骤——与社区链的差异项，也是第 11 步的前提） | 无此步 | 无此步（kmod 链跳过） |
| 8 | GFW 状态评估 | ✅ 读；回放中无等待必要 | ✅ | ✅ |
| 9 | booter 资产：BTL1 解析+签名槽补丁 | ✅（56832/768, fuse v1→slot0） | ✅ | ✅ |
| 10 | GSP-RM 装载准备：radix3（84MB/20606 页）+WprMeta+签名 DMA | ✅ 全部发布成功 | ✅ | ✅ |
| 11 | **booter 点火**（HS@SEC2）：装载+邮箱=WprMeta+启动 | ✅ **已解决**（原"START 写被吞"→ portal 编码 + scrubber 解锁域后正常生效；回放 MAILBOX0=0 于 28e0 硅验证） | ⚠️ 同码尝试，自述 "hardware success unproven" | ✅ 全部卡种（含 4×AD107） |
| 12 | 执行判据：payload=四 PLM+marker / 回放=mailbox0+WPR2 | ✅ 双模式硅验证（回放=mailbox0=0+WPR2；payload=四 PLM+marker `0x67b00001`） | 设计同 | ✅ |
| 13 | 签名还原（payload 模式） | ✅ 固定步骤，FLR 前还原 | 有 | ✅ |
| 14 | FLR（TOTORONG 工艺版） | ✅（flr_once/flr_phase 已硬化，PMCSR 钉 D0） | ✅ | ✅ |
| 15 | 交棒（GOP 恢复/链回 OS 引导） | ✅ HANDOFF-OUT 出口行 + **GOP 重连**（`reconnect_all`，58b258f 采纳）后链回 | ✅（预载+StartImage） | ✅（initramfs 后交内核） |

## 二、卡点精确刻画（第 11 步）——已解决（见顶部状态更新）

本轮探针（c0fce9ac）三连读：`pre=0x10 imm=0x10 +5ms=0x10`，alias(+0x130) 重试同
样 `0x10` —— **SEC2 CPUCTL 的 START 写被静默丢弃**。对照事实：

- 同一写法在 **GSP** 上生效（FWSEC 两次点火复现）；
- SEC2 的其它寄存器写入**全部生效**：MAILBOX0（读回=我们写的 meta 地址）、
  FBIF_CTL/TRANSCFG/DMACTL（读回=我们写的值）、IMEM/DMEM portal（DMEM 读回=
  逐字节匹配）；
- SEC2 CPUCTL 的**读**也门控（冷态 badf5620，GSP 同寄存器读正常 0x10）；
- 冷态 BSI_SCRATCH15 handoff=1（非 DONE=3）。

即：SEC2 域不是整体门控，是**精确到 CPUCTL（启动权）的读写门**。

## 三、卡点候选解释（按可检验性排序）——事后判定：A 的 handoff 门控成分成立
（scrubber 完成 handoff 后域解锁），但 START 写本身随 portal 编码修复即恢复，
B/C 的窗口/PLM 成分未再显现

A. **BSI handoff 状态机持有 SEC2 启动权**：handoff=1（非 DONE）期间 SEC2 拒绝
   被"非 heavy-secure 链路"启动。问题：healthy 卡上 kmod 阶段 handoff 是多少
   无人记录（社区链无 scrubber 代码、不打这个寄存器）。判别实验：在**同一台
   28e0** 上跑 Linux+gpu-unlock-ada，读 SCRATCH_15——若也是 1 而 SEC2 能启，
   A 死；若是 3，则我们的 GPU 处于 BSI 半引导态，问题前移到"为什么 ROM 链没
   跑完"。

B. **窗口差异**：kmod 在 ExitBootServices+内核 PCI 重枚举之后；我们在 DXE。
   判别实验同 A（Linux 侧对照即窗口对照）。

C. **SEC2 PLM 寄存器逐块差异**：读 SEC2 块的 PLM 寄存器族与 GSP 对应块比对
   （FEAT 块 PLM 可读先例 0xffffff8f）。若 SEC2 的 PLM 值异于 GSP，直接看到门。

D. SEC2 BROM 需要 scrubber DONE 才接受 HS boot——与 A 同族；但 scrubber 本身
   是 SEC2 HS boot，鸡生蛋，唯一解法是让 ROM 链自然完成（回 A）。

## 四、对"打真 payload"的顺序结论

payload 与 stock booter 只差签名槽字节，乘同一条第 11 步路径——第 11 步不通时
payload 实验无效。正确顺序：解决 11 → M3 回放完成（宪法 M3 已获确认）→ M4
payload 单独立项（AGENTS 1.7），届时判据三证（marker+HALT+PLM 翻转）+ 签名还原。

## 五、近期行动清单（已被结果取代——卡点在 EFI 侧直接解决，Linux 对照实验不再需要）

1. Linux 对照实验（A/B 一步判两个假设）：28e0 本机 Linux + gpu-unlock-ada，
   `dmesg`/模块日志 + 我们提供的 SCRATCH_15/SEC2_CPUCTL 读数脚本。
2. 源码复查：kernel_falcon.c 的 kflcnReset_IMPL/scrubber 路径有没有我们漏掉的
   "授权类"写（对比 falcon_reset 的每一步）。
3. SEC2 PLM 寄存器族扫描（候选 C），与 GSP 同族寄存器对照。
4. （若 1 显示 handoff=3）研究为何 UEFI 窗口 BSI 未完成——GOP/ROM 时机学。
