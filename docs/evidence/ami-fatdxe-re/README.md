# AMI FAT DXE 逆向定谳（#122/#170 老板 Filesystem 驱动，2026-11-06）

对象：B85M（MS-7823，Tiano 压缩段）与 Z97-K（2902，老式段内 LZMA）的 `Filesystem.ffs`（同 FFS GUID `93022f8c-1f09-47ef-bbb2-5814ff609df5`），参照 Z170-A 的 EDK2 FatPkg（`961578fe-b6b7-44c3-af35-6bc705cd2b1f`）。工具链：FFS/PE 解析 + idalib（IDA 9.0）Hex-Rays。PE 产物不入库，本文只留结论与地址。

## 已锤死（代码级）

1. **两板同一驱动**：模板填充函数同址（B85M/Z97-K 均在 `0x180001AFC`），Open/Close/Delete/Read/Write/Flush/SetInfo 十个槽位里七个完全同址（Flush 两板同为 `sub_180000530`），仅 GetPosition/SetPosition/GetInfo 三函数移位（5A94→5B6C / 5F80→6068 / 45A0→46AC）。
2. **AMI Flush 不含 dirent 提交**：`sub_180000530` 只走卷结构 +354 的脏数据区链表，逐项 `DiskIo->WriteDisk` 写回；dirent 提交辅助函数（`sub_180002E8C`：组 32B 目录项→`sub_180005090` 定位→直写扇区）在 Flush 的调用闭包内**不可达**。Flush 还有门禁：只读句柄或卷 +212 标志置位时返回 ACCESS_DENIED（0x800000000000000F）。
3. **Close ⊇ Flush + 家务**（`sub_180001538`）：FAT32 时若 FSInfo 空闲数漂移 >0x100 则重写 FSInfo 扇区（直写）；删除挂起走 0xE5+LFN 链清理（`sub_180003BA0`，32B dirent 直写）；释放句柄后**末尾调用同一个 Flush**（`sub_180000530`）。即 Close 的磁盘效应严格多于 Flush。
4. **EDK2 对照**：Z170-A 的 FatFlush（`0x2334`）→ `sub_221C(This,0)` → OFile walker（`0x268C`）：每个 OFile **无条件走 `sub_1F04`（= FatStoreDirEnt）+ 数据写**。即 EDK2 的 Flush 提交 dirent，AMI 的不提交——两代驱动在 Flush 语义上的真实 delta。
5. **解释全部 dirent 观测**：旧构建"40+ 行含强制 flush 行、dirent 恒 0，优雅 Close 后 13KB 一次落盘"由 2/3 条结构性成立，不再需要任何猜测。

## 未锤死（诚实清单）

- **挂死归因未定**：RE 只证明 FS 驱动是最后一层碰盘代码。挂点仍可能在下层（DiskIo/BlockIo/USB BOT——1.2s 停顿签名与速度相关性仍偏向 BOT）或驱动内未拆路径（SetPosition/Write 的 FAT 链行走有"能挂的结构"，未证被触发）。"换驱动后存活差"（BIOS 移植实验）才是因果判据，RE 的价值是让该实验的解读不歧义。
- AMI dirent 提交的调用点未定位（`sub_180002E8C` 无静态 xref，经指针/表调用）；只影响" dirent 具体在 Close 哪一步落盘"的细节，不影响上述判读。
- diag5 的逐行 Close 在此驱动上的真实代价：每标记行 = FSInfo 维护（条件）+ 全量脏区写回 + 重开——比 Flush-only 每行多两类 BOT 交互，与"diag4/5 死得更早"相容，但同为相容性证据而非因果。

## 对移植实验的解读指引

Z170-A 驱动上盘后同时改变两件事：①Flush 开始真正提交 dirent（我们的 `tx-flush-lines` 语义从"无效"变"有效"，日志逐行落盘语义改变）；②整个 FS 实现更换（挂死行为可能改变）。**因此判读必须分离**：换驱动后若仍挂 → FS 驱动无罪（挂点在 BOT/控制台）；若不挂且日志逐行提交恢复 → FS 驱动层定责。默认路径（无开关）先跑，再上逐行开关。

产物：`out/*.pe`（PE 镜像，不入库）、`out/*.flush.md / *.fill.md / *.hunt.md`（Hex-Rays 报告）、`ffsparse.py / ida_flush.py / ida_ami.py`（解析与 idalib 脚本）。

## 8. USB 栈家族逆向（2026-11-06 补，UHCD/UsbRtSmm/UsbInt13/UsbBotPei/DxeCore）

对象：两板各 5 个模块对（UHCD `580dd900`、UsbRtSmm `04eaaaa1`、UsbInt13 `4c006cd9`、UsbBotPei `8401a046`、DxeCore `5ae3f37e`）。

**跨板身份**：全家族同 AMI 基线、逐板构建——字节相等率 UHCD 9.2%、UsbRtSmm 8.3%、UsbInt13 43.2%、UsbBotPei 46.9%、DxeCore 74.9%。与 FAT 对（33.5%）同模式：同源不同 build，无逐字节同构。

**结构发现（z97k UHCD 0xc860，Hex-Rays）**：
1. **全驱动 0 次 `gBS->Stall`（+0xF0）调用**。延时全部走事件：`SetTimer`（+0x58）19 处，周期常量 `1000000`（=100ms，100ns 单位）×4、`10000000`（1s）×1。USB 传输的延时粒度 = **100ms 周期事件 tick**。
2. **6 个函数是 CheckEvent（+0x70）在循环内轮询**结构（0x18000110c/13c4/2760/5b10/60b0/87f0）——事件不触发则空转，**结构性可挂死点**；其中 0x180002760 为最大者（USB 传输核心，~5KB 反编译体）。
3. UHCD 字符串含 `UsbMassDevNum/UsbMassDevValid/UsbMassDevNum`（BOT 传输就在 UHCD 内）；B85M 版还含 `E7823IMS.A30`（MSI 板件名）。UsbInt13 = USB 启动 INT13 仿真，UsbBotPei = PEI 阶段 USB BOT（后者 x86/32 位 PE）。
4. UsbRtSmm（SMM USB 运行时，两板 ~104KB PE）：SMI 处理层；直接 `out 0xB2` 未命中（经 PCH 库间接），其存在意味着 DXE 阶段的 USB 定时器可被 SMM 事件饿死——与 2 的轮询空转结构组合即"多秒停顿/永久挂"的机制模型。

**与既有观测的对齐**：#38 在 TU106 实测的 1.23–1.27s flush 定格 ≈ 12×100ms tick（+开销）；#122 日志行间 1.5/2.4/10.2s 停顿同量级。老板 FAT 写路径全链 = 我方 log() → FAT（AMI Filesystem）→ DiskIo（DxeCore，74.9% 同构）→ BlockIo/BOT（UHCD）→ xHCI/eHCI——**每一层都是 AMI 事件量化代码**。timer 分发被饿（TPL 高挂/SMM 风暴）即可同时解释"慢轮多停顿、停顿多易死、死点在写路径"。

**H-usb 升格依据**：此前只 BOT 嫌疑（停顿签名），现在有结构现场（0 Stall + 100ms tick + 6 处事件轮询环）。换驱动实验的判读指引不变：Z170-A EDK2 栈（Stall 忙等 + 独立 BOT 驱动）替换后若存活差显著，则 AMI USB 栈定责；若仍挂，则回控制台/我方管线。
