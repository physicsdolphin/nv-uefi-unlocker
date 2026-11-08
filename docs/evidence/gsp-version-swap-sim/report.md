# GSP 版本置换 sim 实验（2026-11-08，"工牌"理论第一轮）

问题：GSP 镜像版本对解锁事务是否无关紧要（工牌理论：GSP 只是骗 Falcon 开门的工牌，任何带 NVIDIA 签名的版本都行）？

## 度量事实（issue #212 轮的副产品）

| 项 | 610.43.03（钉定） | 610.57.04 |
|---|---|---|
| gsp_ga10x.bin 整 ELF | 84,277,400 B（sha256 57237362…） | 84,310,168 B（c0156954…） |
| `.fwimage` 段（事务实际映射的对象） | 84,226,048 B（0x5053000） | **84,258,816 B（0x505B000，页对齐，+0x8000）** |
| gsp_tu10x.bin | 29,352,832 B（73065619…） | 29,381,504 B（d157e3b7…） |

NVIDIA 每版驱动独立构建 GSP——版本间**内容与尺寸都不同**，"最近版本容器"不可能通过内容寻址闸。

## sim 结论（sim/src/gsp_version_swap.rs，125 测试全绿）

loader 对 GSP 镜像的全部依赖面=三个纯函数，全部换 610.57 尺寸重跑：

1. **layout**：8 个 C-main 寄存器场景全绿；几何真实重排（wpr_end/frts/boot 自顶向下栈与 heap 侧 wpr_start **不随 image_size 动**；`gsp = align_down(boot − image, 64KiB)` 吸收增量——sc0 实测步进 0x10000，映射窗口仍完整容纳更大镜像，这是换版本成立的硬条件）。
2. **radix**：plan/build/独立 walker 逐页字节精确（含全部 L2 边界 511/512/20479/20480）。
3. **WprMeta**：`image_bytes` 字段原样携带新尺寸。
4. **Transaction**：三形态（Ada 三证 / GA102 wait_plm / stock replay）全绿——状态机**根本不吃镜像字节**。
5. **真字节保真**（env 门控 ignored 测试，实跑）：610.57.04 真实 `.fwimage` 过 radix——**20,571 页逐字节映射精确**。

**结论：我们的事务逻辑对 GSP 版本零内容依赖，唯一耦合=尺寸参数且已验证。** 工牌理论在 sim 层成立，且比原始表述更强：payload 模式下 WprMeta 的 signature DMA 指向 V67，booter 对 GSP 镜像自身的验签在 ROP 触发前不会执行——镜像在解锁窗口内是纯 radix 载荷。

## 边界（sim 证不了、留给真机的）

- booter/FWSEC 在硅上是否接受置换镜像（0x91 族=meta/geometry 拒收先例；booter/BL/FWSEC 三件套=真版本咬合面，全不来自 nvidia-utils）。
- stock/诊断模式需 image↔fwsignature 自洽成对；payload 模式 FLR 前签名复原用的 stock_sig 必须与镜像成对（滚动容器天然成对）。
- 真机 A/B（payload 模式 + 610.57 fwimage，判据照旧=读回）排下一个门控轮。

复现：`cargo test -p sim gsp_version_swap`；真字节：`SIM_GA10X_61057_FWIMAGE=<610.57 fwimage_ga10x.bin> cargo test -p sim real_610_57 -- --ignored --nocapture`。
