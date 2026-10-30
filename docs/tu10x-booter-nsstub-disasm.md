# TU10x booter NS 桩反汇编与 mailbox/EXCI 语义定案（Phase C，2026-10-31）

> 对象：钉死资产 `release/assets-tu10x/booter_load_tu10x.bin`（59224 B = 72 B BTL1 + 59136 B image + 16 B sig；SHA 见 release 资产清单）。
> 工具：`envydis -m falcon -V fuc5 -i -n`（必须 `-V fuc5`，默认变体错位；`-i` 二进制输入）。命令：
> `wsl reference/envytools/envydis/envydis -m falcon -V fuc5 -i -n <blob>`
> 配套：[tu10x-booter-0x91-matrix.md](tu10x-booter-0x91-matrix.md)、[tu10x-c-parity-diff.md](tu10x-c-parity-diff.md)。

## C1. 镜像结构（BTL1 头逐字段，h(i)=bundle+8+4i）

| 字段 | 值 | 语义 |
|---|---|---|
| h(0)/h(1) | 0 / 0x100 | NS IMEM 偏移/大小（**NS 桩=image[0..0x100]，明文**） |
| h(2)/h(3) | 0x8500 / 0x6200 | DMEM 偏移/大小（25088 B） |
| h(5)/h(6) | 0x100 / 0x8400 | 安全 IMEM 偏移/大小（**密文**） |
| h(9) patch_loc | 0x8700 | 签名补丁位 |
| h(10) ucode_id | 13 | SEC2 ucode 槽 |
| h(13) sig_bytes | 16 | TU102 BOOT_DIRECT 签名 16B×1 |
| h(14)/h(15) | 0xe700 / 16 | image 59136 / sig 总 16 |

**熵**：NS 桩 256 B = **5.09 bit/byte（明文 fuc5）**；安全段 4 KiB 采样 = **7.955 bit/byte（AES 密文）**。与 fuc-disasm 既有结论一致：量产 booter 安全段离线不可 RE，只有 NS 桩可读。

## C2. NS 桩全量反汇编（image[0x100] 前缀，含注释）

```text
00000000: mov $r9 0x1000 ; iords $r9 I[$r9]      ; 读 I/O 0x1000（=mailbox0 所在口，见 C3）
00000006: mov $r15 0x1650 ; clear b32 $r14
0000000c: st b32 D[0x1650] $r9 ; st D[0x1654] $r14   ; I/O 状态快照存 DMEM 0x1650
00000011: mov $r9 0x1100 ; iords                    ; 读 I/O 0x1100（mailbox1）
00000017: mov $r15 0x1658 ; st; st                  ; 快照存 DMEM 0x1658
00000020: mov $r9 0x10000 ; mov $sp $r9             ; SP=DMEM 顶
00000027: lcall 0x2f
0000002b: exit
0000002d: ret
;---- 0x2f: 主流程 ----
0000002f: mpush $r0
00000031: mov $r13 0x200                             ; 描述符区 DMEM 0x200（=portal verify 读的 0x200！）
00000035: ld b32 $r14 D[0x200]
00000040: and $r0 $r9(0x100) $r15(0xfffffff)        ; $r0 = 0x100（安全入口，供 0xb1 使用）
00000043: bra $r14 0x0 ne 0x76                      ; DMEM[0x200/0x204/0x208/0x20c] 任一非 0 → 跳过拷贝
00000047/4e/55: (三连检查)
0000005c: DMEM[0..0x10) → DMEM[0x200..0x210) 拷贝  ; 空描述符时从 DMEM 头补默认
00000076: lcall 0xd6                                 ; → D[0x6140] = 0xd2408（全局 canary！与 §7.4 GA102 槽 0x6140 一致）
0000007a: mov $r15 0x31
0000007c: mov $r9 0x1000 ; iowrs I[0x1000] $r15     ; ★ mailbox0 ← 0x31（"正在进入安全段"标记）
00000082: mov $r15 0x1  ; iowrs I[0x46700] $r15     ; 引擎使能写 1
0000008b: mov $r15 0xd  ; iowrs I[0x46600] $r15     ; 引擎使能写 0xd
00000095: ??? (f4 3c 02)                             ; ★ envydis 不识别的特权指令（cxset 类密码上下文；见 falcon_crypt.xml）
00000098: mov $r15 0x60000 ; mov $r9 0x200 ; or → 0x60200
000000a5: xdst $r15 $r9 ; xdwait                     ; XDMA 传输设置（0x60200）
000000aa: $r15 = 0x20000 | (0x100>>8)=1 | (0x8500-0x100)<<0x10=0x840000 → 0x860001
000000c9: mov $s10 $r15                              ; 密寄存器 s10 = 0x860001（安全段入口上下文）
000000cc: lcall 0x100                                ; ★★★ 唯一进安全段的调用（固定 0x100）★★★
000000d0: exit
000000d6: (canary 子程序) mov $r15 0xd2408; st D[0x6140]
000000e2: ld D[0x6140] → push → lbra 0xf0            ; 陷阱尾巴：读 canary 入栈后死循环
000000f0: lbra 0xf0                                  ; 自旋
000000f4-0xfe: mov $r0 0x0 ×6                        ; 填充
```

（完整机器可读清单：`docs/evidence/tu10x-nsstub-fuc5.lst`。）

## C3. mailbox0 语义字典（实证收敛）

| 值 | 写入者 | 语义 | 证据 |
|---|---|---|---|
| meta 64 位地址 | loader（输入） | 输入载体 | 点火日志 |
| **0x31** | **NS 桩 @0x7a** | "进入安全段"标记 | 本反汇编；#62 成功对照日志 `mbox live sample0: 0x31 → halted 0x0`（成功时被最终态覆盖）；**PR #67 D/E 轮破坏入口后终值恰为 0x31**（安全段从未运行，无人覆盖） |
| 0x91/0x11/0x0B | 安全段 booter | **安全段运行后的状态码**（0x91=wpr_desc_validate 拒收族，cmp50hx 字典） | 硬核四例终值均非 0x31 → 安全段确实执行过 |
| 0x0 | 安全段 booter | 成功 | 成功链（travis `mailbox0=0x0`；#62 成功对照） |

**推论（修正既有两说）**：
1. "EXCI=0x00900100=入口崩溃" 与 "0x91=meta 拒收" **同时成立**：硬核四例的安全段**跑起来了**（写了 0x91/0x11/0x0B），拒收后 booter 以固定签名自毁——终态 EXCI 锁存 INV_INS@0x100（跳回安全入口且密码上下文已清 → 解码无效）。EXCI 是**拒收后的自毁残影，不是拒收原因**。
2. D/E 轮（搬址/直入）终值 0x31：安全入口被破坏、桩标记后即死——这类才是"入口级"失败。
3. EXCI 位域权威定义（open-gpu-kernel-modules `dev_soe_ip.h` NV_SOE_FALCON_EXCI，Falcon IP 通用）：EXPC[19:0]、**EXCAUSE[24:20]**；cause 9=**INV_INS**（无效指令）、0x0a=MISS_INS、0x10=DMEM_MISS_INS。`0x00900100` = EXPC 0x100 + **INV_INS**（此前"EXCAUSE=0x10=DMEM_MISS_INS"的解读错误；travis 指出的 [24:20]=9 正确，名字为 invalid instruction）。`0x00a00100`=MISS_INS@0x100（D/E 轮空页取指）。

## C4. 修正后的因果模型（与 Phase B 头号嫌疑闭环）

安全段密文经 `cxset` 类上下文 + Falcon CRYPT（falcon_crypt.xml：引擎内 crypto 单元，DEC/KEYREG 操作）在取指时解密——该通路在所有成功链上工作正常（安全段都跑过），**不是硬核四例的失败点**。硬核四例的 0x91 是**安全段 booter 的 wpr_desc_validate 拒收**：booter DMA 读回 WprMeta 与预期/或与 WPR2-MMU 实际状态不符。与 Phase B 头号嫌疑（main 在 FWSEC 未收尾、GSP 未 HALT 时就点火）衔接的机制候选：

- booter 校验 meta 中的 WPR 描述与**硬件实际 WPR2/MMU lock 状态**一致性——FWSEC 还在编程时就点火 → 状态未定型 → 拒收 0x91（机器固件速度决定成败 → 同机确定性、跨机分裂）；
- 与 staging 早释放（页复用）叠加：meta/radix 落在被复用页上 → booter DMA 读回错数据 → 同码 0x91（#14 v0.2.0→v2.1.1 同址翻转的页布局解释）。

两个子机制都由同一 marker（`\tx-fwsec-gsp-halt.txt`：精确 WPR2+GSP HALT 后才点火+延迟释放）一并消融；`\tx-fwsec-hold-dma.txt` 用于分解。

## C5. 对 Phase D 的具体要求（由此推导）

1. **portal 读回校验的对象**：NS 桩/DMEM 可逐字节校验；**安全段是密文**——读回（若硬件允许 NS 读安全页）应等于写入的密文原文；读回被拒/全 badf 也是有效观测（记录之，不判 fail）。
2. mailbox0 活采样序列 now 有完整字典：`meta addr → 0x31（桩）→ [0x91/0x11/0x0B（安全段状态）] → 0x0（成功）`。诊断日志按此解码打印（0x31 卡死=入口级，0x9x=安全段拒收级）。
3. EXCI 解码行按 C3 权威位域与命名输出，并注明"成功链亦出现，非独立判据"。
