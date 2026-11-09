# 2820（RTX 4070M 魔改卡）路由修复案证据（issue #10）

## 本目录内容

`loader.log` = **修复前的 bug 证据**（v0.2.0，2026-10-27 构建，MSI PRO Z690-A D4）：`TX: device 10de:2820 has no asset family mapping - skipping`——路由缺失导致事务零执行、直接链回 Windows。SAMPLE 段正常（AD106 Ampere 簇可读：FEAT 0x823xxx 全 ok）。

## 成功侧证据（无 log，按层级收录）

路由修复（v2.1.0）后报告者确认解锁成功：B 站评论（维护者关闭 issue 的依据，"看到B站评论你解锁成功了，我先关闭了，后续有问题 reopen 即可"）+ issue 正文 GPU-Z 截图。**无修复后 loader.log、无系统内 ssdiag 读回、无性能 A/B。**

## 矩阵口径（维护者裁定 2026-11-10）

bug 数据点（路由缺失）被已实机验证的修复（v2.1.0 路由表）+ 同卡修复后成功确认覆盖 → 按 ✅ 支持计；证据层级局限如实保留（成功确认无日志附件）。issue 状态：CLOSED COMPLETED（2026-11-01）。
