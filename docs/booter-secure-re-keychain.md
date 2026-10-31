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

## 3. TU10x 密钥缺口

- TU102 610.43.03 DBG padding 最频块 = `24e89313dcbd280aa71a071bed8f53c1`（14 次）= **`AES(key_TU10x, 00×16)` 的已知验证值**——拿到 key 的瞬间即可解密钉版包内 TU102 IMAGE_DBG（59136 B，与量产同版）。
- GitHub 全站 code search 该 hex 无命中；gist 无 Turing key。
- 线索：gist 提及 "Lapsus Booter disassembly"（带 `_acrVerifySignature_TU10X` 等**符号**）——Lapsus 2022 泄露源码树中存在过 TU10x booter 的完整反汇编，暗示 TU 明文/debug 工具链在私人手里。次级线索：PetriKrohn/BlackSun 材料、FAUCON/envytools 社区 key 表。

## 4. 状态码写入结构（GA10x 明文实测）

- **status writer helper**：GA102 @`0x1d87`（16 指令）↔ AD102 @`0x31b5`（TOTORONG 已发表，逐字节同构）。入口惯用式 `mov $r10 <code>` → helper 把 `$r10` 写 **mailbox I/O 0x1000**（与 TU10x NS 桩 `mov $r15 0x31; iowrs I[0x1000]` 同一 I/O 口——跨族一致）。
- 调用点：GA102 26 处 / AD102 21 处（两族一一对应）。字面量码：`0x15`（最常见失败码）、`0x38`、`0xa6`（WPR 三连检查点 GA102 0x6bbf/0x6c17/0x6c6f）。**无 0x0B/0x11/0x91**——TU10x 码表独立。
- **cmp170hx wiki（GA100 515 期带符号反汇编）记载**：`wpr_desc_validate` 错误码 **0x89–0x90**；`tgt_falcon_bringup` 含 **0x11**。

## 5. 对 B 类（#57/#82）的含义

1. **0x11 的阶段假说升级**：GA100 谱系里 0x11 属 `tgt_falcon_bringup`——**meta 过验之后的引擎拉起阶段**，不是"早期拒收"（我们字典 v2 的"stage unknown"措辞正确，但阶段方向应从"验证前"翻转为"验证后 bringup"）。若 TU10x 同构，#57/#82 的 0x0B/0x11 意味着 **meta 校验已通过、死在 booter 拉起 GSP/RM 的环节**（radix/boot image 交接）——与 #74 实测的"签名/radix ≥4GiB → bringup 挂死"是同一阶段的两种死法（#74 挂死不报码、#57/#82 报码停机）。
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
| 7 | **0x0B/0x11=bringup 阶段索引（字典 v3）** | `0x2b69(stage_id,…)`=表驱动阶段分发器（17 调用点）；`0x2d79/0x2d90/0x2da7: mov $r10 0xa/0xb/0xc` 后跟 `lcall 0x2b69` 与失败共跳 `0x35ad`（阶段错误收尾：canary 查→带阶段返回值返回）。**小字面量 0x0a/0x0b/0x0c/0x10/0x11/0x12/0x15 全部是阶段号**——⇒ TU 终值 0x0B/0x11 最可能是"死在第 0x0B/0x11 号 bringup 步"的阶段报告，非独立错误码（与 cmp170hx wiki"0x11∈tgt_falcon_bringup"互证） |
| 8 | mailbox0=I/O 0x1000（跨族常量） | TU NS 桩 `mov $r15 0x31; iowrs I[0x1000]`；GA 安全段 `0x7de1: mov $r15 0x47; mov $r9 0x1000; iowrs I[$r9] $r15` |
| 9 | GA 以干净 `exit` 终止（22 处） | TU 特有的 INV_INS@0x100 自毁终态是 TU 代际行为，非跨族 |

**含义**：①#57/#82 的 0x0B/0x11 应按"bringup 阶段号"重新解读——0x91 修复（等 HALT）已在 v2.1.8 默认，剩下的 B 类死亡点在 bringup 步骤（radix/boot image 交接的 DMA 阶段——与 #74 的"签名/radix ≥4GiB→bringup 挂死"同域）；②#74 的 meta≥4G→0x89 在 GA 码表上=meta 读回不符（平台不可达的读失败恰好落进 0x89 族，非邮箱宽度问题）；③stage 表的逐号语义（0x0B/0x11 各对应哪一步）需 TU 明文或 GA 递归下降全注释——后续工作。

## 7. 后续工作项（不在本轮）

- GA 系递归下降全注释（stage 表逐号语义 → 0x0B/0x11 具体步骤名）；envydis 线性扫描在变长指令流有失同步区，正式注释需分支锚定。
- TU key 线索跟踪（Lapsus 树/社区 key 表）。
- 若 TU key 到手：解密 TU102 IMAGE_DBG → 直接检索 TU 码表与 stage 表闭环。
