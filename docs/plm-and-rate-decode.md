# PLM 与速率字段位级解码（owner 口述版整理，2026-11-26）

> 补缺：`0xffffff8f`/`0xffffffff`/`0xffffffef`/`0xffffffcf` 与 SS0 的 `0/8/9/a/b/c`
> 这些值散见 [cold-values](cold-values.md)、[FAQ](faq.md)、[gate-inventory](gate-inventory.md)
> 各处，但位图本身的解码逻辑此前从未成文——本文把它钉下来。与
> [fuserow IFF 文法](evidence/fuserow-iff-grammar/README.md)（同一引导安全路径上的
> 自动写序列）互为姊妹篇。

## 一、PLM（Privilege Level Mask）低字节 = 两个 nibble 的许可位图

每个寄存器块配一个 PLM。**bits[3:0] = L0/L1/L2/L3 的读许可位图**（各层一位），
**bits[7:4] = 同四层的写许可位图**；高 24 位观测值恒 `0xffffff`（写全值即可）。
层级：L0 = host/驱动，L1 = 轻固件，L2 = 中固件，L3 = heavy-secure（认证固件内部——
booter/FWSEC 执行域）。

**已见值全部干净解码**：

| 值 | 低 nibble（读） | 高 nibble（写） | 语义 | 在案 |
|---|---|---|---|---|
| `0xffffff8f` | `1111` 全层可读 | `1000` 仅 bit7 | **仅 L3 可写** | FEAT 族出厂锁定态（cold-values 各族首列）；GFW_PLM 同值（#246 残差表） |
| `0xffffffef` | `1111` | `1110`（bits7,6,5） | **L1+L2+L3 可写，L0（host）锁** | Turing 回锁实测值（随板方差档，faq #relock；#246 残差表） |
| `0xffffffcf` | `1111` | `1100`（bits7,6） | **仅 L2+L3 可写**（深锁，连 L1 固件都不进） | MMU PLM / FBPA PLM（cold-values；#246 残差表实测同值） |
| `0xffffffff` | `1111` | `1111` | **全层可写 = 开** | 解锁后 FEAT_OVR_PLM 目标值（读回判据） |

要点：

- "8f=关、ff=开"不是开关对偶，是**位图裁剪**——关的形态取决于哪些层保留写权；
- `ef` 与 `cf` 是**两种不同的关门**：ef 把 host 踢出去但留低层固件写权（回锁后
  驱动碰不动、卡还能自管），cf 是深锁（heavy-secure 专属，敏感块用）；
- 低 4 位恒开 = **未解锁态也能侦察**（M1 只读采样整季能干，靠的就是这四位）。

## 二、SS0 速率 nibble：bit3 = 覆盖使能，bits[2:0] = 速率码

SS0（0x409664）八个 4 位字段，**每字段：bit3 = override-enable，bits[2:0] = 速率码
（0=1x、1=½、2=¼、3=⅛）**。SS1（0x40966c）单字段同构。

| 值 | 二进制 | 解码 |
|---|---|---|
| `0` | `0000` | **覆盖未使能** → 继承出厂/熔丝默认速率（"0=默认"≠最慢，是"不发言"） |
| `8` | `1000` | 使能 + 速率码 0 = **满速**（`0x88888888` = 八字段全满速覆盖） |
| `9` / `a` / `b` | `1001` / `1010` / `1011` | 使能 + ½ / ¼ / ⅛（速率码 = log2 分母直读） |
| `c` | `1100` | 使能 + **速率码 4——表外**：台架速率梯四档钉死（0/½/¼/⅛），c 不产生第五档 |

nibble2 的 bit1 是硬掩码（单 bit 速率字段）——见
[tu106-ss0-semantics-bench](evidence/tu106-1f07/) 的 11-23 语义实测季。

## 三、fuserow（IFF 池）写的是设备寄存器，BAR0 是投影

IFF op 流在**引导安全路径**上由硬件序列器/熔丝控制器**直接在内部 PRI 总线**上
回放这些写（`0x1183a0`、`0x21068`… 全是 PRI 寄存器文件地址），不经任何 BAR。
**BAR0 只是 host 侧看进同一份寄存器文件的窗口**。同构先例：

- config 0x610 == BAR0+0x88610（XVE 页镜像 config 空间，rebar 轮实证）；
- Turing flat 面 PL0-block 而 −0x7FF000 偏移面可读（两窗口、同一寄存器、不同
  译码门——cold-values §TU104/§TU106 的 flat-vs-shifted 差异）。

**一个寄存器、多个孔径**是本芯片族的常态；读数分歧时先分清站在哪个孔径上。

## 四、与 VFI（电压故障注入）的对照：写权限三层谱系

fuserow 的写权限分三层，**软件层全部无钥匙**：

1. **熔丝阵列本体 = OTP 物理**：已烧位不可清除；未烧位只能经编程路径
   （电荷泵 + PROGRAM 命令）烧——fab/测试域。量产卡的运行期接口只译码
   CMD_READ（ssdiag 驱动的就是它；控制器块 PLM 开的是"发已存在的命令"，
   PROGRAM 不在运行期译码表里）。
2. **译码目标寄存器 = PLM 门**：熔丝只是初始值源，锁行为的是目标块自己的 PLM。
3. **层级谱系 = 攻击面谱系**：

| 形态 | FEAT 写使能 nibble | 软件可解？ | 在案 |
|---|---|---|---|
| **消费卡** | bit7=1（L3 有写权） | **可借**：payload 不是绕过权限检查，是 ROP 重排已签名 booter 的原厂指令，让写在 **L3 上下文**里发生——借权通过 PRI 比较 | 本项目全部交付 |
| **专业卡**（#237 Willian-Zhang 形） | 无任何层持位 | **死路**：ROP 借不到权——不存在持权的软件上下文，写在 PRI 比较器上对每个层级都被静默丢弃 | gate-inventory 边界 2 |
| **A100 类（VFI 公开工作）** | 同上无层持位 | **只剩物理层**：所有软件层无位可借时，唯一攻击面是权限检查机制本身——电压毛刺让 PLM 比较/锁存在采样瞬间出错 | .ga100vfi-notes 材料谱系 |

三类是同一谱系的三段：**消费卡=L3 有位可借；专业卡=无层持位、软件死路；
VFI=不再问"谁有权"，直接把比较器搞错**。fuse array 再加一层绝对性——即使
VFI 穿透 PLM，改的也只是译码后的寄存器；**阵列里的 OTP 位是硅物理，电压攻击
改不回已烧位**，只能让"读出来的效果"瞬时失真。这也是 unlock 为何是易失覆盖
（每引导重放）而非"改熔丝"——物理上不存在第二条路。

## 互链

- 冷值与在案地址：[cold-values.md](cold-values.md)
- PLM 写域实验（closed PLM 11 点拒写/plm 写域四格矩阵）：[gate-inventory.md](gate-inventory.md) §边界 9
- IFF op 文法与五卡解码：[evidence/fuserow-iff-grammar/README.md](evidence/fuserow-iff-grammar/README.md)
- 回锁值方差与门开默认：[faq.md #relock](faq.md)
