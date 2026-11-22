# #276 XPS 8940 重编号族收官（2026-11-23）：bus-number hiding 诊断 → rc7 总线路由恢复 → 同 boot 解锁

## 实验问题

Dell XPS 8940（0427JK，i7-11700，Rocket Lake-S）+ Dell OEM RTX 3060 Ti `10de:2486`（GA104）：**每次 UEFI boot 枚举时 PEG 槽读 vendor 0xffff**，loader 报 "no transaction target"；而卡全程驱动 console（DP/HDMI 双口验证），Windows 里健康且编号 43:00.0。iasl 反解证明 DSDT/SSDT **无任何 ACPI 电源资源**管这条槽（V0PR/V1PR/V2PR 属核显侧 VOL 设备）。

## 判别与裁决（rc6，boot 1）

rc6 的 PORT-DIAG 一次裁决家族：PEG 口 `00:01.0` 读 `D0 / ldis=0 / DLLLA=1（Gen3 x16 训练着）/ secbus=0x00 / bcr=0x1A（VGA Enable 仍在）`，死功能句柄创建于 bus 2——**link 在、显示在、供电在，唯独总线路由被清零**。家族=重编号族（极端形态：sec/sub 被清 0/0）。

机理定性：**bus-number hiding（总线号隐藏）**——固件删掉桥的路由标签而非动设备：卡从未复位（scanout 连续、链路稳定）、从未断电（显示引擎活着）、config 内容（BARs 含）从未丢——死的只是「发往 bus 2 的 config TLP 的可达性」（master-abort→0xffff）。Windows 43:00.0 = pci.sys 自己重编号写的新标签。规范层面 spec-legal：sec/sub=0/0 即上电默认值，UEFI 对 handoff 时桥编号无不变量（详见 issue 内 spec 讨论）。

## rc7 修复 + 复活实录（boot 2，2026-11-23 01:05）

rc7 两修：①父口匹配 devpath 前缀优先（编号无关；rc6 的 bus 包含判 `2∉[0,0]` 在配对步就 decline，杠杆不可达——boot 2 of rc6 实证）；②`NoneRelocated` 从 decline 升级为行动杠杆。

实录（`loader.log.0105-rc7-transaction-UNLOCKED.txt`）：

```
SLOT-REVIVE: 0000:02:00.0 dead under port 0000:00:01.0 (sec=0x00 sub=0x00) ... LA=1 spd=x3 w=x16 lever=NoneRelocated
SLOT-REVIVE: dead slot ROM identity: vendor=0x10de device=0x2486 (NVIDIA)
SLOT-REVIVE: bus registers restored 0x00000000 -> 0x00020200 (readback 0x00020200)
SLOT-REVIVE: 0000:02:00.0 vendor reads 0x10de after the bus restore - ROUTING-ISOLATED, never dead
TX: ===== target 1/1: 10de:2486 @ 0000:02:00.0 =====
```

要点：

- **零复位、零 BAR 恢复**——config 路由恢复后 vendor 直接读回：设备的 config 空间（含 BAR）原样存活，重编号族根本不需要复位杠杆；
- **ROM 身份门首战立功**：config 死状态下从固件缓存影子读出 `0x2486`（3060 Ti），N 卡门通过；audio fn `02:00.1` 无 ROM → 按「不可识别不重置」decline（无害——bus 恢复按总线生效，audio 随 GPU 复活）；
- **交易在原始 bus-2 地址运行**（Windows 接管后才变 43）——这本身就是重编号诊断的实证。

全链：`SS0=0x88888888 SS1=0x8` → `POST-FLR verified` → canary armed+intact → `POST-GOP: unlock intact` → `HANDOFF-OUT: ga/ada:ss0_unlock=true ss1_unlock=true plm_open=true`。系统内同 boot（`ssdiag-20261122-UNLOCKED.txt.txt`）：`0x00823804 = 0xffffffff (OPEN)`、`VERDICT [ampere]: UNLOCKED (override values live)`、`CANARY: intact + SS live`、`POWER: 卡与根口双 D0 -> rail intact`。

## 文件

| 文件 | 内容 | SHA-256（前 16） |
|---|---|---|
| `loader.log.0105-rc7-transaction-UNLOCKED.txt` | rc7 复活+交易全量日志（报告者证据库原文） | `b3d3490a6e0fecf6` |
| `ssdiag-20261122-UNLOCKED.txt.txt` | 同 boot 系统内裁决 | `f8cbd272076730a9` |

上游：报告者证据库 `samlee-wq/xps8940-gpu-unlock-evidence`（含 acpi-decomp、四个 boot 的 loader.log、2234-revive-declined）。

## 边界

- 单机 n=1；性能基准报告者承诺后续（本档先记解锁与判别链）；
- 逐 boot 间歇性：11-20 一晚该机曾正常枚举并跑过事务（canary cleared、NOT_UNLOCKED）——门控按 boot 路径变量触发，触发变量未定位；
- ASM1080 类**无 ROM 死槽**（桥/无影子女设备）会被 ROM 门永久 decline——`rom_identity` 对它们返回不了身份；未来可加 `\tx-revive-id.txt` 显式身份 marker，暂不做（无需求）；
- 解锁持久性：卡回到 bus 2 后交棒 Windows，Windows 重编号到 43——RTD3/睡眠行为与普通 GA104 桌面卡一致预期，待报告者日常使用数据。
