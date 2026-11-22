# 健康基线轮（TU106 1f07 / Z170 AMI / GSP 模式，v2.1.12-rc5）——ebs-watch 真机首验

- 日期：2026-11-22；stick：H 盘（armed + ultrafast + quiet + gop-reorder + `\tx-ebs-watch.txt`，no-console 已去）
- 结果：**解锁全绿**（HANDOFF-OUT tu10x:unlock=true、canary intact、D0）→ **正常进 Windows、系统内解锁正常**
- 证据：loader-round1.log（本目录）、ebs-watch-round1.log

## ebs-watch 健康基线（真机 AMI 固件首验）

`ebs-watch.log` 全文两条 + 屏幕色块（黄+绿，无蓝）：

```
EBS-WATCH installed (gBS patched: ExitBootServices + GetMemoryMap wrapped, header CRC recomputed)
EBS-ENTER n=1 key=0x4868 gmm=22/11 lastkey=0x4868 distinct=11
```

- **健康 pass 形态 = ENTER → NOTIFY（黄）→ RETURN ok（绿），零 INVALID_PARAMETER、单次 ENTER**——三台报告者机器此后对表用的就是这条。
- FB 捕获 = 重建后 native GOP 的 3840x2160@0xc0000000 bgr ✓（native-GOP 重建后安装的仪器抓到的是真显示模式）。

## 两个真机发现

1. **这台 AMI（Z170 代 edk2 衍生）不发 BeforeEBS 事件组**：BEFORE+EBS 都 create_event_ex 成功（loader.log `checkpoints=BEFORE+EBS armed`），但 before_notify 未执行（无蓝行、无 BEFORE 文件行），而 NOTIFY/RETURN 正常。OVMF（edk2-stable202408）有发。**判读协议修正：该 AMI 构建上蓝行缺失≠诊断信号**；ENTER→NOTIFY→RETURN 三点仍是全诊断面。报告者三机固件阵营：Biostar H510M=AMI、MSI X399=AMI（PciBus GUID 同族实证）——蓝缺失可同理视为不诊断；**Lenovo #242=Insyde H2O**（更新包带 H2OFFT-W.exe、.fd dump 全 Insyde/H2O 字符串）——Insyde 是否发 BeforeEBS 未测，蓝缺失在该机=不定而非预期。
2. **预算内自扰没有发生**：ENTER 行的 Close/reopen 在真机上**没有**触发 INVALID_PARAMETER 重试（OVMF selftest 制造过一次）。机理推测：edk2 的 pool 实现按类型持页——本次引导早已开过 BS-data pool 页，watch 提交的小分配落在同页内不产生新描述符 → MapKey 不变 → winload 的 key 干净。OVMF selftest 的 4096B 整页 alloc 才动图。预算按最坏情况设计，真机表现好于设计。

## 其它校验点（全过）

- CHAIN-BOUNDARY：`LoadImage ok` + `StartImage enter` 落盘（loader.log 最后两行）✓ rc4 盲区修复真机生效。
- gmm=22/11 distinct=11：22 次 GetMemoryMap/11 次错误（winload 的 BUFFER_TOO_SMALL 增长舞步）+ 11 个 distinct key——与 TX-ASSET-CACHE 释放 12 项 + 日志提交的图变更量吻合，正常。
- 手术全链真机走查：detach 6→4、EXCLUSIVE claim、五 HIGH-band DMA、radix 9070 页、booter halt、选择器写读回、native-GOP Start=SUCCESS→3840x2160 verified→console rebound。
