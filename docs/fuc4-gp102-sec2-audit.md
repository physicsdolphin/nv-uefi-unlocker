# gp102 SEC2 bootloader 审计（fuc4）——首轮（2026-11-24）

> OQ#20 关闭路径③的执行记录。对象=linux-firmware `nvidia/gp102/sec2/image.bin`
> （99KB，熵判别=明文 fuc4，见 OQ#20）。工具=falcgen `--fuc4`（oracle 对拍
> 33249/33250 行一致，见 35914fa）+ envydis fuc4（WSL，交叉验证）。**本轮为
> 结构测绘，尚未完成函数图追核**——结论标注置信级别。

## 1. 对象与角色

- `image.bin`（99KB）/`image-1.bin`（109KB）：GP10x SEC2 的 boot 固件，
  由宿主驱动经 desc/image/sig 三元组加载进 SEC2 falcon。角色=**加载器/ACR
  形态**：校验并把其他 falcon 的固件（FECS/GPCCS 等）装载/启动——Pascal 的
  "驱动 init 装固件"路径与 Turing 的 SEC2-booter 角色同位不同代。
- 布局：头部 ~6KB=代码+数据（熵 3.9–6.3），其余 ~90KB 纯零（容量填充）。
  入口代码从 offset 0 开始，经典 falcon 序言（iowr 上下文保存 → `add $sp
  -0x38` → `mov $r0 $sp` 建栈帧）。

## 2. 解码健康度

- 33250 条指令，11% unknown——unknown 行**envydis-fuc4 完全同布**（oracle
  一致），属 envytools v4 表自身的编码缺口（该表是 PMU 时代产物，SEC2
  bootloader 的部分 opcode 形状未编码）。命名指令覆盖：st/add/mov/ld/xbit/
  shl/and/or/sub/clear/lbra/lcall/iowr/iord/ret。
- 指令直方图被零区污染（90KB 零按 3 字节解码成 26814 条假 `st`）——**有效
  代码区=头 ~6KB（2123 条）**，本审计以其为对象。

## 3. DMA 模型（关键结构事实）

v4 falcon 的 DMA=Xfer slots（IO 空间，16 槽）：

- 寄存器组：`0x110 XFER_EXT_BASE`（base<<8）、`0x114 XFER_FALCON_ADDR`、
  `0x118 XFER_CTRL`（含 size+方向+trigger）、`0x11c XFER_EXT_ADDR`、
  `0x120 XFER_STATUS`（falcon io.rst:103-107）。
- **单次数据传输硬件封顶 `(4<<size)` 字节，size 3 位 → 最大 256B**；代码
  传输固定 0x100/页（xfer.rst）。**Turing 式"单一无界长度寄存器"在 v4
  xfer 接口中不存在**——这是与 Turing booter 缺陷面的根本结构差异。
- 提交方式两形：专用指令（xfer 类）或 IO poke；队列异步处理。

## 4. 代码区观察（置信=结构级，未追核到函数图）

- IO 访问全走**间接基址**（`I[$rN+off]`，$rN=运行时算出的 IO 基址）——
  直接直方图少计；xfer 寄存器序列（0x110-0x120 簇）藏在间接访问里。
- 调用图：call 目标集中在 ~0xc79-0x4bcd 区（7 个热点函数：0xe3a ×12、
  0xe4d ×11、0x3f54 ×11、0x3f1d ×9、0x3e30 ×8……）——小型加载器的函数
  群形态。
- 指令混合（代码区）：st/ld/mov/add 占绝对多数=逐字拷贝-校验型加载器
  形态，与 ACR 角色一致。

## 5. 未决（③的剩余）与判读

- **聚合越界问题**：单 xfer 256B 封顶 ≠ 无洞——若拷贝循环的次数来自
  宿主可控的描述符字段，多轮 xfer 仍可越界写满 IMEM/DMEM 到栈区。
  判定手段=6KB 函数图追核（xfer 参数的来源寄存器回溯到哪个输入结构）。
  工具需求=interp 的 fuc4 分支或手工追核（一个会话级工作，非立项级）。
- **签名校验形状**：v4 无 CRYPT 特性——校验若存在，走 IO CRC（0x124/0x128
  CRC_DATA/CRC_STATE，io.rst 有此寄存器）或软哈希。CRC 寄存器存在=fast
  校验原语在位；是否对**宿主提供的镜像**做强签名校验（还是 CRC 完整性
  检查）待追核。
- **判读**：即使③最终发现聚合越界，Pascal 无 FEAT 轴=砸开后无可收割门
  （矿卡 Gen2 走配置空间正交车道）——**审计是安全研究输出，不构成解锁
  车道**（OQ#20 ④门槛）。

## 6. 方法与复现

```bash
falcgen dis image.bin --fuc4 --base 0 > full-fuc4.lst   # 33250 行
# envydis oracle: WSL envydis -m falcon -V fuc4 -i -n image.bin（33249/33250 一致）
```
公开固件（linux-firmware），URL 见 OQ#20；二进制不入 git。
