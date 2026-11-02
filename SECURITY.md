# 安全政策 / Security Policy

> English version: [SECURITY.en.md](SECURITY.en.md)

## 支持的版本 / Supported Versions

本项目只为最新发布版与 `main` 分支提供安全修复，不为历史版本回移植。

We only provide security fixes for the latest release and `main`; no backports to older versions.

## 如何报告漏洞 / How to Report a Vulnerability

**请勿用公开 issue 报告安全问题。**

请使用 GitHub 私密漏洞报告：仓库 **Security** 标签页 → **Report a vulnerability**，直达链接：<https://github.com/physicsdolphin/nv-uefi-unlocker/security/advisories/new>。

报告中请尽量包含：

- 受影响的组件（见下方范围表）与工具版本 / commit；
- 复现条件，含硬件平台（GPU 型号、主板、引导方式）；
- 你观察到的实际行为与预期行为的差异；
- 可核验的证据（读回记录、日志摘录）。

请**不要**在报告中附上 NVIDIA 固件文件本身（本项目按纪律不托管任何固件资产）。

### 响应预期 / Response Expectations

本项目由维护者利用业余时间维护，**无法承诺固定的响应时限**：

- 会尽快确认收到报告，通常在一到两周内给出初步评估（影响面、严重度、修复计划）；
- 修复节奏取决于严重度与实际带宽，高危问题优先；
- 修复发布前与报告者协调披露节奏，默认采用协调披露；
- 修复版本发布后致谢报告者（可按你的要求匿名）。

若两周内未收到任何回复，请在原私密报告中跟进提醒——这通常只是通知被漏掉了。

## 范围 / Scope

以下组件中，**可导致非预期 GPU/内存写路径、供应链替换、本地提权或绕过安全门控**的问题属于安全漏洞：

| 组件 | 范围 |
|---|---|
| `loader/` EFI 应用 | BAR0 / PCI 配置空间写路径、FLR 处理、DMA 缓冲生命周期、链式加载、`tx` 门控有效性（默认构建是否真的零写） |
| `assets/` 资产管线 | 钉版输入的 SHA-256 校验是否可被绕过、解压/抽取的越界与路径逃逸、载荷生成完整性 |
| `tools/ssdiag` | 内嵌 `pmxdrv.sys` 的加载面（服务创建/驱动卸载）、BAR0 访问边界、用后清理是否可被绕过 |
| `release/` 打包脚本 | `make-usbroot.*` / `make-provenance.ps1` 的命令注入、路径逃逸、校验短路 |

### 不在范围内 / Out of Scope

- **NVIDIA VBIOS / GSP 固件自身的漏洞**：请向 NVIDIA PSIRT 报告。本项目研究解锁机制，不复刻或验证厂商固件缺陷。
- **使用副作用**（黑屏、STOP、驱动异常、硬件风险）：见 [FAQ 恢复手册](docs/faq.md#recovery)与[安全约束和实现边界](docs/faq.md#safety)。这类问题请走[故障报告模板](.github/ISSUE_TEMPLATE/01-bug-report.yml)，不走本渠道。
- **兼容性问题与性能回退**：见[兼容性矩阵](docs/compatibility.md)，走故障模板。
- 需要**物理接触**、需要**已开启 Secure Boot 且未导入自签证书**的引导链、或依赖**非原厂刷写固件**的攻击场景——本工具的威胁模型假设操作者对本机有完全控制权。

## 安全设计声明 / Security Design Notes

本项目的写路径纪律（安全宪法，[AGENTS.md §1](AGENTS.md)）与本政策的判定直接相关：

1. **读回是唯一成功判据**——任何写操作必须读回验证；状态码、返回值、提示输出都不构成执行证明。
2. **写操作分级门控**——默认构建（不开 `tx` feature）零写；含固件点火的事务需要显式构建与硬件所有者确认。
3. **只读链式加载**——不覆写 ESP 任何文件，不设常驻启动项，只从 U 盘一次性进入。
4. **供应链钉版**——NVIDIA 固件从钉 SHA-256 的官方输入本地再生，不分发、不入 git；hash 不符即硬失败，不得换用其他版本。

如果你发现某条纪律可以被绕过（例如资产 hash 校验失效、`tx` 门控外出现写路径、ssdiag 驱动清理可被劫持），**这正是本政策定义的安全问题**，优先走私密报告。

## 相关文档 / Related Documents

- [LICENSE](LICENSE)（GPL-2.0-only）与 README 中的版权边界声明
- [docs/faq.md](docs/faq.md)（恢复手册与安全边界）
- [AGENTS.md](AGENTS.md)（开发门控与实验纪律）/ [AGENTS.deploy.md](AGENTS.deploy.md)（部署流程）
