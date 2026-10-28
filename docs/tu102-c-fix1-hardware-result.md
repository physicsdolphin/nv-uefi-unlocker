# TU102-C-FIX1 本机实测记录（2026-10-29，UTC+8）

关联 physicsdolphin/nv-uefi-unlocker#17。上游基准 `a99fe4b27d7d156051948ecf16483d63706afdf6`。
本次通过 U 盘一次性 UEFI 启动，空格确认，Windows 启动时间 04:47:15。
Secure Boot 关闭、Above 4G Decoding 开启；测试流程要求完全断电，日志本身不能证明物理断电动作。

## 硬件与镜像

- RTX 2080 Ti 改装 22 GiB，PCI `10de:1e07`，subsystem `24893842`。
- 芯片 `0x162000a1`，VBIOS `90.02.17.00.e9`，容量指纹 `0x2c9`。
- PROM 前 64 KiB CRC32 `2fe1e7b2`；原生 FWSEC SHA256 `584b5fe0826c40aaa8731d70f4fe5fb505380075dca77f7a94d31cfa09d20f6e`。
- Windows 驱动 `616.56`，显存 `22528 MiB`，功率限制 `360 W`。
- EFI 173568 B，SHA256 `48100df15a69b0925b41368595614351767b43a4bc2948e4fab9d4b6f0de3df7`。
- 原始 loader.log 16059 B，SHA256 `89aa4015c335cfd8b9c15d7a40b74a5b02535e9b421c6e7dd56abf550c11754d`。
- 附带日志节选按原始顺序保留相关行，省略 PCI 枚举、文件目录等无关信息；节选哈希不等于原始文件哈希。

## 结果与性能

FWSEC 精确 WPR2 与 GSP HALT 验证通过；booter mailbox0=0；PLM=ffffff8f。
SS0=88888888、SS1=8，sanitize 后以及 HANDOFF-OUT 的 tu10x 字段均保持目标值。
HANDOFF-OUT 通用字段的 BADF/false 不适用于本卡，同行 tu10x_unlock=true 才是本次判断。
Windows 正常加载驱动；性能验证与日志读回相互支持。

cuBLAS GemmEx，同卡/驱动/360W，seed=12345，64 GEMM/batch，CUDA event计时，
每项预热3秒、3组各3秒，报告中位数；每项32点与CPU FP64参考抽样比较，全部通过。

| 模式 | N | 解锁前 TFLOPS | 原生C成功后 | Rust FIX1 | Rust较解锁前 |
|---|---:|---:|---:|---:|---:|
| FP16输入/FP32累加 | 4096 | 64.353 | 99.458 | 97.388 | +51.33% |
| FP16输入/FP32累加 | 8192 | 67.210 | 104.676 | 100.962 | +50.22% |
| FP16输入/FP16累加 | 4096 | 107.491 | 106.844 | 108.212 | +0.67% |
| 普通FP32 | 4096 | 15.430 | 15.369 | 15.431 | +0.01% |

Rust 本轮最高采样温度49℃；FP16/FP32两项比C分别低2.08%和3.55%。
这是独立短测试，尚不能判断该差值的统计意义或归因。未执行长期稳定性测试。

## 结论边界与来源

- 单次本机成功，不宣称普适 TU102 修复；身份门控不会因换设备ID而自动适配其他卡。
- 多个参考行为同时移植，没有完成逐项消融，不能把 dummy RM、heap、GSP HALT、PTIMER 或探测副作用中的任何一项称为唯一根因。
- 原生C对照来自 [chenxluo/tu102-22g-uefi-unlock](https://github.com/chenxluo/tu102-22g-uefi-unlock)，并采用用户提供的同谱系FAT修复源码；DID和横幅适配本机。
- 直接SS载荷生成表来自 xrip/cmp50hx-unlock 的已注明MIT版本；许可保留。NVIDIA固件、生成载荷及EFI不提交。
- 宿主测试92项通过（assets 21 + sim 71），默认只读、普通tx及兼容feature均已编译；256 B元数据与实际C函数独立编译结果逐字节一致。
