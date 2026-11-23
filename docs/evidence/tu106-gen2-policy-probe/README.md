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

## 附

- `post-state-run1.txt` / `post-state-run2.txt`：两轮完整 inventory（config Gen1↔Gen3
  动态 + TLS=Gen2 持有的原始记录）。
