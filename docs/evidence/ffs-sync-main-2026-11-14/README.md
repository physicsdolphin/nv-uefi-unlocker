# main → dxe-driver-ffs 同步轮（2026-11-14）

把 main（v2.1.12-rc3，c11fd20）合入 `dxe-driver-ffs`（PR #100）。

## 同步内容

- 共享代码全部对齐 main：tx 事务面（tx_policy/identity 门/桥 BME/STAGING-FALLBACK/native-GOP）、
  chainload 双 ESP 引导链、keywait 幽灵键抑制、logging 提交阶梯、ultrafast 档。
- 架构保持 FFS 分支形态：`lib.rs` + 双入口（`main.rs` 应用 / `dxe_main.rs` 驱动），
  共享相位序列在 `runner`（按 main 演化版重建：PhaseOutcome 携带 hard_stop/preloaded 回传）。
- `ultrafast()` 提升到 lib 根（tx_prep/pci/tx_detach/logging/chainload 都以 `crate::ultrafast()` 引用）；
  `asset_family` 移居 runner 并在 lib 根重导出。
- 卷解析三层化：`volumes::marker_fs()`（应用=镜像卷，驱动=钉死的 NVUX2 卷）+ 驱动形态
  `.txt` 控制只从授权快照读（`read_boot_file_inner`/`boot_volume_read` 的 armed 分支）。
- `dxe_policy::CONFIG_PATHS` 补入 main 新增四个控制：`tx-no-canary` / `tx-reconnect-legacy` /
  `tx-relock` / `tx-stop-on-failure`（sim 的 every_runtime_control 测试钉住）。
- `logging`：main 提交阶梯 + FFS 三代日志轮转（`\loader-prev*.log`）共存；DXE 锚点
  （FORM/ARM:/FAILSAFE/OVERRIDES）进 FLUSH 与 quiet 两张表。
- `dxe.rs`：hard_stop 在驱动形态=记录后返回固件（驻留驱动不得挂死引导），不跑应用形态的
  STOP 死循环；驱动形态在相位尾部调 `tx::handoff_free_assets()`（它没有链载释放点）。
- `make-usbroot.ps1`：main 的 nvuu-stick.json/根检测 与 FFS 的 `dxe-manifest.txt` 生成共存。

## 验证（全部绿）

1. `cargo check` 四组合零警告：loader/dxe × tx/非-tx（x86_64-unknown-uefi）。
2. host workspace 测试 217 通过（sim/assets/ffspack/romcore，含 every_runtime_control
   与 dxe_policy 三件套）。
3. QEMU+OVMF 负控（本机 Windows QEMU，WSL mkfs.vfat 真镜像 + qemu-xhci usb-storage）：
   - 应用形态（本目录 `qemu-app-negative-control.log`）：无 N 卡优雅全链
     （DIAG/EARLY/INPUT-SETTLE/CONFIRM→ENUMERATE 7 PciIo 无 10de→GOP-PROBE→
     runner: SAMPLE 负控路径→CHAIN 自卷排除→FINAL）。
   - 驱动形态 armed/tamper/exhausted/interactive 四模式全 PASS（tools/dxe-smoke，
     RAM FV + 真 DxeCore 派发 + ReadyToBoot；exit 33 = 装置成功码）。

## 环境备注

- `ffspack build`（含 DXE_BUILD_ID 校验 + subsystem 0x0B 补丁）产出
  `NvUnlockDxe.ffs` sha256=37cb5ee1ccf5ed74c32c1b20fdd5116bd133708f41ed40349340815ca5f042dd。
- 旧 target/ 里 2026-11-01 的 v2.1.9 产物会在只 build `--bin nvunlock-dxe` 时残留
  同目录 loader.efi——本轮曾险些拿旧产物当证据，已重建后重跑；验证一律以
  banner 版本行为准。
