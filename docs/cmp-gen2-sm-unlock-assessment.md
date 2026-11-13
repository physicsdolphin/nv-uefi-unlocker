# 功能评估记录：CMP PCIe Gen2 / SM 开核 / AD102 BAR1+L2（2026-11-01）

三个新功能方向的可行性判定。来源参考：本地 `reference/cmp170hx`（机制百科）、`reference/cmp50hx-unlock`（Turing 实证链）、`reference/cmp90hx-unlock`（GA102 分析）、`reference/ga100-fuse-re`（熔丝布局）、`amoghmunikote/cmpunlocker`（PR #55）、`aajax.top` AD102 L2 研究。判定纪律：读回是唯一判据；改常量不构成已验证移植。

## 1. CMP 矿卡 PCIe Gen2 解锁 —— 落地但需探索（本轮不开 PR）

**机制已完整实证（Turing 侧）**：cmp50hx-unlock 的 `04-cmp50-pcie-gen2.patch` 在真机（TU102 50HX）跑通，寄存器集与 GA100 同构：

- 载荷内（exploit 窗口）打开 **XP3G PLM0 `0x8e1b0` ← 0xffffffff**（cmp50hx patch 01 的 0xBB7C–0xBB98 帧；patch 04 依赖该门保持开）；
- 主机侧策略写（patch 04 adopt 循环，全部带读回验证）：`0x8841c` PRIV_MISC_1（bits 11|13 置、12|14 清）、`0x8e120` VAL0=0、`0x8e110` OVR0=1、`0x88610` VSEC_HIERARCHY（bit12 清 bit0 置）、`0x8c2c0` CYA_0 清 bit2（DIS_G2 chicken bit）、`0x8c040` LINK_CONFIG_0 MAX_RATE[19:18]=2、`0x8c1c0` PL_LINK_RATE[19:18]=0x40000（Turing 编码，注意与 GA100 的 0x00240036 整值写不同）；
- LTSSM kick `0x8872c`←6；配置空间 LNKCTL2 TLS=Gen2（GPU+上游桥）→ 上游桥 RL bit → 轮询 LNKSTA CLS（**LnkSta 才是判据，LnkCap 会说谎**）；
- Turing 的 VAL3/OVR3（0x8e12c/0x8e11c）经 A/B 冷启动证明**不需要**。

**我们 loader 的架构优势**：pre-OS 窗口无 Linux 侧的 transient-window/hammer 竞态；Turing 尾段**无 FLR**（FEAT PLM 不过 FLR，v2.1.8 已证），策略写放在 relock 后、handoff 前即可整体持久进 Windows。170HX 的锤击先例证明"窗口期内训练成 Gen2 的链路在窗口关闭后保持训练态"。

**卡点（本轮不可落地的原因）**：XP3G 门必须由载荷内的链式写打开，而 nlzy 链帧布局在 0xFA00 载荷内**无 24 字节余量**——帧区 dmem 0xFF38–0xFFF8 共 192B 已用 184B，尾部被钉在栈顶 0x10000 之下，第三帧必与尾部碰撞。cmp50hx 的 stockflow 载荷用的是 pivot 重排（帧挪到 0xBB00 区 + stack-safe pivot 0x64EC + 自带 verifier 尾），是另一套 ROP 结构，整包移植需要逐 gadget 审计（宪法 §9.1）。双发 booter 在 AD102 上有第二次调用超时先例（bar1 报告），不能默认可行。

**落地路径**：① envdis 审计 cmp50hx stockflow 载荷（gadget 序、SS1 是否在、尾帧对驱动 sanitize 流的依赖）或参数化 nlzy 生成器+pivot 重排；② loader 加 Gen2 步骤（策略写+读回+重训练，全部照 patch 04 的 policy_ok 七项读回纪律）；③ **门控纪律：仅矿卡设备 ID**（30/40/50HX=1exx/21xx 段内，90HX=22xx），永不按"任意 Gen1 钳制卡"放宽。90HX（GA102）需另证：其 OPT_GEN23 eFuse bit0 已烧（"Gen2 disable"），但 170HX 同位读 0x1 仍被 XP3G 路径攻破，GA102 未测。170HX（GA100）需新家族移植（0x20xx 白名单排除是防错路由的有意设计）+ GA100 资产管线，独立立项。

**平台依赖**：BIOS 槽速需设 Gen2（PL_LINK_RATE bit20 上电态决定能力能否活过 GSP boot，X79 实证）；pre-OS retrain 在部分 Intel X99/X299 有挂死先例（cmp50hx issue #25）→ marker 门控+文档化回滚。

## 2. SM 开核（amoghmunikote/cmpunlocker PR #55 "+4 more SMs"）—— 不能落地

1. **PR #55 本身无公开验证**：README "What Gets Unlocked" 表没有 SM 行；无 issue 跟踪实测；blanket 0xff 写 8 个 GPC 被其 Copilot 评审标 high（无 SM 数校验）；PR 空描述、次日由仓库所有者自合。
2. **机制是 GA100 专属**：写 `0x820a40+gpc*4`（per-GPC OPT shadow override）、读 `0x820c38+gpc*4`、开 `0x8200d8` RECONFIG_PLM——GA100 fuse-block 布局，20–40 系不通用（Turing FEAT 在 0x4096xx）。
3. **决定性反面证据（同作者、同卡、同 SEC2 窗口）**：AD102 4090 48G 的 L2 slice 复活实验（aajax.top 2026-10-28）实测失败——**"effective disable = 熔丝层读出 | 软件掩码"，软件只能加禁用、清不掉熔丝层位**；开启全部软件掩码导致固件 FLR 后卡死。Ada 的 SM/功能禁用走同一熔丝层。
4. **良率切割 vs 分段切割**：170HX 的 +4 SM（若真）成立的前提是数据中心大芯片的禁用多为配置层"healthy-but-binned"；消费卡禁 SM 主要是良率切割（死硅），复活了也会 Xid/花屏。已有的 SS0/SS1 解锁才是"SM 单元解锁"在我们已覆盖的形态——吞吐钳制与 SM 数量是两个问题。

可做的安全第一步（探索项，非本功能）：ssdiag 只读枚举 20–40 系 OPT/fuse-shadow 区域（0x820xxx/0x821xxx/0x4096xx），建立 per-family 寄存器存在性表；并向 cmpunlocker 社区索要 +4 SM 的读回证据。

## 3. AD102 BAR1 64G —— 已落地（PR #95）

见 `feat-ad102-bar1-64g` / PR #95：四写链重参数化（CYA 0x30 / CFG 0x8000000a / XVE PLM / FEAT PLM + marker 0x67b00004），生成镜像与 gpu-unlock-bar1 八卡实证载荷公布 SHA-256（cf3566d0…）逐字节一致；marker `\ad102-bar1-64g.txt` + 2684 设备双门控；预检 CYA/CFG 未知布局即拒；bar1 专属判定集先于 four-plm 三件套硬校验；POST-FLR 记录构造存活。真机验证欠 2684 卡（M5 门）。注意：SEC2 必须直写 CYA/CFG（主机写即使开 PLM 也被拒，八卡实测）；Linux 侧桥窗口分配工具不随本 PR 交付。

## 4. AD102 L2 扩容（72→96 MiB）—— 不能落地

博客自身结论即失败：LTC/LTS 禁用=熔丝层|软件掩码，0x822080 只能 OR 进更多禁用；GA100 RIR 永久改写读出是唯一剩余路径=OTP 烧写，违反本项目"易失覆盖、每次引导重放、可逆"的纪律且有变砖风险（作者本人已停手）。不立项。

## 5. 原始论文评估：A Canary in the Crypto Mine（Zenodo 10.5281/zenodo.20916112 = 含 debug booter 的版本记录，概念 DOI 10.5281/zenodo.20916111，Jon Pry，2026-06-26；[ResearchGate 408132536](https://www.researchgate.net/publication/408132536_A_Canary_in_the_Crypto_Mine_Defeating_Stack_Protection_in_a_GPU_Secure_Coprocessor)）

CMP 170HX（=A100 同芯，三轴 fuse-crippled：SM 速率钳 1/32、显存 10→80GB、PCIe Gen1）的学术化拆解，CC-BY-4.0。与本项目相关的四点：

1. **金丝雀防御无效的更尖锐表述**：stack-protector 的参考字本身就在同一溢出可及的可写 DMEM 里——"secrecy and entropy are irrelevant"，无需猜测原值，同一次溢出同时覆盖参考字与保存槽即可通过校验。这把我们对 §7.4"均匀性攻法"（0xc0deca7e 覆盖全局+保存槽）从经验规律升级为机制解释，且跨代通用（Golden eye 序列的 booter 都用同族 stack protector）。
2. **HS 代码执行面是 booter 回归（regression）**：存在于 open-kernel-modules 时代的较新 booter、较旧 booter 无此洞——"version currency is not a security gradient"。对我们资产管线的含义：解锁面依赖钉版 booter 的具体版本，资产审计必须记录"该版本含无界签名 DMA"这一事实本身；钉版换版时把可利用性当一级属性复查。
3. **论文 defeated 的三轴 = 速率/容量/链路 Gen2，从未攻 SM 数量**：170HX 的 70-SM 配置不在攻击面内。这与本文件 §2 的判定互证——连最完整的学术处理也只解到 config 层的钳制（速率/容量/Gen2），熔丝层数量配置无人攻破；PR #55 的"+4 SM"仍是唯一无证据的例外主张。三轴 resisted（Gen3、on-die ECC、HBM retiming）中 Gen3 与 §1 一致；ECC 后来被 cmpunlocker 于 2026-10 经驱动补丁路线解决（3c52ad37），说明"resisted"≠"死"，但那条路是 GSP-RM 补丁，不是我们易失覆盖的形状。
4. **对 Feature A 的增益**：论文独立确认 Gen1→Gen2 是纯固件门（软件可破）且 2× 实测，Gen3 是另一堵墙（fuses+DevInit+GSP 补丁）；离线仿真确定性微码后构造"合法续延镜像"的方法论 = 我们 sim/ 先行纪律的学术同构。

结论不变：§1 探索、§2 不落地、§3 已落地（PR #95）、§4 不立项；论文为 §1/§2 提供机制层背书。

## 6. ssdiag v0.6.0 第一轮实测（TU106 1f07，2026-11-01，docs/evidence/tu106-1f07/ssdiag-v060-r3-tu106.log）

pcie/gen2 探测并集首轮（零写入，BAR0+ECAM 双传输）：

1. **bar1 构造值逐芯成立**：TU106 `CYA(0x88724)=0x08 / CFG(0x88dcc)=0x02`，与 AD102 stock（0x28/0x80000009）完全不同——PR #95 的预检门会在任何非 AD102 布局上正确拒绝，"未知布局直接拒绝"不是理论而是本轮实测。
2. **XP3G_PLM0 在 TU106 读作 0xffffffff（OPEN）**——与 50HX 卡的 0xffffff8f 相反；XP3G 状态/覆盖/值寄存器全部 PL0 可读（0）。PLM4/8/C、XVE_D0/D4/D8/C、AD1X_XVE_PLM 均为 badf5040；XVE_B（0x88ab4）可读 0xffffff8f。逐 die 的 XP3G/XVE PLM 页面映射差异是 Gen2 探索的下一轮输入：Turing consumer 的 XP3G 门可能无需载荷内打开（50HX 的 closed 是矿卡配置层的产物？待 40HX 卡对照）。
3. **OPT 熔丝区（0x820350/520/57c/580）+ GA100 SM banks（0x820a40/0x820c38）+ L2 掩码区在 TU106 全部 badf5040** = PLM 门控（寄存器解码存在，非 floorswept）——PL0 只读轮回答"被门控"，内容需 M3 窗口内读；PR#55 式覆盖寄存器在 Turing 的存在性判定要靠带窗读。
4. **XVE 镜像地址表双传输互证**：BAR0 LNKCAP=0x00453d03（Gen3 x16）与 ECAM LnkCap 解码一致；BAR0 LNKCAP2=0x0e（bits1-3）与 ECAM [Gen1,Gen2,Gen3] 一致。本轮快照 LnkSta=Gen2 x16（平台训练态，TLS=Gen3）——LnkSta/LnkCap/LnkCtl2 三寄存器在两个传输上逐位对上，XVE shadow 基址 0x88000 的镜像映射正确。

## 7. Gen2 可破、Gen3 不可破的机制分层（cmp170hx frontier/pcie-gen3-gen4 全文 + 24 条被驳方法目录）

**判据尺子：寄存器破解的威力边界 = "锁落在哪一层"。配置层钳制（DevInit 落进可重写寄存器）→ 可破（SS 速率 ✓ / Gen2 ✓ / 170HX 显存几何 ✓）；熔丝层配置（无写端口反射 / 签名固件 / RM 运行期重推导）→ 不可破（SM 数量 ✗ / Gen3/4 ✗ / L2 slices ✗）。**

Gen2 能破的三步逻辑：① Gen1→Gen2 的锁是**纯策略**——同芯 A100 跑 Gen4，5GT/s 模拟前端完好，DevInit 只是往寄存器写锁死表（LINK_CONFIG_0 MAX_RATE、CYA_0 DIS_G2 chicken bit、PRIV_MISC_1、XP3G 覆盖槽关）；② LnkCap/LnkCap2 是 **PHY 反射**，从策略配置再生，不直接读熔丝；③ booter 原语开 PLM 改策略 → 反射再生 Gen2。决定性证据：OPT_GEN23 熔丝位全程未清（写失败、无写端口），Gen2 照样成——熔丝只是被策略层消费的输入。

Gen3 不可破的四重独立墙（每重都有实测）：① **无写端口熔丝反射**——OPT_GEN3 0x820580=0x1（A100 全系=0）；LnkCap2 标 R-EVF 无写端口（"开 PLM 也没用"）；HS 权限把 XP3G PHY rate 强写成 Gen3 值读回成功、链路纹丝不动。② **允许代际掩码 RM 每次重训练重推导**——0x85084 [3:0] 由 GSP-RM 从 0x85080 重算（注入点读=BADF1100，HS 够不着）；LnkCtl2 被直接硬件掩码钳在 Gen2；round-3 伪造实验（LnkCap←Gen4 + LnkCap2←全向量）被硬件裁回 Gen2/0x6。③ **签名 DevInit 抑制位**——Gen4 编程块（写-only 0x88CE0/0x88CE4 + LTSSM 超时表）在镜像里存在但被表 +0x0F suppress 标志（CMP=0x01，A100=0x00）分支跳过；5 个 PCIe 字节 100% 落在 Davies-Meyer csecret(2) MAC 区间（2^128 二次原像，RSA 拒收=不启动）；strap 字段单调限制性（0=全开/3=钳位/0xF=非法——抬上限要写更小的值，无写端口）；FUSE_EN_SW_OVERRIDE=0+FUSE_DIS_SW_OVR=1（软件覆盖路径本身被熔丝关死）。④ **GSP-RM 策略层**——gsp.bin 内定位到熔丝读跳转表（li a2,0x580/0x57c），维护者定论"Gen3 需要 GSP 补丁"且"无人做出过可用的 GSP 补丁"（GSP 加密区不可读）。PHY 侧证据：Gen2-trained 卡 LnkSta2 EqualizationComplete-/Phase1-——Gen3 强制均衡从未运行（EQ 预设未被加载）。

被驳方法目录精华（24 条全文在 frontier 文档）：setpci LnkCap2（静默丢弃）、HS XP3G rate override（读回成功链路不动）、OPT_GEN23 三通道写（host/HS-ROP/booter 全败）、改 VBIOS devinit 5 字节（全在 MAC 区间）、刷 A100 VBIOS（实测失败）、A100 strap 复制（卡不识别）、redriver（只放大不伪造 TS1/TS2，须 retimer）、TOCTOU（Ampere 验签发生在 DMA 进 IMEM 期间、无窗口）、Device-ID 伪装（FUSE_DEVID_SW_OVR_DIS=1）、Gen4 fork 宣称（1 小时内被拆穿）。

未做路径（按成本序，全无结果）：写 OPT_GEN3=0 一行实验（预期失败但值一行记录）；MAGIC_D 可写性 A/B（证据矛盾——一份标注 writable、一份实测只读）；HS 上下文读 0x85080/0x85084/0x881C0（定位真正 sourcing 向量的层）；0x823830-3C 覆盖组 HS 写测试；forced Gen3 时 dump LnkSta2 EQ 字段（**唯一翻案窗口**：若 GEN3_DIS 是 boot 时锁进可重写 PHY config 而非直接进模拟 PHY，则存在 post-boot 寄存器可覆盖）；GSP-RM 补丁；retimer 硬件（Astera Aries/TI DS160PR810 级，双向伪造 Rate-ID）。
