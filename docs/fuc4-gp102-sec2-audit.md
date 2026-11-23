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

## 5a. 第二轮（2026-11-24，同日）：TLV 解析器与签名原语

**软件哈希原语缺席**：全镜像扫描 SHA-1/SHA-256/MD5 轮常数与 CRC32 多项式
（LE/BE）**零命中**；代码区亦无 CRC IO 访问（0x124/0x128 CRC_DATA/STATE）。
**签名校验形状（问题 2 的答案）**：bootloader **自身不做软件签名校验**——
Pascal 的固件鉴权在**每 falcon 自己的 BROM**（FECS/GPCCS/SEC2 各自的 Boot
ROM 对各自 ucode 硬件验签，nova 文档模型）；bootloader 的角色=拷贝/启动/
管理，不是验证器。⇒ 不存在"绕过 bootloader 验签"这一攻击面；硬件 BROM
验签是真正的信任根，且不在可逆向修改的软件路径上。

**问题 1 部分闭合——TLV 描述符解析器现身**：0x1597 循环是**输入长度字段
驱动的 TLV 解析**——`ld D[$r10+0x11]` 读长度字节 → `$r13 = $r10 + len +
0x13`（基址+长度+头大小=值地址）→ `ld D[$r13+$r9]` 按派生地址**读取** →
`lbra` 分发到 0x13b2/0x13cb/0x1376 三态；同函数含 `exit`（主循环）。
即：**宿主提供的描述符长度字段确实驱动本固件的地址计算（读侧已实证）**。
写侧未闭合：同区 `st` 的指针来源回溯尚未完成（0x1792 循环 `ld D[$sp+$r0]`
→ `st D[$r1+0xb]` 形似栈→指针拷贝，但该区 unknown 密度高，存在错位级联
可能，置信降级）。

**判读更新**：读侧 TLV 驱动+写侧未闭合=聚合越界问题**保持开放但收窄**
（若写指针亦由长度字段派生 → Turing 式溢出的 Pascal 类比存在，但受 256B
xfer 封顶与目标 falcon BROM 验签双重限制——即使 SEC2 DMEM 被写穿，
FECS/GPCCS 的鉴权在各自 BROM，不可绕过）。**无门结论不变**（OQ#20 ④）。

## 5b. 剩余（③的尾巴）

## 6. 方法与复现

```bash
falcgen dis image.bin --fuc4 --base 0 > full-fuc4.lst   # 33250 行
# envydis oracle: WSL envydis -m falcon -V fuc4 -i -n image.bin（33249/33250 一致）
```
公开固件（linux-firmware），URL 见 OQ#20；二进制不入 git。
