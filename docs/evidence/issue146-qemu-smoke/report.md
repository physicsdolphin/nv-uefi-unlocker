# Issue #146 diag build — QEMU smoke（负控）

- 日期：2026-11-04；构建：`cargo efi-tx`，banner `v2.1.9 (built Wed 2026-11-04 01:42:47 +08:00)`（touch build.rs 强制刷新 BUILD_TS——`rerun-if-changed=build.rs` 导致时间戳缓存，见 [[github-publish-prep]] 的溯源教训）
- 环境：QEMU 11.1.0 (winget) + 自带 `edk2-x86_64-code.fd`（edk2-stable202408），q35/512MB，`-serial stdio`，64MB 真 FAT32 镜像（WSL mkfs.vfat + mtools）挂 qemu-xhci/usb-storage
- **harness 偏差**：本次 BdsDxe 未自动生成 removable-media 启动项（新 vars 直接进 Shell）——镜像根放 `startup.nsh`（`FS0:\EFI\BOOT\BOOTX64.EFI`）经 Shell 拉起。loader 本体验证不受影响；11-01 轮的直启依赖复用未重置的 vars（启动项持久化），与 memory 里"每次重置 vars"的记载冲突，待下次统一。另：Git Bash printf 会把 `\E` 解析为 ESC（0x1B），写 shell 脚本必须用 python 精确字节。
- 产物：diag 构建 = main + 三个 #146 旋钮（`\tx-dry-run.txt` / `\tx-no-asset-cache.txt` / `\tx-free-assets.txt`）+ `CHAIN-MEMMAP` 链载前内存图

## 判据核对

| 项 | 结果 | 证据行（serial-146f.log） |
|---|---|---|
| TX banner + 新时间戳 | ✅ | L17-18 |
| 30s 超时 ABORT 路径 | ✅ | L33 |
| 枚举/GOP/SAMPLE 负控（无 NVIDIA 优雅继续） | ✅ | L34-46 |
| **CHAIN-MEMMAP 新输出** | ✅ 每描述符行 + 摘要 `458 MiB conventional / 38 MiB resident (LOADER+BS) below 4GiB (64 descriptors shown)` | L48-112 |
| 链载 NOT_FOUND 负控（QEMU 无 bootmgfw）+ FINAL 干净收尾 | ✅ | L129-134 |
| armed 路径 + `\tx-dry-run.txt` 生效 | ✅ `TX-DRY-RUN: ... SKIPPED this boot (zero writes, no detach; ...)` | serial-146e.log L62 |
| 全程零 panic | ✅ | grep 无 |

## QEMU 未覆盖（实机轮验证）

- `\tx-no-asset-cache.txt` / `\tx-free-assets.txt` 的运行时行为（QEMU 无资产读取路径）——逻辑为 probe+clear，风险低
- dry-run 对真事务的抑制（QEMU 本就无 target）

## 原始日志

- [serial-146f.log](serial-146f.log) — 基线负控（本次主证据）
- [serial-146e.log](serial-146e.log) — armed + dry-run 旋钮路径
