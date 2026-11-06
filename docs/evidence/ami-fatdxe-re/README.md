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
