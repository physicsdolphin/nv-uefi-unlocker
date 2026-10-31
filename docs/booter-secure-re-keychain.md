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

## 2. GA 系 debug 密钥（公开，已三重验证）

- Key（gist nvlight/fc7479…，"recovered by BlackSun"，ada-port.md 钉版收录）：
  `REDACTED-GA-DEBUG-KEY-SEE-GIST`（AES-128，ECB，作用于 IMAGE_DBG[0x100:f2]）
- 控制值：`AES(key, 00×16)=717d1494eaca317ff106195258b38377`（GA100/GA102/AD102 DBG padding 实测同值）；`AES(key, FF×16)=fd7e5b57768a309baabcc6903118c8f5`。
- 字节级终验：AD102 解密结果 @0x31b5 与 TOTORONG 发表的 status writer 逐字节相同；GA102 差异仅 canary 槽位 0x6140 vs 0x5f50（芯片差异）。
- 复现流程（未落盘、全内存）：bindata 数组（NvU8/NvU64，raw-DEFLATE+CRC 尾）→ 解出 IMAGE_DBG → `[0x100:hdr.f2]` AES-ECB 解密（DMEM 不解、NS 前缀保留）→ entropy/SHA/控制值三验。

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

## 6. 后续工作项（不在本轮）

- GA102/AD102/GA100 明文全量 envydis 注释（结构参照：wpr_desc_validate/bringup 全调用图）。
- TU key 线索跟踪（Lapsus 树/社区 key 表）。
- 若 TU key 到手：解密 TU102 IMAGE_DBG → 检索 `mov $r10 0x0b`/`0x11`/`0x91` 字面量 → 字典闭环。
