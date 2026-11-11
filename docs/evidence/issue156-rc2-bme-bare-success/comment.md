## rc2（BME 换件）裸跑：**全绿，免 marker 支持成立**

`BOOTX64.EFI.2.1.12-rc2`，tx sha256 `c99779df…`（build 时间 `2026-11-10 15:40:33`），**无任何 marker**，默认 HIGH 裸跑。

### 完整链路

```
[  4179ms] TX: >4GB staging gate: true (family tu10x)
[  5568ms] FWSEC-TU10X: STARTCPU post-START CPUCTL=0x00000000 (pre=0x00000010; 0x0 = core launched)
[  5571ms] FWSEC-TU10X: *** WPR2 UP lo=0x1ffe000 hi=0x1ffee00 after 3ms ***
[  6054ms] BOOTER-DIRECT: fire meta dev=0x10001f000 mbox0=0x0001f000 mbox1=0x00000001
[  6162ms] BOOTER-DIRECT: halted, mailbox0=0x89 mailbox1=0x1 exci=0x00900100      ← 预期的高位失败
[  6803ms] STAGING-FALLBACK: booter 0x89 with host meta magic intact = ... - one retry after a
           function-level FLR + FWSEC re-ignition, LOW band
[  7148ms] STAGING-FALLBACK: FLR done (cold engine, WPR2 wiped) - FWSEC re-ignition follows
[  7169ms] STAGING-FALLBACK: GPU BME re-armed (cmd=0x0003 -> 0x0007) - the FWSEC BL sysmem DMA fetch needs bus master
[  7445ms] FWSEC-TU10X: FBIF trans4=0x00000015 dmactl cleared (vram_staging=false)
[  7466ms] FWSEC-TU10X: STARTCPU post-START CPUCTL=0x00000000 (pre=0x00000010; 0x0 = core launched)
[  7469ms] FWSEC-TU10X: *** WPR2 UP lo=0x1ffe000 hi=0x1ffee00 after 3ms ***   ← 上一轮这里是 5s 超时
[  7561ms] STAGING-FALLBACK: FWSEC re-ignited, WPR2 re-verified - re-staging in the LOW band
[  7748ms] BOOTER-DIRECT: fire meta dev=0x797ac000 mbox0=0x797ac000 mbox1=0x00000000
[  8026ms] BOOTER-DIRECT: halted, mailbox0=0x0 mailbox1=0x0 exci=0x00900100
[  8386ms] STAGING-FALLBACK: LOW-band retry verified - staging rebound to the LOW buffers (staging=low(fallback))
[  8427ms] SELECTOR RESULT: SS0=0x88888888 SS1=0x8
[  8765ms] TU10X FEAT sweep 0x409650 = 0xffffffff
[  8765ms] TU10X FEAT sweep 0x409664 = 0x88888888
[  8765ms] TU10X FEAT sweep 0x40966c = 0x00000008
[ 10642ms] FINAL: transaction complete, all DMA released
[ 10692ms] HANDOFF-OUT: ... wpr2_lo=0x01f77000 wpr2_hi=0x0 canary=intact(0x0164c0de/0x0165cafe)
           pmcsr=0x0008 dstate=D0 | tu10x: plm=0xffffffff ss0=0x88888888 ss1=0x00000008
           readout=0x00000000 | tu10x:unlock=true
```

### 几个读数

- **BME 就是那个缺的上下文**：重臂 `0x0003 -> 0x0007` 之后，同一条 FWSEC 装载路径 3ms 起 WPR2，上一轮是 5s 空等。根因与你上一轮的推断完全一致。
- **新的判别行落地且读数正确**：两次 STARTCPU 都是 `post-START CPUCTL=0x00000000 (pre=0x00000010)` → `0x0 = core launched`。这条行以后能直接把“START 没贴上 / 核起了又死”分开。
- **回退段总代价 1624ms**（6803ms 门 → 8427ms selector 写回），比预估的 2–3s 还低一点。全程 10.7s 交棒，比你带 marker 的原生 LOW 轮（13.4s）还快 2.7s——因为省掉了高位轮的大缓冲分配。
- `canary=intact`、`wpr2_lo=0x01f77000` 与 heap 起点自洽、`plm=0xffffffff`、`readout=0x00000000`。
- 尾部四字段 `0xbadf5040` 照旧（GOP 绑定 artifact），判读走 `tu10x:` 元组。

### 结论

`1e93`（RTX 2080 SUPER 移动版 DGFF，Alienware Area-51m R2）**免 marker 支持成立**：默认 HIGH 撞 0x89 → 自动 FLR + BME 重臂 + FWSEC 重点火 → LOW 带重落 → 交付解锁。矩阵里那一行可以改成「默认配置直接支持，无需 `\tx-low-staging.txt`」。

三轮现场回收的完整链条：rc1 裸跑（FBIF 写面死）→ rc2 裸跑（FWSEC 缺 BME）→ rc2 换件裸跑（全绿）。每轮都往下一层，最后一层就是总线主控位。

rc2 可以转正了。

---

### 附：ssdiag 0.7.2（换了新附件，顺手试了）

**inventory 段跑通了**，而且读到了有用的平台字段：

```
GPU 1 family: tu10x (Turing) - IN-SCOPE (unlock transaction family)
GPU 1 VBIOS: 90.04.96.00.9a (nvapi)
GPU 1 driver: 32.0.15.8195 (581.95), dated 11-18-2025; EnableGpuFirmware = 1 (GSP firmware mode)
CHASSIS: laptop/portable (SMBIOS chassis type 10)
ADVISORY: laptop chassis - the volatile unlock can drop on sleep/RTD3/MUX events
```

但**驱动那一步还是 `StartService failed: win32 1450`**（与 0.6.4 同码、同位置），所以系统内 FEAT 读回仍然拿不到。看是不是 0.7.2 的驱动装载路径和 0.6.4 共用，这条我在 ssdiag 的独立条目里跟，不占 #156。

（顺带：这台是 `EnableGpuFirmware=1` 的 GSP 模式，与矩阵里要求一致。）
