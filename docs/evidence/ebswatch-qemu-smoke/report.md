# v2.1.12-rc5 诊断件 QEMU/OVMF 冒烟（ebswatch + agitate 证据底）

- 日期：2026-11-22；构建：`cargo efi-tx`（`--features tx`），banner `v2.1.12-rc5 (built Sun 2026-11-22 13:0x +08:00)`
- 环境：QEMU 11.1.0（winget，`-drive if=pflash` 装载 `edk2-x86_64-code.fd`——本构建的 `-bios` 加载器对所有路径报 could not load，pflash 可用）、q35/512MB、`-serial stdio`、64MB FAT32（WSL dd+mkfs.vfat+mcopy）、qemu-xhci/usb-storage、经 OVMF 内置 Shell `startup.nsh` 拉起
- harness：`.patches/qemu-smoke.py`（三轮 staging/跑/提取脚本化）

## 三轮判据

| 轮 | stick | 判据 | 结果 | 证据 |
|---|---|---|---|---|
| A selftest | `\tx-ebs-selftest.txt`（自带 watch） | 完整 ENTER/BEFORE/NOTIFY/RETURN 序列 + `\ebs-watch.log` 落盘 | ✅ attempt1=ENTER→BEFORE→INVALID_PARAMETER（预算内记账写入自扰，status 0x8000000000000002）；attempt2=ENTER→**EBS-NOTIFY**→**RETURN ok**→PARKING；`\ebs-watch.log` 370B 逐行累计提交 | serial-rc5-a.log、ebs-watch-round-a.log、loader-round-a.log |
| B 链载边界 | `\EFI\Microsoft\Boot\bootmgfw.efi` = loader 自拷贝（合法 PE） | CHAIN-BOUNDARY 行先落盘再关日志 | ✅ serial 可见 `LoadImage ok` + `StartImage enter`（文件级验证注意：嵌套 loader 会开新 log 覆盖——实机链载 bootmgfw 无此交互） | serial-rc5-b.log |
| C 负控 | 无 marker | 零 EBS-WATCH 输出 | ✅ grep 计数=0 | serial-rc5-c.log |

## 过程中抓出并修复的 bug（发版前）

1. **commit 链 reopen 自删**：`commit_line` 的重开路径复用了"删旧建新"的 `open_watch_file()`——每次提交后刚写的内容被自己的 reopen 销毁，dirent 恒 0 字节（诊断行 `commit pos0=0` 每次归零暴露）。修复：`reopen_watch_file()` 与 create 分离（不看 delete）。修复后 `pos0` 累计（0→99→157→267→370），ebs-watch.log 473B（最终轮 370B，行数随预算）。
2. **gmm 缓冲生命周期**：selftest 的描述符缓冲在成功取 key 后 drop → FreePool 改变内存图 → key 立即过期，三次 attempt 全 INVALID_PARAMETER。修复：缓冲 OUTLIVE EBS 调用，失败后才 drop。**这正是该仪器要抓的 bug 类——首轮即自证有效。**
3. 语义修正：edk2-stable202408（OVMF）在 TerminateMemoryMap 失败后**重发 BeforeEBS**（master 的 `mExitBootServicesCalled` 一次性语义不适用）——写预算（Enter≤1/Before≤1/ErrReturn≤2）由此成为防自馈环的硬约束而非仅文档约定。

## QEMU 未覆盖（实机轮）

- 真事务路径下的 ENTER/BEFORE/NOTIFY/RETURN 时序（QEMU 无 NVIDIA 设备、无 Windows 链）——仪器本身已自检验证
- agitate-only 实机轮（sim 契约已绿：零写台账空 + unwind clean）
- UART/POST 卡在真机器上的可读性（0xE9/0x3F8 写入无害性已保证）
