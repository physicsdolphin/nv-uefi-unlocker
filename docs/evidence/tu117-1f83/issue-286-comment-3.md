# #286 评论（可直接粘贴）：补充数据（三）——探针矩阵、FEAT 排除、首份 16 系物理熔丝行

> 用法：整段贴到 #286 下。若想独立成 issue，把第一行标题换成
> `TU117 1f83：FEAT 全部排除、残留为唯一嫌疑 + 首份 Turing 11X 物理熔丝行 dump` 即可。

---

## 0. 一句话

**FEAT 状态已被全部排除**：门开 / 门关 / SS 满速 / SS 全零 / 完全原厂，只要
"FWSEC+booter 跑过且交棒前没有复位"，驱动一律 Code 43。同时附上**首份 Turing 11X 的 256 行物理熔丝 dump**
（CRC 守卫按声明降级，双 pass 一致、地址已恢复）。

---

## 1. 探针轮矩阵（0.8.9 `--target-offset/--value`；每轮 poke → 设备管理器禁用/启用 → 读状态）

| 时间 | 轮 | poke | 读回 | 门 | 设备状态 | 备注 |
|---|---|---|---|---|---|---|
| 22:21:06 | A | `0x409664<-0x0`, `0x40966c<-0x0` | readback OK | `0xffffffff` 开 | **Error / 43** | SS 全零（出厂语义）仍拒 |
| 22:22:41 | B | `0x409664<-0x88888888`, `0x40966c<-0x0` | readback OK | 开 | **Error / 43** | SS1=0 不救 |
| 22:22:56 | C | `0x409650<-0xffffff8f` | `0xffffffef`（sticky，属正常） | 关 | — | 关门成功 |
| 22:27:21 | A(重) | 同 A | readback OK | 开 | **Error / 43** | A 可复现（中间重跑过一次 U 盘事务） |
| 22:28:10 | B(重) | 同 B | readback OK | 开 | **Error / 43** | B 可复现 |
| 22:28:43 | C(重) | `0x409650<-0xffffff8f` | `0xffffffef` | 关 | — | — |
| 22:28:47 | O | `0x409650<-0xffffffff` | **`0xffffffef`（写被丢）** | 仍关 | — | **门开不回来** |
| 22:28:56 | D | `0x409664<-0x99999999` / `0x40966c<-0x9` | **`0x88888888` / `0x0`（被丢）** | 关 | — | 关门后 SS 也写不进 |
| 22:28:59 | G | `0x409664<-0x8888` / `0x40966c<-0x8` | **`0x88888888` / `0x0`（被丢）** | 关 | — | 同上 |

（A 轮结束后 ssdiag 判读变 `NOT_UNLOCKED` 是**预期**：该判读就是看 SS 值；本轮真正结果是 `Status=Error`。）

### 三个新事实

1. **SS 内容无关**：A 轮 = 门开 + **SS 全零**，仍 Code 43，且复现两次 ⇒ "驱动只对 SS 速率内容敏感"的假设在 `1f83` 上不成立。
2. **`0xffffffef` 在本卡是单向门**：C 之后 O（`0xffffffff`）与 D/G 的 SS 写**全部被丢**（读回完全不变）。
   与 TU102 bench 的"门可热锁回 / 热开回"不一致。建议矩阵里**所有"开门"轮次必须排在关门之前**。
3. **PnP 禁用/启用不复位**：全程 `CANARY=intact`（含 A/B 两个 cycle）⇒ bench 这条在本机成立，poke 值能活过 disable/enable。

> 补充：`C` 轮（门关 + 驱动 re-init）在本会话没跑到——脚本把 sticky 读回判成 MISMATCH 后跳过了 cycle。
> 该格已有 11-22 的独立现场：`\tx-relock.txt` 交棒 `plm=0xffffffef`、`unlock_persist=true`，
> 进系统后启用驱动 = 黑屏 → 恢复 → **Code 43**（ssdiag 仍 `PLM=0xffffffef`、SS 在、`canary intact`）。

---

## 2. replay + `tx-force-continue` 轮：FEAT 完全原厂也拒（但有一个混杂）

配置：rc7 + `tx-force-continue.txt` + **v67 移除**（`payload_mode=false`），驱动**未禁用**。

```
[18322ms] pre-FLR selectors: ss0=0x27123020 ss1=0x3
[19332ms] FWSEC-TU10X: *** WPR2 UP lo=0xffe000 hi=0xffee00 after 3ms ***
[20894ms] BOOTER-DIRECT: halted, mailbox0=0xb
[22434ms] TX: FORCE-CONTINUE: booter failed but the marker file demands the unwind - the payload ROP never ran
[25601ms] POST-GOP: FEAT_PLM=0xffffff8f SS0=0x27123020 SS1=0x3 READOUT_1=0x00000000 ... CANARY=intact
[26354ms] HANDOFF-OUT: ... tu10x:unlock=false
```

系统内（`ssdiag7`）：`PLM=0xffffff8f`（原厂）、`SS0=0x27123020`、`SS1=0x3`、`READOUT=0`、`CANARY=intact`、
**`WPR2_LO/HI=0x00f84000/0x00ffee00`**、设备 Code 43。

**混杂（很重要）**：这一轮 WPR2 **仍是 loader 交棒时的窗口**（`0xf84000/0xffee00`），
说明驱动看到 `WPR2_HI != 0` 后**跳过了自己的 FWSEC**（对比门开轮 `ssdiag3` 的
`WPR2=0x00fcd000/0x00fcde00` 是驱动自建）。所以：

- 本轮失败**更早、更快**（没有长时间黑屏），**不能**当"纯残留"对照；
- **干净的 FEAT 排除证据是 A 轮**：那里 payload 跑过（`WPR2_HI` 被 payload 写 0）⇒ 驱动跑了自己的 FWSEC，
  SS 全零、门仍开，仍 Code 43。

顺带一条可能有用的观察：**payload 的第 2 个写（`WPR2_HI=0`）实际上决定了驱动走哪条初始化路径**；
replay 没有这个写，驱动就直接跳过 FWSEC 并在更早的阶段失败。

---

## 3. 物理熔丝行 dump（首次 Turing 11X，256 行全量见附件 `fuserows.log`）

命令与会话（**stock 会话，未跑事务**；驱动 621.24 GSP）：

```
ssdiag-x86_64-windows-0.8.9.exe --fuserows
```

```
  WPR2_LO= 0x00f3b000   WPR2_HI= 0x00f6c200           (driver's own window)
  CANARY_A/B = 0x00000000/0x00000000 -> cleared       (never armed = no transaction this boot)
  turing 0x4096xx  PLM = 0xffffff8f (closed (stock)) / SS0 = 0 / SS1 = 0 / READOUT = 0
  VERDICT [turing]: NOT_UNLOCKED
```

守卫与结果：

```
--- FUSE ROW DUMP (WRITE MODE) - writes: FUSEADDR row numbers + FUSECTRL CMD_READ only; no programming command; address restored after ---
  protocol: ga100-ff-tools read_rows (two passes, latch checks); a bound kernel driver is a concurrency risk - pass mismatches mark rows unreliable
  FUSEROWS DEGRADED:: CRC_STATUS 0xbadf5040 unreadable at this base (Turing: undecoded/PLM-blocked at PL0) - proceeding WITHOUT the CRC guard; latch verification, two passes and address restore remain active
  domain: rows 0..255 (IFF 0x00ff00b9, saved FUSEADDR 255) - two passes
  FUSEROWS OK: both passes complete, address restored
two passes match (256 rows)
```

开头 8 行（除 0x0/0x2/0x4/0x8… 成对重复外，0x1a/0x1b、0x18/0x19 全零）：

```
FUSEROW 0x0 = 0x001fd63d
FUSEROW 0x1 = 0x001fd63d
FUSEROW 0x2 = 0x879d554c
FUSEROW 0x3 = 0x879d554c
FUSEROW 0x4 = 0xf0d467d4
FUSEROW 0x5 = 0xf0d467d4
FUSEROW 0x6 = 0x04800007
FUSEROW 0x7 = 0x04800007
FUSEROW 0x8 = 0x2440d917
...
```

已知可读的相邻寄存器侧对照（同一次输出）：

```
speed-sel  SM_SPEED_SELECT_DP     0x00021224= 0x00000000
speed-sel  SM_SPEED_SELECT_FFMA   0x0002159c= 0x00000000
speed-sel  SM_SPEED_SELECT_FMLA16 0x000217d4= 0x00000000
speed-sel  SM_SPEED_SELECT_FMLA32 0x000217d8= 0x00000000
speed-sel  SM_SPEED_SELECT_IMLA0..4 0x000217dc..0x000217ec = 全 0x00000000
opt-dis    GEN3_SPEED_DISABLE     0x00021250= 0x00000000
fusectl    EN_SW_OVERRIDE         0x00021040= 0x00000001
FUSE_VER / FUSE_VER_DECODE (0x824140/0x8241c0) = 0xbadf5040 [PRIV_VIOLATION]   ← PL0 挡
```

值得你解码时优先看的几行（其余全量在附件）：
`0x47=0xffffd967`、`0x48–0x4a=0xffffffff`、`0x4b=0x271dffff`、`0x55=0xc000078e`、
`0x57=0x00000163`、`0x5a=0xe98a5400`、`0x5e=0xcb356000`、
`0x60–0x85` 是一整段结构化行、`0x86–0xb8` 全零、
`0xb9–0xf4` 呈强结构（可读作若干条 4 行一组的记录）、
`0xea=0xffffff8f`、`0xec=0xffffff8f`、`0xfe=0xdffffff3`、`0xff=0x2000000c`。

说明两点：
- 我用的是 **0.8.9**（同源，`--fuserows` 可用）；若你需要与 `9fb4969d…` 的 0.8.8 逐字节对齐，我可以再用 0.8.8 重跑一份。
- 这份是**出厂签名**（stock 会话、无解锁、CANARY 0/0），可直接当 consumer 侧锚（对照 CMP 30HX=TU116-100）。

---

## 4. 结论与建议

1. 建议把 #280/#286 的 **"SS 速率敏感性"假设标为否证**（A/B 两轮 + relock 轮 + replay 轮四角覆盖）。
2. 目前唯一稳定的判据是：**"FWSEC/booter 跑过且交棒前未复位" → Code 43**；agitate-only（不跑 FWSEC）与
   "replay 走紧急 FLR"（复位清干净）都正常 —— 两头夹住了"残留"这个变量。
3. **想请你判断的问题**：TU11x（16 系）驱动的 init 在"发现 GPU 已被预初始化"时会检查哪一项？
   - 入口 `WPR2` 的形态（`0x{f8400000}/0` vs 冷态 `0x1ffffe00/0`；本轮证明它直接改变驱动走的初始化路径）；
   - GSP falcon 残留（我们的 BL 在 IMEM `0xfe00`、EXCI 锁存 `0x900100`）；
   - SEC2 DMEM 的 sanitize 图案（`0xdead5ec2`）；
   - `BSI_SCRATCH15` / 交棒状态；
   - PBUS 软件 scratch 里的金丝雀（`0x1480/0x1484 = 0xc0deca7e/0xcafe`）。
   你指一项，我就跑对应的单变量消融（`\tx-no-canary.txt` 那轮我已经准备好，随时可跑）。
4. 仍待补：Code 43 轮的 `DEVPKEY_Device_ProblemStatus`（NTSTATUS）/ `nvlddmkm` 事件序列——我会尽快补上，
   以便和 #246 的 `RmInitAdapter failed! (0x62:0x40:2674)` 对齐到同一精度。

附件：`fuserows.log`（874 行，含 256 行 FUSEROW 全量 + 前后寄存器读数）、`loader13.log`、`ssdiag7.log`、
`exp-logs/*`（探针轮原始输出）。
