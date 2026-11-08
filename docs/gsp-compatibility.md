# GSP 兼容性：砸门的工牌，不挑版本（2026-11-08 实测定谳）

> 结论先行：**解锁对 GSP 固件的版本、尺寸、出处全部不敏感**——GSP 在本管线里只是
> "砸开 FEAT 门用的工牌"（payload 模式下 booter 对镜像自身的验签在 ROP 触发前根本
> 不会执行）。TU106/Z170-A 台架实测 **525→621 全谱系五版本全解锁**；唯二边界是
> **容器格式代际**（pre-52x）与 **fwimage↔fwsignature 成对约束**。

## 一、机制：为什么 GSP 不挑版本

解锁事务对 GSP 镜像的全部依赖面是三个纯函数（sim `gsp_version_swap.rs`，125 测试全绿）：

1. `layout_from_registers`——只吃 **image_size**；
2. `radix::build`——内容无关的页表映射；
3. WprMeta `image_bytes` 字段——还是只存尺寸。

payload 模式下 WprMeta 的 signature DMA 指向 V67 载荷，booter 的签名拷贝越界即
ROP 触发——**对 GSP 镜像自身的任何校验都在那之后**，而那时门已经开了。镜像在解锁
窗口内的全部角色 = radix 载荷 + WPR2 占位。

真正版本咬合的三件套全不来自 nvidia-utils：**booter**（kernel-source BINDATA）、
**GspRmBoot BL**（同源）、**FWSEC**（VBIOS 内置）——它们与 GSP 版本解耦
（`driver-version-unlock-coupling` 结论的镜像侧推论）。

## 二、实测矩阵（TU106 / Z170-A，2026-11-08）

**作用域铁律：全部实测都在图灵谱系（`gsp_tu10x`）上**。GSP 按 GPU 世代分发
（图灵=`gsp_tu10x.bin`；Ampere/Ada=`gsp_ga10x.bin`，Ada 用其中
`.fwsignature_ad10x` 签名段）——**跨代使用不在本结论范围内，也不应尝试**：
40 系（AD10x）的兼容下限 = **40 系起家的最早那版 GSP**（Ada 发布驱动世代，
Windows 522.27 / Linux 525.60.11，需验证其 `fwsignature_ad10x` 在位），
且 Ada 谱系的版本自由需要自己的台架轮次（机制同源，未实测不外推）。

| GSP 版本 | fwimage_tu10x | 大小 | loader | 结果 |
|---|---|---|---|---|
| 525.60.11 | 0x2359FE0 | 37,068,768 B | main(修复后) | ✅ 全链 unlock |
| 560.35.03 | — | 26,250,912 B | v2.1.10 | ✅ 全链 unlock |
| 610.43.03（钉定 stock） | — | 29,339,080 B | v2.1.10 | ✅ 全链 unlock |
| 615.78.08 | 0x15D3DC8 | 22,887,880 B | v2.1.10 | ✅ 全链 unlock |
| 621.01（Windows 驱动仓库） | 0xFE6DD8 | 16,674,264 B | main(修复后) | ✅ 全链 unlock |

尺寸跨度 **16.7–37.1 MB（2.2×）**（图灵谱系内），全部 layout 吸收、radix 字节精确、
`HANDOFF-OUT tu10x:unlock=true`、selector 0x88888888/0x8 落盘、READOUT_1=0x0 全速。
证据：`docs/evidence/z170a-tu106-logcommit/`（525/621 两份完整 unlock log）。

两种 loader（v2.1.10 与 main 修复后）各自跨版本全绿——loader 与 GSP 版本亦无耦合。

## 三、边界一：容器格式代际（510–520 不可打包）

- **510.39.01 / 515.x / 520.56.06** 的 `gsp.bin` 是**单一多代 RISC-V ELF**：
  `.tu10x_resid` / `.ga10x_resid` / `.ga100_resid` 驻留段 + `.acl` + `.manifest`，
  **没有 `.fwimage`、没有 `.fwsignature_*` 段**——分代签名容器从 **525.60.11** 起
  才存在。这是**结构断崖**，不是版本远近问题：现管线（以及 booter 的 WprMeta/radix
  消费面）以 `.fwimage` span 为输入，510-520 容器无法产出该输入。
- makeself 壳同步换代：510–535 载荷为 xz（魔数 `fd377a58`），55x+ 为 zstd
  （`28b52ffd`）——抽固件脚本需双格式支持。

## 四、边界二：fwimage↔fwsignature 必须成对

签名段与镜像出自同一容器（签名覆盖该镜像的构建）。换版本时**两件必须同换**；
stock/诊断模式（无 V67）与 payload 模式 FLR 前的签名复原都依赖这个配对。
滚动容器天然成对，手动拼装不允许。

## 五、默认行为（v2.1.11 起）与对用户的含义

- **默认 = 本地驱动 GSP 优先（badge 模式）**：`make-usbroot` 的 auto 链首选本机已装驱动的 DriverStore/firmware 目录（Windows `DriverStore\FileRepository
v*inf_amd64_*\gsp_*.bin`，按 `DriverVer` 取最新；Linux `/usr/lib|/lib|/run/current-system` firmware `sort -V`），找不到才回退下载链（Arch 归档 → 第一方 .run → CachyOS v3 → v4）。badge 模式**豁免版本钉定**——豁免的依据就是本文件的实测矩阵（525–621 全绿），且本地驱动即交棒对象自带的固件，语义上是最正确的选择。`-UtilsSource/--utils-source local` 可单独钉定该路；`arch|run|cachyos|cachyos4` 钉定某一下载通道（下载通道内部哈希闸不变）。
- **可复现性边界如实陈述**：钉定 610.43.03（宪法 6）仍约束全部**下载**通道；badge 模式产出的 stick 携带的是本机驱动自带的 GSP（loader.log 的 `fwimage_*.bin loaded N bytes` 行即其尺寸指纹）。需要逐位可重建 stick 的场合（evidence-as-commits）应显式走下载通道并保留 `inputs/` 里的钉定文件。
- 下载源若再死：**任何 525–621 的 `gsp_tu10x.bin` 都能当离线替代**——抽 `.fwimage`+`.fwsignature_tu10x` 成对放 `inputs/`/根目录即可（运行时无 hash 钉，只查尺寸窗口）。
- 不提供"GSP 版本选择"下载参数：全兼容意味着选项没有功能收益，反而制造一个需要逐组合验证的虚假支持矩阵；非钉版 GSP 走 badge 模式或离线投放（均为官方路径）。

## 六、证据与复现

- sim：`sim/src/gsp_version_swap.rs`（125 测试）；真机 log：
  `docs/evidence/z170a-tu106-logcommit/`（62101/525 两份完整 unlock log）。
- 抽取配方：`gsp-swap` kit README（U 盘根目录），.run 双格式壳解析
  （skip=行号 + xz/zstd 魔数判别）。
