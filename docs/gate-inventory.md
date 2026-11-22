# 门清单与砸烂边界——横向扩展盘点（2026-11-21）

> 动机：falcgen（[falcon-booter-payload-gen](../tools/falcgen/)）把"写链 → payload"做成了第一性
> 原理工具（G0–G3 金标逐字节复现 + TU116 首例），写链是 payload 唯一的政策输入——于是
> "砸哪扇门"可以参数化。本文回答三个问题：**除了 SS0/SS1 还有哪些门值得砸**、**砸烂的
> 边界在哪里**、**一包载荷物理上能砸几扇门**。判据纪律不变：读回是唯一判据；改常量不
> 构成已验证移植。
>
> 关联：[chain-walkthrough](chain-walkthrough.md)（事务逐步）、
> [first-principles-payload-generator](first-principles-payload-generator.md)（生成器底稿）、
> [cmp-gen2-sm-unlock-assessment](cmp-gen2-sm-unlock-assessment.md)（Gen2/SM/BAR1 评估）、
> [blackwell-gb20x-status](blackwell-gb20x-status.md)（50 系为何关闭）、
> [open-questions](open-questions.md)、[cold-values](cold-values.md)。

## 0. 结论速览

| 门 | 层级 | 家族 | 收益 | 状态 |
|---|---|---|---|---|
| SS0/SS1 SM 速率 | 配置层 | tu10x/tu11x/ga102/ad102 | FP16/BF16/TF32 1.5–2.6× 实测 | **在树**（四族预设已进 falcgen `--gate ss`，钉 SHA 自测） |
| AD102 BAR1 size（2^n） | 配置层 | ad102 | BAR1 上限 24/48G → 64G（vLLM TP8 实证） | 构造已并入 falcgen `--gate bar1`（64G=实证档；其余 2^n 档 UNPROVEN）；**实机仍欠 2684 卡验证（M5 门）** |
| CMP PCIe Gen2 | 配置层 | Turing CMP（50HX 实证） | 链路 Gen1→Gen2 2× | 研究档：Turing 帧预算 2 写封顶，第三写（XP3G）进不去——突破=pivot 重排（数周级） |
| Turing BAR1 (ReBAR) | strap 层 | tu10x/tu11x | BAR1 至 64GiB | **不需要载荷**（PSTRAPS DXE 枚举期写，NvStrapsReBar 实证）——独立立项行 |
| XP3G 域 | 配置层 | GA102 拒写 / TU102 可开 | 未定 | 采样登记（OQ#12） |
| TPC/L2 影子复活 | 熔丝层 | 全族 | — | 关闭：eff=熔丝\|软件，软件只能**加**禁用（aajax L2 复活实验失败） |
| SM 数量 | 熔丝层 | 全族 | — | 关闭（PR #55 无证据；论文三轴也不含它） |
| PCIe Gen3/4 | 熔丝层+DevInit 抑制+RM 重推导 | 全族 | — | 关闭（四重独立墙，每重有实测） |
| Blackwell GB20x | PL3-only 门 `0xfffffb8f` | GB20x | — | 关闭：引导链**无 SEC2 段**，载具不存在 |
| 专业卡模板 | FEAT_OVR_PLM 写使能 nibble [7:4]=0 | pro AD102（#237） | — | 结构性关闭：载荷写不能 latch；等第二张 pro 卡对照 |
| Pascal（10 系） | FWSEC=V1 描述符；无 FEAT selector 记载 | GP1x | 未定 | 未来研究行（见 §5） |

## 1. 复盘：攻击原理精确版与载荷物理约束

### 1.1 原理（一句话精确版）

**验签从未被骗。** booter 镜像永远是原厂签名的 stock image——RSA/hash 照过；
`booter_patch_v67()` 只是把 0xFA00 的 V67 镜像盖进 patch 槽。真正的洞在签名**搬运**
这一步：签名段拷贝进 SEC2 DMEM 的**长度**取自 WprMeta+0x50，主机可控且无上界。
主机把"签名长度"报成 0xFA00，booter 就把 64000 字节（真签名只占很小一段，其余全是
我们的 fill 字+帧链）原样 DMA 进自己的 DMEM 栈区 → 溢出。V67 段全程**不被验**——
它是数据不是代码；哈佛架构 IMEM 不可写，它也永远不可能变成代码，执行的始终是
booter 自己的指令（ROP 只是重排调用序）。

**canary 均匀性攻法**（用户常表述为"执行段末端检验位和检验源被覆写成相同 pattern"
——正确）：stack protector 的全局参考字（GA102 @ DMEM 0x6140 / AD102 @ 0x5F50）与
所有保存槽都在同一次溢出的可及范围内，用同一常量覆盖两处 → 每个尾声的 V==V 校验
必然通过。论文 *A Canary in the Crypto Mine*（DOI 10.5281/zenodo.20916112）把这从
经验规律升格为机制解释："secrecy and entropy are irrelevant"——参考字不需要猜，
它和被覆写的槽位在同一块可写 DMEM 里。

配套纪律：FLR 前**签名复原**（绝不让 V67 留给后续原厂引导）+ 读回三证
（marker/HALT/PLM；GA10x 为 PLM-flip，Turing 为 PLM-flip+relock 尾）。

### 1.2 载荷物理约束

- **尺寸钉死在 0xFA00 = 64000 字节**（四族一律；代码常量 `PAYLOAD_SIZE`，#246 实物
  64000B 证实）。溢出 DMA 的目的基是 booter 代码行为、**固定不可控**（我们唯一的
  自由度就是长度），终点必须精确落在 SEC2 DMEM 顶 0x10000——帧链住在顶部
  0xFF38–0xFFFC。短于 ~0xF938 够不着帧区；长于 0xFA00 越过 DMEM 顶（行为未验证）。
  **更大尺寸买不到任何东西**——可写窗口被 64 KiB DMEM 钉死。
- **写数上限 = 帧预算（逐 die）**，现已是 falcgen 的机械校验（`write_budget`，
  超限硬拒并打印全表）：

  | 族 | 上限 | 依据 |
  |---|---|---|
  | tu102 / tu11x | **2 写** | 链帧区 (0xFF80−0xFF50)/0x18 = 2 帧；第三帧必与 mutex 尾块碰撞（cmp-gen2 §1 的碰撞算术） |
  | ga102 | **1 写** | 19 词 put-list 模板容量 |
  | ad102 | **4 写** | 25 词 put-list 模板容量（bar1-64g 现役即 4 写） |

- **主机侧补写不占预算**：FEAT PLM 开后的 SS0/SS1 写、XP3G 开后的 7 项策略写都由
  主机/EFI 完成——"一包载荷砸门数"与"端到端解锁门数"是两个口径。
- **突破路径两条，均为研究档（未实现）**：
  1. **pivot 重排**：cmp50hx stockflow 式，帧挪 0xBB00 区 + stack-safe pivot——另一套
     ROP 结构，逐 gadget 重审计（宪法 §9.1），数周级。
  2. **FLR 间隔多发**：门开在 AON 岛、FLR 打不回去（OQ#13 实测），FLR 又清 WPR2/
     复位冷态（bar1 报告）——原理上"一发一扇门、FLR、再发"可绕开单包预算。但
     **AD102 双发第二发超时有实测反例**（bar1 报告："早期重复 Booter 方案在第二次
     调用超时"），最可能死因=第一发最小链 park 自循环不释放 ACR mutex（全 parity
     尾帧的 mutex 级联就是为干净收尾存在的）；且 loader 的 WPR2/plm 冷态守卫会把
     第二发判 EBUSY/EALREADY（守卫语义需分级改造）。若未来立项，验收=每发读回
     三证 + 第二发起全 parity 干净收尾 + FLR 后逐门存活读回。

## 2. 门清单（全量展开）

### 2.1 SS0/SS1 SM 速率门（在树）

- 寄存器：Turing FEAT 块在 **0x4096xx**（PLM=0x409650/SS0=0x409664/READOUT=0x409668/
  SS1=0x40966c，cmp50hx 定址；OQ#1 的答案改写）；GA10x/Ada 在 **0x8238xx**
  （FEAT_OVR_PLM=0x823804/SS0=0x82381c/SS1=0x823820/READOUT_1=0x823818）。
- 写链：tu10x/tu11x = FEAT PLM 开 + WPR2_HI=0（2 写）；ga102 = FEAT PLM 单写+
  主机写 SS（1 写）；ad102 = WPR_CFG 0xfffff0ff → FBPA → WPR → FEAT 四写链。
- 收益：FP16/BF16/TF32 出厂半速 → 2×；逐卡实测见[兼容性矩阵](compatibility.md)与
  [theoretical-uplift](theoretical-uplift.md)。
- 判据：GA10x = wait-PLM flip；Ada = marker/HALT/读回三件套；Turing = PLM-flip+
  SS 读回（**relock 尾见 §3.7**）。

### 2.2 AD102 BAR1 size 门（2^n，构造已工具化）

- 寄存器与构造（[bar1-64g-report](../reference/gpu-unlock-bar1/docs/bar1-64g-report.md)
  八卡实证；PR #95 分支落地）：CYA 0x88724=0x30、CFG 0x88dcc=0x8000000a、
  XVE PLM 0x88ff4=0xffffffff、FEAT 0x823804=0xffffffff；marker 0x67b00004；
  **SEC2 必须直写 CYA/CFG**（主机写即使开 PLM 也被拒，八卡实测）。
- **尺寸编码是 2^n 表**（`64MiB<<i`），三点交叉验证：TU106 stock CFG 0x02=256MiB
  （i=2）、AD102 stock CFG 0x9=32GiB（i=9，bar1 报告 FLR "restore to 32 GiB"）、
  64GiB=0xa（i=10）。CYA 两点拟合 `(index-4)<<3`（0x28@32G、0x30@64G）。
  **只有 64G 是实证档**；32G/128G 等按同一规则可构造，产 manifest 标 UNPROVEN。
  48G 不是 2^n，不在表内。
- falcgen `--gate bar1 --bar1-size 64` 产物 = cf3566d0（八卡工件 SHA）逐字节复现
  （tests/goldens.rs 钉死）；128G 产 UNPROVEN 构造。
- 平台侧配套（桥窗口分配）是 Linux 侧独立工具，不随本仓交付。

### 2.3 CMP PCIe Gen2 门（研究档）

- 机制已完整实证（Turing 侧，cmp50hx patch 04 + [cmp-gen2 §1](cmp-gen2-sm-unlock-assessment.md)）：
  载荷内开 **XP3G PLM 0x8e1b0** ← 0xffffffff，主机侧 7 项策略写
  （0x8841c/0x8e120/0x8e110/0x88610/0x8c2c0/0x8c040/0x8c1c0）+ LTSSM kick 0x8872c←6 +
  配置空间 LNKCTL2 TLS=Gen2；**LnkSta 才是判据**（LnkCap 会说谎）。
- 收益：矿卡 Gen1→Gen2 2× 链路带宽（P104-100 先例 p104-gen2-unlock 同理 2×；
  该先例走 PCI 配置空间+PL 内部寄存器运行期写——**正交车道**，不经 PLM）。
- 卡点：Turing 帧预算 2 写封顶，XP3G 第三写进不去——**falcgen 的预算校验把这条
  边界变成了机械拒绝**（`--family tu102 --writes 0x8e1b0=…,0x409650=…,0x1fa828=…`
  → exit 2）。落地路径=pivot 重排（§1.2）或 stockflow 载荷审计。
- 门控纪律：仅矿卡设备 ID（30/40/50HX=1exx/21xx、90HX=22xx），永不按"任意 Gen1
  钳制卡"放宽。TU106 consumer 的 XP3G PLM 读 0xffffffff（OPEN）——40HX 卡对照
  是下一轮输入（ssdiag PCIE_LINK_ROWS/XP3G 面已可采）。

### 2.4 Turing BAR1 ReBAR（strap 层，不需要载荷）

PSTRAPS SET0[15:14]+SET1[22:20]（BAR0+0x101000/0x10100c，和值编码 0=64MiB…10=64GiB）
在平台 DXE PCIe 枚举期写（NvStrapsReBar 实证，OQ#11）——**与 SEC2 载荷正交**，载具是
主板 UEFI 镜像内 DXE 驱动。app 期改 straps 需自重配 PCI 窗口，若做须单独立项。
注意 §2.2 的尺寸表与这里是同一张 64MiB<<i 表的两个消费方。

### 2.5 登记行（证据在库、无落地路径或无收益）

- **XP3G 域**（OQ#12）：cmp90hx 实证 GA102 拒写、TU102 可开；语义未定，采样面已进
  ssdiag（M1 采样扩展清单）。
- **TPC/L2 floorsweep 复活**（OQ#13/#19）：Ada 熔丝控制器 12 个 PLM 实址已定位、
  V67 开的 PLM 过 FLR 存活；GA100 先例（170hx-rir-tool）有 `_CP` 影子易失改写——
  但 aajax 在 AD102 上的 L2 复活实验**失败**：effective disable = 熔丝层|软件掩码，
  软件只能加禁用。关闭。
- **SM 数量**：PR #55 "+4 SM" 无公开验证；GA100 专属机制；决定性反证同上。关闭。
- **PCIe Gen3/4**：四重独立墙（无写端口熔丝反射 / RM 每次重训练重推导 / 签名 DevInit
  抑制位 / GSP-RM 策略层），24 条被驳方法目录在 cmp-gen2 §7。关闭。
- **Blackwell GB20x**：门全测绘（FEAT_OVR_PLM=0xfffffb8f，PL3-only）但引导链
  FSP→GSP BootROM→FMC→GSP-RM **没有 SEC2 段**，签名 booter 溢出载具不存在；
  debug FMC 与 prod 逐字节同密文、MD5-IV 置换族 16! 全搜负结果。详见
  [blackwell-gb20x-status](blackwell-gb20x-status.md)。等 PL3 执行原语出现前不立项。

## 3. 砸烂边界（判据清单，exe 逐条机械化）

1. **层级尺**（cmp-gen2 §7）：配置层钳制（DevInit 落进可重写寄存器）→ 可破
   （SS 速率 ✓ / Gen2 ✓ / 170HX 显存几何 ✓）；熔丝层配置（无写端口反射 / 签名固件 /
   RM 运行期重推导）→ 不可破（SM 数量 ✗ / Gen3-4 ✗ / L2 切片 ✗）。
2. **PLM 写使能 nibble**：冷读 FEAT_OVR_PLM [7:4] 必须 =8（L3-write-only）；=0 是
   专业卡模板（#237 Willian-Zhang AD102）——载荷写不能 latch，结构性关闭。已进
   manifest 前置条件行。
3. **无 SEC2 段的族**（GB20x）→ 无载具，不在 family 白名单（`write_budget=0`）。
4. **帧预算**：§1.2 上限表；falcgen 硬拒 + 全表打印。
5. **debug key 硬边界**：没有 debug 明文的族，生成器无输入（GA=#37 族/TU=#6 族已破，
   AD/GA100 明文在库）——这是"不是工程缺口"的硬边界。
6. **熔丝总开关**：OPT_FEATURE_FUSES_OVERRIDE_DISABLE（0x8203f0）=0 是解锁域存在
   前提（GB20x 亦为 0=未关，唯一墙是特权级）。
7. **OS 侧接受度（2026-11-22 更新）**：交棒形态谱——open `0xffffffff` 被拒
   （#246，TU117/Linux）；**relock `0xffffffef` + SS live 在 TU116 上同样 Code 43**
   （#280，Windows 621.14/v2.1.11）；TU102 两形皆活（#12/#103/#125/#240）→ **TU11x 的
   RM 拒绝与 PLM 交棒形态无关，"已证形"不外推出 TU102**。判别阶梯（#280 回帖已发）：
   ①legacy 驱动格（TU11x×legacy 未测；TU106 四象限先验为负——门开×legacy=Code 43、
   relock 门关×legacy=TDR 洗回厂，rc4 定谳 → `EnableGpuFirmware=1` 是 20 系机理级硬前置）
   ②rc5 keep-open 重放（v2.1.11 tag 在 061aef8 之前；relock 写在 621.01/630.20 代有
   #99 PF-FLR 反应家族，621.14 同代）③全厂回锁 `0xffffff8f` 消融（TU102 上
   0x8f+SS live 已证好——"bits 5:6 sticky-open 非 die 普适"；loader knob 筹备中）
   ④残差 diff/⑤RM 错误码解码/⑥驱动代际。
   GA10x = PLM-flip verdict；Ada = marker/HALT。manifest 按 family 打印对应提示。
   manifest 按 family 打印对应提示。
8. **逐芯片几何**：booter 尺寸/gadget 地址/签名段名逐族不同——跨族移植=重抽取重审计
   （宪法 §9.1）；falcgen 把"新 die 边际成本"降到一把 debug key + 一条命令，但
   **不降低审计义务**。

## 4. falcgen 工具面（本轮落地）

```bash
# 各族 SS 解锁链（四预设被钉 SHA 逐字节验收）
falcgen synth --family tu11x --gate ss            # = c79208ac…（pinned）
# AD102 BAR1 64G（= cf3566d0… 八卡工件，PROVEN）
falcgen synth --family ad102 --gate bar1 --bar1-size 64 --out v67-bar1-64g.bin
# 128G 实验构造（UNPROVEN：CFG=0x8000000b/CYA=0x38，manifest 明示）
falcgen synth --family ad102 --gate bar1 --bar1-size 128
# 原始写链（预算校验；gen2 三写链在 Turing 被硬拒 = 边界 4 的机械证据）
falcgen synth --family tu102 --writes 0x8e1b0=0xffffffff,0x409650=0xffffffff,0x1fa828=0
```

- `--out` 落 manifest 侧车（写链/marker/预期读回/前置条件/UNPROVEN 标记）；
  产物与金标永不入库。
- 行为级验证：`sim::run_payload_chain`（FakeGpu  privilege 模型）接受任意写链，
  载荷写按 heavy-secure 落地、主机写按 PLM 过滤——预设链的"开门→主机收割"语义
  有场景测试。
- 集成深度：**仅离线产物**。loader/maker 零改动；产物进 U 盘属用户后续门控决策，
  实机执行仍走里程碑门控 + 显式确认。

## 5. 未来行（登记，不承诺）

- **Pascal（10 系）**：FWSEC 走 V1 描述符（接口偏移布局不同）；无 FEAT selector 域
  记载（Turing 的 0x823xxx 在 Turing 自身都 badf，Pascal 更无人测绘）。候选门：
  ①PCIe Gen2（P104 先例=配置空间+PL 寄存器运行期写，正交车道，无需 SEC2 载荷）；
  ②FP64 速率机理未知。无 debug key 公开记录=生成器无输入（边界 5）。先做只读
  词典轮（ssdiag GA100VFI 词典路径），再谈载体。
- **Gen2 pivot**（§1.2 路径 1）：envdis 审计 cmp50hx stockflow 载荷 + falcgen pivot
  文法，单独立项。
- **FLR 间隔多发**（§1.2 路径 2）：需守卫语义分级 + 全 parity 干净收尾先行，单独立项。
- **BAR1 非 64G 档**：2^n 规则已可构造（128G manifest 标 UNPROVEN）；任何实机使用前
  需先拿该档 stock 读回对照 CYA/CFG 规则，并补 ReBAR capability 证据。

## 6. 来源

实测：本仓 evidence 树（per-issue/die 捕获）、bar1-64g-report（八卡）、#246/#237/
#168 issue 记录、ssdiag 0.8.x 寄存器面。社区/reference：cmp50hx-unlock（Gen2 实证）、
gpu-unlock-bar1（BAR1 构造）、p104-gen2-unlock（Pascal Gen2 先例）、cmp170hx +
*Canary in the Crypto Mine*（机制层）。裁决顺序照任务书 §10：
open-gpu-kernel-modules > 参考仓库实现 > 文档 > 记忆。
