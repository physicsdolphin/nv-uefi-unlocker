# CSM 兼容：reconnect 卡死类与两条解法路径（issue #107 谱系）

> 2026-11-02，branch `issue107-csm-compat`。起因：#107（AD103 4080S / ASUS B550-F）实证 CSM on → `RECONNECT: ConnectController (recursive=true) begin` 卡死、CSM off → 全链绿（+82~88%）；解锁写入与 FLR 存活两种模式完全一致——病灶纯在 reconnect。本文是 CSM 类的签名表、已实现的 native-GOP 显式恢复路径、实验矩阵与证据索引。（曾评估的 Driver####/bcfg 预注册形态已裁撤：动态加载已覆盖需求，无需常驻 NVRAM 项。）

## 1. 病理签名

| 观察点 | CSM on | CSM off |
|---|---|---|
| 选择器写入/FLR 存活 | 正常 | 正常 |
| `RECONNECT: ConnectController begin` 之后 | 无后续行（卡死在固件 ConnectController 内部，嫌疑 = 重新选中已停掉的 BiosVideo binding） | `ok (recursive=true)` |
| 日志盲区（#107 报告者指出） | v2.1.8 的 RECONNECT 行先 flush 再进 console 镜像，分不清 ConnectController 内部挂死 vs console 输出挂死，也不知道选了哪个显示驱动 | — |

**v2.1.9 起的盲区修复**：`DIAG *-OPENERS` 行现在携带驱动镜像 base+size 指纹（`tx_detach.rs::log_openers`），detach 前/重连后都能看出是 BiosVideo 还是 vendor GOP 在管 GPU。

## 2. 路径一：native-GOP 显式恢复（`\tx-native-gop.txt`，tx 构建）

reconnect 之前**显式选择**原厂 GOP 驱动，不调用会卡死的通用 ConnectController（"ConnectController priority is not an exclusion filter"——给了偏好列表也排不掉别的候选，必须显式 Start）：

1. 取 GOP EFI 镜像：`\tx-native-gop.txt` 首行为卷上模块文件名（如 `\gop.efirom`，make-usbroot 兼容包或本卡 VBIOS dump 经 `assets extract-gop` 本地再生）；首行为空/无内容 = 用本卡 PciBus 影子 ROM（`PciIo RomImage`）。
2. `romcore` 解析（PCIR 链 + NPDE 步进 → UEFI 13.4.2 EFI ROM 头 → PE32+ 校验）；压缩 payload（CompressionType=1，NVIDIA GOP 全系如此）交给**固件自己的 EFI_DECOMPRESS_PROTOCOL**——PciBus 处理压缩 OPROM 用的就是它，权威实现，Rust 侧保持纯解析。
3. `LoadImage(FromBuffer)` + `StartImage`；定位**恰好一个**属于新镜像的 Driver Binding（`18A031AB-…`），对目标卡显式 `Supported()`→`Start()`。
4. 读回验证（宪法 §1.1）：目标卡必须出现属于它的 GOP child（`path_belongs_to` 节点对齐判定）。
5. console 恢复只在 **GOP child** 上做 `ConnectController(child, non-recursive)`——child 级连接不会触碰 PCI 级驱动，BiosVideo 无从被再次选中。
6. VGA forwarding（pre-claim 捕获的 `0x001C` 子集）+ IO|MEM 解码经 PciIo Attributes 恢复；**BME 保持 off**（交棒形状不变）。

失败策略：任何一步失败 → 大声记录并交给 bootmgfw 自建 console（实证的安全落点），**永不回退到会卡死的 ConnectController**。

## 3. 实验矩阵（按序，全部 marker 门控、默认关）

| # | 配置 | 验证什么 |
|---|---|---|
| 1 | CSM on + 无 marker（现状） | 基线（预期 #107 形状卡死） |
| 2 | CSM on + `\no-reconnect.txt` | bootmgfw 能否在 CSM 下重建 console——补 Code 12 定律的 CSM 边界（#106/#83 的 no-reconnect 经验是否跨 CSM 成立） |
| 3 | CSM on + `\tx-reconnect-nonrecursive.txt` | 非递归 vendor-GOP-only 绑定在 CSM 下的行为 |
| 4 | CSM on + `\tx-native-gop.txt` | **本计划主体**：显式原厂 GOP 恢复（#107 报告者已在自有实现上验证该思路可行） |

## 4. GOP 模块的来源（无下载依赖，按宪法 §1.6）

1. **本卡影子 ROM（默认，零下载）**：固件枚举时已把整块 Option ROM 影子拷进 UEFI RAM（`PciIo RomImage`），loader 直接解析——与卡绝对匹配，无分发问题。
2. **用户自己的 VBIOS dump**：GPU-Z 等工具 dump 本卡 ROM → `assets extract-gop <dump> <outdir>` 本地提取 `.efirom` 模块（SHA-256 打印钉死）。
3. **兼容包（make-usbroot 接线，规划中）**：GOP 驱动只存在于 VBIOS ROM（NVIDIA 驱动包不含），钉住的对象是按族的官方原版 VBIOS dump（TechPowerUp VBIOS 数据库等存档，SHA-256 进 manifest），脚本下载后**本地**提取——与 gsp.bin/booter 的发放模式同构，项目永不自托管 NVIDIA 固件。压缩 .efirom 直接可用：解压在运行时经固件 EFI_DECOMPRESS_PROTOCOL 完成，loader 侧零解压代码。

## 5. 开放项

1. **assets 解压 PE 产出**：目前不需要——兼容包模块是压缩 .efirom，解压在运行时经固件 EFI_DECOMPRESS_PROTOCOL 完成，loader 侧零解压代码。若未来需要"落盘即 PE"的形态，再考虑 host 侧 Tiano 解码（对拍 repro 已留存，对照 EDK2 BaseUefiDecompressLib 语义逐行可查）。
2. GOP 模块尾部 VID/DID + checksum 的改写语义（Wudimobile GOP Updater 的规范化逻辑）对**运行时 LoadImage** 大概率无关（Supported() 查 PCI 设备而非尾部），真机验证时顺带确认；仅 GOP 烧写替换场景需要。
3. `\tx-native-gop.txt` 的真机矩阵（表 3 的 #2/#4 两行）等 #107 报告者配合 + 本机 Z370 复验。

## 6. 证据索引

- #107 报告者实现（行为参考，无许可头 → 只借鉴行为）：`RTX4080SUPER-Tensor-Core-Unlocker` 的 extract_gop / native_gop / ownership / pci_snapshot / vga_restore。
- 社区工具参照（同上，行为级）：Wudimobile GOP Updater（Turing/Ampere 两套提取-规范化脚本；`55 aa ?? 00 f1 0e` 头模式与本文 romcore 解析一致）。
- 许可：解锁侧 GPL-2.0-only 兼容；本项目全程 GPL-2.0-only 独立实现（宪法 §1.10）。
