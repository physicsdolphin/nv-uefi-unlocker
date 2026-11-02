<!--
中文：按检查单逐项确认，证据节放读回记录/日志链接；没跑真机请如实说明。
English: confirm each checklist item; put readback records / log links in Evidence; state honestly if no hardware run was performed.
-->

## 摘要 / Summary

<!-- 一句话说明本 PR 做什么、为什么 / One line: what this PR does and why -->

## 类别 / Kind

- [ ] 代码 / Code（loader / assets / sim / tools）
- [ ] 文档 / Docs
- [ ] 兼容性数据 / Compatibility data
- [ ] 打包与脚本 / Packaging and scripts

## 检查单 / Checklist

- [ ] 默认构建（不开 `tx`）零写路径不变 / Default (non-tx) build keeps the zero-write guarantee
- [ ] 未引入 NVIDIA 固件资产、密钥或解密产物 / No NVIDIA firmware assets, keys, or decrypted artifacts added
- [ ] `cargo xt` 全绿 / `cargo xt` passes
- [ ] EFI 产物改动已过 `check_efi.py` banner 验证 / EFI artifact changes verified via `check_efi.py`
- [ ] 行为主张附读回证据（状态码不构成执行证明） / Behavior claims include readback evidence (status codes alone are not proof)
- [ ] 运行时字符串英文，注释与文档中文 / Runtime strings in English; comments and docs in Chinese
- [ ] 内容已自查，无臆造日志/实测/验证 / Content self-reviewed; no fabricated logs, tests, or verification

## 证据 / Evidence

<!-- 读回记录、模拟输出或日志的链接/摘录；未跑真机请写明"未跑真机" /
     Links or excerpts of readback records, simulator output, or logs; write "no hardware run" if none -->
