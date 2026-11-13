# gpu-unlock-ada 前人经验审计（2026-11-08）

对象：`reference/gpu-unlock-ada`（本地 clone，HEAD 含 `docs/ada-*-report.md`、
`windows/ad104-msi/` 实验性 UEFI 移植、177 场景注入测试）。

读法：先读文档（ada-port / testing / 六份逐卡报告 / latence-app），再逐行读
`app.c`、`checks.c`、`port.c`、`msi_release.c`、`boot_io.c`、`profile.c` 与三个
测试文件，最后把每条坑对照我们自己的 loader 找反制或缺口。

结论先行：**他们的交易体比我们薄（无 scrubber、无多卡、无 portal），载具的
"拒绝纪律"比我们厚**。厚的那部分是本轮的主要收获——但不是每一条都能直接搬，
有两条是他们自己踩出来的过度收紧（USB 图不变式、逐板钉死 BDF/ROM），照抄会
把我们的多卡工具变成单机工具。

---

## A. 我们还没遇到、应当避开的坑

按"对我们是否真危险"排序。每条给他们的现场证据、机制、我们的现状与处置。

### A1. 开放式协议图不变式会把正常 USB 热插拔判成故障（高，真实）

证据：`docs/ad104-windows-latence-app.md` 自述 "Earlier attempts exposed
normal periodic USB changes"；随后不得不为 USB 分支写专用豁免
（`msi_release.c` `UsbBaseline`/`UsbPointerPath`/`PointerAddition`/
`UsbConsoleChild`，注释直说 RUN0005 是"late USB mouse attaches to the
existing virtual console"）。测试里还专门复现了 165 → 152 的关系跃迁。

机制：整机在事务窗口内持续枚举/摘除 USB 设备（键鼠、hub、外接盘）。任何
"句柄集合前后相等"的朴素判据都会假阳性。

我们的现状：**只有诊断、没有判据**——`tx_detach.rs` `log_display_topology`
与 `log_openers` 只打印，从不比对。所以今天不会假阳性，但也检不出 console
栈没回来。

处置：若将来上不变式，先按他们的粒度建模，而不是先写"相等"：
- 允许消失/重连的只有"物理 USB 设备 + 其子边"；
- host controller 的 **driver/PCI 归属边必须恒在**（他们 `UsbBaseline`
  找不到唯一 host 就 `EFI_UNSUPPORTED`，宁可不做）；
- 新增子边必须满足：agent = 既有 splitter、controller = 原 console、
  `BY_CHILD_CONTROLLER`、`count==1`、owner 在非 GPU USB 路径上 `BY_DRIVER`、
  设备路径只含 `MSG_USB_DP` 节点且有深度上限（8）。

### A2. 过严的 config 漂移判据会误杀（中高）

证据：`msi_release.c:335-350` `ConfigStable` 的注释写着 "RUN0002 did not
identify PCI drift: do not assume it caused the stop"。最终允许集只有两个
status 位（`0x00090000`，且只允许从置位变清零），其余任何位变动即
`config-drift` 失败。

机制：重连 console / 重新 StartImage 自家 GOP 会合法改写 devctl、Cache Line
Size、BAR、VGA 位。全 256 字节比对必炸。

我们的现状：**基线拍了但从来没用**。`tx_detach.rs:174` `config_snapshot`、
`:366` 取 `baseline` 存进 `Claim`，grep 全仓无消费点；`tx.rs:1491` 的
`diag_handoff_pci` 只记录不留判语。等于零检出。

处置：分两步。先做**只读 diff + 允许集**（跟在 native-GOP 恢复之后、VGA
restore 之前），判语进日志不失败；等真实漂移面收敛了再谈硬门。允许集至少
要覆盖：command 的 MEM/IO、devctl、cache line、BAR0-5、VGA 0x3c 段、status
两位。**不要**照抄他们的两位允许集——我们多一条 native-GOP 重建路径。

### A3. 逐板钉死会把工具锁死在单卡单板单槽（中，已知取舍）

证据：`checks.c` 开头硬拒 `Segment!=0 || Bus!=1 || Device!=0 || Function!=0`，
外加 `Config[0] == GU_BOARD_ID`（device+subsystem 双钉，1462:5132）；
`profile.c` 钉 1 MiB PROM SHA-256、五个资产 SHA-256、精确 FWSEC 几何。

机制：他们的定位是"单板实验机"，所以钉到子系统 ID 和 BDF。用户换槽（
x16→x8）、多卡机、或 OEM 推一次 VBIOS 更新，全部硬停。

我们的现状：只钉 vendor:device（族表），多卡逐卡枚举，BDF 用
`\tx-select-bdf.txt` 可选钉。**比他们宽**，但也有真实缺口——见 A4。

处置：不抄 BDF 钉。抄"子系统 ID 可记录"，仅作日志与 `docs/cold-values.md`
基线，不作门。

### A4. PROM/FWSEC 逐板钉死与"卡自解析"之间的空档（中高，我们确实空白）

证据：同一仓库两条路线并存——
- Linux 模块**卡自解析**：`ada-2782-report.md` 明说 "It reads FWSEC-FRTS from
  the card's own PROM, and this ASUS ROM parsed without changes (four-image
  chain, FWSEC signature versions `0x3`, fuse version 1)"；
- Windows 移植**整镜像钉死**：`profile.c` 1 MiB PROM SHA-256
  `c81eb48f…` + 精确几何，换板即拒。

机制：同 device ID 不同板商的 ROM 在 FWSEC 几何、签名槽数、fuse 版本上可以
不同。V3 描述符的 `SigCount`/`SigVersions`/熔丝版本位图选槽依赖这些字段；
钉死一份 ROM 的实现换个板就选错槽。

我们的现状：`grep` 全仓 `loader/src` 无 SHA-256（只有 `assets/` 侧有），
没有 loader 内 PROM hash；Ada 资产来自钉版 610.43.03 的固定几何。
`docs/totorong-comparison.md` 候选 1 已经把"loader 内 no_std SHA-256 +
按机钉 PROM hash"列为欠账。

处置：**中道**——解析照旧、把 PROM 全镜像 hash 记进日志与冷值表；比对
"同 device 的已知基线表"：命中=绿，未命中=黄（继续，记 `prom-unknown`），
显式 `\tx-prom-strict.txt` 才转红。这样既拿到宪法 §1.5 的完整形态，又不会
被一次厂商 VBIOS 更新锁死。**注意退化风险**：V3 槽位必须来自 ROM 自身
字段，不能沿用钉版常量。

### A5. 驱动仍在绑定态就跑事务（中）

证据：`ada-28a0-report.md`——驱动链还绑着时模块 `-EBUSY` 拒绝、卡零改动；
操作者卸载 `nvidia*` 后才成功；成功后还跑了卸载/重载循环。

机制：绑定态下驱动持有 BAR 与设备上下文，事务写会被覆盖，读回是驱动的值。

我们的现状：UEFI 阶段天然无 OS 驱动，但**有固件驱动**（GOP、
ConSplitter、厂商 DXE）。`claim_guards` 有 `EBusy`，`detach_and_claim` 走
staged disconnect，但没有"断不干净就拒绝"的判据——断不掉的 opener 只会被
`log_openers` 记下来。

处置：抄他们的**拒绝而非硬抢**。detach 后复查 PciIo opener 集合，仍有
`BY_DRIVER` 且非我们自己的，记 `claim-openers` 并返回 `EBusy`；
`\tx-force-claim.txt` 放行。

### A6. BME 置位=拒绝，不是修复（中，与我们相反，值得记录）

证据：`checks.c` `(Command & BUS_MASTER) != 0` 直接 return FALSE，不在
准入里清。他们的立场：上电就有 BME，说明有别的东西在驱动这条链路。

我们的现状：**相反**——我们把 BME 当可修复项：claim 时连同 IO/MEM 一起
置上（`attributes_enable(0x7)`），FLR 前清，桥上的置位由 `BridgeMaster`
guard 在返回时还原自己置的那些位。这是 issue #1 的 0x89 根因（上游桥 BME
没开导致 Master Abort）推着我们走的方向，实机已验证，不打算改。

处置：保留我们的做法，但在日志里显式打"入口 BME=1"——若未来出现一批
入口 BME 脏的板，这是第一现场证据。**不要**改成硬拒。

### A7. 断连要放到回调消费者，而不只是控制器（中）

证据：`docs/ad104-windows-latence-app.md` "UEFI callbacks release physical
TextOut, GraphicsConsole and NVIDIA GOP consumers"；`msi_release.c`
`ProtectedHandle`/`ProtectedEdge` 把 console 的回调边列为必须保持的护栏，
`Removed` 分三段才允许摘。

机制：`DisconnectController` 只解驱动绑定，**不注销** `RegisterProtocolNotify`
/ TextOut / GraphicsConsole 这类回调消费者。留着它们，事务中途回调进已
FLR 的卡就是一次挂死。

我们的现状：`tx_detach.rs` 有 staged detach + `stale_pathless_gop` 处理，
但没枚举非控制器消费者。`#170` 的"console 写即挂"正是这一类。

处置：在 detach 前把 console 相关的 opener 全量记档（含非 BY_DRIVER 的
attributes），断连后复查；有残留就 `EBusy`（同 A5 一条通路解决）。

### A8. 日志关闭/StartImage 之后的盲区（中，双方都没解）

证据：`docs/ad104-windows-latence-app.md` 自述 "Final checks after USB close
and StartImage are unlogged"。他们把 final gate 放在 close 之后但选择不打
日志（避免 close 后再写盘），于是 final 结果只能靠操作者口述与事后基准。

我们的现状：`tx_booter.rs:1219` `probe_post_console` 打在 StartImage **之前**。
StartImage 之后到 Windows 接管之间，我们同样瞎。`#164` 的 POST-GOP 争议正
落在这个窗口。

处置：不在 loader 内解（关盘后再开是另一类风险）。**用系统内工具解**：
`ssdiag` 在 Windows 侧读 FEAT/SS 全表就是这条盲区的终点证据，文档里把
"loader 日志到 StartImage 为止，之后以 ssdiag 为准"写成显式边界。

### A9. 单次尝试：失败后不重试（低，已一致）

证据：`app.c` 的 `failed:` 路径先做收容（mutated/started/retained 才动作）
再 `Halt()`；重试的唯一合法形式是重启或断电后重来（`ada-2783-report.md`
的暖重启也是一次全新 boot）。

我们的现状：一致——失败 → `emergency_flr`（`tx.rs:1713`，`main.rs:515`
调用）→ 链回 Windows，同会话不重试。差异（他们停机、我们交还）已记录在
`docs/totorong-comparison.md`，保留。

### A10. DMA 生命周期：映射/解映射失败必须保留缓冲（低，已覆盖）

证据：`port.c` `GuDmaAllocate` 上限 128 MB、页对齐、`>4GB` 时要求
32 位 DMA 能力；`Unmap`/`FreeBuffer` 失败即置 retained，`GuDmaFree` 在
retained/pending/started-not-quiesced 时拒绝。测试里 `UNMAP_FAIL`/`MAP_FAIL`/
`MAP_PARTIAL` 三个注入场景。

我们的现状：`pci.rs:619` "Release is EXPLICIT (`release`) so the transaction's
unwind can verify nobody freed twice or leaked"，`DmaBuffer::release` 有读回；
`tx.rs` 解链前逐块释放并核账。等价，且我们的 84 MiB 单块 + radix 超过他们
128 MB 单分配上限，属于设计差异不是缺口。

处置：无。只在文档里记一句：**他们的 128 MB 上限来自单分配可验证性，不是
硬件限制**，我们分块。

### A11. 未知 PCI capability 即拒（低中，按需）

证据：`checks.c` `default: return FALSE`，注释 "A format we do not
understand cannot be checked for overlap/length"。

机制：保证 cap 链可完整遍历、无重叠、无环（`Seen` 位图 + `Occupied` 逐字节
占位 + `Steps` 上限），这样后面的 PM/PCIe 定位可信。

我们的现状：`pci.rs` 的 `pm_cap`/`pcie_cap` 遍历有迭代上限与 below-0x100
兜底（`:1330` 附近注释），但没有重叠检测，也不拒未知 cap。

处置：抄重叠/环检测进 cap 遍历（纯防御，不影响准入）；未知 cap 只记 warn，
不硬拒——消费卡上厂商 cap 不算罕见。

### A12. Secure Boot 变量显式拒绝（低，仍在候选）

证据：`app.c` 开篇读 `SecureBoot` 变量，非 0 直接拒。

我们的现状：loader 无 SecureBoot 读取（grep 无命中）。宪法要求实验机 SB 关
或自签入 db，但没有运行时判语；SB 开着时用户只会看到"卡住了"。

处置：`docs/totorong-comparison.md` 候选 2，本轮仍维持候选。SB 开着时
BOOTX64 根本不会被执行（除非签名），所以这条只在"记分=诊断"上有价值。

---

## B. 我们已经正确发现机制的坑（对照表）

同一条坑，我们已有反制。列出来是为了确认覆盖面，不是自夸。

| 坑 | 他们的处理 | 我们的反制 |
|---|---|---|
| badf 毒值混桶 | 单列 `0xbadf5620` 拒收（仅此一个） | `tx.rs:80` `inaccessible()` 全族分桶（5040/1100/20xx/5620），`claim_guards` 按桶给判语 |
| WPR2 已被本周期初始化 | `hi>>4` 拒绝，`-EBUSY` | `claim_guards` 的 EBUSY/EAlready 双码（`tx.rs:422`/`:466`），`main.rs:472` 把 EAlready 当 skip |
| BME 脏态 | 准入拒（A6） | claim `attributes_enable(0x7)` + `BridgeMaster` 置位/还原 + `log_dma_path` 前后取证 |
| FLR 后立刻读 BAR | `GuFlr`：等 pending 清零 → devctl 保留式置位 → **100 ms 全静默** → 等 config 身份复原 | `tx_booter.rs:1353` `flr_phase` 1:1 移植（`totorong-comparison` 本轮已采纳 1） |
| 恢复顺序错（先 command 后 BAR） | 解码关 → 高 BAR dword 先 → command `&~4` 最后 | 同上，已修 |
| 占位签名留给后续引导 | FLR 前恢复原厂签名 | `tb::booter_patch_stock` 写回 `stock_dma`，FLR 前复原并读回 |
| 失败后缓冲被释放/复用 | retained + `GuSafeToReturn` | 显式 `release` + `emergency_flr` 收容 |
| 84 MB 资产劈碎低位内存 | （他们无此问题，池分配不过 128 MB） | `chainload.rs:84` `handoff_free_assets` + `\tx-keep-assets.txt` 退出阀（#146） |
| 暖入口（resume/D3hot）当 POST 跑 | `Plan.ready`/stage 门 | `tx_detach.rs:198` `pm_preflight` 读 PMCSR 先行报暖 + `\tx-require-d0.txt` 转硬拒 |
| 多卡误伤 | 只支持单卡单槽（A3） | 逐卡枚举 + `path_belongs_to` 只断目标卡 + `\tx-select-bdf.txt` |
| 交棒 console 归属 | `GuMsiReleased` + `DisplayCheck` | `gop_native::recover` + `restore_vga`（#109/#35 双链）+ 失败非致命 |

---

## C. 可借鉴的设计（按可搬性排序）

他们比我们厚的地方集中在"拒绝纪律"和"证据纪律"。逐条给结论。

### C1. 可搬：收容判定 `GuSafeToReturn` + 三态收尾（成本低）

`port.c:59`。语义：只有"没改动过、没启动过、没保留缓冲"三者全真才允许
静默返回；否则必须显式收容后才许走。`app.c` 的 `failed:` 严格按此分派。

我们：`main.rs` 失败路径已经是"按失败类分派 + emergency FLR"，但没有一个
单一谓词把这三条写死，靠各分支自觉。建议落一个 `safe_to_return()`，失败
路径只读它决定收容动作。**收益**：新增失败分支时不会漏掉某一条。

### C2. 可搬：entry selector 锁存 `GuLatchEntrySelectors`（成本低，防误判）

`port.c:15`。进事务前把入口 SS0/SS1 锁存，之后所有 selector 读回都跟锁存
值比；`profile.c:80` 锁存失败即 `EFI_NOT_READY`。

我们：目前只在 `claim_guards` 里把 SS 的高低钳位当 EAlready/Ebusy 判据，
没有锁存做后验比对。**收益**：能区分"我们写失败"与"别人又改了"。
**风险**：冷入口 SS 本来就可能全 0（`feat-block-aon-reset-mechanisms` 的
AON 指纹），锁存值必须允许 0 且不当作期望值，只作"非预期变动"参照。

### C3. 可搬：selector 阶段机 + SS1 单次写消费（成本中，防重复写）

`port.c:24-30,138-142`：`ENTRY → SS0_WRITTEN → SS1_WRITE → TARGET`，
`Ss1WriteConsumed` 在写回调里置位，阶段跳变必须与之吻合，否则拒。

我们：`tx.rs` 是直序写 SS0/SS1，没有阶段约束。**收益**：重复写 SS1、
乱序写、回调里二次进入都能被抓到。搬的时候要把我们的写入点包进阶段回调，
不改写值语义。

### C4. 可搬：交棒前 fresh-MMIO 终门 `GuMsiReadyForWindows`（成本低，价值高）

`msi_release.c:513-523`。位置在 USB close 之后、StartImage 之前，**不经过
任何日志回调**，重新读 chip/CPUCTL/WPR2/SS0/SS1/readout/PLM 七项全验，
外加 `ConfigStable` 与协议图复检。

我们：`probe_post_console` 是日志探针，读同样几个值，但**没有判语**，也
没有"避开日志回调"的讲究。**收益**：把最终判据从"日志看起来对"变成
"最后一刻的硅状态对，否则拒绝交棒"。搬进 `tx.rs`，失败走收容 + 不链回
（或按现有失败类走）。

### C5. 可搬：cap 链完整性与 config 准入 `GuCheckConfig`（成本中）

`checks.c` 全函数。重叠位图、环检测、步数上限、长度语义逐 ID 校验、
PM 必须 D0、PCIe 必须是 endpoint v1/v2、MSI 必须未使能、ROM 解码必须关。

我们：`pci.rs` 的 cap 遍历只有步数上限和 below-0x100 兜底。**收益**：
把"cap 链可遍历完"变成可证事实，后面的 PM/PCIe 偏移才可信。
**降级搬**：未知 cap 记 warn 不拒（A11）。

### C6. 可搬：DMA 前驱白名单（成本低）

`port.c` 只在 `GuDmaAllocate` 里做页对齐/≤128 MB/`>4GB` 时查 32 位能力，
并把"谁在什么时候分配了哪块"记进 `Gu.dma_count`，final 报告里给
`retained=0 DMA_live=0`。

我们：分配器有 band 选择与描述符扫描，但没有一个计数器能一句话报
"本次事务还有几块活着"。**收益**：final 报告可读性；对照 #146 也顺手。

### C7. 可搬：177 场景注入矩阵的组织法（成本中高，价值最高）

`tests/test_efi_transaction.c:39-49` 的枚举：成功、FWSEC 超时、POST-FLR
超时、BME 读回失败、MMIO 写失败、Flush 失败、FLR 写失败、Unmap/Map/部分
映射失败、恢复失败、显示变化、selector 丢失、FWSEC 无 WPR、FWSEC 仍在跑、
marker 缺失、HALT 缺失、CYA/WPR/FEAT 未开、readout 未变、错误码，加上
`GU_ENTRY_LATCHED` 条件下的锁存类 8 项。断言写"什么不许发生"（arm 前不许
有写、SS1 恰写一次、未静止不许释放、失败不许 FLR、不许 PIO/DMEM 回退），
并且**成功场景也断言必须发生的事**。

我们：`sim/` 有 62 场景矩阵。**缺口在"不许发生"这一半**——我们的矩阵偏
"结果对不对"，他们偏"副作用有没有"。建议下一轮把每个成功路径补上反向
断言（尤其"arm 前零写""未静止零释放""失败零 FLR"）。

### C8. 可选：开放协议图 `Snapshot`/`Expected`（成本高，先别全搬）

`msi_release.c` 全套。他们用 165 → 152 的关系跃迁证明过它的价值，但也
为 USB 假阳性付了整整一轮返工（A1）。**建议**：等 C4 落地、真实故障面
收窄后再考虑，且只做"消失集必须 ⊆ 允许集"的半张图，不做全等。

### C9. 明知不搬

- 硬拒 BME（A6）：与我们 #1 的 0x89 修复方向冲突。
- BDF/子系统硬钉（A3）：我们是多卡工具。
- 停机 `Halt()` 不交还：已论证，保留 emergency FLR 交还。
- `Plan.stage` 整数阶段 + 每次 `Step()` 断言日志就绪：我们的日志是尽力
  而为，改成硬断言会把"盘写不进去"升级成事务失败，收益不抵风险。
- 128 MB 单分配上限：我们资产更大，分块已解。

---

## D. 采纳顺序建议

1. **C4 终门** + **C1 收容谓词**：纯判据，不动交易体，直接对上我们的
   "读回是唯一判据"和 #164 窗口。
2. **C2 锁存** + **C3 阶段机**：动 selector 写入路径，先在 `sim/` 全绿。
3. **A2 config 漂移只读 diff**（连同 C5 的 cap 链校验）：先记不判。
4. **A4 PROM hash 中道方案**：解析/记录先行，strict 开关后置。
5. **A5/A7 opener 与回调消费者残检**：一条 `EBusy` 通路解决两件事。
6. **C7 反向断言**：补进 `sim/`，与上述各项同批。
7. 观察一轮真机后再评估 C8 半张图。

## E. 一句话总结

他们的价值不在交易体（比我们薄），在**"不确定就不交棒"的判定密度**：三态
收容、锁存、阶段机、fresh-MMIO 终门、以及"什么不许发生"的测试写法。前四
条搬进来是净收益；他们踩过的 USB 假阳性与逐板硬钉则是我们必须绕开的两个
反向教训。
