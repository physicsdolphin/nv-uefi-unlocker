#!/usr/bin/env python3
"""NVIDIA unlock state reader / re-applier (host-OS side, Linux).

Reads the FEAT cluster straight off BAR0 via sysfs (no driver needed):
mmap /sys/bus/pci/devices/<bdf>/resource0 and read dwords. Root required.

Families (PMC_BOOT_0 bits [31:20]):
  Turing  (0x160..0x16F)  FEAT_PLM 0x00409650 / SS0 0x00409664 / SS1 0x0040966C / READOUT 0x00409668
  Ampere/Ada (0x170..0x17F, 0x220..0x22F, 0x100)  0x00823804 / 0x0082381C / 0x00823820
  (+ READOUT_1 0x00823818 on the 0x823xxx block; 0x00409668 on Turing, per
  nlzy/turing-unlock - same constant ssdiag carries, issue #42)
  Blackwell (0x1A0..0x1BF, plus 0x210..0x21F for the GB202 RM-dump form)
  shares the 0x823xxx block and additionally prints
  its fuse selectors. Blackwell is READ-ONLY: no SEC2 stage, no GB20x booter,
  so the PLM can only be opened by PL3 firmware and --reapply always refuses
  (issue #168, docs/blackwell-gb20x-status.md). READOUT_1 is decoded with the
  real non-contiguous bit offsets as fields - Blackwell 3 is ~half rate, not
  the Ada 1/8.
  Ampere/Ada also get a per-GPC TPC floorsweep block (STATUS bits = fused-off
  TPCs, CTRL = the writable override, plus the GPC-level disable/defect
  masks and the raw OPT/DEFECT banks) - read-only, issue #169.

ssdiag 0.7.4 surface parity (2026-11-11, GA100 VFI dictionary round): the
kate-land/ga100vfi fuse-controller dictionary probed on every die at the
GA100 offsets (controller core, 17 row0/1 security aliases + FFDO, fused
SM_SPEED_SELECT family, STATUS_OPT_* effective words + PES/ROP_L2/TPC
arrays), the 16-word x 3-address mirror probe, and - Turing only - the
whole dictionary again at -0x7FF000 where the real option bank lives
(open-questions #13) plus the GR-topology 0x409604 anchor. BAR0 rd/wr carry
the out-of-aperture guard (offsets >= 16 MiB read all-ones / drop writes).

Why this exists (issue #6): the EFI unlock's volatile state is only useful if
it survives into the OS. This tool answers, from inside the running system:
  - did the SS override values persist?           (unlock state)
  - is the FEAT PLM gate still open?              (re-apply possible?)
`--reapply` rewrites SS0/SS1 while the door is open - the "separate-lock"
method: devinit may re-derive the rates from fuses (it holds L3), but with
the gate left open by the EFI chain the host can simply write them again.

Usage:
  sudo python3 diag-unlock.py                 # read all NVIDIA dGPUs
  sudo python3 diag-unlock.py --bdf 0000:01:00.0
  sudo python3 diag-unlock.py --reapply       # SS0=0x88888888 SS1=0x8 when open
  python3 diag-unlock.py --inventory          # unelevated hardware snapshot
"""

import argparse
import mmap
import os
import platform
import subprocess
import sys

SS0_UNLOCK = 0x8888_8888
SS1_UNLOCK = 0x8

# offset -> name, per family
TURING = {"plm": 0x0040_9650, "ss0": 0x0040_9664, "readout": 0x0040_9668, "ss1": 0x0040_966C}
GAADA = {
    "plm": 0x0082_3804,
    "ss0": 0x0082_381C,
    "ss1": 0x0082_3820,
    "readout": 0x0082_3818,
}
# Blackwell GB20x uses the same 0x823xxx bank (issue #168, GB206 10de:2d04:
# PLM 0xfffffb8f, SS0 0x06277073, SS1 0x00025005, READOUT_1 0x00c36180).
# Separate table because the readout decode and the rate law differ, and no
# transaction can run on the die (no GB20x SEC2 booter).
BLACKWELL = dict(GAADA)
BLACKWELL_ROWS = [
    ("FUSE_EN_SW_OVERRIDE", 0x0082_0040),
    ("FUSE_SS_PLM", 0x0082_00FC),
    ("SECURE_FEAT_OVR_QUADRO_WR", 0x0082_038C),
    ("FUSE_SS_DP", 0x0082_0224),
    ("FUSE_SS_FFMA", 0x0082_059C),
    ("FUSE_SS_FMLA16", 0x0082_07D4),
    ("FUSE_SS_FMLA32", 0x0082_07D8),
    ("FUSE_SS_IMLA0", 0x0082_07DC),
    ("FUSE_SS_IMLA1", 0x0082_07E0),
    ("FUSE_SS_IMLA2", 0x0082_07E4),
    ("FUSE_SS_IMLA3", 0x0082_07E8),
    ("FUSE_SS_IMLA4", 0x0082_07EC),
]
# FEAT_READOUT_1 is NOT nine contiguous 3-bit fields: offsets 0,3,7,10,13,
# 16,19,22,28 (gaps at bit 6 and bits 27:25). Contiguous decode yields
# out-of-range 6s. Field order imla0 fmla16 fmla32 ffma imla1 imla2 imla3
# imla4 DP.
READOUT1_SHIFT = [0, 3, 7, 10, 13, 16, 19, 22, 28]
# Core/debug block, synced to ssdiag 0.6.4 (CORE_ROWS): the 2026-11-04 sync
# brings the python twin to the ssdiag 0.5/0.6 probe surface - 26-register
# PLM survey was already here; the PCIe/XP3G/OPT/Ada-fuse tables, the
# full core block and the badf classification are new.
# Reset canary (issues #164/#165), twin of ssdiag v0.6.5: the loader writes
# these two PBUS scratch words after its FLR; any reset clears them, and no
# driver pass writes this pair. Read with the FEAT values - canary intact +
# SS gone = a plain write; cleared + SS live = FLR/SBR-class reset; both gone
# = rail-off / full-chip reset.
CANARY_A = 0x0000_1480
CANARY_B = 0x0000_1484
CANARY_MAGIC_A = 0x0164_C0DE
CANARY_MAGIC_B = 0x0165_CAFE


def canary_state(bar):
    a, b = bar.rd(CANARY_A), bar.rd(CANARY_B)
    poison = lambda v: v == 0xFFFF_FFFF or v >> 16 == 0xBADF
    if poison(a) and poison(b):
        return "unreadable"
    if a == CANARY_MAGIC_A and b == CANARY_MAGIC_B:
        return "intact"
    if a == 0 and b == 0:
        return "cleared"
    if (a == CANARY_MAGIC_A) != (b == CANARY_MAGIC_B):
        return "partial"
    return "other"


CORE_ROWS = [
    ("PMC_BOOT_0", 0x0000_0000),
    ("PSTRAPS_SET0", 0x0010_1000),
    ("PSTRAPS_SET1", 0x0010_100C),
    ("FB_SIZE_MIB", 0x0011_83A4),
    ("GFW_PLM", 0x0011_8128),
    ("GFW_SCRATCH", 0x0011_8234),
    ("BSI_SCRATCH15", 0x0011_80FC),
    ("VGA_WORKSPACE", 0x0062_5F04),
    ("WPR2_LO", 0x001F_A824),
    ("WPR2_HI", 0x001F_A828),
    ("MMU_PLM", 0x001F_A7C8),
    ("MMU_LOCK_LO", 0x001F_A82C),
    ("MMU_LOCK_HI", 0x001F_A830),
    ("GSP_CPUCTL", 0x0011_0100),
    ("GSP_EXCI", 0x0011_00D0),
    ("SEC2_CPUCTL", 0x0084_0100),
    ("SEC2_MBOX0", 0x0084_0040),
    ("SEC2_MBOX1", 0x0084_0044),
    ("SEC2_EXCI", 0x0084_00D0),
    ("FUSE_VER_DECODE", 0x0082_41C0),
]
PRIV_ROWS = [("FUSE_OPT_FEATURE_OVR", 0x0082_03F0), ("FUSE_VER", 0x0082_4140)]
ADA_ROWS = [
    ("WPR_PLM", 0x001F_A7C4),
    ("WPR_CFG_PLM", 0x001F_A7CC),
    ("FBPA_PLM", 0x009A_0148),
]
AD1X_ROWS = [
    ("AD1X_CYA", 0x0008_8724),
    ("AD1X_CFG", 0x0008_8DCC),
    ("AD1X_XVE_PLM", 0x0008_8FF4),
    ("AD1X_BAR1_CAP_MIRROR", 0x0008_8BBC),
]
# Ada fuse-controller / TPC-disable block (ssdiag 0.6.4, KohakUnlock #16):
# badf on non-Ada dies is expected and is data.
ADA_FUSE_ROWS = [
    ("FUSE_CTRL", 0x0082_0000),
    ("FUSE_ADDR", 0x0082_0004),
    ("FUSE_REG_50", 0x0082_0050),
    ("FUSE_REG_54", 0x0082_0054),
    ("FUSE_DEBUGCTRL", 0x0082_0060),
    ("DISABLE_FUSE_PROG", 0x0082_007C),
    ("OPT_GPC_DISABLE", 0x0082_0350),
    ("OPT_GPC_DEFECTIVE", 0x0082_05C4),
    ("CTRL_OPT_9C8", 0x0082_09C8),
]
# Per-GPC TPC floorsweep (open question #13). STATUS 0x820c38+4g is the
# effective fused mask (bit n = TPC n of GPC g), CTRL 0x820838+4g the
# writable override - the pair nvgpu drives in tpc_gv11b.c (write, then poll
# STATUS until equal), so the semantics are driver-sourced. The two banks are
# printed raw: 0x820350/0x8205c4 are documented GPC-level masks, the per-GPC
# assignment inside the banks is still probe-derived (KohakUnlock#16).
TPC_CTRL_BASE = 0x0082_0838
TPC_STATUS_BASE = 0x0082_0C38
TPC_BANK_A = 0x0082_03A0
TPC_BANK_B = 0x0082_05C0
OPT_GPC_DISABLE = 0x0082_0350
OPT_GPC_DEFECTIVE = 0x0082_05C4
TPC_MAX_GPC = 8
PLM_SURVEY = [
    0x0082_00D0, 0x0082_00D4, 0x0082_00D8, 0x0082_00DC, 0x0082_00E0, 0x0082_00E4,
    0x0082_00E8, 0x0082_00EC, 0x0082_00F0, 0x0082_00F4, 0x0082_00FC,
    0x0082_3800, 0x0082_3804, 0x0082_3B00,
    0x009A_0008, 0x009A_000C, 0x009A_0148, 0x009A_014C, 0x009A_0168, 0x009A_03F0,
    0x009A_0554, 0x009A_0BFC,
    0x0010_0B10, 0x0010_0B38, 0x0010_0B84, 0x0010_0B9C,
]
AMPADA_SWEEP = [
    0x0082_3800, 0x0082_3808, 0x0082_380C, 0x0082_3810, 0x0082_3814, 0x0082_3824,
    0x0082_3828, 0x0082_382C, 0x0082_3B00,
]
# PCIe link block (ssdiag field round 3 union: cmp50hx patch04 + cmpunlocker
# 0007 + gpu-unlock-bar1 + AD102 L2 research). XVE shadow base 0x88000.
PCIE_LINK_ROWS = [
    ("LNKCAP", 0x0008_8084),
    ("LNKCTL_LNKSTA", 0x0008_8088),
    ("LNKCAP2", 0x0008_80A4),
    ("LNKCTL2_LNKSTA2", 0x0008_80A8),
    ("PRIV_MISC_1", 0x0008_841C),
    ("VSEC_DEVICE", 0x0008_860C),
    ("VSEC_HIERARCHY", 0x0008_8610),
    ("XVE_OVR_LTSSM", 0x0008_872C),
    ("XVE_B_PLM", 0x0008_8AB4),
    ("XVE_D0_PLM", 0x0008_8FE8),
    ("XVE_D4_PLM", 0x0008_8FEC),
    ("XVE_D8_PLM", 0x0008_8FF0),
    ("XVE_C_PLM", 0x0008_8FF8),
    ("LINK_CONFIG_0", 0x0008_C040),
    ("PL_LINK_RATE", 0x0008_C1C0),
    ("CYA_0_DISG2", 0x0008_C2C0),
]
# XP3G fuse-override block: the Gen2 capability layer - XP3G_PLM0 must read
# OPEN before any policy write can land.
XP3G_ROWS = [
    ("XP3G_STATUS0", 0x0008_E100),
    ("XP3G_STATUS3", 0x0008_E10C),
    ("XP3G_OVR0", 0x0008_E110),
    ("XP3G_OVR3", 0x0008_E11C),
    ("XP3G_VAL0", 0x0008_E120),
    ("XP3G_VAL3", 0x0008_E12C),
    ("XP3G_PLM0", 0x0008_E1B0),
    ("XP3G_PLM4", 0x0008_E1B4),
    ("XP3G_PLM8", 0x0008_E1B8),
    ("XP3G_PLMC", 0x0008_E1BC),
]
# Fuse/OPT region union: the Gen2 fuse wall, the GPC-disable shadow, the
# AD102 L2 disable-mask layer (effective = fuse layer | SW mask).
OPT_FUSE_ROWS = [
    ("OPT_GPC_DISABLE", 0x0082_0350),
    ("OPT_MAGIC", 0x0082_0520),
    ("OPT_GEN23", 0x0082_057C),
    ("OPT_GEN3", 0x0082_0580),
    ("L2_CANDIDATE", 0x0082_1200),
    ("L2_SW_MASK", 0x0082_2080),
    ("L2_EFF_MASK", 0x0082_2900),
]
# cmpunlocker PR#55's GA100 per-GPC SM override/status banks. On 20-40
# series expect badf - the existence/nonexistence answer per die, read-only.
GA100_SM_OVR_BASE = 0x0082_0A40
GA100_SM_STATUS_BASE = 0x0082_0C38
GA100_GPCS = 8

# ---------------------------------------------------------------------------
# GA100 VFI dictionary round (ssdiag 0.7.4 parity; docs/reading-notes.md
# "GA100 电压故障注入（VFI）考据" 2026-11-11). kate-land/ga100vfi published
# the GA100 fuse-controller map; 0x8203f0 and the SM_SPEED_SELECT offsets are
# identical across GA100/TU106/GB20x documentations, so the dictionary is a
# Rosetta probe. Bank-base split measured 2026-11-11: GA10x/AD10x/GB20x decode
# option VALUES at 0x820xxx, Turing's real bank sits at -0x7FF000 (0x021xxx,
# open-questions #13) while 0x820xxx on Turing is a control-agreeing surface
# with empty option slots. badf5040 = "PL0-blocked" both for words behind a
# PLM AND for cross-block absence (the filter precedes decode) - existence
# evidence is only the in-block plain/badf interleaving. NV_FUSE_OPT_ prefix
# dropped throughout.
GA100_FUSECTL_ROWS = [
    ("FUSECTRL", 0x0082_0000),
    ("FUSEADDR", 0x0082_0004),
    ("EN_SW_OVERRIDE", 0x0082_0040),
    ("FUSEDEBUG", 0x0082_0060),
    ("CRC_STATUS", 0x0082_0070),
    ("ENABLE_FUSE_PROG_STAT", 0x0082_0078),
    ("DISABLE_FUSE_PROG_STAT", 0x0082_007C),
    ("DISABLE_SW_OVR_STAT", 0x0082_0084),
    ("FUSEWDATA_PLM", 0x0082_00F4),
    ("FUSECTRL_PLM", 0x0082_00F8),
    ("FUSEOPT_PLM", 0x0082_00FC),
    ("RING_GLOBAL_CTL", 0x0012_0060),
    ("RING_RESET_PLM", 0x0012_00A0),
]
GA100_SEC_ALIAS_ROWS = [
    ("DEBUG_FEAT_SW_OVR_EN", 0x0082_0148),
    ("IST_DEBUG_FA", 0x0082_0158),
    ("PRIV_SEC_EN", 0x0082_0434),
    ("SECURE_PRI_SRC_ISO_EN", 0x0082_0410),
    ("PRIVKEY_ACC_DIS", 0x0082_0324),
    ("FEATURE_FUSES_OVR_DIS", 0x0082_03F0),
    ("SFK_ACC_DIS", 0x0082_05A8),
    ("SECURE_PMU_DEBUG_DIS", 0x0082_0640),
    ("SECURE_SECENG_DEBUG_DIS", 0x0082_063C),
    ("SECURE_NVDEC_DEBUG_DIS", 0x0082_0644),
    ("SECURE_MINION_DEBUG_DIS", 0x0082_06FC),
    ("SECURE_GSP_DEBUG_DIS", 0x0082_074C),
    ("HDCP_RAM_ACCESS_DIS", 0x0082_07F4),
    ("DEBUG_SW_OVR_EN", 0x0082_0E20),
    ("XV2CLK_RETIME_DIS", 0x0082_1064),
    ("SECURE_FUSE_RESENSE_DIS", 0x0082_1058),
    ("SECURED_BURN_FEAT_EN", 0x0082_1108),
    ("SECURE_XUSB_DEBUG_DIS", 0x0082_0B98),
    ("SECURE_FUSE_WDATA_WR_SEC", 0x0082_048C),
    ("DEVID_SW_OVR_DIS", 0x0082_0584),
]
GA100_SPEED_SEL_ROWS = [
    ("SM_SPEED_SELECT_DP", 0x0082_0224),
    ("SM_SPEED_SELECT_FFMA", 0x0082_059C),
    ("SM_SPEED_SELECT_FMLA16", 0x0082_07D4),
    ("SM_SPEED_SELECT_FMLA32", 0x0082_07D8),
    ("SM_SPEED_SELECT_IMLA0", 0x0082_07DC),
    ("SM_SPEED_SELECT_IMLA1", 0x0082_07E0),
    ("SM_SPEED_SELECT_IMLA2", 0x0082_07E4),
    ("SM_SPEED_SELECT_IMLA3", 0x0082_07E8),
    ("SM_SPEED_SELECT_IMLA4", 0x0082_07EC),
]
GA100_STATUS_OPT_ROWS = [
    ("STAT_HALF_FBPA", 0x0082_0C00),
    ("STAT_DISPLAY", 0x0082_0C04),
    ("STAT_DISP_HEAD", 0x0082_0C08),
    ("STAT_NVJPG", 0x0082_0C0C),
    ("STAT_HALF_LTC", 0x0082_0C10),
    ("STAT_FBIO", 0x0082_0C14),
    ("STAT_FBPA", 0x0082_0C18),
    ("STAT_GPC", 0x0082_0C1C),
    ("STAT_PERLINK", 0x0082_0C20),
    ("STAT_NVDEC", 0x0082_0C24),
    ("STAT_SEC", 0x0082_0C28),
    ("STAT_PCIE_LANE", 0x0082_0C2C),
    ("STAT_SPARE_FS", 0x0082_0C30),
    ("STAT_FB_CONFIG", 0x0082_0C34),
    ("STAT_FBP", 0x0082_0D38),
    ("STAT_NVENC", 0x0082_0D68),
    ("STAT_GC6_ISLAND", 0x0082_0D6C),
    ("STAT_NVLINK", 0x0082_0DB8),
    ("STAT_SYS_PIPE", 0x0082_0E30),
    ("STAT_OFA", 0x0082_0E34),
]
GA100_PES_STATUS_BASE = 0x0082_0DBC
GA100_ROP_L2_STATUS_BASE = 0x0082_0D70
GA100_TPC_STATUS_BASE = 0x0082_0C38

# Mirror probe: 16 representative words x {0x820xxx, -0x800000, -0x7FF000}.
# mirror == primary -> bit-23 alias; mirror badf while primary is plain ->
# 0x820xxx exclusive; different plain values in two columns -> two live banks.
# On GA10x the -0x7FF000 column is the control (expected badf).
# Coverage-completion round (2026-11-12, same pinned surface): the repo's
# option map holds 672 words; these add the segmentation carriers - the
# fuse-side _DISABLE/_DEFECTIVE pairs, the _CP shadow family (volatile
# floorsweep-reversal candidates, GA100 170hx-rir-tool precedent), TPC
# per-GPC disable/reconfig/defective words and PCIE devid/magic. Excluded:
# ~550 GA100-specific blocks (XUSB/AZA/ADCS/clocks/Tegra legacy/FPF spares).
# HALF_FBPA/HALF_LTC are _ENABLE polarity (1=on).
GA100_OPT_DIS_ROWS = [
    ("FBIO_DISABLE", 0x0082_036C),
    ("FBIO_DEFECTIVE", 0x0082_05D4),
    ("FBPA_DISABLE", 0x0082_0368),
    ("FBPA_DEFECTIVE", 0x0082_05D0),
    ("FBP_DISABLE", 0x0082_0364),
    ("FBP_DEFECTIVE", 0x0082_05CC),
    ("PES_DISABLE", 0x0082_0690),
    ("PES_DEFECTIVE", 0x0082_0698),
    ("ROP_L2_DISABLE", 0x0082_02C4),
    ("ROP_L2_DEFECTIVE", 0x0082_05E8),
    ("NVDEC_DISABLE", 0x0082_0378),
    ("NVDEC_DEFECTIVE", 0x0082_05E0),
    ("NVENC_DISABLE", 0x0082_0414),
    ("NVENC_DEFECTIVE", 0x0082_05DC),
    ("NVJPG_DISABLE", 0x0082_0370),
    ("NVJPG_DEFECTIVE", 0x0082_05BC),
    ("OFA_DISABLE", 0x0082_02D8),
    ("OFA_DEFECTIVE", 0x0082_02F0),
    ("NVLINK_DISABLE", 0x0082_0684),
    ("NVLINK_DEFECTIVE", 0x0082_068C),
    ("SYS_PIPE_DISABLE", 0x0082_03A4),
    ("SYS_PIPE_DEFECTIVE", 0x0082_03AC),
    ("DISPLAY_DISABLE", 0x0082_037C),
    ("DISPLAY_DEFECTIVE", 0x0082_05D8),
    ("DISP_HEAD_DISABLE", 0x0082_0418),
    ("PCIE_LANE_DISABLE", 0x0082_0394),
    ("PCIE_LANE_DEFECTIVE", 0x0082_05E4),
    ("PERLINK_DISABLE", 0x0082_07F0),
    ("SEC_DISABLE", 0x0082_04FC),
    ("GC6_ISLAND_DISABLE", 0x0082_0568),
    ("HALF_FBPA_ENABLE", 0x0082_049C),
    ("HALF_LTC_ENABLE", 0x0082_060C),
    ("FB_CONFIG", 0x0082_0328),
    ("GEN3_SPEED_DISABLE", 0x0082_0250),
]
GA100_DEVID_ROWS = [
    ("PCIE_DEVIDA", 0x0082_04D8),
    ("PCIE_DEVIDB", 0x0082_056C),
    ("PCIE_MAGIC_A", 0x0082_0558),
    ("PCIE_MAGIC_B", 0x0082_0518),
    ("PCIE_MAGIC_C", 0x0082_051C),
    ("PCIE_MAGIC_D", 0x0082_0520),
]
# DISABLE contiguous 0x820760+4g, RECONFIG contiguous 0x820788+4g.
GA100_TPC_DISABLE_BASE = 0x0082_0760
GA100_TPC_RECONFIG_BASE = 0x0082_0788
GA100_TPC_DEF_ROWS = [
    ("TPC_GPC0_DEFECTIVE", 0x0082_05C8),
    ("TPC_GPC1_DEFECTIVE", 0x0082_0508),
    ("TPC_GPC2_DEFECTIVE", 0x0082_050C),
    ("TPC_GPC3_DEFECTIVE", 0x0082_0514),
    ("TPC_GPC4_DEFECTIVE", 0x0082_0564),
    ("TPC_GPC5_DEFECTIVE", 0x0082_05AC),
    ("TPC_GPC6_DEFECTIVE", 0x0082_0B84),
    ("TPC_GPC7_DEFECTIVE", 0x0082_0B80),
]
# The _CP shadow family (24 on GA100): soft-disable copies behind
# EN_SW_OVERRIDE - the volatile floorsweep-reversal candidates (OQ#13).
GA100_CP_ROWS = [
    ("FBP_DISABLE_CP", 0x0082_0234),
    ("GPC_DISABLE_CP", 0x0082_022C),
    ("TPC_GPC0_DISABLE_CP", 0x0082_0230),
    ("TPC_GPC1_DISABLE_CP", 0x0082_0504),
    ("TPC_GPC2_DISABLE_CP", 0x0082_05F4),
    ("TPC_GPC3_DISABLE_CP", 0x0082_05A0),
    ("TPC_GPC4_DISABLE_CP", 0x0082_04C4),
    ("TPC_GPC5_DISABLE_CP", 0x0082_04CC),
    ("TPC_GPC6_DISABLE_CP", 0x0082_0B8C),
    ("TPC_GPC7_DISABLE_CP", 0x0082_0B88),
    ("FBIO_DISABLE_CP", 0x0082_0460),
    ("FBPA_DISABLE_CP", 0x0082_0454),
    ("NVDEC_DISABLE_CP", 0x0082_0458),
    ("NVENC_DISABLE_CP", 0x0082_045C),
    ("NVJPG_DISABLE_CP", 0x0082_0488),
    ("ROP_L2_DISABLE_CP", 0x0082_02E4),
    ("OFA_DISABLE_CP", 0x0082_02EC),
    ("PCIE_LANE_DISABLE_CP", 0x0082_0248),
    ("DISPLAY_DISABLE_CP", 0x0082_024C),
    ("DISP_HEAD_DISABLE_CP", 0x0082_0450),
    ("SYS_PIPE_DISABLE_CP", 0x0082_03A8),
    ("PES_DISABLE_CP", 0x0082_0694),
    ("NVLINK_DISABLE_CP", 0x0082_0688),
    ("FB_CONFIG_CP_ANCHOR", 0x0082_0328),
]
GA100_MIRROR_WORDS = [
    ("FUSECTRL", 0x0082_0000),
    ("EN_SW_OVERRIDE", 0x0082_0040),
    ("FUSEDEBUG", 0x0082_0060),
    ("CRC_STATUS", 0x0082_0070),
    ("FUSEWDATA_PLM", 0x0082_00F4),
    ("FUSECTRL_PLM", 0x0082_00F8),
    ("FUSEOPT_PLM", 0x0082_00FC),
    ("DEBUG_FEAT_SW_OVR_EN", 0x0082_0148),
    ("SM_SPEED_SELECT_DP", 0x0082_0224),
    ("PRIVKEY_ACC_DIS", 0x0082_0324),
    ("OPT_GPC_DISABLE", 0x0082_0350),
    ("WDATA_WR_SECURE", 0x0082_048C),
    ("OPT_GPC_DEFECTIVE", 0x0082_05C4),
    ("STAT_GPC", 0x0082_0C1C),
    ("STAT_TPC_GPC0", 0x0082_0C38),
    ("SBFE", 0x0082_1108),
]


def find_gpus():
    out = []
    root = "/sys/bus/pci/devices"
    for bdf in sorted(os.listdir(root)):
        p = os.path.join(root, bdf)
        try:
            vendor = open(os.path.join(p, "vendor")).read().strip()
            cls = open(os.path.join(p, "class")).read().strip()
        except OSError:
            continue
        if vendor != "0x10de" or not cls.startswith("0x03"):
            continue
        # function 0 only (the GPU, not its audio/USB-C companions)
        if ".0" not in bdf and "0000:" in bdf and not bdf[-2] == ".":
            continue
        out.append((bdf, p))
    return out


# ---------------------------------------------------------------- inventory
# The unelevated hardware snapshot twin of ssdiag 0.7.0 --inventory (same
# KEY: line prefixes, so tools/collect_compat.py ingests both identically):
# sysfs + DMI + /proc + nvidia-smi reads only, never root, never a write.
# Serial-number DMI files are deliberately not opened.

INVENTORY_FAMILIES = [
    ((0x2600, 0x28FF), "ad10x (Ada)"),
    ((0x2400, 0x25FF), "ga10x (Ampere GA103/104/106/107)"),
    ((0x2200, 0x223F), "ga102 (Ampere)"),
    ((0x1E02, 0x1FFF), "tu10x (Turing)"),
    ((0x2182, 0x21D1), "tu11x (Turing TU116/117)"),
]


def family_verdict_by_devid(devid):
    for (lo, hi), name in INVENTORY_FAMILIES:
        if lo <= devid <= hi:
            return name, True
    return "outside the tu10x/ga10x/ad10x families", False


def _read(path):
    try:
        with open(path) as f:
            return f.read().strip()
    except OSError:
        return ""


def _nvidia_smi(query):
    """Best-effort nvidia-smi CSV query -> list of row lists (no raise)."""
    try:
        out = subprocess.run(
            ["nvidia-smi", f"--query-gpu={query}", "--format=csv,noheader"],
            capture_output=True, text=True, timeout=30)
        if out.returncode != 0:
            return []
        return [r.split(",") for r in out.stdout.strip().splitlines() if r.strip()]
    except (OSError, subprocess.TimeoutExpired):
        return []


def _gsp_runtime():
    """aggregate GSP state, fail-closed semantics like make-usbroot.sh gsp_state"""
    try:
        q = subprocess.run(["nvidia-smi", "-q"], capture_output=True,
                           text=True, timeout=30)
        if q.returncode != 0:
            return "unknown"
    except (OSError, subprocess.TimeoutExpired):
        return "unknown"
    hits = [ln for ln in q.stdout.splitlines() if "GSP Firmware Version" in ln]
    if not hits:
        return "unknown"
    for ln in hits:
        val = ln.split(":", 1)[1].strip() if ":" in ln else ""
        if val.lower() in ("n/a", "na", ""):
            return "inactive"
    return "active"


def inventory_block():
    """The inventory block itself: platform lines, per-GPU rows, advisories.

    Printed by --inventory alone AND at the top of every full (root) run -
    ssdiag 0.7.0 parity, so a register dump carries its own platform
    context instead of the reporter having to describe it."""
    print("===== diag-unlock inventory (read-only; no root, no BAR0) =====")
    pretty = ""
    try:
        with open("/etc/os-release") as f:
            for ln in f:
                if ln.startswith("PRETTY_NAME="):
                    pretty = ln.split("=", 1)[1].strip().strip('"')
                    break
    except OSError:
        pass
    print(f"OS:      {pretty or platform.system()} (kernel {platform.release()}, {platform.machine()})")
    cpu = ""
    try:
        with open("/proc/cpuinfo") as f:
            for ln in f:
                if ln.startswith("model name"):
                    cpu = ln.split(":", 1)[1].strip()
                    break
    except OSError:
        pass
    print(f"CPU:     {cpu or 'unknown'}")
    dmi = "/sys/class/dmi/id"
    board = f"{_read(os.path.join(dmi, 'board_vendor'))} {_read(os.path.join(dmi, 'board_name'))}".strip()
    system = f"{_read(os.path.join(dmi, 'sys_vendor'))} {_read(os.path.join(dmi, 'product_name'))}".strip()
    if board:
        print(f"BOARD:   {board}" + (f" (system: {system})" if system else ""))
    else:
        print(f"BOARD:   {system or 'unknown'}")
    bios = _read(os.path.join(dmi, "bios_version"))
    bios_date = _read(os.path.join(dmi, "bios_date"))
    print(f"BIOS:    {bios or 'unknown'}" + (f" ({bios_date})" if bios_date else ""))
    chassis_word, ctype = "unknown", 0
    ct = _read(os.path.join(dmi, "chassis_type"))
    if ct.isdigit():
        t = int(ct)
        chassis_word = ("laptop/portable" if t in (8, 9, 10, 11, 14, 30, 31, 32)
                        else "desktop/tower" if t in (3, 4, 5, 6, 7, 13, 15, 16, 17, 23, 24, 28, 35)
                        else "other/unknown")
        ctype = t
    print(f"CHASSIS: {chassis_word} (DMI chassis type {ctype})")

    sb = "unknown"
    try:
        r = subprocess.run(["mokutil", "--sb-state"], capture_output=True,
                           text=True, timeout=15)
        if r.returncode == 0:
            sb = "ON" if "enabled" in r.stdout.lower() else "OFF"
    except (OSError, subprocess.TimeoutExpired):
        pass
    print(f"PLATFORM: Secure Boot = {sb}")

    nvrm = _read("/proc/driver/nvidia/version")
    if nvrm:
        print(f"NVIDIA driver: {nvrm.splitlines()[0]}")
    # per-GPU rows keyed by nvidia-smi's PCI bus id suffix
    smi = {}
    for row in _nvidia_smi("pci.bus_id,name,vbios_version,driver_version"):
        if len(row) != 4:
            continue
        bus, name, vbios, drv = (c.strip() for c in row)
        smi[bus.split(":")[-2] + ":" + bus.split(":")[-1]] = (name, vbios, drv)
    gsp = _gsp_runtime()
    if gsp != "unknown":
        print(f"PLATFORM: GSP firmware = {gsp} (nvidia-smi)")

    devs = find_gpus()
    advisories = []
    if sb == "ON":
        advisories.append("Secure Boot ON - the EFI stick needs the FFS/driver form or a db/MOK-signed EFI (FAQ: Secure Boot)")
    for i, (bdf, p) in enumerate(devs, 1):
        devid_s = _read(os.path.join(p, "device"))
        sub_dev = _read(os.path.join(p, "subsystem_device"))
        sub_ven = _read(os.path.join(p, "subsystem_vendor"))
        devid = int(devid_s, 16) if devid_s.startswith("0x") else 0
        name, vbios, drv = smi.get(bdf, ("unknown", "", ""))
        print(f"GPU {i}: {name}")
        pci = f"10de:{devid:04x}"
        if sub_dev.startswith("0x") and sub_ven.startswith("0x"):
            # Windows SUBSYS order = subsystem device, then subsystem vendor
            pci += f" SUBSYS {sub_dev[2:6].upper()}{sub_ven[2:6].upper()}"
        print(f"GPU {i} PCI: {pci} BDF {bdf.split(':', 1)[1] if ':' in bdf else bdf}")
        fam, in_scope = family_verdict_by_devid(devid)
        print(f"GPU {i} family: {fam} - {'IN-SCOPE (unlock transaction family)' if in_scope else 'OUT-OF-SCOPE'}")
        if vbios and vbios.lower() != "[n/a]":
            print(f"GPU {i} VBIOS: {vbios} (nvidia-smi)")
        else:
            print(f"GPU {i} VBIOS: unknown (nvidia-smi unavailable)")
        if drv:
            print(f"GPU {i} driver: {drv} (nvidia-smi)")
        if in_scope and fam.startswith("tu") and gsp == "inactive":
            advisories.append("Turing adapter with GSP firmware NOT active (nvidia-smi reports N/A) - unlocking it WILL TDR; get GSP mode running first (FAQ: Turing)")
    if chassis_word.startswith("laptop"):
        advisories.append("laptop chassis - the volatile unlock can drop on sleep/RTD3/MUX events; note the display mode (hybrid vs dGPU-direct) when reporting (FAQ: losing the unlock)")
    dpm = ""
    try:
        with open("/proc/driver/nvidia/params") as f:
            for ln in f:
                if ln.startswith("DynamicPowerManagement:"):
                    dpm = ln.split(":", 1)[1].strip()
                    break
    except OSError:
        pass
    if dpm.lower() in ("0x02", "0x2", "2", "0x03", "0x3", "3"):
        advisories.append(f"NVIDIA runtime D3 enabled (DynamicPowerManagement={dpm}) - D3cold parks erase the volatile unlock after handoff (issue #9); fix: NVreg_DynamicPowerManagement=0x00 in modprobe.d, then reboot; one-shot fixer: sudo bash tools/disable-rtd3.sh (repo)")
    print("===== end inventory =====")
    for a in advisories:
        print(f"ADVISORY: {a}")
    if not advisories:
        print("ADVISORY: none of the known pre-flight cautions apply")


def inventory():
    inventory_block()
    print("inventory complete - read-only, nothing was written")
    print("posting this block? review it and redact anything you consider personal first")
    return 0


class Bar0:
    def __init__(self, devpath):
        with open(os.path.join(devpath, "resource")) as f:
            start = int(f.readline().split()[0], 16)
        if start == 0:
            raise RuntimeError("BAR0 not assigned (device powered down?)")
        self.start = start
        f = open(os.path.join(devpath, "resource0"), "r+b", 0)
        self.map = mmap.mmap(f.fileno(), 16 * 1024 * 1024)

    def rd(self, off):
        # Host-side last-resort net (ssdiag 0.7.4 parity, 2026-11-11 lesson):
        # every table offset lives inside the 16 MiB aperture; anything past
        # it is an arithmetic bug upstream (a shifted-base subtraction that
        # underflowed froze nothing but killed the Windows exe with an access
        # violation on unmapped user VA). Answer all-ones instead of reading.
        if off >= 0x0100_0000:
            return 0xFFFF_FFFF
        return int.from_bytes(self.map[off : off + 4], "little")

    def wr(self, off, val):
        if off < 0x0100_0000:
            self.map[off : off + 4] = int.to_bytes(val, 4, "little")


def family_of(boot0):
    # die = bits [31:20]; nvchip series per nv_arch.h GPU_ARCHITECTURE table
    # (MAXWELL=0x110 PASCAL=0x130 VOLTA=0x140 TURING=0x160 AMPERE=0x170
    #  HOPPER=0x180 ADA=0x190 GB1XX=0x1A0 GB2XX=0x1B0 RUBIN=0x1C0)
    # field anchors: TU106=0x166 AD106=0x196 AD107=0x197 GA100=0x170 GA106=0xb76
    die = (boot0 >> 20) & 0xFFF
    if 0x110 <= die <= 0x11F:
        return "maxwell", None
    if 0x120 <= die <= 0x12F:
        return "maxwell2", None
    if 0x130 <= die <= 0x13F:  # GP104=0x134 GP106=0x136
        return "pascal", None
    if 0x140 <= die <= 0x14F:  # GV100=0x140
        return "volta", None
    if 0x150 <= die <= 0x15F:
        return "volta2", None
    if 0x160 <= die <= 0x16F:
        return "turing", TURING
    if 0x170 <= die <= 0x17F:  # GA100 reads 0x170000a1 plainly
        return "ampere", GAADA
    if 0x180 <= die <= 0x18F:  # GH100=0x180
        return "hopper", None
    if 0x190 <= die <= 0x19F:  # Ada is NV19x, NOT 0x22x (device ids)
        return "ada", GAADA
    if 0x1A0 <= die <= 0x1AF:  # GB100=0x1A0, GB102=0x1A2
        return "blackwell-gb1", BLACKWELL
    if 0x1B0 <= die <= 0x1BF:  # GB202/GB203M/GB206 class (50-series)
        return "blackwell-gb2", BLACKWELL
    if 0x210 <= die <= 0x21F:  # GB202 also reports 0x21120000 in an RM debug dump
        return "blackwell-gb2", BLACKWELL
    if 0x1C0 <= die <= 0x1CF:
        return "rubin", None
    if 0xB70 <= die <= 0xB7F:  # GA10x quirk: 0xb7xx00a1 (GA102=b74, GA106=b76)
        return "ampere", GAADA
    return None, None


def decode_plm(v):
    if v == 0xFFFF_FFFF:
        return "OPEN"
    if v == 0xFFFF_FF8F:
        return "closed (stock)"
    if v == 0xFFFF_FB8F:
        return "closed (PL3-only writes)"
    if v == 0xFFFF_FFEF:
        return "closed (relock+drift)"
    return "closed (non-standard)"


def decode_readout1(v):
    """Fields, not rates. Blackwell 3 measures ~half rate, not the Ada 1/8."""
    return " ".join(str((v >> s) & 7) for s in READOUT1_SHIFT)


def decode_tpc_bits(v):
    """Bit names for a per-GPC TPC mask (bit n = TPC n)."""
    return " ".join(f"TPC{b}" for b in range(8) if v & (1 << b))


def dump_tpc(bar):
    """Read-only per-GPC TPC floorsweep survey (open question #13)."""
    print("  --- tpc (per-GPC floorsweep; STATUS bit n = TPC n of that GPC) ---")
    off_total = 0
    off_mask = 0
    statuses = []
    for gpc in range(TPC_MAX_GPC):
        ctrl = bar.rd(TPC_CTRL_BASE + 4 * gpc)
        status = bar.rd(TPC_STATUS_BASE + 4 * gpc)
        statuses.append(status)
        b = badf_bucket(status)
        if b:
            note = f" [{b}]"
            n = 0
        else:
            note = f" [{decode_tpc_bits(status)}]"
            n = bin(status).count("1")
        off_total += n
        if n:
            off_mask |= 1 << gpc
        print(f"  GPC{gpc} CTRL={ctrl:#010x} STATUS={status:#010x}{note}")
    dis = bar.rd(OPT_GPC_DISABLE)
    defv = bar.rd(OPT_GPC_DEFECTIVE)
    print(f"  OPT_GPC_DISABLE={dis:#010x} (bit per GPC)  OPT_GPC_DEFECTIVE={defv:#010x} (bit per GPC)")
    a = " ".join(f"{bar.rd(TPC_BANK_A + 4 * g):#010x}" for g in range(TPC_MAX_GPC))
    b = " ".join(f"{bar.rd(TPC_BANK_B + 4 * g):#010x}" for g in range(TPC_MAX_GPC))
    print(f"  bank A {TPC_BANK_A:#010x}..: {a}")
    print(f"  bank B {TPC_BANK_B:#010x}.. (+4 = OPT_GPC_DEFECTIVE above): {b}")
    if off_total == 0:
        print("  verdict: no TPC fused off in the STATUS masks (full for this GPC count)")
    elif defv & off_mask == 0:
        print(f"  verdict: {off_total} TPC(s) fused off in {bin(off_mask).count('1')} GPC(s),"
              " no defect bit on them - override candidates (write path out of scope)")
    else:
        print(f"  verdict: {off_total} TPC(s) fused off; affected GPCs also carry defect bits"
              f" (mask {defv & off_mask:#x})")
    # Reference map from the field (issue #169): byte-identical STATUS vectors
    # seen on independent machines, so a future report self-classifies.
    if statuses == [0x30, 0x30, 0x30, 0x30, 0x31, 0x30, 0x3F, 0x00]:
        print("  ref: STATUS vector matches the standard RTX 3070 (2484) binning map"
              " - byte-identical on two independent samples (#169 aston89, #54 GWASDW123);"
              " 19 TPCs off, 23 active = the 46-SM SKU")


def badf_bucket(v):
    """ssdiag 0.6.x classification: 0xBADF5040 privilege violation,
    0xBADF1100 undecoded, 0xBADF20xx floorswept, all-ones no decode."""
    if v == 0xFFFF_FFFF:
        return "NO_DECODE"
    if (v >> 16) != 0xBADF:
        return None
    lo = v & 0xFFFF
    if v == 0xBADF_5040:
        return "PRIV_VIOLATION"
    if lo == 0x1100:
        return "BADF_UNDECODED"
    if (lo >> 8) == 0x20:
        return "BADF_FLOORSWEPT"
    return "BADF"


def decode_ss(v, width):
    # 4-bit fields: bit3 = override-enable, bits[2:0] = rate (0 full 1 half 2 quarter 3 eighth)
    names = {0: "full", 1: "1/2", 2: "1/4", 3: "1/8"}
    fields = []
    for i in range(width):
        nib = (v >> (4 * i)) & 0xF
        if nib == 0:
            fields.append(".")
        elif nib & 0x8:
            fields.append(names.get(nib & 7, "?") + "*")
        else:
            fields.append("r%d" % (nib & 7))
    return " ".join(fields)


def rrow(bar, rows, plm_suffix="PLM"):
    for name, off in rows:
        v = bar.rd(off)
        note = f" ({decode_plm(v)})" if name.endswith(plm_suffix) else ""
        b = badf_bucket(v)
        tag = f" [{b}]" if b else ""
        print(f"  {name:<22} = {v:#010x}{note}{tag}")


def read_rebar(devpath):
    """ReBAR ECAM probe via sysfs config (ssdiag 0.5.x parity, loader
    pci.rs decode semantics): walk the extended-capability chain for cap
    id 0x15, then decode ctrl/cap - size_idx n = 2^n MiB, supported size
    = 2^(bit-4) MiB for set bits >= 4 in the capability dword."""
    try:
        with open(os.path.join(devpath, "config"), "rb") as f:
            cfg = f.read(4096)
    except OSError as e:
        print(f"  ReBAR: config space unreadable ({e})")
        return
    if len(cfg) < 0x100:
        return
    off = 0x100
    seen = 0
    while 0x100 <= off <= len(cfg) - 4 and seen < 48:
        cap_id = int.from_bytes(cfg[off : off + 2], "little")
        nxt = int.from_bytes(cfg[off + 2 : off + 4], "little") >> 4
        if cap_id == 0x15:  # PCI_EXT_CAP_ID_REBAR
            for bar_idx in range(8):
                ctrl = int.from_bytes(cfg[off + 4 + bar_idx * 4 : off + 8 + bar_idx * 4], "little") & 0x3FFF
                capd = int.from_bytes(cfg[off + 4 + 32 + bar_idx * 4 : off + 8 + 32 + bar_idx * 4], "little")
                idx = ctrl & 0xF
                sizes = [f"{1 << (b - 4)}G" if b >= 4 else f"{1 << (b + 10)}M"
                         for b in range(32) if capd & (1 << b) and b >= 4]
                if idx or capd:
                    cur = f"{1 << idx}MiB" if idx else "disabled"
                    print(f"  ReBAR BAR{bar_idx}: supported [{'/'.join(sizes) or '-'}]"
                          f" current {cur} (size_idx {idx}, ctrl {ctrl:#06x}, cap {capd:#09x})")
            return
        if nxt == 0:
            break
        off = nxt
        seen += 1
    print("  ReBAR: no resizable-BAR extended capability (0x15) in the ECAM chain")


def report(bar, tag="", devpath=None):
    boot0 = bar.rd(0x0)
    fam, regs = family_of(boot0)
    if fam is None:
        if boot0 in (0, 0xFFFFFFFF):
            state = "all zeros" if boot0 == 0 else "all ones"
            print(f"  PMC_BOOT_0={boot0:#010x} [{state}] - device likely D3 / MEM decode off; nothing to diagnose")
        else:
            print(f"  PMC_BOOT_0={boot0:#010x} - NVIDIA-class unknown die; diagnosis-only pass (FEAT addresses unmapped)")
        return None
    if regs is None:
        print(f"  VERDICT: family recognized - FEAT addresses not mapped for this family (diagnosis only)")
        return None
    plm = bar.rd(regs["plm"])
    ss0 = bar.rd(regs["ss0"])
    ss1 = bar.rd(regs["ss1"])
    print(f"  PMC_BOOT_0={boot0:#010x} family={fam} {tag}")
    blackwell = fam.startswith("blackwell")
    print(f"  FEAT_PLM  = {plm:#010x} ({decode_plm(plm)})")
    print(f"  FEAT_SS0  = {ss0:#010x} [{decode_ss(ss0, 8)}]")
    print(f"  FEAT_SS1  = {ss1:#010x} [{decode_ss(ss1, 1)}]")
    if "readout" in regs:
        ro = bar.rd(regs["readout"])
        note = "PLM-blocked" if (ro & 0xFFFF0000) == 0xBADF0000 else ("full speed" if ro == 0 else "")
        if blackwell and ro and note != "PLM-blocked":
            note = f"fields {decode_readout1(ro)} - not rates"
        print(f"  READOUT_1 = {ro:#010x} {note}")

    def rd_tagged(name, off):
        v = bar.rd(off)
        b = badf_bucket(v)
        return v, (f" [{b}]" if b else "")

    print("  --- core/debug block (ssdiag 0.6.4 surface) ---")
    for name, off in CORE_ROWS:
        v, t = rd_tagged(name, off)
        print(f"  {name:<22} = {v:#010x}{t}")
    print("  --- privileged (badf at PL0 is the norm, still data) ---")
    rrow(bar, PRIV_ROWS)
    print("  --- family-specific / survey (badf on the wrong die is expected) ---")
    rrow(bar, ADA_ROWS)
    rrow(bar, AD1X_ROWS)
    if blackwell:
        print("  --- blackwell fuse/aux block (read-only; selectors are fuse mirrors) ---")
        rrow(bar, BLACKWELL_ROWS)
        print(f"  READOUT_1 fields (imla0 fmla16 fmla32 ffma imla1 imla2 imla3 imla4 DP) ="
              f" {decode_readout1(bar.rd(0x0082_3818))}")
        print("  note: on Blackwell selector 3 measures ~HALF rate, not the Ada 1/8 - trust the raw word")
        print("  note: FUSE_EN_SW_OVERRIDE=0 gates CTRL_OPT floorsweep only; FUSE_FEAT_OVR_DIS=0 leaves the"
              " override path open; FUSE_SS_PLM is NOT the effective gate")
    if fam in ("ampere", "ada"):
        rrow(bar, ADA_FUSE_ROWS)
        dump_tpc(bar)
        rrow(bar, OPT_FUSE_ROWS)
        print("  --- GA100-style per-GPC SM banks (badf elsewhere = absent) ---")
        ovr = "  ".join(f"{bar.rd(GA100_SM_OVR_BASE + 4 * g):#010x}" for g in range(GA100_GPCS))
        stat = "  ".join(f"{bar.rd(GA100_SM_STATUS_BASE + 4 * g):#010x}" for g in range(GA100_GPCS))
        print(f"  GA100_SM_OVR  {ovr}")
        print(f"  GA100_SM_STAT {stat}")
        print("  --- PCIe link block (XVE shadows; consumer dies read stock/benign) ---")
        rrow(bar, PCIE_LINK_ROWS)
        print("  --- XP3G fuse-override block (Gen2 capability layer) ---")
        rrow(bar, XP3G_ROWS)
    print("  --- ga100 vfi dictionary (kate-land/ga100vfi; read-only, cross-die probe) ---")
    for label, rows in [
        ("fusectl", GA100_FUSECTL_ROWS),
        ("sec-alias", GA100_SEC_ALIAS_ROWS),
        ("speed-sel", GA100_SPEED_SEL_ROWS),
        ("status-opt", GA100_STATUS_OPT_ROWS),
        ("opt-dis", GA100_OPT_DIS_ROWS),
        ("devid", GA100_DEVID_ROWS),
        ("tpc-def", GA100_TPC_DEF_ROWS),
        ("cp-shadow", GA100_CP_ROWS),
    ]:
        for name, off in rows:
            v = bar.rd(off)
            note = f" ({decode_plm(v)})" if name.endswith("PLM") else ""
            b = badf_bucket(v)
            tag = f" [{b}]" if b else ""
            print(f"  {label:<10} {name:<26} = {v:#010x}{note}{tag}")
    for label, base, count in [
        ("PES_GPC", GA100_PES_STATUS_BASE, 16),
        ("ROP_L2_FBP", GA100_ROP_L2_STATUS_BASE, 16),
        ("TPC_GPC", GA100_TPC_STATUS_BASE, 32),
        ("TPC_DIS", GA100_TPC_DISABLE_BASE, 8),
        ("TPC_RECFG", GA100_TPC_RECONFIG_BASE, 8),
    ]:
        for chunk in range(0, count, 8):
            cells = []
            for i in range(chunk, min(chunk + 8, count)):
                v = bar.rd(base + 4 * i)
                b = badf_bucket(v)
                cells.append(f"{v:#010x}" + (f" [{b}]" if b else ""))
            print(f"  {label:<10} [{chunk:>2}] " + " ".join(cells))
    print("  note: ga100 offsets, NV_FUSE_OPT_ prefix dropped; badf5040 = PL0-blocked (exists-behind-PLM OR")
    print("  cross-block absence - the filter precedes decode); plain value = GA100 semantics;"
          " ffffffff/badf1100 = not decoded here")
    if fam == "turing":
        # Mirror-round verdict (2026-11-11): on Turing the GA100-layout option
        # VALUES live at -0x7FF000 (the #187/TU104 bank); 0x820xxx is a
        # control-agreeing surface with empty option slots. The shift is only
        # defined inside the 0x82xxxx bank - the two PRI-ring rows at
        # 0x1200xx stay flat (they are already printed above).
        print("  --- turing fuse bank (ga100 dictionary at -0x7FF000; #13 / cold-values TU104) ---")
        for label, rows in [
            ("fusectl", GA100_FUSECTL_ROWS),
            ("sec-alias", GA100_SEC_ALIAS_ROWS),
            ("speed-sel", GA100_SPEED_SEL_ROWS),
            ("status-opt", GA100_STATUS_OPT_ROWS),
            ("opt-dis", GA100_OPT_DIS_ROWS),
            ("devid", GA100_DEVID_ROWS),
            ("tpc-def", GA100_TPC_DEF_ROWS),
            ("cp-shadow", GA100_CP_ROWS),
        ]:
            for name, off in rows:
                if off < 0x800_000:
                    continue  # not an option-bank row (the two RING words)
                v = bar.rd(off - 0x7FF_000)
                note = f" ({decode_plm(v)})" if name.endswith("PLM") else ""
                b = badf_bucket(v)
                tag = f" [{b}]" if b else ""
                print(f"  {label:<10} {name:<26} = {v:#010x}{note}{tag}")
        for label, base, count in [
            ("PES_GPC", GA100_PES_STATUS_BASE, 16),
            ("ROP_L2_FBP", GA100_ROP_L2_STATUS_BASE, 16),
            ("TPC_GPC", GA100_TPC_STATUS_BASE, 32),
            ("TPC_DIS", GA100_TPC_DISABLE_BASE, 8),
            ("TPC_RECFG", GA100_TPC_RECONFIG_BASE, 8),
        ]:
            for chunk in range(0, count, 8):
                cells = []
                for i in range(chunk, min(chunk + 8, count)):
                    v = bar.rd(base - 0x7FF_000 + 4 * i)
                    b = badf_bucket(v)
                    cells.append(f"{v:#010x}" + (f" [{b}]" if b else ""))
                print(f"  {label:<10} [{chunk:>2}] " + " ".join(cells))
        print(f"  GR_TOPO_409604         = {bar.rd(0x0040_9604):#010x}"
              " (TU104 KO anchor: 5 GPC/3 ROP = 0x06030005)")
    print("  --- turing mirror probe (0x820xxx vs -0x800000 vs -0x7FF000; open-questions #13) ---")
    print("  {:<24} {:>13} {:>13} {:>13}".format("word", "0x820xxx", "-0x800000", "-0x7FF000"))
    for name, off in GA100_MIRROR_WORDS:
        cells = []
        for base in (0, 0x800_000, 0x7FF_000):
            v = bar.rd(off - base)
            b = badf_bucket(v)
            cells.append(f"{v:#010x}" + (f" [{b}]" if b else ""))
        print(f"  {name:<24} " + " ".join(f"{c:>13}" for c in cells))
    print("  note: mirror == primary means a bit-23 alias (both addresses valid); mirror badf while")
    print("  primary is plain means 0x820xxx exclusive; different plain values in two columns = two live banks")
    for i in range(0, len(PLM_SURVEY), 4):
        row = PLM_SURVEY[i:i+4]
        print("  " + "  ".join(f"{o:#09x}={bar.rd(o):#010x}" for o in row))
    print("  --- ampere/ada FEAT sweep ---")
    for i in range(0, len(AMPADA_SWEEP), 5):
        row = AMPADA_SWEEP[i:i+5]
        print("  " + "  ".join(f"{o:#09x}={bar.rd(o):#010x}" for o in row))
    can = canary_state(bar)
    print(f"  CANARY_A/B = {bar.rd(CANARY_A):#010x}/{bar.rd(CANARY_B):#010x} -> {can}")
    unlocked = ss0 == SS0_UNLOCK and ss1 == SS1_UNLOCK

    print(f"  VERDICT: {'UNLOCKED (override values live)' if unlocked else 'not unlocked'}"
          f"{' - reapply available (door open)' if not unlocked and plm == 0xFFFF_FFFF else ''}")
    if blackwell:
        print("  FAMILY NOTE: no SEC2 stage and no GB20x booter on Blackwell - the transaction cannot run"
              " on this die; the FEAT block is read-only here (docs/blackwell-gb20x-status.md)")
        print("  FAMILY NOTE: nonzero SS0/SS1 are pre-driver shadow state, not an unlock -"
              " enable bits clear means the fused selectors are in force")
    return fam, regs, plm, unlocked


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bdf", help="only this PCI address, e.g. 0000:01:00.0")
    ap.add_argument("--reapply", action="store_true",
                    help="write SS0=0x88888888 SS1=0x8 where the PLM gate is open")
    ap.add_argument("--inventory", action="store_true",
                    help="print the unelevated read-only hardware inventory "
                         "block (OS/CPU/board/BIOS/chassis/per-GPU family "
                         "verdict + advisories) and exit - no root needed")
    args = ap.parse_args()

    if args.inventory:
        return inventory()

    if os.geteuid() != 0:
        print("root required (sysfs BAR0 mmap)", file=sys.stderr)
        return 2

    devs = find_gpus()
    if args.bdf:
        devs = [(b, p) for (b, p) in devs if b.endswith(args.bdf) or b.split(":")[-1] == args.bdf.split(":")[-1]]
    if not devs:
        print("no NVIDIA display controller found", file=sys.stderr)
        return 1

    # ssdiag 0.7.0 parity: a full dump opens with its own platform block
    inventory_block()

    rc = 0
    for bdf, devpath in devs:
        print(f"{bdf}:")
        try:
            bar = Bar0(devpath)
        except (PermissionError, RuntimeError) as e:
            print(f"  BAR0 unavailable: {e}")
            rc = 1
            continue
        st = report(bar, devpath=devpath)
        read_rebar(devpath)
        if st is None:
            continue
        fam, regs, plm, unlocked = st
        if args.reapply:
            if plm != 0xFFFF_FFFF:
                if fam.startswith("blackwell"):
                    print("  reapply REFUSED: the PLM gate only opens from PL3 firmware and no GB20x payload"
                          " exists - nothing to re-apply, the EFI chain cannot open it either")
                else:
                    print("  reapply REFUSED: FEAT PLM gate is closed - the EFI chain owns door-opening;"
                          " re-run the USB unlock instead")
                rc = 1
            else:
                bar.wr(regs["ss0"], SS0_UNLOCK)
                bar.wr(regs["ss1"], SS1_UNLOCK)
                rb0, rb1 = bar.rd(regs["ss0"]), bar.rd(regs["ss1"])
                ok = rb0 == SS0_UNLOCK and rb1 == SS1_UNLOCK
                print(f"  reapply: SS0={rb0:#010x} SS1={rb1:#010x} -> {'OK' if ok else 'READBACK MISMATCH'}"
                      " (read-back is the only success criterion)")
                rc = 0 if ok else 1
    return rc


if __name__ == "__main__":
    sys.exit(main())
