# 社区复现截图 / Community screenshots

[中文介绍](../README.md#社区成功反馈与复现截图) · [English overview](../README.en.md#community-success-reports-and-screenshots) · [兼容性矩阵](compatibility.md)

这里收录 15 位观众的 22 张图片，来自[项目视频 BV1d6xN66ECM 的公开评论区](https://www.bilibili.com/video/BV1d6xN66ECM/)。资料采集于 **2026-11-02 18:51:09–19:07:33（北京时间）**。

这是观众反馈的存档，未独立实机复测。终端读回、前后基准、只有解锁后数值和 AI 汇总报告分别注明；AI 汇总图本身不能替代原始日志。相同型号的不同平台、VBIOS、功耗和驱动条件不可直接合并。记录中的版本与部署方式仅描述当时实验，当前操作请看 [README](../README.md#开始使用)。

21 张图片保持导出原始字节；S48 仅增加 EXIF 显示方向，让照片正向阅读，JPEG 图像数据未重编码。索引同时保留该图的原始 SHA-256 与发布文件 SHA-256；点击图片可查看完整尺寸。每张图都保留评论者署名、原评论、原图链接与 SHA-256（见[来源索引](evidence/bilibili-2026-11-02/sources.json)）。索引还包含 README 硬件表中其余文字报告及必要上下文的链接，不包含完整评论导出或账号资料。

## 有前后对比的报告

<a id="s30"></a>

### S30 · RTX 3060 · 哲学香蕉菌

来源：[哲学香蕉菌，2026-10-30](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318661261937&comment_secondary_id=318661261937)（评论 `318661261937`）。

**报告结果：** FP16 25.37 → 45.79、BF16 25.29 → 46.31 TFLOPS（4096）；Anima BF16生图1.20 → 1.94、0.599 → 1.05、0.402 → 0.685 it/s。

**条件与边界：** 3060 12GB GA106；v2.1.6、驱动617.42。图中不同分辨率生图吞吐提高约62%–75%；功耗147.9→170.2W。

型号或后续更正的上下文：[318652838881](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318652838881&comment_secondary_id=318652838881)。

<details>
<summary>查看原图（1 张）</summary>

<p align="center">
<b>基准与 Anima 生图对比（观众汇总）</b><br>
<a href="evidence/bilibili-2026-11-02/318661261937-1.png"><img src="evidence/bilibili-2026-11-02/318661261937-1.png" width="880" alt="基准与 Anima 生图对比（观众汇总）"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/ed342d333f7b788e39ce73a86b5e2357571852961.png">评论原图</a> · <a href="#s30">来源与条件</a>
</p>

</details>

<a id="s28"></a>

### S28 · RTX 3080 10GB · laxy2333

来源：[laxy2333，2026-10-30](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322392362704&comment_secondary_id=322392362704)（评论 `322392362704`）。[此前成功反馈](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322352868720&comment_secondary_id=322352868720)与补图为同一作者；补图原文为“补图补图，丐版3080大约50%以上提升”。两张测试图均显示设备 NVIDIA GeForce RTX 3080、显存 10.0GB。

| 同一组补图中的指标 | 解锁前（图 1） | 解锁后（图 3） | 相对变化 |
| --- | ---: | ---: | ---: |
| FP16 混合负载测试摘要 | 53.91 TFLOPS | 84.60 TFLOPS | +56.9% |
| BF16 混合负载测试摘要 | 54.97 TFLOPS | 81.74 TFLOPS | +48.7% |
| 系统侧状态（图 2） | 未附对应读回 | `VERDICT [ampere]: UNLOCKED` | 状态截图单独列出 |

**测试条件：** 混合负载脚本的 Phase 1 摘要，不是应用端到端耗时。图中未提供两轮对应的驱动版本、功耗、频率与超频设置，因此仅记录本次观众测试，不按严格控参实验外推。此前 Family 选错的操作记录只描述当时部署过程。

<details open>
<summary>查看前后对照与系统侧状态（3 张原图）</summary>

<table>
<tr>
<td align="center" valign="top" width="50%">
<p align="center">
<b>解锁前</b><br>
<a href="evidence/bilibili-2026-11-02/322392362704-1.png"><img src="evidence/bilibili-2026-11-02/322392362704-1.png" width="420" alt="解锁前"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/b839cdf0263aa0f4ba4a8f5a27fe0f55219938505.png">评论原图</a> · <a href="#s28">来源与条件</a>
</p>
</td>
<td align="center" valign="top" width="50%">
<p align="center">
<b>解锁后</b><br>
<a href="evidence/bilibili-2026-11-02/322392362704-3.png"><img src="evidence/bilibili-2026-11-02/322392362704-3.png" width="420" alt="解锁后"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/b662ff17923e795151329772fb5fd428219938505.png">评论原图</a> · <a href="#s28">来源与条件</a>
</p>
</td>
</tr>
</table>

<p align="center">
<b>系统侧 UNLOCKED 读回（不代替上方性能对比）</b><br>
<a href="evidence/bilibili-2026-11-02/322392362704-2.png"><img src="evidence/bilibili-2026-11-02/322392362704-2.png" width="880" alt="系统侧 UNLOCKED 读回（不代替上方性能对比）"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/e0c3437116e95c583ce7a710b6f8f133219938505.png">评论原图</a> · <a href="#s28">来源与条件</a>
</p>

</details>

<a id="s12"></a>

### S12 · RTX 3070 Laptop · Ndxopcgvx

来源：[Ndxopcgvx，2026-10-29](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318487921761&comment_secondary_id=318487921761)（评论 `318487921761`）。

**报告结果：** FP16 42.05 → 82.06 TFLOPS，+95.1%；BF16 42.04 → 82.04，+95.1%。

**条件与边界：** 原文说应该成功，附图给出前后对比；R9000P 2021，150W VBIOS、核心/显存超频 115/545；测试脚本由 AI 提供，未独立验证；两张终端图的 FP16、BF16 均为 FP32 累加路径。未收录仅显示 U 盘目录的第三张附件。

<details>
<summary>查看原图（2 张）</summary>

<table>
<tr>
<td align="center" valign="top" width="50%">
<p align="center">
<b>解锁前</b><br>
<a href="evidence/bilibili-2026-11-02/318487921761-1.png"><img src="evidence/bilibili-2026-11-02/318487921761-1.png" width="420" alt="解锁前"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/9cb9e7d16c254ba50f0ceda5a71defcd352059262.png">评论原图</a> · <a href="#s12">来源与条件</a>
</p>
</td>
<td align="center" valign="top" width="50%">
<p align="center">
<b>解锁后</b><br>
<a href="evidence/bilibili-2026-11-02/318487921761-2.png"><img src="evidence/bilibili-2026-11-02/318487921761-2.png" width="420" alt="解锁后"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/235b4e32699b20ff10b5e0b4df561ce9352059262.png">评论原图</a> · <a href="#s12">来源与条件</a>
</p>
</td>
</tr>
</table>

</details>

<a id="s16"></a>

### S16 · RTX 4060 Laptop · 我想变成light

来源：[我想变成light，2026-10-29](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318513166865&comment_secondary_id=318513166865)（评论 `318513166865`）。

**报告结果：** BF16 31.14 → 53.86（1.73倍）；FP16/FP32累加 31.12 → 54.33（1.75倍）；FP8 62.14 → 117.71（1.89倍）。

**条件与边界：** 截图未写单位；N=16384，PyTorch事件计时；INT8 120.09→118.54，未提升。只据该图记录，不外推全型号。

<details>
<summary>查看原图（1 张）</summary>

<p align="center">
<b>不同精度的前后对比（观众汇总）</b><br>
<a href="evidence/bilibili-2026-11-02/318513166865-1.png"><img src="evidence/bilibili-2026-11-02/318513166865-1.png" width="880" alt="不同精度的前后对比（观众汇总）"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/26b7b00a4b14debdbd087154f334ca7b66820689.png">评论原图</a> · <a href="#s16">来源与条件</a>
</p>

</details>

<a id="s21"></a>

### S21 · RTX 4070 Laptop · 天蓝色的烟雨

来源：[天蓝色的烟雨，2026-10-29](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322273213728&comment_secondary_id=322273213728)（评论 `322273213728`）。

**报告结果：** FP16 N=2048：30.121 → 48.630；4096：33.296 → 64.005；6144：43.249 → 69.190 TFLOPS。

**条件与边界：** 早先失败，后以FAT32硬盘分区解决；按后续量化结果更新。截图注明尚不能独立确认所有差异都由解锁造成。

型号或后续更正的上下文：[322211869408](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322211869408&comment_secondary_id=322211869408) · [322270354624](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322211869408&comment_secondary_id=322270354624)。

<details>
<summary>查看原图（1 张）</summary>

<p align="center">
<b>FP16 GEMM 前后对比（观众汇总）</b><br>
<a href="evidence/bilibili-2026-11-02/322273213728-1.png"><img src="evidence/bilibili-2026-11-02/322273213728-1.png" width="880" alt="FP16 GEMM 前后对比（观众汇总）"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/bac394d7bcbf0837a688cdbd6814acda73701409.png">评论原图</a> · <a href="#s21">来源与条件</a>
</p>

</details>

<a id="s23"></a>

### S23 · RTX 4070 Ti SUPER · 这一管叫做深情

来源：[这一管叫做深情，2026-10-30](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318588984337&comment_secondary_id=318588984337)（评论 `318588984337`）。

**报告结果：** 4096 FP16 57.00 → 150.93；BF16 57.51 → 127.32；8192 FP16 55.50 → 127.28；BF16 54.97 → 128.39 TFLOPS。3参考图生视频自述快至少1分钟。

**条件与边界：** 本人明确同时超频和解锁，不能把约2.65倍跑分全归因解锁。功耗227→291.58W；应用未提供前后总耗时；必须结合本人后续“超频加解锁”的说明阅读截图中的倍数。

型号或后续更正的上下文：[318606154961](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318588984337&comment_secondary_id=318606154961) · [318606748657](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318588984337&comment_secondary_id=318606748657)。

<details>
<summary>查看原图（1 张）</summary>

<p align="center">
<b>系统侧状态与基准汇总；同时进行了超频</b><br>
<a href="evidence/bilibili-2026-11-02/318588984337-1.png"><img src="evidence/bilibili-2026-11-02/318588984337-1.png" width="880" alt="系统侧状态与基准汇总；同时进行了超频"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/17a4d625a968d83de126f8f610d0f88287903194.png">评论原图</a> · <a href="#s23">来源与条件</a>
</p>

</details>

<a id="s13"></a>

### S13 · RTX 4070 台式机 · 重生之我是虾头

来源：[重生之我是虾头，2026-10-29](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322202417520&comment_secondary_id=322202417520)（评论 `322202417520`）。

**报告结果：** FP16 61.99 → 76.32–87.82 TFLOPS，+23%–42%；BF16 62.40 → 77.89–90.71，+25%–45%；出图 35 → 27 秒。

**条件与边界：** 同一人两条成功评论只计一次。图中 150W/200W 条件影响范围；35→27 秒是耗时减少22.9%、吞吐提升29.6%；图中视频耗时是推算，未当实测。

型号或后续更正的上下文：[322202428288](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322202326304&comment_secondary_id=322202428288)。

<details>
<summary>查看原图（1 张）</summary>

<p align="center">
<b>测试汇总照片；底部视频耗时为推算，不作实测引用</b><br>
<a href="evidence/bilibili-2026-11-02/322202417520-1.jpg"><img src="evidence/bilibili-2026-11-02/322202417520-1.jpg" width="880" alt="测试汇总照片；底部视频耗时为推算，不作实测引用"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/8083040d148114726690cbc8249d12de636075104.jpg">评论原图</a> · <a href="#s13">来源与条件</a>
</p>

</details>

<a id="s25"></a>

### S25 · RTX 3090 Ti · 吼吼吼lo

来源：[吼吼吼lo，2026-10-30](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322309028240&comment_secondary_id=322309028240)（评论 `322309028240`）。

**报告结果：** H3 INT8工作流同418.5W：单条123.47 → 108.08秒；采样75.13 → 65.50秒；再提至445.5W单条110.74秒。

**条件与边界：** 截图记录6条批次对比；同功耗总耗时减少12.5%、吞吐提高14.2%。额外提高功耗没有进一步收益；不能只取transition局部约50%作整段加速；仅收录包含原始分阶段数值的表格，未重复收录结论图。

<details>
<summary>查看原图（1 张）</summary>

<p align="center">
<b>H3 工作流分阶段及单条总耗时对比</b><br>
<a href="evidence/bilibili-2026-11-02/322309028240-1.jpg"><img src="evidence/bilibili-2026-11-02/322309028240-1.jpg" width="880" alt="H3 工作流分阶段及单条总耗时对比"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/bd2588d7f6276996ed5f29628acc7e40540051932.jpg">评论原图</a> · <a href="#s25">来源与条件</a>
</p>

</details>



<a id="s46"></a>

### S46 · RTX 3090 20GB OEM（本人标注） · FIcCAT

来源：[FIcCAT，2026-11-02](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322654906480&comment_secondary_id=322654906480)（评论 `322654906480`）。

**报告结果：** FP16 77.36→115.20、BF16 78.11→116.75、TF32 40.46→72.78 TFLOPS。

**条件与边界：** 本人称OEM超龙20GB、核心GA102-250-KD-A1，需修改驱动；型号按原文保留，不改写成其他零售型号。后续睡眠唤醒解除解锁状态。

型号或后续更正的上下文：[322657710656](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322654906480&comment_secondary_id=322657710656) · [322668036848](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322654906480&comment_secondary_id=322668036848)。

<details>
<summary>查看原图（1 张）</summary>

<p align="center">
<b>前后基准对比（观众汇总）</b><br>
<a href="evidence/bilibili-2026-11-02/322654906480-1.png"><img src="evidence/bilibili-2026-11-02/322654906480-1.png" width="880" alt="前后基准对比（观众汇总）"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/9219fa7fbfb0db7b4210a334bf622ec02833817.png">评论原图</a> · <a href="#s46">来源与条件</a>
</p>

</details>

<a id="s45"></a>

### S45 · RTX 4090 · ふわとろ

来源：[ふわとろ，2026-11-01](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322634742704&comment_secondary_id=322634742704)（评论 `322634742704`）。

**报告结果：** TF32 79.4→148.5、81.2→155.4；BF16 156.7→188.9；MOSS BF16自述+15%–20%，H3+10%。

**条件与边界：** 保留功耗墙。文字称名人堂，附AI截图称ROG，品牌不一致；可确认均写4090，不细分品牌。截图算力单位未写明。

<details>
<summary>查看原图（1 张）</summary>

<p align="center">
<b>基准对比（AI 汇总；显卡品牌与文字反馈不一致）</b><br>
<a href="evidence/bilibili-2026-11-02/322634742704-1.png"><img src="evidence/bilibili-2026-11-02/322634742704-1.png" width="880" alt="基准对比（AI 汇总；显卡品牌与文字反馈不一致）"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/8467017d9214a1c3af3722078446eee71525736.png">评论原图</a> · <a href="#s45">来源与条件</a>
</p>

</details>

<a id="s48"></a>

### S48 · RTX 3080 Ti（形态未说明） · 十二悠

来源：[十二悠，2026-11-02](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318960839137&comment_secondary_id=318960839137)（评论 `318960839137`）。

**报告结果：** TF32 38.5→66.7，FP16 73.9→123.2，BF16 74.8→125.8；FP32 26.5→26.4基本不变。

**条件与边界：** 图片为 AI 汇总测试报告；1950MHz+ 且注明无降频，单位未明确。已通过 EXIF 将显示方向逆时针校正 90°，未重编码 JPEG 或改动图中文字；原始与发布文件的校验值均见索引。

<details>
<summary>查看原图（1 张）</summary>

<p align="center">
<b>基准对比照片（AI 汇总；已校正显示方向）</b><br>
<a href="evidence/bilibili-2026-11-02/318960839137-1.jpg"><img src="evidence/bilibili-2026-11-02/318960839137-1.jpg" width="880" alt="基准对比照片（AI 汇总；已校正显示方向）"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/0dbb11a6ec1a9389f103941290aadd1949804397.jpg">评论原图</a> · <a href="#s48">来源与条件</a>
</p>

</details>

## 只有解锁后数值或图片的报告

<a id="s01"></a>

### S01 · RTX 4060 Laptop · TualatinPentium

来源：[TualatinPentium，2026-10-27](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318379087793&comment_secondary_id=318379087793)（评论 `318379087793`）。

**报告结果：** 解锁成功；截图显示 FP16 47.32 TFLOPS。

**条件与边界：** 截图只有解锁后值，没有前后增幅；另有 Quadro RTX 5000 的计划，不能算已完成。

<details>
<summary>查看原图（1 张）</summary>

<p align="center">
<b>仅解锁后测试；没有解锁前对照</b><br>
<a href="evidence/bilibili-2026-11-02/318379087793-1.png"><img src="evidence/bilibili-2026-11-02/318379087793-1.png" width="880" alt="仅解锁后测试；没有解锁前对照"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/05bc00da1c33e312790c25f761c2c55d481086477.png">评论原图</a> · <a href="#s01">来源与条件</a>
</p>

</details>

<a id="s07"></a>

### S07 · RTX 4070 Ti SUPER 16GB · 未然极光

来源：[未然极光，2026-10-28](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318434036593&comment_secondary_id=318434036593)（评论 `318434036593`）。

原评论的测试背景：

> 仅解锁，不进行其他超频，4070tis v1 热点温度也从80提高到100

| 项目 | 原评论或截图提供的内容 | 缺失的对照条件 |
| --- | --- | --- |
| 设备 | 评论写 4070 Ti SUPER v1；截图显示 RTX 4070 Ti SUPER、16.0GB | 未给完整板卡、VBIOS 与驱动版本 |
| 操作 | 本人称仅解锁、不进行其他超频 | 未附前后功耗、频率和同负载日志 |
| FP16 | 解锁后摘要 103.12 TFLOPS | 未提供解锁前算力，无法计算性能增幅 |
| Hot Spot | 本人自述 80°C → 100°C | 缺少同环境、同负载、同散热的记录；温升不作为提速证明 |

本条作为“仅解锁后测试和温度反馈”保留，性能对比可另看同型号的 [S23](#s23)；S23 同时进行了超频，两条报告不能拼成同一组前后对照。

<details>
<summary>查看解锁后原图（1 张）</summary>

<p align="center">
<b>仅解锁后测试；没有解锁前对照</b><br>
<a href="evidence/bilibili-2026-11-02/318434036593-1.png"><img src="evidence/bilibili-2026-11-02/318434036593-1.png" width="880" alt="仅解锁后测试；没有解锁前对照"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/d46fdea662dbf1b23b7bfef38b11e840216963966.png">评论原图</a> · <a href="#s07">来源与条件</a>
</p>

</details>

<a id="s34"></a>

### S34 · RTX 4050 Laptop · snpdz

来源：[snpdz，2026-10-31](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322434523792&comment_secondary_id=322434523792)（评论 `322434523792`）。

**报告结果：** 自述成功，截图FP16 39.15 TFLOPS（仅解锁后）。

**条件与边界：** 6GB；截图另有125W、核心+65、显存+105设置。无解锁前算力，不能计算增幅。

<details>
<summary>查看原图（2 张）</summary>

<p align="center">
<b>仅解锁后 FP16 测试</b><br>
<a href="evidence/bilibili-2026-11-02/322434523792-1.jpg"><img src="evidence/bilibili-2026-11-02/322434523792-1.jpg" width="880" alt="仅解锁后 FP16 测试"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/8a0fb9ed2844a41bab800d5b7b531cc8306417542.jpg">评论原图</a> · <a href="#s34">来源与条件</a>
</p>

<p align="center">
<b>6GB 配置及超频、功耗设置</b><br>
<a href="evidence/bilibili-2026-11-02/322434523792-2.jpg"><img src="evidence/bilibili-2026-11-02/322434523792-2.jpg" width="880" alt="6GB 配置及超频、功耗设置"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/6c6e4b6723d055d7eeef6cc840022f08306417542.jpg">评论原图</a> · <a href="#s34">来源与条件</a>
</p>

</details>

## 仅文字反馈（未收录为对比截图）

<a id="s42"></a>

### S42 · RTX 4060 Laptop · 网友陈胤平

来源：[网友陈胤平，2026-11-01](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322579294896&comment_secondary_id=322579294896)（评论 `322579294896`）。本人称两个生图工作流解锁前分别需“200多秒”和“140多秒”，并说明“不开性能模式，显卡也没超频”。

附件只有解锁后 103.31 秒与 54.79 秒的两个缩略图，没有解锁前截图，也没有模型、工作流或采样参数；无法形成可靠的前后对比，已从图集移除。README 保留为该观众的文字应用反馈，不据此计算精确增幅。原图链接与校验信息在来源索引的 `omitted_screenshots` 中保留。

## 测试条件待补的原始反馈

<a id="s32"></a>

### S32 · RTX 4080 SUPER 32GB 改卡 · Wulai0-0（测试条件待补）

来源：[Wulai0-0，2026-10-30](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318683830785&comment_secondary_id=318683830785)（评论 `318683830785`）。

**设备身份与上下文：** 同一观众在[此前评论](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318436712481&comment_secondary_id=318436712481)明确写“我4080s（32g）改了”；下方图 3 的任务管理器显示 NVIDIA GeForce RTX 4080 SUPER、专用显存 32.0GB。型号、容量据这两项来源记录，不从算力读数反推。

**原评论的应用测试条件：**

> minimaxh3同一工作流同一素材和提示词，int8量化模型，跑0.9 15秒，没解锁之前是730-760之间，解锁后ComfyUI冷启动和热启动各跑了一次

本人[后续更正](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318683830785&comment_secondary_id=318772456913)：“图片是解锁后冷热启动各跑一次的时间，没解锁前是750左右”。右上角任务队列中的两条均为解锁后记录：

| H3 INT8 工作流状态 | 单次耗时 | 证据类型 |
| --- | ---: | --- |
| 解锁前 | 约 730–760 秒，后续概括为约 750 秒 | 仅本人文字自述，没有解锁前截图 |
| 解锁后、ComfyUI 冷启动 | 699.82 秒 | 图 4 任务队列 `xiaoma_00001_` |
| 解锁后、ComfyUI 热启动 | 674.01 秒 | 图 4 任务队列 `xiaoma_00002_` |

**基准衰减与功耗背景：** 图 1 同一终端内先连续显示 180.5 → 85.9 TFLOPS，随后重新调用 `bench.py` 得到 93.4 TFLOPS；图 2 只显示一次 181.9 TFLOPS。缺少脚本、精度、矩阵规模、预热及同步方法，也没有与这些读数逐时对应的功耗、频率和限频标志。图 1 存在明显衰减，不能只取末尾 93.4 与图 2 的 181.9 确认解锁倍数。

[作者当时的回复](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318683830785&comment_secondary_id=318685439137)据监控图判断“你的频率可见撞320W功耗墙严重”，报告者随后表示[“不折腾功耗了”](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=318683830785&comment_secondary_id=318687001185)。这说明当时讨论了功耗限制；现有材料仍不足以独立确认衰减的唯一原因。因此本条保留为待补测试条件的原始反馈，不用于 README 的基准提升数字或兼容性验证等级；同型号较完整的独立反馈见 [S43 原评论](https://www.bilibili.com/video/BV1d6xN66ECM/?comment_on=1&comment_root_id=322585313104&comment_secondary_id=322585313104)。

<details>
<summary>查看设备、终端与工作流原图（4 张；需结合上方上下文）</summary>

<p align="center">
<b>图 3：RTX 4080 SUPER 32GB 身份与运行监控</b><br>
<a href="evidence/bilibili-2026-11-02/318683830785-3.png"><img src="evidence/bilibili-2026-11-02/318683830785-3.png" width="880" alt="图 3：RTX 4080 SUPER 32GB 身份与运行监控"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/dcdc97c73aabd2e16b45555230c54c66285348395.png">评论原图</a> · <a href="#s32">来源与条件</a>
</p>

<table>
<tr>
<td align="center" valign="top" width="50%">
<p align="center">
<b>图 1：解锁前终端（读数衰减，测试方法待补）</b><br>
<a href="evidence/bilibili-2026-11-02/318683830785-1.png"><img src="evidence/bilibili-2026-11-02/318683830785-1.png" width="420" alt="图 1：解锁前终端（读数衰减，测试方法待补）"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/e4e272ebe04bb6b331c0f376ccf0b254285348395.png">评论原图</a> · <a href="#s32">来源与条件</a>
</p>
</td>
<td align="center" valign="top" width="50%">
<p align="center">
<b>图 2：解锁后单次读数（不能据此确认倍数）</b><br>
<a href="evidence/bilibili-2026-11-02/318683830785-2.png"><img src="evidence/bilibili-2026-11-02/318683830785-2.png" width="420" alt="图 2：解锁后单次读数（不能据此确认倍数）"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/351d3f4af50686669e8026a4563eb17a285348395.png">评论原图</a> · <a href="#s32">来源与条件</a>
</p>
</td>
</tr>
</table>

<p align="center">
<b>图 4：工作流右上角的 699.82 秒与 674.01 秒均为解锁后结果</b><br>
<a href="evidence/bilibili-2026-11-02/318683830785-4.png"><img src="evidence/bilibili-2026-11-02/318683830785-4.png" width="880" alt="图 4：工作流右上角的 699.82 秒与 674.01 秒均为解锁后结果"></a><br>
<a href="http://i0.hdslb.com/bfs/new_dyn/943d1edf3f7fa2d9ed5d46d431d60033285348395.png">评论原图</a> · <a href="#s32">来源与条件</a>
</p>

</details>

## 收录口径与后续补充

README 按原文型号和形态分组，共引用 37 位观众的报告；同一人后续更正与补图回到同一条来源。未明确型号的反馈、失败或尚未完成的尝试、他人的型号提问，以及其他超频工具的成功，均不据此增加硬件条目。不从评论数量推算成功率，也不把不同测试的增幅平均。

本次没有采用 Linux 方案来源尚不明确的 RTX 3070 Laptop 图作为本项目复现图，也没有把 RTX 4090 Laptop 的 AI 配置识别图当成解锁结果截图；后者的成功自述在 README 单独标明。RTX 3080 Ti 的条目使用明确成功自述和前后对比照片，不以缺少场景与误差范围的游戏帧率增幅充当验证。

补充记录请使用[硬件实测反馈模板](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/new?template=02-hardware-report.yml)，提供 GPU / 显存 / 平台、工具与驱动版本、系统侧读回、同负载前后数据以及功耗和超频条件。图片请保留可追溯来源，并在上传前去除与测试无关的私人信息。
