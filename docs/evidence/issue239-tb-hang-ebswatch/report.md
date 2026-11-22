# #239 雷电挂死 A/B —— ebs-watch 真机首次把家族挂死钉到扇出尾段（NOTIFY→RETURN）

- 日期：2026-11-22；报告者 rc5 + `\tx-ebs-watch.txt` + `\tx-reconnect-legacy.txt`，TB-on/TB-off 各一轮
- 结果：**软锁家族第一个被钉到具体阶段的真机挂死**——最后一次 EBS 调用通过 TerminateMemoryMap 进入扇出（黄=NOTIFY）后**永不返回**（无绿），且仅 TB-on 出现；TB-off 同路径完整返回（绿）正常引导
- 证据：failed-loader.log / failed-ebs-watch.log / success-loader.log / success-ebs-watch.log / hang-photo.jpg（本目录）

## A/B 结构：两轮在进入第二次 EBS 调用前逐字节等价

| 项 | failed（TB-on） | success（TB-off） |
|---|---|---|
| 事务 | SS0=0x88888888/SS1=0x8、canary intact、HANDOFF-OUT tu10x:unlock=true | 同左（逐项相同） |
| 重连 | pathless GOP → non-recursive ConnectController ok | 同左 |
| 日志终点 | CHAIN-BOUNDARY: StartImage enter | 同左 |
| ebs-watch 文件 | install + ENTER n=1 (gmm=24/12 distinct=12) + RETURN INVALID_PARAMETER ×1 | **逐字节等价** |
| 色块 | **黄有、蓝无、绿无**（照片核验） | （未见照片，预期黄+绿） |

文件通道在 enter#2 后按预算静默（enter n≥2 不写文件）——判别交给色块，设计如此。

## 判读

1. **单次 INVALID_PARAMETER = 预算内自扰**（观测器自己的记账写入 stale 了 winload 的 MapKey），两轮各恰好一次——设计与实机完全一致。
2. **挂死点 = 扇出尾段**：黄=该次调用 TerminateMemoryMap 已过（进入通知扇出中点）；无绿=该次调用永不返回。edk2 尾段=MemoryProtection 回调/debug-timer 关停/CPU 中断关停/ST 清理/ZeroMem(gBS) 区间——**只在 TB-on 出现**。
3. **DeepSeek 的 MapKey 机理被证伪**："雷电后台 PCIe 活动让 MapKey 失效、反复重试"——两轮重试结构完全相同（各一次=我们的预算写入），gmm 计数器逐位相同，不存在第二次 MapKey 失效，重试循环不存在。雷电源的楔子在扇出**内部**，不在内存图。
4. **蓝行缺失**：Insyde 与 Z170 AMI 都不发 BeforeEBS 事件组——真机判读表坍缩为 ENTER→NOTIFY→RETURN 三点（蓝行保留给 OVMF/QEMU 自检）。

## 里程碑

- 软锁家族（#242/#278/#279/#239）诊断第一次拿到**真机挂死的阶段定位**；
- 第一个 **actor-resolved** 实例：摘掉雷电 → 扇出尾段走完——actor 模型（交接扇出 × 活态 GPU）拿到机制级实例；
- ebs-watch 仪器闭环自证：健康机基线（黄→绿）→ 真挂死定位（黄无绿）。

## 下一轮（单变量，一轮定乾坤）

`\tx-agitate-only.txt` + `\tx-ebs-watch.txt` + TB ON：区分"活态解锁 GPU × 雷电" vs "loader 手术 × 雷电"——挂死（黄无绿）=手术即足、解锁写入非必要；干净引导（黄→绿）=解锁态×雷电为楔子。顺带补 success 轮照片（预期黄+绿）完成视觉 A/B。
