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
