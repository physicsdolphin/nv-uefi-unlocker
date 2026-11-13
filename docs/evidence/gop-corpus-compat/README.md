# GOP 解析全量兼容性实验（issue #107 native-GOP 路径安全验证）

2026-11-02，branch issue107-csm-compat。语料：本地 VBIOS 集 41 块
（GM200→Blackwell 5090 ES，`D:\git-repo\nvoc\reverse\VBIOS`，NVIDIA
固件不入库——只归档结论与 hash）。

## 两段验证

1. **rust-parser-sweep.log** — loader 实际使用的 romcore 解析器全量跑：
   41/41 解析成功。逐块：恰好 1 个 code-0x03 EFI 镜像、EfiSubsystem=11、
   machine=x64、CompressionType=1、payload 边界全部成立。
   实验暴露并修复一个真缺陷：Blackwell（98.xx）dump 是 euda 包装头，
   旧的 `%4` 启发式定位器被 0x33c00 处的假 AA55（PCIR 指针为 0）带偏 →
   `romcore::walk::locate_start` 升级为「0xAA55 且 +0x18 指向真
   PCIR/NPDS/RGIS 签名」，41/41 通过。
2. **reference-deep-pass.log** — 参考 Tiano 解码器（与固件
   EFI_DECOMPRESS_PROTOCOL 同语义）解压全部 payload + PE 校验：
   41/41 得到 PE32+ x64 subsystem=11 的 GOP 驱动。

## 黄金交叉验证

`AD104_4070S` 与 `Honor_4060` 两块卡解出的 GOP sha256 = `b69af88e…`
（164608 B，SizeOfImage 0x27ac0）= #107 报告者公布的 GOP hash ——
整条流水线对 #107 卡完成已知答案验证。

## 对 loader 的结论

- 解析器覆盖 Pascal→Blackwell 全代际；`恰一个 EFI 镜像` 的断言在语料内成立，保留。
- 全语料 CompressionType=1：未压缩分支目前无真机样本，保留实现并记录。
- 50 系 GOP 只有少数几个构建（多个共享同一 sha），按族发货的兼容包思路成立。
