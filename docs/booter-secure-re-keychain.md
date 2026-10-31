# Booter 安全段 RE 钥匙链与状态码结构（W6 狩猎报告，2026-10-31）

> 目标：为 B 类（#57/#82 终值 0x0B/0x11）取得 booter 安全段语义。结论：TU10x 明文无公开来源；GA 系明文**现已可得**（公开 debug 密钥）；TU10x 密钥存在但未公开，线索在 Lapsus 泄露树。GA 系结构已给出 0x11 的阶段归属假说。
> 配套：[tu10x-booter-nsstub-disasm.md](tu10x-booter-nsstub-disasm.md)（NS 桩与 mailbox/EXCI 字典）。

## 1. 候选排查结论（全量）

| 候选 | 来源 | 安全段熵 | 判定 |
|---|---|---|---|
| `booter_load_tu10x.bin`（量产） | assets/out（钉版 610.43.03） | 7.994 | AES 密文 |
| TU102 `IMAGE_DBG`（610.43.03 tarball 内 bindata） | release/inputs | 7.994 | 密文——**TU 专属 debug key**（见 §3） |
| TU102/TU116/GA100/GA102/AD102 `IMAGE_DBG`（open-gpu-kernel-modules 新版） | reference/ | 全 7.99 | 密文；GA100/GA102/AD102 与公开 GA 系 key 匹配 |
| **GA102 `IMAGE_DBG` 解密后**（610.43.03） | 同 tarball | **6.915** | ✅ 明文 fuc5；sha256[0:0x8a00)=`bdb38e3e…` 与 gpu-unlock-ada 公布值一致 |
| **AD102 `IMAGE_DBG` 解密后** | 同 tarball | **6.832** | ✅ = TOTORONG 公布的 "Debug Plaintext IMEM"（10383 指令那份） |
| cmp40hx `booter_ucode_dbg.bin`/`gsp_rm_boot_dbg.bin` | reference/cmp50hx-unlock blobs | **0（全零）** | **占位文件，从来不是镜像**（sha256=59136 个零，与 cmp50hx README 记录一致——悬案结案） |
| `fwsec_50hx_dbg/prod.bin` | 同上 | 7.71/7.71 | 密文（dbg≈prod 头 32B 同） |

HF TOTORONG/AD103_win_unlocker 与本地克隆逐字节同版，无隐藏明文/key。

## 2. GA 系 debug 密钥（公开，已三重验证；**本仓库不收录密钥字节**）

- Key 出处（指针引用，宪法 §1.6"密钥永不入库"——密钥字节请从原 gist 获取）：gist `nvlight/fc7479629b0ace094873e58ba7d75eb2`（标注 "recovered by BlackSun"，gpu-unlock-ada `ada-port.md` 钉版收录第 "Debug csecret(37)" 行）。
- 独立验证控制值（拿到 key 后自验，本仓库不提供 key）：`AES(key, 00×16)=717d1494eaca317ff106195258b38377`（GA100/GA102/AD102 DBG padding 实测同值）；`AES(key, FF×16)=fd7e5b57768a309baabcc6903118c8f5`。
- 字节级终验：AD102 解密结果 @0x31b5 与 TOTORONG 发表的 status writer 逐字节相同；GA102 差异仅 canary 槽位 0x6140 vs 0x5f50（芯片差异）。
- 复现流程（未落盘、全内存）：bindata 数组（NvU8/NvU64，raw-DEFLATE+CRC 尾）→ 解出 IMAGE_DBG → 按 HEADER_DBG 字段 `[sec_off : sec_off+sec_size]` AES-ECB 解密（DMEM 不解、NS 前缀保留）→ entropy/SHA/控制值三验。**解密产物与全量反汇编列表只在本地工作目录，永不入库。**

## 3. TU10x 密钥（~~缺口~~ → **已破，2026-11-01，见 §7**）

- 验证标记 `24e89313dcbd280aa71a071bed8f53c1`（=AES(key6,00×16)）最初来自 TU102 DBG padding 频谱；**key=编号测试 key 族 #6**（cmp170hx csecret 表 secret(6)+shuttie published test-key family），TU102/TU116 共用。破过程与 TU 语义图见 §7；本节保留为历史记录。GitHub 全站码搜无命中（key 字节串从未公开提交过——公开的只有族结构与 KAT）。

## 4. 状态码写入结构（GA10x 明文实测）

- **status writer helper**：GA102 @`0x1d87`（16 指令）↔ AD102 @`0x31b5`（TOTORONG 已发表，逐字节同构）。入口惯用式 `mov $r10 <code>` → helper 把 `$r10` 写 **mailbox I/O 0x1000**（与 TU10x NS 桩 `mov $r15 0x31; iowrs I[0x1000]` 同一 I/O 口——跨族一致）。
- 调用点：GA102 26 处 / AD102 21 处（两族一一对应）。字面量码：`0x15`（最常见失败码）、`0x38`、`0xa6`（WPR 三连检查点 GA102 0x6bbf/0x6c17/0x6c6f）。**无 0x0B/0x11/0x91**——TU10x 码表独立。
- **cmp170hx wiki（GA100 515 期带符号反汇编）记载**：`wpr_desc_validate` 错误码 **0x89–0x90**；`tgt_falcon_bringup` 含 **0x11**。

## 5. 对 B 类（#57/#82）的含义

1. **0x0B/0x11 的语义（v3，已被 §6 递归下降修正）**：GA100 wiki 说 0x11∈`tgt_falcon_bringup` 的方向正确（meta 过验之后的签名验证域），但机制不是"阶段索引"——§6 证明 0x2b69 是 **GetParam（验签结构参数访问器）**，0xb/0x11 是**验签参数空间的 id**（0xb 同时=PSS 终判哈希不等的错误码）。若 TU10x 同构，#57/#82 的 0x0B/0x11 意味着 **meta/WPR2 校验已通过、死在签名页 DMA→PSS 验签→期望值对账域**——与 #74 实测的"签名/radix ≥4GiB → 挂死"同域（#74 挂死不报码、#57/#82 报码停机）。早期"阶段索引/早期拒收"两说均作废。
2. 近程判别仍靠现场：`\tx-bootvec-sec.txt`（桩 vs 安全段）+ v2.1.8 默认轮；**新增一个值得加的消融**：把 radix/boot image 与 meta 一起压低带（`\tx-low-staging.txt` 全局低位）在 #57/#82 上各跑一轮——若 0x0B/0x11 消失，bringup 阶段假说即闭环（这两台此前只在 v2.1.6 时代试过全局低位且当时死于 0x91=meta 读锁；v2.1.8 的 FWSEC-低+meta-高的新默认组合未曾测过，正好补这个矩阵格）。
3. TU10x 字典的最终钥匙 = TU debug key（§3）；GA 系明文（§2）可先建全调用图作结构参照——后续工作项。

## 6. 源码级实证（2026-11-01，GA102/AD102/GA100 明文全量反汇编）

> 方法：open-gpu-kernel-modules generated bindata → raw-DEFLATE 解出 IMAGE_DBG/HEADER_DBG → AES-128-ECB（§2 密钥）解密安全段 → envydis `-V fuc5` 全量反汇编（GA102 11342 行 / AD102 10681 / GA100 10919）。本仓流程脚本：`C:\Users\YLW-DORM\AppData\Local\Temp\booterhunt\extract.py`（AES 控制值内置自验）。列表文件在同目录 `*_sec.lst`。

**逐条实证（GA102 偏移，AD102 同构）**：

| # | 结论 | 源码证据 |
|---|---|---|
| 1 | 安全段入口镜像 NS 桩惯用式 | `0x0: mov $r9 0x1000; iords` → I/O 0x1000/0x1100 快照存 DMEM 0x1660/0x1668；SP=0x10000；随后 `f5 3c xx` 特权指令群=密码单元编程（16 条成对模式）；I/O 0x9100 引擎门握手（bit31 读/bit30 写） |
| 2 | **meta 以完整 64 位地址 DMA 读入** | `0x2355: lo=D[0x1660], hi=D[0x1664]|D[0x1668]`（两处 hi 快照**按位或**）→ `lcall 0x5b9(lo,hi,dst=0x400,len=0x100)`——**无 32 位截断**（#74 的"邮箱只装 32 位"推断源码级证伪） |
| 3 | WPR_META_MAGIC 逐字节一致 | `0x2388/0x2395: cmp D[0x400]==0x371a60b3 / D[0x404]==0xdc3aae21`——与我们 loader 写入的 magic 相同（即 VRAM staging 校验读回的那对值） |
| 4 | 0x89/0x8a=meta 校验族码 | `0x2784: mov $r0 0x89`（二次魔数比对失败路径）；`0x278b: mov $r0 0x8a`（描述符字段 D[+0xac] 比对失败） |
| 5 | **0x91=WPR 窗口几何校验失败（对账活寄存器）** | `0x1fa824/0x1fa828`（WPR2 lo/hi）作为字面量被读出与描述符重建比对；`0x288e-0x28b6`: 两 64 位量做减法，**差值必须恰为 0x100000（1 MiB）**，否则 `mov $r10 0x91`。**⇒ B0 因果链源码闭环：FWSEC 未收尾→WPR2/MMU 寄存器半编程→几何校验失败→0x91**（#62 的竞态修复与此逐位吻合） |
| 6 | 金丝雀护 DMA helper（ROP 机制） | `0x5b9`（sysmem DMA 读 helper）内部与每个错误收尾都比对 `D[0x6140]`（GA102 全局 canary；AD102=0x5f50）；**canary 完整→带码返回；canary 破坏→`lcall 0x7ed3` 校验响应**（0x7ed3=141 调用点的验签包装） |
| 7 | **0x0B/0x11=验签参数域 id（字典 v3；修正 §5.1 的"阶段索引"预判）** | 递归下降（见上）证明 `0x2b69`=GetParam（验签结构参数访问器，比较链 switch，非阶段分发器）；`0x2d79/0x2d90/0x2da7: mov $r10 0xa/0xb/0xc` 是 **GetParam 的 id 实参**（读 ctx+0x818/81a/814 的 u16 使能字段，须==1）；失败返回 0x1/0x1c/0x1d，**0xb 同时=PSS 终判哈希不等的错误码**（文件 0x35ab `mov $r0 0xb`）；0x11=期望值表 id（拷贝 count*8 供对账）。⇒ TU 终值 0x0B/0x11 = "meta/WPR2 校验已过、死在签名页 DMA→PSS 验签→期望值对账域"（TU 精确映射待 TU key） |
| 8 | mailbox0=I/O 0x1000（跨族常量） | TU NS 桩 `mov $r15 0x31; iowrs I[0x1000]`；GA 安全段 `0x7de1: mov $r15 0x47; mov $r9 0x1000; iowrs I[$r9] $r15` |
| 9 | GA 以干净 `exit` 终止（22 处） | TU 特有的 INV_INS@0x100 自毁终态是 TU 代际行为，非跨族 |

**含义**：①#57/#82 的 0x0B/0x11 应按"bringup 阶段号"重新解读——0x91 修复（等 HALT）已在 v2.1.8 默认，剩下的 B 类死亡点在 bringup 步骤（radix/boot image 交接的 DMA 阶段——与 #74 的"签名/radix ≥4GiB→bringup 挂死"同域）；②#74 的 meta≥4G→0x89 在 GA 码表上=meta 读回不符（平台不可达的读失败恰好落进 0x89 族，非邮箱宽度问题）；③stage 表的逐号语义（0x0B/0x11 各对应哪一步）需 TU 明文或 GA 递归下降全注释——后续工作。

## 7. TU debug key 已破（2026-11-01，key 族 #6）+ TU102 明文语义图

**Key**：TU10x（TU102/TU116 共用）debug key = 编号测试 key 族的 **#6**（族结构：`key(N) = [N] || EF CD AB 89 67 45 23 10 32 54 76 98 BA DC FE`——MD5-IV 词流的 BE 字节序删 0x01 后前插索引；GA 系=#37/0x25，同族）。出处：cmp170hx csecret 表"ECB blob decryption = secret(6)"+shuttie/bf16-unlock-blackwell-research 定义的 published test-key family；**密钥字节按宪法不入库**，KAT 指针代替：`AES(key6, 00×16)=24e89313dcbd280aa71a071bed8f53c1`、`AES(key6, FF×16)=1610a8b51609725eb31e1e1fc13f22ab`。TU116 旧"58e4591c"条目证伪（噪声）。复现：`nv-uefi-re/tudecrypt.py`（KAT 自验内置）+ 钉版/本地 generated bindata。

**TU102 明文验证链**：首指令 `89 fc ff 00`（=cmp170hx 记载的 `mov $r9 0xfffc`）；熵 6.943；零块 5 块↔密文标记 5 次自洽；TU116 同 key（首指令同、熵 6.937、零块 6）。

**TU102 语义图（文件地址；真=文件+0x100；与 GA 同构）**：

| 结构 | TU102 文件偏移 | GA102 对照 |
|---|---|---|
| 0x91 产生点 | **0x2995** `mov $r10 0x91` | 0x28b6 |
| 0x91 机制 | **0x2937-0x2981：读活 WPR2（0x1fa824/28）→ hi 侧 +0x20000 → 64 位差值须恰 0x100000（1 MiB）**→ 不等 0x91 | 同构（窗口尾常数代际差异） |
| 0x8a/0x89 | 0x1263 / 0x17e6、0x17f4、0x2863 | 同构 |
| **0x0b 产生点** | **0x37dd `mov $r0 0xb`**：0x37cc `ld $r10 D[$sp+0x44]` → 0x37d5 `lcall 0x7c93(…, 0x5a00, 0x20)` **32B 终判比较** → 不等 0xb | GA 0x8254 终判（sig+0x15f, 0x5a00, 0x20）→ 同构 |
| GetParam | **0x2d5e**（真 0x2e5e）：`mov $r10 <id>; lcall 0x2d5e`——0xa/0xb/0xc/0xd… 使能字段序列 @0x2fab-0x2fd9+（id 实参≠错误码）；0x11×5 处（0x323d/0x33ae/0x5f03/0x60d6/0x62b4）=期望值表 id | GA 0x2b69 同角色 |
| 错误登记 | 0x1b0f ×24（码 0x38/0x15…与 GA 同族） | GA 0x1b56 ×26 |
| __stack_chk_fail | 0x7948 区：`mov $r15 0x47; mov $r9 0x1000; iowrs`（写 0x47 停机）+ 0x7a38 包装 ×135 | GA 0x7ed3 ×141 |
| MMIO r/w | 0xf27 ×69 / 0xe3b ×66 | GA 0xf7c/0xe90 |
| DMA helper | 0x532 ×15（canary 0x6140 同 GA） | GA 0x5b9 |
| meta magic | 0x371a60b3/dc3aae21 ×6 | 同 |
| mailbox 语义 | EXIT 时 $r10→mailbox0（NS 桩 0x31 为进度标记被终值覆盖） | 同 |

**B 类最终判读（TU 源码级）**：0x91=**WPR 窗口几何校验失败**（逐指令确认，v2.1.8 竞态修复的 TU 侧机制闭环）；**0x0B=终判 32B 哈希比较失败（PSS 尾，逐指令确认）**；0x11=验签参数域（期望值表 id，精确失败指令待 GetParam 0x2d5e 递归下降）。⇒ **#57（0x0B/0x11）/#82（0x0B）：meta/WPR2 校验全部通过、死在签名页 DMA→PSS 验签→哈希对账域**——与 #74"签名/radix ≥4GiB→挂死"同域；第一嫌疑=该域 DMA 地址带，首选消融=全局低位轮（协议 v2 B 类第 1 条）。

**方法学 caveat（重要，2026-11-01）**：TU102 debug 明文 0x12b4-0x12cd 含一处 **sigSize≤0x1000 的 sanity 检查**（`(sigSize-1)≤0xfff` 否则 0x1c；紧随 `meta+0x20&0xff==0` 否则 0x8b）——而产版场上 payload 模式发布的 sigSize=0xFA00 却成功（mailbox0=0x0）。即 **debug 构建携带产版没有的额外校验**。规则：从 debug 明文导出的**结构/流程/机制**层结论可信（0x91/0x0B/0x11 的机制均被场产终值互证），但**个别边界检查/常量不得直接当作产版行为**——凡从 debug 推断产版约束，必须过场产数据交叉验证（本表各结论已按此规则标注）。附带：booter 明文（TU102/GA102）**无任何 4GiB 边界逻辑**（无 0x1_0000_0000 构造；仅两处 0x0FFFFFFF=256MB 掩码属 XDMA 上下文）——"地址带墙"全部位于 FBIF 孔径翻译硬件+平台 DMA 层（booter 软件之下），booter 自身以每次传输显式重编程孔径（GA102 0x8553 区 `(x&~3)|4/|5`）应对。

## 8. 后续工作项（不在本轮）

- TU GetParam 0x2d5e 递归下降（0x11 失败返回路径定案）；TU116 差分。
- GA 系递归下降全注释（正式分支锚定版）。
