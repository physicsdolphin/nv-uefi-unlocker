# falcgen — 第一性原理 V67 payload 生成器：设计文档与操作手册

> 分支 `falcon-booter-payload-gen`（研究线）；主线资产仍以钉值表为准。
> 调研底稿：[first-principles-payload-generator.md](first-principles-payload-generator.md)。
> 本文 = 工具的完整设计说明 + 操作手册 + 验证状态 + 新 die 工作流。

---

## 1. 定位与能力边界

falcgen 是一个**离线研究工具**（Rust，workspace 成员 `tools/falcgen`，零外部依赖），
从解密 booter 明文出发完成三件事：

1. **读懂 booter**：位置无关 fuc5 反汇编（与 envydis 逐条对拍零分歧）+ gadget 扫描 + 窄解释器。
2. **推导 payload 几何**：gadget 地址、帧形状、链机制——全部从明文逆向，不抄钉值表。
3. **合成 payload**：写链（唯一政策输入）+ per-die 几何 → payload 字节。

**验证状态（2026-11-07）**：三家钉值 golden 全部**逐字节复现**——

| die | 命令 | SHA-256 | 对照物 |
|---|---|---|---|
| TU102 | `synth --writes 0x409650=0xffffffff,0x1fa828=0` | `b0ec9029…` | 本地再生成（pinned 包） |
| GA102 | `synth --family ga102 --writes 0x823804=0xffffffff` | `2a0aa1f8…` | `assets/out/v67-ga102.bin` |
| AD102 | `synth --family ad102 --writes 0x1fa7cc=…,0x9a0148=…,0x1fa7c4=…,0x823804=…` | `44c36b2d…` | `assets/out/v67-ad102-four-plm.bin` |

**不是什么**：

- **不是 loader 的生产路径**。主线 loader 仍用 `assets/src/payload.rs` 的钉值表；
  synth 产物进入 loader 前必须走完 sim 验证 + 宪法第 1.8 条（先 mock 后真机）+ UNMEASURED→PIN 流程。
- **不做密钥、不碰加密产物**。输入 = 解密明文（TU=#6/GA=#37 族 key 已破的 die 才有输入）；
  明文与 golden 载荷**永不入库**（资产纪律）。
- **零 GPU 接触**。G0–G2 全程离线；G3 的真机验证是独立立项。

---

## 2. 架构

```
解密明文 (*.bin)                          envydis *.lst (oracle)
     │                                        │
     ▼                                        ▼
┌─────────┐  ┌─────────┐  ┌──────────┐  ┌────────────┐
│ isa.rs  │→ │ decode  │→ │  scan.rs │  │ lst-compare│   G0：读懂
│ falcon.c│  │ 引擎    │  │ gadget   │  │ (oracle)   │
│ 18 表   │  │core-dis │  │ 候选分类 │  └────────────┘
└─────────┘  └────┬────┘  └──────────┘
                  ▼
           ┌────────────┐
           │  interp.rs │   G1：栈几何（窄解释器）
           │ 窄解释器   │   服务邮箱/PRI读/fuse/XDMA 环境模型
           └─────┬──────┘   活帧链 trace 实证
                 ▼
           ┌────────────┐        ┌──────────────┐
           │  synth.rs  │←───────│ Geometry     │   G2：合成
           │ 帧文法引擎 │        │ (per-die)    │
           └─────┬──────┘        └──────────────┘
                 ▼
            payload 字节（SHA 对拍 golden）
```

### 模块清单（约 3200 行，全部零依赖）

| 模块 | 行数 | 内容 | 验证 |
|---|---|---|---|
| `isa.rs` | 709 | envydis/falcon.c 18 表逐条转写（~380 条目）+ 变体门控（FUC5 = fuc3op+fuc4op+fuc5p；CRYPT 显式开启）+ 条件码表 | 与 envydis 助记符逐条一致 |
| `decode.rs` | — | core-dis.c 引擎语义：`decode_one`（任意字节偏移可入——中项 gadget 天然支持）、`decode_linear`、`parse_lst` | 5 die 54,670 条零分歧 + 360 非规范偏移对拍 |
| `scan.rs` | 119 | 前向行走 gadget 分类（LCall/DmemStore/DmemLoad/DmemImm/ParkSelf/LBra），未知字节容忍 | 已知代码入口 17/17 |
| `interp.rs` | 1086 | 窄 fuc5 解释器：寄存器/栈/DMEM/IO/DMA/crypt 桩 + **环境模型**（服务邮箱 0xd000-0xd500、PRI 读服务 0xf27、fuse 行写、XDMA $xdbase+xdld/xdst/xdwait） | 15.8M 步真实流程；活帧链全实证 |
| `synth.rs` | 291 | 帧文法引擎：TU102 帧循环语法（Geometry 参数化）+ GA102/AD102 put-list 模板 | 三家 golden 逐字节 |

### 关键逆向结论（设计依据，细节见调研底稿）

- **帧形状**：`mpopaddret $r3 0x4` 尾声族（9 处，0x900/0x9a1/0x2b9b 类服务帧）=
  帧 [sp]=r3、[+4]=r2、[+8]=r1、[+C]=r0、[+0x10]=canary、[+0x14]=ret；展开恰 0x18。
- **golden 链帧**：guard@[+0]（=canary 地址作值，**不是** canary 伪造——canary 检查
  [帧+0x10] vs D[0x6140] 双双 = fill 天然通过）、value@[+8]、addr@[+C]、gadget@[+0x14]=pc。
- **fuc5 条件码 a/b = 无符号**（envytools branch.rst 明文）——sig_size cap（≤0x1000）
  恰卡在 stock 签名尺寸上（驱动 `.fwsignature` 节对齐 256 = 恰 0x1000）。
- **0x7a38 mismatch handler** = mailbox0←0x47 自旋 park——实机"永不 halt"的来源；
  golden 不触发（canary 天然通过）。
- **token 族**（0x10B9/0xCBD/0xC95/0xE85/0x1F89）= 各 helper 的中项入口：
  body 消费 pop 交付的寄存器值（写执行器读 r1/r2），尾声同形串下一链帧。
- **per-family 文法**：GA-line/Ada booter 尾声形状 ≠ TU102——GA/AD 走显式 put-list 模板
  （写链槽位参数化）；"从明文推导 put-list" = 机械工作（对拍 fill 差分即得全部 put 位）。

---

## 3. 构建与前置

```bash
# 工具本身（workspace 成员，零依赖）
cargo build --release -p falcgen
cargo test -p falcgen          # 单元测试（无条件跑）

# oracle 对拍（可选，env 门控；需要 RE 工作区与 WSL envydis）
export FALCGEN_SEC_DIR=D:/git-repo/nv-uefi-re          # 五 die 解密明文 + .lst
export FALCGEN_ENVYDIS="wsl -e /mnt/d/.../envydis"     # WSL 命令前缀
cargo test -p falcgen                                   # oracle 测试自动启用
```

**输入资产**（永不入库）：

| 资产 | 位置 | 用途 |
|---|---|---|
| 解密 booter 明文 | `D:/git-repo/nv-uefi-re/<DIE>_sec_plain.bin` | dis/scan/run 的输入 |
| envydis lst | 同目录 `<DIE>_sec.lst` | lst-compare 的 oracle |
| 钉值 golden 载荷 | `assets/out/v67-*.bin`（TU102 需自生成，见 §5.4） | synth --golden 对照 |

**TU102 golden 再生成**（cmpunlocker 参考实现 + pinned 包）：

```bash
# Windows 无 zstd，走 WSL；包 SHA 与生成器 PACKAGE_SHA256 一致才有效
wsl -e bash -c "cd /mnt/d/.../reference/cmpunlocker/linux/cpu-rm/firmware && \
  python3 generate_v67.py /mnt/d/.../assets/in/nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst <out-dir>"
# 产物 v67-tu102-610.43.03.bin 的 SHA = b0ec9029…（与 GSP_SHA256 一并自验）
```

---

## 4. 操作手册

### 4.1 `dis` — 反汇编

```bash
falcgen dis <file> [--base <va>] [--crypt] [--all-offsets]
```

| 参数 | 默认 | 说明 |
|---|---|---|
| `--base` | 0 | VA 基址（booter SEC 段 = 0x100） |
| `--crypt` | 关 | 开启 CRYPT 特性位（booter 必开——SEC 入口有 cxset/csigclr 族） |
| `--all-offsets` | 关 | **每个字节偏移**各解一条（中项 gadget 搜索用；输出大） |

```bash
# 规范流
falcgen dis TU102_sec_plain.bin --base 0x100 --crypt > tu102.lst
# 每字节偏移（找中项 gadget）
falcgen dis TU102_sec_plain.bin --base 0x100 --crypt --all-offsets > tu102-alloff.txt
```

输出行：`<va> n=<oplen> <mnemonic operands>`；未指令字节 = `??? … [unknown: hex]`。

### 4.2 `scan` — gadget 扫描

```bash
falcgen scan <file> --base <va> [--crypt] [--check <va>,<va>,...]
```

前向行走分类（每偏移 ≤N 条，未知字节容忍），产出候选表；
`--check` 对指定 VA 报 HIT/miss 及其原语标签。

```bash
falcgen scan TU102_sec_plain.bin --base 0x100 --crypt \
  --check 0x10b9,0x1f89,0xcbd,0xc95,0xe85,0x7ea4
# candidates: 25897
#   0x010b9: HIT [LCall]
#   0x07ea4: HIT [ParkSelf, LBra]
#   ...
```

标签语义：`LCall`（长调用=链汇聚点）、`DmemStore/DmemLoad/DmemImm`（DMEM 访问）、
`ParkSelf`（自旋=park 候选）、`LBra`。**新 die 的 gadget 族由此找回**（不抄钉值表）。

### 4.3 `lst-compare` — envydis 对拍（oracle）

```bash
falcgen lst-compare <plain.bin> <envydis.lst> --base 0
```

逐条对拍 (offset, oplen, mnemonic)；零分歧退出码 0。**回归守护**：isa.rs/decode.rs
任何改动后必须重跑（五 die 全绿才算数）。

### 4.4 `run` — 窄解释器

```bash
falcgen run <file> --base <va> [--trace] [--steps N] [--sig-len N] \
            [--sig-hex XX..] [--break <pc>]
```

| 参数 | 默认 | 说明 |
|---|---|---|
| `--steps` | 20,000,000 | 步预算 |
| `--sig-len` | 0x10 | 种子进 meta+0x50（stock=0x10；payload 模式=0xFA00） |
| `--sig-hex` | 0xaa 填充 | 签名缓冲内容（hex 字节流） |
| `--trace` | 关 | 全指令 trace（stdout，量大——重定向后 grep） |
| `--break` | — | 执行到 PC 前停（帧链抓取点） |

**内置环境模型**（全部从 booter 自身代码逆向，改动需 trace 实证）：

- WprMeta 种子于 DMEM 0x400（TU102 11-GiB 自洽布局；meta 校验链 0x89/0x8a/0x91… 依赖它）
- 服务邮箱（IO 0xd000-0xd500）：0xd000=就绪位 0x1c0000、0xd100=提交、0xd200=写载荷、
  0xd400=门铃/ack、0xd500=响应值（per-id 种子在 `seed_meta_sig_len`）
- PRI 读服务（0xf27）：IO 0x1c100=行选、0x1c000=触发（f1 读/f2 写）、0x1c200=行数据
- fuse 行表（0xa00 状态、0x21434/0x118128/0x118234/0x20800/4、0x1fa81c-28 WPR2/FRTS 活值…）
- XDMA：$xdbase 页基址 + xdld/xdst 0x100B 块 + xdwait 完成；快照 = sig-page（xdbase=6）
  且触及 0xff00+

**输出**：`halted/reason/steps` + DMA 副本表 + 栈尾 dump（非零词，带 canary/imem-range 标注）
或 IO 写尾表。

```bash
# 全流程（15.8M 步级）
falcgen run TU102_sec_plain.bin --base 0x100 --steps 50000000
# 帧链抓取：停在签名 DMA 调用点，栈尾即活帧
falcgen run TU102_sec_plain.bin --base 0x100 --steps 50000000 --break 0x39bb
```

**判读**：`reason=exit` + SCRATCH0（IO 0x1000）状态码 = booter 自身判定；
常见码：0x1c 输入拒绝族、0x91 WPR2 1MiB 窗口、0x73 L2 熔丝 vs fbSize、
0x1d 描述符块头、0xb 使能位、0x47 canary-mismatch park。**trace 是唯一裁判**——
状态码只指认"哪个检查"，`--trace` 才给"为什么"。

### 4.5 `synth` — payload 合成

```bash
falcgen synth --family tu102|ga102|ad102 --writes <addr>=<val>[,...] \
              [--golden <file>] [--out <file>]
```

- `--writes`：写链（十六进制，`addr=value` 逗号分隔；无 `=` 视为 value=0）。
  **写链 = 唯一政策输入**，顺序 = 执行顺序。
- `--golden`：对照物（不入库！）；逐字节比对，差异报前 8 处（带 DMEM 偏移）。
- `--out`：写出产物（同样不入库）。

```bash
falcgen synth --writes 0x409650=0xffffffff,0x1fa828=0 \
  --golden v67-tu102-610.43.03.bin
# payload: 64000 bytes, sha256 b0ec9029…
# BYTE-IDENTICAL ✓
```

**TU102 走帧循环语法**（Geometry 参数化：fill/guard/main_sp/活帧/链基/步长/gadget 族/尾块）；
**GA102/AD102 走 put-list 模板**（GA-line/Ada booter 尾声形状不同——GA102 单写 +
0x81ee 终止符；AD102 四写链 + marker 0x67b00001 + 0x31b5/0x771e 收尾）。

---

## 5. 新 die 工作流（G3 recipe）

以 TU116 为例（TU11x，debug key #6 族已破；无 golden，对拍对象 =
`assets/src/payload.rs` 的 `PATCHED_WORDS_TU11X` 手工单字重锚）：

1. **明文就位**：TU116_sec_plain.bin（不入库）。
2. **`dis`**：规范流 + all-offsets 两份。
3. **`lst-compare`**：与 envydis 零分歧（解码器回归）。
4. **`scan`**：找回 gadget 族——写中项、teardown、token、park（对照 TU102 的
   17/17 标签形状；`ParkSelf` = park 候选）。
5. **`run`**：驱动到签名 DMA 调用点（`--break`），栈尾 = 活帧链；
   逐帧记 prologue/epilogue 形状 → Geometry（fill/guard/main_sp/帧位/步长）。
6. **`synth`**：新 Geometry + 写链 → payload；**逐词对拍 PATCHED_WORDS_TU11X**——
   差异即发现（单字重锚只改了写值槽，其余应逐位吻合）。
7. **资产化**：对拍全绿后按 UNMEASURED→PIN 流程入 manifest（宪法）；
   真机验证 = 独立立项。

**判定纪律**：推导值与手工锚不一致时，**先怀疑推导**（trace 复核），
再怀疑手工锚（当初是单字人工修正）——以 booter 代码为准，两轮对不上就开 issue。

---

## 6. 约束与诚实边界

1. **尾块级联常量部分逆向**：GA102/AD102 的 put-list 常量（0x8E18/0xC95/0x1F89/
   0xE85/0x7EA4/0xCBD/0xFFFF token 等）语义 = mutex 释放级联（部分理解），
   当前以 per-die 常量进模板。**G3 的 TU116 推导若在这些常量上失败，
   即为逆向深度的诚实边界。**
2. **三家 golden ≠ 三家独立验证**：GA/AD 的 put-list 模板结构参考了
   generate_v67.py 的同族实现；逐字节复现证明的是"文法+常量无误"，
   不是"文法从零推导"。**TU116（无参考实现）才是真测试。**
3. **sig_size 分支矛盾未闭**：payload 模式（0xFA00）在 TU102 build 的 meta
   校验（0x13b4）即拒，但 TU106 实机跑通——golden 流程走的 ORCH 分支
   尚未在解释器里锁定（见调研底稿"sig_size 之谜"节）。
4. **解释器环境模型 = 非完备**：服务响应值/fuse 行是按当前流程种的
   **最小充分集**；新分支可能需要新种子（判据：trace 停在哪就补哪，
   每个种子必须能指认"booter 哪段代码在读它"）。
5. **载荷不入库**：golden、synth 产物、解密明文一律留在本地/临时目录；
   入库的只有代码、文档、哈希钉值。

---

## 7. 踩坑记录（改代码前先读）

1. **fuc5 a/b = 无符号比较**（envytools branch.rst 明文）——任何"signed"直觉
   都要对照 branch.rst。
2. **ld/st 宽度**：b8/b16/b32 修饰必须生效——曾经 st b8 写 4 字节砸邻居
   （幻影 0x8d）。
3. **sethi 保低位**：`sethi r9,0` 保留 bits 15:0——块搬运器的
   `ld b16 + sethi` 组合靠它。
4. **adc/sbb 吃进位**：64 位 meta 运算链全靠它。
5. **mpopaddret 全恢复**：r0/r1 是 caller 契约；返回值走 $r10 在 pop 前移入。
6. **wsl.exe stdin 管道非二进制安全**：字节流走 WSL 内重定向。
7. **十六进制字面量的十进制误读**：`0x19c` 的十进制 = 412——写分析脚本时
   candidate id 与 count 分开打印并带 `#x` 格式。
8. **`--crypt` 别忘**：默认变体解不了 booter（SEC 入口即 crypt 指令）。
9. **状态码只指认检查点**：`mov $rN <err>` 是无条件默认值，
   成功路径的 clear 才是真信号——追"为什么失败"必须上 trace。

---

## 8. 相关文档

- 调研与里程碑底稿：[first-principles-payload-generator.md](first-principles-payload-generator.md)
- booter RE 工具链（key 族/解密脚本）：memory `fuc-disasm-toolchain`
- 主线资产（钉值表权威）：`assets/src/payload.rs`、`assets/src/manifest.rs`
- 宪法与门控：根目录 `AGENTS.md` §1、§6、§7
