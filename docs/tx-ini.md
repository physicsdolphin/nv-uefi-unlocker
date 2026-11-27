# `\tx.ini` — 可选聚合配置（issue #290）

> 状态：**实验性新面**（feat-tx-ini 分支）。marker 文件仍是完全有效的配置方式；
> `\tx.ini` 是可选的收纳层，不引入任何新行为，只改变"怎么写配置"。

## 一句话

把盘根十几个零字节/单行 marker 文件收进一个 `\tx.ini`。没这个文件 = 与从前
逐位相同；有这个文件 = 同名臂以 ini 为准；**两处同时设同一臂 = 拒绝引导**
（日志逐行列出冲突，删掉一边再跑）。

## 语法

- 编码：UTF-8（带不带 BOM 都行）。**UTF-16 会被拒绝**并提示重存（记事本
  "另存为"选 UTF-8 或 ANSI）。
- 行尾：CRLF / LF / CR 均可；`#` 或 `;` 开头为注释。
- 布尔臂：`key` 单独一行 = 开；`key = true` / `key = false` 亦可。
- 值臂：`key = value`（等号两侧空格随意）。
- 列表臂（`ignore_key` / `input_reset` / `gop_hide`）：**同键多行**，每行一条，
  语义等同 marker 的"每行一条"。
- 错误处理：解析/取值错误（拼错键值、越界、重复键、UTF-16）= **拒绝引导**，
  GPU 零写入，`INI-REFUSED:` 行给出文件名+行号；未知键 = `INI-WARNING:`
  警告后继续（台架旋钮故意不进 ini，见下）。文件上限 64 KiB。

## 键表

### 归并族（多 marker → 一键）

| 键 | 取值 | 替代的 marker |
|---|---|---|
| `output` | `normal` / `quiet` / `ultrafast` | `tx-quiet.txt`、`tx-ultrafast.txt` |
| `log_commit` | `full` / `lines` / `off` | `tx-log-commit.txt`、`tx-log-commit-lines.txt` |
| `reconnect` | `on` / `off` | `tx-no-reconnect.txt` | off=跳过 native-GOP 重建，暗交棒；**默认 on，rebar 臂激活时默认 off**（对写+重连=已证 Code 12 族） |

> **staging 不是键（2026-11-25 终态）**：自动重试梯（booter 侧 STAGING-FALLBACK、
> FWSEC 侧 FWSEC-FALLBACK）端到端拥有带选择权，无任何手动档。三个历史 marker
> （`tx-low-staging.txt`、`tx-fwsec-high-staging.txt`、`tx-fwsec-vram-staging.txt`）
> 全部退役（零探测，盘上惰性）——VRAM 预设的唯一收益（省一次失败尝试 + 一次
> 函数级 FLR）发生在 GOP 已断、屏幕已黑的窗口内，操作者无感知。

> **退役（2026-11-25）**：legacy 递归重连路径整体删除，`tx-reconnect-legacy.txt`、
> `tx-reconnect-nonrecursive.txt`、`no-reconnect.txt` 一并退役；同轮退役的还有
> `tx-stop-on-failure.txt`（失败路径无条件走 emergency FLR + 下一卡）、
> `tx-keep-assets.txt`（资产释放无条件执行）、staging 三 marker（见上）、
> `tx-meta-in-vram.txt`（#89 证伪轴）、`tx-sec2-c-load.txt`（#17/#62 C 参照火，
> 连函数删除）、`tx-sec-base.txt`/`tx-bootvec-sec.txt`（nsstub 摆位季）、
> `tx-no-bridge-master.txt`（BME 使能无条件）、`pre-flr-on.txt`（pre-FLR 实验
> 完结，kernel-matching 流终胜，函数删除）、`tx-accept-warm-wpr2.txt`（温 WPR2
> 逃逸口；宪法正确恢复=断电）、`tx-require-d0.txt`（warm 入口的响亮日志+强制
> D0 读回即是现场分辨）、`tx-sbr-gpu.txt`（SBR 内建进所有 rebar 臂——XVE 页
> reset-gated 是硬前提不是旋钮；纯解锁 boot 的事务内 SBR 实验臂同轮退役）。
> 全部零探测、盘上惰性。
> legacy 重连退役依据：早于 native-GOP 重建默认（v2.1.10），自带 #35/#61
> 递归死锁家族与 R7000 DDC-CI 链路未训练形态；#52/D#131 的 nonrec 实证全部
> 早于 native 默认。重建异常的逃生阀现在是 `reconnect = off`（暗交棒，
> bootmgfw 自建控制台，Z170-A 对照已证；rebar 激活时已是默认）。

### 布尔臂（`= true` 可省略）

`no_log`、`no_console`、`dry_run`、`agitate_only`、`agitate_stage`、`fwsec_only`、`exit_fw`、
`relock`、`force_continue`、`preload`、`ebs_watch`、`ebs_selftest`、
`host_probe`、`acpi_dump`、`gop_no_reorder`、`rebar_gop`、
`rebar_then_tx`、`no_asset_cache`

（逐条语义见 FAQ 的 marker 表对应行；键名 = marker 名去掉 `tx-` 前缀和
`.txt`，`tx-` 族外的 `input-reset.txt` → `input_reset`。）

> 注意：`dry_run` / `agitate_only` / `fwsec_only` / `exit_fw` 是**四个独立布尔**，
> 不是单选 `mode`——marker 世界的合法组合（如 dry-run + exit-fw）必须原样
> 可表达。

### `agitate_stage`（D#297 手术元素消融阶梯，诊断 profile 专用）

`full` / `no_bridge_bme`（-桥 BME，rc7 已证伪臂，复现锚）/ `no_endpoint_command`
（-GPU COMMAND 写，IO|MEM|BME 不动卡，BAR 读预期 garbage）/ `no_dma`（-DMA
churn）/ `disconnect_only`（只做断连：GOP 目标句柄+PCI 句柄，无 claim/COMMAND/
DMA/BME/重建；**rc10 起每次断连前后打全量协议引用计数清单**——逐驱动 open
信息 + 所属镜像 base+size，即 Stop() 伤亡名单，H-CM vs H-Stop 判别读数）/
`no_disconnect`（rc10 新增，**claim-first**：-全部 DisconnectController 调用，
EXCLUSIVE claim 直接打在未动驱动栈上；claim 被 BLOCK 时打阻塞者清单并中止——
不回退到断连，回退等于重新引入被测调用路径；native-GOP 重建随断连一起 held
OUT；活 = "断连是 claim 前置"在该机上被证伪）。每级都是完整手术的
leave-one-out（disconnect_only 除外=最小臂）；全部零 GPU 写。与 `agitate_only`
同设=INI-REFUSED（二义性拒跑）；本键为 ini-only（无 marker 双胞胎，诊断
profile 专用）。

### 值臂

| 键 | 值 | 替代的 marker | 严格性 |
|---|---|---|---|
| `i_know_what_i_am_doing` | 秒数 0..=30（裸键 = 默认 5） | `i_know_what_i_am_doing.txt` | ini 严格（marker 宽松钳制） |
| `chain_target` | EFI 路径 | `chain-target.txt` | 站点解析不变 |
| `prom_sha256` | 64 位十六进制 | `tx-prom-sha256.txt` | 站点解析（不符 = 拒） |
| `select_bdf` | BDF | `tx-select-bdf.txt` | 站点解析 |
| `fb_gb` | GiB 1..=64 | `tu10x-fb-gb.txt` | ini 严格 |
| `native_gop` | 卷上 GOP 模块路径 | `tx-native-gop.txt` | 站点探测 |
| `flush_lines` | 0..=4096 | `tx-flush-lines.txt` | ini 严格 |
| `rebar` | `read` / `scan` / `<target>/<advertise>`（如 `11/9`；**恒斜杠形，0=该侧不写，`0/0`=整臂跳过**；裸 `11`/半斜杠拒跑） | `tx-rebar.txt`、`tx-rebar-read.txt`、`tx-rebar-scan.txt` | ini 严格（target 0..=19，advertise 0..=15）。**SBR 内建**：所有 rebar 臂自动先跑纪律窗 SBR（XVE 页 reset-gated，无 SBR=硬死锁），`sbr_gpu` 臂已随内建化退役 |
| `rebar_route` | `mmio` / `ecam` | `tx-rebar-ecam.txt` | — |

### 列表臂

| 键 | 每行格式 | 替代的 marker |
|---|---|---|
| `ignore_key` | 键码（同 marker 每行一键） | `tx-ignore-key.txt` |
| `input_reset` | `usb VVVV:DDDD`（十六进制） | `input-reset.txt` |
| `gop_hide` | `bb:dd.f` / `dddd:bb:dd.f` / `vvvv:dddd` | `tx-gop-hide.txt` |

## 故意不进 ini 的（台架/消融旋钮，marker-only）

`tx-no-canary.txt`、`tx-ptimer-seed.txt`、`tx-no-gsp-halt-gate.txt`、
`tx-no-pm-preflight.txt`、`tx-no-flr.txt`、`tu10x-no-ss.txt`、
`tx-revive-dead-slot.txt`、`tx-revive-force-sbr.txt`、
`gop-disconnect-all.txt`（2026-11-25 自 tx-gop-disconnect 改名——原名没说 ALL）

这些只在 evidence 指引里点名出现；写成 ini 键会得到 unknown-key 警告。
零字节文件本身就是"你得知道它存在才能设"的使用门槛，保留。

## 示例

最小武装盘：

```ini
i_know_what_i_am_doing = 5
```

TU106 合并交付形态（SS 解锁 + BAR1 32G 同 boot，对应 marker 组合
`tx-rebar.txt`=11/9 + `tx-rebar-then-tx.txt` + `tx-no-reconnect.txt`；
SBR 无需单独 marker——2026-11-25 起 SBR 内建进所有 rebar 臂，旧
`sbr_gpu` 键与 `tx-sbr-gpu.txt` 已随上方退役节删除，残留文件/键零效果）：

```ini
i_know_what_i_am_doing = 5
rebar = 11/9
rebar_then_tx
reconnect = off
```

已验证机器的快速诊断盘：

```ini
i_know_what_i_am_doing
output = ultrafast
```

## 实现说明

- 解析器/模式表在 workspace 成员 `txini/`（no_std + alloc，romcore 模式），
  host 测试直接跑真码（17 用例：BOM/编码/行尾/重复键/冲突/越界/rebar 形状）。
- loader 侧 `loader/src/config.rs`：boot 最先 `config::init()`（logger 之前，
  静默采集），logger 起来后回放 `INI-WARNING:` / `INI-REFUSED:`；拒绝 =
  GPU 零写入、交回固件 boot manager。
- 内容臂的 ini 值重组为 `\n` 连接的字节流喂给**原有站点解析器**——两套
  配置方式共享同一套语义判定，不存在第二套解析。
- 冲突检测在 `txini::resolve` 内以 marker 探测闭包注入，与固件 I/O 解耦。
