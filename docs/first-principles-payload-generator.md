# 第一性原理 V67 payload 生成器——调研报告（2026-11-05）

> 动机：#151（TU117 `1f82`）暴露"逐芯片几何"依赖人工 RE——上游没有 16 系 payload，我们手工做了单字重锚（`assets/src/payload.rs` PATCHED_WORDS_TU11X）。本调研回答：能否造一个**只吃解密 booter 明文、自动产出完整 payload** 的生成器（"fuc5 ROP 编译器"），以及它的边界在哪里。
>
> **黄金标准**：生成器在三张已钉的上游表上逐字节复现 SHA-256——GA102 `2a0aa1f8…`（Zagitta/gpu-unlock 谱系）、TU102 `b0ec9029…`（nlzy/turing-unlock）、AD102 `44c36b2d…`（gpu-unlock-ada）。三张全中 = 生成器被证明，此后新 die（TU116 已是第一个受益者）不再等上游。

## 1. 结论摘要

| 层 | 可行性 | 依据 |
|---|---|---|
| 帧算法 | **已完全解剖** | cmpunlocker cpu-rm 的 `generate_v67.py` 就是平表的算法形式，逐字节解释我们的 TU102 表（§2） |
| gadget 层 | **可机械推导（原型 4/6 + 2/5）** | 150 行位置幼稚扫描器从明文自动找回 TU102 的 0x1F89/0xCBD/0xE85/0x7EA4 与 GA102 的 0xD44/0x1101；miss 全部精确归因（§3） |
| 栈几何层 | **最难，两条路线，数周级** | live 帧槽位/步长/拷贝基需要"签名 DMA 完成时刻的调用栈现场"——静态帧分析或仿真到断点（§4） |
| 尾帧语义 | **分两档目标** | 全 parity（mutex 释放、干净 HALT）难；最小 write-and-park 链容易但改 loader 判定契约（§5） |
| debug→prod 迁移 | **有一个实证分歧点 + 一个兜底实验** | NS 前置页两立即数逐 build 不同（0x1650/0x1658 vs 0x1660/0x1668）；SEC 密文不可比；黄金标准复现实验恰好度量迁移性（§6） |

**总判断**：可行，值得立项，但它是"数周级工具项目"不是"几天级脚本"。最大杠杆是**位置无关 fuc5 解码器**（fuc5 操作码表在 `envydis/falcon.c`，一次性转录）；最大风险是栈几何推导需要仿真基础设施（faucon 只有 asm/disasm，**没有仿真器**——此前假设有，已纠正）。

## 2. 帧算法已完全解剖（cmpunlocker `generate_v67.py`）

`reference/cmpunlocker/linux/cpu-rm/firmware/generate_v67.py` 的 `build_payload()` 与我们的平表逐字节同构（我自己逐位对过全部 43 词）。它揭示了算法骨架——payload 只有一处"政策输入"，其余全是几何常量：

```
writes = [(0x409650, 0xffffffff), (0x1fa828, 0)]   # 唯一政策输入：写链
拷贝基 0x600（payload DMA 进 DMEM 0x600..0x10000，put(dmem) = dmem-0x600）
live 帧：0xFF38=guard | 0xFF40/44=write0 值/址 | 0xFF4C=返回槽→0x10B9（跳过函数序言的中项入口）
链帧：sp = 0xFF50 + i*0x18（帧步长 18 = 尾声弹出量）
       [sp]=guard | [sp+8/+C]=下一写的值/址 | [sp+14]=0x10B9；最后一帧 → 0x7C43(teardown)
补丁：0xFF74←0、0xFF7C←0xCBD          # teardown 被换成交互 token——平表"先 0x7C43 后 0xCBD"之谜的解释
尾块（步长 0x14）：0xFF80..0xFFEC       # 0xC95/0x1F89/0xE85 token 帧 + 0x7EA4 park
canary 地址 0x6140；main SP 0xFFF0；fill 0x4A7
```

于是第一性原理问题收敛为一张**常量推导清单**：

| 常量 | 性质 | 推导途径 |
|---|---|---|
| fill 0x4A7 | ISA 常量 | fuc5 自循环编码，一次性 |
| gadget VA ×7 | 代码属性 | **扫描器（§3，原型已证）** |
| canary 地址 0x6140 | 代码属性 | 扫描器：找 `mov $r15,imm; st D[$r15]` 全局置位模式（NS 前置页与 0x1F89 帧都引用它） |
| 拷贝基 0x600 | 行为属性 | 签名 DMA 描述符：找"长度取自 WprMeta+0x50"的 DMA 设置代码，提目的地址 |
| live 帧 0xFF38/40/44/4C、步长 0x18、main SP 0xFFF0 | 调用栈现场 | **栈几何推导（§4，硬点）** |
| 尾块 token 0xCBD/0xC95/0xE85/0x1F89、teardown 0x7C43、park 0x7EA4 | 代码+协议属性 | token=尾声级联的可通过点（扫描器给候选，仿真给排序）；park=任一 `lbra self`（扫描器已证） |

## 3. gadget 层原型实证（`.relcheck/tu11x/gadget_scan.py`）

方法：从 envydis 线性反汇编（`*_sec.lst`）**自举指令长度表**（首字节→长度，TU102 得 115 个首字节、仅 4 个歧义），然后从**每个字节偏移**（双奇偶——中项 gadget 是奇地址）线性解码 ≤10 条，以 ret 类/`lcall`/`lbra` 为 ROP 终结符，按原语兴趣分类（mmio-write/dmem-store/dmem-imm/park-self/lcall/ret）。

结果（不告知已知位点）：

| die | 已知位点 | 自动命中 | miss 归因 |
|---|---|---|---|
| TU102 | 0x10B9, 0x1F89, 0xCBD, 0xC95, 0xE85, 0x7EA4 | **4/6** | 0x10B9/0xC95 = 中项入口，其操作码首字节（`04`/`00`）在规范指令流中从不出现→位置幼稚长度表天然覆盖不到 |
| GA102 | 0xD44, 0x1FCE, 0x5789, 0xD52, 0x1101 | **2/5** | 同上；另注：0x84C8/0x8E18/0xFFBC 是 DMEM/栈**操作数**而非代码入口，最初把它们列进已知集是我方归类错误 |

**归因即路线**：miss 集中于一个已知工程缺口——fuc5 没有位置无关的开源解码器封装。操作码权威在 `envydis/falcon.c`（C 表，fuc5/fuc6 变体齐全），一次性转录成 Rust（约数百条目）后，中项入口问题即消。envydis 保留为交叉校验 oracle。**结论：gadget 层是机械工作，不依赖灵感。**

## 4. 栈几何层（硬点，两条路线）

要推导的：签名 DMA 完成时刻的调用栈现场（live 帧槽位、尾声级联的帧步长序列、栈顶）。两条路线：

1. **静态帧分析**：从"读 WprMeta+0x50 作长度"的 DMA 发起点出发，沿调用链上溯累计帧尺寸，重建 0xFF00–0x10000 的槽位图。工具需求：跨 `lcall` 的帧尺寸传播——本质是过程间分析。
2. **仿真到断点**：fuc5 解释器跑到"签名拷贝完成"断点，直接读 $sp 与栈内容。工具需求：fuc5 CPU 仿真器（含 CRYPT 取指解密语义、DMA 模型）。

**faucon 评估（纠正此前假设）**：`reference/faucon` 只有 assemble/disassemble/isa，**没有解释器/符号执行**。本地检出仅一个 commit（汇编器 parser，README 仅一行标题），作者路线图中的模拟器未落地；公开生态检索（2026-11-05）也未找到现成的 fuc5 执行环境——envytools 同样只有反汇编。所以路线 2 的仿真器要自研（fuc5 指令集小，`sim/` 有行为级基建可复用；faucon 的 isa/ 目录可作语义参考）或用 envytools 补。工作量估计：静态帧分析 1–2 周、仿真器 2–4 周——这是整个项目从"脚本"变"工具"的分界。注意栈几何问题**不需要完整机器模拟**：debug 明文喂入（免 CRYPT 取指解密）、MMIO 写全 stub、DMA 模型只做"拷贝即完成"，收敛为"窄解释器 + 桩环境"；且栈顶初始化本身零推导——booter stub 第一条指令即 `mov $r9 0xfffc; mov $sp $r9`（main SP ≈ 0xFFF0 直接读出）。

## 5. 尾帧语义：两档目标

- **最小链（write-and-park）**：两帧写完即落 park 自循环。gadget 需求最少（写中项+park），栈几何需求最少（只需 live 帧三槽+步长）。**代价**：booter 不再干净收尾——TU106 全 parity 链的成功日志是 `halted + exci=0`，最小链会停在自循环，loader 的 `verify_execution_tu10x` 判定路径必须重新设计（或按宪法"读回是唯一成功判据"绕过 booter 判定，直接以 FEAT 读回定案；FLR/Turing-no-FLR 语义也要重审）。
- **全 parity 链**：复现 mutex 释放级联（0xCBD/0xC95/0xE85/0x1F89 token 序），booter 走回自身收尾。需要尾声级联的完整仿真——依赖 §4 路线 2。

建议 G0 先做最小链（快速闭环验证栈几何推导的正确性），G2 再攻全 parity。

## 6. debug→prod 迁移性（诚实的风险面）

- **实证分歧点（本次调研新发现）**：NS 前置页在 debug 与 prod 构建间**代码骨架相同但两个 16 位立即数不同**（debug `0x1660/0x1668` vs prod `0x1650/0x1658`，疑似 DMEM scratch 基址）——debug≠prod 有先例，且恰恰是"参数类常量"。
- **不可比区**：SEC 代码区 prod 为密文、debug 为明文，逐字节比对在原理上不可行。
- **兜底**：payload 引用的 DMEM 常量（0x6140 canary、0x8E18 等）在 prod 上的正确性目前由"上游表实机有效"背书。**黄金标准复现实验恰好就是迁移性度量**：生成器只看 debug 明文，若复现出 prod 正确的三张表，即证明该 die 族的 debug→prod 几何不变式成立；若复现失败，差异立即定位到具体常量类。
- 前提条件：每个 die 族要有一把 debug key（TU=#6 族、GA=#37 族已破；AD/GA100 的明文已在库）。**没有 debug 明文的新族，生成器无输入**——这是硬边界，不是工程缺口。

## 7. 立项草案（G0–G3）

> **G0 已落地（2026-11-05，分支 falcon-booter-payload-gen，6d7a4dc/e75046d）**：
> - 解码器：falcon.c 18 表 + core-dis.c 引擎语义全转写 Rust（tools/falcgen）；线性解码在**五份解密 booter 段上与 envydis lst 逐条零分歧**（TU102/TU116/GA102/AD102/GA100，54,670 条）+ 360 个非规范偏移逐点对拍（env-gated oracle 测试）。
> - 扫描器：前向行走分类移植，**17/17 已知代码入口全中**——TU102 6/6（含 python 版 miss 的中项 0x10b9/0xc95）、GA102 5/5、AD102 6/6（0x31b5 status writer 形状正确：DmemStore+mailbox 立即数+LCall）。
> - **G0 实证发现（2026-11-05 勘误）**：初判"envydis 表有编码空洞"**有误**——盲区是 **CRYPT 特性位未开启**（lst 生成时无 crypt）；开启 `F_CRYPT` 后五段 **零未知指令**，SEC 入口页解码为 `csigclr` + csecret/cxor 清密钥序列，全部 ??? 簇都是 cxset/cocmd 族密码指令。扫描器/解释器已改用 FUC5_CRYPT。cmp50hx 的"gadget PC 处 archive 字节≠运行时字节"独立成立（不同密钥流）。同日落地 **G1 Track B 窄解释器**（src/interp.rs）：明文喂入、MMIO 桩、DMA-as-copy；bring-up 至 41K 条真实指令，**main SP = 0xFFF0 已由解释器推导**（与 nlzy 表 golden 一致）+ SEC 入口 SP init 0xfffc / canary 0x6140 / park 0x7ea4 机械化推导。当前 frontier：mpush/mpopaddret 族 ABI 无公开文档（envytools branch.rst 自注 todo），booter 自身 canary 存储与返回槽关系的 bring-up 迭代进行中。
> - 复评结论：G1 双轨并行（用户裁定）按原案启动；G2/G3 排程不变。
>
> **G1 Track B 重大推进（2026-11-07，497fc3b + 后续）**：解释器沿真实流程驱动至 XDMA 层，沿途修复**四个语义真 bug**（mpop 全恢复、adc/sbb 进位、ld/st 宽度、sethi 保低位）——每个 bug 都曾被流程当作"引导问题"而实际是模型错误。环境模型（全部从 booter 自身代码逆向，非猜测）：
> - **安全服务邮箱**（IO 0xd000-0xd500）：0xd000 = 就绪/错误位（0x1c0000 就绪、0xc00000 错误）、0xd100 = 提交（id|0x10000 = 读型请求；写型 = 0xd200 载荷 + 0xd100 id，参数存入服务存储供写-验证回读）、0xd400 = 门铃/应答、0xd500 = 响应值。旧 GetParam id 表（0xa-0xe 使能、0x11 期望值、0x6-9 签名半块）**全部在描述符块结构中得到解释**（见下）。
> - **PRI 读服务**（0xf27）：0x1c100 = 行选择、0x1c000 = 触发（0x800000f1 读/f2 写）、0x1c200 = 行数据——同时服务熔丝行和**活寄存器读**（WPR2 0x1fa824/28、FRTS 界 0x1fa81c/20、L2 熔丝 0x100ce0、修订对 0x20800/4）。
> - **描述符块**（DMEM 0x600，块搬运器 0x2d5e 按尺寸类抽取字段）：头 u16@+2=2、+0x10 ≤0x200、**使能位 +0x814-0x81c（b16，类 0xa-0xe）**、期望值表 +0x820（计数 ≤0xb @+0x878）、+0x880-0x890 五个 u32 字段（类 0x12-0x16）、+0x414/+0x614 = 签名半块（类 0x5/0x6，≤0x200）。旧 id 表的 ctx 即此块。
> - **meta 校验链**（状态码全部定位到检查点）：0x91 = (WPR2_HI-WPR2_LO)<<8+0x20000 == 1MiB（256B 单位）+ 0x11821c bit17；0x73 = L2 熔丝 (mask<<20)==meta.fbSize（TU102 = 11<<6）；0x8d = frtsSize 必须恰 1MiB + wprEnd-frts 关系；0x1b/0x1d/0xb 如上。**meta 种子已换为自洽的 TU102 11-GiB 布局**（fbSize=0x2c000000 驱动 L2 熔丝与堆公式）。
> - **XDMA**（拷贝驱动 0x8070/分块器 0x532）：$xdbase = 页基址寄存器，xdld/xdst = 0x100 字节/块（op0 = 页内偏移，op1 = (aperture<<16)|源偏移），xdwait 完成。RM 镜像装入 GSP DMEM（>0x10000 地址）、radix 经 0xf00000 窗口构建。**签名 DMA 调用点已定位**：0x3924 函数（dst=0x600、0x1000 分块、源 = meta+0x48/4c，0x39bb）与 0x3f87 主路径（0x40cd，dst=0x5d00，"FREE"/"PAEH" 魔数校验）。
> - 前沿：XDMA 模型落地后流程扰动（boot 镜像自装载覆盖 0x600 块——真实顺序=数据镜像含描述符块），快照点已通（sig-page 过滤 + 栈触及检测），待下一轮把装载顺序与描述符块来源对齐。

| 里程碑 | 内容 | 验收 | 估时 |
|---|---|---|---|
| G0 | 位置无关 fuc5 解码器（falcon.c 表→Rust）+ 扫描器完整化 | TU102 六站点 6/6、GA102/AD102 已知代码入口全中，envydis 交叉校验零分歧 | 3–5 天 |
| G1 | 栈几何推导（§4 路线 1） | 只从明文推导出 TU102 的 0xFF38/40/44/4C、0x18 步长、0x600/0xFFF0——与钉值表逐项对上 | 1–2 周 |
| G2 | 最小链合成 + 黄金标准复现 | 生成 payload 与 `b0ec9029` 逐字节相同（或差异项逐一解释为"已证无害"）；同法过 GA102/AD102 | 1 周 |
| G3 | 全 parity 尾帧（可选）+ TU116 首用 | TU116 表由生成器产出并与手工单字重锚版对拍；sim 全绿 | 1–2 周 |

宪法映射：G0/G1/G2 全程离线（零 GPU 接触）；产物进 `assets/`；黄金标准钉值进 `manifest.rs` 沿用现有 UNMEASURED→PIN 流程。模拟面先行（§1.8），无真机步骤。

## 8. 与 tu11x 便宜路径的关系

今日已交付的 tu11x（die 专属归档 + 手工单字重锚，f76c262/ceb40b4/aa7ab19）是**便宜路径**：它依赖"TU102/TU116 解密段逐字节审计"这种一次性人工工作。生成器是**通用路径**：G2 通过后，TU116 表改由生成器产出并与手工版对拍（G3 验收项），此后每个新 die 的边际成本 = 一把 debug key + 一条命令。两者不冲突——便宜路径先行验证了"帧结构跨 die 继承"这个生成器赖以成立的前提。
