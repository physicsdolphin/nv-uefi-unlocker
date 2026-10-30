# 回复草稿（2026-10-31 取证诊断轮）——待维护者批准后发布

> 状态：**草稿，未发布**。批准后由代理用 gh 逐条发出。诊断件随每条评论附件携带（BOOTX64.EFI.tu10x-forensics-diag + SHA256）。
> **技术底稿**：所有回复的技术论据集中在 `docs/tu10x-booter-0x91-matrix.md` 的"机理问答"与"时序冲突全表"两节（含内核端一手复核 `kgspExecuteHsFalcon_TU102`/`kgspExecuteFwsec_TU102`）+ `docs/tu10x-c-parity-diff.md` B0/B0.1。
> 配套文档：`docs/tu10x-booter-0x91-matrix.md`（成败总表）、`docs/tu10x-c-parity-diff.md`（差分与 marker 优先级）、`docs/tu10x-booter-nsstub-disasm.md`（NS 桩反汇编与 mailbox/EXCI 字典）、`docs/tu10x-forensics-diag-protocol.md`（协议一页纸）。

---

## Issue #17（@OBC100）

@OBC100 三方差分做完了，这轮把结论和下一个实验一起给你。

**根因侧的三个新事实**（详见 [总表](…) / [差分表](…) / [NS 桩反汇编](…)）：

1. **所有成功实现都等 GSP HALT，唯独我们的 tu10x 路径不等**。nlzy/turing-unlock（Linux）、你的 PR #49、travis 的 MSI 卡 guarded 构建——三个成功链，加上我们自己场验证的 GA10x/Ada 路径——全部在 FWSEC 后等 GSP CPUCTL 进入 HALT（+精确 WPR2 lo/hi）才点火 booter 并释放 staging；而 main 的 tu10x 路径在 WPR2 寄存器刚匹配（你机器上实测 3ms）就立即释放 28MiB staging 并继续分配 84MiB radix——释放的页可能被复用、FWSEC 可能还在收尾。你 PR 文档里那句"避免把 WPR2 刚出现当作固件完成或**提前复用缓冲区**"，正是三方交集里唯一带明确机制的差分。
2. **NS 桩反汇编定案 mailbox 字典**：桩在 `lcall 0x100` 前把 **0x31** 写进 mailbox0（"正在进入安全段"标记）。你历轮终值都是 0x91 而非 0x31 → **安全段确实运行了并写了拒收码**（0x91 = wpr_desc_validate 族）；EXCI=0x00900100 是拒收后的自毁残影（权威位域：EXCAUSE[24:20]=9=INV_INS，此前"0x10=DMEM_MISS_INS"的解读有误），且它在成功链里同样出现（travis 三次 attempt 全有）——不是独立判据。
3. **安全段是 AES 密文**（熵 7.955，NS 桩 5.09 明文）——你们 C 链一直能跑通它，所以"解密坏"不是你这台的失败点，0x91 拒收与"WPR2/MMU 未收尾就点火"的衔接是当前头号机制候选。

**请你跑一轮（附件 EFI，基线 v2.1.6 + 取证诊断）**：
- **R1**：冷启动，U 盘根目录放空文件 `\tx-fwsec-gsp-halt.txt` → FWSEC 完成判据改为"精确 WPR2 + GSP HALT（10s）"，HALT 前不点火、不释放 staging。若你这台转绿 → 根因唯一归因闭环。
- 若 R1 阴性 → **R2**：换 `\tx-fwsec-hold-dma.txt`（只延迟释放，不等待）分解"页复用 vs 提前点火"两轴。
- 每轮完整断电冷启动、回传完整 loader.log（新增 `entry WPR2 [fwsec=ran/skipped]`、`BOOTER-PLACE`、`BOOTER-AUDIT … MATCH/MISMATCH/DENIED`、`EXCI decode`、`mailbox0 verdict` 行都会自动记录）。协议全文：[diag-protocol](…)。

## Issue #62（@chenc222、@ProxVirtualZone）

（核心同上三点，压缩为一段 + 个性化感谢）

@chenc222 你的 D/E/F 轮是整个调查里唯一让异常类动过的轴，现在完全对上了：**终值 0x31 = NS 桩的"进安全段"标记未被覆盖 = 安全段没跑**（桩反汇编实证），D/E 轮搬址后 EXCAUSE 9→0x0a（INV→MISS）与"空页取指"逐位自洽，F 轮 DMEM 完全未动也自洽。请跑 R1（`\tx-fwsec-gsp-halt.txt`，附件诊断件，冷启动）；阴性再 R2（`\tx-fwsec-hold-dma.txt`）。
@ProxVirtualZone 你的 0x0B 终值同样落在"安全段跑了但早期拒收"桶里——请同样跑 R1/R2。你们两位的读数（1e81 同 ID 成败分裂）是"分野在机器不在卡"的直接实证，已进总表。

## Issue #57（@hackerwxsk）

（核心三点 + 24G 特定说明）

@hackerwxsk v2.1.6 起 heap 已按驱动公式化（8+align1MiB(96K×GiB)+96，钳位 [88,280]MiB），24G=107MiB 已内置，**二进制补丁不再需要**。你们三轮的 0x11/0x91 切换与 EXCI 残影已并入字典。请用附件诊断件跑 R1（`\tx-fwsec-gsp-halt.txt`）→ R2；注意你们是链 shimx64 的 Linux 目标机，协议相同（彻底断电冷启动）。另外 nlzy/turing-unlock 内核链的 FWSEC 完成判据本身就是 `poll(GSP+0x100, HALT, 5s)`——Linux 成功侧从未有过"不等 HALT"的路径，这与 EFI 侧差分互为印证。

## PR #49（@OBC100）

按先前说明保持最小 diff 路线，PR 暂 hold（不关闭）。你的移植文档与实测是本轮差分表的三方证据之一——"FWSEC 精确 WPR2 + GSP HALT + 不提前复用缓冲"现在已是头号嫌疑轴。诊断分支已把该成分以 `\tx-fwsec-gsp-halt.txt` marker 复刻进主路径（默认关）。若你在自己卡上跑 R1 转绿，你的多变量移植里哪一项必要将有唯一归因——那也会是这个 PR 的最终归宿（提取必要项进 main，其余归档）。感谢这套干净的工作。

## PR #64（@travis-mcdevitt）

字段修复（TokenEntries u8 / NPDE 0x100-0x101 / V2 +28/+40）与当前 0x91 攻坚线正交，质量看起来扎实（89 项 host 测试）。合并窗口排在 TU106 实机回归之后——它动 FWSEC 解析路径，且与 #49 的 `data_off` 断言、#43/#44 的解析器搬迁有交互，需要一批回归。你的 guarded 证据包与 source-overlay 是这轮差分表的直接输入（B1 表已逐项引用），谢谢。

## PR #65（@travis-mcdevitt）

PLM 全 1 分类与 `EClaimUnreadable`（不带 emergency FLR）的语义我们认同，0x823818 措辞问题也已在诊断文档里同向修正（NS 桩文档 C5：该行采样的是 PL0 锁定簇，不能作 Turing 判定）。PR 保持 open，与 #64 同批回归窗口处理。

## PR #67（@Catnip202X）

D/E/F 轮是全场唯一动过异常类的一手数据，0x31 字典（NS 桩反汇编实证）直接证实了你"换载荷不可能改变崩溃"的可证伪预测与"入口级失败"的判读。`sec_placement` 日志与两个开关已在诊断分支原生实现（`loader/src/tx_booter.rs`，注释注明出处与你的 PR）。处置建议：功能已被采纳，本 PR 可 close as superseded——或你愿意的话保留到下个合并窗口走正式 review，都行。

## Reddit（经 @SkYwAlKeR973019 转问，或维护者账号回复）

英文短问：Great numbers! Which build did the two 1E07 cards unlock with (release version, and was it the default path or any marker files)? Trying to pin down why the same ID fails on some machines (issue #17/#62 on GitHub) — the readout format looks like ssdiag, if you still have the EFI-side loader.log from that boot please attach it too.
