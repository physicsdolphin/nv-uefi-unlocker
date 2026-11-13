# #156 rc2 第三版（BME 重臂）裸跑成功——STAGING-FALLBACK 全链闭环（2026-11-10）

## 构建与配置

- v2.1.12-rc2 换件版（sha `c99779df…`，含 0ac10f6 BME 重臂修复），**无 marker 裸跑**，默认 HIGH
- 机器：`1e93`（2080S Mobile DGFF，Area-51m R2）——高位不可达类实机（#74/#156/#191 类）

## 判据链（全绿）

1. 首发 HIGH：booter `mailbox0=0x89`（≥4GiB 读未送达）→ 回退门开火
2. `FLR done (cold engine, WPR2 wiped)` → `GPU BME re-armed (cmd=0x0003 -> 0x0007)` **← 新增判别行，实证 BME 缺失即上轮楔死根因**
3. FWSEC 重新点火：`post-START CPUCTL=0x00000000`（核已启动——判别行落地）→ `WPR2 UP after 3ms` → HALT gate 87ms
4. LOW 重建 → 重试 booter `mailbox0=0x0` → `LOW-band retry verified (staging=low(fallback))`
5. selectors `0x88888888`/`0x8` 写读回一致 → FLR → 交棒
6. `HANDOFF-OUT ... tu10x: plm=0xffffffff ss0=0x88888888 ss1=0x00000008 readout=0x00000000 | tu10x:unlock=true` + `canary=intact`
7. 首发同样带上了 STARTCPU 判别行（`0x00000000`，3ms WPR2 UP）——正常路径的诊断增强双向落地

## 意义

- **免 marker 路径在 1e93 成立**：高位不可达机从"必须手工 `\tx-low-staging.txt`"变为"插盘即解锁"。
- STAGING-FALLBACK 三版演化收官：v1（原地再开火，FBIF 写面证伪）→ v2（FLR 全链重跑，BME 缺失楔死）→ **v3（BME 重臂，端到端成功）**。每一版死点都由报告者日志定位（FBIF 三行表 → CPUCTL 轨迹 → 本轮两行判别全部按预期读出）。
- `#74`/`#191`（同类高位不可达）自动受益；rc2 可转正。
