# CSM 兼容：reconnect 卡死类与两条解法路径（issue #107 谱系）

> 2026-11-02，branch `issue107-csm-compat`。起因：#107（AD103 4080S / ASUS B550-F）实证 CSM on → `RECONNECT: ConnectController (recursive=true) begin` 卡死、CSM off → 全链绿（+82~88%）；解锁写入与 FLR 存活两种模式完全一致——病灶纯在 reconnect。本文是 CSM 类的签名表、已实现的 native-GOP 显式恢复路径、实验矩阵与证据索引。（曾评估的 Driver####/bcfg 预注册形态已裁撤：动态加载已覆盖需求，无需常驻 NVRAM 项。）

## 0. 谁需要 CSM——这个兼容补丁的受众（先读这个）

**我们仍然强烈建议绝大多数用户关闭 CSM、使用纯 UEFI**——本仓库的全部默认路径、性能验证与显示稳定性都以 CSM off 为基准。CSM off 能一次消掉本家族的全部症状（reconnect 卡死、交棒后 Code 12、显示路由漂移），没有理由为了本工具打开它。

但 CSM on 有一批**真实存在、无法简单弃用**的受众，他们的机器上 CSM 不是可选项：

1. **依赖 legacy-only OPROM 的老设备**：部分老阵列卡/采集卡/专业卡只带 legacy x86 ROM，纯 UEFI 下固件根本不执行其 OPROM，设备无法在预引导阶段使用（RAID 卡引导卷、老卡自检画面等场景）；
2. **亮机卡没有 UEFI GOP**：矿场搭配的极老亮机卡（GT 210/Quadro NVS 早期/GT 610 之前世代）VBIOS 里**没有 EFI 驱动可以提取**——本工具的 native-GOP 兼容路径与 ReBAR/引导都无从谈起，这类卡在纯 UEFI 下甚至可能点不亮；
3. **VBIOS 容量太小插不进 GOP**：一些老卡 VBIOS 总共只有 64 KB 且已满，UEFI GOP 模块（~130 KB 压缩前）物理上放不进去——"刷个带 GOP 的 VBIOS"对它们不是选项；
4. 传统引导的系统盘 / 老企业平台（部分 OEM 锁死 CSM 行为）。

这些人**同样有解锁需求**（矿卡/老卡恰是钳制重灾区），而他们的 CSM-on 环境正是 reconnect 卡死族（#106/#107/#133/#134/#157）的暴露面——所以这个兼容补丁不是给"懒得关 CSM"的人准备的便利，是给**没有关闭选项**的机器准备的。如果你不确定自己是否属于上述场景：**先关 CSM 试，通了就别开回来**。

**2026-11-04 起 native-GOP 已是默认行为**（维护者裁定，四轮 Z170 本机验证 + #107/#128 两台真机字段记录）：本卡影子 ROM 原厂 GOP 重建 console 取代 firmware `ConnectController` 重连。设计公理 = **错误重连不如不连，console 怎么起来的就怎么继续**——原厂驱动从原厂 ROM 重建，任何失败（提取/解压/绑定/child 校验）都保持 boot 时的原样、交给 bootmgfw 自建，**永不回落到会卡死的 ConnectController**。风险不对称论证：CSM 板上旧默认的最坏情况是硬挂死必须断电，native-GOP 的最坏情况是冻帧到 Windows 接管（四轮全数进 Windows）；已知失败形态只有"混合接屏下 child 安装不确定"（CSM 模块抢 console 所致，两轮相反结局），优雅回落两头接住。退出阀 = U 盘根 `\tx-reconnect-legacy.txt`（恢复旧版 recursive + PR#13 逻辑）；`\tx-native-gop.txt` 保留为可选 GOP 模块 pin。失败形态若出现 **StartImage 内部挂死**（非 verify 失败）则撤回默认化——这是唯一敬畏的失败面。

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

## 3.1 实测记录（2026-11-03 更新）

- **CSM Disabled 单变量消除 C1 已有三例**：#107（B550/AD103，原报告）、#133（B350M/2206，+90% 收官）、#134 同板族待回。**CSM off 是 C1 的第一动作**（先于任何 marker 实验）。
- **"CSM 占用 legacy VGA" 的参照证据形态**（#133 实证）：`pnputil /enum-devices /resources` 在 CSM on 时含 IO `3B0-3BB`/`3C0-3DF` + MEM `A0000-BFFFF`，CSM off 后消失、其余不变。
- **黑屏误判警告**（#133 报告者实测踩坑）：清除全部 marker 后 loader 在 CONFIRM 停 30 s 然后 `ABORT (zero GPU writes)`——黑屏下极易误读为"CSM 没起作用"。判读点：日志末行为 ABORT = 没跑事务，不是事务失败。
- 空 marker（0 字节）自 v2.1.9 重建资产（`fc1f70ac` 起）起即为合法开关形态，无需占位字节。

## 3.2 TU106 本机复验（2026-11-04，Z170 + 2070，native-GOP 首轮 Turing 真机）

事务全绿（unlock_persist、FLR 按设计跳过）、HANDOFF-OUT 族标注首秀正确（`tu10x:unlock=true`），native-GOP 链走到 **Start=SUCCESS** 后发现一个驱动层事实：

- **TU1xx GOP 驱动安装 child 与否并非确定性**（四轮修正）：同样的"显示器接核显"拓扑下，一轮 750 ms 三探只有核显的 GOP child（probe hex 解码 = `Acpi(PNP0A03,0)/Pci(0x2,0)/AcpiAdr`，恰为 PR13 已知的 child 形态），**另一轮 probe 0 即出现独显 child（36 B = 独显 path + AcpiAdr）→ `verified GOP child (3840x2160)` → `console rebound` → `complete (display path verified)`**——Turing native-GOP 首次端到端完整成功（证据 `evidence/tu106-native-gop-z170/loader-native-gop-success.log`）。
- **第四轮（纯 UEFI 对照）把变量精确化：决定"child 装不装"的是谁持有 boot console**（`loader-pure-uefi-success.log`）。切回纯 UEFI 后 console 归属反转——GOP-PROBE 唯一句柄是**独显的**（36 B，`on_target=true`），dGPU 从 D3cold 入场变为 **D0 活跃入场**（SAMPLE 全可读、NV_PROM crc 正常；CSM-on 轮独显被 RTD3 停泊、SAMPLE 全 all-ones），`pre-claim attributes raw=0x300`（CSM-on 轮是 0x0——CSM 模块连 PciIo attributes 跟踪都扰乱），detach 正常清放（GOP 6→4→no handles）。native-GOP **probe 0 即中 4K** → console rebound → Windows 全程独显可见、无假死机段。结论：**CSM-on 下 CSM 模块把 boot console 抢给核显，是独显 child 安装不确定性的根源**；纯 UEFI 下确定性恢复。native-GOP 在纯 UEFI 下同样工作良好（其实际作用 = 无缝重建被 detach 的独显 console，不等 bootmgfw 重绑）——marker 非 CSM 专属。附带观察：0x823xxx FEAT 别名镜像在 CSM-on 轮可读、纯 UEFI 轮全 badf（0x4096xx 恒可读）——别名 decode 差异待查，不影响事务。
- **定位收敛（混合形态的两种结局都已实证）**：**显示器接独显、但 CSM 把 boot console 走到核显**时（本机实测形态），native-GOP 成功后独显口的物理屏会**定在重绑帧**——bootmgfw 画的是核显 console，没人向独显口更新——直到 Windows 里独显驱动接管点亮锁屏（无 Code 12）。视觉上有一段"假死机"，属预期路径而非故障；而真正的"显示器接核显"形态下 native-GOP 失败 → bootmgfw 用核显 console，核显口**正常显示无冻结**。两种结局都不阻断进 Windows。冻帧的本质 = 重绑帧之后到 OS 接管之前，boot console 与物理输出口分离（CSM 显示路由所致），纯 UEFI 下 console 与物理口一致，无此段。
- **休眠恢复两形态**：恢复成功时（hiberfile 被接受）驱动/资源原样，无 Code 12、独显直接可用——冷启动 Windows 重跑资源仲裁才会撞仲裁失败，Code 12 定性收窄为**纯仲裁问题**；恢复失败时 Windows 检测到 GPU 状态与 hiberfile 不符，**丢弃镜像转正常启动**（本机第四轮实测），无损失、解锁正常。休眠 + 解锁组合仍属敏感路径，推荐日常用普通关机/重启。
- 设备管理器禁用→启用不掉解锁（OS 内自救），见 compatibility.md TU106 节。
- Z170 的 NVRAM Windows 选项 = **HD 短格式 + FILE 节点混合形态**（`hd(partN,sig) + file:"\EFI\Microsoft\Boot\BOOTMGFW.EFI"`），短格式 resolve 按 BmExpandPartitionDevicePath 分区身份匹配——真机三连全中（diag.5 字段验证第二平台）。

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
