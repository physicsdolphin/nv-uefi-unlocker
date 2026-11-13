# Issue #165 rounds 8–12：禁用/启用复活命中 + workaround 复验（2026-11-10/11）

## Round 8：复现冲突态（pin 03，预期失败）

- 03（Gigabyte @1C1C94C）= UNLOCKED + `CM_PROB_NORMAL_CONFLICT`；04 = NONE/NOT_UNLOCKED；nvidia-smi 仅 04——round3 形态逐位复现（坏卡=被 FLR 的第一枚举位卡）。

## Round 9：**设备管理器禁用→启用复活命中**

- 同 boot 内 disable→re-enable 冲突卡 → **两卡 `CM_PROB_NONE`、nvidia-smi 双见、03 仍 UNLOCKED（PLM=0xffffffff + SS0=0x88888888）**。
- **#196 法在 T5810 复现**：冲突 = 开机时刻的资源仲裁 artifact，非持久保留——重触发仲裁即愈，且解锁态经 PnP 卸载/重加载存活（无 FLR，AON 原样）。

## Round 10：workaround 复验（pin 04）

- 双卡 NONE/双见；ssdiag 04=UNLOCKED、03=NOT_UNLOCKED——round4 形态稳定重现。

## Rounds 11/12：拆单卡复验（不换槽，先拆另一张）

- Gigabyte 单独（11）：03:00.0 全绿 UNLOCKED/NONE。
- ASUS 单独（12）：03:00.0 全绿 UNLOCKED/NONE——与 round6/7（swap-and-replace）同判，单卡态两种实现等价。

## 十二轮收官结论

1. **机理定谳**：Code 12 = Windows 开机时刻的资源仲裁 artifact，条件=双卡在场+第一枚举位卡被 FLR；任何"持久保留"假说（A4G/devnode/卡身份/足印改变）全部排除。
2. **系统内自救实锤**：禁用→启用（或设备管理器禁用设备）即愈，解锁不丢——**免冷重启的自救路径**。
3. **Workaround 双实证**：解锁第二枚举位卡（r4/r10）或事后禁用/启用（r9）。
4. 与 #196（Turing PnP disable≠FLR）呼应但对象不同：#196 是"禁用不掉解锁"，本条是"禁用/启用修复开机仲裁"——同一操作两个用途。
