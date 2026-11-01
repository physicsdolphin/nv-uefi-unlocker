# 同行生态全景对照（2026-11-02，reference/ 28 仓全量更新日）

> 调查轮：reference/ 27 仓全量 fetch/pull（前快照 2026-10-19 ~ 10-27）+ 新增克隆
> X-WIDE/X-WIDE-gpu-unlock-win（0b66dd9）与 impacifist/Windows-GPU-UNLOCK（fd8bbea），
> 删除重复克隆 nlzy-turing-unlock（与 turing-unlock 同 origin 同 HEAD 417f333）。
> 对照 `docs/totorong-comparison.md` 体裁扩展为全景版；P1~P5 分级见表 §4。
> 事实冲突裁决顺序照任务书 §10：open-gpu-kernel-modules 源码 > 参考仓实现 > 文档 > 记忆。

## §1 reference/ 更新日志（2026-11-02 执行）

有更新的仓（4/23，另有 3 特例）：

| 仓 | 前 → 后 | 新提交 | 要点 |
|---|---|---|---|
| gpu-unlock (Zagitta) | af28cbb(10-24) → 7eb51bc(10-28) | 14 | **Release 2.0**（0532473）+ recipe-driven 重构（09305c4）+ GA106M 2560（5b24f44，PR#160）+ 3070 LHR 2488（e9f9408）+ 3090 报告×4 + unraid 支持（1fc6d62）+ PMCSR D0 helper（ca78f02）+ sparse 修复 |
| KohakUnlock | 533013a(10-27) → a381091(11-01) | 22 | **TU104 2060 1e89 移植**（13a615a）、TU102 2080 Ti 1e04/子系统 12fa（effa3bb）、**免 U 盘 ESP+GUID 钉死载荷分区**四阶段（e0927d4→e309733）、KOH_AUTOCONFIRM_MS 自动确认（573212a）、driver_override 跨内核兼容（c160fe4） |
| nv_unlock_win (mirabarukaso) | f78a0fb(10-27) → a4c8b90(11-01) | 14 | loader 0.2.6→0.2.14 五连发：USB 引导列表（51cd967）、**3050 与未列名 Ampere 强注入拦截 + 支持表**（e4e6688）、**Ada 4080 MMA 验证**（2e7c95e，issue#3）、TITAN RTX/3070 Ti 实测 + 3070 Ti 黑屏记录（ded98be/1ee9dfd）、Hide-GpuUnlockEsp.ps1（55df5b9）、逐行时间戳（be6ef8d） |
| p104-gen2-unlock (skbskhud) | 2a97cb8(10-27) → 0bbfb29(11-01) | 23 | Linux 一键脚本发布后又**四连 revert 转私有仓**（d04fa90..3d10102）、v3 双仓发版（cc9a33a）、x16 状态公告+暂休（0bbfb29）、GPU-Z 闪断注记 |

无更新（仍为快照 HEAD）：170hx-rir-tool 03a612a(10-21)、AD103_win_unlocker 51df9c1(10-23)、
NvStrapsReBar 5ce12bb(2024-11)、ReBarUEFI 9c80fdb(2025-12)、cmp170hx d923a25(08-22)、
cmp50hx-unlock df2fe11(10-10)、cmp90hx-unlock 76940ae(08-22)、faucon e7297b9(09-08)、
ga100-fuse-re 7a10172(10-03)、gpcprobe 7aa144b(10-20)、gpu-unlock-ada d28c485(10-25，实测 origin==本地，无分叉)、
gpu-unlock-bar1 7053907(10-25)、gpuunlock d8f747d(10-27)、rtx30-unlock 5c076dc(10-21)、
rtx30-unlock-windows-toolkit 97edcb5(10-25)、rtx3080-unlock-public a08254d(10-19)、
tu102-22g-uefi-unlock 6ee61bd(10-25)、turing-unlock 417f333(10-26)、turing-unlock-haohervchb a188b24(10-22)。

特例处理：

- **open-gpu-kernel-modules**（shallow）：fetch 后 origin/main 仍在 **615.78.08**（ca2d03e，2026-10-07），
  最新 RELEASE tag 也是 615.78.08——**上游无新驱动版本**，作为环境常量记录（我方资产钉版不受影响）。
- **envytools**：本地 `nvbios/D.c` 存在 **BIT 'd'/DP-INFO 表解码增强**（176+/42-，重命名 unk2→dp_info 等），
  已 `stash push`（"local BIT-d/DP-INFO decode patch (pre pull 2026-11-02)"）保留；上游仍停在 2023（f102b82）。
- **travis-mcdevitt-fork**（分离头 2654419）：其独有提交（MSI TU102 guarded boot 证据，10-29）已捕获；
  fork 的 main 与上游（我方 56e200d）同步，无需动作。

## §2 同行普查（统一字段）

### 2.1 载具同型（Windows/UEFI 与 Linux 交付）

| 仓 | 载具 | 实机 ID（有证据的） | 冷态守卫 | 多卡 | 笔记本 | 文档 | 分发 | 本轮动态 |
|---|---|---|---|---|---|---|---|---|
| **windows-GPU-UNLOCK**（原 impacifist，已转组织，★1465，v0.3.1，fd8bbea） | UEFI app 三引导路（内置 ESP F8 headless 条目/无头 USB/常规 USB），wrapper+child，链回 bootmgfw | GA102：3090FE/3090Ti/3080-12G/3080Ti；GA104：3070Ti/3060-12G 2487/3060Ti 2489·24c9/3070 2484；GA106：2504 | PROM hash/PCI 段钉、冷态预检、BME、Esc 窗、22 项 USB 写门、一次性权限 | 显式不支持 | 否 | EN+KO | 源码-only tag + SOURCE-MANIFEST（924 文件 SHA-256）+ provenance 逐字节重建 + Discord | **不署名我们/chenxluo/TOTORONG**；兼容性收集流水线（issue 模板→离线 collect→逐机 LOCAL profile 生成） |
| **KohakUnlock**（a381091） | Linux kmod/initramfs（TU102）+ Windows EFI USB（AD102，衍生自 TOTORONG 51df9c1） | 2080Ti 1e04 22G×2（+85.3% mmapeak）、4090 2684；1e89 TU104（未验证，实验门）、1e04/12fa（有 @280W 网格 +74%） | 拒绝未知状态、编译期实验门 | **是（唯一 NCCL 双卡实证，64MiB 36.77GB/s）** | 否 | EN | 源码 only | 免 U 盘 ESP+GUID 载荷分区四阶段、KOH_AUTOCONFIRM_MS 自动确认、mmapeak 零输入坍缩方法论修正 |
| **nv_unlock_win**（a4c8b90） | EDK2 三族 EFI（分族三 app：Turing/Ampere/Ada，移植 nlzy/Zagitta/ada 三树） | 4080 2704（+63/66% MMA）、3070 Ti 2482（+78%）、TITAN RTX 1e02（满速，强注入无增益，诚实记录）；4090 数字**主动撤回待复测** | USB 引导列表（连 NVMe/SATA 控制器枚举内部 FAT 卷）、白名单拦截 GA106 2507/2508 与未列名 Ampere | 未见 | 未见（4080 是桌面） | EN+ZH | GitHub 源码+构建脚本 | 0.2.6→0.2.14 五连发：**预载 bootmgfw 交接**（0.2.12）、逐行时间戳+黑屏三重 flush（0.2.14） |
| TOTORONG（51df9c1，无更新） | UEFI app（HF 分发 **EFI+载荷二进制**——与我们资产纪律相反） | 4080 AD103 2704/1795（32G 魔改） | device+ROM hash 钉死、WPR2/state 门、FBPA 非致命 | 否 | 否 | EN+KO | HuggingFace | v0.4.18/RUN19 无新版本 |
| tu102-22g-uefi-unlock（chenxluo，6ee61bd） | UEFI app；**内置 ESP BootNext 一次性**（无常驻）或 USB 救援 | 2080Ti 1e04 22G（B350） | 容量门拒 11G、FWSEC hash 钉 | 否 | 否 | ZH | 源码+get-blobs.sh | 三处链回 bug 修复记录（RAM 载入→ExitBootServices 中止→改 DevicePath 文件载入等） |
| turing-unlock（nlzy，417f333） | **Linux kmod**（Arch/Deb/RPM 打包），cmdline opt-in | 2080Ti 1e07 Dell 22G（575.51.03） | 无冷窗（驱动后绑定） | 设计上支持（targets=） | 否 | EN | 发行版包、钉输入 SHA | pkgrel 2 |
| turing-unlock-haohervchb（a188b24） | EFI app 经**一次性 GRUB**（Linux 宿主） | 2080Ti 1e07/1043:866a 11G（580.178.04，Above-4G off+CSM on） | 严格状态拒绝；失败**驻留 UEFI 禁 OS 继续** | 否 | 否 | EN | 源码 only | 无新提交；矿卡负结果如实报告 |
| travis-mcdevitt-fork（2654419） | 同我们（fork） | **delta 仅 1 个文档提交**：MSI TU102 1e07/1462:371d guarded boot 三试（002 停在自己守卫 bug，003 成功） | guarded-auto 策略：入口 HALT、**不 SEC2 复位**、一次性权限 token、失败不重试不复位 | 同我们 | 同我们 | ZH+EN | fork | **新观察：TU102 FEAT 回锁值 0xffffffef（bits5/6），非 0xffffff8f** |
| rtx3080-unlock-public（a08254d，无更新） | UEFI app USB（ko 文档） | 3080 2206/1462:3890（+79.4/+85.7%，FP32 对照 -0.6%） | 设备/显示路径/PROM 检查 | 否 | 否 | KO | 源码 v0.10 | impacifist 的直接基座 |
| rtx30-unlock（craftingmod，5c076dc，无更新） | UEFI app；**Releases 直接分发编译 EFI zip + firmware-assets.exe 抓取器**（与资产纪律冲突） | 3090Ti 2203/7377:2000 | profile 钉 | 否 | 否 | EN | Releases 二进制 | 无新 |
| rtx30-unlock-windows-toolkit（Siselea，97edcb5） | 配套工具（包裹 craftingmod）：免 U 盘隐藏 ESP 首引导项（**常驻，违宪法**）、status.ps1 系统内校验 | 3060 2487（+58-85%）；**负结果：llama.cpp pp512 仅 +6.7%（Q4_K MMQ 为 FP16-acc 不受影响）** | 复用 wrapper 门 + 自检 | 否 | 否 | EN+ZH-CN | patches only | 单显示器补丁（免假负载）、fast-startup helper |
| X-WIDE-gpu-unlock-win（0b66dd9） | **自签内核驱动 gpuunlock.sys + 三计划任务 + GUI 面板**（testsigning+关 SB） | GA102 3090Ti（BF16 2.07×）；矩阵把 A6000 填进"真机验证"行（唯一基准是 3090Ti） | 关机事件禁卡→开机冷态事务→+5min 救援；脏关机绝不动硬件 | 未实现 | 否 | ZH+EN | 整合包 v3.1.1（站外，含证书+预构建 sys）+B 站 | **GA104/GA106/GA107"做不了"被多方证伪**（见 §2.4） |

### 2.2 工具/机制/研究线

| 仓 | 状态 | 事实 |
|---|---|---|
| gpuunlock（miounet11，d8f747d） | 无更新 | **手工合并 kmod**（Zagitta+ada 两树合一）+ arch/deb/rpm/DKMS + initramfs 集成；gpuunlock.com **7 语言站点** + 5 站友链环；4090 165.6→331.6 TF/s、2705 canary 95.0→189.9 |
| p104-gen2-unlock（0bbfb29） | **暂休** | P104 1b87 PCIe Gen1x4→Gen2x4（0.78→1.55GB/s）；物理 x16 判不可达；开发转 skbskhud/p104-unlock-v3（v3.3，20ms 看门狗宽度回训）；Linux 端四连 revert 后转私有仓 |
| gpcprobe（7aa144b） | 活跃 | 只读 BAR0 探测 kmod + GA100 floorsweep 熔丝解码 + inforom 黑匣子解码；**STATUS 影子==熔丝值的一致性校验法**（零写偏移表验证） |
| ga100-fuse-re（7a10172） | 无更新 | FF 池指令布局破解（chain[5:0]/offset[15:6]/data[31:16]，145 条无损重编码）；RIR 烧写清单勘误（网传 [2,6,7,10,12,13] 是错的，正解 [2,7,9,10,15]）；64→80GiB append 判 INCOMPLETE + NO-GO |
| 170hx-rir-tool（03a612a） | 无更新 | GA100 **线性 RIR 编码**（非 Maxwell 加扰）；**不可逆 OTP 烧写 disable_sw_override 已实证**（0x00950015→Row0/1 bit5）；自述 AI 辅助、要求审计写路径 |
| cmp170hx（d923a25） | 无更新 | 机制百科（10 篇 docs，EN+ZH）仍是 CMP 域权威 |
| cmp90hx-unlock（76940ae） | 无更新 | **XP3G 特权域**（PLM @0x8E1B0）：GA102 上主机写（含 V67 开 FEAT 后）与 SEC2 ROP 写全被拒；**TU102 可经合成签名写打开**（0x8E110/8E11C/8E12C/8E1B0）；XVE 镜像图修正一个 dword（LnkCap 0x88084/LnkCtl 0x88088/LnkCap2 0x880a4/LnkCtl2 0x880a8）；link-policy 寄存器组清单（0x8841C/88610/8C040/8C1C0/8C2C0/8872C）。我方 TU106 XP3G_PLM0=OPEN 与 TU102 逐 die 一致 |
| cmp50hx-unlock（df2fe11） | 无更新 | **334MB IDA 库 gsp_tu10x_610.43.03.elf.i64（恰为我方钉版）**+ 2981 行 CMP50HX.md；证据分级法（"I—IDA"=在开源二进制内核对过函数/指令/xref）；定位 stock RM 策略接线（pcie_apply_link_speed_policy @TU102 0x4CB25B8）；**前 OS 重训练挂死 X99/X299（issue#25）**——前 OS 不碰链路策略的实证理由；固件域签名注记（查 db，MOK 不放行固件启动的 EFI 项） |
| open-gpu-kernel-modules | 无新版本 | 615.78.08（ca2d03e，2026-10-07）仍为最新 RELEASE tag |
| envytools / faucon | 无上游更新 | envytools 本地 stash 保留 BIT 'd'/DP-INFO 解码补丁（176 行，显示链知识非解锁机制）；faucon e7297b9 新增汇编器前端 |
| NvStrapsReBar / ReBarUEFI | 无更新 | 2024-11 / 2025-12 冻结；仍为我方 §#11 BAR1 证据源 |

### 2.3 本轮新发现（GitHub/Reddit/Bilibili 全网扫描，2026-10-20~11-02）

**有实质信号的新实现（9）**：

1. aarvsn/gpu-unlock-windows（10-28，★0）——**WDM 下层过滤驱动**（LowerFilters 挂 nvlddmkm 下），重平台 Zagitta unlock_610.c，**resume 时重放事务**；声称 GA100/CMP40HX/GA10x/AD10x/TU102。零证据，单 commit。
2. xchgeaxeax/ada-unlock-kit（10-29）——Linux DKMS 四发行版安装器+GRUB 自愈；4090D FP8×2.18/FP16·FP32×2.00。
3. mitchross/talos-nvidia-gpu-unlock（10-31）——**Talos Linux/Proxmox** early-initramfs 新平台；3090 BF16+39.5%@250W；诚实注明"仅单卡初始化"。
4. Zihao-Wu06/gpu-unlock-for-windows（10-25，★10）——一键安装器：UEFI app 部署到隐藏 FAT32 分区逐引导重放；仅 40 桌面；4090 TF32 +55%；**一台 4080 黑屏交接失败有记录**；上游引 Zagitta/TOTORONG/brandonjank/gpu-unlock-ada（= JustAResearcher 真 fork，10-24，加实验性 Windows UEFI 端口）。
5. BrightRaider/ada-4090-universal-unlock（10-30，★2）——自称通用一键；4090 自测有对照，**Ampere 数字自认推算**；营销腔+LLM 润色痕迹。
6. NekoMirra/rtx4060-laptop-tensor-unlock（10-26，★17）——**AD107 28a0 4060 Laptop**（机械革命单机）；GEMM 10.8→49.7 TFLOPS；血统 MarvinFS/rtx4090-uefi←ada←Zagitta。
7. Q1y1ng/gpu-unlock-win（10-25）——**GA106M 3060 Laptop**（Legion Y9000P）UEFI 前置端口，进度日志齐全。
8. QingZhu233/tu102-windows-tensor-unlock（10-27）——TU102 2080Ti 22G Windows（C，gnu-efi），驱动 621.01/GSP 620.67.01，**VALIDATION.json 证据**，45.33→89.94 TFLOPS；致谢 nlzy/chenxluo/xrip/impacifist。
9. cgboss/rtx4090-laptop-ad103-unlock（10-28）——**AD103 2757 4090 Laptop**（与我们的 M18R1 同 ID）。

**新平台/打包类**：sw1313/synology-gpu-unlock-ada（群晖 DSM SA6400）、nonkronk+tabilzad 两只 Unraid 插件、Sticzu/nixos-gpu-unlock（flake）、Glat0s/gpu-unlock-windows-generic、tz61/gpu-unlock-ada-580-rtx4090（**驱动 580.178.04 移植**——对上 haohervchb 同版本）、MarvinFS/rtx4090-uefi（★10）与 PantACRO4life（★15）逐机 4090 口、wslgy X570 双 3080 20G 魔改、chungtair 免 U 盘声称、LightEssence "universal in-RAM engine"（未验证）、DaZuiZZZZ P106 Gen2/×16 两只（★7/★6）、skbskhud/p104-unlock-v3（★11）。

**仅报告/数据点**（无工程）：betizzel **首个 TU104 2080 Super 报告**、joogle8888 OCuLink eGPU 3080Ti、Dorlander 3070 Laptop GA104M MUXless、matrixxx1 4090 Laptop G834JYR、742645 3060 Laptop 115W VBIOS 覆写、hyls9527 4050 AD107M writeup、Ckurius 3070Ti Laptop（+50/+48% W-perf）、ThuGie 2080Ti Win11、hdcdn notes（含**辟谣小节**）等 14 个。

**垃圾/洗稿/AI slop 标记**：awujiana/bimbab189 两只 gpu-unlock-ada 脱离式复上传（fork=false 洗出处）、debughui-crypto/lemonaide152 标语复制、MindAllik "FMLA32"（非 NVIDIA 术语）、NoobUnderTreant DLSS5（域外）、SERU247153 "Rocky Linux Docker Windows 卡"（载具不可信）、Mustafa1p、iosis3、macky-droid 未审。

### 2.4 谣言勘误与媒体观察

- **X-WIDE "GA104/GA106/GA107 做不了（2026-11-01 实测不是猜的）"被多方证伪**：上游 Zagitta 稳定列表恰含 3050/3060/3060Ti/3070/3070Ti（100+ 报告）；impacifist 实机 GA104 3070Ti/3060-12G 2487/3060Ti 2489·24c9、GA106 2504；nv_unlock_win 实机 3070 Ti 2482 +78%。其"三源证据"实为同一血统（open-gpu-kernel-modules 610.43.03 的 BooterLoad 表只覆盖 TU116/TU102/GA100/GA102/AD102）——**把"我的抽取来源抽不到"误诊为"芯片族不可能"**，与我们的钉驱动包抽取管线及 V67 系实证矛盾。
- **VideoCardz（11-01）事实误传一处**：把我方 4090 级 FP16 165.6→331.6 TF/s 数据写成了"RTX 2070 FP32 从 165.6 到 331.6"（2070 FP32 峰值 ~7.5 TFLOPS，明显张冠李戴，数据实际源自 gpuunlock.com 的 4090 FP16 表）；另有"Secure Boot 风险因工具修改固件级设置"的口径失真（我方为易失覆盖+每引导重放，不改固件）。
- **B 站生态信任事件**：上传者苦楝籽连发两条撤 retract（BV15Vx36WEDW 10-26、BV1JQsX61EY7 10-28，"内容有误/乌龙抱歉"）；AI小蔡狗 40 系解锁视频 BV1iHxM6nEjE 达 83,926 播放——中文生态热度与信息质量同步上涨。
- **impacifist/windows-GPU-UNLOCK 不署名上游**：README/provenance 全文 grep 仅致谢 Zagitta 与 rough2515-hub，未见 chenxluo/TOTORONG/我方（README.md:260 + release/provenance.json）。

### 2.5 我方媒体波收录状态（本轮普查核实）

已收录（普查前）：TechPowerUp 354316（EN）、wccftech（EN）、r/overclocking 1xj5jp3（EN）、搜狐（ZH）。
**本轮实际追加**（README.md + README.en.md，rebase 合流后）：VideoCardz（EN）、WindowsForum 450453（EN）、Notebookcheck（EN；zh 侧原本缺失）、Hacker News Show HN（EN）。**Tom's Hardware 由并行会话先行收录**（合流时去重保留单条）；并行会话同期另补 PixLib（RU）、ShiftDelete 论坛（TR）、网易二条（ZH）、X/xiax0603（ZH），并把 Instagram 条目改标 ZH-HK。
**存在但 URL 未核实，暂不收录**：快科技/mydrivers（ZH，11-01）、新浪财经（ZH，11-01）——核实法要求逐条 URL 落实后再入表。
## §3 差距清单（五维，只列核实过的真实差距）

**① 芯片覆盖**：上游 recipes.c 理论表 87 ID（TU1x 19/GA10x 51/AD10x 17）远大于我方实机 22；但 2.0 重构把 GA106M 2520/2560 与 GA107 2582/25a2 从支持面**丢掉**（PR#160 只进了旧 main.c，refactor 未搬入 recipes.c，#238 用户靠手补一行宏）——逐 ID 表的维护性回归，反衬我方全段放行+映射表的稳健。上游 Turing **零实机报告**（#238）；我方 Turing 是全网唯一有 GSP 模式纪律+取证矩阵+Windows 实机（1f07/1f06 + fork 侧 MSI TU102）的深度线。笔记本：上游新增 27e0/2860/25a2/24dd 等报告，我方对应 ID 仍是 🟡/待复测（2560 复测、2507/25a2、2820）——**追赶点**。
**② 冷窗质量**：各家守卫同构（PROM 钉/冷态预检/BME/脏关机不动硬件）；我方 WPR2/EALREADY/EBUSY + PM preflight + warm 选择性接受仍是独有层；Linux 侧上游有 resume 自动重放（`gpu_unlock.resume`，休眠不覆盖）而我方 Windows 侧=ssdiag --reapply 手动——**Windows 侧自动 resume 重放无人做到**（aarvsn 声称未验证）。
**③ 载具鲁棒性**：**无头/无 GOP 场景同行领先**——impacifist 有无头 USB 路径+F8 测试引导项、KohakUnlock 有 headless-display PR、Siselea 有单显示器免假负载补丁；我方 gop_belongs_to_target END 节点 bug（pr13 已知）+无 GOP 方案未产品化。多卡：Kohak 唯一 NCCL 双卡实证；我方多卡=顺序解锁+tx-select-bdf（无双卡数值实证）。TB4 低位 staging 仍独我方。
**④ 分发与生态**：**impacifist 社区流程规模领先**（12 天 ★1465、Discord、issue→离线收集→逐机 LOCAL profile 生成→22 项门→USB 的支持性机器、SOURCE-MANIFEST 924 文件清单+provenance 逐字节重建证明）；gpuunlock.com 7 语言站点+友链环；上游 Linux 打包生态广（arch/deb/rpm/unraid 插件+CI，周边有 NixOS/群晖/Talos/Unraid 移植）；我方在**中文深度、12 语言媒体收录、逐 issue 取证文档**领先，但无 Discord/无 profile 生成管线。上游 FAQ 把 Windows 用户指给 impacifist 并警告"四处流传的安装器"——第三方整合包正在分走 Windows 用户。
**⑤ 验证纪律**：impacifist 的受控锁定基线+维护者/贡献者测试区分+逐字节重建证明值得学；Kohak 抓出上游 mmapeak 零输入坍缩——基准方法论审计有价值；X-WIDE（A6000 填充+GA106 错判）、BrightRaider（Ampere 数字自认推算）是纪律反例；媒体误传（VideoCardz 把 4090 FP16 写成 2070 FP32）说明对外口径要盯。

## §4 可采纳项分级表

> P1=立即采纳；P2=应采纳；P3=实验/观察；P4=仅记录不采纳；P5=明确不采纳（反例/违宪法）。代码级采纳一律另行立项，本轮只做登记与文档级落地。

| # | 来源 | 事实/长处 | 我方现状 | 采纳形态 | 成本 | 优先级 |
|---|---|---|---|---|---|---|
| 1 | impacifist | SOURCE-MANIFEST.json（924 文件 SHA-256）+ provenance.json（成功 EFI 逐字节重建证明）+受控锁定基线模板 | release 有 SHA256SUMS 但无 manifest/重建证明 | 发布工艺增补 manifest+provenance（另立项） | 中 | **P1** |
| 2 | 上游 #28（3615074213） | 独立复现我方 #88/#61：X99 AMI CSM-on→Code 12、CSM-off 正常；板级天花板 1024 vs 846 FLOP/SM/clk | Code 12 定律已定（#35/#61） | 归档旁证进 community-reports/docs（本轮文档级落地） | 零 | **P1** |
| 3 | travis 2654419 | MSI TU102：FEAT 回锁值 **0xffffffef**（bits5/6，非 0xffffff8f）；guarded-auto 策略（不 SEC2 复位、一次性 token、失败驻留不重试） | cold-values/open-questions 未记录该回锁值方差 | 补录 cold-values.md + open-questions（本轮） | 零 | **P1** |
| 4 | cmp90hx-unlock 76940ae | XP3G 特权域 @0x8E1B0 全图：GA102 全拒（含 SEC2 ROP）/TU102 可开（合成签名写）；XVE 镜像修正图；link-policy 寄存器组清单 | TU106 XP3G_PLM0=OPEN 已观察但无可写性验证；XVE 偏移无记录 | M1 采样扩展（只读，无门控）+cold-values 补录（另立项采样） | 低 | **P1** |
| 5 | 本轮媒体核实 | VideoCardz/WindowsForum/Tom's Hardware/Notebookcheck/HN 五条 EN 条目 URL 已核实 | README 缺这五条 | **本轮追加** README 列表 | 零 | **P1** |
| 6 | nv_unlock_win 0.2.14 | 黑屏场景逐行时间戳+**三重 flush**（注入前/注入返回/StartImage 前） | flush 旋钮存在（#97）但无边界强制 | 登记为实现任务（关键边界强制 flush，另立项） | 低 | **P1** |
| 7 | nv_unlock_win 0.2.12 + TOTORONG | 预载 bootmgfw（事务前 LoadImage）→…→StartImage，实证可行 | totorong 对照标"可选优化"未做 | chainload 预载立项（消除事务后分配失败面） | 中 | **P2** |
| 8 | impacifist+Kohak+Siselea | 无头 USB/F8 测试项/headless-display PR/单显示器免假负载补丁 | gop_belongs_to_target END bug 已知、无 GOP 方案未产品化 | 无头/单显路径立项（连带修 END 节点比对） | 中高 | **P2** |
| 9 | cmp50hx-unlock df2fe11 | GSP-RM TU10x **610.43.03** IDA 库（恰同钉版）+"I—IDA"证据分级法 | RE 工具链无 GSP-RM 库 | 本地引入（334MB LFS 不入库）+分级法写进 RE 惯例 | 低 | **P2** |
| 10 | 上游 kmod resume 语义 | resume 自动重放（+1s）、D3cold 旋钮 NVreg_DynamicPowerManagement=0x00、休眠不覆盖 | RTD3 擦解锁已定（issue#9），Windows 有 --reapply | docs 交叉引用 Linux 经验（本轮一句话落地） | 零 | **P2** |
| 11 | KohakUnloak 方法论 | 上游 mmapeak 编译期零输入坍缩——基准工具可能虚高 | README 口径用系统级实测（张量~2.6×） | 基准纪律补一句教训（本轮） | 零 | **P2** |
| 12 | KohakUnlock | 唯一 NCCL 双卡实证（2×2080Ti 22G，64MiB 36.77GB/s） | 多卡顺序解锁无双卡数值实证 | 列为未来实验候选（需双卡机器） | 高 | **P3** |
| 13 | aarvsn | WDM 下层过滤驱动+resume 重放声称（零证据，★0） | — | 观察其是否出证据 | 零 | **P3** |
| 14 | 上游 2.0 回归 | recipes 丢 GA106M/GA107 条目，用户靠手补 | 全段放行天然免疫 | 观察上游修复；作为全段放行论据记录（本轮已写） | 零 | **P3** |
| 15 | 上游 #238 笔记本新增 | 27e0 首报/2860 D3cold/25a2/2560 SB-on 成功 | 我方对应 ID 🟡 待复测 | 排进复测队列（2560 优先） | 中（需实机） | **P3** |
| 16 | X-WIDE 载具 | 三计划任务+脏关机守卫"屏幕一定回来"的恢复措辞；rescue 分层 | — | 记录措辞与任务编队形态 | 零 | **P4** |
| 17 | chenxluo | 内置 ESP **BootNext 一次性**安装（无常驻项）；三处链回 bug 修复记录 | 交付物=USB（#73 裁决不动） | 仅记录生态形态；链回 bug 教训归档 | 零 | **P4** |
| 18 | 170hx-rir-tool/ga100-fuse-re | **不可逆 OTP 烧写已在生态内实证**；FF 池/RIR 布局破解；RIR 烧写清单勘误 | 宪法 §1.6 永不烧写 | 宪法引文补一句生态现实（本轮） | 零 | **P4** |
| 19 | X-WIDE "GA104/106/107 做不了" | 被上游+impacifist+nv_unlock_win 多方证伪（§2.4） | — | 反例存档，答复社区引用时注明 | 零 | **P5** |
| 20 | X-WIDE A6000 行 | 专业卡无钳制（上游原话 "runs at full rate"），预期增益≈0，无 A6000 实测证据 | 我方矩阵只收有解锁收益的卡 | 反例存档 | 零 | **P5** |
| 21 | X-WIDE 载具前提 | testsigning+关 SB+预构建 sys 分发（pmxdrv/#40 定谳同构） | 宪法 §9 冲突 | 明确不采纳 | 零 | **P5** |
| 22 | TOTORONG/craftingmod 分发形态 | 直接分发 EFI/payload 二进制 | 宪法 §1.6 资产纪律 | 明确不采纳 | 零 | **P5** |

## §5 覆盖矩阵裁决：「是否已是全网最大的覆盖矩阵兼容生态」

**口径 A——单项目实机验证 device ID 数（带可归档证据）**：我方 **22 个 ✅**（Ada 12：2684/2689/2704/2705/2709/2757/2783/2803/2805/28a0/28a1/28e0；GA10x 8：2206/2216/2420/2489/249d/2504/2520/2484-仅EFI；Turing 2：1f07/1f06），外加我方血统 fork 的 MSI TU102 1e07 guarded 实机（2654419）。同行单仓最高为 **impacifist ≈9 个**（GA102×4/GA104×4/GA106×1，维护者+贡献者分级证据）、KohakUnlock 4 个（含双卡）、nv_unlock_win 3 个（含负结果）、TOTORONG 1 个。→ **该口径下我方居首，且是唯一三族（Turing/Ampere/Ada）单工具全覆盖并各自有实机锚点的项目**。

**口径 B——社区报告总量**：上游 Zagitta "100+ 用户报告"（Ampere stable，50+ 条 #238 讨论、驱动跨 550–615、含 SB-on 成功与 8×4090/多卡 Proxmox），报告条数领先但 Turing 零、证据为论坛自述级；我方 README 收录 38 条媒体/12 语言+community-reports 24 行实证表。→ **报告总量口径上游更大，但证据密度与 Turing 深度我方独占**。

**口径 C——理论支持面**：上游 recipes 87 ID（逐 ID 表，Release 2.0 重构时丢过 GA106M/GA107 条目）vs 我方全段路由（0x1e02..0x1fff|0x2182..0x21d1 / 0x2200..=0x223f|0x2400..=0x25ff / 0x2600..0x28ff；**0x2240..0x22ff 音频/Tegra/H100-NVSwitch 尾段已于 2026-11-02 按 pci.ids 剜出**（cc32265，纵深防御——class-03/function-0 枚举过滤器本就跳过它们），ES/原型 SKU 显式列出（a4f7a06），无零售名 Turing bare-die/内部板命名补齐（e890f18））——**理论路由宽度我方更大（全段+纵深防御 vs 逐 ID），实机验证深度我方更大（22 vs 上游 Turing 零/GA10x~AD10x 约 20 个有报告的 ID）**。

**结论**：「全网最大覆盖矩阵兼容生态」在**口径 A（实机证据矩阵）+ Turing 深度 + 三族单工具**意义上**成立**；不成立的维度要如实说：**社区运营规模**（impacifist ★1465/Discord/逐机 profile 管线）与 **Linux 打包生态宽度**（上游发行版全家桶+周边 NixOS/群晖/Talos）被同行超越；我方的不可替代层=取证纪律（evidence-as-commits、0x91 矩阵、WPR2/守卫语义、ssdiag 系统内读回、TB4 低位 staging）、中文生态深度、以及 Turing-GSP 这条全网唯一有人走通的 Windows Turing 线。风险项：impacifist 不署名上游血统（含我方）且正在收编 Windows 用户心智，上游 FAQ 又把 Windows 用户导流给它——Windows 端话语权竞争加剧。

## §6 本轮已落地（文档级）

1. README 媒体列表中英双语追加（rebase 合流去重）：VideoCardz/WindowsForum/Notebookcheck/Hacker News 由本轮核实追加；Tom's Hardware/PixLib/ShiftDelete 论坛/网易×2/X-xiax0603 由并行会话收录（见 §2.5）。
2. 论文引用修正：docs/credits.md 与 docs/cmp-gen2-sm-unlock-assessment.md 的 Pry canary 论文条目补全题名/日期/双 DOI（概念 20916111 / 含 debug booter 版本记录 20916112）+ ResearchGate 链接（见 §7.1 注）。
3. 本报告归档；reference/ 更新与特例处理见 §1（envytools 本地补丁入 stash、重复克隆已删）。
4. P1 中 #1/#3/#4/#6/#8 的代码/采样级落地均**另行立项**，本轮不动 loader/sim/assets。

## §7 全景时间线（公开参照版）

> 用途：给后来者一张"NVIDIA 寄存器算力解锁"生态的演进图。日期一律以可归档证据（Zenodo DOI / 仓库首提交 hash / tag / 带日期报道）为准；口说日期不录。

### 7.1 主线四阶段

| 阶段 | 时间 | 事件 | 证据 |
|---|---|---|---|
| ① 机制论文 | **2026-06-26** | Jon Pry《A Canary in the Crypto Mine: Defeating Stack Protection in a GPU Secure Coprocessor》上 Zenodo：出厂 GSP debug booter 弱加密 + 栈保护击破，普通 root 主机完成 **FLR 存活的持久解锁**——V67 canary 理论源头（概念 DOI 10.5281/zenodo.20916111，含 debug booter 版本记录 20916112） | [Zenodo DOI](https://doi.org/10.5281/zenodo.20916111) · [ResearchGate 408132536](https://www.researchgate.net/publication/408132536_A_Canary_in_the_Crypto_Mine_Defeating_Stack_Protection_in_a_GPU_Secure_Coprocessor) |
| ② 数据中心/矿卡算力解锁 | 2026-07-31 | CMP 170HX（GA100=A100 同硅）公开解锁线成型：Falcon BootROM `.fwsignature_ga100` 先载后验 → ROP → HS 写 PLM → 64GB HBM2e/满算力（cmpunlocker，Consensus-Protocol）；「Liberating the A100 beast」口径出圈 | cmp170hx 技术文档 README 07-31 版（本地 HEAD d923a25 08-22 加中文化 PR#1） |
| ③ 机制深化与矿卡外溢 | 2026-08 ~ 10 | 08-18/08-21 hyper.ai、irz.fr 报道；08-22 cmp90hx-unlock（GA102 V67 链 + **XP3G PLM 发现**）与 cmp170hx 文档站同期；09-12 LTT Labs 170HX 文；09-18 gpcprobe；10-02 ga100-fuse-re（FF 池/RIR 布局）；10-10 cmp50hx-unlock IDA 库线 + 170hx-rir-tool（**不可逆 OTP 烧写实证**）；10-17 p104-gen2-unlock 公开（P104 Gen1x4→Gen2x4）→ 10-28 v3 双仓 + DaZuiZZZZ P106 Gen2/×16 两仓——**矿卡 PCIe 解锁线** | 各仓首提交 hash（见 §1/§2.2） |
| ④ 消费级解锁高潮 | 2026-10-19 ~ 11-02 | USB/UEFI 预驱动载具爆发：两周内 ≥25 个公开实现/移植/打包仓 + 我方一线五版本 + 上游 Release 2.0 + 主流媒体四语种报道波；逐仓节点见 §7.2 | 各仓 tag/首提交 |

注：坊间有「4 月论文」之说，公开检索（arXiv/Zenodo/ResearchGate）**落实到的最早机制论文即上述 06-26 Zenodo**；若另有所指（如中文社区私有渠道），拿到出处即可续表，宁可空一格也不写无证据日期。

### 7.2 消费级高潮逐仓节点线（高竞争力 repo）

| 仓 | 节点线（均 2026 年） |
|---|---|
| **windows-GPU-UNLOCK**（impacifist，★1465） | 10-19 基座 rough2515 v0.10（3080 2206 首报）→ 10-20 **v0.1.0 首发**（9c54159）→ 10-23 v0.2.0（英文化+兼容上报流程）→ 10-24 v0.3.0/v0.3.1 → 11-01 fd8bbea（PR#139 MSI 3080Ti）+ 转组织运营。12 天 118 commits/139 PRs |
| **gpu-unlock**（Zagitta，Linux kmod 权威） | 本地浅克隆自 10-24（af28cbb，更早史不在本地）→ 10-26 GA106M 2560（PR#160）+ 3070 LHR 2488（PR#140）+ 3090 报告×4 + recipe 重构（09305c4）→ 10-27 **Release 2.0**（0532473）+ unraid 插件 + #238 成功报告帖 → 10-28 sparse/PMCSR-D0 修复。100+ 用户报告，Turing 零 |
| **KohakUnlock** | 10-25 首发（guarded TU102 研究端口 bc0c727）+ TU104 2060 1e89 移植（13a615a）→ 10-27 headless-display PR#7（533013a）+ 免 U 盘 ESP/GUID 载荷分区四阶段 + auto-attempt → 10-29 TU102 2080Ti 1e04/子系统 12fa（effa3bb，@280W 网格 +74%）→ 11-01 双 PR 合并（1e89/driver_override）。唯一 NCCL 双卡实证 |
| **nv_unlock_win** | 10-27 0.2.4（f78a0fb）→ 10-28 0.2.6 USB 引导列表 + 0.2.7 隐藏 ESP 脚本 → 10-29 **0.2.12 Ada 4080 MMA 验证**（2e7c95e，+63/66%）+ 0.2.13 白名单拦截 3050/未列名 Ampere → 10-30 0.2.14 逐行时间戳+三重 flush → 11-01 TITAN/3070Ti 口径修订 |
| **TOTORONG**（AD103_win_unlocker，HF） | 10-23 v0.4.18/RUN19（3ac2fee 首提交同日）——AD103 Windows EFI 先例 + **唯一二进制分发先例**；此后无更新（单卡实验收束） |
| **gpu-unlock-ada**（JustAResearcher） | 10-24 4e7fefc（AD102 四写链+验证纪律；PR#6 已合）→ 10-25 笔记本功耗基准修（d28c485）；其 brandonjank fork（10-24）开 Windows UEFI 实验支线 |
| **X-WIDE-gpu-unlock-win** | 10-31 一小时 7 commits 冲出（自签内核驱动 gpuunlock.sys + 三计划任务 + 面板 v3）→ 11-01 整合包 v3.1.1 + 「GA104/106/107 做不了」矩阵（**被上游/impacifist/nv_unlock_win 多方证伪**，见 §2.4） |
| **我方 nv-uefi-unlocker** | 10-26 首提交（1767113，M0-M4 全量落地）→ 10-28 **v2.1.0–v2.1.4 五连**（GA10x 全段放开+A6000/3050A 类内显卡修正）→ 10-29 v2.1.5（GA106 2504/2520 首批全链，#70/#71）→ 10-30 **v2.1.6 stable**（Code 12 定律收官+非递归重连，PR#66）+ TechPowerUp 报道 → 10-31 v2.1.8（GSP-HALT 门默认+FWSEC 低位 staging 默认，#14/#16 定案）→ 11-01 v2.1.9-rc1（WprMeta 转储+VRAM staging 载具，#89）+ VideoCardz/Tom's 报道 → 11-02 本轮普查（3747e3f）。**7 天 9 个版本 tag、22 实机 ID、12 语言报道** |

### 7.3 读写这张图的三条规律

1. **理论先行，产品滞后 3 个月**：06-26 论文 → 07-31 矿卡闭环 → 10 月消费级爆发；机制（canary/ROP/PLM）全程未变，变的只是载具（kmod→GRUB→UEFI USB→内核驱动）与工程化（守卫/白名单/日志/兼容上报）。
2. **算力解锁与 PCIe/容量解锁是两棵树**：同源于 Pry 论文，但速率钳（消费级 tensor）、容量钳（170HX 显存）、链路钳（矿卡 Gen2）由不同 PLM/熔丝域把守（cmp90hx XP3G、ga100-fuse-re 分别证实 Gen3 与容量是另一堵墙）；跨树移植不成立（本报告 §2.2 XP3G 行）。
3. **分发模式在分岔**：源码+钉输入自建（我方/上游/Kohak）、二进制直发（TOTORONG/craftingmod/X-WIDE 整合包）、社区流水线（impacifist LOCAL profile）——三者合规与安全姿态差异极大，上游 FAQ 已公开警告来路不明的 Windows 安装包。
