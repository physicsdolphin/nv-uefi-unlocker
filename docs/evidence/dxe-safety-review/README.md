# DXE 安全修复验证

基线：PR #100，`c1bd16106a2a1be09eebfbcf88f302ece0e7327f`。
本轮仅构建、主机测试、无 GPU 的 QEMU；没有刷写固件或运行真实 GPU 事务。

修复映射：

| 审查项 | 修复 |
| --- | --- |
| FFS 校验字段颠倒 | PI Header @16 / File @17；独立规范偏移测试 |
| BDS 检测错误 | 删除 BDS_ARCH 分支；按 LoadedImage 文件来源决定入口/事件路径 |
| 回调键盘 TPL 错误 | ReadyToBoot 仅允许固定授权的无人值守策略；不降低 TPL、不读键盘 |
| 计数过早清零 | 两次尝试预算，只有 app 明确重新授权才重置 |
| watchdog 时序假设 | 主动启动、检查返回值；失败不运行 |
| 可变 manifest/覆盖标记 | NVUX2 固定 manifest 和完整控制文件快照，缺席同样固定 |
| 日志早于计数且可 panic | 保护先于文件/控制台；无 stdout 时安全跳过 console mirror |
| latch 失败放行 | 读写均 fail-closed，写后读回、镜像内一次性守卫 |
| scrubber 校验遗漏 | 预加载、校验、传入内存字节；不在事务中重新读盘 |

测试命令：`cargo xt`、`cargo efi`、`cargo efi-tx`、`cargo efi-dxe`、`cargo efi-dxe-tx`。
纯授权/manifest 测试直接编译 `loader/src/dxe_policy.rs`，不使用文本镜像。
真实 GPU 事务、主板刷写、平台 watchdog 到期行为仍需独立授权和真机验收。


## 本地验证结果

- `cargo xt`：134 项通过（原有 128 项 + 独立 FFS 规范校验及 5 项直接使用授权实现的测试）。
- `cargo efi` / `efi-tx` / `efi-dxe` / `efi-dxe-tx`：全部通过。
- 两类 FFS 产物均通过独立 Python 按 PI 固定偏移计算的头/数据校验；余数均为 0。
- QEMU + 系统 OVMF 4 MiB，无 PCI/GPU 直通：7 个场景通过，日志见本目录。
  每个场景均由 `ProcessFirmwareVolume` 接纳 RAM FV，再由真正的 DXE dispatcher 调度 FFS；
  已存在 BDS_ARCH 时 entry 仍不执行，直到测试程序发出 ReadyToBoot 才尝试。

| 场景 | 预期及实际结果 |
| --- | --- |
| armed | ReadyToBoot 跑只读负控，计数保持 1，不因正常返回清零 |
| tamper | NVRAM 中的策略摘要不匹配，拒绝进入 phases |
| exhausted | 计数为 2，拒绝进入 phases，计数保持 2 |
| interactive | FV 回调未授权无人值守，拒绝进入 phases，不调用键盘 API |
| headless | 系统表 stdin/stdout 置空，已授权无人值守仍安全跑只读负控、日志落盘 |
| oversized-arm | 65 字节授权变量，栈缓冲返回超限，计数仍为 0，不执行 phases |
| malformed-counter | 计数 `1junk`，严格解析拒绝执行，不清零、不作为数字 1 放行 |

七种场景还验证了重复 ReadyToBoot 和第二个独立镜像加载均不重复执行。
QEMU 成功退出码 33 来自专用 `isa-debug-exit`，不是 GPU 执行判据。

复现（仅 QEMU，勿将测试程序放到真实机器）：

```sh
cargo efi-dxe
cargo run -p ffspack -- build target/x86_64-unknown-uefi/release/nvunlock-dxe.efi /tmp/NvUnlockDxe-ro
python3 tools/dxe-smoke/run.py /tmp/NvUnlockDxe-ro.ffs --out /tmp/dxe-smoke-results
```

`run.py` 默认使用 `/usr/share/edk2/x64/OVMF_CODE.4m.fd` / `OVMF_VARS.4m.fd`，可通过
`--code` / `--vars` 改为本机路径。不修改原始固件文件；测试使用临时 VARS 和 vvfat 目录。
QEMU 的 vvfat 可能需要在 `/var/tmp` 创建临时文件。

覆盖限制：本测试在已经进入 BDS 的 OVMF 中注入 RAM FV，验证真实 FV dispatch 和事件回调，
不等同于修改主板 SPI 镜像后的初始 DXE 启动。没有验证 watchdog 实际到期复位、变量服务
故障注入、真实 GPU 事务或 SB/TPM 行为。


## 独立 subagent 复核

完成九项安全修复的只读复核。发现并修复一项 P2：删除已授权 direct-SS 文件可能回退到
family 载荷或 replay。现在两条候选的内容及缺席均纳入校验，新增两种 fallback 条件的
回归测试。窄范围终审后，无未解决的 P1/P2 findings。

subagent 独立执行主机测试、最新 5 项策略测试、UEFI TX driver `cargo check` 和
`git diff --check`；QEMU 证据由主代理执行并归档，subagent 未独立重跑。
NVRAM 保护使用 64 字节栈缓冲，超长变量不会触发按变量长度动态分配。
