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
  sudo python3 diag-unlock.py --force-diag    # IO_STRICT_DEVMEM boxes: auto
                                              # unbind/re-bind around the run
  sudo python3 diag-unlock.py --bdf 0000:01:00.0 \
       --target-offset 0x409664 --value 0x88888888 --target-offset 0x40966c --value 0x8
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
    ("SM_ISSUE_RATE_MODIFIER", 0x0050_4204),
    ("SEC2_PLM", 0x0084_03C4),
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
# Round 6 (JeusChrit 11-21): OVR role in `RECONF & ~OVR` = 0x820a40+4g
# (cmpunlocker PR#55/#58), NOT the CTRL 0x820838 pair; AD102/AD103 6-page
# dumps show a live decoded island 0x820a40..0x820abc (32 words, zero)
# between badf5040 strips - sweep the full region so boundaries map per die.
GA100_SM_OVR_BASE = 0x0082_0A00
GA100_SM_OVR_ISLAND = 0x0082_0A40
GA100_SM_OVR_REGION_WORDS = 72  # 0x820a00..0x820b1c inclusive
GA100_SM_STATUS_BASE = 0x0082_0C38
# #237 round 6: OBJFUSE +0x248 companion words (0x557bd4, dispatched on
# fbp) - all 0 on the AD103 capture; swept so cut-L2 captures classify them.
OBJFUSE_248_ROWS = (
    ("OBJ248_821224", 0x0082_1224),
    ("OBJ248_FBP0", 0x0082_20C0),
    ("OBJ248_FBP1", 0x0082_20C4),
    ("OBJ248_FBP2", 0x0082_2100),
    ("OBJ248_FBP3", 0x0082_2104),
    ("OBJ248_FBP4", 0x0082_2108),
    ("OBJ248_FBP5", 0x0082_210C),
)
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
    # FEATURE_FUSES_OVR_DIS (0x8203f0) not listed: already probed as
    # PRIV_ROWS FUSE_OPT_FEATURE_OVR - used to print twice (#237 round 4).
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

# Issue #237 round (595.44.15-sourced): AD102-class arrays run gpc=0..11;
# Ada RECONFIG RAM + per-GPC ROP; clock trims; override pairs; PLM bits;
# GB20x HAL 67-71 PES/OFA relocation.
AD10X_ROP_PER_GPC_BASE = 0x0082_2880
AD10X_TPC_RECONFIG_BASE = 0x0082_2800
CLK_VF_SPEEDO_TRIMS = [0x0082_13A0, 0x0082_13A4, 0x0082_13A8, 0x0082_13AC, 0x0082_13B0, 0x0082_13B4, 0x0082_13B8]
VBIOS_OVR_PAIR_REGIONS = [(0x0082_2148, 23), (0x0082_2300, 41)]
PLM_PL0_WE = 0x0082_00F8  # bit4 = PL0 write enable
PLM_PL2_WE = [0x0082_3B24, 0x0082_3B28]  # bit6 = PL2 write enable
GB20X_PES_STATUS_BASE = 0x0082_2A78
GB20X_OFA_DISABLE = 0x0082_2A68
# #237 round 3: the 4090 L2-slice cut is FLAT BAR0 - per-LTC 4-bit LTS mask
# (~(val>>10)) & 0xf at NV_LTC_PRI_BASE 0x00140000 + 0x2000*K; broadcast
# 0x0017e02c (NV_PLTCG_LTCS). Boot-static (no writers in app01/07/08/GSP).
LTS_LTC0_SLICE = 0x0014_002C
LTS_LTC_STRIDE = 0x2000
LTS_LTC_SLOTS = 12
LTS_BROADCAST = 0x0017_E02C

# #237 round 4 (2026-11-20, AD102 4090 first live LTS readout): the per-FBP
# L2 arrays on Ada are the (LTC[2f+1]<<4)|LTC[2f] packing of the per-LTC RAW
# disable nibbles (bits[13:10] as stored, NOT complemented) - the 4090 proves
# the identity (nibbles 1/8/2/4... pack to 0x81/0x42/0x24/0x18 = the words at
# 0x821200/0x822900). sw/eff sit +0x880 apart (the Page-2 rule, same as
# per-GPC ROP 0x822000 -> 0x822880). GA100 dict's 0x8202c4/0x8205e8 moved.
AD10X_L2_CAND_FBP = 0x0082_1200  # OPT_ROP_L2_DISABLE_FBP[0..5] (stage-1 fuse)
AD10X_L2_SW_FBP = 0x0082_2080  # CTRL_OPT_ROP_L2_FBP[0..5] (RPR 37-42, OBJFUSE +0x228)
AD10X_L2_EFF_FBP = 0x0082_2900  # STATUS_OPT_ROP_L2_FBP[0..5] (OBJFUSE +0x270)
# Round-5 solve (was mislabeled the OPT_ROP_L2_DEFECTIVE_FBP hunt): the
# nonzero core 0x8211e8..0x8211fc is per-FBPA memory/PHY trim fuse data -
# app08 consumer copy (PC 0x3e00..0x3ffb) merges into NV_PFB_FBPA_n + 0x3d50.
# Word counts scale 0/2/4 with populated FBPAs (per-card data, not floorsweep);
# the 4090's six words f1/20/00/10/e1/10 are the AD102 reference row.
# DEFective-FBP hunting returns to OBJFUSE +0x248 only.
AD10X_FBPA_TRIM_BASE = 0x0082_11C0
AD10X_FBPA_TRIM_WORDS = 33
# app08 0x6ae4 SOURCE tables (what it would write if the four gates pass).
APP08_SRC_A_BASE = 0x0082_1480
APP08_SRC_A_WORDS = 66  # 0x821480..0x821584 inclusive, the reporter's stated end
APP08_SRC_B_BASE = 0x0082_2A48
APP08_SRC_B_WORDS = 128  # 0x822a48..0x822c44 inclusive
# Helper 0x944c inputs: pass iff (q1&1)!=0 || (q5&1)==0. Retail q1=0, q5=1
# -> BLOCK unless overridden.
APP08_GATE_Q1 = 0x0082_03F4
APP08_GATE_Q5 = 0x0082_04E4
# app08 65b1..6647 copies 0x12006c low 5 bits (active LTC count) into both
# broadcast words; GSP MemorySystem +0x740/+0x638 also touch 0x17e27c bits[4:0].
LTS_BCAST_LOW = 0x0017_E000
LTS_BCAST_HIGH = 0x0017_E27C
RING_LTC_COUNT = 0x0012_006C


def lts_disable_nibble(v):
    """RAW disable nibble bits[13:10] as stored (complement = active mask)."""
    return (v >> 10) & 0xF


def l2_pack_fbp(nibs):
    return [(nibs[2 * f + 1] << 4) | nibs[2 * f] for f in range(len(nibs) // 2)]


def iff_decode(v):
    """IFF_RECORD (0x820050) row range: START[9:0], inclusive LAST[25:16].

    ga100-ff-tools documents [8:0]/[24:16] for the 512-row GA100 (LAST <=
    511 fits 9 bits); our consumer captures need the 10-bit width -
    GA106/GA102 LAST=575, AD107 LAST=831, FUSE_ADDR agreeing exactly.
    """
    return (v & 0x3FF, (v >> 16) & 0x3FF)

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


# --force-diag bookkeeping: (devpath, driver-name) pairs unbound for the
# run, re-bound automatically at exit (strict-DEVMEM boxes; #246).
_FORCE_DIAG_UNBOUND = []


def _bound_driver_name(devpath):
    try:
        return os.path.basename(os.path.realpath(os.path.join(devpath, "driver")))
    except OSError:
        return None


def _driver_io(name, action, devpath):
    with open(os.path.join("/sys/bus/pci/drivers", name, action), "w") as f:
        f.write(os.path.basename(devpath))


class Bar0:
    def __init__(self, devpath, force_diag=False):
        with open(os.path.join(devpath, "resource")) as f:
            start = int(f.readline().split()[0], 16)
        if start == 0:
            raise RuntimeError("BAR0 not assigned (device powered down?)")
        self.start = start
        f = open(os.path.join(devpath, "resource0"), "r+b", 0)
        self.bound_driver = None
        try:
            self.map = mmap.mmap(f.fileno(), 16 * 1024 * 1024)
        except OSError as e:
            f.close()
            if e.errno != 22:
                raise
            # #246: CONFIG_IO_STRICT_DEVMEM kernels (Arch/Manjaro-style
            # configs) treat a driver-claimed BAR as exclusive - even for
            # root (iomem_is_exclusive in pci-sysfs.c; a lockdown denial
            # would be EPERM, not EINVAL). Two outs: iomem=relaxed on the
            # cmdline, or --force-diag (unbind, run, re-bind on exit).
            if not force_diag:
                raise RuntimeError(
                    "mmap(resource0) EINVAL - this kernel maps driver-claimed BARs "
                    "exclusively (CONFIG_IO_STRICT_DEVMEM=y, Arch/Manjaro-style "
                    "configs): boot with iomem=relaxed, or re-run with --force-diag "
                    "(auto unbind/re-bind; read the risk note in --help first), or "
                    "check zgrep IO_STRICT_DEVMEM /boot/config-$(uname -r)"
                ) from e
            drv = _bound_driver_name(devpath)
            if not drv:
                raise RuntimeError(
                    "mmap(resource0) EINVAL with nothing bound - not the strict-DEVMEM "
                    "claim; check the device state"
                ) from e
            print(f"  force-diag: unbinding {drv} from {os.path.basename(devpath)} "
                  "(auto-restore at exit; known risk: driver unbind can hang - "
                  "recovery = hard reset)")
            _driver_io(drv, "unbind", devpath)
            self.bound_driver = drv
            f = open(os.path.join(devpath, "resource0"), "r+b", 0)
            try:
                self.map = mmap.mmap(f.fileno(), 16 * 1024 * 1024)
            except OSError as e2:
                _driver_io(drv, "bind", devpath)
                self.bound_driver = None
                f.close()
                raise RuntimeError(
                    f"mmap(resource0) EINVAL even unbound ({e2}) - not the "
                    "strict-DEVMEM claim; driver re-bound"
                ) from e2
            _FORCE_DIAG_UNBOUND.append((devpath, drv))
            print("  force-diag: mmap OK unbound - strict-DEVMEM refusal confirmed; "
                  "the driver is re-bound automatically at exit")

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
    if 0x1B0 <= die <= 0x1BF:  # GB202/GB203M/GB205/GB206 class (50-series)
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
    print("  --- tpc (per-GPC floorsweep; STATUS bit set = TPC fused OFF; SM-count verified 5-die 2026-11-21) ---")
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
        # GB20x TPC CTRL overrides (0x820838+4g, 12 GPCs; ssdiag 0.7.7 parity, discussion #216).
        tpc_ctrl = " ".join(f"{bar.rd(TPC_CTRL_BASE + 4 * g):#010x}" for g in range(12))
        print(f"  TPC_CTRL (0x820838+4g, 12 GPCs): {tpc_ctrl}")
        rrow(bar, BLACKWELL_ROWS)
        print(f"  READOUT_1 fields (imla0 fmla16 fmla32 ffma imla1 imla2 imla3 imla4 DP) ="
              f" {decode_readout1(bar.rd(0x0082_3818))}")
        print("  note: on Blackwell selector 3 measures ~HALF rate, not the Ada 1/8 - trust the raw word")
        print("  note: FUSE_EN_SW_OVERRIDE=0 gates CTRL_OPT floorsweep only; FUSE_FEAT_OVR_DIS=0 leaves the"
              " override path open; FUSE_SS_PLM is NOT the effective gate")
        # GB20x HAL 67-71 relocated faces (0x820dbc/0x820e34 are GA100-era):
        # PES/PPC 0x822a78+4g, OFA 0x822a68, per-FBP LTC 0x822900+4f (#237).
        print("  GB20X faces (HAL 67-71):")
        print("   PES_STATUS " + " ".join(
            f"{bar.rd(GB20X_PES_STATUS_BASE + 4 * g):#010x}" for g in range(12)))
        v = bar.rd(GB20X_OFA_DISABLE)
        b = badf_bucket(v)
        print(f"   OFA_DISABLE {v:#010x}" + (f" [{b}]" if b else ""))
        print("   FBP_LTC    " + " ".join(
            f"{bar.rd(0x0082_2900 + 4 * f):#010x}" for f in range(8)))
    if fam in ("ampere", "ada"):
        rrow(bar, ADA_FUSE_ROWS)
        dump_tpc(bar)
        rrow(bar, OPT_FUSE_ROWS)
        print("  --- GA100-style per-GPC SM banks (badf elsewhere = absent) ---")
        # Round 6: full OVR region sweep (guard/island/tail). The island IS
        # the OVR layer of `STATUS = OPT | (RECONF & ~OVR)` (cmpunlocker
        # #55/#58 wrote 0x820a40+4g to cancel RECONF cuts on GA100, +4 SM).
        print(f"  GA100_SM_OVR_RGN {GA100_SM_OVR_BASE:#010x}..{GA100_SM_OVR_BASE + 4 * (GA100_SM_OVR_REGION_WORDS - 1):#010x} (island 0x820a40+4g = RECONF-cancel OVR per cmpunlocker#55/#58)")
        for chunk in range(0, GA100_SM_OVR_REGION_WORDS, 8):
            start = GA100_SM_OVR_BASE + 4 * chunk
            zone = "guard" if start < GA100_SM_OVR_ISLAND else ("island" if start < GA100_SM_OVR_ISLAND + 32 * 4 else "tail")
            cells = " ".join(f"{bar.rd(GA100_SM_OVR_BASE + 4 * i):#010x}"
                             for i in range(chunk, min(chunk + 8, GA100_SM_OVR_REGION_WORDS)))
            print(f"    [{zone}] {start:#010x} {cells}")
        stat = "  ".join(f"{bar.rd(GA100_SM_STATUS_BASE + 4 * g):#010x}" for g in range(GA100_GPCS))
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
            # Round 4 label, corrected round 6: bank A is NOT the GPC0
            # disable mask - 4090/4080 read identical 1,0,1,0 despite
            # different GPC0 masks (0x3f vs 0x01), a proven coincidence -
            # it is die-constant data of unknown role.
            ada_note = ""
            if fam == "ada":
                if 0x0082_03A0 <= off <= 0x0082_03B8:
                    ada_note = " [Ada bank A: die-constant, NOT GPC0 disable]"
                elif name in ("TPC_GPC7_DEFECTIVE", "TPC_GPC7_DISABLE_CP"):
                    ada_note = " [GA100-only]"
            print(f"  {label:<10} {name:<26} = {v:#010x}{note}{tag}{ada_note}")
    for label, base, count in [
        ("PES_GPC", GA100_PES_STATUS_BASE, 12),
        ("ROP_L2_FBP", GA100_ROP_L2_STATUS_BASE, 16),
        ("TPC_GPC", GA100_TPC_STATUS_BASE, 32),
        ("TPC_DIS", GA100_TPC_DISABLE_BASE, 12),
        ("TPC_RECFG_G100", GA100_TPC_RECONFIG_BASE, 12),
        ("TPC_RECFG_ADA", AD10X_TPC_RECONFIG_BASE, 12),
        ("ROP_GPC_ADA", AD10X_ROP_PER_GPC_BASE, 12),
    ]:
        for chunk in range(0, count, 8):
            cells = []
            for i in range(chunk, min(chunk + 8, count)):
                v = bar.rd(base + 4 * i)
                b = badf_bucket(v)
                cells.append(f"{v:#010x}")
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
            ("PES_GPC", GA100_PES_STATUS_BASE, 12),
            ("ROP_L2_FBP", GA100_ROP_L2_STATUS_BASE, 16),
            ("TPC_GPC", GA100_TPC_STATUS_BASE, 32),
            ("TPC_DIS", GA100_TPC_DISABLE_BASE, 12),
            ("TPC_RECFG_G100", GA100_TPC_RECONFIG_BASE, 12),
            ("TPC_RECFG_ADA", AD10X_TPC_RECONFIG_BASE, 12),
            ("ROP_GPC_ADA", AD10X_ROP_PER_GPC_BASE, 12),
        ]:
            for chunk in range(0, count, 8):
                cells = []
                for i in range(chunk, min(chunk + 8, count)):
                    v = bar.rd(base - 0x7FF_000 + 4 * i)
                    b = badf_bucket(v)
                    cells.append(f"{v:#010x}")
                print(f"  {label:<10} [{chunk:>2}] " + " ".join(cells))
    print("  --- turing mirror probe (0x820xxx vs -0x800000 vs -0x7FF000; open-questions #13) ---")
    print("  {:<24} {:>13} {:>13} {:>13}".format("word", "0x820xxx", "-0x800000", "-0x7FF000"))
    for name, off in GA100_MIRROR_WORDS:
        cells = []
        for base in (0, 0x800_000, 0x7FF_000):
            v = bar.rd(off - base)
            b = badf_bucket(v)
            cells.append(f"{v:#010x}")
        print(f"  {name:<24} " + " ".join(f"{c:>13}" for c in cells))
    print("  note: mirror == primary means a bit-23 alias (both addresses valid); mirror badf while")
    print("  primary is plain means 0x820xxx exclusive; different plain values in two columns = two live banks")
    # i237 disassembly round: clock trims, VBIOS override pairs, PLM bits.
    print("  --- i237 disassembly round (595.44.15-sourced; read-only) ---")
    print("  CLK_TRIMS (10-bit words: bits[5:0] signed-6 temp, bits[9:6] signed-4 volt)")
    for t in CLK_VF_SPEEDO_TRIMS:
        v = bar.rd(t)
        b = badf_bucket(v)
        if b:
            print(f"    {t:#010x} = {v:#010x} [{b}]")
            continue
        t6 = (v & 0x3F) - 64 if v & 0x20 else (v & 0x3F)
        hi = (v >> 6) & 0xF
        v4 = hi - 16 if hi & 0x8 else hi
        print(f"    {t:#010x} = {v:#010x} temp={t6:+} volt={v4:+}")
    for base, pairs in VBIOS_OVR_PAIR_REGIONS:
        printed = 0
        for chunk in range(pairs):
            off = base + chunk * 8
            d, c = bar.rd(off), bar.rd(off + 4)
            if d == 0 and c == 0:
                continue
            if printed % 4 == 0:
                if printed > 0:
                    print()
                # row header = this line's first pair address (pairs step +8)
                print(f"  OVR_PAIRS  {off:#010x}", end="")
            print(f" d={d:#010x} c={c:#010x}", end="")
            printed += 1
        if printed > 0:
            print()
    # #237 round 4: the per-FBP L2 arrays on Ada + the packing identity.
    # cand 0x821200+4f = OPT_ROP_L2_DISABLE_FBP (stage-1 fuse), sw
    # 0x822080+4f = CTRL_OPT_ROP_L2_FBP (RPR 37-42, OBJFUSE +0x228), eff
    # 0x822900+4f = STATUS_OPT_ROP_L2_FBP (OBJFUSE +0x270); pred =
    # (LTC[2f+1]<<4)|LTC[2f] from the LTS_L2SLICE face below - the 4090
    # capture predicts 0x81/0x42/0x24/0x18/0x81/0x42 for cand+eff.
    print("  L2_FBP (fbp=0..5; cand 0x821200+4f / sw 0x822080+4f / eff 0x822900+4f; pred=(LTC[2f+1]<<4)|LTC[2f])")
    for f in range(6):
        c = bar.rd(AD10X_L2_CAND_FBP + 4 * f)
        s = bar.rd(AD10X_L2_SW_FBP + 4 * f)
        e = bar.rd(AD10X_L2_EFF_FBP + 4 * f)
        lo = bar.rd(LTS_LTC0_SLICE + (2 * f) * LTS_LTC_STRIDE)
        hi = bar.rd(LTS_LTC0_SLICE + (2 * f + 1) * LTS_LTC_STRIDE)
        if badf_bucket(lo) is None and badf_bucket(hi) is None:
            pr = l2_pack_fbp([lts_disable_nibble(lo), lts_disable_nibble(hi)])[0]
            marks = ""
            for nm, w in (("cand", c), ("eff", e)):
                if badf_bucket(w) is None:
                    tag = "pack" if w == pr else f"!=pack({pr:#04x})"
                    marks += f" {nm}={tag}"
            pred_s = f"{pr:#04x}"
        else:
            pred_s, marks = "n/a", ""
        print(f"    FBP[{f}] {AD10X_L2_CAND_FBP + 4 * f:#010x} cand={c:#010x} sw={s:#010x} eff={e:#010x} pred={pred_s}{marks}")
    # #237 round 6: OBJFUSE +0x248 companion words (0x557bd4 dispatch on
    # fbp) - nonzero on the 4090/3060 would make them the DEF-array
    # candidate that round 4 failed to find at the DEF window.
    print("  OBJFUSE_248 (0x557bd4 paths: 0x821224; fbp<2 0x8220c0/c4; fbp>=2 0x822100+4f)")
    for name, off in OBJFUSE_248_ROWS:
        v = bar.rd(off)
        b = badf_bucket(v)
        tag = f" [{b}]" if b else ""
        print(f"    {name:<14} {off:#010x} = {v:#010x}{tag}")
    # Round-5 solve: FBPA_TRIM - the nonzero core 0x8211e8..0x8211fc is
    # per-FBPA mem/PHY trim fuses (app08 consumer copy merges into
    # NV_PFB_FBPA_n + 0x3d50), NOT the DEFective array - that stays hunted
    # at OBJFUSE +0x248. Trim words are per-card data; compare same-die only.
    print("  FBPA_TRIM (0x8211c0..0x821240; per-FBPA mem/PHY trim fuses, ex-L2_DEF_SCAN; DEF hunt stays at OBJFUSE +0x248)")
    for chunk in range(0, AD10X_FBPA_TRIM_WORDS, 8):
        cells = " ".join(f"{bar.rd(AD10X_FBPA_TRIM_BASE + 4 * i):#010x}"
                         for i in range(chunk, min(chunk + 8, AD10X_FBPA_TRIM_WORDS)))
        print(f"    {AD10X_FBPA_TRIM_BASE + 4 * chunk:#010x} {cells}")
    # #237 round 4 sweep: the app08 0x6ae4 SOURCE tables (what it would write
    # if the four gates pass) - 4090 slice pattern or the AD103 values?
    for name, base, words in (
        ("APP08_SRC_A", APP08_SRC_A_BASE, APP08_SRC_A_WORDS),
        ("APP08_SRC_B", APP08_SRC_B_BASE, APP08_SRC_B_WORDS),
    ):
        print(f"  {name} ({base + 4 * (words - 1):#010x} end, {words} words)")
        for chunk in range(0, words, 8):
            cells = " ".join(f"{bar.rd(base + 4 * i):#010x}"
                             for i in range(chunk, min(chunk + 8, words)))
            print(f"    {base + 4 * chunk:#010x} {cells}")
    pl0we = bar.rd(PLM_PL0_WE)
    print(f"  PLM_PL0_WE {PLM_PL0_WE:#010x} = {pl0we:#010x} bit4={(pl0we >> 4) & 1} (FUSECTRL_PLM bit4; 1=host-level writes enabled)")
    # #237 round 4: the complete app08 0x6ae4 write gate. Helper 0x944c tests
    # two OBJFUSE query words: pass iff (q1&1)!=0 || (q5&1)==0 (retail q1=0,
    # q5=1 -> BLOCK unless overridden); plus both PL2 write-enable bits.
    # DMEM8[0x646]/[0x64d] are ROM-internal.
    q1 = bar.rd(APP08_GATE_Q1)
    q5 = bar.rd(APP08_GATE_Q5)
    print(f"  APP08_GATE_Q1 {APP08_GATE_Q1:#010x} = {q1:#010x} bit0={q1 & 1} (OBJFUSE query 0xff000001; retail 0)")
    print(f"  APP08_GATE_Q5 {APP08_GATE_Q5:#010x} = {q5:#010x} bit0={q5 & 1} (OBJFUSE query 0xff000005; retail 1)")
    # bit6 read from a badf word is the POISON's bit, not the register's
    # (0xbadf5040 bit6 = 1 - caught on the 2026-11-20 stock TU106 dump):
    # degrade to "?" and never let a poison bit decide the gate.
    pl2_bits = []
    for p in PLM_PL2_WE:
        v = bar.rd(p)
        pl2_bits.append(None if badf_bucket(v) else (v >> 6) & 1)
    if badf_bucket(q1) or badf_bucket(q5):
        helper, gate = "?", "?"
    else:
        h = (q1 & 1) != 0 or (q5 & 1) == 0
        helper = "PASS" if h else "BLOCK"
        if not h:
            gate = "CLOSED"
        elif pl2_bits == [1, 1]:
            gate = "OPEN"
        elif any(b is None for b in pl2_bits):
            gate = "?"
        else:
            gate = "CLOSED"
    print(f"  APP08_HELPER_944C = {helper} ((q1&1)!=0 || (q5&1)==0)")
    for p, b6 in zip(PLM_PL2_WE, pl2_bits):
        v = bar.rd(p)
        print(f"  PLM_PL2_WE {p:#010x} = {v:#010x} bit6={'?' if b6 is None else b6} (VBIOS app08 gate)")
    print(f"  APP08_WRITE_GATE = {gate} (PL2 pair bit6=1/1 + helper 0x944c; DMEM8[0x646]/[0x64d] are ROM-internal)")
    # #237 round 3: per-LTC LTS (L2 slice) masks - flat BAR0, every family.
    print("  LTS_L2SLICE (per-LTC 4-bit slice mask = (~val>>10)&0xf; flat 0x14002c + 0x2000*K)")
    for k in range(LTS_LTC_SLOTS):
        a = LTS_LTC0_SLICE + k * LTS_LTC_STRIDE
        v = bar.rd(a)
        b = badf_bucket(v)
        if b:
            print(f"    LTC[{k:>2}] {a:#010x} = {v:#010x} [{b}]")
            continue
        active = (~(v >> 10)) & 0xF
        print(f"    LTC[{k:>2}] {a:#010x} = {v:#010x} slices={active:04b} ({active.bit_count()}/4)")
    v = bar.rd(LTS_BROADCAST)
    b = badf_bucket(v)
    if b:
        print(f"    BCAST     {LTS_BROADCAST:#010x} = {v:#010x} [{b}]")
    else:
        active = (~(v >> 10)) & 0xF
        print(f"    BCAST     {LTS_BROADCAST:#010x} = {v:#010x} slices={active:04b} ({active.bit_count()}/4)")
    # #237 round 4: the other two LTC broadcast words - app08 65b1..6647
    # copies 0x12006c low 5 bits (active LTC count) into both; GSP
    # MemorySystem +0x740/+0x638 also read/write bits[4:0] of 0x17e27c.
    for name, off in (("LTCNT_LO", LTS_BCAST_LOW), ("LTCNT_HI", LTS_BCAST_HIGH), ("LTCNT_SRC", RING_LTC_COUNT)):
        v = bar.rd(off)
        b = badf_bucket(v)
        if b:
            print(f"    {name:<9} {off:#010x} = {v:#010x} [{b}]")
        else:
            print(f"    {name:<9} {off:#010x} = {v:#010x} ltcnt={v & 0x1f}")
    # GR topology anchor, every family (open-questions #19): byte3 = die GPC
    # slots, byte0 = active GPC count - three-point calibrated (TU104 KO
    # 06030005, TU106 2070 08040003, TU106 1e93 08040006). On Ampere it is
    # the OQ#19 polarity discriminator (GA106 byte0 4 -> masks bit=off,
    # 7 -> bit=on).
    # ga100-ff-tools cross round: IFF_RECORD row range + MAGIC_D dual bits.
    iff = bar.rd(0x0082_0050)
    b = badf_bucket(iff)
    if b:
        print(f"  IFF_RECORD {0x0082_0050:#010x} = {iff:#010x} [{b}] (row-range decode needs a plain word; TU106 PL0-blocks it)")
    else:
        start, last = iff_decode(iff)
        fa = bar.rd(0x0082_0004)
        agree = ""
        if badf_bucket(fa) is None and fa == last:
            agree = " ; == FUSE_ADDR"
        print(f"  IFF_RECORD {0x0082_0050:#010x} = {iff:#010x} rows={start}..{last} ({last + 1} rows; GA100 domain 222..511 - consumer dies extend{agree})")
    magic = bar.rd(0x0082_0520)
    b = badf_bucket(magic)
    if b:
        print(f"  MAGIC_D_BITS {0x0082_0520:#010x} = {magic:#010x} [{b}] (bit decode needs a plain word)")
    else:
        print(f"  MAGIC_D_BITS {0x0082_0520:#010x} = {magic:#010x} bit21(app08 target)={(magic >> 21) & 1} bit25(gen4 restriction)={(magic >> 25) & 1}")
    gr = bar.rd(0x0040_9604)
    print(f"  GR_TOPO_409604 0x00409604 = {gr:#010x}"
          f" (byte3 GPC-slots {(gr >> 24) & 0xff} / byte0 active-GPC {gr & 0xff};"
          " anchors 06030005/08040003/08040006)")
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


# --- #237 round 6: --fuserows, WRITE MODE (user-approved) ---
# The same fuse-controller IP block lives at two bases: GA10x/Ada at
# 0x820000, Turing at 0x21000 (envytools pfuse; both read plausible idle
# CTRL on the 2070). Offsets inside the block are identical.
FUSE_BASE_GA100_LAYOUT, FUSE_BASE_TURING = 0x0082_0000, 0x0002_1000
FUSE_CMD_READ, FUSE_CMD_MASK, FUSE_SPIN_LIMIT = 0x1, 0x3, 1 << 20
FUSE_MAX_ROWS = 512


def fuse_ctrl(base):
    return base


def fuse_ctrl_plm(base):
    return base + 0xF8


def fuse_addr(base):
    return base + 0x4


def fuse_data(base):
    return base + 0x8


def fuse_iff(base):
    return base + 0x50


def fuse_debug(base):
    return base + 0x60


def fuse_crc(base):
    return base + 0x70


def _fuse_guard_ok(bar, base, crc_readable=True):
    """One ff-tools guard round; returns an error string or None.

    crc_readable=False (Turing: CRC_STATUS badf at PL0) skips the CRC
    check - decided once before any write, announced in the header.
    """
    ctrl = bar.rd(fuse_ctrl(base))
    if ctrl & 0xFFFF0000 == 0xBADF0000:
        return f"FUSECTRL reads {ctrl:#010x} (PRI error) - controller not accessible"
    if ctrl == 0xFFFF_FFFF:
        return "FUSECTRL reads all-ones - not decoded/accessible here"
    if (ctrl >> 16) & 31 != 4 or ctrl & FUSE_CMD_MASK != 0:
        return f"FUSECTRL {ctrl:#010x} not idle (state={(ctrl >> 16) & 31} cmd={ctrl & FUSE_CMD_MASK})"
    crc = bar.rd(fuse_crc(base))
    # GA100 reads bare 0x55 (ff-tools full-word check); EVERY consumer die
    # observed reads 0x4555 (GA106 live 2026-11-22 user run + GA102/AD103/
    # AD107/AD102 archived) - the low byte is the ready signature, upper
    # bits carry die-dependent status. Strict equality was GA100-tuned and
    # aborted GA10x/Ada before any row.
    if crc_readable and crc & 0xFF != 0x55:
        return f"CRC_STATUS {crc:#010x} - low byte != 0x55"
    dbg = bar.rd(fuse_debug(base))
    if dbg & 0xFFFF0000 == 0xBADF0000 or dbg == 0xFFFF_FFFF:
        return f"FUSEDEBUG {dbg:#010x} unreadable - debug state unknown"
    # Two observed-normal values: 0 on GA100/GA10x, 0x400000 on Ada
    # (4060L/4090/2860, identical locked and unlocked). Anything else is an
    # unknown debug state - row data could be falsified; abort rather than
    # guess bit semantics we do not have.
    if dbg != 0 and dbg != 0x0040_0000:
        return f"FUSEDEBUG {dbg:#010x} not a known-normal value (0 GA100/GA10x, 0x400000 Ada) - possible debug mode, row data could be falsified"
    return None


def _fuse_wait_idle(bar, base):
    for _ in range(FUSE_SPIN_LIMIT):
        ctrl = bar.rd(fuse_ctrl(base))
        if (ctrl >> 16) & 31 == 4 and ctrl & FUSE_CMD_MASK == 0:
            return None
    return "controller never returned to idle within the spin limit"


def fuse_row_dump(bar, rng, base=FUSE_BASE_GA100_LAYOUT):
    """Physical fuse-row capture, ported from ga100-ff-tools read_rows.

    Writes are ONLY row numbers to FUSEADDR (latch-verified) and CMD_READ
    to FUSECTRL - no programming command on this path; the original
    FUSEADDR is restored+verified on every exit path. Returns complete.
    """
    print("--- FUSE ROW DUMP (WRITE MODE) - writes: FUSEADDR row numbers + FUSECTRL CMD_READ only; no programming command; address restored after ---")
    print("  protocol: ga100-ff-tools read_rows (two passes, latch checks); a bound kernel driver is a concurrency risk - pass mismatches mark rows unreliable")
    fail = None
    crc_word = bar.rd(fuse_crc(base))
    crc_readable = not (crc_word & 0xFFFF0000 == 0xBADF0000 or crc_word == 0xFFFF_FFFF)
    err = _fuse_guard_ok(bar, base, crc_readable)
    if err:
        print(f"  FUSEROWS ABORT: {err} - no writes performed")
        return False
    if not crc_readable:
        print(f"  FUSEROWS DEGRADED: CRC_STATUS {crc_word:#010x} unreadable at this base (Turing: undecoded/PLM-blocked at PL0) - proceeding WITHOUT the CRC guard; latch verification, two passes and address restore remain active")
    saved = bar.rd(fuse_addr(base))
    if saved & 0xFFFF0000 == 0xBADF0000 or saved == 0xFFFF_FFFF:
        print(f"  FUSEROWS ABORT: FUSEADDR {saved:#010x} unreadable - no writes performed")
        return False
    # IFF first (issue #237 round 9, ported from ssdiag 0.8.15): the per-die
    # decoded bound is the sane pre-check for the resting FUSEADDR - a
    # hardcoded GA100 512-row cap aborted Ada's 832-row domain (A6000 GA102:
    # 576 rows) at the exact capture it exists to enable. FUSE_MAX_ROWS now
    # guards only dies with NO readable IFF.
    iff = bar.rd(fuse_iff(base))
    iff_readable = not (iff & 0xFFFF0000 == 0xBADF0000 or iff == 0xFFFF_FFFF)
    if iff_readable:
        _, last = iff_decode(iff)
        if saved > last:
            print(f"  FUSEROWS ABORT: FUSEADDR {saved} outside the IFF domain 0..{last} - no writes performed")
            return False
        if rng == "all":
            first, last_row = 0, last
        else:
            ra, rb = rng
            if rb > last:
                # Explicit over-domain range: read-class, rows past LAST
                # read junk (or the controller refuses the latch - TU106
                # bench, row LAST+1), never harm. Degrade, don't refuse.
                print(f"  FUSEROWS DEGRADED: explicit range {ra:#x}-{rb:#x} extends past the IFF domain 0..{last} - out-of-domain rows read junk, not harm")
            first, last_row = ra, rb
    else:
        # No IFF on this die: domain unverified; explicit range mandatory.
        if saved >= FUSE_MAX_ROWS:
            print(f"  FUSEROWS ABORT: FUSEADDR {saved} outside the {FUSE_MAX_ROWS}-row address cap (IFF unreadable, no per-die bound) - no writes performed")
            return False
        if rng == "all" or rng is None:
            print(f"  FUSEROWS ABORT: IFF_RECORD {iff:#010x} unreadable and no explicit range given - pass --fuserows=A-B (capped at {FUSE_MAX_ROWS} rows); no writes performed")
            return False
        ra, rb = rng
        if rb >= FUSE_MAX_ROWS:
            print(f"  FUSEROWS ABORT: range {ra:#x}-{rb:#x} exceeds the {FUSE_MAX_ROWS}-row address cap")
            return False
        print(f"  FUSEROWS DEGRADED: IFF_RECORD {iff:#010x} unreadable - row domain UNVERIFIED, dumping the requested {ra:#x}..{rb:#x}; out-of-domain rows read junk, not harm")
        first, last_row = ra, rb
    print(f"  domain: rows {first}..{last_row} (IFF_RECORD {iff:#010x}, saved FUSEADDR {saved}) - two passes")
    passes = [[], []]
    for p in range(2):
        buf = passes[p]
        for row in range(first, last_row + 1):
            err = _fuse_guard_ok(bar, base, crc_readable)
            if err:
                fail = f"pass {p + 1} row {row}: {err}"
                break
            bar.wr(fuse_addr(base), row)
            if bar.rd(fuse_addr(base)) != row and row != 0:
                # Distinguish the two failure shapes: a PLM-gated fuse block
                # (GB20x: FUSECTRL_PLM write nibble closes PL0 - host writes
                # dropped, row 0 only "works" because the resting address
                # already is 0) vs a controller-state anomaly.
                fcp = bar.rd(fuse_ctrl_plm(base))
                if not (fcp & 0xFFFF0000 == 0xBADF0000 or fcp == 0xFFFF_FFFF) and not (fcp >> 4) & 1:
                    fail = (f"pass {p + 1} row {row}: FUSEADDR did not latch - "
                            f"FUSECTRL_PLM {fcp:#010x} has PL0-write CLOSED: the fuse-block "
                            f"registers are PLM-gated against host writes (observed on GB20x "
                            f"professional parts; the row dump needs a controller with "
                            f"host-write enabled, e.g. GA10x/Turing)")
                else:
                    fail = f"pass {p + 1} row {row}: FUSEADDR did not latch"
                break
            bar.wr(fuse_ctrl(base), FUSE_CMD_READ)
            err = _fuse_wait_idle(bar, base)
            if err:
                fail = f"pass {p + 1} row {row}: {err}"
                break
            v = bar.rd(fuse_data(base))
            if v & 0xFFFF0000 == 0xBADF0000:
                fail = f"pass {p + 1} row {row}: FUSEDATA {v:#010x} (PRI error)"
                break
            buf.append(v)  # all-ones is a legal OTP word, kept verbatim
        if fail:
            break
    bar.wr(fuse_addr(base), saved)
    restored = bar.rd(fuse_addr(base)) == saved
    if not restored:
        print(f"  FUSEROWS RESTORE FAILED: original FUSEADDR {saved} DID NOT RESTORE (reads {bar.rd(fuse_addr(base)):#010x}) - investigate before trusting anything else")
    complete = fail is None and restored
    if fail:
        print(f"  FUSEROWS ABORT: {fail} - gathered prefix below, INCOMPLETE")
    elif complete:
        print("  FUSEROWS OK: both passes complete, address restored")
    mismatches = [i for i in range(len(passes[0])) if passes[0][i] != passes[1][i]] if complete else []
    if complete and not mismatches:
        print(f"  two passes match ({len(passes[0])} rows)")
    elif complete:
        print(f"  PASS MISMATCH: {len(mismatches)} row(s) differ between passes - treat as unreliable")
        for i in mismatches[:16]:
            print(f"    row {first + i:#x}: pass1 {passes[0][i]:#010x} pass2 {passes[1][i]:#010x}")
    for i, v in enumerate(passes[0]):
        ann = _iff_op_annotate(passes[0], i, v)
        if ann:
            print(f"FUSEROW {first + i:#x} = {v:#010x}   {ann}")
        else:
            print(f"FUSEROW {first + i:#x} = {v:#010x}")
    return complete


def _iff_op_annotate(rows: list, i: int, v: int):
    """IFF op-grammar role of row i (value v) in the sequential stream
    (issue #237 round 10, ssdiag 0.8.16 parity): op = w & 0x3f (0x3d =
    2-word WRITE_IMM, 0x3c = 3-word RMW), addr = (w & 0xffffffc0) >> 4.
    Operand rows back-scan to their head; an undecodable head orphans
    only itself."""
    op = v & 0x3F
    if op not in (0x3D, 0x3C):
        for h in range(max(0, i - 2), i):
            hw = rows[h]
            hop = hw & 0x3F
            ln = 2 if hop == 0x3D else 3 if hop == 0x3C else 0
            if ln and h + ln > i:
                which = "value" if i == h + 1 and hop == 0x3D else ("and-mask" if i == h + 1 else "or-mask")
                kind = "WRITE_IMM" if hop == 0x3D else "RMW"
                return f"[{which} of {kind} {(hw & 0xFFFFFFC0) >> 4:#010x}]"
        return None
    addr = (v & 0xFFFFFFC0) >> 4
    # Semantic tag for the known-target heads (issue #237 rounds 10-11):
    # PLM-class writes, the GC6_AON pair, the CYA_0_DISG2 close. Unknown
    # addresses stay untagged - a reading aid, not a decode claim.
    if op == 0x3D:
        tag = {0x0000E224: " (PLM)", 0x00021068: " (PLM)",
               0x00118FEC: " (PLM)", 0x00118FFC: " (PLM)",
               0x0011839C: " (GC6_AON)", 0x001183A0: " (GC6_AON)"}.get(addr, "")
    else:
        tag = {0x0008C2C0: " (CYA_0_DISG2)"}.get(addr, "")
    return f"[{'WRITE_IMM' if op == 0x3D else 'RMW'} {addr:#010x}{tag}]"


# --- Read-only VBIOS PROM BIT / ucode table metadata walker (ssdiag parity) ---
PROM_BASE = 0x0030_0000
PROM_SIZE = 0x0010_0000  # 1 MiB NV_PROM aperture (0x300000..0x3ffffc)

PROM_APP_NAMES = {
    0x01: "PRE_OS",
    0x05: "FIRMWARE_SEC_LIC",
    0x08: "LS_UDE",
    0x23: "APP_0x23",
    0x45: "FWSEC_DBG",
    0x49: "SCRUBBER",
    0x85: "FWSEC_PROD",
}


def prom_app_name(app_id):
    return PROM_APP_NAMES.get(app_id, "-")


def _le16(buf, off):
    if off < 0 or off + 2 > len(buf):
        return None
    return int.from_bytes(buf[off : off + 2], "little")


def _le32(buf, off):
    if off < 0 or off + 4 > len(buf):
        return None
    return int.from_bytes(buf[off : off + 4], "little")


def prom_pcir_info(rom, off):
    if off < 0 or off + 0x1A > len(rom) or rom[off : off + 2] != b"\x55\xaa":
        return None
    pcir_rel = _le16(rom, off + 0x18)
    if pcir_rel is None:
        return None
    p = off + pcir_rel
    if p + 0x16 > len(rom):
        return None
    tag = rom[p : p + 4]
    if tag not in (b"PCIR", b"NPDS", b"RGIS"):
        return None
    slen = _le16(rom, p + 0x0A)
    sub = _le16(rom, p + 0x10)
    code_type = rom[p + 0x14]
    npde_at = (p + slen + 0xF) & ~0xF
    if rom[npde_at : npde_at + 4] == b"NPDE":
        ext_len = _le16(rom, npde_at + 6)
        sub_len = _le16(rom, npde_at + 8)
        if ext_len is not None and sub_len is not None and ext_len >= 10 and npde_at + ext_len <= len(rom):
            sub = sub_len
    return {
        "offset": off,
        "tag": tag.decode("ascii"),
        "code_type": code_type,
        "image_len": sub * 512,
    }


def prom_find_expansion(image):
    curr = 0
    base_size = None
    for _ in range(16):
        info = prom_pcir_info(image, curr)
        if info is None:
            return None
        if info["code_type"] == 0xE0:
            if base_size is None or curr < base_size:
                return None
            return curr - base_size
        if info["code_type"] == 0x00 and base_size is None:
            base_size = info["image_len"]
        if info["image_len"] == 0:
            return None
        nxt = curr + info["image_len"]
        if nxt >= len(image):
            return None
        curr = nxt
    return None


def parse_prom_bit(rom):
    """Parse VBIOS PROM BIT + token 0x70 v2 ucode table metadata (no firmware dumps)."""
    if len(rom) < 4:
        return {"error": "unreadable", "reason": "aperture shorter than 4 bytes"}
    first_word = _le32(rom, 0)
    if (first_word >> 16) == 0xBADF:
        return {"error": "unreadable", "reason": "PRI error (0xBADF) at PROM+0x0"}
    trimmed = rom[: len(rom) - (len(rom) % 4)]
    if trimmed and all(b == 0 for b in trimmed):
        return {"error": "unreadable", "reason": "all-zero PROM aperture"}
    if trimmed and all(b == 0xFF for b in trimmed):
        return {"error": "unreadable", "reason": "all-ones PROM aperture"}

    rom_images = []
    for off in range(0, max(0, len(rom) - 0x19), 0x200):
        info = prom_pcir_info(rom, off)
        if info is not None:
            rom_images.append(info)

    bit_off = None
    for a in range(0, max(0, len(rom) - 11), 2):
        if _le16(rom, a) == 0xB8FF and rom[a + 2 : a + 6] == b"BIT\x00":
            hs = rom[a + 8]
            if hs >= 12 and a + hs <= len(rom) and (sum(rom[a : a + hs]) & 0xFF) == 0:
                bit_off = a
                break
    if bit_off is None:
        return {"error": "no_bit_header"}

    bit_hdr_size = rom[bit_off + 8]
    bit_stride = rom[bit_off + 9]
    bit_token_count = rom[bit_off + 10]
    if not (6 <= bit_stride <= 8):
        return {"error": "unsupported_stride", "stride": bit_stride}

    preceding_x86 = [img for img in rom_images if img["offset"] <= bit_off and img["code_type"] == 0x00]
    preceding_any = [img for img in rom_images if img["offset"] <= bit_off]
    if preceding_x86:
        image_base = preceding_x86[-1]["offset"]
    elif preceding_any:
        image_base = preceding_any[-1]["offset"]
    else:
        image_base = 0

    expansion_off = prom_find_expansion(rom[image_base:])
    adjust = image_base + expansion_off if expansion_off is not None else image_base

    found_0x70 = False
    ucode_tables = []
    for t in range(bit_token_count):
        tok = bit_off + bit_hdr_size + t * bit_stride
        if rom[tok : tok + 2] != b"\x70\x02":
            continue
        found_0x70 = True
        raw_ptr = _le32(rom, tok + 4) if bit_stride >= 8 else _le16(rom, tok + 4)
        if raw_ptr is None:
            continue
        data_ptr = image_base + raw_ptr
        table_rel = _le32(rom, data_ptr)
        if table_rel is None:
            continue
        table_off = adjust + table_rel
        if table_off + 6 > len(rom):
            continue
        ver = rom[table_off]
        hdr_size = rom[table_off + 1]
        entry_size = rom[table_off + 2]
        slot_count = rom[table_off + 3]
        if ver != 1 or hdr_size < 6 or entry_size < 6:
            continue

        empty_slots = 0
        entries = []
        for e in range(slot_count):
            at = table_off + hdr_size + e * entry_size
            if at + 6 > len(rom):
                break
            app_id = rom[at]
            target_id = rom[at + 1]
            desc_rel = _le32(rom, at + 2)
            if app_id == 0 and target_id == 0 and desc_rel == 0:
                empty_slots += 1
                continue
            desc_off = adjust + desc_rel
            if desc_off + 4 > len(rom):
                entries.append({
                    "kind": "oob",
                    "app_id": app_id,
                    "target_id": target_id,
                    "desc_rel": desc_rel,
                    "abs_off": desc_off,
                })
                continue
            vdesc = _le32(rom, desc_off)
            desc_ver = (vdesc >> 8) & 0xFF
            desc_size = (vdesc >> 16) & 0xFFFF
            encrypted = bool(vdesc & (1 << 2))
            stored_size = _le32(rom, desc_off + 4)
            meta = {"kind": "other"}
            if desc_ver == 2 and desc_size >= 60 and desc_off + 52 <= len(rom):
                meta = {
                    "kind": "v2",
                    "imem_load": _le32(rom, desc_off + 24),
                    "sec_base": _le32(rom, desc_off + 32),
                    "sec_size": _le32(rom, desc_off + 36),
                    "dmem_load": _le32(rom, desc_off + 48),
                }
            elif desc_ver == 3 and desc_size >= 44 and desc_off + 42 <= len(rom):
                meta = {
                    "kind": "v3",
                    "pkc_off": _le32(rom, desc_off + 8),
                    "engine_mask": _le16(rom, desc_off + 36),
                    "ucode_id": rom[desc_off + 38],
                    "sig_count": rom[desc_off + 39],
                    "sig_versions": _le16(rom, desc_off + 40),
                }
            elif desc_ver == 6 and desc_size >= 80:
                meta = {"kind": "v6", "image_start": desc_off + desc_size}
            entries.append({
                "kind": "readable",
                "app_id": app_id,
                "target_id": target_id,
                "desc_off": desc_off,
                "desc_ver": desc_ver,
                "desc_size": desc_size,
                "encrypted": encrypted,
                "stored_size": stored_size,
                "meta": meta,
            })
        ucode_tables.append({
            "table_off": table_off,
            "ver": ver,
            "hdr_size": hdr_size,
            "entry_size": entry_size,
            "slot_count": slot_count,
            "empty_slots": empty_slots,
            "entries": entries,
        })

    if not found_0x70:
        return {"error": "no_ucode_token"}

    return {
        "error": None,
        "first_word": first_word,
        "rom_images": rom_images,
        "bit_off": bit_off,
        "bit_hdr_size": bit_hdr_size,
        "bit_stride": bit_stride,
        "bit_token_count": bit_token_count,
        "image_base": image_base,
        "expansion_off": expansion_off,
        "adjust": adjust,
        "ucode_tables": ucode_tables,
    }


def dump_prom_bit(bar):
    print("\n--- PROM BIT TABLE ENUMERATION (BAR0+0x300000, read-only metadata) ---")
    buf = bytearray(PROM_SIZE)
    for i in range(0, PROM_SIZE, 4):
        buf[i : i + 4] = int.to_bytes(bar.rd(PROM_BASE + i), 4, "little")
    rep = parse_prom_bit(bytes(buf))
    err = rep.get("error")
    if err == "unreadable":
        print(f"PROM unreadable: {rep['reason']} (device parked / no ROM shadow)")
    elif err == "no_bit_header":
        print("PROM-BIT: no checksum-valid BIT header found in 1 MiB aperture")
    elif err == "unsupported_stride":
        print(f"PROM-BIT: BIT token stride {rep['stride']} outside expected range 6..=8")
    elif err == "no_ucode_token":
        print("PROM-BIT: BIT header parsed, but token 0x70 v2 (falcon ucode table) is absent")
    else:
        print(f"PROM first word: {rep['first_word']:#010x}")
        print(f"verified PCI ROM images ({len(rep['rom_images'])}):")
        for img in rep["rom_images"]:
            print(
                f"  PROM+{img['offset']:#07x}: tag=\"{img['tag']}\" "
                f"code_type={img['code_type']:#04x} len={img['image_len']:#x}"
            )
        exp_s = f"{rep['expansion_off']:#x}" if rep["expansion_off"] is not None else "none"
        print(
            f"BIT at PROM+{rep['bit_off']:#07x} (hdr={rep['bit_hdr_size']} "
            f"stride={rep['bit_stride']} tokens={rep['bit_token_count']}) | "
            f"image_base={rep['image_base']:#x} expansion={exp_s} adjust={rep['adjust']:#x}"
        )
        for tbl in rep["ucode_tables"]:
            print(
                f"ucode table at PROM+{tbl['table_off']:#07x}: ver={tbl['ver']} "
                f"hdr={tbl['hdr_size']} entry_size={tbl['entry_size']} slots={tbl['slot_count']}"
            )
            print(
                f"  {'APP':>4}  {'TARGET':>6}  {'NAME':>16}  {'DESC_OFF':>10}  "
                f"{'VER':>4}  {'ENC(bit2)':>9}  {'DESC_SIZE':>9}  STORED_SIZE"
            )
            print("  " + "-" * 82)
            readable = 0
            oob = 0
            for entry in tbl["entries"]:
                if entry["kind"] == "oob":
                    oob += 1
                    print(
                        f"  {entry['app_id']:#04x}  {entry['target_id']:#04x}  "
                        f"{prom_app_name(entry['app_id']):>16}  "
                        f"OOB(rel={entry['desc_rel']:#x}, abs={entry['abs_off']:#x})"
                    )
                else:
                    readable += 1
                    enc_s = "YES" if entry["encrypted"] else "no"
                    stored = entry["stored_size"]
                    stored_s = f"{stored:#x} ({stored // 1024} KiB)" if stored is not None else "-"
                    print(
                        f"  {entry['app_id']:#04x}  {entry['target_id']:#04x}  "
                        f"{prom_app_name(entry['app_id']):>16}  {entry['desc_off']:#010x}  "
                        f"v{entry['desc_ver']:<3}  {enc_s:>9}  {entry['desc_size']:>6} B  {stored_s}"
                    )
                    m = entry["meta"]
                    if m["kind"] == "v2" and not entry["encrypted"]:
                        print(
                            f"    V2: imemLoad={m['imem_load']:#x} secBase={m['sec_base']:#x} "
                            f"secSize={m['sec_size']:#x} dmemLoad={m['dmem_load']:#x}"
                        )
                    elif m["kind"] == "v3":
                        print(
                            f"    V3: pkcOff={m['pkc_off']:#x} engineMask={m['engine_mask']:#x} "
                            f"ucodeId={m['ucode_id']} sigCount={m['sig_count']} "
                            f"sigVersions={m['sig_versions']:#x}"
                        )
                    elif m["kind"] == "v6":
                        print(
                            f"    V6: desc_size={entry['desc_size']} "
                            f"image_start=PROM+{m['image_start']:#x}"
                        )
            print(
                f"  summary: {tbl['slot_count']} slot(s) "
                f"({readable + oob} non-empty: {readable} readable, {oob} out-of-aperture; "
                f"{tbl['empty_slots']} empty)"
            )
        print("note: metadata-only read-only scan; raw firmware bytes are intentionally omitted")
    print("--- END PROM BIT ---")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bdf", help="only this PCI address, e.g. 0000:01:00.0")
    ap.add_argument("--reapply", action="store_true",
                    help="write SS0=0x88888888 SS1=0x8 where the PLM gate is open")
    ap.add_argument("--prom-bit", action="store_true",
                    help="READ-ONLY: read BAR0+0x300000 (1 MiB NV_PROM) and walk PCI ROM -> BIT -> "
                         "token 0x70 v2 ucode table metadata (no firmware byte dumps; cannot be "
                         "combined with --reapply or --fuserows)")
    ap.add_argument("--fusebase", default=None, metavar="HEX",
                    help="fuse-controller block base override (default family-auto: "
                         "Turing 0x21000, GA10x+/Ada 0x820000; same layout)")
    ap.add_argument("--fuserows", nargs="?", const="all", default=None, metavar="A-B",
                    help="WRITE MODE: physical fuse-row capture (ga100-ff-tools read_rows"
                         " protocol: FUSEADDR row select + FUSECTRL CMD_READ only, guarded,"
                         " two passes, address restored). Range A-B hex/decimal;"
                         " default = whole IFF domain")
    ap.add_argument("--target-offset", dest="target_offsets", action="append", default=None,
                    metavar="HEX",
                    help="ARBITRARY WRITE MODE, repeatable ordered pairs with --value: "
                         "write a hex 32-bit value to a BAR0 hex offset and read back. "
                         "Lands regardless of the NVIDIA driver's state; a wrong offset "
                         "can hang the box (recovery = power cycle; the unlock state "
                         "survives reboots, rail-off clears it)")
    ap.add_argument("--value", dest="poke_values", action="append", default=None,
                    metavar="HEX",
                    help="hex 32-bit value for the preceding --target-offset")
    ap.add_argument("--force-diag", action="store_true",
                    help="IO_STRICT_DEVMEM boxes only: on EINVAL from the BAR0 mmap, "
                         "automatically UNBIND the NVIDIA driver, run, and RE-BIND at "
                         "exit (root). Explicit opt-in because a driver unbind can "
                         "hang (D-state history); the safer route is iomem=relaxed "
                         "on the kernel cmdline")
    ap.add_argument("--inventory", action="store_true",
                    help="print the unelevated read-only hardware inventory "
                         "block (OS/CPU/board/BIOS/chassis/per-GPU family "
                         "verdict + advisories) and exit - no root needed")
    args = ap.parse_args()

    pokes = []
    if args.target_offsets or args.poke_values:
        offs = args.target_offsets or []
        vals = args.poke_values or []
        if len(offs) != len(vals):
            print(
                "--target-offset/--value count mismatch (each offset needs exactly one value)",
                file=sys.stderr,
            )
            return 2
        for o, v in zip(offs, vals):
            try:
                off = int(o, 16)
                val = int(v, 16)
            except ValueError:
                print(f"bad poke pair {o!r}={v!r}: hex only", file=sys.stderr)
                return 2
            if off >= 0x0100_0000:
                print(
                    f"bad --target-offset {o}: BAR0-relative offsets live inside the 16 MiB aperture",
                    file=sys.stderr,
                )
                return 2
            pokes.append((off, val))

    if args.prom_bit and (args.reapply or args.fuserows is not None or pokes):
        print(
            "--prom-bit is strictly read-only and cannot be combined with --reapply, --fuserows or --target-offset/--value",
            file=sys.stderr,
        )
        return 2

    fuserows = None
    if args.fuserows is not None:
        if args.fuserows == "all":
            fuserows = "all"
        else:
            try:
                ra, rb = args.fuserows.split("-", 1)
                pa = int(ra, 16) if ra.strip().lower().startswith("0x") else int(ra)
                pb = int(rb, 16) if rb.strip().lower().startswith("0x") else int(rb)
                assert 0 <= pa <= pb
                fuserows = (pa, pb)
            except (ValueError, AssertionError):
                print("bad range; use A-B, hex or decimal (e.g. --fuserows=0x2ff-0x33f)", file=sys.stderr)
                return 2

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
    if pokes and len(devs) > 1 and not args.bdf:
        print(
            f"poke REFUSED: {len(devs)} NVIDIA GPUs discovered and no --bdf given - "
            "arbitrary writes would hit every card; pass --bdf",
            file=sys.stderr,
        )
        return 2

    # ssdiag 0.7.0 parity: a full dump opens with its own platform block
    inventory_block()

    rc = 0
    for bdf, devpath in devs:
        print(f"{bdf}:")
        try:
            bar = Bar0(devpath, force_diag=args.force_diag)
        except (PermissionError, RuntimeError) as e:
            print(f"  BAR0 unavailable: {e}")
            rc = 1
            continue
        st = report(bar, devpath=devpath)
        read_rebar(devpath)
        if st is None:
            if args.prom_bit:
                dump_prom_bit(bar)
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
        if pokes:
            # 0.8.9-twin: arbitrary in-vivo writes through the same sysfs
            # BAR0 mmap the diagnostics use - lands regardless of the
            # NVIDIA driver's state (the #246 bench: RmInitAdapter failed,
            # gate open). Read-back is the only success criterion.
            print(f"  poke: ARBITRARY WRITE MODE, {len(pokes)} write(s) in order - a wrong offset"
                  " can hang the box (recovery = power cycle; unlock state survives reboots,"
                  " rail-off clears it)")
            for off, val in pokes:
                bar.wr(off, val)
                rb = bar.rd(off)
                ok = rb == val
                print(f"  poke {off:#x} <- {val:#x} -> readback {rb:#010x}"
                      f" {'OK' if ok else 'MISMATCH (dropped or masked - PLM filter?)'}")
                if not ok:
                    rc = 1
        if fuserows is not None:
            base = None
            if args.fusebase:
                base = int(args.fusebase, 16) if args.fusebase.lower().startswith("0x") else int(args.fusebase, 16)
            if base is None:
                base = FUSE_BASE_TURING if fam == "turing" else FUSE_BASE_GA100_LAYOUT
            if not fuse_row_dump(bar, fuserows, base):
                rc = 1
        if args.prom_bit:
            dump_prom_bit(bar)
    for devpath, drv in reversed(_FORCE_DIAG_UNBOUND):
        print(f"  force-diag: re-binding {drv} to {os.path.basename(devpath)}")
        try:
            _driver_io(drv, "bind", devpath)
        except OSError as e:
            print(f"  force-diag WARNING: re-bind failed ({e}) - bind manually via "
                  f"/sys/bus/pci/drivers/{drv}/bind", file=sys.stderr)
            rc = 1
    return rc


if __name__ == "__main__":
    sys.exit(main())
