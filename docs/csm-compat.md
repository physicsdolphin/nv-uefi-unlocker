# CSM 兼容：reconnect 卡死类与两条解法路径（issue #107 谱系）

> 2026-11-02，branch `issue107-csm-compat`。起因：#107（AD103 4080S / ASUS B550-F）实证 CSM on → `RECONNECT: ConnectController (recursive=true) begin` 卡死、CSM off → 全链绿（+82~88%）；解锁写入与 FLR 存活两种模式完全一致——病灶纯在 reconnect。本文是 CSM 类的签名表、两条已实现路径（native-GOP 显式启动 / Driver#### 预注册）、实验矩阵与证据索引。

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

## 3. 路径二：Driver#### 预注册（`\tx-register-driver.txt`，默认构建）

PR #100 讨论定调的"bcfg driver load 模式"：非 FV 侵入的常驻形态。

- `\tx-register-driver.txt` 首行 = 驱动文件名（默认 `\NvUnlockDxe.efi`，须为本卷上真正的 PE 驱动，如 dxe-ffs 分支产物）；`\tx-unregister-driver.txt` = 一条命令卸载（删本卷注册的所有 Driver#### 并重写 DriverOrder）。
- 注册 = `Driver####`（EFI_LOAD_OPTION：卷设备路径 + FilePath 节点）+ `DriverOrder` 追加；写入前查重（幂等）。
- 恢复属性：**拔 U 盘 = Driver#### 指向的介质消失，加载非致命失败，正常引导**——正是"出问题拔掉重启"的安全语义。
- Secure Boot：Driver#### 走 LoadImage → SB 验签**适用**（SB-ON 需自签入 db；与 FV-embed 的免验签不同）。
- 宪法 §1.4 边界：非 BootOrder 但属持久 NVRAM——仅显式 marker 触发、介质可拔即失效、卸载一条命令，文档明示。
- 手工等价（UEFI Shell）：`bcfg driver add 0 fs0:\NvUnlockDxe.efi "NvUnlock"` / `bcfg driver rm <n>` / `bcfg driver dump`。
- GOP 预注册用途（console 域等效 CSM off）：Driver#### 在 BDS console 连接**之前**加载，原生 GOP 抢在 CSM BiosVideo 之前绑定。注意 Driver#### 文件必须是可 LoadImage 的 PE——压缩 .efirom 模块不行，需等 assets 管线产出解压 PE（见 §5 开放项）。

## 4. 实验矩阵（按序，全部 marker 门控、默认关）

| # | 配置 | 验证什么 |
|---|---|---|
| 1 | CSM on + 无 marker（现状） | 基线（预期 #107 形状卡死） |
| 2 | CSM on + `\no-reconnect.txt` | bootmgfw 能否在 CSM 下重建 console——补 Code 12 定律的 CSM 边界（#106/#83 的 no-reconnect 经验是否跨 CSM 成立） |
| 3 | CSM on + `\tx-reconnect-nonrecursive.txt` | 非递归 vendor-GOP-only 绑定在 CSM 下的行为 |
| 4 | CSM on + `\tx-native-gop.txt` | **本计划主体**：显式原厂 GOP 恢复（#107 报告者已在自有实现上验证该思路可行） |
| 5 | 任意 + `\tx-register-driver.txt` | Driver#### 交付形态（QEMU 已过，见证据索引） |

## 5. 开放项

1. **QEMU：BDS 未在第二班 StartImage 预注册驱动**——注册的变量结构已验证规范（见证据索引），嫌疑 = Driver#### 处理时序的存储栈连接（Boot0001 走 IDE 兼容路径、注册路径走 AHCI 栈）。需 DEBUG OVMF 或真机复验；社区 bcfg 注入 RAID/NVMe 驱动是该机制的成熟先例。
2. **assets 解压 PE 产出**：GOP 兼容包若要支撑 Driver#### 预注册，需要解压后的 PE（运行时解压用固件协议没有问题，但落盘文件必须是 PE）。Tiano 解码的 Rust 移植在 host 侧已有一个"除一处 rare construct 外"对拍通过的版本，repro 保留（对比 EDK2 BaseUefiDecompressLib 语义逐行可查）。
3. GOP 模块尾部 VID/DID + checksum 的改写语义（Wudimobile GOP Updater 的规范化逻辑）对**运行时 LoadImage** 大概率无关（Supported() 查 PCI 设备而非尾部），真机验证时顺带确认；仅 GOP 烧写替换场景需要。
4. `\tx-native-gop.txt` 的真机矩阵（表 4 的 #2/#4 两行）等 #107 报告者配合 + 本机 Z370 复验。

## 6. 证据索引

- `docs/evidence/csm-driver000-smoke/`：QEMU/OVMF 三跑（负控 / 注册 / 幂等）+ open item 说明。
- #107 报告者实现（行为参考，无许可头 → 只借鉴行为）：`RTX4080SUPER-Tensor-Core-Unlocker` 的 extract_gop / native_gop / ownership / pci_snapshot / vga_restore。
- 社区工具参照（同上，行为级）：Wudimobile GOP Updater（Turing/Ampere 两套提取-规范化脚本；`55 aa ?? 00 f1 0e` 头模式与本文 romcore 解析一致）。
- 许可：解锁侧 GPL-2.0-only 兼容；本项目全程 GPL-2.0-only 独立实现（宪法 §1.10）。
