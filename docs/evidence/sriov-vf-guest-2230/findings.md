# SR-IOV VF guest 寄存器窗首个完整形态（geforce-sriov；宿主 3090 + gpu-unlock）

2026-11-10。`ssdiag-v0.7.2-vf-guest.log` = 报告者在 Windows guest 里的完整一次运行（逐字），此前 0.7.1 及更早构建在同一环境死于扩展能力链跨页读（静默 AV，[67bb859](https://github.com/physicsdolphin/nv-uefi-unlocker/commit/67bb859) 修复）。**本目录的读数不构成宿主卡的解锁判据**（判读边界见下与 FAQ `#vgpu-vf-guest`）。

## 环境 / Environment

| 侧 | 内容 |
| --- | --- |
| 宿主 | RTX 3090（GA102，`10de:2204`），Linux；**算力解锁 = 上游 [Zagitta gpu-unlock](https://github.com/Zagitta/gpu-unlock) kmod**；SR-IOV vGPU = [geforce-sriov](https://github.com/noon-at-cgn/geforce-sriov)（KVM/libvirt/QEMU） |
| guest | Windows，vGPU guest 驱动 **582.53**（= geforce-sriov README 钉死的 Windows-Ampere guest 版本），`EnableGpuFirmware` 未设（LEGACY 模式） |
| guest 设备 | `10de:2230` SUBSYS `1501:10DE`，profile **RTXA6000-8Q**——profile 身份，非宿主硅身份 |
| 栈实锤 | 根口 `VEN_1B36&DEV_0001` = QEMU；驱动钉值吻合；扩展配置空间全 FF（VF 形态）——三重独立印证 |

## 三个发现

1. **VF 窗指纹：零填充，不是 badf**。`0x0100` 起扩展配置空间全 FF（ReBAR/link 探针自动跳过）；除 `PMC_BOOT_0` 外**全部约 100 项探测寄存器硬线回 `0x00000000`**（FEAT 簇、26 寄存器 PLM survey、fuse/tpc、pcie/gen2 union、GSP/SEC2、WPR2、canary）；无 PM cap，标准链无 PCIe cap。关键语义：**零填充 ≠ badf**——badf 编码器在 PF 的 PRI 上，VF 窗没有，所以读"成功"但语义为空；`closed (non-standard)` 之类的标签是**填充效应，不是门状态**。0.7.3 起对这种形态打印 `NOTE: ... virtualized / SR-IOV VF window` 提示行（跨四 bank 采样：裸机误报需要 `WPR2 HI` 与 `FUSE_VER` 同时为 0 压在非退化 BOOT0 下）。
2. **BOOT0 是固件合成的虚拟板卡身份**：`0x1fa52a02`，`die=(>>20)&0xFFF=0x1fa`——不属于任何已知 die 段（Turing 0x16x / GA100 0x170 / Ada 0x19x / GB 0x1b0·0x210 / GA10x-quirk 0xb7x）。物理 GA10x family 读 `0xb7x000a1` 形态（GA104=`0xb74000a1`、GA106=`0xb76000a1`、GA107=`0xb77000a1` 实测；GA102 quirk 形态应为 `0xb72000a1` 族），VF 值的低半字 `0x52a02` 连形态都不同。open-gpu-kernel-modules 中无 `0x1fa` 出处（grep 只中固件常量噪声）——虚拟芯片 ID 的权威表在闭源 vGPU 管理器侧，离线不可定谳；结论不依赖它。
3. **伪装有两层，guest 身份 = profile 身份**：宿主层 identity hook 让 `nvidia-vgpud` 把 GeForce 当受支持专业卡、放出 RTXA6000-8Q profile；固件层 PF 把该身份投进 guest——`DEV_2230`/`SUBSYS_1501`/VBIOS `0.00.00.00.00`/BOOT0 `0x1fa52a02` 全是这一层的投影。guest 看到的"RTX A6000"**不是宿主那块 3090 硅的身份**。

## 判读边界（协议）

- **guest 侧 ssdiag 的任何判读（UNLOCKED/NOT_UNLOCKED/PLM 标签）不构成宿主卡解锁证据**——本例中即不证明也不证伪宿主的 gpu-unlock 解锁状态。FECS/PLM 属 PF 裸 PRI 状态，guest 里不存在可读投影。
- **有效的 VM 数据点类 = VFIO/DDA 整卡直通**（guest 拿到真 PF MMIO；[#135](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/135) 先例）。mdev/GPU-PV 型 vGPU 连真 MMIO 都没有（BAR 访问被 hypervisor trap 转给宿主 mediator）。
- **探测点在宿主**：Linux 宿主跑上游 gpu-unlock 的验证路径或本仓 `tools/diag-unlock.py`；Windows 裸机跑 ssdiag。
- 按 2026-11-10 维护者裁定：本形态**不入 compat 矩阵**（非宿主判据），以本目录证据 + FAQ 条目落档。

## 修复史

- `< 0.7.1`：在此环境 ranges 后静默 AV（扩展链全 FF 头 → `next=0xFFF` → 非对齐 u32 跨出单页 ECAM 映射）；[67bb859](https://github.com/physicsdolphin/nv-uefi-unlocker/commit/67bb859) 加 `cid==0xffff` 终止 + `next≤0xffc` 有界 + 区别化报告行。
- `0.7.2`：修复作为 rc2 独立附件发布（SHA256SUMS 同步）。
- `0.7.3`：零填充窗 NOTE 行（本 finding #1 的自动化）。

---

# First complete SR-IOV VF guest register-window fingerprint (geforce-sriov; host 3090 + gpu-unlock)

2026-11-10. `ssdiag-v0.7.2-vf-guest.log` is the reporter's verbatim full run in a Windows guest; builds ≤0.7.1 died silently at the extended-cap walk in the same environment (page-edge AV, fixed in [67bb859](https://github.com/physicsdolphin/nv-uefi-unlocker/commit/67bb859)). **Nothing in this directory is evidence of the host card's unlock state** (reading boundary below and FAQ `#vgpu-vf-guest`).

- Host: RTX 3090 (GA102, `10de:2204`) Linux, compute-unlocked via the upstream [Zagitta gpu-unlock](https://github.com/Zagitta/gpu-unlock) kmod; SR-IOV vGPU via [geforce-sriov](https://github.com/noon-at-cgn/geforce-sriov) (KVM/libvirt/QEMU). Guest: Windows, vGPU guest driver 582.53 (= geforce-sriov's pinned Windows-Ampere guest version), LEGACY firmware mode.
- **Zero-fill, not badf**: every probed register (~100: FEAT cluster, 26-register PLM survey, fuse/tpc, pcie/gen2 union, GSP/SEC2, WPR2, canary) reads hard-wired `0x00000000` except BOOT0; extended config space is all-FF from 0x0100; no PM cap; no PCIe cap in the standard chain. The badf encoder lives on the PF's PRI and the VF window has none — reads succeed but carry no state, so `closed (non-standard)` labels are fill artifacts. 0.7.3 prints a `NOTE` line for this shape.
- **BOOT0 is firmware-authored virtual-board identity**: `0x1fa52a02`, die segment 0x1fa matches no known die; physical GA10x reads the `0xb7x000a1` shape and even the low halfword differs (`0x52a02` vs `0x000a1`). The guest's `10de:2230`/"RTX A6000" enumeration is likewise the **profile identity**, not the host 3090 silicon — the disguise is two-layer (host identity hook → profile; PF firmware projects it into the guest).
- **Reading boundary (protocol)**: no guest-side ssdiag verdict is evidence of the host card's unlock state (including the host's gpu-unlock state). The only valid VM datapoint class remains full-device VFIO/DDA passthrough (#135); the measurement point for the host is the host itself (upstream gpu-unlock verification or `tools/diag-unlock.py` on Linux; ssdiag on bare-metal Windows). Maintainer ruling 2026-11-10: this shape stays **out of the compat matrix**; it is archived here plus an FAQ entry.
- Fix history: ≤0.7.1 silent AV at the ECAM walk (all-FF header → next=0xFFF → unaligned u32 crossing the single mapped page); 67bb859 added the guards + distinct report line; 0.7.2 shipped as an rc2 standalone attachment; 0.7.3 adds the flat-zero NOTE line.
