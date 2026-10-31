# 第三轮：全量失败案例复核 + booter 安全段语义图（2026-11-01）

> 本轮方法：issue 区全量扫描（open+closed）+ GA 系明文 booter 递归下降语义图 + TU debug key 定向猜解 + AD102 同深度标注。**结论均已源码级实证**（.lst 偏移可 grep 复核，列表在本地 `D:\git-repo\nv-uefi-re\`）。按维护者指令：**未发任何 GitHub 回复**。
> 前序：[tu10x-booter-0x91-matrix.md](tu10x-booter-0x91-matrix.md)、[booter-secure-re-keychain.md](booter-secure-re-keychain.md)、[tu10x-forensics-diag-protocol.md](tu10x-forensics-diag-protocol.md)。

## 1. 重大语义修正（字典 v3，已进 loader/sim/docs）

递归下降发现 `.lst` 的**真实 IMEM 地址 = 文件地址 + 0x100**（lcall/lbra 绝对目标差 0x100；逐字节 tiling+三族交叉验证）。据此：

| 项 | 旧判读（已作废） | v3 判读（源码实证） |
|---|---|---|
| `0x2b69`(GA102 真) | "阶段分发器/阶段索引" | **GetParam**——验签结构参数访问器（比较链 switch；签名 ($r10=id,$r11=ctx,$r12=out)，错误码 0x1/0x1c/0x1d） |
| 小字面量 0xa-0xe/0x11/0x12… | "阶段号" | **GetParam 的 id 实参**：0xa-0xe=使能字段（ctx+0x818/81a/814/816/81c，u16/u32 须==1）、0x11=期望值表（ctx+0x820，count≤0xb 拷贝）、0x6/7/8/9=签名 blob（ctx+0x414/614/14/214，≤0x200）、0x12/13/14/15/16=读字段、0xf=期望值对账 |
| 0xb | "阶段号" | **双身份**：既是 GetParam id（0x81a 使能字段），**又是 PSS 终判哈希不等的错误码**（GA102 文件 0x35ab `mov $r0 0xb`） |
| 0x91 | "cmp50hx 拒收族" | **WPR 窗口几何校验失败**：booter 把 meta 描述符与**活 WPR2 寄存器**（0x1fa824/28）重建比对，64 位差值必须**恰=0x100000（1 MiB）**（GA102 文件 0x28b6）——v2.1.8 竞态修复的机制级闭环 |
| 0x89/0x8a | "DMA 读回不符" | **0x89=meta magic 二次比对失败、0x8a=描述符字段（D[+0xac]）失败**——对账对象是活 WPR2/MMU 状态 |
| meta 寻址 | （#74 曾疑 32 位截断） | **完整 64 位**（hi=D[0x1664]\|D[0x1668] 按位或，GA102 文件 0x2255 起）——#74 已撤回，源码级证伪 |

**对 B 类（#57/#82 终值 0x0B/0x11）的最终判读**：meta/WPR2 校验**已通过**，死亡点在**签名页 DMA→两轮 RSASSA-PSS 验签→期望值对账**域——与 #74 的"签名/radix ≥4GiB→SEC2 永不停机挂死"同域（#74 挂死不报码、#57/#82 报码停机）。GA 侧证据为指令级；**TU 精确映射仍待 TU debug key**（§3）。

## 2. GA 系 booter 完整语义图（GA102 真/文件地址，AD102 同构）

**总控流水线**（文件 0x218e-0x27aa）：
1. sec-args：读 MMIO `0x118244/48`（16B 块前 8B）→ 真 0x19ea 解析 → 清零；
2. meta 地址 = 启动代码存的 mailbox 快照（DMEM 0x1660/0x1668）；
3. `lcall 0x5b9` DMA 读 0x100B → DMEM 0x400（helper 自检 canary D[0x6140]）；
4. meta 双格式校验：新（0x371a60b3/dc3aae21，+8==1，+0xf8/fc==0xa0a0a0a0）/旧（0xc6c39d93/8a3bb9e6，+8==2）→ 0x89/0x8a/0x91；
5. hash 上下文（DMEM 0x318/310/330）+ region 注册（真 0x2abc：(pf<<4)|8 写硬件数组）；
6. **签名页 DMA**（SMALL 真 0x36f2）：`0x5b9(dst=0x600,max=0x1000,addr=meta+0x48/4c)`=sysmemAddrOfSignature 消费点；GetParam 0x6|0x8/0x7|0x9 两轮取签名 blob（≤0x200）→ 0x5b00；
7. **两轮 RSASSA-PSS 验签**（BIG 真 0x2e3d/文件 0x2d3d）：384B 签名首尾交换、末字节==0xBC、SHA-256+MGF1、`lcall 0x8254` 终判 32B → 不等 `mov $r0 0xb`；
8. hash 清单页（DMEM 0x5200）打补丁（GetParam 0x10/11/12/13）+ 15MB 分块哈希（真 0x5e29）+ 期望值对账；
9. meta 回写（真 0x194c：+0xf8/fc 重盖 0xa0a0a0a0、+0x48 清零）；
10. 握手：`0x1182d0←meta+0xec`、`0x11821c|=0x20000`、最终 真 0x703a（0x111668←0x111、0x111388←1、poll 0x1100f4 bit0x2000）；
11. 三处 0xa6 超时点（文件 0x684c/68a4/68fc：对 0x136600/136e00/136400 配置+轮询 4×100ms → 登记 DMEM 0x2384 日志区+`exit`）；
12. canary 破坏 → `lcall 0x7ed3`（真）= `mov $r15 0x47; iowrs I[0x1000]`（**写 0x47 到 CPUCTL 停机**）——GA 干净 exit（22 处）对照下，TU 的 INV_INS@0x100 自毁为 TU 代际行为。

三族对照（GetParam/bringup/SMALL/BIG/错登/hash-walker/停机点/canary 偏移）已逐项核对一致（canary：GA102=0x6140、AD102=0x5f50、GA100=0x6340）。

## 3. TU debug key 已破（key 族 #6）——B 类源码级终判

- **Key**：TU10x（TU102/TU116 共用）= 编号测试 key 族 **#6**（族结构 `key(N)=[N]||EFCDAB896745231032547698BADCFE`；GA=#37）。线索链：cmp170hx csecret 表"ECB blob decryption = secret(6)" + shuttie/bf16-unlock-blackwell-research 的 published test-key family 定义 → 逐号试到 #6 命中 KAT。**密钥字节不入库**，KAT 指针：`AES(k6,00)=24e89313…`、`AES(k6,FF)=1610a8b5…`。TU116 旧"58e4591c"证伪（噪声）。复现：`nv-uefi-re/tudecrypt.py`。
- **TU102 明文**（34304B，熵 6.943，首指令 `89 fc ff 00`=cmp170hx 记载值；TU116 同 key）已全量反汇编（`nv-uefi-re/TU102_sec.lst` 10959 行/TU116 10964 行）。
- **B 类源码级终判**（keychain §7 全表）：
  - **0x91 = WPR 窗口几何校验失败**（文件 0x2937-0x2995：读活 WPR2 0x1fa824/28→hi+0x20000→64 位差值须恰 1 MiB→不等 `mov $r10 0x91`）——逐指令确认，与 GA 同构；v2.1.8 竞态修复的 TU 侧机制闭环。
  - **0x0B = 终判 32B 哈希比较失败**（0x37cc-0x37dd：`lcall 0x7c93(…,0x5a00,0x20)` 不等→`mov $r0 0xb`）——PSS 尾，逐指令确认，GA 同构（0x8254/0x5a00/0x20 同款）。
  - **0x11 = 验签参数域**（GetParam=0x2d5e 的期望值表 id ×5 调用点；精确失败指令待递归下降）。
  - mailbox0 终值=安全段 EXIT 时 $r10（NS 桩 0x31=进度标记被覆盖）。
  - ⇒ **#57/#82：meta/WPR2 校验全部通过、死在签名页 DMA→PSS 验签→哈希对账域**；与 #74"签名/radix ≥4GiB→挂死"同域；首选消融=全局低位轮（B 类第 1 条）。
- 猜解过程存档：trivia——暴力族（GA 算术/测试向量/magic 半组合 ~651+64 万候选）阴性后由族结构+编号表命中；`tukey3.py/tukey4.py`（CNG 加速版）留存 nv-uefi-re。

## 4. issue 全量失败案例归档（23 例+4 线程内数据点）

（主表见本轮调查附件；此处只列**源码机制归位**后的关键更新）
- **#89（11-01 已跑通）**：v2.1.8 低位默认不够（EXCI 0x009003ff）→ 用户 **FWSEC VRAM staging**（BAR1 写+TRANSCFG4→LOCAL_FB+BL DESC 指 VRAM）全链绿 +63.6%；**meta 必须留 sysmem**（VRAM→0x89，与 GA 明文"meta 按 sysmem 口径解释"一致）。A 类第三形态。→ 上游提案：FWSEC staging 分配器加 sysmem 自测失败→VRAM 兜底（代码提案待批）。
- **#74（第 17 轮修正）**：全低位轮=**V67 模式**且 `PLM still locked`——UEFI 链在该机从未完整跑通；撤回 32 位邮箱推断；第 28 轮"签名/radix ≥4G→SEC2 poll timeout 挂死"= B 类同域的第三种死形（挂死不报码）。剩余阻塞=Windows USB4 接管。
- **ProxVirtualZone（#62 内）翻案**：0x0B 出现在 **replay 模式**、换 V67 即成功——replay 的 0x0B 不必然死刑（v3 语义下自洽：replay 也走 PSS 验签，stock 签名页任何不匹配即 0xb）。
- **#91（Catnip202X 新 issue）**：提议 dump DMEM 内 publish 后完整 WprMeta 与成功机 A/B——与 v3 语义互补（0x89/0x91 已源码定位到具体字段/几何，DMEM dump 可直接看到 booter 眼中的 meta）。可由 BOOTER-AUDIT 框架扩展（0x400 页全字段打印）。
- **#84**：自关无解释（TU102/1e04/22G、CSM on、驱动 572.60 无 GSP 行——**GSP 疑未真跑**，形态与 #84 类"配置不完整"一致）。
- 特征计数：0x91×6、0x0B×3、0x11×1、0x89×4、FWSEC 死×2、Windows 丢锁×5、交棒挂死×4、误判×3；已解决 14、OPEN 未解 7（#35/#37/#54/#55/#57/#74/#90）。

## 5. 下一步建议（等维护者拍板，未实施）

1. **FWSEC VRAM staging 兜底**（#89 型）：`\tx-fwsec-vram-staging.txt` marker 或自动回退（sysmem 分配自测→BAR1 写读回校验失败→切 VRAM）——复用 tx.rs:1291 的 VRAM staging 基建，改动集中在 fwsec_ignite_tu10x。
2. **WprMeta 全字段 dump**（#91 提议）：BOOTER-AUDIT 后加 DMEM 0x400 页 64 字打印（0x91/0x89 专用），配合成功机 A/B 即可源码级归位剩余字段差异。
3. **B 类现场轮**（#57/#82）：协议 v2 三轮（默认/`\tx-low-staging.txt`/`\tx-bootvec-sec.txt`）回收后与 §1 语义图对账。
4. ~~TU key~~ **已破**（§3，key 族 #6）——后续仅剩 TU GetParam 0x2d5e 递归下降与 TU116 差分（低成本）。

## 6. 本轮产物清单

- docs：keychain §6-7（v3 语义+修正）、matrix（#74/#89/#91 更新+维护结论定案版）、protocol v2.1（B 类三轮）、nsstub-disasm C3（v3）、loader/sim 字典 v3+stale 指引修复（`d896aea`+本提交）。
- 本地 RE 工作区（`D:\git-repo\nv-uefi-re\`）：GA102/AD102/GA100/TU102/TU116 明文与列表、tukey3.py 猜解脚本。
- 未动：四个 PR（#64/#65 DRAFT、#77 OPEN、#86 DRAFT）；全部 GitHub 回复（等你指令）。
