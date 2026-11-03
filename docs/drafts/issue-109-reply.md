两个 track 分开说：

**Track A（Code 12 / reconnect）**：`no-reconnect.txt` 能让重连不再黑屏、关 iGPU 能消 Code 12——这与 #35/#61/#88 定下的交棒定律一致：交棒健康取决于 console 归属链是否连贯，iGPU 与解锁卡的 legacy VGA 争夺是 Code 12 的精确成因（0xC0000018 = STATUS_CONFLICTING_ADDRESSES）。你机器上两条缓解都有效这一点本身就是有价值的数据点（Z390P/AMI 进了兼容矩阵）。iGPU 关闭如果你能接受，就是当前最稳的配置。

**Track B（GSP devinit 把 SS0/SS1 打回厂值）**：这是 GA10x 的已知机制，不是你这张 20GB 改装卡特有的问题——UEFI 阶段写入的选择器存放在 AON 域，FLR 存活（你的 log 里 `ss0_unlock=true ss1_unlock=true` 正确），但 Windows 驱动加载后 GSP devinit 会**重新推导**并覆写它们。项目内为此设计的工具就是 **ssdiag**（发布包里 `ssdiag-x86_64-windows.exe`）：

- 管理员运行 `ssdiag --select <设备ID>` 先看 `VERDICT`（你现在的预期是 NOT_UNLOCKED，SS=厂值）；
- 然后跑 `ssdiag --reapply`（GA10x 的 FEAT 门在 Windows 里保持 open，重写 SS0=0x88888888 SS1=0x8 并读回验证）；
- 之后即可跑 benchmark 验证收益。

注意两点：1) 你贴的 log 是 **v2.1.5**，当前 stable 是 v2.1.8（#83/#99 之后对 Windows 侧 relock 的判别和兼容矩阵都更新过，建议先换 v2.1.8 重跑一遍 EFI 阶段）；2) 你 log 里 UEFI 写入后的 SS0=0x88880000（低半字段无 override）与标准写入 0x88888888 不同——如果 --reapply 后 readout 仍有字段不生效，把 ssdiag 完整输出发上来，我们对照 floorsweep 检查。

关于"不依赖任何进系统后工具的持久解锁"：devinit 是 heavy-secure 域在 GSP 固件里执行的，主机/EFI 侧没有任何已知的写法能让选择器在 devinit 之后存活（这正是 PLM/熔丝体系的设计意图）。`--reapply` 是当前架构下的正解，且解锁效果是易失覆盖、每引导重放，本就是整个方案的模型。
