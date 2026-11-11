# Issue #165 rounds 5–7：devnode 清理 A/B + 单卡换位（2026-11-10，报告者 junkjunker）

## Round 5：devnode 清理（Device Cleanup Tool 移除 M2000 phantom）→ 冷重启 → 双卡 pin 03

- 解锁全绿照旧（03=UNLOCKED），但**被解锁的 Gigabyte（SUBSYS_4072 @1C1C94C）仍 CM_PROB_NORMAL_CONFLICT**、ASUS=NONE、nvidia-smi 仅 04。
- **devnode 历史假说被排除**（第三假说退役；A4G 与之并列）。
- 附带的 system.evtx 为空导出（0 记录——日志在用时的常见伪象）；wevtutil 查询空结果也可能含 PowerShell 引号转义因素，未定。

## Rounds 6/7：换位单卡（swap-and-replace，物理互换不可行——ASUS 尺寸过大）

| 轮 | 单卡 | BDF | BAR0 | ssdiag | pnp | nvidia-smi |
|---|---|---|---|---|---|---|
| 6 | Gigabyte SUBSYS_4072 | **03:00.0** | f6000000 | UNLOCKED | **NONE** | 03:00.0 可见 |
| 7 | ASUS SUBSYS_8818 | **03:00.0** | f6000000 | UNLOCKED | **NONE** | 03:00.0 可见 |

**两张卡单独在目标位置都全绿**（解锁+可见+无冲突）。且单卡态 BDF 编号为 03:00.0——**总线编号随在场设备动态分配**，双卡态的"03/04"只是第一/第二枚举位，不是固定物理槽。

## 七轮矩阵收敛

| 轮 | 配置 | pin | 结局 |
|---|---|---|---|
| 1 | 双卡（A4G off） | 03 | 解锁 03 → 03 冲突 |
| 2 | 双卡（A4G off） | 04 | 双卡存活 |
| 3 | 双卡（A4G on） | 03 | 同轮 1（A4G 排除） |
| 4 | 双卡（A4G on） | 04 | 双卡存活 |
| 5 | 双卡+devnode 清理 | 03 | 同轮 1（devnode 排除） |
| 6 | 单卡 Gigabyte | 03 | 全绿 |
| 7 | 单卡 ASUS | 03 | 全绿 |

**失败条件收敛到唯一组合：双卡在场 + 第一枚举位的卡被解锁。** 卡身份（6/7）、devnode 历史（5）、A4G（3/4）、单槽本身（6/7）、BAR 足印（早前取证=恢复逐位一致）全部排除。嫌疑=Windows 对 FLR 过的第一枚举位卡的重仲裁交互（双卡在场的资源分配顺序）。

## 下一步读数

1. **`pnputil /enum-devices /resources`**（管理员）在冲突态跑、两卡各一份——直接点名冲突范围与竞争设备（#133 破 CSM-vga 用的同一方法）。
2. **系统内禁用→启用**冲突卡（#196 法，至今未跑）——复活即开机时序 artifact。
3. 现行 workaround 不变：要解锁的卡放第二枚举位（pin 04，round4 实证）。
