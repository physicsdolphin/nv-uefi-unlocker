# TU106 XP3G gen2 策略链机制探针（2026-11-24）——写全粘、速率不跟随

## 上下文

- 机器：TU106 台架（RTX 2070, 10de:1f07），Windows in-OS，XP3G PLM0=0xffffffff（OPEN，原生开归因待 Phase 0 冷态读）。
- 操作：gate-inventory §2.3 的 50HX patch04 主机侧七项 + LTSSM kick + TLS 镜像探针，
  共 9 写，ssdiag 0.8.9 `--target-offset/--value`（写+读回）。
- 目的：TU106 consumer 格（§2.3 留空）——开门后主机策略写是否生效（LnkSta 为判据）。

## 结果一：9/9 全粘（边界 9 的反面补全）

```
poke 0x8841c <- 0x20b42d00 -> readback OK   (PRIV_MISC_1)
poke 0x8e120 <- 0x0        -> readback OK   (XP3G_VAL0)
poke 0x8e110 <- 0x1        -> readback OK   (XP3G_OVR0)
poke 0x88610 <- 0x1        -> readback OK   (VSEC_HIERARCHY)
poke 0x8c2c0 <- 0x60731b2  -> readback OK   (CYA_0 同值探针)
poke 0x8c040 <- 0x800c5800 -> readback OK   (LINK_CONFIG_0 MAX_RATE=2)
poke 0x8c1c0 <- 0x150036   -> readback OK   (PL_LINK_RATE Turing 编码)
poke 0x880a8 <- 0x1f0002   -> readback OK   (LnkCtl2 TLS=Gen2 镜像探针)
poke 0x8872c <- 0x6        -> readback OK   (LTSSM kick)
```

与 PLM 级联探针（tu106-plm-cascade-probe/）合起来：**closed 的 PLM 寄存器 host 写
一个位不动；open 的门后面 host 写全粘**——两级边界都实测。

## 结果二：config 镜像写穿透（独立正 datum）

0x880a8 ← 0x1f0002（LnkCtl2 TLS=Gen2）后，**config space**（ssdiag 头部 cap 读，
非 MMIO）读回 `LnkCtl2 TLS Gen2`——PRI→config 镜像可写且穿透到真实 config space。
两轮 post-state 都保持 Gen2，跨轮未回退（NVIDIA 驱动没有重写它，H5 驱动回写假设排除）。

## 结果三：链路速率不跟随（判据层面的反证）

- **决定性单行**（run 2 config 头部）：`LnkSta Gen3 x16 | LnkCtl2 TLS Gen2` 同行共存。
  TLS 是目标上限——空闲降到 Gen1 合法（低于 TLS 不矛盾），**活跃态 Gen3 高于 TLS
  是 spec 级违反**：若 GPU 发送端真按 Gen2 广告，Gen3 永远训不上来。⇒ XVE 发送端
  广告路径不取 config TLS，由 XP3G/XVE 内部态（固件启动期改写）驱动。
- MMIO 策略寄存器跨轮保持 poke 值（run 2 里 0x8c040/0x8c1c0/0x8841c 等全部仍是
  写入值）——值粘住、驱动不动它、链路照样 Gen3。

## 结果四：Gen1/Gen2 观测 = 空闲 PM（P5/P8 节能），非钳制效果

- **同一次运行内**（run 1）：头部 config 读 `LnkSta Gen1`，几秒后 MMIO
  0x88088=0x11010043（LnkSta=Gen3 x16）——链路动态在 Gen1↔Gen3 之间跳。
- LnkCtl ASPM 位逐轮漂移（run 1 0x1101=L0s only → run 2 0x1103=L0s+L1）；
  LnkCap 的 L0s 延迟字段也变（0x00453d03 → 0x00454d03）——电源管理活动的旁证。
- 结论：任何单点 LnkSta 采样必须负载限定；空闲 Gen1/Gen2 是 PM 形态，不是本实验效果。

## 结构解读（cmp170hx 谱系参照，实验阶段表述）

- 170HX dead-ends #978：strap 复制死案结案词——"the straps don't do anything" /
  "**falcon is driving the rewrites**" / "**there's no gen3 override register**"。
  gen 上限=falcon 固件启动期改写 PCIe 配置强制的**天花板**，不是 strap。
- 170HX dead-ends #914：XP3G PHY-rate override **孤立写**不动链路（mask 开、override
  可写、rate 字段读回 Gen3-capable，链路仍 Gen1）——override 块是组合拳组件，
  单独写无效，与本次 consumer 观测同形。
- 合并解读：XP3G OVR 块的作用方向是**解锁**（抬起矿卡上固件钳的 Gen1 天花板），
  不存在压低机制——consumer TU106 无钳可解，方向反转的实验注定只产"写粘、不动"。
- 本轮未试的腿：OVR3(0x8e11c)=0x4 / VAL3(0x8e12c)=0x00200000（cmp170hx 谱系
  register-index 含此对；结构解读预测同样无效，可作 2-poke 收尾负控）。

## 收口（同日第三轮）：OVR3/VAL3 补腿 = gen3 天花板旋钮——上节"方向反转"解读作废

第三轮补腿（die 值修正后，`round3-clamp.txt`）：

```
poke 0x8e12c <- 0x16480000 -> readback OK   (XP3G_VAL3 = 本卡 PCIE_MAGIC_D / XP3G_STATUS3 值)
poke 0x8e11c <- 0x4        -> readback OK   (XP3G_OVR3)
poke 0x8872c <- 0x6        -> readback OK   (LTSSM kick)
```

**结果：PCIe 上限被钳到 Gen2**（owner 报告）。逐条更正：

- 上节"无压低机制——consumer 无钳可解，方向反转"**被本轮证伪**：170HX #978 谱系
  （"no gen3 override register"）不迁移到 Turing consumer——slot3 就是活体 gen3
  天花板旋钮（in-OS、驱动在跑、原生开 PLM0 门后）。
- "VAL3 = PCIE_MAGIC_D 值"模式逐 die 成立：cmp170hx 链写 A100 的 MAGIC_D；TU106
  写**本卡自己的** 0x16480000（dump 里 PCIE_MAGIC_D 0x21520 与 XP3G_STATUS3 同值）。
- 工作假设（待更多 die）：slot0=Gen2 侧 override（矿卡 50HX 抬限用它）、
  slot3=Gen3 侧 override（consumer 压限用它）——两方向旋钮都真实存在。
- 同轮负格：GR_TOPO 0x409604 host 写拒（0x8040002→读回 0x8040003）；GFW_SCRATCH
  0x118234 host 写拒——写域四格矩阵闭合（见 gate-inventory §3.9）。
- 未测：可逆性（OVR3←0 + kick 是否回 Gen3）、跨重启持久性、负载下带宽确认。

## 轮四（同日）：可逆性负结果——旋钮作用在能力向量层

```
poke 0x8e11c <- 0x0 -> readback OK
poke 0x8872c <- 0x6 -> readback OK
```

**Gen2 未回退**；设备管理器禁用-启用也不行（MMIO config 寄存器，无复位不洗）。
新读数：config 头行 **LnkCap max Gen2 x16 | LnkCap2 [Gen1,Gen2]**（原 max Gen3 +
[Gen1,Gen2,Gen3]）——**旋钮把 Gen3 能力位从广告里摘除了**，作用层=能力向量，
深于 TLS/MaxRate。这同时解释了轮一为何九写不动链路：那些写不治理，能力广告
才是治理层，slot3/VAL3 动的是广告源头。

- 脏寄存器清单（相对首轮 dump）：0x8841c、0x8e110、0x88610、0x8c040、0x8c1c0、
  0x880a8、0x8872c、0x8e12c（VAL3 仍 0x16480000——OVR3←0 只关了使能位）。
- 全量回退清单（原值）：`0x8841c←0x20b40500 / 0x8e110←0 / 0x88610←0x1001 /
  0x8c040←0x80045800 / 0x8c1c0←0x130036 / 0x880a8←0x1f0003 / 0x8e12c←0 /
  0x8e11c←0 / 0x8872c←0`，回退后 kick 再看 LnkCap；仍 Gen2 → **温重启**
  （devinit/固件启动期改写 = 保证恢复路径）。

## 轮五（同日）：S4 休眠（rail-off）+ 解锁 boot —— 旋钮挥发性与 Phase 0 归因一次收口

- **Gen3 能力安全恢复**：LnkCap max Gen3、LnkCap2 [Gen1,Gen2,Gen3]、TLS Gen3——
  轮三/轮四的能力向量改动**随断电洗净**（旋钮=power-cycle-volatile，无持久损害）。
  残余瞬态：LINK_CONFIG_0=0x80085800（[19:18]=2）+ LnkSta 暂挂 Gen2=驱动从休眠
  镜像恢复时的链路策略重放（我们写的是 3，原值是 1——既非原值也非我写值=本轮
  驱动/固件所为），禁用-启用后 GPU-Z 恢复正常 Gen3（owner 实测），可自愈。
- **意外收获：S4 恰好执行了 Phase 0**（rail-off + 本次解锁 boot 的 payload 只写
  FEAT 0x409650）——**XP3G PLM0=0xffffffff、FUSECTRL_PLM/FUSEOPT_PLM=0xffffffff
  全部原生开**（TU106 consumer），fuserow scan 副作用假设作废；
  DEBUG_FEAT_SW_OVR_EN 0x21148=1 同为原生值。FEAT_PLM=本 boot loader 武装
  （canary intact 同 boot）。
- XP3G 策略寄存器全组回原值（0x20b40500/0x1001/0x130036/OVR/VAL 全 0）——
  **gen2 旋钮 = 会话内易失**，无任何跨断电残留。
- 附注：本 inventory 内部又有 config LnkSta Gen2 vs MMIO 0x88088=0x143(Gen3) 的
  同轮分叉 = 读数时链路正在重训（disable-enable 后的重训过程），终态以 GPU-Z
  正常 Gen3 为准。

## 轮五b（同日）：负控——slot3 三连打单独执行无效，配方=合取

S4 后纯净态（XP3G 全原值）单跑轮三三连打：

```
poke 0x8e12c <- 0x16480000 -> readback OK
poke 0x8e11c <- 0x4        -> readback OK
poke 0x8872c <- 0x6        -> readback OK
```

**不钳 Gen2**。轮三成功时轮一九写仍同会话在场 → 有效配方 = **轮一策略组（先导）
+ slot3/VAL3（触发）合取**，两者单独都无效。与 cmp170hx #914（"override 孤立写
不动链路"）精确同形——override 块是组合拳组件，两条谱系在此汇合。最小成分二分
（哪一项先导是必要项：OVR0/VAL0？MAX_RATE？PL_LINK_RATE？）留待需要时再做——
每步需 rail-off 洗净 + 单变量，成本高，机制主线已闭合、优先级低。

## 轮五c（同日）：顺序不对称——正序立即钳、逆序要 PnP 周期

owner 实时二分第三轴：同样两组写（九写策略组 + slot3 三连），

- **正序**（九写 → slot3 三连，kick 收尾）：**立即**钳 Gen2；
- **逆序**（slot3 三连 → 九写，kick 收尾）：不立即钳，**禁用-启用设备**后才锁。

终态寄存器相同、差别只在到达顺序 → "寄存器态→重训"触发非纯组合逻辑。候选解释：
① slot 使能写入顺序参与触发（正序 OVR0←1 在 OVR3←4 之前=先导位，逆序变收尾；
② kick 边沿语义——但 6→6 在正序触发了，站不住）。禁用-启用=PnP 周期的链路
down/up 是已证可靠重训杠杆，正序的立即钳=带内 kick 在全量态就位后触发。

## 轮五d（同日）：还原 dissociation——降速带内即时、升速要链路 down/up

9+3 复现钳 Gen2 后，跑全量还原命令（11 寄存器回原值 + 收尾 kick）：

- **立即 LnkCap 回 Gen3**（GPU-Z linkcap 3.0）——能力向量随寄存器还原 + 带内
  kick 即时恢复，**GPU 侧无隐藏脏寄存器**（"还有没有不干净的地方"就此终结）；
- **LnkSta 仍挂 Gen2**；禁用-启用后 **3.0/3.0** 完全恢复。

不对称结论：**降方向（Gen3→Gen2）带内即时**（轮三），**升方向（Gen2→Gen3）
要链路 down/up**。候选机制：①LTSSM 带内路径只做降速，升速需完整重训；②钳制
是双边的——驱动在钳制期把上游桥（00:01.0）的 TLS 也镜像到了 Gen2，我方还原
只清了 GPU 侧，桥侧 Gen2 压着协商结果（可测预测：下次还原后禁用-启用前，用
HWiNFO/GPU-Z 读根口 00:01.0 的链路状态，应见 Gen2）。实用配方定型：**还原命令
+ 禁用-启用 = 干净回 3.0/3.0**。

## 轮六（同日）：归一化二分——状态层 6 写封顶，触发层=驱动重初始化事件

- **R1 负结果**：{OVR0, MAX_RATE, PL_LINK_RATE} + slot3 → 不钳（VSEC_HIERARCHY
  疑似状态必要项；R1 是否做过 PnP 未记录，待补测）。
- **逻辑出列两项**：0x8c2c0（同值写 no-op）、0x8e120（VAL0←0=原值 no-op）。
- **R2c 正结果**：{OVR0, MAX_RATE, PL_LINK_RATE, VSEC_HIERARCHY} + slot3（6 态写）
  → PnP 后钳 Gen2——**状态层 6 写充分**；TLS/PRIV_MISC_1 非状态项。
- **触发层排除**：slot3 单打（轮五b）、PRIV_MISC_1 单加、TLS 单加、两项合取、
  30–60s 时延、**空闲降速-回升循环（T3，P5/P8 自然重协商不触发）**、带内 kick
  ——全部不立即。
- **nvoc-cli restart-display-driver = 双向触发器**（nvoc 项目工具）：钳制态刷新
  到 Gen2、还原态刷新到 Gen3——**NVIDIA 驱动重初始化事件会重读能力向量并应用**；
  设备管理器禁用-启用同效（全 PnP 版）。硬件层重协商（idle 循环）不应用。
- **归一化终态**：
  - 进 2.0 = `OVR0(0x8e110←1) + MAX_RATE(0x8c040←0x800c5800) +
    PL_LINK_RATE(0x8c1c0←0x150036) + VSEC_HIERARCHY(0x88610←1) + VAL3(0x8e12c←
    0x16480000) + OVR3(0x8e11c←4)` + 驱动重初始化；
  - 回 3.0 = 上述 6 项回原值（0x8e110←0 / 0x88610←0x1001 / 0x8c040←0x80045800 /
    0x8c1c0←0x130036 / 0x8e12c←0 / 0x8e11c←0）+ 驱动重初始化。
  - 对称干净：进/出同构 6 写，TLS/PRIV_MISC_1 从配方中彻底退役。
- **未决尾巴**：①轮三立即性未复现（当时无驱动重启发生，孤例处理）；②轮五d
  kick 能抬能力向量但 T 系 kick 不能压（单向性注脚）；③kick 是否必要未单独测
  （R2c 含 kick，可试 6 写无 kick + restart-display-driver）；④降阶梯未走完：
  VSEC 必要性（R1 PnP 缺证）、MAX_RATE vs PL_LINK_RATE 单项、OVR0 必要性、纯
  slot 对（5b 当时无触发器配合，需用 restart-display-driver 复测）。

## 轮七（同日）：二分到底——**3 态写 + 驱动重启 = 终极配方**

- T-A（去 VSEC）不钳 → VSEC 必要；T-E（纯 slot 对）不钳（复测确认 5b）；
  **T-D 钳**（{VSEC, MAX_RATE, VAL3, OVR3}，OVR0/PL_LINK_RATE 双出列）；
  **T-F 钳**（{VSEC, VAL3, OVR3}）→ MAX_RATE 出列。
- 逻辑闭合：T-E ⊂ T-F 且 T-E 败 = VSEC 必要；T-F 仅 3 项且充分 = **极小集**。

### 终版配方（12 写 → 3 写 + 1 条触发命令）

```
# 进 2.0（3 态写 + 驱动重初始化）
ssdiag.exe --target-offset 0x88610 --value 0x1 --target-offset 0x8e12c --value 0x16480000 --target-offset 0x8e11c --value 0x4
nvoc-cli restart-display-driver

# 回 3.0（对称还原 + 驱动重初始化）
ssdiag.exe --target-offset 0x88610 --value 0x1001 --target-offset 0x8c040 --value 0x80045800 --target-offset 0x8e12c --value 0x0 --target-offset 0x8e11c --value 0x0
nvoc-cli restart-display-driver
```

（回程含 0x8c040 回厂值属保险项——T-F 已证非必要，不写它就不会脏；保留只为
兼容任何用过旧配方的会话。）

### 语义压缩

- `0x88610` VSEC_HIERARCHY 清 bit12 = **武装位**（bit0 保留；bit12 语义未解码，
  但它是必要先导）；
- `0x8e12c` VAL3 ← **本卡 PCIE_MAGIC_D 值**（0x16480000，逐 die 映射）；
- `0x8e11c` OVR3 ← 4 = slot3 使能；
- `nvoc-cli restart-display-driver` = 触发（驱动重 init 重读能力向量）；
- 全部 power-cycle-volatile； LnkSta 升方向需驱动事件、降方向可带内即时。

## 附

- `post-state-run1.txt` / `post-state-run2.txt`：两轮完整 inventory（config Gen1↔Gen3
  动态 + TLS=Gen2 持有的原始记录）。
