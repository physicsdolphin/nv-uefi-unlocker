# falcgen — 第一性原理 V67 payload 生成器 · [English](README.en.md)

位置无关 fuc5 解码器 + 窄解释器 + gadget 扫描器 + **payload 合成器**：只吃解密的
booter 明文，自动推导栈几何，合成逐字节可验证的 V67 载荷——**写链是 payload 唯一
的政策输入**（`writes = [(addr, value), ...]`）。可行性研究底稿见
[docs/first-principles-payload-generator.md](../../docs/first-principles-payload-generator.md)。

脱离根 workspace 的独立 crate（与 [tools/ssdiag](../ssdiag/) 同车道：独立版本
0.1.0 / 独立 Cargo.lock / 独立 target，为独立 release 设计）。**零依赖、全离线**：
`synth` 不需要任何 NVIDIA 资产或输入文件；产物/金标/明文永不入库。

## 快速开始

```bash
# 在 tools/falcgen/ 内（或任意位置加 --manifest-path tools/falcgen/Cargo.toml）
cargo build --release        # 产物 tools/falcgen/target/release/falcgen.exe
cargo test                   # 单元 + 金标钉 SHA 自测（无条件跑）

# 各族 SS 解锁链预设（产物被 tests/goldens.rs 钉死在 pinned SHA 上逐字节验收）
falcgen synth --family tu11x --gate ss                 # = c79208ac…（pinned）
# AD102 BAR1 64G（= cf3566d0… 八卡工件，PROVEN；其余 2^n 档产 manifest 标 UNPROVEN）
falcgen synth --family ad102 --gate bar1 --bar1-size 64 --out payload.bin
# 原始写链（过帧预算校验，超限硬拒并打印各 die 上限表）
falcgen synth --family tu102 --writes 0x409650=0xffffffff,0x1fa828=0
```

`--out` 会同时落 manifest 侧车 `<out>.manifest.txt`：写链、marker、预期读回、
前置条件（FEAT PLM 写使能 nibble [7:4]=8 检查、Turing relock 尾提示）、UNPROVEN
标记。产物放 U 盘属用户门控决策，实机执行是另一回事。

## 子命令

| 命令 | 用途 |
|---|---|
| `dis` | 位置无关 fuc5 反汇编（`--crypt` 启用 CRYPT 特性位；`--all-offsets` 逐字节偏移解码，扫中项 gadget） |
| `scan` | gadget 分类扫描（forward-walk，未知容忍；已知入口 17/17 全中） |
| `lst-compare` | envydis oracle 对拍（offset/oplen/助记符三元组，零分歧验收） |
| `run` | 窄 fuc5 解释器（明文喂入、MMIO 打桩、DMA 即完成；`--break`/`--trace`/`--no-stop-after-dma`） |
| `synth` | payload 合成：原始写链或门控预设 + 帧预算校验 + 金标对拍 + manifest |

## 里程碑（全部落地）

- **G0** 位置无关解码器（envydis/falcon.c 18 表转录 + core-dis 语义）：五 die 解密
  段 54,670 条指令零分歧，360 个非规范偏移逐点一致（env 门控 oracle）。勘误：所谓
  "未知字节"是 **CRYPT 门控指令**而非表缺口——开 crypt 后五 die 零未知；SEC 入口以
  `csigclr` + csecret/cxor 清钥序列开场，故 scanner/interpreter 一律跑 `FUC5_CRYPT`。
- **G1** 栈几何推导（窄解释器 trace）：活帧 0xFF38/40/44/4C、步长 0x18、拷贝基
  0x600、主 SP 0xFFF0 全部 trace 证明。
- **G2** 金标逐字节复现：TU102 `b0ec9029…` / GA102 `2a0aa1f8…` / AD102 `44c36b2d…`。
- **G3** TU116 第一性原理首例（与手工单字重锚 `c79208ac…` 对拍仅 4 个操作数字位差）
  + 首个小说产品：SS0/SS1 直接写链（`1db35f51…`）。
- **金标自测不需要金标文件**：synth 是纯函数，预设产物被 `tests/goldens.rs` 钉死在
  pinned SHA 上——digest 即证明。

## 它增加什么（与"手改常量"的差别）

1. **零接触跨 die 移植**——几何（gadget 位点/活帧槽/步长/栈布局）只从解密 booter 段
   推导，每个新 die 的边际成本 = 一把 debug key + 一条命令；TU116（GTX 16 系）是
   第一个没有上游表、完全第一性原理产出的 die。
2. **超文法结构的自动验证**——多写链、尾块重排、不同 teardown：金标复现 + `sim/`
   行为级（`sim::run_payload_chain` 走 PLM 特权模型），不是 try-and-read-back。
3. **可达面本身的扩展**——新原语（位宽/DMA 携带写/读改写）需要新 gadget 推导，
   G0/G1 是前置。

硬边界：没有破出 debug 明文的 die 没有生成器输入（TU=#6 族、GA=#37 族已破；
AD/GA100 明文在库）。逐 die 写数预算（tu102/tu11x=2、ga102=1、ad102=4）是机械
边界，更大写链需要 pivot 重排（未实现）——见
[门清单与砸烂边界总账](../../docs/gate-inventory.md)。

## 纪律

- 宪法 #8：全离线、零 GPU 接触；实机执行是独立门控决策（里程碑门 + 显式确认）。
- Turing 交棒：PLM-open 交棒被 Linux 580.x GSP-RM 拒（issue #246），relock 尾
  `0xffffffef` + SS intact 是唯一已证形（manifest 按 family 提示）。
- 载荷引用的 DMEM 常量在 prod 上的正确性由"上游表实机有效"背书；钉版换版时把
  可利用性当一级属性复查。

## 文档与目录

- 操作手册与设计依据：[docs/falcgen-manual.md](../../docs/falcgen-manual.md)
- 门清单与砸烂边界（横向扩展总账）：[docs/gate-inventory.md](../../docs/gate-inventory.md)
- `src/`：isa/decode/scan/interp/synth + CLI；`tests/oracle.rs`：envydis 真值对拍
  （env 门控：`FALCGEN_SEC_DIR` 指向 RE 工作区明文，`FALCGEN_ENVYDIS` 为 WSL 内
  envydis 前缀——wsl.exe stdin 自 Windows 管道非二进制安全，故经 WSL 内脚本跑）；
  未设置则静默跳过。
- `gadget_scan.py`：历史 Python 原型（已被 `src/scan.rs` 取代），留作可行性证据。

许可：GPL-2.0-only（随仓库根 LICENSE）。
