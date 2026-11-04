# Blackwell（GB20x / RTX 50 系）为什么不能解锁——机理、探索现状与可读寄存器面

> 立项来源：[issue #168](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/168)
> （GB206 只读采样，2026-11-04）+ [steviecurrie/gpu-unlock-gb206](https://github.com/steviecurrie/gpu-unlock-gb206)
> （GPL-2.0，随 issue 提交的探针与两次捕获运行）。
>
> 本文只登记**只读**结论。50 系在事务路径上仍然跳过（`loader/src/main.rs`
> `asset_family()` 的段外），ssdiag / diag-unlock 从本轮起记录 FEAT 块而不是打印
> "FEAT addresses not mapped"。
>
> 一手/二手分层：**实测**=社区或本仓在真卡上读回；**公开头文件/反汇编**=可复现的
> 静态证据；**推断**=明确标注。冲突裁决顺序照任务书 §10。

## 0. 结论摘要

| 问题 | 现状 |
|---|---|
| 钳制存在吗？ | **存在，且全系同形。** GB206 `FUSE_SS_FMLA32=3`、`IMLA1/2/4=3`，与 GB202 / GB203M 逐字相同；`FEAT_READOUT_1=0x00c36180` 三卡**逐字节相同**，尽管 GB206 的 SM 数不到 5090 一半 |
| 门是什么？ | `FEAT_OVR_PLM=0xfffffb8f`：读掩码 `0xf`、写掩码 `0x8`（**仅 PL3**）。主机（PL0/PL2）写 SS0/SS1/PLM 全部静默丢弃（3/3，读回不变） |
| 覆盖通路本身关了吗？ | **没关。** `FUSE_FEAT_OVR_DIS=0`、`FUSE_EN_SW_OVERRIDE=0`，与 GB202/GB203M/A100/170HX 同值。唯一墙是特权级 |
| 是硅的问题吗？ | **不是。** 同 GB202 die 的 RTX PRO 6000 满速（BF16→FP32 518 TF/s）、`FEAT_READOUT_1=0`——纯产品分割 |
| 为什么旧办法不行？ | Blackwell 引导链**没有 SEC2 阶段**：FSP(ROM,PL3)→GSP BootROM(ROM,PL3)→FMC(主机供、签名+加密、PL3)→GSP-RM(明文,PL2)。Ada/Ampere 的"签名 booter 栈溢出→ROP gadget→写 PLM"载具不存在 |
| 有 GB20x booter 吗？ | **公开不存在。** `kgspGetBinArchiveBooterLoadUcode` 只为 TU102/TU116/GA100/GA102/AD102 开分支，其余落共享 fallback；开放模块里该 fallback 是 NULL 桩，真实映像只在闭源 `nv-kernel.o` |
| debug 镜像能当后门吗？ | **不能。** GB202 debug/prod FMC 的 `UCODE_IMAGE`/`UCODE_HASH` 逐字节相同（密文也一样），只有 96 B `UCODE_SIG` 不同，且**没有多签名槽表**可降级 |
| 解 FMC 呢？ | 失败且已穷尽一条路线：MD5-IV 置换族 `16!` 全搜 × 两种容器 × 五种模式，零命中；换用真实 RISC-V 码评分器后仍无键 |
| 还开着什么？ | ①suspend/resume 信任面（SR 状态 205.875 MiB/卡，熵 7.9998）；②FSP/FMC 解析器（host 输入按对齐/精确值/边界/认证四类检查，heap 有 11 档 lever）；③每 SM selector（只探过全局 FEAT 块） |
| 本项目能做什么？ | 只读：efi SAMPLE、ssdiag、diag-unlock 都已覆盖该块（本轮）。事务：不能跑，等待 PL3 执行原语 |

## 1. 钳制与门：与 Ada 同形，但读出的不是 Ada 语义

GB206 实测（`10de:2d04`，RTX 5060 Ti 16 GB，驱动 615.71.10，Linux 6.12.113，
BAR0 `0xf8000000`，`nvidia` 绑定态，逐字 `ioread32`）：

| 行 | 偏移 | GB206 | GB202（5090） | GB203M |
|---|---|---:|---:|---:|
| `PMC_BOOT_0` | `0x000000` | `0x1b6000a1` | `0x21120000` | `0x1b3000a1` |
| `FUSE_SS_FMLA16` | `0x8207d4` | 0 | 0 | 0 |
| **`FUSE_SS_FMLA32`** | `0x8207d8` | **3** | **3** | **3** |
| `FUSE_SS_IMLA1` | `0x8207e0` | 3 | 3 | 3 |
| `FUSE_SS_IMLA2` | `0x8207e4` | 3 | 3 | 3 |
| `FUSE_SS_IMLA4` | `0x8207ec` | 3 | 3 | 3 |
| `FUSE_SS_DP` / `FFMA` / `IMLA0` / `IMLA3` | — | 0 | 0 | 0 |
| `FUSE_FEAT_OVR_DIS` | `0x8203f0` | 0 | 0 | 0 |
| `FUSE_EN_SW_OVERRIDE` | `0x820040` | 0 | 0 | 0 |
| `FUSE_SS_PLM` | `0x8200fc` | `0xfffffbff` | — | `0xfffffbff` |
| **`FEAT_OVR_PLM`** | `0x823804` | **`0xfffffb8f`** | **`0xfffffb8f`** | **`0xfffffb8f`** |
| `FEAT_READOUT_1` | `0x823818` | **`0x00c36180`** | **`0x00c36180`** | `0x00c36180` |
| `FEAT_READOUT_0` | `0x823814` | `0x00000233` | `0x00000233` | — |
| `FEAT_OVR_ECC` | `0x82380c` | `0x00888888` | `0x00888888` | — |
| `FEAT_OVR_ECC_1` | `0x823810` | `0xaa2a2aaa` | `0x002a2aaa` | — |
| `SECURE_FEAT_OVR_QUADRO_WR_SEC` | `0x82038c` | 2 | 2 | — |
| `SM_ISSUE_RATE_MODIFIER` | `0x504204` | 5 | 5 | — |
| GFW falcon CPUCTL | `0x110100` | `0xbadf5720` | `0xbadf4100` | — |

写入梯（`SS0=0x88888888`、`SS1=0x8`、`PLM=0xffffffff`，每次读回后恢复原值）：
**0/3 接受，读回不变，静默丢弃**，与 GB202/GB203M 相同。

### 1.1 `FEAT_READOUT_1` 的位域不连续

Ada/Ampere 采样器惯用的"九个连续 3-bit 字段"（`(v >> (3*i)) & 7`）是错的。
字段落在位偏移 `0, 3, 7, 10, 13, 16, 19, 22, 28`（bit6 与 bits27:25 是空位）。
对出厂字 `0x00c36180`，连续解码会打印 `0 0 6 0 6 6 0 6 0`（四个越界 6）；
正确解码是 `0 0 3 0 3 3 0 3 0`（字段序 `imla0 fmla16 fmla32 ffma imla1
imla2 imla3 imla4 DP`）。

### 1.2 在 Blackwell 上"3"不是 1/8

公开 `ctrl2080gr.h` 的枚举把 selector `3` 定义成 `REDUCED_SPEED_1_8`，但
Blackwell 的码→速率律不同：

- `fmla32 = 3` 实测约 **半速**（FP16→FP32acc 48.33 TF/s，出厂 180 W 上限；
  厂标 47.4，未钳制通路 86.27，天花板 ~1.79×），不是 1/8；
- `imla1/2/4 = 3` 下 INT8 仍满速；
- `READOUT_1` 是 **fuse 镜像**，不是有效速率读数；两侧 `READOUT_0/2` 的 RM
  消费者全是 ECC 能力位，没有一个读速率。

所以打印行必须同时给原始字和"字段（非速率）"标注，**不要**按 Ada 律把 3 解成 1/8。

## 2. 真正的断点：引导链少了 SEC2 这一段

Blackwell 的 reset→GSP-RM 链（本代公开内核文档与 nova-core 同述）：

```
FSP          （不可变 ROM，PL3）
  → GSP BootROM（不可变 ROM，PL3）
    → FMC       （主机供、签名、加密，PL3）
      → GSP-RM  （主机供、签名、明文，PL2）
        → host driver / CUDA
```

对比 Ada/Ampere：那条链上有 SEC2，驱动把**签名 booter** 送进 SEC2 邮箱，
booter 的签名 DMA 无界拷贝溢出 DMEM 栈 → 复用原厂指令当 ROP gadget →
在 heavy-secure 里写 `FEAT_OVR_PLM`。Blackwell 上：

- **没有 GB20x SEC2 booter 映像**。开放模块里只剩一个 303 B 的 Turing booter
  和一个 6,233 B 的 Ada secure-scrub；调度的共享 fallback 是 NULL 桩。
  nova-core 文档直说 Hopper/Blackwell "no SEC2 involvement, no Booter ucode, and
  no FWSEC-FRTS stage"。
- **SEC2 硬件在，但是空的**。DMEM `0x8401c0/c4` 是真端口（`AINCR` 会走），前
  8 KiB 全 0；IMEM 对主机 `0xbadf5720`、对 GSP `0xbadf5723`；EMEM 读
  `0x02000040`/0；PLM `0x8403c4 = 0x00000b0f`（全读、零写）。
- **GFW Falcon CPUCTL 不可读**（`0xbadf5720`），所以本仓事务里"等 GFW、点火
  FWSEC"的脚手架在 50 系上也没有对象。
- **host 驱动 FMC 引导是设计性死锁**：`RmDisableCotCmd=0x10` 选
  `_kgspBootstrapGspFmc_GH100`，第一步要 GSP reset，而 reset 需要
  `NV_PGSP_FALCON_RESET_PRIV_LEVEL_MASK`（`0x1103c4`）的 L0 读/写使能；冷启动
  时这两位是清的，只有跑起来的 GSP 固件会置。主机没法复位它需要复位的 GSP。
  失败会留下一个永不返回的 ioctl，是一次性实验。

另外，FMC 之后的解析器检查很密——COT/`WprMeta`/签名段/radix3 指针按对齐、精确
值、边界、认证四类检查，bootloader 每一个字节都在认证后面。唯一"软"的位置是
heap size（不动保护区，只从 WPR 基址挪非保护区，11 档布局 lever）。这说明即便
拿到加密 FMC，改 boot 输入也不是随便改。

## 3. 四条"还能怎么进去"的路，逐条现状

### 3.1 找/挖一个 GB20x booter —— 公开不存在

`kgspGetBinArchiveBooterLoadUcode` 的 HAL 只为 TU102/TU116/GA100/GA102/AD102
开显式变体，其余（含全部 Blackwell）落 `_80f438` 共享 fallback；开放模块里该
fallback 是 NULL 桩（对象另行生成），GB20x 映像只在闭源 `nv-kernel.o`。
没有明文可反汇编的 booter，就没有可挖 gadget 的对象。

### 3.2 debug 签名镜像当替代 —— 逐字节相同

曾有一个合理猜想：debug 构建可挖 → 在 PL3 跑 → 不需要 booter。实测否定。
615.71.10 开放模块 `src/nvidia/generated/` 里：

| 资产 | debug | prod |
|---|---|---|
| `UCODE_IMAGE`（287,304 B） | 密文熵 8.00 | **与 debug 逐字节相同** |
| `UCODE_HASH`（48 B） | `b54e23af…` | **与 debug 逐字节相同** |
| `UCODE_SIG`（96 B） | `c9907005…` | 不同（`207634c2…`） |

"DebugSigned" 只换签名。且 FMC 是**单条 96 B 签名**、没有 `num_sigs` 槽表，
`gpu_unlock_booter_patch()` 那种"按熔丝版本选槽"的降级面也不存在。另外进入
debug 熔丝态在零售卡上没有公开方法。

### 3.3 解 FMC —— 一条路线已穷尽，仍是负结果

FMC 是 `0x48` 明文头 + 加密体（code+data+`0xA00` manifest），P-384/SHA-384 签名；
体熵 7.999、无 nonce、ECB 重复块每镜像至多 1 个。公开的 SEC2 test-key #37
（AES-128-ECB，MD5 IV 的 16! 置换族）在 GB202 上不成立：

| 轮次 | 覆盖 | 用时（GPU0/GPU1） | 结果 |
|---|---|---|---|
| zero-block KAT | 全 16! | 442.5 / 440.2 s | 无 |
| FF-block KAT | 全 16! | 443.2 / 440.9 s | 无 |
| last-round-key 变换 | 全 16! | 555.4 / 553.3 s | 无 |
| FMC `c09` × 五模式 | 全 16! | 1,902 s | 0 有效 |
| FMC `c11` × 五模式 | 全 16! | 1,907 s | 0 有效 |

（五模式=ECB、零 IV CBC、头作 IV 的 CBC、CTR 起 0、CTR 起 1。旧"首块像头"过滤器
曾在正确键首块是 RISC-V 码时误杀，已换成 GSP-RM 明文码训练的 16-bit parcel
评分器，精确 DP 假阳率 `1.2e-14`。）

结论只覆盖 MD5-IV 置换族 × 上述模式，不是"所有 AES 模式/密钥源"。生产 FMC 的
密钥大概率是 SCP 硬件里的 `csecret(6)`，不可软件恢复。

### 3.4 其它门，逐条"为什么不行"

| 门 | 结果 |
|---|---|
| GSP-RM（PL2）直接写 FEAT | 写被丢弃；PL2 太晚。可打补丁的 GSP-RM 只作其它探针的立足点 |
| CUDA debugger class 写 | 类能读块，写仍被丢 |
| 刷 VBIOS | VBIOS 签名；且 VBIOS 里根本没有 FEAT 地址 |
| 改熔丝 | boot fuse check 不过 |
| 电压毛刺 | 无公开方法 |
| `SM_ISSUE_RATE_MODIFIER`（`0x504204`） | 可写但 inert（context init 重新断言） |
| FECS doorbell（`0x409800`） | 主机可发命令、完成回显会回，但已知命令集里没有 selector 字段；未分配 ID 会让 FECS fatal |
| 每 SM selector | 只探过全局 FEAT 块；per-SM 副本可能门不同，进到 caller-owned RM 通道后未继续 |

## 4. 还开着的面（按上游重排后的优先级）

1. **suspend/resume 信任面**：SR 状态每卡 205.875 MiB（`0xcde0000`），熵 7.9998；
   SR 描述符与页表未拆。SR 复用冷启动 COT 与已映射的 GSP-RM，只有恢复的
   **状态**能改变跑什么——信任面比冷路径宽。
2. **FSP/FMC 解析器**：上面 §2 的检查面已画图；FSP 帧格式、heap 11 档 lever、
   vBIOS app `0x23` 尾部待做。
3. **每 SM selector**：需要 caller-owned RM 通道，未做。

上游列的"算作解锁"判据（可直接借作我们的验收）：驱动速率 getter 报的钳制位为 0；
BF16 issue rate 接近 ~440 TF/s 且有配对 GEMM 与数值校验；覆盖活过 FLR、驱动重载、
设备 enable 且驱动无 GPU/GSP 错误；第二次冷启动与第二张卡复现。

## 5. 靶子：PRO 6000 证明是产品分割，不是硅

同 GB202 die 的 RTX PRO 6000（`10de:2bb1`，96 GB）：

- BF16→FP32 **518.0 TF/s**、FP16→FP16 515.0（厂白皮书 503.8，3% 内）；
- **`FEAT_READOUT_1 = 0`**，全部算术速率熔丝 0；
- 它的 FEAT 块**仍是 PL3-only 写门**（同 `0xfffffb8f`）。

所以"满速"不是另一个硅状态，只是另一套熔丝值；缺的原语依旧是 PL3 执行。

## 6. 可读寄存器面（本轮补进工具的部分）

`0x823xxx` 就是 Blackwell 的 FEAT 块（与 Turing 的 `0x4096xx` 不同；Turing 那四行
在 50 系上会 badf，属数据）。本轮把下列只读行补进 ssdiag / diag-unlock / efi
SAMPLE：

| 组 | 偏移 |
|---|---|
| FEAT 覆盖字 | `0x823800/04/08/0C/10/14/18/1C/20/24/28/2C` |
| 速率熔丝 | `0x8207D4/D8/DC/E0/E4/E8/EC`、`0x82059C`、`0x820224` |
| 覆盖总闸/使能 | `0x8203F0`、`0x820040`、`0x8200FC`（`FUSE_SS_PLM`，注意**不是**有效门） |
| 旁证 | `0x82038C`（Quadro 安全写）、`0x504204`（inert 修改器）、`0x8403C4`（SEC2 PLM） |

解读纪律：

1. **零不等于没钳制。** `FUSE_EN_SW_OVERRIDE=0` 只管 CTRL_OPT floorsweep，
   不管速率 selector；170HX 与 4090 都在它为 0 时解锁成功。
2. **`FUSE_SS_PLM=0xfffffbff` 不是有效门。** FLR 会把它复位成
   `0x80842bcf`（等同 `FEAT_OVR_PLM`），随后它再锁回 `0xfffffbff`、FEAT PLM
   锁回 `0xfffffb8f`。
3. **SS0/SS1 非零不等于解锁。** 出厂时每个 enable bit（nibble bit3）都是清的，
   熔丝 selector 生效；非零字是 pre-driver 影子态，冷启动间会变、同一 boot 内
   不变。例：GB206 `SS0=0x06277073`（`3 7 0 7 7 2 6 0`）、`SS1=0x00025005`。
4. **`READOUT_1` 按 §1.1 的位移表解，按 §1.2 的"字段非速率"打印。**
5. 判解锁仍只看 `SS0==0x88888888 && SS1==0x8`；50 系上它只能由事务产生，
   因此 ssdiag 的 `--reapply` 在 50 系上会正确地 REFUSED（PLM 未开）。

## 7. 本项目的落点

- **事务路径不变**：`asset_family()` 不含 `0x2bxx`，`tx_one_card` 继续
  "no asset family mapping - skipping"。这条 skip 是正确的，不是缺陷。
- **只读路径补齐**：efi SAMPLE 表、ssdiag FEAT/熔丝行、diag-unlock 同面。三类
  badf 分桶照旧（0x5040 特权 / 0x1100 未解码 / 0x20xx floorswept）。
- **将来要进门，需要的第一块砖**是以下任一：
  1. 一个明文/可解的 GB20x PL3 载体（booter 或 FMC）；
  2. FMC 加密模式/密钥的新证据（新驱动族、跨版本或跨卡比较）；
  3. FMC 解析器上的可利用缺陷（改 host 输入而不改签名映像）。
- 拿到载体后仍是逐芯片重抽取、重审 gadget 地址；GA102 载荷在 AD102 上 HALT 的
  教训对 Blackwell 只会更硬。

## 8. 来源与许可

- 实测数据：issue #168 与 `gpu-unlock-gb206` 的两份捕获日志（GPL-2.0；
  工具派生自 `JustAResearcher/gpu-unlock-ada` 的 `diagnostics/peek/gpu_peek.c`）。
- 引导链/密钥/解析器：`shuttie/bf16-unlock-blackwell-research`（文本 CC BY 4.0，
  工具 MIT/GPL-2.0），87 次实验的负结果与四层密钥表。
- booter 调度与"缺 debug booter"：`phiraml/gpu-unlock-blackwell`（MIT）。
- GB202/GB203M 对照值与跨部件样本：`JustAResearcher/gpu-unlock-ada` issue #12。
- 免责：以上均为读数与静态证据，没有 50 系的成功解锁；本仓不在事务路径上
  支持 50 系，直到出现上面的第一块砖。
