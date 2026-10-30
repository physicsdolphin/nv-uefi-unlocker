# TU10x 0x91 取证诊断轮协议（diag/tu10x-forensics，2026-10-31）

> 诊断件：`release/BOOTX64.EFI.tu10x-forensics-diag`（分支 `diag/tu10x-forensics` 构建，TX 门控）。基线：v2.1.6 main + 本分支诊断与消融 marker。**普通用户无需任何 marker 即可运行**——所有常开诊断自动记录；marker 仅给失败案例的消融轮。
>
> 背景：[tu10x-booter-0x91-matrix.md](tu10x-booter-0x91-matrix.md)（成败总表）、[tu10x-c-parity-diff.md](tu10x-c-parity-diff.md)（根因差分）、[tu10x-booter-nsstub-disasm.md](tu10x-booter-nsstub-disasm.md)（mailbox/EXCI 字典）。

## 每次运行自动新增的日志行（回收清单）

| 行前缀 | 内容 | 用途 |
|---|---|---|
| `TX-tu10x: entry WPR2 … [fwsec=ran/skipped]` | 入口 WPR2 与 FWSEC 是否由 loader 点火 | 成败两组的冷/热分布（矩阵 A4-5） |
| `FWSEC-TU10X: *** WPR2 UP … after Nms ***` | START→WPR2 出现耗时 | FWSEC 速度分档 |
| `BOOTER-PLACE: …` | NS/安全段 dest/tag、BOOTVEC、开关状态 | 放置维（#67 谱系） |
| `BOOTER-AUDIT: ns/sec/dmem readback MATCH/MISMATCH/DENIED (fnv …)` | **START 前全段读回**（安全段为密文，读回即所写密文） | 区分"写入丢失"与"取指侧差异" |
| `BOOTER: EXCI decode: EXPC=… EXCAUSE=… (INV_INS…)` | 权威位域解码（[24:20]，9=INV_INS） | 替代旧 0x10 误读 |
| `BOOTER: mailbox0 verdict: 0x31/0x91/…` | NS 桩字典（0x31=安全段没跑；0x91=跑了且拒收） | 失败分级 |
| `GSP-HALT gate: …halted=true after Nms` | （marker 开启时）FWSEC 收尾确认 | 头号消融轴 |

## 失败案例消融轮（每轮一次冷启动，一个变量）

按 [tu10x-c-parity-diff.md](tu10x-c-parity-diff.md) B4 优先级，U 盘根目录放 marker 文件（空文件即可，除注明内容外）：

1. **R1（头号假设）**：`\tx-fwsec-gsp-halt.txt` —— FWSEC 完成判据改为精确 WPR2（lo+hi）**且 GSP HALT**（10s 预算），HALT 前不点火、不释放 staging。三个成功实现（nlzy Linux/PR #49/travis guarded）全部使用此判据。
2. **R2（分解轴）**：`\tx-fwsec-hold-dma.txt` —— 只延迟释放 staging 到重启，不加 HALT 等待。R1 成 R2 败→"提前点火"为主因；R1/R2 都成→"页复用"即足够。
3. **R3（弱嫌疑，仅 R1/R2 阴性时）**：`\tx-no-pm-preflight.txt`。（~~`\tx-no-bridge-master.txt`~~ **已撤销 2026-10-31**：BridgeMaster 是 PR #5/issue #1 场验证的必要修复——上游桥 BME 未开=booter 0x89 族；跳过要么 no-op 要么引入 0x89，不判别 0x91。请勿使用。）
4. **入口维（可选，对应 #67 D/E 轮复现）**：`\tx-sec-base.txt`（内容如 `0x200`，256 对齐）搬安全段 dest+tag；`\tx-bootvec-sec.txt` BOOTVEC 直入安全段。

**纪律**：每轮完整冷启动（彻底断电）；同会话失败绝不重试；回收完整 `loader.log`（新诊断行全在）。R1 预期：若硬核案例（#17/#62/#57/ProxVirtualZone）任一转绿→根因闭环，B0 差分表收束为单变量。

## 安全边界

- 所有消融 marker 默认关闭；无 marker 行为=main v2.1.6 + 只读诊断。
- `\tx-fwsec-hold-dma.txt` 故意保留 ~28MiB staging 至重启（无 Drop、无持久固件改动）。
- 安全段读回为读操作；`DENIED` 是有效观测不判失败。
