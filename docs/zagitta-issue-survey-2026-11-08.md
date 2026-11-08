# Zagitta/gpu-unlock issue 全量通读（2026-11-08）——前人踩过的坑对照表

> 采样口径：`Zagitta/gpu-unlock` 全部 320 个 issue（state=all，含 129 open），逐条读标题与
> 分类，46 个技术条目全文+评论区精读。目的是把上游 Linux kmod 的失败模式映射到本仓
> （Windows UEFI 预驱动、一次性事务）的暴露面上。
>
> 结论先行：上游绝大多数坑我们**已经踩过并定过机理**（Turing 簇 8 条、平台/固件簇 6 条、
> 恢复簇 3 条）；真正**还没踩、需要主动避开**的集中在"解锁成功但没收益"的判读层、
> 板级/驱动侧速率上限、以及两个尚未闭合的上游缺陷（第二发 booter、失败 containment）。
>
> 上游是 in-OS kmod，拥有设备生命周期、可重试、可隔离；我们是 pre-driver 一次性事务，
> 失败语义更窄。下文凡是上游靠"重试/隔离"解决的，我们的等价物是"入口守卫 + 失败 FLR +
> 链回 bootmgfw"。

## 1 必须记在 AGENTS §9 的验收坑（我们还没踩）

### 1.1 同一次事务里**第二发 booter 永不 HALT**（ZG#252，GA102 实测）

上游想把 PLM 关回去，做法是"再点一次 booter"：reset、reload、start 全部成功，`HALTED`
5 s 内不置位（`poll timeout reg=0x840100`），模块保留缓冲、拒绝交棒、卡到断电为止。
上游的退路是**放弃关回**，让大门常开。

- 对我们的暴露：我们在 Turing 上已经把 `0x409650` 关回 `0xffffff8f`→实测 `0xffffffef`
  （`loader/src/tx.rs:1429`），但那是**主机写**（门还开着时），不是第二发。如果哪天有人
  想在 GA10x/Ada 上也"关回"，第二发这条路是已知死路。
- 规避：GA10x/Ada 关回只走主机写；要走第二发必须先有 `HALTED` 到不了位的超时判据。

### 1.2 PLM 常开 = 交棒后的篡改面（ZG#252 的立项动机）

常开的 `FEAT_OVR/WPR_CFG` 让任何能到 BAR0 的代码（驱动、root 进程、透传的 VM/容器）
继续改选择器和 WPR2 围墙。单机低风险，租卡/云主机是实质风险面。

- 现状：Turing 已关回（上面那条），GA10x/Ada **仍常开**（`tx.rs:436` 只对 Turing 换地址，
  非 Turing 从不回写）。
- 规避：`docs/faq.md` 增一条"交棒后门是开的"披露 + 透传/多租场景提示；关回方案见 1.1。

### 1.3 板级/驱动侧速率上限——"解锁成功"≠"2×"

`ZG#28` 一块 GA102-250（board `0x300`）在同一块卡上跑出 512→668→846 FLOP/SM/clk 三级台阶：
驱动版本（615→610）抬一级，Windows UEFI 路径再抬一级，但永远到不了同型号 `0x151E` 板的
1024。结论是 VBIOS 速率表差异，不是工具问题。

- 对我们的暴露：兼容矩阵里"×2"是承诺口径；遇到低速率表板卡，用户会报"工具没用"。
- 规避：矩阵按板卡记期望值而非倍数；GEMM 倍率旁必须同时记 `FLOP/SM/clk` 折算值
  （`TFLOPS / clk / SM`，1024=满速），让"刚好一半"可被算术识别。

### 1.4 默认 GEMM kernel 吃不满 → BF16 表观 1.00×（ZG#308）

AD102 实测：PyTorch 默认 cuBLAS kernel 是 64×64 tile、L2 利用率 92.8%、tensor 利用率
48.1%，解锁前后 1.002×。换成显式 cuBLASLt 256×128 后同频 1.976×、同卡 2.12×。

- 对我们的暴露：我们的 FAQ"没收益"判据目前只讲功耗/SM 饱和（`faq.md:178`），缺
  "算法/kernel 选择"这一轴。用户照着跑默认 matmul 会得到假阴性。
- 规避：判据加一条——**先用大 tile 内核（cuBLASLt/Triton 128×256+）跑出 `FLOP/SM/clk`
  折算值**，再谈端到端负载；默认 kernel 的倍率只作参考不作判据。

### 1.5 计量陷阱三连

- `CUBLAS_COMPUTE_16F` 在 CUDA 13.4/sm_89 上返回垃圾（0.02 ms、7000+ TFLOPS，`ZG#168`）。
- 解锁后 `nvidia-smi` SM 时钟遥测可在满载下钉死 210 MHz，纯观感问题（`ZG#258`，同机
  580 驱动正常、595 复现）。
- SS0 写完读回**可能合法地不相等**：某 die 的 bit9/10 硬连 0，写 `0xFFFFFFFF` 读回
  `0xFFFFF9FF`，且全 1 会让 BF16 通路死锁超 120 s（`ZG#28`）。

- 规避：`write_selector()` 的"读回必须精确等于写入值"（`tx_booter.rs:416`）在硬连位卡上
  会假失败——改成"八个 nibble 的 override 位全部置起 + selector 为 000"逐字段判定，而不是
  整字相等；全 1 不进默认值。

### 1.6 SS0 上电值不是 `0x00000001`

同一 `ZG#28` 实测冷入口 `ss0=0x25313044`（八字段全 off 但选择器数值 2/5/3/1/3/0/4/4），且
跨启动不稳定（另见 `0x25113044`）。拿"出厂=1"当基线会把这类 SKU 误判成异常。

- 规避：M1 采样把 SS0/SS1 原值按字段解码落日志（现在只落原值），并在读回判定里接受
  "字段已 off 但 nibble 非 0"。

### 1.7 透传/虚拟化下的回滚语义是假的

`ZG#255`：ESXi 客户机里去掉 `gpu_unlock.targets` 并重启客户机，性能仍是解锁值——客户机
重启够不着 AON。`ZG#263`：PVE 直通不勾 PCIe 直接 `-EOPNOTSUPP`。

- 规避：FAQ"回滚手册"补一条——**VM/DDA 场景的"重启"不等于断电**，必须宿主冷启或整机断电；
  直通必须勾 PCIe。

### 1.8 交换机后多卡的整体性风险（ZG#260）

5×3090 在 PEX880xx 后面，解锁成功后交换机端口 `24:10.0` 报致命错→DPC→整片交换机丢失→
Xid 79。我们是逐卡顺序事务，一块卡的 AER 可能带走同交换机兄弟卡。

- 规避：多卡顺序执行的间隔与失败后置位要留痕；文档提示"交换机后多卡"按批跑、
  一次只动一块、失败即停（现行"失败不阻断"策略对交换机拓扑要加限定）。

### 1.9 恢复路径本身可能挂死宿主（ZG#311、ZG#303）

- `ZG#311`：S3 恢复期解锁路径 FLR+GSP 交换后 3 s 报 Xid 79，随后 `nvidia_modeset` 恢复
  路径 slab 损坏、`systemctl poweroff` 卡死，只能 `poweroff -f`+物理断电。
  24 次好、1 次坏，入口 `ss0` 与好样本同值——**无先兆**。
- `ZG#303`：4 卡 VM（direct 模式）空闲 1.5 h 正常，一加载模型宿主整机冻结，两次不同版本复现。

- 规避：这是我们"事务完成后交棒"的最大残余风险面，不在我们能控的一侧；文档要写
  "解锁态 + 长跑/睡眠"本就是未定区间，恢复步骤必须给到 `poweroff -f`。

### 1.10 两条路线判断，别再重开

- **改 VBIOS 刷写替代本工具**（ZG#203）：维护者结论是"必需的那部分信息根本不在 VBIOS 里"，
  因为解锁作用于驱动自己加载的固件。但 VBIOS 确实决定板级速率表（见 1.3）和 P-state 策略
  （本仓 open-questions #14 的 P8 钉死），两件事要分清。
- **NVIDIA 会不会封/变砖**（ZG#169）：最坏是驱动侧加校验（可 patch、失效而非关锁），
  砖不了。本仓已有 `driver-version-unlock-coupling` 一系分析，无需重做。

### 1.11 供应链：同源校验和只防损坏，不防投毒

ZG#306 指出 Debian/Unraid 路径下载零校验、Unraid 的 MD5 与包同源、`run:` 里直接插值外部
JSON、容器里编下载来的代码。我们的资产管线已经是"钉 SHA-256 + 本地再生 + 产物 hash 不符
硬失败"，且 pin 校验落在**输入**上（`assets/src/main.rs:16,24`），缓存命中同样过闸——这条基本
免疫。残余只是下载环节本身还走 curl，参见 #205 那轮的传输层修复。

## 2 我们已经踩中且机理已定（上游同坑对照）

| # | 上游坑 | 上游表现 | 我们的一手结论 / 实现 |
|---|---|---|---|
| 1 | 板载固件留着 BME | ZG#5/ZG#6 Supermicro X11/H13 全线 `-EBUSY`，改代码后四卡全绿 | AGENTS §9.3；`tx_booter.rs:1378` 事务前清 BME+读回，FLR 段 `BME stuck` 拒绝；`bridge-bme-root-cause.md` 定位到上游桥 |
| 2 | 冷态守卫语义 | ZG#43 要求"保留/隔离"而非盲清；ZG#23 证明 `-EBUSY` 承诺被打脸 | `claim` 的 `-EBUSY`（WPR2 高）与 `-EALREADY`（FEAT 已开）两码分家（`tx.rs:463,470`），`claim_plm` 三分类 |
| 3 | 布局必须等 GFW | ZG#70 雷电 eGPU 晚枚举，FB 读回 `0xbadf5108`→`-22` | `tx.rs:671` 先等 GFW 再读布局（对应上游 `wait_post_flr_gfw`） |
| 4 | Turing loader 死在 HALTED | ZG#242/ZG#243/ZG#247/ZG#248 十余张卡全灭，`last_mmio=0x110100:0x10` | `tx_booter.rs:994` FBIF `ALLOW_PHYS_NO_CTX`(bit7) 保留置起 + `DMACTL=0`；`tx_fwsec.rs:1014` TU10X GSP-HALT 门等 halted |
| 5 | Turing SEC2 CPUCTL 复位后仍 HALTED | ZG#294 需容忍 `-ETIMEDOUT` | `tx_booter.rs:1008` BOOTER-C 入口前置判定（halted+可读） |
| 6 | Turing FEAT 块在 `0x4096xx` 不是 `0x823xxx` | 上游首版照 GA102 写 `0x823804`，Turing 全灭 | `tx.rs:433,1659`；`open-questions.md` ZG#1 已把旧前提推翻 |
| 7 | Turing `WPR_CFG_PLM` 从不向主机开 | ZG#294 `restore_frts` 被丢弃 `-5`，上游直接跳过 | `tx_booter.rs:1271` WPR2_HI=0 由载荷第二写完成，主机不写 WPR_CFG |
| 8 | Turing 需要 ACR mutex 3 释放 | GA102 六写链在 Turing 上 `0x7bbc`/`MAILBOX0=0x47` 空转 | `tx_booter.rs:1271` TU102 两写链，注释写明 mutex 3；FEAT=`0x409650` |
| 9 | Turing FLR 冲掉选择器 | 上游所以跳 FLR | 我们同样跳（handoff 三值判定），并额外发现 **FEAT 门开着会把 Windows Turing 驱动打死**，因此回锁到 `0xffffffef`（`tx.rs:1429`）——上游 Linux 不需要，我们比上游多一步 |
| 10 | `badf` 毒值必须分桶 | 上游 1.x 把 `0xbadf...` 既当"完成"又当"不可达"（ZG#42） | `sample.rs:116` 分 `5040`/`1100`/`20xx`/`VARIANT` 四桶；`tx_booter.rs:930` 判定用 `0xffff0000==0xbadf0000` 统一 |
| 11 | ≥4 GiB DMA 可能不可达 | ZG#241（RPi5 全窗口在 4 GiB 上）→改 47 位掩码 | `tx_fwsec.rs:804` 低带默认 + `\tx-fwsec-high-staging.txt` 回高带；本仓#74 定案 VT-d 是分界线；本仓#156 DGFF 0x89 |
| 12 | CSM 打开即 Code 12 | ZG#28 补充：X99+CSM 事务全绿但 Windows 立刻 `Code 12/0xC0000018` | `docs/csm-compat.md`、本仓#35/#61 定律"交棒 console 归属链连贯与否"；native-GOP 重建默认化 |
| 13 | 失败后保留资源→驱动绑不上 | ZG#21/ZG#23/ZG#184/ZG#242/ZG#243/ZG#247/ZG#248 全族：`request_mem_region` 失败、整机无 GPU、LUKS 提示都没了 | 失败路径强制应急 FLR + 完整 unwind（`tx.rs:1709`），签名先复原（`tx.rs:1360`）；不靠 errno 判断 |
| 14 | 预检失败被当"固件已启动" | ZG#184 纯 DMA 几何校验失败也走保留路径 | `tx.rs:968` 预检失败"什么都没动、无可 unwind"的显式分支 |
| 15 | DMEM 尺寸非 256 对齐 | ZG#184/`ZG#35`/`ZG#57` `0x980` → `-EINVAL`/`-22` | `tx_fwsec.rs:228,584` 上行取整 256 + 尾部清零 |
| 16 | 白名单漏 ID 静默跳过 | ZG#312/ZG#270/ZG#212（`0x2520` 缺条目，零日志） | `main.rs:358` 显式 "no asset family mapping - skipping"；`main.rs:365` 注释直接写"曾被静默跳过"；白名单全段开放策略 |
| 17 | 资产 codegen 会变 | ZG#224/ZG#168 610 字节 token→615 `NvU64` word token + 8 字节对齐尾巴 | 逐级 pin hash，新版本硬失败而非静默降级；`docs/cold-values.md`/open-questions 记版本漂移 |
| 18 | 交棒时把低位内存劈碎 | 本仓#146（winload `0xc000009a`） | `docs/handoff-lowmem.md`：默认释放资产、`\tx-keep-assets` 退出阀、CHAIN-MEMMAP largest block |
| 19 | 一套"没收益"误判 | ZG#124/ZG#135/ZG#230/ZG#269/ZG#111 | FAQ `faq.md:145`（Linux）、`:178` 判据、`:355` 睡眠、`:208` 回锁机理五类 |

## 3 尚未闭合的残余（诚实清单）

1. **GA10x/Ada 的 PLM 关回**（见 1.1/1.2）——需要"主机写能否关门"的实机确认，Turing 已证可以。
2. **硬连位 die 上的 selector 判定**——`write_selector` 仍是整字相等（见 1.5）。
3. **算法/kernel 选择轴进判据**（见 1.4）——纯文档工作，未做。
4. **交换机拓扑下的多卡失败策略**——现行"失败不阻断"未区分直连与交换机后。
5. **VM 回滚语义**——FAQ 未写"客户机重启≠断电"。

## 4 元教训

- `fc83321` 一次重构把 Turing 全线打死，上游靠用户逐个报、再整族回退（ZG#20/ZG#21/ZG#23/ZG#242…）。
  我们的等价物是**逐族几何 + 每族独立回归**；任何跨族共用路径（Falcon、DMA、unwind）
  改动必须在 `sim/` 上族矩阵全绿，不能只跑一族真机。
- 上游的"失败后保留资源"是为了不让固件继续 DMA 的正确取舍，但没有 containment 就变成
  "整机在本次引导里没显卡"。ZG#272/ZG#43 用了两个 issue 才把方向掰回"用成功路径的 FLR 收尾"。
  我们的 `tx.rs:1709` 已经是这个形态，别退化。
- 上游 README 写过的承诺（"失败会把卡留在原样"）与实现不符，被 ZG#23 抓住。**承诺要有
  读回证据撑**。
