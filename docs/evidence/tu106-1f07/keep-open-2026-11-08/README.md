# TU106 keep-open bench（2026-11-08）——交棒不重锁 FEAT 门，端到端健康

## 实验问题

Turing 交棒前的 `FEAT PLM 0x409650 ← 0xffffff8f` 重锁写是否必要。旧结论（第 13/14 火，2026-10-27，Run A/B）判"PLM 开着交棒 = Windows Turing 驱动死（Code 43）"，Run C（重锁）正常。本次实验怀疑该结论被叠加变量污染：Run A/B 时代 GSP 模式（EnableGpuFirmware=1）尚未成为 Turing 解锁的唯一受支持驱动路径。

## 本轮配置

- 卡：TU106 `10de:1f07`（台式机主力 bench）
- 构建：main `537f633` + 本地 knob（`\tu10x-keep-plm-open.txt` = 跳过重锁写），TX banner 验证过
  - loader.EFI sha256 `55c0e7360558c1c19d8c01b5f5027902fc740ff18e7200dc3be5efcec87f52e9`（449024 B）
  - 被替换的上一构建备份 sha256 `b5e36dba24b1e295523d2aaa5339cd03985e3d6d71607705f405a21fe5dc4943`（448512 B，默认重锁行为）
- 链路：FWSEC 点火 → WPR2 建立 → 原签 booter（BOOT_DIRECT）→ payload 开门（shape B）→ 主机写 SS0/SS1 → **跳过重锁** → Turing 跳过 FLR → native-GOP console 重建 → 链回 bootmgfw → Windows
- 冷态全链一次性通过，无 STOP、无黑屏

## 判据链（全绿）

1. 交棒前（`loader.log` 行 437）：`TU10X PLM keep-open: relock SKIPPED ... plm=0xffffffff SS0=0x88888888 SS1=0x00000008 unlock_persist=true`
2. 卡自身 GOP 驱动 StartImage 之后（行 487-488）：`POST-GOP: FEAT_PLM=0xffffffff SS0=0x88888888 SS1=0x8 READOUT_1=0x00000000 CANARY=intact` + `unlock intact (keep-open experiment)` —— **固件侧 console 重建不重关门**
3. Windows 驱动 init 之后（系统内 ssdiag）：PLM 仍 `0xffffffff (OPEN)`、SS0/SS1 覆盖值在位、READOUT `0x00000000 [full speed]`、`VERDICT [turing]: UNLOCKED (override values live)`、`CANARY: intact + SS live` —— **驱动 init 不重关门、无复位事件**

全程无任何一方（卡 GOP、bootmgfw、Windows 驱动）重关这扇门。

## 结论与后续

- **重锁写在当前栈（GSP 模式驱动）上不必要**。旧 Run A/B 负结果在 GSP 栈上未复现，归因于非 GSP 时代的叠加变量（未逐项坐实，但"开门交棒有害"在现栈不成立）。
- 直接后果（同日落主线）：三代统一**默认门开交棒**（Turing 加入 GA10x/Ada 本来的开门形态）；`\tx-relock.txt` marker 恢复旧重锁（Turing 臂=原位重锁；Ampere/Ada 臂=FLR 判定后重锁，两臂中后者从未上过 bench）。
- 门开的附带收益：系统内写入面开放——`ssdiag --reapply` 的 Turing 只读限制具备解除条件（工具侧待跟进），SS0 各 4 位速率字段的编码语义可在系统内自由实验。

## 记录局限

- 系统内 ssdiag 输出仅存摘录（`ssdiag-in-os-excerpt.txt`），完整输出与驱动版本号未归档。
- 硬件快门照片未拍；以上以 `loader.log` 全文（本目录）与摘录为准。
