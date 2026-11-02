# 贡献指南 / Contributing

> English version: [CONTRIBUTING.en.md](CONTRIBUTING.en.md)

感谢关注本项目。这是一个 GPU 固件安全研究工具（GPL-2.0-only），在开始之前建议先读 [README](README.md) 与 [FAQ](docs/faq.md)。

## 我该从哪里开始？

| 你想做的事 | 入口 |
|---|---|
| 分享一张卡的实测（兼容性 / 读回 / 性能） | [实测反馈模板](.github/ISSUE_TEMPLATE/02-hardware-report.yml) |
| 报告制盘、EFI 执行、系统交接或诊断工具的故障 | [故障报告模板](.github/ISSUE_TEMPLATE/01-bug-report.yml) |
| 功能或文档建议 | [建议模板](.github/ISSUE_TEMPLATE/03-request.yml) |
| 使用咨询与开放讨论 | [Discussions](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions) |
| 改代码或改文档 | 本文余下部分 |

**实测数据是最有价值的贡献**：兼容性矩阵的逐卡实测完全依赖社区报告。可以用 `tools/collect_compat.py` 把 `loader.log` / ssdiag 输出归一为 JSON 附在报告里，维护者据此更新矩阵与标签。

## 提交代码前

### 构建

构建指令与 [README 自行构建](README.md#自行构建可选)一致：

```
rustup target add x86_64-unknown-uefi
cargo efi          # loader release UEFI 构建 -> target/x86_64-unknown-uefi/release/loader.efi
cargo efi-debug    # loader debug 构建
cargo xt           # host 侧全部测试（assets + sim；loader 不在 host 测试范围）
python check_efi.py <loader.efi>   # banner / 构建类别验证
```

别名来自 `.cargo/config.toml`。仓库结构：`loader/`（UEFI 应用）、`assets/`（资产管线）、`sim/`（行为级模拟器）、`tools/`（宿主侧诊断）。

### 硬性规则（违反任何一条的 PR 会被拒绝）

1. **不引入 NVIDIA 固件资产、密钥或解密产物。** `.gitignore` 已覆盖常见形态（`*.rom` / `*.bin` / `payload_*` 等）；固件从钉 SHA-256 的官方输入本地再生，hash 不符即停止，不得换用其他版本。
2. **默认构建保持零写路径。** 所有 GPU 写路径都在 `tx` feature 门后（`--features tx` 仅在明确的事务实验时构建）；如果你的改动让默认构建出现任何 PciIo 写调用，请在 PR 里说明理由。
3. **行为主张附读回证据。** 状态码、返回值、自检输出都不构成执行证明，读回比对才是（背景见 [FAQ 安全边界](docs/faq.md#safety)）。没有跑真机的改动请如实说明，不要推断真机行为。
4. **许可 GPL-2.0-only。** 参考实现（gpu-unlock 等）均为 GPL-2.0；行为借鉴请声明来源，跨许可代码不接受。
5. **不臆造证据。** 日志、硬件测试、复现步骤、成功验证都必须真实；缺失的信息填"未知"。issue 模板头部对 AI 起草的说明同样适用于 PR。

### 惯例

- 运行时输出串一律英文（既有惯例）；注释与文档中文。
- 实验产物按 evidence-as-commits 归档；`.gitattributes` 已把证据文件标 `-text`，保证跨平台逐字节一致。
- AI 辅助的贡献可以接受，但提交者对内容负全责，披露口径遵循 [README 作者节](README.md#关于作者与团队)的原则式声明；请特别自查有没有臆造日志或验证。
- 保持既有代码风格与模块边界；`sim/` 中与 `loader/` 镜像的代码块带有"必须逐字一致"注释，改动任何一侧时必须同步另一侧，并跑 `cargo xt` 验证。

## 真机实验不强求

代码与模拟层的工作（`cargo xt` 全绿 + QEMU 负控）已经足够构成合格的贡献——没有硬件也完全欢迎。反过来，如果你拥有自己的硬件并愿意实测，那更受欢迎：这正是[兼容性矩阵](docs/compatibility.md)的数据来源，按[实测反馈模板](.github/ISSUE_TEMPLATE/02-hardware-report.yml)分享即可。

两点提醒：

- 实测请遵循 [FAQ 恢复手册](docs/faq.md#recovery)与[安全边界](docs/faq.md#safety)：读回验证、异常后同会话不重试、断电恢复。
- 若你的 PR 改写了点火事务代码本身，请用模拟层证据（`cargo xt`）而非新跑的真机结果来支撑——写操作里程碑有门控流程（[AGENTS.md](AGENTS.md)），由维护者与硬件所有者按节奏推进。

## 移植类贡献的边界

不要提交"改 ID 常量"式的跨芯片移植——逐芯片几何（booter 尺寸、gadget 地址、签名段）每族不同，改常量不构成已验证移植。跨族移植=重新抽取+重新审计，欢迎先开 issue 讨论。

## 行为准则

参与 issue、PR 与讨论请遵守[行为准则](CODE_OF_CONDUCT.md)。
