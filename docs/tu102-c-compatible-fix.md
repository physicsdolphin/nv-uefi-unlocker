# TU102-C-FIX1：根据本机C成功记录实现的Rust修复候选

## 状态与范围

基于上游main `a99fe4b27d7d156051948ecf16483d63706afdf6`（版本2.1.5）。
本机TU102 1e07、22GiB、VBIOS90.02.17.00.e9已通过原生C路径解锁：
FP16/FP32累加4096为64.353→99.458 TFLOPS，8192为67.210→104.676 TFLOPS；
交接Windows前SS0=88888888、SS1=8，驱动616.56/GSP615.65.06正常。

**2026-10-29，本Rust修复候选也已在同一本机完成一次启动及短时算力验证。**
详细读回、性能数据与验证边界见[本机测试记录](tu102-c-fix1-hardware-result.md)。
成功C镜像与原始日志已备份；本候选通过U盘启动，没有在Windows中再次访问GPU MMIO。
目前不能从多变量变化中认定某一差异就是0x91的唯一根因。

参考源码：chenxluo/tu102-22g-uefi-unlock（MIT，基准6ee61bdaa8929e27926734f963873f356fc159ca），
以及用户技能包的FAT修复版。原MIT声明保留于third-party/tu102-c-reference-LICENSE.txt。
本机C运行源码仅相对技能包改了DID和版本提示；成功发生在A无复位首轮，没有执行B重试。

## 默认构建中的小修正

- 22GiB heap加入107MiB档位。此前落入8GiB的105MiB回退值，已与C成功参数核对。
- 修正Turing DMEM采样日志：只看到sanitize或未采到载荷填充值，不能推断“DMA从未到达”。
  既有#14/#16成功日志也存在采样阴性；新描述只报告观察，不虚构拒绝阶段。

其他GPU继续走现有tx路径；旧载荷生成器与默认只读构建保留。

## 隔离的TU102修复配置

`--features tu102-c-compat`只允许本机1e07/162000a1、容量指纹2c9、PROM前64KiB
CRC32=2fe1e7b2及本机FWSEC SHA256=584b5fe0…的组合。其他卡或更换ROM拒绝执行。
该配置是试验性单卡配置，不宣称支持全部2080Ti/22GiB卡。
普通tx不会自动选择这条路径。

与成功C首轮对齐的部分：

1. 保留主机探测的实际写序列。成功C日志证明它会改变部分PLM/SS可写位，
   所以不能当作纯只读诊断而从移植中省略。
2. GFW完成等待、PTIMER种子、高位保留内存16MiB分槽及分配顺序。
   参考C的RTC转换遗漏日期字段，种子为“月首+时分秒”；本兼容候选明确保留
   该算法，便于复现实验，不称其为精确UTC时间。后续需单独验证是否可以更正。
3. 使用同尺寸全零dummy RM镜像、同radix层级、256B WPR meta、22GiB/107MiB几何。
   不把真实RM内容与dummy方案混合；dummy的独立原始分配保留以对齐分槽顺序，
   其开头仅保存原签名恢复副本，booter实际消费的radix数据仍为全零。
4. 本机PROM动态解析FWSEC，固定匹配本卡未修补固件哈希；GSP的GFW退出、复位、
   scrub等待、BCR/RM、WITH_LOADER、缓存发布及portal读回按参考首轮顺序执行。
5. FWSEC不仅要建立精确WPR2，还必须在5秒内进入GSP HALT；否则不点火SEC2。
   避免把WPR2刚出现当作固件完成或提前复用缓冲区。
6. 生成独立的直接SS载荷，SHA256=de7288bc…5ce271；旧载荷不覆盖。
   SEC2沿用halted状态，无额外复位，不重写参考首轮保留的TRANSCFG。
   单次START后静默3秒，再读回；不运行中途DMEM采样。
7. 两个SS精确匹配才成功，PLM可以已恢复ffffff8f。之后SEC2 sanitize，再确认SS保留。
   不做成功FLR、GOP断连或重连；宿主meta中的超长签名引用先恢复为原签名副本。

有意不复制的部分：

- C的B复位重试及GA102备用点火：本项目规定失败同会话不重试。
- ReBAR调整、NVRAM/ESP安装、主板资源重分配：与本轮算力路径修复无关。
- 用状态码或日志文字代替SS读回；HALT或mailbox0=0本身不构成成功。

## DMA与错误处理

每个分槽按页取整，地址从0x110000000开始；UEFI AllocateAddress不成功即换下一候选，
最多12个候选，不降级为低地址或其他内存类型。所有缓冲区从分配开始由主调方跟踪。
固件执行阶段不释放。成功sanitize或失败emergency FLR后检查两个引擎的HALT，
满足条件才释放；若仍不能证明停止，明确记录保留页至下次重启，避免释放活跃DMA。
这可能让失败会话暂时保留约56MiB内存；它不是常驻驱动，也不是持久固件修改。

预检要求WPR2尚未初始化，防止热态重复事务。清理时恢复PCI命令字并读回。
身份、输入、PM D0预检失败发生在任何GPU写之前，不做emergency FLR。
已解锁会话返回EAlready，不重复点火。预检拒绝旧实验标记及自动武装文件，保留空格确认。

## 测试与可复现构建

- `cargo xt`：原有矩阵及新增生产MMIO路径测试。
- `tools/check-tu102-c-reference.py --source <成功C源码> --clang <clang.exe>`：
  提取真实C结构体和build_wpr_meta函数独立编译，产生256B合成metadata fixture；
  Rust生产打包函数逐字节匹配它。fixture不含NVIDIA固件。
- MMIO mock覆盖静默窗口、唯一START、无额外SEC2复位/TRANSCFG写、部分SS写丢失、
  mailbox0=0但SS未变、HALT超时、BADF、scrub未完成及sanitize后SS保留。
- `cargo run -p assets -- generate-tu102-direct <out.bin>`：从数据表重建载荷并校验完整SHA256。
- `cargo efi`、普通`--features tx`和`--features tu102-c-compat`分别编译，确认隔离。
- `tools/build-tu102-fix.ps1`仅执行宿主测试和编译，不访问U盘、GPU、ESP或启动项。

本机打包目录中的新EFI、源码补丁、测试日志、C fixture校验及输入清单可供审阅。
NVIDIA固件、生成载荷、编译产物和私人日志不加入Git；payload生成代码可重现已验证哈希。

## 复测方法与边界

另行装载候选前，保存当前成功C镜像和本轮证据。准备文件：
`BOOTX64.EFI`、`booter_load_tu10x.bin`、`gsp_boot_tu10x.bin`、
`fwsignature_tu10x.bin`、`sec2_bl_tu10x.bin`、`v67-tu102-direct-ss.bin`。
本配置的dummy由内存生成，不要求fwimage_tu10x.bin或FB覆盖标记。

Shift关机并实际断电，选择一次性UEFI U盘，确认`TU102-C-FIX1`再按空格。
回来核对FWSEC/GSP HALT、单次START、两个SS、sanitize后状态及同次启动性能。
若失败，保存日志并恢复已验证C镜像。当前成功证据仅覆盖本卡的一次启动与短时测试，
未验证多次冷启动、长期稳定性或其他TU102卡；多变量移植尚不能定位唯一根因。
